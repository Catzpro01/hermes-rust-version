use crate::session_menu::{
    inspect_session, list_sessions, parse_resume, search_sessions, select_session, show_messages,
    show_tool_calls,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use hermes_core::{
    config::HermesConfig,
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
    println!("Hermes-RS session {session_id} (provider {provider_name})");
    println!("Commands: /provider [name], /new, /sessions, /inspect <id>, /messages <id>, /tool-calls <id>, /search <query>, /resume <id>, /exit");
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
    use hermes_core::provider::{FAKE_PROVIDER, ProviderRegistry};

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
}
