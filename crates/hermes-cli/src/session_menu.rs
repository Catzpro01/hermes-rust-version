use anyhow::{bail, Context, Result};
use hermes_core::session::{SessionId, SessionStore};
use rustyline::{error::ReadlineError, DefaultEditor};

pub fn select_session(store: &SessionStore, editor: &mut DefaultEditor) -> Result<SessionId> {
    let sessions = store.list()?;
    if sessions.is_empty() {
        return Ok(store.create_session("cli")?);
    }
    println!("Sessions:");
    for (index, id) in sessions.iter().enumerate() {
        println!("  {}. {}", index + 1, id);
    }
    println!("  n. New session");
    let input = editor
        .readline("select> ")
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    if input.trim().eq_ignore_ascii_case("n") || input.trim().is_empty() {
        return Ok(store.create_session("cli")?);
    }
    let index: usize = input
        .trim()
        .parse()
        .context("enter a session number or n")?;
    sessions
        .get(index.saturating_sub(1))
        .copied()
        .context("session number out of range")
}

pub fn list_sessions(store: &SessionStore) -> Result<()> {
    let sessions = store.list()?;
    if sessions.is_empty() {
        println!("No sessions.");
        return Ok(());
    }
    for id in sessions {
        let history = store.resume(&id)?.turns;
        let preview = history
            .iter()
            .find_map(|turn| match turn {
                hermes_core::conversation::Turn::User { content } => Some(content.as_str()),
                _ => None,
            })
            .unwrap_or("(empty)");
        println!("{id}  {preview}");
    }
    Ok(())
}

pub fn parse_resume(input: &str) -> Result<SessionId> {
    input
        .split_once(' ')
        .map(|(_, id)| id.trim())
        .filter(|id| !id.is_empty())
        .context("usage: /resume <session-id>")
        .and_then(|id| id.parse::<SessionId>().map_err(|e| anyhow::anyhow!(e)))
}

#[allow(dead_code)]
fn _readline_error(e: ReadlineError) -> anyhow::Error {
    anyhow::anyhow!(e.to_string())
}
#[allow(dead_code)]
fn _bail() -> Result<()> {
    bail!("invalid session")
}
