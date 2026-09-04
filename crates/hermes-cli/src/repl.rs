use crate::{
    render::render_stream,
    session_menu::{list_sessions, parse_resume, select_session},
};
use anyhow::{Context, Result};
use hermes_core::{conversation::ConversationRunner, provider::Provider, session::SessionStore};
use rustyline::{error::ReadlineError, DefaultEditor};
use std::io::IsTerminal;
use std::sync::{Arc, Mutex};
#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};
use tokio_util::sync::CancellationToken;

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
                let stream = runner
                    .chat_with_cancel(input.to_owned(), turn_cancel.clone())
                    .await
                    .map_err(|error| anyhow::anyhow!(error.to_string()))?;
                #[cfg(unix)]
                let result = tokio::select! {
                    _ = sigint.recv() => {
                        turn_cancel.cancel();
                        runner.discard_pending_user();
                        println!("\n⚡ interrupted");
                        return Err(anyhow::anyhow!("interrupted"));
                    }
                    result = render_stream(stream) => result,
                };
                #[cfg(not(unix))]
                let result = render_stream(stream).await;
                match result {
                    Ok(response) => {
                        runner.push_assistant(response);
                        for turn in &runner.turns()[before..] {
                            store.save_turn(&session_id, turn)?;
                        }
                    }
                    Err(error) if error.to_string().contains("cancelled") => {
                        runner.discard_pending_user();
                        println!("\n⚡ cancelled");
                        return Err(anyhow::anyhow!("interrupted"));
                    }
                    Err(error) => {
                        runner.discard_pending_user();
                        return Err(error);
                    }
                }
            }
        }
    }
    Ok(())
}
