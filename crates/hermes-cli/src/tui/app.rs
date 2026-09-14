//! Spec 012 — App state machine + drawing (Ticket 04: transcript/tool-log/input).
//!
//! [`App`] holds the small amount of *display* state the dashboard needs. It is
//! fed by [`TuiEvent`]s drained from the worker→renderer queue (each payload is
//! already sanitized at the source), and it owns:
//! * a **streaming transcript** — model `Chunk`s accumulate into an open
//!   assistant message that is finalized on `Done`;
//! * an independent scroll offset per panel (transcript / tool log);
//! * a small **single-line input editor** with cursor movement + history.
//!
//! Drawing is a pure function over a ratatui [`Frame`], so it is unit-testable
//! headlessly with [`TestBackend`] at any terminal size.

use std::collections::VecDeque;

use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use super::event::TuiEvent;
use super::layout;

/// Rolling caps for the message and tool-log histories.
const MAX_MESSAGES: usize = 400;
const MAX_TOOL_LOG: usize = 200;
/// How many submitted lines to remember for input history.
const MAX_HISTORY: usize = 20;

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

/// A single-line input buffer with a character cursor. Kept as `Vec<char>` so
/// cursor arithmetic stays multi-byte safe.
#[derive(Default)]
struct InputBuffer {
    chars: Vec<char>,
    cursor: usize,
}

impl InputBuffer {
    fn text(&self) -> String {
        self.chars.iter().collect()
    }
    fn insert(&mut self, c: char) {
        self.chars.insert(self.cursor, c);
        self.cursor += 1;
    }
    fn backspace(&mut self) {
        if self.cursor > 0 && self.cursor <= self.chars.len() {
            self.chars.remove(self.cursor - 1);
            self.cursor -= 1;
        }
    }
    fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }
    fn right(&mut self) {
        if self.cursor < self.chars.len() {
            self.cursor += 1;
        }
    }
    fn clear(&mut self) {
        self.chars.clear();
        self.cursor = 0;
    }
}

/// Display state for the dashboard.
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
    // Transcript: completed messages (bounded) + the in-flight assistant chunk.
    messages: VecDeque<String>,
    streaming: String,
    pub awaiting: bool,
    // Tool log (bounded).
    tool_log: VecDeque<String>,
    // Scroll: 0 = follow bottom; larger = show that many older entries.
    transcript_scroll: usize,
    tool_scroll: usize,
    // Input editor.
    input: InputBuffer,
    history: Vec<String>,
    history_pos: Option<usize>,
    pub should_quit: bool,
    // Spec 017 T08 (T04 option 3) — composer placeholder ghost text shown
    // while the input line is empty (parity of v0.21.0
    // `COMPOSER_PLACEHOLDERS`, tips.py L495). Re-rolled each time the line
    // is cleared, mirroring `get_random_composer_placeholder()` (a fresh
    // `random.choice` per call).
    placeholder: String,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Builds dashboard state with a freshly-rolled composer placeholder.
    pub fn new() -> Self {
        Self {
            placeholder: crate::tui::tips::random_composer_placeholder().to_owned(),
            ..App::fields_default()
        }
    }

    /// All-default field values (everything but the placeholder).
    fn fields_default() -> Self {
        Self {
            session_id: String::new(),
            provider: String::new(),
            estimate: 0,
            limit: None,
            iteration: 0,
            goal_status: String::new(),
            plan_active: false,
            reflection_on: false,
            messages: VecDeque::new(),
            streaming: String::new(),
            awaiting: false,
            tool_log: VecDeque::new(),
            transcript_scroll: 0,
            tool_scroll: 0,
            input: InputBuffer::default(),
            history: Vec::new(),
            history_pos: None,
            should_quit: false,
            placeholder: String::new(),
        }
    }

    /// Rolls a fresh composer placeholder for the next empty input line.
    fn roll_placeholder(&mut self) {
        self.placeholder = crate::tui::tips::random_composer_placeholder().to_owned();
    }

    /// The current composer placeholder (for tests / renderers).
    #[cfg(test)]
    pub(crate) fn placeholder(&self) -> &str {
        &self.placeholder
    }
}

impl App {
    /// Applies a worker event to display state. All text is already clean.
    pub fn apply(&mut self, event: TuiEvent) {
        match event {
            TuiEvent::StatusChanged {
                session_id,
                provider,
            } => {
                self.session_id = session_id;
                self.provider = provider;
            }
            TuiEvent::StatusMeta {
                goal_status,
                plan_active,
                reflection_on,
            } => {
                self.goal_status = goal_status;
                self.plan_active = plan_active;
                self.reflection_on = reflection_on;
            }
            TuiEvent::TokenTick { estimate, limit } => {
                self.estimate = estimate;
                self.limit = limit;
            }
            TuiEvent::Iteration(n) => self.iteration = n,
            // Streaming chunk from the model: append to the in-flight message.
            TuiEvent::Chunk(text) => {
                self.streaming.push_str(&text);
                self.awaiting = true;
            }
            // A tool call begins: its accompanying assistant text is transient
            // scaffolding for a tool round, so discard it and log the tool.
            TuiEvent::ToolStarted { name, arguments } => {
                self.streaming.clear();
                self.push_tool_log(format!("▶ {name} {arguments}"));
            }
            TuiEvent::ToolDone { name, status } => {
                self.push_tool_log(format!("✓ {name} ({status})"));
            }
            // Final tool-free assistant answer: authoritative end of the turn.
            TuiEvent::Done(final_text) => {
                let final_text = if final_text.trim().is_empty() {
                    std::mem::take(&mut self.streaming)
                } else {
                    std::mem::take(&mut self.streaming);
                    final_text
                };
                self.push_message(format!("assistant▸ {final_text}"));
                self.awaiting = false;
                self.iteration = 0;
            }
            TuiEvent::Notice(text) => self.push_message(format!("[note] {text}")),
            TuiEvent::MaxIterations(n) => {
                self.push_message(format!("[stop] reached {n} iterations"));
                self.iteration = n;
                self.awaiting = false;
            }
            TuiEvent::Blocked(reason) => {
                self.push_message(format!("[blocked] {reason}"));
                self.awaiting = false;
            }
        }
    }

    fn push_message(&mut self, line: String) {
        if self.messages.len() >= MAX_MESSAGES {
            self.messages.pop_front();
        }
        self.messages.push_back(line);
        self.follow_transcript();
    }

    fn push_tool_log(&mut self, line: String) {
        if self.tool_log.len() >= MAX_TOOL_LOG {
            self.tool_log.pop_front();
        }
        self.tool_log.push_back(line);
        self.follow_tool();
    }

    fn follow_transcript(&mut self) {
        self.transcript_scroll = 0;
    }
    fn follow_tool(&mut self) {
        self.tool_scroll = 0;
    }

    /// User submitted a prompt: record it in the transcript + history.
    fn record_user_message(&mut self, line: String) {
        self.push_message(format!("you▸ {line}"));
        // Start collecting a fresh assistant stream.
        self.streaming.clear();
        self.awaiting = true;
        // Remember in history (dedupe consecutive).
        if self.history.last().map(String::as_str) != Some(line.as_str()) {
            self.history.push(line);
            if self.history.len() > MAX_HISTORY {
                self.history.remove(0);
            }
        }
        self.history_pos = None;
    }

    /// Number of finalized transcript messages (for tests/assertions).
    #[cfg(test)]
    pub(crate) fn messages_len(&self) -> usize {
        self.messages.len()
    }

    /// The in-flight streaming assistant text (for tests/assertions).
    #[cfg(test)]
    pub(crate) fn streaming(&self) -> &str {
        &self.streaming
    }

    /// Handles a key event, mutating input/scroll state and returning a control
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
                let line = self.input.text();
                if line.trim().is_empty() {
                    KeyAction::None
                } else {
                    self.record_user_message(line.clone());
                    self.input.clear();
                    self.roll_placeholder();
                    KeyAction::Submit(line)
                }
            }
            KeyCode::Backspace => {
                self.input.backspace();
                if self.input.chars.is_empty() {
                    self.roll_placeholder();
                }
                KeyAction::None
            }
            KeyCode::Esc => {
                self.input.clear();
                self.roll_placeholder();
                KeyAction::None
            }
            KeyCode::Left => {
                self.input.left();
                KeyAction::None
            }
            KeyCode::Right => {
                self.input.right();
                KeyAction::None
            }
            KeyCode::Home => {
                self.input.cursor = 0;
                KeyAction::None
            }
            KeyCode::End => {
                self.input.cursor = self.input.chars.len();
                KeyAction::None
            }
            // Input history navigation.
            KeyCode::Up => {
                self.history_back();
                KeyAction::None
            }
            KeyCode::Down => {
                self.history_forward();
                KeyAction::None
            }
            // Transcript scroll (PgUp/PgDn). Tool log is bottom-anchored.
            KeyCode::PageUp => {
                self.scroll_up();
                KeyAction::None
            }
            KeyCode::PageDown => {
                self.scroll_down();
                KeyAction::None
            }
            KeyCode::Char(c) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.input.insert(c);
                }
                KeyAction::None
            }
            _ => KeyAction::None,
        }
    }

    fn history_back(&mut self) {
        if self.history.is_empty() {
            return;
        }
        // Save what the user was typing if we're leaving the "new line" slot.
        match self.history_pos {
            None => {
                self.history_pos = Some(self.history.len() - 1);
            }
            Some(0) => {}
            Some(i) => self.history_pos = Some(i - 1),
        }
        if let Some(i) = self.history_pos {
            let text = self.history[i].clone();
            self.load_input(&text);
        }
    }

    fn history_forward(&mut self) {
        match self.history_pos {
            None => {}
            Some(i) if i + 1 < self.history.len() => {
                self.history_pos = Some(i + 1);
                let text = self.history[i + 1].clone();
                self.load_input(&text);
            }
            _ => {
                self.history_pos = None;
                self.load_input("");
                self.roll_placeholder();
            }
        }
    }

    fn load_input(&mut self, text: &str) {
        self.input.chars = text.chars().collect();
        self.input.cursor = self.input.chars.len();
    }

    fn scroll_up(&mut self) {
        // Scrolling up reveals more of the transcript history.
        self.transcript_scroll += 1;
    }
    fn scroll_down(&mut self) {
        self.transcript_scroll = self.transcript_scroll.saturating_sub(1);
    }

    /// Renders the whole dashboard into the current frame.
    pub fn render(&self, frame: &mut Frame) {
        let panels = layout::split(frame.area());
        self.render_header(frame, panels.header);
        self.render_transcript(frame, panels.transcript);
        self.render_tool_log(frame, panels.tool_log);
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

    /// Builds the visible slice of a history as display lines for a Paragraph.
    /// `scroll == 0` follows the bottom (newest `cap` entries, plus a live
    /// streaming tail line). `scroll > 0` reveals older entries by excluding
    /// that many of the newest.
    fn window_of(
        items: &VecDeque<String>,
        scroll: usize,
        streaming: Option<&str>,
        area: Rect,
    ) -> String {
        let cap = area.height.saturating_sub(2) as usize;
        let exclude_newest = scroll.min(items.len());
        let end = items.len().saturating_sub(exclude_newest);
        let start = end.saturating_sub(cap.max(1));
        let mut out: Vec<&str> = items
            .iter()
            .skip(start)
            .take(end - start)
            .map(String::as_str)
            .collect();
        if scroll == 0 {
            if let Some(s) = streaming {
                if !s.is_empty() {
                    out.push(s);
                }
            }
        }
        out.join("\n")
    }

    fn render_transcript(&self, frame: &mut Frame, area: Rect) {
        let streaming = if self.awaiting && !self.streaming.is_empty() {
            Some(self.streaming.as_str())
        } else {
            None
        };
        let body = Self::window_of(&self.messages, self.transcript_scroll, streaming, area);
        let title = if self.awaiting {
            "Transcript (…)"
        } else {
            "Transcript"
        };
        let block = Block::default().borders(Borders::ALL).title(title);
        let paragraph = Paragraph::new(body).block(block).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
    }

    fn render_tool_log(&self, frame: &mut Frame, area: Rect) {
        let body = Self::window_of(&self.tool_log, self.tool_scroll, None, area);
        let block = Block::default().borders(Borders::ALL).title("Tool log");
        let paragraph = Paragraph::new(body).block(block).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
    }

    fn render_input(&self, frame: &mut Frame, area: Rect) {
        let input: String = self.input.text();
        // Spec 017 T08 (T04 option 3): an empty composer shows a random
        // v0.21.0 `COMPOSER_PLACEHOLDERS` string as ghost text — the theme's
        // `placeholder` style (TUI §8: #888888 italic). It never enters the
        // input buffer and is re-rolled on every clear.
        let line = if input.is_empty() {
            let theme = crate::tui::theme::HermesTheme::dark_canonical();
            ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(
                    crate::tui::welcome::PROMPT_SYMBOL.to_string(),
                    theme.input_text(),
                ),
                ratatui::text::Span::styled(self.placeholder.clone(), theme.placeholder()),
            ])
        } else {
            ratatui::text::Line::from(format!("{}{input}", crate::tui::welcome::PROMPT_SYMBOL))
        };
        let block = Block::default().borders(Borders::ALL).title("Input");
        let paragraph = Paragraph::new(line).block(block);
        frame.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn app_with_a_turn() -> App {
        let mut app = App::default();
        app.apply(TuiEvent::StatusChanged {
            session_id: "sess-1".into(),
            provider: "fake".into(),
        });
        app.apply(TuiEvent::TokenTick {
            estimate: 1234,
            limit: Some(128_000),
        });
        // One tool round then a final assistant message.
        app.apply(TuiEvent::ToolStarted {
            name: "read_file".into(),
            arguments: "{}".into(),
        });
        app.apply(TuiEvent::ToolDone {
            name: "read_file".into(),
            status: "success".into(),
        });
        app.apply(TuiEvent::Done("hello world".into()));
        app
    }

    #[test]
    fn apply_populates_state_and_finalizes_transcript() {
        let app = app_with_a_turn();
        assert_eq!(app.session_id, "sess-1");
        assert_eq!(app.estimate, 1234);
        assert_eq!(app.limit, Some(128_000));
        // One finalized assistant message carrying the final answer.
        assert_eq!(app.messages.len(), 1);
        assert!(app.messages.front().unwrap().contains("hello world"));
        assert_eq!(app.streaming, "");
        assert!(!app.awaiting);
        // Tool round appeared in the tool log.
        assert_eq!(app.tool_log.len(), 2);
    }

    #[test]
    fn streaming_chunks_accumulate_then_done_finalizes_once() {
        let mut app = App::default();
        app.apply(TuiEvent::Chunk("hello ".into()));
        app.apply(TuiEvent::Chunk("world".into()));
        assert_eq!(app.streaming, "hello world");
        assert_eq!(app.messages.len(), 0, "not finalized yet");
        app.apply(TuiEvent::Done("hello world".into()));
        assert_eq!(app.messages.len(), 1);
        assert!(app.messages.front().unwrap().contains("hello world"));
        assert_eq!(app.streaming, "");
        assert!(!app.awaiting);
    }

    #[test]
    fn histories_are_bounded() {
        let mut app = App::default();
        for i in 0..600 {
            app.apply(TuiEvent::Chunk(format!("line {i}")));
        }
        // Streaming keeps growing; bounded by finalization. Check messages cap
        // separately by pushing many finalized messages.
        let mut app2 = App::default();
        for i in 0..500 {
            app2.apply(TuiEvent::Done(format!("msg {i}")));
        }
        assert_eq!(app2.messages.len(), MAX_MESSAGES);
        assert!(app2.messages.iter().any(|m| m.contains("msg 499")));
        assert!(!app2.messages.iter().any(|m| m.contains("msg 0")));
    }

    #[test]
    fn user_message_recorded_in_transcript_and_history() {
        let mut app = App::default();
        app.record_user_message("hello".into());
        app.record_user_message("hello".into());
        app.record_user_message("world".into());
        // Consecutive duplicate suppressed in history but both in transcript.
        assert_eq!(app.messages.len(), 3);
        assert_eq!(app.history, vec!["hello".to_owned(), "world".to_owned()]);
        assert!(app.awaiting);
    }

    #[test]
    fn key_handling_edits_input_with_cursor() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = App::default();
        app.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::NONE));
        assert_eq!(app.input.text(), "hXi");
        app.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(app.input.text(), "hi");
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            KeyAction::Submit("hi".into())
        );
        assert!(app.input.text().is_empty());
    }

    #[test]
    fn history_navigation_loads_previous_lines() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = App::default();
        app.record_user_message("first".into());
        app.record_user_message("second".into());
        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.input.text(), "second");
        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.input.text(), "first");
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.input.text(), "second");
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.input.text(), "");
    }

    #[test]
    fn scroll_keys_adjust_transcript_offset() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = App::default();
        assert_eq!(app.transcript_scroll, 0);
        app.handle_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));
        assert_eq!(app.transcript_scroll, 1);
        app.handle_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));
        assert_eq!(app.transcript_scroll, 2);
        app.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
        assert_eq!(app.transcript_scroll, 1);
        app.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
        assert_eq!(app.transcript_scroll, 0, "clamps at bottom");
    }

    #[test]
    fn quit_and_interrupt() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = App::default();
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
            KeyAction::Quit
        );
        assert!(app.should_quit);
        let mut app2 = App::default();
        assert_eq!(
            app2.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            KeyAction::QuitInterrupt
        );
        assert!(app2.should_quit);
    }

    /// Renders at a given size headlessly; panics only on a real layout bug.
    fn render_at(w: u16, h: u16) -> App {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = app_with_a_turn();
        terminal.draw(|frame| app.render(frame)).unwrap();
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

    /// Spec 017 T08 (T04 option 3): an empty composer renders the current
    /// composer placeholder (a verbatim v0.21.0 catalog member) as ghost
    /// text in the input bar.
    #[test]
    fn composer_placeholder_rendered_when_input_empty() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = App::new();
        let placeholder = app.placeholder().to_owned();
        assert!(
            crate::tui::tips::COMPOSER_PLACEHOLDERS.contains(&placeholder.as_str()),
            "placeholder must be a catalog member, got {placeholder:?}"
        );
        terminal.draw(|frame| app.render(frame)).unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(
            text.contains(&placeholder),
            "ghost placeholder missing from input bar:\n{text}"
        );
    }

    /// The placeholder is re-rolled (submit / Esc / backspace-to-empty /
    /// history-forward past the top) and is always a catalog member.
    #[test]
    fn placeholder_rolls_and_stays_in_catalog() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = App::new();
        let in_catalog =
            |app: &App| crate::tui::tips::COMPOSER_PLACEHOLDERS.contains(&app.placeholder());
        assert!(in_catalog(&app));
        app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(action, KeyAction::Submit("x".to_owned()));
        assert!(app.input.text().is_empty());
        assert!(in_catalog(&app));
        app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert!(in_catalog(&app));
        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(in_catalog(&app));
    }
}
