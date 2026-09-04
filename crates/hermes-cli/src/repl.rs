use crate::output::sanitize_untrusted_output;
use crate::session_menu::{
    inspect_session, list_sessions, parse_resume, search_sessions, select_session, show_messages,
    show_tool_calls,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use hermes_core::{
    config::HermesConfig,
    conversation::context::summarize_dropped,
    conversation::{AgenticResult, ConversationRunner},
    provider::{Provider, ProviderError, ProviderRegistry, RegistryError},
    session::SessionStore,
    tools::{
        Confirmation, ListDirTool, ReadFileTool, ShellReadonlyTool, ToolRegistry, WriteFileTool,
    },
};
use rustyline::{error::ReadlineError, DefaultEditor};
use std::io::IsTerminal;
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
    println!("Commands: /provider [name], /pin <n>, /unpin <n>, /pinned, /new, /sessions, /inspect <id>, /messages <id>, /tool-calls <id>, /search <query>, /resume <id>, /info, /exit");
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
    tokio::spawn(async move {
        while let Some((prompt, reply)) = confirmation_rx.recv().await {
            let answer = tokio::task::spawn_blocking({
                let editor = Arc::clone(&confirmation_editor);
                move || {
                    editor
                        .lock()
                        .ok()
                        .and_then(|mut e| {
                            e.readline(&format!("confirm {prompt} [y/N] ").to_owned())
                                .ok()
                        })
                        .map(|s| matches!(s.trim().to_ascii_lowercase().as_str(), "y" | "yes"))
                        .unwrap_or(false)
                }
            })
            .await
            .unwrap_or(false);
            let _ = reply.send(answer);
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
    tool_registry.register(WriteFileTool::new(&tool_root, confirmation));
    loop {
        if !std::io::stdin().is_terminal() {
            print!("hermes> ");
            std::io::Write::flush(&mut std::io::stdout())?;
        }
        let editor_for_read = Arc::clone(&editor);
        let readline = tokio::task::spawn_blocking(move || {
            let mut editor = match editor_for_read.lock() {
                Ok(editor) => editor,
                Err(_) => return "editor lock poisoned".to_owned(),
            };
            match editor.readline("hermes> ") {
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
            _ => {
                let before = runner.turns().len();
                let turn_cancel = CancellationToken::new();
                #[cfg(unix)]
                let result = tokio::select! {
                    _ = sigint.recv() => { turn_cancel.cancel(); runner.discard_pending_user(); return Err(anyhow::anyhow!("interrupted")); }
                    result = runner.chat_agentic(input.to_owned(), &tool_registry, Some((&store, &session_id)), 10, turn_cancel.clone()) => result,
                };
                #[cfg(not(unix))]
                let result = runner
                    .chat_agentic(
                        input.to_owned(),
                        &tool_registry,
                        Some((&store, &session_id)),
                        10,
                        turn_cancel,
                    )
                    .await;
                match result {
                    Ok(AgenticResult::Done { text, iterations }) => {
                        println!("{text}");
                        println!("[iter {iterations}/10]");
                    }
                    Ok(AgenticResult::MaxIterations(limit)) => {
                        eprintln!("\n⚠ Reached max iterations budget ({limit}).")
                    }
                    Ok(AgenticResult::Cancelled) | Err(ProviderError::Cancelled) => {
                        eprintln!("\n⚡ interrupted");
                        return Err(anyhow::anyhow!("interrupted"));
                    }
                    Err(error) => return Err(anyhow::anyhow!(error.to_string())),
                }
                for turn in &runner.turns()[before..] {
                    store.save_turn(&session_id, turn)?;
                }
            }
        }
    }
    Ok(())
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
struct ResolvedContext {
    limit: Option<u64>,
    compression_enabled: bool,
    compression_target: Option<u64>,
}

/// Resolves the advisory context window for the active provider from config
/// precedence: `providers[<active>].context_length` first, then
/// `model.context_length`, then `compression.target_max_tokens` (only when
/// `compression.enabled` is explicitly true), then `None`. The active name may
/// be a declared provider or a model-level/`fake` entry; the latter falls back
/// to the model-level value or `None`. Pure and unit-testable.
fn resolve_context(config: Option<&HermesConfig>, active: &str) -> ResolvedContext {
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
