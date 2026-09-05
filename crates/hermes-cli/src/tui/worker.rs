//! Spec 012 — worker side of the TUI (Ticket 03: real agentic wiring).
//!
//! This is the **real** agentic worker: it owns a [`ConversationRunner`] over
//! the actual configured provider, a [`SessionStore`], and the tool registry,
//! and drives each submitted prompt through the shared `chat_agentic` engine
//! (Spec 009 + Spec 012 observer). There is deliberately **no second agentic
//! loop**: the same single `chat_agentic` used by the readline REPL produces
//! the events; this worker only observes them through the core [`AgentEvent`]
//! channel, sanitizes at the boundary, and forwards [`TuiEvent`]s to the
//! renderer.
//!
//! Sanitization/redaction happens **here**, at the CLI boundary — never in
//! hermes-core.
//!
//! Concurrency note: `SessionStore` wraps a `rusqlite` connection, which is
//! **not `Send`**, so this worker is deliberately *not* `tokio::spawn`ed (the
//! REPL has the same constraint — it runs on the main future). [`run_tui`]
//! drives it inline and `select!`s against the blocking renderer instead.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use hermes_core::config::HermesConfig;
use hermes_core::conversation::{AgentEvent, AgenticResult, ConversationRunner};
use hermes_core::provider::{Provider, ProviderError};
use hermes_core::session::{SessionId, SessionStore};
use hermes_core::tools::{
    Confirmation, ListDirTool, ReadFileTool, ShellReadonlyTool, ToolRegistry, WriteFileTool,
};
use tokio_util::sync::CancellationToken;

use super::channel::{EventQueue, TuiCommand};
use super::event::TuiEvent;
use crate::repl::resolve_context;

/// Scrubs a raw text field for display: strips ANSI/control sequences, then
/// redacts credentials. Applied to every model/tool text at the CLI boundary —
/// the renderer never sanitizes.
fn scrub(text: &str) -> String {
    super::event::redact(&crate::output::sanitize_untrusted_output(text))
}

/// Maps a raw core [`AgentEvent`] to a display [`TuiEvent`], sanitizing and
/// redacting all text at this boundary. Pure + unit-testable.
pub(crate) fn agent_event_to_tui(event: AgentEvent) -> TuiEvent {
    match event {
        AgentEvent::Chunk { text } => TuiEvent::Chunk(scrub(&text)),
        AgentEvent::ToolStarted { name, arguments } => TuiEvent::tool_started(name, &arguments),
        AgentEvent::ToolDone {
            name,
            status,
            result: _result,
        } => TuiEvent::ToolDone {
            name,
            status: status.as_str().to_owned(),
        },
        AgentEvent::Iteration { current, max: _max } => TuiEvent::Iteration(current),
        AgentEvent::StatusChanged {
            goal_status,
            plan_active,
            reflection_on,
        } => TuiEvent::StatusMeta {
            goal_status: goal_status
                .map(|g| g.as_str().to_owned())
                .unwrap_or_else(|| "NotStarted".to_owned()),
            plan_active,
            reflection_on,
        },
        AgentEvent::TokenTick { estimate, limit } => TuiEvent::TokenTick { estimate, limit },
        AgentEvent::Done { text } => TuiEvent::Done(scrub(&text)),
        AgentEvent::Error { message } => TuiEvent::Notice(scrub(&message)),
    }
}

/// A confirmation sink for the TUI that always denies interactive actions
/// (write file / shell-exec). Interactive confirmation + rich input bar arrive
/// with the input/history work in later tickets; denying by default is the safe
/// choice — no destructive action happens without a user-affirmed flow.
#[derive(Clone)]
struct DenyConfirmation;

#[async_trait::async_trait]
impl Confirmation for DenyConfirmation {
    async fn confirm(&self, _prompt: &str) -> bool {
        false
    }
}

/// Everything the worker needs to drive turns for one live session. Kept as a
/// unit so construction and the run loop stay tidy.
struct AgentRuntime {
    store: SessionStore,
    session_id: SessionId,
    runner: ConversationRunner<Box<dyn Provider>>,
    registry: ToolRegistry,
    provider_name: String,
}

/// Builds the runtime (store + session + runner + safe tool set). On failure it
/// posts a `Notice` and returns `None` so the caller can stay alive and drain
/// commands until the renderer quits.
fn build_runtime(
    queue: &EventQueue,
    home: &std::path::Path,
    provider: Box<dyn Provider>,
    provider_name: String,
    config: Option<HermesConfig>,
) -> Option<AgentRuntime> {
    let db = home.join("state.db");
    let store = match SessionStore::open(&db) {
        Ok(s) => s,
        Err(e) => {
            queue.push(TuiEvent::Notice(format!("failed to open state.db: {e}")));
            return None;
        }
    };
    let session_id = match store.list() {
        Ok(ids) => ids.last().copied(),
        Err(_) => None,
    };
    let session_id = match session_id {
        Some(id) => id,
        None => match store.create_session("tui") {
            Ok(id) => id,
            Err(e) => {
                queue.push(TuiEvent::Notice(format!("failed to create session: {e}")));
                return None;
            }
        },
    };
    let existing = store.resume(&session_id).map(|r| r.turns).unwrap_or_default();
    let mut runner = ConversationRunner::from_turns(provider, existing);
    let ctx = resolve_context(config.as_ref(), &provider_name);
    runner.set_context_limit(ctx.limit);

    // Safe tool set (Spec 002/011 constructors, reused as the REPL does).
    let root = std::env::current_dir().unwrap_or_default();
    let confirm = DenyConfirmation;
    let mut registry = ToolRegistry::new();
    registry.register(ReadFileTool::new(&root));
    registry.register(ListDirTool::new(&root));
    registry.register(ShellReadonlyTool::new(confirm.clone(), Duration::from_secs(30)));
    registry.register(WriteFileTool::new(&root, confirm));

    Some(AgentRuntime {
        store,
        session_id,
        runner,
        registry,
        provider_name,
    })
}

/// Real agentic worker. Never returns early: on init failure it posts a notice
/// and simply drains commands until the renderer quits (channel closed), so the
/// terminal is always left by the renderer, never orphaned here.
pub async fn run_agent(
    queue: Arc<EventQueue>,
    mut cmds: tokio::sync::mpsc::UnboundedReceiver<TuiCommand>,
    home: PathBuf,
    provider: Box<dyn Provider>,
    provider_name: String,
    config: Option<HermesConfig>,
) {
    match build_runtime(&queue, &home, provider, provider_name, config) {
        Some(mut rt) => run_loop(&queue, &mut cmds, &mut rt).await,
        None => {
            while cmds.recv().await.is_some() {
                // drain until the renderer drops its sender
            }
        }
    }
}

async fn run_loop(
    queue: &Arc<EventQueue>,
    cmds: &mut tokio::sync::mpsc::UnboundedReceiver<TuiCommand>,
    rt: &mut AgentRuntime,
) {
    // Initial status so the header is populated.
    queue.push(TuiEvent::StatusChanged {
        session_id: format!("{}", rt.session_id),
        provider: rt.provider_name.clone(),
    });
    queue.push(TuiEvent::StatusMeta {
        goal_status: "NotStarted".to_owned(),
        plan_active: false,
        reflection_on: false,
    });
    queue.push(TuiEvent::TokenTick {
        estimate: rt.runner.estimated_tokens(),
        limit: rt.runner.context_limit(),
    });
    // Ticket 04: replay the session's persisted tool calls so the tool log is
    // populated when a user resumes an older session.
    if let Ok(records) = rt.store.list_tool_calls(&rt.session_id) {
        if !records.is_empty() {
            queue.push(TuiEvent::Notice(format!(
                "resumed session with {} prior tool call(s)",
                records.len()
            )));
            for (name, status) in records {
                queue.push(TuiEvent::ToolDone {
                    name,
                    status: status.as_str().to_owned(),
                });
            }
        }
    }
    queue.push(TuiEvent::Notice(
        "Hermes-RS TUI live — type a message and press Enter (q to quit).".to_owned(),
    ));

    while let Some(TuiCommand::Line(prompt)) = cmds.recv().await {
        // Fresh observer channel for this turn. A forwarder task drains it live
        // so streaming chunks/tool events appear in real time rather than at
        // the end. The forwarder only carries `AgentEvent`s + the queue Arc
        // (both `Send`), so it is safe to `tokio::spawn` here even though the
        // non-`Send` `SessionStore` stays on this task.
        let (agent_tx, mut agent_rx) = tokio::sync::mpsc::channel::<AgentEvent>(256);
        rt.runner.set_observer(agent_tx);

        let before = rt.runner.turns().len();
        let cancel = CancellationToken::new();

        let fwd_queue = Arc::clone(queue);
        let forwarder = tokio::spawn(async move {
            while let Some(ev) = agent_rx.recv().await {
                fwd_queue.push(agent_event_to_tui(ev));
            }
        });

        // Run the shared agentic engine (single source of truth).
        let result = rt
            .runner
            .chat_agentic(
                prompt,
                &rt.registry,
                Some((&rt.store, &rt.session_id)),
                10,
                cancel.clone(),
            )
            .await;

        // Let the forwarder flush the tail events, then stop it.
        tokio::time::sleep(Duration::from_millis(10)).await;
        forwarder.abort();

        match &result {
            Ok(AgenticResult::Done { .. }) => {}
            Ok(AgenticResult::MaxIterations(limit)) => {
                queue.push(TuiEvent::MaxIterations(*limit));
            }
            Ok(AgenticResult::Blocked { reason }) => {
                queue.push(TuiEvent::Blocked(crate::output::sanitize_untrusted_output(reason)));
            }
            Ok(AgenticResult::Cancelled) => {
                queue.push(TuiEvent::Notice("interrupted".to_owned()));
            }
            Err(ProviderError::Cancelled) => {
                queue.push(TuiEvent::Notice("interrupted".to_owned()));
            }
            Err(e) => {
                queue.push(TuiEvent::Notice(crate::output::sanitize_untrusted_output(&e.to_string())));
            }
        }
        // Persist any turns the engine produced this turn.
        for turn in &rt.runner.turns()[before..] {
            let _ = rt.store.save_turn(&rt.session_id, turn);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::App;
    use hermes_core::conversation::goal::GoalStatus;
    use hermes_core::tools::ToolExecutionStatus;

    #[test]
    fn maps_streaming_chunk_with_sanitization() {
        let ev = agent_event_to_tui(AgentEvent::Chunk {
            text: "hello\x1b[31mred\x1b[0m\x07".to_owned(),
        });
        assert_eq!(ev, TuiEvent::Chunk("hellored".to_owned()));
    }

    #[test]
    fn maps_tool_events_with_redaction() {
        let started = agent_event_to_tui(AgentEvent::ToolStarted {
            name: "read_file".to_owned(),
            arguments: "{\"token\":\"sk-proj-abcdefghijklmno\"}".to_owned(),
        });
        match started {
            TuiEvent::ToolStarted { name, arguments } => {
                assert_eq!(name, "read_file");
                assert!(!arguments.contains("abcdefghijklmno"), "secret leaked: {arguments}");
            }
            other => panic!("expected ToolStarted, got {other:?}"),
        }
        let done = agent_event_to_tui(AgentEvent::ToolDone {
            name: "read_file".to_owned(),
            status: ToolExecutionStatus::Success,
            result: "some content".to_owned(),
        });
        assert_eq!(
            done,
            TuiEvent::ToolDone {
                name: "read_file".to_owned(),
                status: "success".to_owned(),
            }
        );
    }

    #[test]
    fn maps_status_and_token_events() {
        let meta = agent_event_to_tui(AgentEvent::StatusChanged {
            goal_status: Some(GoalStatus::InProgress),
            plan_active: true,
            reflection_on: false,
        });
        assert_eq!(
            meta,
            TuiEvent::StatusMeta {
                goal_status: "in progress".to_owned(),
                plan_active: true,
                reflection_on: false,
            }
        );
        let tick = agent_event_to_tui(AgentEvent::TokenTick {
            estimate: 42,
            limit: Some(1000),
        });
        assert_eq!(
            tick,
            TuiEvent::TokenTick {
                estimate: 42,
                limit: Some(1000),
            }
        );
    }

    #[test]
    fn maps_iteration_and_final() {
        let it = agent_event_to_tui(AgentEvent::Iteration { current: 3, max: 10 });
        assert_eq!(it, TuiEvent::Iteration(3));
        // Final answer is carried by `Done` (not re-emitted as a streaming chunk).
        let done = agent_event_to_tui(AgentEvent::Done {
            text: "final answer\x1b[0m".to_owned(),
        });
        assert_eq!(done, TuiEvent::Done("final answer".to_owned()));
    }

    /// End-to-end redaction: a secret injected at the core boundary must not
    /// reach the rendered terminal buffer (rendered headlessly).
    #[test]
    fn injected_credential_never_reaches_the_panel() {
        let secret = "sk-proj-supersecret1234567890";
        let started = agent_event_to_tui(AgentEvent::ToolStarted {
            name: "write_file".to_owned(),
            arguments: format!("{{\"token\":\"{secret}\"}}"),
        });
        let mut app = App::default();
        app.apply(started);
        app.apply(TuiEvent::ToolDone {
            name: "write_file".to_owned(),
            status: "success".to_owned(),
        });
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| app.render(frame)).unwrap();
        let buffer = terminal.backend().buffer();
        let rendered: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(
            !rendered.contains("supersecret1234567890"),
            "secret leaked into the rendered panel"
        );
    }
}
