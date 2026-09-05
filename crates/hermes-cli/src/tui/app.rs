//! Spec 012 — App state machine + drawing (Ticket 02 renderer shell).
//!
//! [`App`] holds the small amount of *display* state the dashboard needs. It is
//! fed by [`TuiEvent`]s drained from the worker→renderer queue (each payload is
//! already sanitized at the source), and it owns the input line plus quit
//! semantics. Drawing is a pure function over a ratatui [`Frame`], so it is
//! unit-testable headlessly with [`TestBackend`] at any terminal size.

use std::collections::VecDeque;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use super::event::TuiEvent;
use super::layout;

/// Rolling cap for transcript and tool-log line histories.
const MAX_TRANSCRIPT: usize = 300;
const MAX_TOOL_LOG: usize = 200;

/// Outcome of handling one key event (returned to the renderer loop).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyAction {
    /// Key consumed with no effect on control flow.
    None,
    /// A full line of input was submitted and should go to the worker.
    Submit(String),
    /// User requested a clean quit (exit code 0).
    Quit,
    /// User requested an interrupt-style quit (Ctrl-C; exit code 130).
    QuitInterrupt,
}

/// Display state for the dashboard.
#[derive(Default)]
pub struct App {
    // Header / status.
    pub session_id: String,
    pub provider: String,
    pub estimate: usize,
    pub limit: Option<u64>,
    pub iteration: usize,
    pub goal_status: String,
    pub plan_active: bool,
    pub reflection_on: bool,
    // Histories (bounded).
    pub transcript: VecDeque<String>,
    pub tool_log: VecDeque<String>,
    // Input line.
    pub input: String,
    /// Set when the user requested quit (q or Ctrl-C). The renderer loop maps
    /// the quit *kind* from the [`KeyAction`] it receives.
    pub should_quit: bool,
}

impl App {
    /// Applies a worker event to display state. All text is already clean.
    pub fn apply(&mut self, event: TuiEvent) {
        match event {
            TuiEvent::StatusChanged { session_id, provider } => {
                self.session_id = session_id;
                self.provider = provider;
            }
            TuiEvent::TokenTick { estimate, limit } => {
                self.estimate = estimate;
                self.limit = limit;
            }
            TuiEvent::Iteration(n) => self.iteration = n,
            TuiEvent::StatusMeta {
                goal_status,
                plan_active,
                reflection_on,
            } => {
                self.goal_status = goal_status;
                self.plan_active = plan_active;
                self.reflection_on = reflection_on;
            }
            TuiEvent::Chunk(text) => self.push_transcript(text),
            TuiEvent::Done(final_text) => {
                self.push_transcript(format!("· {final_text}"));
                self.iteration = 0;
            }
            TuiEvent::Notice(text) => self.push_transcript(format!("[note] {text}")),
            TuiEvent::MaxIterations(n) => {
                self.push_transcript(format!("[stop] reached {n} iterations"));
                self.iteration = n;
            }
            TuiEvent::Blocked(reason) => {
                self.push_transcript(format!("[blocked] {reason}"));
            }
            TuiEvent::ToolStarted { name, arguments } => {
                self.push_tool_log(format!("▶ {name} {arguments}"));
            }
            TuiEvent::ToolDone { name, status } => {
                self.push_tool_log(format!("✓ {name} ({status})"));
            }
        }
    }

    fn push_transcript(&mut self, line: String) {
        if self.transcript.len() >= MAX_TRANSCRIPT {
            self.transcript.pop_front();
        }
        self.transcript.push_back(line);
    }

    fn push_tool_log(&mut self, line: String) {
        if self.tool_log.len() >= MAX_TOOL_LOG {
            self.tool_log.pop_front();
        }
        self.tool_log.push_back(line);
    }

    /// Handles a key event, mutating input state and returning a control
    /// decision for the renderer loop.
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> KeyAction {
        use crossterm::event::{KeyCode, KeyModifiers};
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
                KeyAction::QuitInterrupt
            }
            KeyCode::Char('q') => {
                self.should_quit = true;
                KeyAction::Quit
            }
            KeyCode::Enter => {
                let line = std::mem::take(&mut self.input);
                if line.trim().is_empty() {
                    KeyAction::None
                } else {
                    KeyAction::Submit(line)
                }
            }
            KeyCode::Backspace => {
                self.input.pop();
                KeyAction::None
            }
            KeyCode::Esc => {
                self.input.clear();
                KeyAction::None
            }
            KeyCode::Char(c) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.input.push(c);
                }
                KeyAction::None
            }
            _ => KeyAction::None,
        }
    }

    /// Renders the whole dashboard into the current frame.
    pub fn render(&self, frame: &mut Frame) {
        let panels = layout::split(frame.area());
        self.render_header(frame, panels.header);
        self.render_feed(frame, panels.transcript, "Transcript", &self.transcript);
        self.render_feed(frame, panels.tool_log, "Tool log", &self.tool_log);
        self.render_input(frame, panels.input);
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let tokens = match self.limit {
            Some(limit) => format!("{}/{}", self.estimate, limit),
            None => format!("{}", self.estimate),
        };
        let session = if self.session_id.is_empty() {
            "(no session)".to_owned()
        } else {
            self.session_id.clone()
        };
        let provider = if self.provider.is_empty() {
            "-".to_owned()
        } else {
            self.provider.clone()
        };
        let line1 = format!(
            "session: {session}   provider: {provider}   tokens: {tokens}   iteration: {}",
            self.iteration
        );
        // Ticket 03 status line: goal / plan / reflection.
        let goal = if self.goal_status.is_empty() {
            "not started".to_owned()
        } else {
            self.goal_status.clone()
        };
        let plan = if self.plan_active { "on" } else { "off" };
        let reflect = if self.reflection_on { "on" } else { "off" };
        let line2 = format!("goal: {goal}    plan: {plan}    reflection: {reflect}");
        let body = format!("{line1}\n{line2}");
        let paragraph =
            Paragraph::new(body).block(Block::default().borders(Borders::ALL).title("Hermes-RS"));
        frame.render_widget(paragraph, area);
    }

    fn render_feed(&self, frame: &mut Frame, area: Rect, title: &str, lines: &VecDeque<String>) {
        let cap = area.height.saturating_sub(2) as usize;
        let n = lines.len().min(cap.max(1));
        let start = lines.len().saturating_sub(n);
        let body = lines.iter().skip(start).cloned().collect::<Vec<_>>().join("\n");
        let block = Block::default().borders(Borders::ALL).title(title);
        let paragraph = Paragraph::new(body)
            .block(block)
            .wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
    }

    fn render_input(&self, frame: &mut Frame, area: Rect) {
        let body = if self.input.is_empty() {
            "Type a message (Enter to send, q / Ctrl-C to quit)".to_owned()
        } else {
            format!("> {}", self.input)
        };
        let block = Block::default().borders(Borders::ALL).title("Input");
        let paragraph = Paragraph::new(body).block(block);
        frame.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn app_with_some_state() -> App {
        let mut app = App::default();
        app.apply(TuiEvent::StatusChanged {
            session_id: "sess-1".into(),
            provider: "fake".into(),
        });
        app.apply(TuiEvent::TokenTick {
            estimate: 1234,
            limit: Some(128_000),
        });
        app.apply(TuiEvent::Chunk("hello ".into()));
        app.apply(TuiEvent::Chunk("world".into()));
        app.apply(TuiEvent::ToolStarted {
            name: "read_file".into(),
            arguments: "{}".into(),
        });
        app.apply(TuiEvent::ToolDone {
            name: "read_file".into(),
            status: "success".into(),
        });
        app.apply(TuiEvent::Done("done replying".into()));
        app
    }

    #[test]
    fn apply_populates_state() {
        let app = app_with_some_state();
        assert_eq!(app.session_id, "sess-1");
        assert_eq!(app.provider, "fake");
        assert_eq!(app.estimate, 1234);
        assert_eq!(app.limit, Some(128_000));
        assert_eq!(app.transcript.len(), 3); // two chunks + done
        assert_eq!(app.tool_log.len(), 2);
    }

    #[test]
    fn histories_are_bounded() {
        let mut app = App::default();
        for i in 0..400 {
            app.apply(TuiEvent::Chunk(format!("line {i}")));
        }
        assert_eq!(app.transcript.len(), MAX_TRANSCRIPT);
        assert!(app.transcript.iter().any(|l| l.contains("line 399")));
        // The first 100 oldest lines were dropped, so "line 0" must be gone.
        assert!(!app.transcript.iter().any(|l| l.contains("line 0")));
        assert!(app.transcript.iter().any(|l| l.contains("line 100")));
    }

    #[test]
    fn key_handling_submit_quit_interrupt() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = App::default();
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)),
            KeyAction::None
        );
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE)),
            KeyAction::None
        );
        assert_eq!(app.input, "hi");
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            KeyAction::Submit("hi".into())
        );
        assert!(app.input.is_empty());

        // q quits clean; Ctrl-C quits as interrupt.
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
            KeyAction::Quit
        );
        assert!(app.should_quit);

        let mut app2 = App::default();
        assert_eq!(
            app2.handle_key(KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL
            )),
            KeyAction::QuitInterrupt
        );
        assert!(app2.should_quit);
    }

    /// Renders at a given size headlessly; panics only on a real layout bug.
    fn render_at(w: u16, h: u16) -> App {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = app_with_some_state();
        terminal
            .draw(|frame| app.render(frame))
            .unwrap();
        app
    }

    #[test]
    fn render_large_and_wide_terminal() {
        render_at(200, 50);
        render_at(100, 30);
    }

    #[test]
    fn render_small_terminal_no_panic() {
        for (w, h) in [(20, 8), (10, 6), (5, 3), (3, 3), (2, 2)] {
            render_at(w, h);
        }
    }

    #[test]
    fn render_narrow_no_panic() {
        render_at(4, 20);
        render_at(8, 30);
    }
}
