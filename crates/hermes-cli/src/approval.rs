//! Interactive Command Approval Card matching Python Hermes UI dialog.

use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, size},
};
use std::io::{self, IsTerminal, Write};

/// Choices presented to the user
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalChoice {
    AllowOnce,
    AllowSession,
    Deny,
}

/// Prompt the user with a structured approval card matching Python rich box style.
/// Returns true if allowed (AllowOnce or AllowSession), false if denied or cancelled.
pub fn prompt_approval(command: &str) -> bool {
    // If not a TTY, fallback to safe denial
    if !std::io::stdin().is_terminal() {
        return false;
    }

    let mut stdout = io::stdout();
    if enable_raw_mode().is_err() {
        return false;
    }
    let _ = execute!(stdout, Hide);

    let options = [
        ("Yes, allow once", ApprovalChoice::AllowOnce),
        (
            "Yes, allow all for this session",
            ApprovalChoice::AllowSession,
        ),
        ("No, deny command", ApprovalChoice::Deny),
    ];
    let mut cursor = 0;

    let res = (|| -> bool {
        loop {
            let (cols, _) = size().unwrap_or((80, 24));
            let box_width = (cols as usize).clamp(40, 72);
            let inner_width = box_width.saturating_sub(4);

            let mut out = String::new();
            out.push_str(
                "
[1;33m┌─ ⚠ Command Approval Required ",
            );
            let title_prefix = "┌─ ⚠ Command Approval Required ";
            let title_len = title_prefix.chars().count();
            if box_width > title_len {
                out.push_str(&"─".repeat(box_width - title_len - 1));
            }
            out.push_str(
                "┐[0m
",
            );

            // Info row
            let info_txt = "The agent wants to run:";
            let pad_info = box_width.saturating_sub(info_txt.len() + 4);
            out.push_str(&format!(
                "│  [2m{}[0m{}│
",
                info_txt,
                " ".repeat(pad_info)
            ));

            // Command snippet
            let cmd_display = if command.chars().count() > inner_width.saturating_sub(2) {
                let mut s: String = command
                    .chars()
                    .take(inner_width.saturating_sub(5))
                    .collect();
                s.push_str("...");
                s
            } else {
                command.to_string()
            };
            let pad_cmd = box_width.saturating_sub(cmd_display.len() + 6);
            out.push_str(&format!(
                "│    [1;36m{}[0m{}│
",
                cmd_display,
                " ".repeat(pad_cmd)
            ));

            let ask_txt = "Allow this command?";
            let pad_ask = box_width.saturating_sub(ask_txt.len() + 4);
            out.push_str(&format!(
                "│  [2m{}[0m{}│
",
                ask_txt,
                " ".repeat(pad_ask)
            ));

            // Options
            for (idx, (label, _)) in options.iter().enumerate() {
                let is_cursor = idx == cursor;
                let radio = if is_cursor { "(●)" } else { "(○)" };
                let line_str = format!("    {} {}", radio, label);
                let pad_opt = box_width.saturating_sub(line_str.len() + 4);
                if is_cursor {
                    out.push_str(&format!(
                        "│  [1;32m{}[0m{}│
",
                        line_str,
                        " ".repeat(pad_opt)
                    ));
                } else {
                    out.push_str(&format!(
                        "│  {}{}│
",
                        line_str,
                        " ".repeat(pad_opt)
                    ));
                }
            }

            out.push_str(&format!(
                "[1;33m└{}┘[0m
",
                "─".repeat(box_width.saturating_sub(2))
            ));

            let _ = write!(stdout, "{}", out);
            let _ = stdout.flush();

            // Read keyboard input
            if let Ok(Event::Key(key_event)) = event::read() {
                if key_event.modifiers.contains(KeyModifiers::CONTROL)
                    && key_event.code == KeyCode::Char('c')
                {
                    return false;
                }
                match key_event.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if cursor == 0 {
                            cursor = options.len() - 1;
                        } else {
                            cursor -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if cursor + 1 >= options.len() {
                            cursor = 0;
                        } else {
                            cursor += 1;
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        return options[cursor].1 != ApprovalChoice::Deny;
                    }
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('n') => {
                        return false;
                    }
                    KeyCode::Char('y') => {
                        return true;
                    }
                    _ => {}
                }
            }
        }
    })();

    let _ = execute!(stdout, Show);
    let _ = disable_raw_mode();
    let _ = stdout.flush();
    println!();
    res
}
