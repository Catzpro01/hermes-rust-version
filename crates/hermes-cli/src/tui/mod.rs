//! Spec 012 — TUI dashboard event model & boundary (Ticket 01).
//!
//! The agentic worker (running a turn) communicates with the Ratatui renderer
//! exclusively through [`TuiEvent`] on a bounded channel. By convention every
//! `String` payload is **pre-sanitized and pre-redacted at the source** (the
//! same `sanitize_untrusted_output` + `redact_credentials` the readline REPL
//! uses) before it is put on the channel. The renderer never sees raw model or
//! tool content, so there is no second path that could forget to sanitize.

/// A display event emitted by the agentic worker toward the TUI renderer.
/// Every text payload is already sanitized/redacted at the source.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // variants are constructed by the renderer workers (Ticket 02+)
pub enum TuiEvent {
    /// Conversation-level status changed (session id, provider name).
    StatusChanged {
        session_id: String,
        provider: String,
    },
    /// Advisory token accounting changed (estimate / configured limit).
    TokenTick {
        estimate: usize,
        limit: Option<u64>,
    },
    /// A chunk of assistant text streamed (pre-sanitized).
    Chunk(String),
    /// A tool call was issued (arguments pre-sanitized/redacted/trimmed).
    ToolStarted {
        name: String,
        arguments: String,
    },
    /// A tool call finished with the given status string.
    ToolDone {
        name: String,
        status: String,
    },
    /// Iteration counter advanced within an agentic turn.
    Iteration(usize),
    /// The turn produced a final (tool-free) assistant answer.
    Done(String),
    /// A non-fatal message to surface (pre-sanitized).
    Notice(String),
    /// The turn stopped because it hit the iteration budget.
    MaxIterations(usize),
    /// The goal is blocked (reason pre-sanitized).
    Blocked(String),
}

impl TuiEvent {
    /// Constructs a pre-sanitized `Chunk`.
    #[allow(dead_code)] // used by the renderer workers (Ticket 02+)
    pub fn sanitized_chunk(raw: &str) -> Self {
        TuiEvent::Chunk(crate::output::sanitize_untrusted_output(raw))
    }
    /// Constructs a `ToolStarted` with sanitized + redacted arguments.
    #[allow(dead_code)] // used by the renderer workers (Ticket 02+)
    pub fn tool_started(name: impl Into<String>, arguments: &str) -> Self {
        let safe = redact(&crate::output::sanitize_untrusted_output(arguments));
        let safe: String = safe.chars().take(120).collect();
        TuiEvent::ToolStarted {
            name: name.into(),
            arguments: safe,
        }
    }
}

/// Redacts credentials in untrusted content (mirrors the REPL render path).
#[allow(dead_code)] // used by the renderer panels (Ticket 02+)
pub(crate) fn redact(input: &str) -> String {
    hermes_core::search::redact::redact_credentials(input)
}

/// Entry point for the TUI dashboard (Spec 012). Ticket 01 only establishes the
/// event model and the entry point signature; the Ratatui renderer loop and
/// panels are added in later tickets. The gate on an interactive terminal is
/// enforced in `main` before this is reached.
pub async fn run_tui() -> anyhow::Result<()> {
    eprintln!("TUI dashboard not wired yet (Spec 012 Ticket 02+)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitized_chunk_strips_ansi_and_controls() {
        let ev = TuiEvent::sanitized_chunk("hi\x1b[31mred\x1b[0m\x07plain");
        assert_eq!(ev, TuiEvent::Chunk("hiredplain".to_owned()));
    }

    #[test]
    fn tool_started_redacts_credentials_and_truncates() {
        // Redaction keeps a recognizable prefix (e.g. `sk-proj-`) but removes
        // the secret value that follows it.
        let long = "{\"path\":\"/x\",\"token\":\"sk-proj-abcdefghijklmno\"}".to_owned();
        let ev = TuiEvent::tool_started("read_file", &long);
        match ev {
            TuiEvent::ToolStarted { name, arguments } => {
                assert_eq!(name, "read_file");
                assert!(
                    !arguments.contains("abcdefghijklmno"),
                    "secret value leaked: {arguments}"
                );
                assert!(arguments.len() <= 120);
            }
            other => panic!("expected ToolStarted, got {other:?}"),
        }
    }

    #[test]
    fn enum_is_debug_and_eq() {
        // Ensures variants used by the renderer are comparable (for tests).
        let a = TuiEvent::Iteration(2);
        let b = TuiEvent::Iteration(2);
        assert_eq!(a, b);
        assert_ne!(a, TuiEvent::Iteration(3));
        // All variants exist (smoke construction).
        let _ = [
            TuiEvent::StatusChanged { session_id: "s".into(), provider: "p".into() },
            TuiEvent::TokenTick { estimate: 10, limit: Some(100) },
            TuiEvent::Chunk("c".into()),
            TuiEvent::ToolStarted { name: "t".into(), arguments: "".into() },
            TuiEvent::ToolDone { name: "t".into(), status: "success".into() },
            TuiEvent::Iteration(1),
            TuiEvent::Done("d".into()),
            TuiEvent::Notice("n".into()),
            TuiEvent::MaxIterations(10),
            TuiEvent::Blocked("r".into()),
        ];
    }
}
