use crate::session_menu::{list_sessions, parse_resume, select_session};
use anyhow::{Context, Result};
use async_trait::async_trait;
use hermes_core::{
    conversation::{AgenticResult, ConversationRunner},
    provider::{Provider, ProviderError},
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
use tokio_util::sync::CancellationToken;

struct CliConfirmation;
#[async_trait]
impl Confirmation for CliConfirmation {
    async fn confirm(&self, prompt: &str) -> bool {
        eprint!("\n⚠ [Tool] {prompt} ");
        let _ = std::io::Write::flush(&mut std::io::stderr());
        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            return false;
        }
        matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes")
    }
}

pub async fn run_repl(
    home: &std::path::Path,
    provider: Box<dyn Provider>,
    resume: bool,
) -> Result<()> {
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
    let mut tool_registry = ToolRegistry::new();
    let tool_root = std::env::current_dir().context("resolve CLI tool root")?;
    tool_registry.register(ReadFileTool::new(&tool_root));
    tool_registry.register(ListDirTool::new(&tool_root));
    tool_registry.register(ShellReadonlyTool::new(
        CliConfirmation,
        Duration::from_secs(30),
    ));
    tool_registry.register(WriteFileTool::new(&tool_root, CliConfirmation));
    println!("Hermes-RS session {session_id}");
    println!("Commands: /new, /sessions, /resume <id>, /exit");
    let editor = Arc::new(Mutex::new(editor));
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
