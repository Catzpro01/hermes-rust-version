//! Spec 012 — TUI dashboard (Ratatui + crossterm). Opt-in via `--tui`.
//!
//! Architecture
//! ============
//! * **Worker** — the agentic turn runner lives in a tokio task and only ever
//!   *pushes* display events into an [`EventQueue`]. In Ticket 02 this is a
//!   labelled demonstration worker; real agentic data wiring lands in Tickets
//!   03/04 by reusing the same queue boundary.
//! * **Renderer** — a Ratatui terminal loop runs on a dedicated blocking
//!   thread (so it never stalls the async worker). Each frame it drains the
//!   queue into [`App`], redraws, and services keyboard input.
//! * **Channel** — worker→renderer is a bounded, drop-oldest queue (Warning B);
//!   renderer→worker is an unbounded [`TuiCommand`] sender (user keystrokes are
//!   never lossy).
//! * **Sanitization boundary** — every `TuiEvent` payload is pre-sanitized and
//!   pre-redacted at the source; the renderer never sanitizes.
//!
//! Terminal hygiene is an invariant: a [`RawGuard`] restores the terminal on
//! every exit path (normal quit, Ctrl-C, error, or unwind).

mod app;
mod channel;
mod event;
mod layout;
mod worker;

use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{App, KeyAction};
use channel::{EventQueue, TuiCommand, DEFAULT_QUEUE_CAPACITY};

use crossterm::cursor;
use crossterm::event as cev;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;

/// Terminal-guard that guarantees restoration even on early return/unwind.
struct RawGuard;

impl RawGuard {
    fn enter() -> anyhow::Result<Self> {
        terminal::enable_raw_mode()
            .with_context(|| "failed to enter raw mode (interactive terminal required)")?;
        let mut stdout = io::stdout();
        stdout
            .execute(EnterAlternateScreen)
            .with_context(|| "failed to switch to the alternate screen")?;
        stdout
            .execute(cursor::Hide)
            .with_context(|| "failed to hide the cursor")?;
        Ok(RawGuard)
    }
}

impl Drop for RawGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = io::stdout().execute(LeaveAlternateScreen);
        let _ = io::stdout().execute(cursor::Show);
        let _ = io::stdout().flush();
    }
}

/// The renderer loop's exit classification (maps to process exit codes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Exit {
    /// Normal quit (e.g. `q`) → exit 0.
    Clean,
    /// Ctrl-C → exit 130.
    Interrupted,
}

/// Runs the TUI dashboard until the user quits. Must only be reached after the
/// interactive-terminal gate in `main`.
pub async fn run_tui() -> anyhow::Result<()> {
    let queue = Arc::new(EventQueue::new(DEFAULT_QUEUE_CAPACITY));
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel::<TuiCommand>();

    // Worker runs in a tokio task so the blocking renderer never stalls it.
    let worker_queue = Arc::clone(&queue);
    let worker = tokio::spawn(async move { worker::run_demo(cmd_rx, worker_queue).await });

    // Renderer runs on a blocking thread (separate from the async worker).
    let renderer_queue = Arc::clone(&queue);
    let renderer_cmd = cmd_tx.clone();
    let exit = tokio::task::spawn_blocking(move || render_loop(renderer_queue, renderer_cmd))
        .await
        .with_context(|| "TUI renderer task panicked")??;

    // Dropping our sender lets the worker observe channel closure and wind down.
    drop(cmd_tx);
    let _ = worker.await;

    match exit {
        Exit::Clean => Ok(()),
        Exit::Interrupted => anyhow::bail!("interrupted (Ctrl-C)"),
    }
}

/// Blocking renderer loop: raw mode, event draining, draw, keyboard handling.
fn render_loop(queue: Arc<EventQueue>, cmd_tx: tokio::sync::mpsc::UnboundedSender<TuiCommand>) -> anyhow::Result<Exit> {
    let _guard = RawGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))
        .with_context(|| "failed to initialise terminal")?;
    terminal.clear().with_context(|| "failed to clear terminal")?;

    let mut app = App::default();
    let mut interrupted = false;

    loop {
        terminal.autoresize()?;
        // Drain every pending worker event into display state.
        for ev in queue.drain() {
            app.apply(ev);
        }

        terminal.draw(|frame| app.render(frame))?;

        if app.should_quit {
            break;
        }

        // Poll keyboard briefly (bounded tick: no uncontrolled busy-loop).
        if cev::poll(Duration::from_millis(50))? {
            if let cev::Event::Key(key) = cev::read()? {
                if key.kind == cev::KeyEventKind::Press {
                    match app.handle_key(key) {
                        KeyAction::None => {}
                        KeyAction::Submit(line) => {
                            let _ = cmd_tx.send(TuiCommand::Line(line));
                        }
                        KeyAction::Quit => break,
                        KeyAction::QuitInterrupt => {
                            interrupted = true;
                            break;
                        }
                    }
                }
            }
        }
    }

    Ok(if interrupted { Exit::Interrupted } else { Exit::Clean })
}
