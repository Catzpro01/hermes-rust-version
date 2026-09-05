//! Spec 012 — UI-agnostic agentic event stream (core observer).
//!
//! The agentic loop in [`super::ConversationRunner::chat_agentic`] owns every
//! live datum the TUI needs (streaming text, tool progress, token accounting,
//! goal/plan/reflection state) but historically only surfaced a final
//! [`super::AgenticResult`]. To let a *front end* (TUI, a future web UI, or a
//! headless test) observe that stream **without coupling hermes-core to any
//! UI crate**, the runner holds an optional observer sender. When set, it emits
//! [`AgentEvent`]s at defined points. When unset (the REPL and all existing
//! callers), [`super::ConversationRunner::emit`] is a no-op — zero regression.
//!
//! These are *domain* events: raw model/tool text flows through them. Sanitizing
//! and redacting happens in the consumer (CLI render boundary), never here.

use super::goal::GoalStatus;
use crate::tools::ToolExecutionStatus;

/// A domain event produced live by the agentic loop.
///
/// UI-agnostic by design: the TUI layer maps each variant onto its own display
/// event, applying sanitize + redact at that boundary.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// A streaming text chunk from the model (raw, not yet sanitized).
    Chunk { text: String },
    /// A tool call is about to execute.
    ToolStarted { name: String, arguments: String },
    /// A tool call completed.
    ToolDone {
        name: String,
        status: ToolExecutionStatus,
        result: String,
    },
    /// Iteration progress within a single agentic turn.
    Iteration { current: usize, max: usize },
    /// Goal/plan/reflection status changed.
    StatusChanged {
        goal_status: Option<GoalStatus>,
        plan_active: bool,
        reflection_on: bool,
    },
    /// Token accounting refreshed.
    TokenTick { estimate: usize, limit: Option<u64> },
    /// The turn produced a final, tool-free assistant answer.
    Done { text: String },
    /// A non-fatal error/message to surface.
    Error { message: String },
}
