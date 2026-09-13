//! Spec 014: Interactive radiolist selector matching Python `curses_radiolist` pixel/character-for-character.

use std::io::{self, Write};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, size, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

#[allow(dead_code)]
pub struct RadiolistOption<'a> {
    pub text: &'a str,
}

impl<'a> From<&'a str> for RadiolistOption<'a> {
    fn from(text: &'a str) -> Self {
        Self { text }
    }
}

/// Run an interactive curses-style radiolist selection matching Python Hermes `curses_radiolist`.
/// Returns `Some(index)` on Enter/Space, or `None` on ESC/q/Ctrl+C.
pub fn prompt_radiolist<T: AsRef<str>>(
    title: &str,
    items: &[T],
    initial_selected: usize,
) -> Option<usize> {
    if items.is_empty() {
        return None;
    }

    let mut stdout = io::stdout();
    if enable_raw_mode().is_err() {
        return None;
    }
    let _ = execute!(stdout, EnterAlternateScreen, Hide);

    let mut cursor = initial_selected.min(items.len() - 1);
    let selected = initial_selected.min(items.len().saturating_sub(1));
    let mut scroll_offset: usize = 0;

    let result = (|| -> Option<usize> {
        loop {
            let (cols, rows) = size().unwrap_or((80, 24));
            let max_x = cols as usize;
            let max_y = rows as usize;

            let header_rows = 3; // title (1), hint (1), blank (1)
            let reserve_bottom = 1;
            let visible_rows = if max_y > header_rows + reserve_bottom {
                max_y - header_rows - reserve_bottom
            } else {
                1
            };

            // Clamp scroll offset
            if cursor < scroll_offset {
                scroll_offset = cursor;
            } else if cursor >= scroll_offset + visible_rows {
                scroll_offset = cursor + 1 - visible_rows;
            }
            if scroll_offset + visible_rows > items.len() && items.len() >= visible_rows {
                scroll_offset = items.len() - visible_rows;
            }

            // Draw screen
            let _ = execute!(stdout, MoveTo(0, 0), Clear(ClearType::All));

            // Row 0: Title in Bold Yellow
            let _ = write!(stdout, "\x1b[1;33m{}\x1b[0m\r\n", title);

            // Row 1: Hint in Dim
            let _ = write!(
                stdout,
                "\x1b[2m  \u{2191}\u{2193} navigate  ENTER/SPACE select  ESC cancel\x1b[0m\r\n\r\n"
            );

            // Item rows
            let end_idx = (scroll_offset + visible_rows).min(items.len());
            for i in scroll_offset..end_idx {
                let is_cursor = i == cursor;
                let is_sel = i == selected;

                let radio = if is_sel { "\u{25cf}" } else { "\u{25cb}" };
                let arrow = if is_cursor { "\u{2192}" } else { " " };
                let item_text = items[i].as_ref();

                let prefix = format!(" {} ({}) ", arrow, radio);
                let available_width = if max_x > prefix.len() + 1 {
                    max_x - prefix.len() - 1
                } else {
                    10
                };

                let truncated_text = if item_text.chars().count() > available_width {
                    let mut s = String::new();
                    for ch in item_text.chars().take(available_width) {
                        s.push(ch);
                    }
                    s
                } else {
                    item_text.to_string()
                };

                if is_cursor {
                    // Whole line bold green for cursor row
                    let _ = write!(stdout, "\x1b[1;32m{}{}\x1b[0m\r\n", prefix, truncated_text);
                } else {
                    let _ = write!(stdout, "{}{}\r\n", prefix, truncated_text);
                }
            }

            let _ = stdout.flush();

            // Read keyboard event
            if let Ok(Event::Key(key_event)) = event::read() {
                if key_event.modifiers.contains(KeyModifiers::CONTROL) && key_event.code == KeyCode::Char('c') {
                    return None;
                }
                match key_event.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if cursor == 0 {
                            cursor = items.len() - 1;
                        } else {
                            cursor -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if cursor + 1 >= items.len() {
                            cursor = 0;
                        } else {
                            cursor += 1;
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        return Some(cursor);
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        return None;
                    }
                    _ => {}
                }
            }
        }
    })();

    let _ = execute!(stdout, Show, LeaveAlternateScreen);
    let _ = disable_raw_mode();
    let _ = stdout.flush();

    result
}
