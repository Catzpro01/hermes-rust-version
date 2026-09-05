use crate::output::sanitize_untrusted_output;
use crate::session_menu::{
    inspect_session, list_sessions, parse_resume, search_sessions, select_session, show_messages,
    show_tool_calls,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use hermes_core::{
    config::{HermesConfig, McpServerConfig},
    conversation::context::summarize_dropped,
    conversation::goal::GoalStatus,
    conversation::{AgenticResult, ConversationRunner},
    mcp::{McpServer, McpTool},
    provider::{Provider, ProviderError, ProviderRegistry, RegistryError},
    session::SessionStore,
    tools::{
        Confirmation, ListDirTool, ReadFileTool, ShellReadonlyTool, Tool, ToolRegistry,
        WriteFileTool,
    },
};
use rustyline::{error::ReadlineError, DefaultEditor};
use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
struct CliConfirmation {
    requests: mpsc::Sender<(String, oneshot::Sender<bool>)>,
}
#[async_trait]
impl Confirmation for CliConfirmation {
    async fn confirm(&self, prompt: &str) -> bool {
        let (reply, result) = oneshot::channel();
        if self
            .requests
            .send((prompt.to_owned(), reply))
            .await
            .is_err()
        {
            return false;
        }
        result.await.unwrap_or(false)
    }
}

pub async fn run_repl(
    home: &std::path::Path,
    provider: Box<dyn Provider>,
    provider_name: String,
    registry: ProviderRegistry,
    config: Option<HermesConfig>,
    base_url_override: Option<String>,
    resume: bool,
) -> Result<()> {
    let mut provider_name = provider_name;
    #[cfg(unix)]
    let mut sigint = signal(SignalKind::interrupt())?;
    let db = home.join("state.db");
    let mut store = SessionStore::open(&db).context("open Hermes state.db")?;
    let mut editor = DefaultEditor::new().context("create terminal editor")?;
    // Spec 013 Ticket 03 — startup welcome banner (Python parity, spec §3).
    // TTY-only: piped E2E invocations must keep byte-stable, ANSI-free stdout
    // (render-boundary invariant; several E2E tests assert no `\x1b` on piped
    // output).
    if std::io::stdin().is_terminal() {
        let _ = crate::tui::welcome::print_banner(&mut std::io::stdout(), terminal_width());
    }
    let mut session_id = if resume || !std::io::stdin().is_terminal() {
        match store.list()?.last().copied() {
            Some(id) => id,
            None => store.create_session("cli")?,
        }
    } else {
        select_session(&store, &mut editor)?
    };
    let existing = store.resume(&session_id)?.turns;
    let mut runner = ConversationRunner::from_turns(provider, existing);
    // Advisory context limit resolved from config precedence for the active
    // provider (ProviderConfig.context_length -> ModelConfig.context_length ->
    // compression.target_max_tokens -> None). Feeds the runner's token
    // accounting + pre-send warning.
    let mut ctx = resolve_context(config.as_ref(), &provider_name);
    runner.set_context_limit(ctx.limit);
    println!("Hermes-RS session {session_id} (provider {provider_name})");
    println!("Commands: /provider [name], /pin <n>, /unpin <n>, /pinned, /goal [on|off|reset], /plan [on|off|reset], /reflect [on|off], /new, /sessions, /inspect <id>, /messages <id>, /tool-calls <id>, /search <query>, /resume <id>, /info, /exit");
    if let Some(limit) = ctx.limit {
        println!(
            "[context ~{} tokens / limit {limit} | compression {}]",
            runner.estimated_tokens(),
            compression_label(&ctx)
        );
    }
    let editor = Arc::new(Mutex::new(editor));
    let (confirmation_tx, mut confirmation_rx) =
        mpsc::channel::<(String, oneshot::Sender<bool>)>(8);
    let confirmation_editor = Arc::clone(&editor);
    // Spec 013 Ticket 04 — kawaii waiting face while awaiting approval
    // (spec §7); the flag pauses + clears the spinner line for the duration
    // of the prompt. The face is display-only (never enters canonical text).
    let confirm_active = Arc::new(AtomicBool::new(false));
    tokio::spawn({
        let confirm_flag = Arc::clone(&confirm_active);
        async move {
            let mut n = 0u32;
            while let Some((prompt, reply)) = confirmation_rx.recv().await {
                let face = crate::tui::kawaii::KAWAII_WAITING
                    [(n as usize) % crate::tui::kawaii::KAWAII_WAITING.len()];
                n += 1;
                confirm_flag.store(true, Ordering::Relaxed);
                let answer = tokio::task::spawn_blocking({
                    let editor = Arc::clone(&confirmation_editor);
                    move || {
                        editor
                            .lock()
                            .ok()
                            .and_then(|mut e| {
                                e.readline(&format!("{face} confirm {prompt} [y/N] ").to_owned())
                                    .ok()
                            })
                            .map(|s| matches!(s.trim().to_ascii_lowercase().as_str(), "y" | "yes"))
                            .unwrap_or(false)
                    }
                })
                .await
                .unwrap_or(false);
                confirm_flag.store(false, Ordering::Relaxed);
                let _ = reply.send(answer);
            }
        }
    });
    let mut tool_registry = ToolRegistry::new();
    let tool_root = std::env::current_dir().context("resolve CLI tool root")?;
    let confirmation = CliConfirmation {
        requests: confirmation_tx,
    };
    tool_registry.register(ReadFileTool::new(&tool_root));
    tool_registry.register(ListDirTool::new(&tool_root));
    tool_registry.register(ShellReadonlyTool::new(
        confirmation.clone(),
        Duration::from_secs(30),
    ));
    tool_registry.register(WriteFileTool::new(&tool_root, confirmation.clone()));
    // Spec 011: connect configured MCP servers and register their tools. Off by
    // default (no `mcp_servers` -> nothing spawns). A server that fails to
    // start/discover is reported per-server and skipped; the rest stay up.
    // Each live server is tracked in `mcp_handles` so `/mcp list` and
    // `/mcp restart <name>` can inspect/swap it. Child processes are killed on
    // drop (kill_on_drop) when run_repl returns on any exit path.
    let mut mcp_handles: Vec<McpHandle> = Vec::new();
    if let Some(config) = &config {
        let mut names: Vec<&String> = config.mcp_servers.keys().collect();
        names.sort();
        for name in names {
            let cfg = &config.mcp_servers[name];
            let handle = McpHandle::connect(name, cfg.clone(), confirmation.clone(), &mut tool_registry).await;
            if handle.server.is_some() {
                eprintln!(
                    "mcp[{name}]: connected, registered {} tool(s)",
                    handle.tool_names.len()
                );
            } else if let Some(e) = &handle.error {
                eprintln!("mcp[{name}]: {e}");
            }
            mcp_handles.push(handle);
        }
    }
    loop {
        if !std::io::stdin().is_terminal() {
            print!("{}", crate::tui::welcome::PROMPT_SYMBOL);
            std::io::Write::flush(&mut std::io::stdout())?;
        }
        let editor_for_read = Arc::clone(&editor);
        let readline = tokio::task::spawn_blocking(move || {
            let mut editor = match editor_for_read.lock() {
                Ok(editor) => editor,
                Err(_) => return "editor lock poisoned".to_owned(),
            };
            match editor.readline(crate::tui::welcome::PROMPT_SYMBOL) {
                Ok(line) => line,
                Err(ReadlineError::Eof) => "__HERMES_EOF__".to_owned(),
                Err(ReadlineError::Interrupted) => "__HERMES_INTERRUPTED__".to_owned(),
                Err(other) => other.to_string(),
            }
        });
        #[cfg(unix)]
        let line = tokio::select! {
            result = readline => result.map_err(|e| anyhow::anyhow!(e.to_string()))?,
            _ = sigint.recv() => return Err(anyhow::anyhow!("interrupted")),
        };
        #[cfg(not(unix))]
        let line = readline.await.map_err(|e| anyhow::anyhow!(e.to_string()))?;
        if line == "__HERMES_EOF__" {
            println!();
            break;
        }
        if line == "__HERMES_INTERRUPTED__" {
            return Err(anyhow::anyhow!("SIGINT"));
        }
        if !line.trim().is_empty() {
            let _ = editor
                .lock()
                .map_err(|_| anyhow::anyhow!("editor lock poisoned"))?
                .add_history_entry(line.as_str());
        }
        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        match input {
            "/exit" => break,
            "/sessions" => {
                list_sessions(&store)?;
                continue;
            }
            command if command.starts_with("/inspect ") => {
                inspect_session(&store, parse_resume(command)?)?;
                continue;
            }
            command if command.starts_with("/messages ") => {
                show_messages(&store, parse_resume(command)?)?;
                continue;
            }
            command if command.starts_with("/tool-calls ") => {
                show_tool_calls(&store, parse_resume(command)?)?;
                continue;
            }
            command if command.starts_with("/search ") => {
                let query = command
                    .split_once(' ')
                    .map(|(_, q)| q.trim())
                    .filter(|q| !q.is_empty())
                    .ok_or_else(|| anyhow::anyhow!("usage: /search <query>"))?;
                search_sessions(&store, query)?;
                continue;
            }
            "/new" => {
                let id = store.create_session("cli")?;
                session_id = id;
                runner.replace_turns(Vec::new());
                println!("New session {id}");
                continue;
            }
            command if command.starts_with("/resume ") => {
                let id = parse_resume(command)?;
                let history = store.resume(&id)?.turns;
                session_id = id;
                runner.replace_turns(history);
                println!("Resumed {id}");
                continue;
            }
            // `/info` shows current context accounting for the active provider.
            "/info" => {
                let limit = ctx
                    .limit
                    .map(|l| l.to_string())
                    .unwrap_or_else(|| "none".to_owned());
                let sent = runner.turns().len().saturating_sub(runner.dropped_turns().len());
                println!(
                    "provider: {provider_name} | estimated context: ~{} tokens | limit: {limit} | window: {sent}/{} turns sent | pinned: {} | compression: {}",
                    runner.estimated_tokens(),
                    runner.turns().len(),
                    runner.pinned().len(),
                    compression_label(&ctx)
                );
                if let Some(w) = runner.context_warning() {
                    println!("{w}");
                }
                // Advisory visibility of what the sliding window would drop
                // (Ticket 03). This is DISPLAY ONLY and is never injected into
                // the LLM context. Output is ANSI-sanitized and credentials are
                // redacted before it reaches the terminal.
                let dropped = runner.dropped_turns();
                if !dropped.is_empty() {
                    let summary = summarize_dropped(&dropped);
                    let safe =
                        hermes_core::search::redact::redact_credentials(&sanitize_untrusted_output(
                            &summary,
                        ));
                    println!("  {safe}");
                }
                continue;
            }
            // `/pin <n>` marks a turn (0-based into current history) so the
            // sliding window never drops it. Pins are in-memory per-session.
            command if command.starts_with("/pin ") => {
                let n = parse_index_arg(command)?;
                match runner.pin(n) {
                    Ok(()) => println!("Pinned turn {n}"),
                    Err(e) => eprintln!("error: {e}"),
                }
                continue;
            }
            // `/unpin <n>` removes a pin.
            command if command.starts_with("/unpin ") => {
                let n = parse_index_arg(command)?;
                match runner.unpin(n) {
                    Ok(()) => println!("Unpinned turn {n}"),
                    Err(e) => eprintln!("error: {e}"),
                }
                continue;
            }
            // `/pinned` lists all pinned turn indices.
            "/pinned" => {
                let pinned = runner.pinned();
                if pinned.is_empty() {
                    println!("no pinned turns");
                } else {
                    let labels: Vec<String> = pinned
                        .iter()
                        .map(|&i| {
                            // Sanitize a short preview of each pinned turn.
                            let t = &runner.turns()[i];
                            let text = match t {
                                hermes_core::conversation::Turn::User { content }
                                | hermes_core::conversation::Turn::Assistant { content } => {
                                    content.as_str()
                                }
                                hermes_core::conversation::Turn::Tool { name, .. } => name.as_str(),
                            };
                            let preview: String = text.chars().take(40).collect();
                            format!("{i}:{preview}")
                        })
                        .collect();
                    println!("pinned: {}", labels.join(" | "));
                }
                continue;
            }
            // `/goal` shows the active tracked goal (Spec 009 Ticket 01). The
            // goal is advisory in-memory state; display is sanitized/redacted.
            "/goal" => match runner.goal() {
                None => println!("no active goal (run `/goal on` to auto-track)"),
                Some(text) => {
                    let safe =
                        hermes_core::search::redact::redact_credentials(&sanitize_untrusted_output(
                            text,
                        ));
                    println!("goal [{}]: {safe}", runner.goal_status().as_str());
                }
            },
            // `/goal on|off|reset` controls Spec 009 goal tracking.
            command if command.starts_with("/goal ") => {
                match command.trim_start_matches("/goal").trim() {
                    "on" => {
                        runner.set_goal_tracking(true);
                        println!("Goal tracking on (records your next task)");
                    }
                    "off" => {
                        runner.set_goal_tracking(false);
                        println!("Goal tracking off");
                    }
                    "reset" => {
                        runner.reset_goal();
                        println!("Goal cleared");
                    }
                    "achieved" => {
                        runner.set_goal_status(GoalStatus::Achieved);
                        println!("Goal marked achieved");
                    }
                    "blocked" => {
                        runner.set_goal_status(GoalStatus::Blocked);
                        println!("Goal marked blocked");
                    }
                    other => eprintln!("error: unknown /goal arg '{other}' (use on|off|reset)"),
                }
                continue;
            }
            // `/plan` shows planned mode and the active plan (Spec 009 Ticket 02).
            "/plan" => {
                println!(
                    "plan mode: {}",
                    if runner.plan_mode() { "on" } else { "off" }
                );
                match runner.plan() {
                    None => println!("no active plan"),
                    Some(plan) => {
                        let mut body = String::from("active plan:\n");
                        for (i, step) in plan.steps().iter().enumerate() {
                            body.push_str(&format!("  {}. {step}\n", i + 1));
                        }
                        let safe =
                            hermes_core::search::redact::redact_credentials(
                                &sanitize_untrusted_output(&body),
                            );
                        print!("{safe}");
                    }
                }
                continue;
            }
            // `/plan on|off|reset` controls planned mode / plan state.
            command if command.starts_with("/plan ") => {
                match command.trim_start_matches("/plan").trim() {
                    "on" => {
                        runner.set_plan_mode(true);
                        println!("Plan mode on (a plan will be generated for your next task)");
                    }
                    "off" => {
                        runner.set_plan_mode(false);
                        println!("Plan mode off (reactive)");
                    }
                    "reset" => {
                        runner.clear_plan();
                        println!("Active plan cleared");
                    }
                    other => eprintln!("error: unknown /plan arg '{other}' (use on|off|reset)"),
                }
                continue;
            }
            // `/reflect` shows / toggles the reflection gate (Spec 009 Ticket 03).
            "/reflect" => {
                println!(
                    "reflection: {} (reflections used this step: {})",
                    if runner.reflection_enabled() { "on" } else { "off" },
                    runner.reflections_used()
                );
                continue;
            }
            command if command.starts_with("/reflect ") => {
                match command.trim_start_matches("/reflect").trim() {
                    "on" => {
                        runner.set_reflection(true);
                        println!("Reflection on (tool outcomes gate the goal)");
                    }
                    "off" => {
                        runner.set_reflection(false);
                        println!("Reflection off");
                    }
                    other => eprintln!("error: unknown /reflect arg '{other}' (use on|off)"),
                }
                continue;
            }
            // `/provider` lists available providers and marks the active one.
            "/provider" => {
                list_providers(&registry, &provider_name);
                continue;
            }
            // `/provider <name>` switches mid-session. The REPL only reads a new
            // command once the previous turn has finished (the read loop is idle
            // here), so a switch is always at a turn boundary and can never split
            // one turn across two providers.
            command if command.starts_with("/provider ") => {
                let target = command.trim_start_matches("/provider").trim();
                if target.is_empty() {
                    list_providers(&registry, &provider_name);
                } else {
                    match resolve_provider(&registry, config.as_ref(), target, base_url_override.as_deref())
                    {
                        Ok(new_provider) => {
                            runner.replace_provider(new_provider);
                            provider_name = target.to_owned();
                            // Context limit (and compression status) follow the
                            // active provider.
                            ctx = resolve_context(config.as_ref(), &provider_name);
                            runner.set_context_limit(ctx.limit);
                            println!("Switched provider to {target}");
                        }
                        Err(err) => {
                            // Failed init leaves the active provider untouched
                            // (rollback, not a half-finished switch).
                            eprintln!(
                                "error: {err}; keeping provider {provider_name}"
                            );
                        }
                    }
                }
                continue;
            }
            // `/mcp` / `/mcp list` shows each connected MCP server and its tool
            // count (Spec 011b #04). `/mcp restart <name>` swaps one server.
            "/mcp" | "/mcp list" => {
                if mcp_handles.is_empty() {
                    println!("no MCP servers connected (add `mcp_servers:` to config.yaml)");
                } else {
                    println!("MCP servers:");
                    for h in &mcp_handles {
                        let status = if h.server.is_some() { "connected" } else { "down" };
                        let mode = if h.config.confirm { "confirm" } else { "auto" };
                        println!(
                            "  {:<12} {:<10} {} tool(s) ({} mode)",
                            h.name,
                            status,
                            h.tool_names.len(),
                            mode
                        );
                    }
                }
                continue;
            }
            command if command.starts_with("/mcp restart ") => {
                let target = command.trim_start_matches("/mcp restart").trim();
                if target.is_empty() {
                    eprintln!("usage: /mcp restart <name>");
                    continue;
                }
                match mcp_handles
                    .iter_mut()
                    .find(|h| h.name == target)
                {
                    None => eprintln!("error: no MCP server named '{target}'"),
                    Some(handle) => {
                        handle.restart(confirmation.clone(), &mut tool_registry).await;
                        if handle.server.is_some() {
                            eprintln!(
                                "mcp[{target}]: restarted, registered {} tool(s)",
                                handle.tool_names.len()
                            );
                        } else if let Some(e) = &handle.error {
                            eprintln!("mcp[{target}]: restart failed: {e}");
                        }
                        continue;
                    }
                }
            }
            // Spec 013 Ticket 03 — `/help` with the verbatim kawaii header.
            "/help" => {
                use crate::tui::welcome::{HELP_HEADER, SEPARATOR};
                println!("{HELP_HEADER}");
                println!("{SEPARATOR}");
                for (cmd, desc) in [
                    ("/exit", "leave Hermes-RS"),
                    ("/new", "start a new session"),
                    ("/sessions", "list sessions"),
                    ("/resume <id>", "resume a session"),
                    ("/info", "provider + context accounting"),
                    ("/search <query>", "full-text search (read-only)"),
                    ("/inspect <id>", "inspect a session"),
                    ("/messages <id>", "show session messages"),
                    ("/tool-calls <id>", "show session tool calls"),
                    ("/pin <n>", "pin a turn (never windowed)"),
                    ("/unpin <n>", "unpin a turn"),
                    ("/pinned", "list pinned turns"),
                    ("/goal [on|off|reset]", "guided goal tracking"),
                    ("/plan [on|off|reset]", "plan-then-execute"),
                    ("/reflect [on|off]", "self-reflection gate"),
                    ("/provider [name]", "show / switch provider"),
                    ("/mcp [list|restart <name>]", "MCP servers"),
                    ("/help", "this help"),
                ] {
                    println!("  {cmd:<24} {desc}");
                }
                continue;
            }
            _ => {
                let before = runner.turns().len();
                let turn_cancel = CancellationToken::new();
                // Spec 013 Ticket 04 — live streaming display. ONE task owns
                // the whole turn (turn future + observer channel + 120 ms
                // spinner tick + SIGINT), so stdout writes are ordered and
                // race-free. Every chunk is scrubbed + redacted at this
                // boundary (invariant 4); canonical bytes stay untouched
                // (invariant 5).
                let tty = std::io::stdin().is_terminal();
                let width = if tty { terminal_width() } else { 60 };
                let (disp_tx, mut disp_rx) =
                    mpsc::channel::<hermes_core::conversation::AgentEvent>(64);
                runner.set_observer(disp_tx);
                let mut renderer = crate::streaming::StreamRenderer::new(tty, width);
                let mut spinner: Option<crate::streaming::SpinnerState> = tty.then(|| {
                    crate::streaming::SpinnerState::new(crate::streaming::SpinnerMode::Thinking)
                });
                let mut turn = Box::pin(runner.chat_agentic(
                    input.to_owned(),
                    &tool_registry,
                    Some((&store, &session_id)),
                    10,
                    turn_cancel.clone(),
                ));
                let mut next_tick = std::time::Instant::now() + crate::streaming::TICK_INTERVAL;
                let result: Result<AgenticResult, ProviderError> = loop {
                    // 1. Drain every ready display event, in arrival order.
                    while let Ok(ev) = disp_rx.try_recv() {
                        let _ = crate::streaming::apply_event(
                            &mut renderer,
                            &mut spinner,
                            &mut std::io::stdout(),
                            &ev,
                        );
                    }
                    // 2. Spinner tick due? (TTY only; paused while the
                    //    approval prompt is up — the waiting face then sits
                    //    on the prompt line instead).
                    let tick_due =
                        tty && spinner.is_some() && !confirm_active.load(Ordering::Relaxed);
                    let wake_at = tokio::time::Instant::from(next_tick);
                    #[cfg(unix)]
                    {
                        tokio::select! {
                            res = &mut turn => break res,
                            ev = disp_rx.recv() => {
                                if let Some(ev) = ev {
                                    let _ = crate::streaming::apply_event(
                                        &mut renderer,
                                        &mut spinner,
                                        &mut std::io::stdout(),
                                        &ev,
                                    );
                                }
                            }
                            _ = sigint.recv() => {
                                // Spec 013 invariant #2: exit 130, partial
                                // turn discarded. Close any open box and clear
                                // the spinner line before leaving.
                                turn_cancel.cancel();
                                let _ = renderer.finish(&mut std::io::stdout());
                                if tty {
                                    let mut out = std::io::stdout();
                                    let _ = write!(&mut out, "\r\x1b[2K");
                                    let _ = std::io::Write::flush(&mut out);
                                }
                                // Release the turn future's `&mut runner`
                                // borrow before discarding the partial turn.
                                drop(turn);
                                runner.discard_pending_user();
                                return Err(anyhow::anyhow!("interrupted"));
                            }
                            _ = tokio::time::sleep_until(wake_at), if tick_due => {
                                if let Some(sp) = &mut spinner {
                                    let line = sp.advance();
                                    let mut out = std::io::stdout();
                                    let _ = write!(&mut out, "\r{line}");
                                    let _ = std::io::Write::flush(&mut out);
                                    next_tick = std::time::Instant::now()
                                        + crate::streaming::TICK_INTERVAL;
                                }
                            }
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        tokio::select! {
                            res = &mut turn => break res,
                            ev = disp_rx.recv() => {
                                if let Some(ev) = ev {
                                    let _ = crate::streaming::apply_event(
                                        &mut renderer,
                                        &mut spinner,
                                        &mut std::io::stdout(),
                                        &ev,
                                    );
                                }
                            }
                            _ = tokio::time::sleep_until(wake_at), if tick_due => {
                                if let Some(sp) = &mut spinner {
                                    let line = sp.advance();
                                    let mut out = std::io::stdout();
                                    let _ = write!(&mut out, "\r{line}");
                                    let _ = std::io::Write::flush(&mut out);
                                    next_tick = std::time::Instant::now()
                                        + crate::streaming::TICK_INTERVAL;
                                }
                            }
                        }
                    }
                };
                // The turn future is complete; drop it so its borrows of
                // `runner`/`store` end before we save turns below.
                drop(turn);
                match result {
                    Ok(AgenticResult::Done { text, iterations }) => {
                        // Spec 013 Ticket 04 — the streaming path already
                        // rendered the answer (box + scrubbed text), so the
                        // final text is NOT printed again (no duplication).
                        // Non-streaming providers (no Chunk events) fall back
                        // to rendering the final text through the same
                        // renderer path — box + scrubbing still apply.
                        if !renderer.any_text() {
                            let _ = renderer.emit_final(&mut std::io::stdout(), &text);
                        }
                        println!("[iter {iterations}/10]");
                    }
                    Ok(AgenticResult::MaxIterations(limit)) => {
                        let _ = renderer.finish(&mut std::io::stdout());
                        eprintln!("\n⚠ Reached max iterations budget ({limit}).")
                    }
                    Ok(AgenticResult::Blocked { reason }) => {
                        let _ = renderer.finish(&mut std::io::stdout());
                        eprintln!("\n⛔ blocked: {reason}");
                    }
                    Ok(AgenticResult::Cancelled) | Err(ProviderError::Cancelled) => {
                        let _ = renderer.finish(&mut std::io::stdout());
                        eprintln!("\n⚡ interrupted");
                        return Err(anyhow::anyhow!("interrupted"));
                    }
                    Err(error) => {
                        let _ = renderer.finish(&mut std::io::stdout());
                        return Err(anyhow::anyhow!(error.to_string()));
                    }
                }
                for turn in &runner.turns()[before..] {
                    store.save_turn(&session_id, turn)?;
                }
            }
        }
    }
    // Spec 013 Ticket 03 — goodbye on clean exit only. SIGINT never reaches
    // this point (it returns `interrupted` early) and must stay a bare
    // exit-130 (invariant #2).
    println!("{}", crate::tui::welcome::GOODBYE);
    Ok(())
}

/// Best-effort terminal width in columns for banner/response-frame sizing
/// (fallback 100 when the size cannot be determined).
fn terminal_width() -> u16 {
    crossterm::terminal::size()
        .map(|(cols, _)| cols.max(60))
        .unwrap_or(100)
}

/// Builds a provider by name using the same registry resolution as startup:
/// the explicit name wins, and the model-level config is the fallback. Returns
/// an error when the name is unknown or cannot be constructed, so a mid-session
/// switch can keep the currently active provider (rollback).
fn resolve_provider(
    registry: &ProviderRegistry,
    config: Option<&HermesConfig>,
    name: &str,
    base_url_override: Option<&str>,
) -> Result<Box<dyn Provider>, RegistryError> {
    registry.select(Some(name), None, base_url_override, config)
}

/// Parses a non-negative integer argument from a slash command, e.g. the index
/// in `/pin 3`. Returns a usage error on a missing or non-numeric argument.
fn parse_index_arg(command: &str) -> Result<usize, anyhow::Error> {
    let raw = command.split_once(' ').map(|(_, r)| r.trim()).unwrap_or("");
    if raw.is_empty() {
        return Err(anyhow::anyhow!("usage: {command} needs a turn index"));
    }
    raw.parse::<usize>()
        .map_err(|_| anyhow::anyhow!("'{raw}' is not a valid turn index"))
}

/// The resolved window configuration for the active provider. Carries both the
/// effective limit (fed to the sliding window) and enough context to render a
/// human-readable `/info` line about compression status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResolvedContext {
    pub(crate) limit: Option<u64>,
    pub(crate) compression_enabled: bool,
    pub(crate) compression_target: Option<u64>,
}

/// Resolves the advisory context window for the active provider from config
/// precedence: `providers[<active>].context_length` first, then
/// `model.context_length`, then `compression.target_max_tokens` (only when
/// `compression.enabled` is explicitly true), then `None`. The active name may
/// be a declared provider or a model-level/`fake` entry; the latter falls back
/// to the model-level value or `None`. Pure and unit-testable.
pub(crate) fn resolve_context(config: Option<&HermesConfig>, active: &str) -> ResolvedContext {
    let compression = config.and_then(|c| c.compression.as_ref());
    // Compression is OFF by default (backward compatible): it only contributes
    // a limit when the user explicitly set `enabled: true`.
    let compression_enabled = compression.map(|c| c.enabled == Some(true)).unwrap_or(false);
    let compression_target = compression.and_then(|c| c.target_max_tokens);
    let provider_limit = config
        .and_then(|c| c.providers.get(active))
        .and_then(|p| p.context_length);
    let model_limit = config.and_then(|c| c.model.context_length);
    // Compression contributes a target only while enabled; otherwise the
    // window stays unset even if a target is present.
    let compression_limit = if compression_enabled { compression_target } else { None };
    let limit = provider_limit.or(model_limit).or(compression_limit);
    ResolvedContext {
        limit,
        compression_enabled,
        compression_target,
    }
}

/// Human-readable compression status for `/info` and the startup banner:
/// "off", "on (no target)", or "on (target ~N tokens)".
fn compression_label(ctx: &ResolvedContext) -> String {
    if !ctx.compression_enabled {
        return "off".to_owned();
    }
    match ctx.compression_target {
        Some(t) => format!("on (target ~{t} tokens)"),
        None => "on (no target)".to_owned(),
    }
}

/// A live MCP server plus enough metadata for the REPL to list and restart it.
struct McpHandle {
    name: String,
    config: McpServerConfig,
    /// The live server, or `None` after a failed start/restart.
    server: Option<McpServer>,
    /// Registry names (`{server}__{tool}`) this handle registered.
    tool_names: Vec<String>,
    /// A human-readable error from the most recent connect/restart attempt.
    error: Option<String>,
}
impl McpHandle {
    /// Spawns `name` from `cfg`, discovers its tools, and registers them into
    /// `registry` under `{server}__{tool}` names. Returns a handle; on failure
    /// the handle carries `server: None` and `error: Some(..)`.
    async fn connect<C: Confirmation + Clone + Send + Sync + 'static>(
        name: &str,
        cfg: McpServerConfig,
        confirmation: C,
        registry: &mut ToolRegistry,
    ) -> McpHandle {
        let server = match McpServer::spawn(name, cfg.clone()).await {
            Ok(s) => s,
            Err(e) => {
                return McpHandle {
                    name: name.to_owned(),
                    config: cfg,
                    server: None,
                    tool_names: Vec::new(),
                    error: Some(e.to_string()),
                };
            }
        };
        let mut handle = McpHandle {
            name: name.to_owned(),
            config: cfg,
            server: Some(server),
            tool_names: Vec::new(),
            error: None,
        };
        if let Err(e) = handle.discover_and_register(confirmation, registry).await {
            // Tool discovery failed but the child is up; drop it and mark down.
            if let Some(s) = handle.server.take() {
                s.shutdown().await;
            }
            handle.error = Some(e.to_string());
        }
        handle
    }

    /// Runs `tools/list` on the live server and registers each tool. Returns the
    /// number added, or an error.
    async fn discover_and_register<C: Confirmation + Clone + Send + Sync + 'static>(
        &mut self,
        confirmation: C,
        registry: &mut ToolRegistry,
    ) -> Result<(), String> {
        let Some(server) = &self.server else {
            return Err("no live server".into());
        };
        let descs = server
            .list_tools()
            .await
            .map_err(|e| format!("tool discovery failed: {e}"))?;
        let confirm = server.confirm;
        let mut added = Vec::new();
        for desc in &descs {
            let tool = McpTool::new(server, desc, confirm, confirmation.clone());
            if registry.get(tool.name()).is_some() {
                continue; // collision: leave existing tool; do not silently clobber
            }
            registry.register(tool);
            added.push(desc.hermes_name());
        }
        self.tool_names = added;
        Ok(())
    }

    /// Restarts this server: shuts the child down, unregisters its tools, then
    /// re-spawns and re-registers from the stored config.
    async fn restart<C: Confirmation + Clone + Send + Sync + 'static>(
        &mut self,
        confirmation: C,
        registry: &mut ToolRegistry,
    ) {
        // Drop the old child and remove its tools from the shared registry.
        if let Some(s) = self.server.take() {
            s.shutdown().await;
        }
        for tn in &self.tool_names {
            registry.unregister(tn);
        }
        self.tool_names.clear();
        self.error = None;
        // Re-connect.
        let name = self.name.clone();
        let cfg = self.config.clone();
        match McpServer::spawn(&name, cfg.clone()).await {
            Ok(server) => {
                self.server = Some(server);
                if let Err(e) = self.discover_and_register(confirmation, registry).await {
                    if let Some(s) = self.server.take() {
                        s.shutdown().await;
                    }
                    self.error = Some(e.to_string());
                }
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }
}

/// Prints every registered provider (sorted) with the active one marked. If the
/// active provider came from the model-level fallback and is not a registered
/// name, it is still printed so the marker is always present.
fn list_providers(registry: &ProviderRegistry, active: &str) {
    let names = registry.available();
    if !names.iter().any(|n| n == active) {
        println!("  {active} (active)");
    }
    for name in &names {
        let marker = if name == active { " (active)" } else { "" };
        println!("  {name}{marker}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hermes_core::config::CompressionConfig;
    use hermes_core::provider::{FAKE_PROVIDER, ProviderRegistry};

    /// Convenience: limit-only view of the resolved window for assertions.
    fn lim(config: Option<&HermesConfig>, active: &str) -> Option<u64> {
        resolve_context(config, active).limit
    }

    fn compression(enabled: bool, target: Option<u64>) -> CompressionConfig {
        CompressionConfig {
            enabled: Some(enabled),
            target_max_tokens: target,
        }
    }

    #[test]
    fn resolve_provider_resolves_the_builtin_fake_offline() {
        let registry = ProviderRegistry::offline();
        // Fake must resolve without any config, producing a usable provider.
        let _provider = resolve_provider(&registry, None, FAKE_PROVIDER, None)
            .expect("fake must resolve without config");
    }

    #[test]
    fn resolve_provider_rejects_an_unknown_name() {
        let registry = ProviderRegistry::offline();
        match resolve_provider(&registry, None, "does-not-exist", None) {
            Err(RegistryError::UnknownProvider { name, .. }) => {
                assert_eq!(name, "does-not-exist");
            }
            Ok(_) => panic!("unknown provider must not resolve"),
            Err(other) => panic!("expected UnknownProvider, got {other}"),
        }
    }

    #[test]
    fn offline_registry_lists_fake() {
        let registry = ProviderRegistry::offline();
        let available = registry.available();
        assert_eq!(available, vec![FAKE_PROVIDER.to_owned()]);
    }

    fn provider_config(limit: Option<u64>) -> hermes_core::config::ProviderConfig {
        hermes_core::config::ProviderConfig {
            api: Some("http://localhost:9/".into()),
            name: None,
            api_mode: None,
            key_env: None,
            models: std::collections::HashMap::new(),
            context_length: limit,
        }
    }

    #[test]
    fn context_limit_none_without_config() {
        assert_eq!(lim(None, "fake"), None);
        assert_eq!(lim(Some(&HermesConfig::default()), "fake"), None);
    }

    #[test]
    fn context_limit_uses_provider_config_first() {
        let mut config = HermesConfig::default();
        config.model.context_length = Some(200);
        config
            .providers
            .insert("b".to_owned(), provider_config(Some(4000)));
        // Declared provider uses its own context_length.
        assert_eq!(lim(Some(&config), "b"), Some(4000));
        // An undeclared active (e.g. model-level/fake) falls back to model.
        assert_eq!(lim(Some(&config), "fake"), Some(200));
    }

    #[test]
    fn context_limit_provider_absent_field_falls_back_to_model() {
        let mut config = HermesConfig::default();
        config.model.context_length = Some(300);
        config
            .providers
            .insert("b".to_owned(), provider_config(None));
        // Provider 'b' declared but no context_length -> model value.
        assert_eq!(lim(Some(&config), "b"), Some(300));
    }

    #[test]
    fn context_limit_is_none_when_nothing_configured() {
        let config = HermesConfig::default();
        assert_eq!(lim(Some(&config), "fake"), None);
        assert_eq!(lim(Some(&config), "b"), None);
    }

    #[test]
    fn compression_off_by_default_even_with_a_target() {
        let mut config = HermesConfig::default();
        // enabled absent (None) -> OFF regardless of target. Defaults when
        // `compression:` absent entirely.
        assert!(!resolve_context(Some(&config), "fake").compression_enabled);
        config.compression = Some(compression(false, Some(5000)));
        assert!(!resolve_context(Some(&config), "fake").compression_enabled);
        assert_eq!(lim(Some(&config), "fake"), None, "disabled compression must not trim");
    }

    #[test]
    fn compression_enabled_target_becomes_the_limit_when_nothing_else_set() {
        let config = HermesConfig {
            compression: Some(compression(true, Some(3000))),
            ..HermesConfig::default()
        };
        let ctx = resolve_context(Some(&config), "fake");
        assert!(ctx.compression_enabled);
        assert_eq!(ctx.limit, Some(3000));
        // Label reflects the target.
        assert_eq!(compression_label(&ctx), "on (target ~3000 tokens)");
    }

    #[test]
    fn compression_precedence_sits_below_provider_and_model() {
        // provider > model > compression: even if compression is enabled with a
        // target, provider/model context_length win.
        let mut config = HermesConfig::default();
        config.model.context_length = Some(2000);
        config.compression = Some(compression(true, Some(500)));
        config
            .providers
            .insert("b".to_owned(), provider_config(Some(4000)));
        assert_eq!(lim(Some(&config), "b"), Some(4000), "provider wins over compression");
        assert_eq!(lim(Some(&config), "fake"), Some(2000), "model wins over compression");
    }

    #[test]
    fn compression_enabled_without_target_leaves_window_unset() {
        let config = HermesConfig {
            compression: Some(compression(true, None)),
            ..HermesConfig::default()
        };
        let ctx = resolve_context(Some(&config), "fake");
        assert!(ctx.compression_enabled);
        assert_eq!(ctx.limit, None);
        assert_eq!(compression_label(&ctx), "on (no target)");
    }

    #[test]
    fn compression_label_is_off_when_disabled() {
        let ctx = resolve_context(Some(&HermesConfig::default()), "fake");
        assert_eq!(compression_label(&ctx), "off");
        assert_eq!(ctx.limit, None);
    }
}
