use crate::{
    render::render_events,
    session_menu::{list_sessions, parse_resume, select_session},
};
use anyhow::{Context, Result};
use hermes_core::{
    conversation::ConversationRunner, provider::FakeProvider, session::SessionStore,
};
use rustyline::{error::ReadlineError, DefaultEditor};

pub async fn run_repl(home: &std::path::Path) -> Result<()> {
    let db = home.join("state.db");
    let mut store = SessionStore::open(&db).context("open Hermes state.db")?;
    let mut editor = DefaultEditor::new().context("create terminal editor")?;
    let session_id = select_session(&store, &mut editor)?;
    let existing = store.resume(&session_id)?.turns;
    let mut runner = ConversationRunner::from_turns(FakeProvider, existing);
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
                println!("New session {id}");
                continue;
            }
            command if command.starts_with("/resume ") => {
                let id = parse_resume(command)?;
                let history = store.resume(&id)?.turns;
                runner = ConversationRunner::from_turns(FakeProvider, history);
                println!("Resumed {id}");
                continue;
            }
            _ => {
                let before = runner.turns().len();
                let events = runner
                    .prompt(input.to_owned())
                    .await
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                render_events(&events)?;
                for turn in &runner.turns()[before..] {
                    store.save_turn(&session_id, turn)?;
                }
            }
        }
    }
    Ok(())
}
