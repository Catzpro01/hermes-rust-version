//! Spec 012 — worker side of the TUI (Ticket 02: shell demonstration).
//!
//! In Ticket 02 the renderer shell is exercised with a **labelled
//! demonstration worker** that streams representative [`TuiEvent`]s so the
//! layout, bounded queue and keyboard plumbing are genuinely exercised end to
//! end in a live terminal. It runs in a tokio task, reads [`TuiCommand`]s from
//! the renderer, and never blocks the renderer.
//!
//! The real agentic worker — driving the provider/goal/plan/reflection engine
//! and the tool registry — replaces the body of the `react` helper in Tickets
//! 03/04. The *boundary* (EventQueue producer + `TuiCommand` consumer) is the
//! durable contract and stays unchanged.

use std::sync::Arc;
use std::time::Duration;

use super::channel::{EventQueue, TuiCommand};
use super::event::TuiEvent;

/// Demo worker: answers each submitted line with a small simulated agentic
/// turn, and posts an initial status so the dashboard is populated at start.
pub async fn run_demo(
    mut cmds: tokio::sync::mpsc::UnboundedReceiver<TuiCommand>,
    queue: Arc<EventQueue>,
) {
    queue.push(TuiEvent::StatusChanged {
        session_id: "demo-session".to_owned(),
        provider: "fake".to_owned(),
    });
    queue.push(TuiEvent::TokenTick {
        estimate: 0,
        limit: Some(128_000),
    });
    queue.push(TuiEvent::Notice(
        "Hermes-RS TUI — demo worker (real agentic data arrives in Tickets 03/04)".to_owned(),
    ));
    queue.push(TuiEvent::Notice(
        "Type a message and press Enter to see a simulated turn; press q to quit.".to_owned(),
    ));

    let mut turns: usize = 0;
    while let Some(TuiCommand::Line(text)) = cmds.recv().await {
        turns += 1;
        // Pause briefly so the live streaming is visible rather than dumping
        // everything into one frame.
        tokio::time::sleep(Duration::from_millis(60)).await;
        queue.push(TuiEvent::sanitized_chunk(&format!("you asked: {text}\n")));
        queue.push(TuiEvent::Iteration(1));
        for k in 1..=2 {
            queue.push(TuiEvent::tool_started(
                format!("demo_tool_{k}"),
                "{\"note\":\"simulated call\"}",
            ));
            tokio::time::sleep(Duration::from_millis(40)).await;
            queue.push(TuiEvent::ToolDone {
                name: format!("demo_tool_{k}"),
                status: "ok".to_owned(),
            });
        }
        queue.push(TuiEvent::TokenTick {
            estimate: turns * 42 + 128,
            limit: Some(128_000),
        });
        queue.push(TuiEvent::Done(format!(
            "Simulated reply #{turns}: handled \"{text}\"."
        )));
    }
}
