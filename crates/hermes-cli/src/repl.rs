use crate::{
    render::render_stream,
    session_menu::{list_sessions, parse_resume, select_session},
};
use anyhow::{Context, Result};
use hermes_core::{conversation::ConversationRunner, provider::Provider, session::SessionStore};
use rustyline::{error::ReadlineError, DefaultEditor};
use tokio_util::sync::CancellationToken;

pub async fn run_repl(home: &std::path::Path, provider: Box<dyn Provider>) -> Result<()> {
    let db = home.join("state.db");
    let mut store = SessionStore::open(&db).context("open Hermes state.db")?;
    let mut editor = DefaultEditor::new().context("create terminal editor")?;
    let mut session_id = select_session(&store, &mut editor)?;
    let existing = store.resume(&session_id)?.turns;
    let mut runner = ConversationRunner::from_turns(provider, existing);
    println!("Hermes-RS session {session_id}");
    println!("Commands: /new, /sessions, /resume <id>, /exit");
    loop {
        let line = match editor.readline("hermes> ") {
            Ok(line) => {
                if !line.trim().is_empty() {
                    let _ = editor.add_history_entry(line.as_str());
                }
                line
            }
            Err(ReadlineError::Eof) | Err(ReadlineError::Interrupted) => {
                println!();
                break;
            }
            Err(error) => return Err(anyhow::anyhow!(error.to_string())),
        };
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
                let cancel = CancellationToken::new();
                let signal_cancel = cancel.clone();
                let signal_task = tokio::spawn(async move {
                    if tokio::signal::ctrl_c().await.is_ok() {
                        signal_cancel.cancel();
                    }
                });
                let result = runner.chat_with_cancel(input.to_owned(), cancel).await;
                let result = match result {
                    Ok(stream) => render_stream(stream).await,
                    Err(error) => Err(anyhow::anyhow!(error.to_string())),
                };
                signal_task.abort();
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
