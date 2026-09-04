use crate::output::sanitize_untrusted_output;
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
        let session = store.resume(&id)?;
        let preview = session
            .turns
            .iter()
            .find_map(|turn| match turn {
                hermes_core::conversation::Turn::User { content } => Some(content.as_str()),
                _ => None,
            })
            .unwrap_or("(empty)");
        println!(
            "{id}  started={:.3}  {}",
            session.started_at,
            sanitize_untrusted_output(preview)
        );
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

pub fn inspect_session(store: &SessionStore, id: SessionId) -> Result<()> {
    let session = store.resume(&id)?;
    println!("Session: {}", session.id);
    println!("Source: {}", sanitize_untrusted_output(&session.source));
    println!("Started: {:.3}", session.started_at);
    println!("Turns: {}", session.turns.len());
    let tools = store.list_tool_call_details(&id)?;
    println!("Tool calls: {}", tools.len());
    Ok(())
}

pub fn show_messages(store: &SessionStore, id: SessionId) -> Result<()> {
    let session = store.resume(&id)?;
    for (index, turn) in session.turns.iter().enumerate() {
        let (role, content) = match turn {
            hermes_core::conversation::Turn::User { content } => ("user", content),
            hermes_core::conversation::Turn::Assistant { content } => ("assistant", content),
            hermes_core::conversation::Turn::Tool { name, content } => (name.as_str(), content),
        };
        println!(
            "[{}] {}: {}",
            index + 1,
            sanitize_untrusted_output(role),
            sanitize_untrusted_output(content)
        );
    }
    Ok(())
}

pub fn show_tool_calls(store: &SessionStore, id: SessionId) -> Result<()> {
    for call in store.list_tool_call_details(&id)? {
        println!(
            "{} [{}] {} args={} result={}",
            call.id,
            call.status,
            sanitize_untrusted_output(&call.tool_name),
            sanitize_untrusted_output(&call.arguments),
            sanitize_untrusted_output(&call.result)
        );
    }
    Ok(())
}

pub fn search_sessions(store: &SessionStore, query: &str) -> Result<()> {
    let limits = hermes_core::search::SearchLimits::default();
    // Explicit rebuild keeps the external-content index deterministic; this writes only derived FTS state.
    store.repair_search_index()?;
    let results = match store.search_messages(query, None, limits) {
        Ok(results) => results,
        Err(hermes_core::search::SearchError::IndexNotReady) => {
            store.repair_search_index()?;
            store.search_messages(query, None, limits)?
        }
        Err(error) => return Err(anyhow::anyhow!(error)),
    };
    if results.is_empty() {
        println!("No search results.");
        return Ok(());
    }
    println!(
        "Search results for: {}",
        hermes_core::search::redact::redact_credentials(&sanitize_untrusted_output(query))
    );
    for (index, result) in results.iter().enumerate() {
        println!(
            "[{}] session={} message={} role={} rank={:.3}",
            index + 1,
            result.session_id,
            result.message_id,
            sanitize_untrusted_output(&result.role),
            result.rank
        );
        let safe = sanitize_untrusted_output(&result.snippet);
        println!(
            "  {}",
            hermes_core::search::redact::redact_credentials(&safe)
        );
    }
    Ok(())
}
