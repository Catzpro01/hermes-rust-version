//! Spec 017 T09 — session browse picker (`main.py::_session_browse_picker`).
//!
//! Parity target is `docs/HERMES_UI_SPEC.md` §F (Hermes Python v0.21.0):
//! the hint line, column header, row layout, footer, `d` + `[y/N]` delete
//! flow and the numbered non-curses fallback are verbatim. The curses event
//! loop itself is ported onto crossterm (same crate the TUI already uses),
//! the way T08 ported prompt_toolkit completion onto rustyline —
//! *paritas = perilaku, bukan crate*.
//!
//! Deliberate adaptations (no Python source for these details in §F):
//! * `sid` shows the first 8 hex chars of the UUID (Python session ids are
//!   short; a full 36-char UUID would not fit an 80-column picker).
//! * `status` is `done` for any session with messages and `empty` otherwise —
//!   Hermes-RS never recorded `interrupted`/`error` lifecycle states, so
//!   `intr`/`err` cannot be distinguished (column kept for parity).
//! * `name` is the first user message (single-lined); Hermes-RS has no
//!   session titles yet, so there is no title half of `Title / Preview`.
//! * The cursor row is reverse video; Python colors rows via `_status_attr`,
//!   whose palette was not captured in §F.
//! * `q` is a filter character in the curses-style browser (the hint lists
//!   `Esc quit` only); `q to cancel` exists solely in the numbered fallback.

use std::io::{BufRead, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use hermes_core::session::{SessionId, SessionStore};

use crate::output::sanitize_untrusted_output;

/// Hint line while no filter text is typed (spec §F, verbatim).
pub const HINT_BROWSE: &str =
    "  Browse sessions — ↑↓ navigate  Enter select  Type to filter  Esc quit";
/// Empty store, no filter (spec §F, verbatim — no leading spaces).
pub const NO_SESSIONS: &str = "No sessions found.";
/// Filter matches nothing (spec §F, verbatim — two leading spaces).
pub const NO_MATCH: &str = "  No sessions match the filter.";
/// Terminal below the minimum size (spec §F, verbatim).
pub const TOO_SMALL: &str = "Terminal too small";
/// Delete success / failure (spec §F, verbatim).
pub const DELETED: &str = "Deleted.";
pub const DELETE_FAILED: &str = "Delete failed.";
/// Non-curses fallback header (spec §F, verbatim, incl. newlines).
pub const FALLBACK_HEADER: &str = "\n  Browse sessions  (enter number to resume, q to cancel)\n";

/// Minimum terminal size for the curses-style browser. §F does not record
/// Python's threshold; 60×8 fits the fixed columns plus a 10-char name.
pub const MIN_WIDTH: u16 = 60;
pub const MIN_HEIGHT: u16 = 8;

/// Hint line while filter text is typed (spec §F, verbatim shape).
pub fn hint_filter(search: &str) -> String {
    format!("  Browse sessions — filter: {search}█")
}

/// Column header (spec §F, verbatim shape).
pub fn column_header(name_width: usize) -> String {
    format!(
        "  {:<nw$}  {:<5}  {:>5}  {:<10}  {:<5} {}",
        "Title / Preview",
        "Stat",
        "Msgs",
        "Active",
        "Src",
        "ID",
        nw = name_width,
    )
}

/// One session row (spec §F, verbatim shape). The f-string in L1387 carries
/// no leading indent but curses draws it under the indented header, so the
/// two spaces are part of the rendered row.
pub fn format_row(
    name: &str,
    status: &str,
    msgs: usize,
    last_active: &str,
    source: &str,
    sid: &str,
    name_width: usize,
) -> String {
    format!(
        "  {:<nw$}  {:<5}  {:>5}  {:<10}  {:<5} {}",
        truncate_chars(name, name_width),
        status,
        msgs,
        last_active,
        source,
        sid,
        nw = name_width,
    )
}

/// Footer line (spec §F, verbatim shape).
pub fn footer(cursor_one_based: usize, shown: usize, total: usize) -> String {
    let mut out = format!("  {cursor_one_based}/{shown} sessions");
    if shown != total {
        out.push_str(&format!(" (filtered from {total})"));
    }
    out.push_str("   d delete");
    out
}

/// Delete confirmation prompt (spec §F, verbatim shape, explicit `[y/N]`).
pub fn delete_prompt(label: &str) -> String {
    format!("  Delete session '{label}'? [y/N]")
}

/// Name-column width for a terminal width. Fixed columns occupy 44 cells
/// (`  ` + `  Stat ` + `   Msgs ` + `  Active    ` + `  Src  ` + ` ` + 8-char
/// sid); the remainder goes to the name, clamped to a readable range.
pub fn name_width(term_width: u16) -> usize {
    (term_width as usize).saturating_sub(44).clamp(10, 48)
}

fn truncate_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    text.chars().take(max).collect()
}

/// Relative `Active` column (never exceeds 10 cells). The exact Python
/// format was not captured in §F; relative ages keep the column truthful
/// without inventing a date layout.
pub fn relative_active(now_secs: f64, then_secs: f64) -> String {
    let delta = (now_secs - then_secs).max(0.0) as u64;
    if delta < 60 {
        "just now".to_owned()
    } else if delta < 3_600 {
        format!("{}m ago", delta / 60)
    } else if delta < 86_400 {
        format!("{}h ago", delta / 3_600)
    } else if delta < 30 * 86_400 {
        format!("{}d ago", delta / 86_400)
    } else if delta < 365 * 86_400 {
        format!("{}w ago", delta / (7 * 86_400))
    } else {
        format!("{}y ago", delta / (365 * 86_400))
    }
}

fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

/// One pickable session row, newest first (the `store.list()` order).
pub struct SessionRow {
    pub id: SessionId,
    pub name: String,
    pub status: &'static str,
    pub msgs: usize,
    pub last_active: String,
    pub source: String,
    /// First 8 chars of the UUID for the narrow `ID` column.
    pub sid: String,
}

/// Collect display rows from the canonical store. Names and sources pass
/// through the render-boundary sanitizer (ANSI/C0 stripped); the canonical
/// bytes are never touched.
pub fn collect_rows(store: &SessionStore) -> anyhow::Result<Vec<SessionRow>> {
    let now = now_secs();
    let mut rows = Vec::new();
    for id in store.list()? {
        let session = store.resume(&id)?;
        let name = session
            .turns
            .iter()
            .find_map(|t| match t {
                hermes_core::conversation::Turn::User { content } => Some(content.as_str()),
                _ => None,
            })
            .or_else(|| {
                session.turns.iter().find_map(|t| match t {
                    hermes_core::conversation::Turn::Assistant { content } => {
                        Some(content.as_str())
                    }
                    _ => None,
                })
            })
            .unwrap_or("(empty)");
        // Single-line: any whitespace run (incl. newlines) becomes one space.
        let flat: String = name.split_whitespace().collect::<Vec<_>>().join(" ");
        let name = sanitize_untrusted_output(&flat);
        let name = if name.trim().is_empty() {
            "(empty)".to_owned()
        } else {
            name
        };
        let last = store
            .list_messages(&id)?
            .iter()
            .map(|m| m.timestamp)
            .fold(session.started_at, f64::max);
        let full = id.to_string();
        rows.push(SessionRow {
            id,
            name,
            status: if session.turns.is_empty() {
                "empty"
            } else {
                "done"
            },
            msgs: session.turns.len(),
            last_active: relative_active(now, last),
            source: sanitize_untrusted_output(&session.source),
            sid: full.chars().take(8).collect(),
        });
    }
    Ok(rows)
}

/// Indices of `rows` matching `query` (case-insensitive substring over the
/// name, short id and source). An empty query matches everything.
pub fn apply_filter(rows: &[SessionRow], query: &str) -> Vec<usize> {
    if query.is_empty() {
        return (0..rows.len()).collect();
    }
    let q = query.to_lowercase();
    rows.iter()
        .enumerate()
        .filter(|(_, r)| {
            r.name.to_lowercase().contains(&q)
                || r.sid.to_lowercase().contains(&q)
                || r.source.to_lowercase().contains(&q)
        })
        .map(|(i, _)| i)
        .collect()
}

/// Restores the terminal (leave alternate screen, raw mode off, show cursor)
/// on every exit path, mirroring the TUI's `RawGuard`.
struct ScreenGuard;
impl ScreenGuard {
    fn enter() -> anyhow::Result<Self> {
        crossterm::terminal::enable_raw_mode()
            .context("enter raw mode (interactive terminal required)")?;
        {
            let mut out = std::io::stdout();
            crossterm::execute!(out, crossterm::terminal::EnterAlternateScreen)
                .context("enter alternate screen")?;
            crossterm::execute!(out, crossterm::cursor::Hide)?;
        }
        Ok(ScreenGuard)
    }
}
impl Drop for ScreenGuard {
    fn drop(&mut self) {
        let mut out = std::io::stdout();
        let _ = crossterm::execute!(out, crossterm::terminal::LeaveAlternateScreen);
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(out, crossterm::cursor::Show);
    }
}

/// Outcome of a browse interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowseOutcome {
    Selected(SessionId),
    Cancelled,
    /// The store held no sessions (`No sessions found.` was printed).
    Empty,
}

/// Pure frame builder for the curses-style browser (no SGR; the interactive
/// layer wraps the cursor line in reverse video). Returns the lines to draw
/// top to bottom, footer last.
pub fn frame_lines(
    rows: &[SessionRow],
    shown: &[usize],
    cursor: usize,
    filter: &str,
    name_width: usize,
    max_rows: usize,
) -> Vec<String> {
    let mut lines = Vec::new();
    if filter.is_empty() {
        lines.push(HINT_BROWSE.to_owned());
    } else {
        lines.push(hint_filter(filter));
    }
    lines.push(column_header(name_width));
    if shown.is_empty() {
        lines.push(NO_MATCH.to_owned());
    } else {
        let start = if cursor >= max_rows {
            cursor + 1 - max_rows
        } else {
            0
        };
        for &i in shown.iter().skip(start).take(max_rows) {
            let r = &rows[i];
            lines.push(format_row(
                &r.name,
                r.status,
                r.msgs,
                &r.last_active,
                &r.source,
                &r.sid,
                name_width,
            ));
        }
    }
    let cursor_one_based = if shown.is_empty() { 0 } else { cursor + 1 };
    lines.push(footer(cursor_one_based, shown.len(), rows.len()));
    lines
}

/// Numbered non-curses fallback (spec §F `L1639`). Prints the verbatim
/// header plus one numbered row per session and reads a single selection
/// line. `q`/empty/EOF cancels; anything else must be a 1-based number.
/// Generic over `BufRead`/`Write` so unit tests drive it with cursors.
pub fn browse_numbered<R: BufRead, W: Write>(
    store: &SessionStore,
    mut input: R,
    output: &mut W,
    term_width: u16,
) -> anyhow::Result<BrowseOutcome> {
    let rows = collect_rows(store)?;
    if rows.is_empty() {
        writeln!(output, "{NO_SESSIONS}")?;
        output.flush()?;
        return Ok(BrowseOutcome::Empty);
    }
    let nw = name_width(term_width.max(MIN_WIDTH));
    write!(output, "{FALLBACK_HEADER}")?;
    // Number gutter is 7 cells (`  [ 1] `); the header is padded equally so
    // the columns stay aligned with the numbered rows.
    writeln!(output, "       {}", column_header(nw).trim_start())?;
    for (n, r) in rows.iter().enumerate() {
        let body = format_row(
            &r.name,
            r.status,
            r.msgs,
            &r.last_active,
            &r.source,
            &r.sid,
            nw,
        );
        writeln!(output, "  [{:>2}] {}", n + 1, body.trim_start())?;
    }
    loop {
        write!(output, "select> ")?;
        output.flush()?;
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(BrowseOutcome::Cancelled);
        }
        let choice = line.trim();
        if choice.is_empty() || choice.eq_ignore_ascii_case("q") {
            return Ok(BrowseOutcome::Cancelled);
        }
        match choice.parse::<usize>() {
            Ok(n) if (1..=rows.len()).contains(&n) => {
                return Ok(BrowseOutcome::Selected(rows[n - 1].id));
            }
            _ => writeln!(output, "invalid selection (enter a number or q)")?,
        }
    }
}

/// Curses-style interactive browser (alternate screen + raw mode). The caller
/// must have verified an interactive terminal; too-small terminals print
/// `Terminal too small` and cancel. Notices (`Deleted.`/`Delete failed.`)
/// print after the screen is restored.
pub fn browse(store: &SessionStore) -> anyhow::Result<BrowseOutcome> {
    use crossterm::event::{self, Event, KeyCode, KeyModifiers};
    use crossterm::terminal;
    use crossterm::{cursor, execute};

    let (cols, term_rows) = terminal::size().context("query terminal size")?;
    if cols < MIN_WIDTH || term_rows < MIN_HEIGHT {
        println!("{TOO_SMALL}");
        return Ok(BrowseOutcome::Cancelled);
    }
    let mut rows = collect_rows(store)?;
    if rows.is_empty() {
        println!("{NO_SESSIONS}");
        return Ok(BrowseOutcome::Empty);
    }
    let nw = name_width(cols);

    let _guard = ScreenGuard::enter()?;

    let mut filter = String::new();
    let mut cursor_idx = 0usize;
    let mut notices: Vec<&str> = Vec::new();
    // Header (2) + footer (1) + one spare row.
    let max_rows = (term_rows as usize).saturating_sub(4).max(1);

    let outcome = loop {
        let shown = apply_filter(&rows, &filter);
        cursor_idx = cursor_idx.min(shown.len().saturating_sub(1));
        // Full redraw: home + clear + frame (flicker is acceptable here).
        {
            use crossterm::terminal::ClearType;
            let mut out = std::io::stdout();
            execute!(out, cursor::MoveTo(0, 0), terminal::Clear(ClearType::All))?;
            let frame = frame_lines(&rows, &shown, cursor_idx, &filter, nw, max_rows);
            let start = if cursor_idx >= max_rows {
                cursor_idx + 1 - max_rows
            } else {
                0
            };
            for (n, line) in frame.iter().enumerate() {
                // Row lines sit between header (1) and footer (last); the
                // cursor row gets reverse video.
                let is_cursor_row = !shown.is_empty()
                    && n >= 2
                    && n - 2 == cursor_idx.saturating_sub(start)
                    && n < frame.len() - 1;
                if is_cursor_row {
                    use crossterm::style::{Attribute, Print, SetAttribute};
                    execute!(
                        out,
                        SetAttribute(Attribute::Reverse),
                        Print(line),
                        SetAttribute(Attribute::Reset)
                    )?;
                } else {
                    use crossterm::style::Print;
                    execute!(out, Print(line))?;
                }
                execute!(out, cursor::MoveToNextLine(1))?;
            }
            out.flush()?;
        }
        if !event::poll(std::time::Duration::from_millis(100))? {
            continue;
        }
        match event::read()? {
            Event::Resize(_, _) => continue,
            Event::Key(key) => {
                // Like the TUI renderer: only press events drive input, so a
                // terminal that reports release/repeat never double-applies.
                if key.kind != event::KeyEventKind::Press {
                    continue;
                }
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    anyhow::bail!("interrupted");
                }
                match key.code {
                    KeyCode::Esc => break BrowseOutcome::Cancelled,
                    KeyCode::Enter => {
                        if let Some(&i) = shown.get(cursor_idx) {
                            break BrowseOutcome::Selected(rows[i].id);
                        }
                    }
                    KeyCode::Up => cursor_idx = cursor_idx.saturating_sub(1),
                    KeyCode::Down => {
                        if cursor_idx + 1 < shown.len() {
                            cursor_idx += 1;
                        }
                    }
                    KeyCode::Backspace => {
                        filter.pop();
                        cursor_idx = 0;
                    }
                    KeyCode::Char('d') if filter.is_empty() && !shown.is_empty() => {
                        let target = &rows[shown[cursor_idx]];
                        // Inline confirm on the bottom row; one raw keystroke.
                        let mut out = std::io::stdout();
                        execute!(
                            out,
                            cursor::MoveTo(0, term_rows - 1),
                            terminal::Clear(terminal::ClearType::CurrentLine)
                        )?;
                        {
                            use crossterm::style::Print;
                            execute!(out, Print(delete_prompt(&target.name)))?;
                        }
                        out.flush()?;
                        let confirm = loop {
                            match event::read()? {
                                Event::Key(k) if k.kind == event::KeyEventKind::Press => break k,
                                Event::Resize(_, _) => continue,
                                _ => continue,
                            }
                        };
                        if confirm.modifiers.contains(KeyModifiers::CONTROL)
                            && confirm.code == KeyCode::Char('c')
                        {
                            anyhow::bail!("interrupted");
                        }
                        if matches!(confirm.code, KeyCode::Char('y') | KeyCode::Char('Y')) {
                            match store.delete_session(&target.id) {
                                Ok(true) => notices.push(DELETED),
                                _ => notices.push(DELETE_FAILED),
                            }
                            rows = collect_rows(store)?;
                            if rows.is_empty() {
                                break BrowseOutcome::Empty;
                            }
                        }
                        cursor_idx = 0;
                    }
                    KeyCode::Char(c)
                        if !key.modifiers.contains(KeyModifiers::CONTROL)
                            && !key.modifiers.contains(KeyModifiers::ALT)
                            && c != '\n'
                            && c != '\r' =>
                    {
                        filter.push(c);
                        cursor_idx = 0;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    };
    drop(_guard);
    for notice in notices {
        println!("{notice}");
    }
    if outcome == BrowseOutcome::Empty {
        println!("{NO_SESSIONS}");
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_rows() -> Vec<SessionRow> {
        vec![
            SessionRow {
                id: "550e8400-e29b-41d4-a716-446655440000".parse().unwrap(),
                name: "deploy the thing".into(),
                status: "done",
                msgs: 12,
                last_active: "2h ago".into(),
                source: "cli".into(),
                sid: "550e8400".into(),
            },
            SessionRow {
                id: "660f8400-e29b-41d4-a716-446655440001".parse().unwrap(),
                name: "(empty)".into(),
                status: "empty",
                msgs: 0,
                last_active: "just now".into(),
                source: "tui".into(),
                sid: "660f8400".into(),
            },
        ]
    }

    #[test]
    fn hint_header_row_footer_are_verbatim_shapes() {
        assert_eq!(
            HINT_BROWSE,
            "  Browse sessions — ↑↓ navigate  Enter select  Type to filter  Esc quit"
        );
        assert_eq!(hint_filter("dep"), "  Browse sessions — filter: dep█");
        assert_eq!(
            column_header(20),
            "  Title / Preview       Stat    Msgs  Active      Src   ID"
        );
        assert_eq!(
            format_row("deploy", "done", 12, "2h ago", "cli", "550e8400", 20),
            "  deploy                done      12  2h ago      cli   550e8400"
        );
        assert_eq!(footer(1, 2, 2), "  1/2 sessions   d delete");
        assert_eq!(
            footer(1, 1, 2),
            "  1/1 sessions (filtered from 2)   d delete"
        );
        assert_eq!(delete_prompt("deploy"), "  Delete session 'deploy'? [y/N]");
        assert_eq!(NO_SESSIONS, "No sessions found.");
        assert_eq!(NO_MATCH, "  No sessions match the filter.");
        assert_eq!(TOO_SMALL, "Terminal too small");
        assert_eq!(
            FALLBACK_HEADER,
            "\n  Browse sessions  (enter number to resume, q to cancel)\n"
        );
    }

    #[test]
    fn row_truncates_long_names_to_the_column() {
        let row = format_row(
            "abcdefghijklmnopqrstuvwxyz",
            "done",
            1,
            "now",
            "cli",
            "abcdef01",
            10,
        );
        assert!(row.contains("abcdefghij"), "{row}");
        assert!(!row.contains("klmnop"), "{row}");
    }

    #[test]
    fn name_width_fits_an_80_column_terminal() {
        assert_eq!(name_width(80), 36);
        assert_eq!(name_width(60), 16);
        assert_eq!(name_width(40), 10);
        assert_eq!(name_width(200), 48);
    }

    #[test]
    fn relative_active_stays_within_ten_cells() {
        let now = 1_700_000_000.0;
        for (then, want) in [
            (now - 5.0, "just now"),
            (now - 300.0, "5m ago"),
            (now - 7_200.0, "2h ago"),
            (now - 3.0 * 86_400.0, "3d ago"),
            (now - 60.0 * 86_400.0, "8w ago"),
            (now - 400.0 * 86_400.0, "1y ago"),
        ] {
            let got = relative_active(now, then);
            assert_eq!(got, want);
            assert!(got.len() <= 10, "{got}");
        }
    }

    #[test]
    fn filter_matches_name_sid_and_source_case_insensitively() {
        let rows = fixture_rows();
        assert_eq!(apply_filter(&rows, ""), vec![0, 1]);
        assert_eq!(apply_filter(&rows, "DEPLOY"), vec![0]);
        assert_eq!(apply_filter(&rows, "660f"), vec![1]);
        assert_eq!(apply_filter(&rows, "tui"), vec![1]);
        assert!(apply_filter(&rows, "nope").is_empty());
    }

    #[test]
    fn frame_lines_cover_hint_header_rows_footer_and_no_match() {
        let rows = fixture_rows();
        let shown = apply_filter(&rows, "");
        let frame = frame_lines(&rows, &shown, 0, "", 20, 10);
        assert_eq!(frame[0], HINT_BROWSE);
        assert_eq!(frame[1], column_header(20));
        assert!(frame[2].contains("deploy the thing"), "{frame:?}");
        assert!(frame[3].contains("(empty)"), "{frame:?}");
        assert_eq!(*frame.last().unwrap(), "  1/2 sessions   d delete");

        let shown = apply_filter(&rows, "zzz");
        let frame = frame_lines(&rows, &shown, 0, "zzz", 20, 10);
        assert_eq!(frame[0], "  Browse sessions — filter: zzz█");
        assert!(frame.contains(&NO_MATCH.to_owned()), "{frame:?}");
        assert_eq!(
            *frame.last().unwrap(),
            "  0/0 sessions (filtered from 2)   d delete"
        );
    }

    fn temp_store() -> (tempfile::TempDir, SessionStore) {
        let dir = tempfile::TempDir::new().unwrap();
        let store = SessionStore::open(&dir.path().join("state.db")).unwrap();
        (dir, store)
    }

    #[test]
    fn collect_rows_sanitizes_and_single_lines_names() {
        let (_dir, mut store) = temp_store();
        let id = store.create_session("cli").unwrap();
        store
            .save_turn(
                &id,
                &hermes_core::conversation::Turn::User {
                    content: "line one\nline two \u{1b}[31mred".into(),
                },
            )
            .unwrap();
        let rows = collect_rows(&store).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].name, "line one line two red",
            "newlines folded, ANSI stripped"
        );
        assert_eq!(rows[0].status, "done");
        assert_eq!(rows[0].msgs, 1);
        assert_eq!(rows[0].sid.len(), 8);
    }

    #[test]
    fn collect_rows_marks_empty_sessions() {
        let (_dir, store) = temp_store();
        let id = store.create_session("tui").unwrap();
        let rows = collect_rows(&store).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, id);
        assert_eq!(rows[0].name, "(empty)");
        assert_eq!(rows[0].status, "empty");
        assert_eq!(rows[0].msgs, 0);
    }

    #[test]
    fn numbered_fallback_selects_cancels_and_rejects_garbage() {
        let (_dir, store) = temp_store();
        let a = store.create_session("cli").unwrap();
        let _b = store.create_session("cli").unwrap();
        // Newest first: `1` picks the second-created session.
        let mut out = Vec::new();
        let outcome = browse_numbered(&store, std::io::Cursor::new(b"1\n"), &mut out, 80).unwrap();
        assert_eq!(outcome, BrowseOutcome::Selected(store.list().unwrap()[0]));
        let text = String::from_utf8(out).unwrap();
        assert!(
            text.contains("  Browse sessions  (enter number to resume, q to cancel)"),
            "{text}"
        );
        assert!(text.contains("Title / Preview"), "{text}");
        assert!(
            text.contains('1') && text.contains(&a.to_string()[..8]),
            "{text}"
        );

        for quit in ["q\n", "Q\n", "\n", ""] {
            let mut out = Vec::new();
            let outcome =
                browse_numbered(&store, std::io::Cursor::new(quit.as_bytes()), &mut out, 80)
                    .unwrap();
            assert_eq!(outcome, BrowseOutcome::Cancelled, "quit={quit:?}");
        }
        let mut out = Vec::new();
        let outcome =
            browse_numbered(&store, std::io::Cursor::new(b"99\nxx\n2\n"), &mut out, 80).unwrap();
        assert_eq!(outcome, BrowseOutcome::Selected(a));
        let text = String::from_utf8(out).unwrap();
        assert_eq!(
            text.matches("invalid selection (enter a number or q)")
                .count(),
            2,
            "{text}"
        );
    }

    #[test]
    fn numbered_fallback_reports_an_empty_store() {
        let (_dir, store) = temp_store();
        let mut out = Vec::new();
        let outcome = browse_numbered(&store, std::io::Cursor::new(b""), &mut out, 80).unwrap();
        assert_eq!(outcome, BrowseOutcome::Empty);
        assert_eq!(String::from_utf8(out).unwrap(), "No sessions found.\n");
    }
}
