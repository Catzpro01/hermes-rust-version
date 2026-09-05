//! Spec 012 — headless end-to-end checks (Ticket 05 closure).
//!
//! These simulate a full agentic session through the **real** pipeline — raw
//! [`AgentEvent`] from the core observer → [`worker::agent_event_to_tui`]
//! (sanitize/redact at the CLI boundary) → [`App`] state → render into a
//! [`ratatui::backend::TestBackend`] — then assert on the final terminal buffer.
//! They are the sandbox-safe replacement for interactive `--tui` testing.

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use hermes_core::conversation::AgentEvent;
use hermes_core::conversation::goal::GoalStatus;
use hermes_core::tools::ToolExecutionStatus;

use super::app::{App, KeyAction};
use super::worker;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Runs a sequence of raw core events through the real mapping + App and
/// returns both the App (for state asserts) and the rendered buffer text.
fn render_events(events: &[AgentEvent]) -> (App, String) {
    let mut app = App::default();
    for ev in events {
        app.apply(worker::agent_event_to_tui(ev.clone()));
    }
    let text = render_app(&app);
    (app, text)
}

fn render_app(app: &App) -> String {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.render(frame)).unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

/// A representative full agentic turn: header state, streaming chunks, a tool
/// round, and a final tool-free answer.
fn full_turn_events() -> Vec<AgentEvent> {
    vec![
        AgentEvent::TokenTick {
            estimate: 500,
            limit: Some(1000),
        },
        AgentEvent::StatusChanged {
            goal_status: Some(GoalStatus::InProgress),
            plan_active: true,
            reflection_on: true,
        },
        AgentEvent::Iteration { current: 1, max: 10 },
        // Streaming first half (draft text that will be replaced by the final
        // answer on `Done`, and cleared on the tool round).
        AgentEvent::Chunk {
            text: "step one ".to_owned(),
        },
        AgentEvent::Chunk {
            text: "details".to_owned(),
        },
        AgentEvent::ToolStarted {
            name: "read_file".to_owned(),
            arguments: "{\"path\":\"Cargo.toml\"}".to_owned(),
        },
        AgentEvent::ToolDone {
            name: "read_file".to_owned(),
            status: ToolExecutionStatus::Success,
            result: "[contents]".to_owned(),
        },
        // Final streaming answer + authoritative `Done`.
        AgentEvent::Iteration { current: 2, max: 10 },
        AgentEvent::Chunk {
            text: "Here is the ".to_owned(),
        },
        AgentEvent::Chunk {
            text: "final answer.".to_owned(),
        },
        AgentEvent::Done {
            text: "Here is the final answer.".to_owned(),
        },
    ]
}

#[test]
fn full_agentic_session_populates_every_panel() {
    let (app, text) = render_events(&full_turn_events());
    // Token meter in the header.
    assert!(text.contains("500/1000"), "token meter missing:\n{text}");
    // Goal / plan / reflection status line.
    assert!(text.contains("goal: in progress"), "goal status missing:\n{text}");
    // Tool log panel shows the tool call.
    assert!(text.contains("read_file"), "tool log missing:\n{text}");
    // Transcript carries the authoritative final answer.
    assert!(text.contains("final answer"), "transcript missing:\n{text}");
    // Input bar placeholder present.
    assert!(text.contains("Type a message"), "input bar missing:\n{text}");
    // State: exactly one finalized assistant message (tool text was transient).
    assert_eq!(app.messages_len(), 1);
    assert!(app.streaming().is_empty());
}

#[test]
fn streaming_chunks_visible_live_before_done() {
    // Before `Done`, chunks accumulate in the streaming buffer (live tail).
    let (app, _) = render_events(&[
        AgentEvent::Chunk {
            text: "typing... ".to_owned(),
        },
        AgentEvent::Chunk {
            text: "more".to_owned(),
        },
    ]);
    assert_eq!(app.streaming(), "typing... more");
    assert_eq!(app.messages_len(), 0, "not finalized until Done");
}

#[test]
fn injected_credential_and_ansi_never_render() {
    let secret = "sk-proj-topsecretvalue987654321";
    let (_, text) = render_events(&[
        // A chunk that echoes a secret + ANSI escape.
        AgentEvent::Chunk {
            text: format!("reply with {secret} \u{1b}[31mred\u{1b}[0m"),
        },
        // A tool argument carrying the same secret.
        AgentEvent::ToolStarted {
            name: "write_file".to_owned(),
            arguments: format!("{{\"token\":\"{secret}\"}}"),
        },
        AgentEvent::Done {
            text: format!("done using {secret}"),
        },
    ]);
    assert!(
        !text.contains("topsecretvalue987654321"),
        "raw secret leaked into the buffer"
    );
    assert!(
        !text.contains('\u{1b}'),
        "raw ANSI escape leaked into the buffer"
    );
}

#[test]
fn ctrl_c_requests_interrupt_quit_for_exit_130() {
    let mut app = App::default();
    // A user has typed; then Ctrl-C.
    app.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
    let action = app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    // `main` maps an "interrupted" error to exit code 130; the render loop
    // surfaces Ctrl-C as QuitInterrupt (restore via the RawGuard Drop guard).
    assert_eq!(action, KeyAction::QuitInterrupt);
    assert!(app.should_quit);
}
