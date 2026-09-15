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
//! * `status` follows the reference's own lifecycle classifier over a
//!   session's **last message row** (`hermes_state.classify_session_status`,
//!   pinned at `docs/hermes-ui-spec/017/evidence/upstream-lifecycle-status/`):
//!   an error `finish_reason` gives `err`, an unanswered turn — a user/tool row
//!   or an assistant row whose tool call has no result — gives `intr`, any other
//!   last row gives `done`, and a session with no message row gives `empty`.
//!   `finish_reason` and `tool_calls` are read only when the database carries
//!   them (databases written by Hermes Python do, ones this crate creates do
//!   not), so on a Rust-created database `err` cannot arise — exactly as in the
//!   reference, which has no other source for it either.
//! * when that status cannot be read at all — the query fails — the column
//!   renders `-` and the picker stays up. The reference's
//!   `_annotate_session_statuses` swallows every error from the same query
//!   (contract 7), so an unreadable database costs one column, not the picker.
//! * `name` is the first user message (single-lined); Hermes-RS has no
//!   session titles yet, so there is no title half of `Title / Preview`.
//! * The cursor row is painted with palette slot 2 + bold, and every row that
//!   is not the cursor gets its five-cell status tag recoloured by Python's
//!   `_status_attr` contract (complete/pair1 green, interrupted/pair2 yellow,
//!   error/pair5 red, empty/pair4 palette8, otherwise A_NORMAL). Both the tag
//!   wording and the ink were read out of the pinned upstream source, kept at
//!   `docs/hermes-ui-spec/017/evidence/upstream-status-attr/`, because §F named
//!   `_status_attr` without its mapping.
//! * `q` is a filter character in the curses-style browser (the hint lists
//!   `Esc quit` only); `q to cancel` exists solely in the numbered fallback.
//! * The frame is drawn when the screen changed, not on every loop turn: the
//!   reference draws at the top of its loop and then blocks in `getch()`, so it
//!   paints once per key and nothing while it waits. The port keeps polling (100
//!   ms) to stay responsive to signals, but only repaints when a key or a resize
//!   marked the screen dirty.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use crossterm::style::Color;
use hermes_core::session::{SessionId, SessionStatus, SessionStore};

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

/// Narrowest terminal that still draws the picker. The pinned reference
/// (`_session_browse_picker`) refuses `max_y < 5 or max_x < 40`, so 40 columns is
/// usable (the fixed columns get clipped) and anything below that shows the notice.
pub const PICKER_MIN_COLUMNS: u16 = 40;
pub const PICKER_MIN_ROWS: u16 = 5;

/// Width floor for the numbered non-curses fallback's own padding, which the
/// reference does not constrain. The curses picker uses `PICKER_MIN_*` instead.
pub const MIN_WIDTH: u16 = 60;

/// Hint line while filter text is typed (spec §F, verbatim shape).
pub fn hint_filter(search: &str) -> String {
    format!("  Browse sessions — filter: {search}█")
}

/// Column-header name field. The pinned 100/80 reference draws the header with
/// its own field (width-59, floored at the 80-column value of 20), so `Stat`
/// lands three cells right of the body status column, as the reference does.
pub fn header_name_width(term_width: u16) -> usize {
    (term_width as usize).saturating_sub(59).max(20)
}

/// Column header (spec §F, verbatim shape; the pinned three-cell indent).
pub fn column_header(term_width: u16) -> String {
    format!(
        "   {:<nw$}  {:<5}  {:>5}  {:<10}  {:<5} {}",
        "Title / Preview",
        "Stat",
        "Msgs",
        "Active",
        "Src",
        "ID",
        nw = header_name_width(term_width),
    )
}

/// One session row (spec §F, verbatim shape). The f-string in L1387 carries no
/// leading indent, but curses draws it behind the three-cell cursor column the
/// reference shows (` → ` on the cursor row, three spaces otherwise).
pub fn format_row(row: &SessionRow, name_width: usize, selected: bool) -> String {
    format!(
        "{}{:<nw$}  {:<5}  {:>5}  {:<10}  {:<5} {}",
        row_prefix(selected),
        truncate_chars(&row.name, name_width),
        row.status,
        row.msgs,
        row.last_active,
        row.source,
        row.sid,
        nw = name_width,
    )
}

/// Footer line (spec §F, verbatim shape).
pub fn footer(cursor_one_based: usize, shown: usize, total: usize, can_delete: bool) -> String {
    if shown == 0 {
        return format!("  0/{total} sessions");
    }
    let mut out = format!("  {cursor_one_based}/{shown} sessions");
    if shown != total {
        out.push_str(&format!(" (filtered from {total})"));
    }
    if can_delete {
        out.push_str("   d delete");
    }
    out
}

/// Delete confirmation prompt (spec §F, verbatim shape, explicit `[y/N]`).
pub fn delete_prompt(label: &str) -> String {
    format!("  Delete session '{label}'? [y/N]")
}

/// Body name-column width for a terminal width. The pinned 100/80 reference
/// puts the body status column at width-57, i.e. a name field of width-62
/// floored at the 80-column value of 20; terminals narrower than the reference
/// are not evidenced.
pub fn name_width(term_width: u16) -> usize {
    (term_width as usize).saturating_sub(62).max(20)
}

/// Ink of the five-cell status tag (`_status_attr` in the pinned upstream
/// source, kept under `docs/hermes-ui-spec/017/evidence/upstream-status-attr/`):
/// `done` pair1 green, `intr` pair2 yellow, `err` pair5 red, `empty` pair4
/// palette8; any other tag keeps the terminal's normal ink (A_NORMAL).
pub fn status_ink(status: &str) -> Color {
    match status {
        "done" => Color::DarkGreen,
        "intr" => Color::DarkYellow,
        "err" => Color::DarkRed,
        "empty" => Color::DarkGrey,
        _ => Color::Reset,
    }
}

/// Character span of the status tag inside a body row: the three-cell cursor
/// column, the name field and its two separator cells, i.e. `3 + name + 2`
/// cells in, exactly `_status_attr`'s `tag_x = 3 + max(20, max_x - 62) + 2`.
pub fn status_tag_span(name_width: usize) -> std::ops::Range<usize> {
    let start = 3 + name_width + 2;
    start..start + 5
}

/// Byte offset of a character index, clamped to the end of the string.
fn char_offset(line: &str, char_index: usize) -> usize {
    line.char_indices()
        .nth(char_index)
        .map(|(offset, _)| offset)
        .unwrap_or(line.len())
}

/// Three-cell cursor column shared by every body row. The reference draws
/// ` → ` on the cursor row and three spaces on the others.
pub fn row_prefix(selected: bool) -> &'static str {
    if selected {
        " \u{2192} "
    } else {
        "   "
    }
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

/// The five-cell `Stat` tag for one session, given the result of the one
/// grouped lifecycle query.
///
/// `None` means that query **failed**, which is not the same as a session
/// missing from the map: a missing session has no message row at all and
/// renders `empty`, while a failed query renders `-`. The distinction is the
/// reference's contract 7 — `_annotate_session_statuses` swallows the error
/// so the picker degrades instead of failing — and dropping it would turn an
/// unreadable column into an unopenable picker.
fn status_tag(statuses: Option<&HashMap<String, SessionStatus>>, id: &str) -> &'static str {
    match statuses {
        Some(map) => map.get(id).copied().unwrap_or(SessionStatus::Empty).tag(),
        None => SessionStatus::Unknown.tag(),
    }
}

/// Collect display rows from the canonical store. Names and sources pass
/// through the render-boundary sanitizer (ANSI/C0 stripped); the canonical
/// bytes are never touched.
pub fn collect_rows(store: &SessionStore) -> anyhow::Result<Vec<SessionRow>> {
    let now = now_secs();
    let mut rows = Vec::new();
    // One grouped query for the listed sessions, not one per row: the status is
    // derived from each session's last message row, and the grouping is
    // narrowed to the ids listed here the way the reference narrows it. A
    // session that ends up `empty` is one with no message row at all.
    //
    // Contract 7 of the pinned reference: `_annotate_session_statuses` swallows
    // every error this query can raise, so a database whose lifecycle columns
    // cannot be read costs one column (`-`) instead of the whole picker.
    let ids = store.list()?;
    let statuses = store.lifecycle_statuses(&ids).ok();
    for id in ids {
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
            status: status_tag(statuses.as_ref(), &full),
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

/// Reference `_curses_browse` navigation: the cursor is modulo — Down on the
/// last item lands on the first, Up on the first item lands on the last.
pub fn wrapped_cursor(cursor: usize, len: usize, down: bool) -> usize {
    if len == 0 {
        return 0;
    }
    if down {
        (cursor + 1) % len
    } else if cursor == 0 {
        len - 1
    } else {
        cursor - 1
    }
}

/// Reference `_curses_browse` scroll window: `scroll_offset` only moves when
/// the cursor leaves the visible window, and then by exactly enough to bring
/// it back (moving up never recentres; wrapping to item 0 snaps to the top).
pub fn clamped_offset(offset: usize, cursor: usize, max_rows: usize) -> usize {
    let rows = max_rows.max(1);
    if cursor < offset {
        cursor
    } else if cursor >= offset + rows {
        cursor + 1 - rows
    } else {
        offset
    }
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
/// layer styles the hint, the column header, the cursor row and the footer).
/// Returns the lines to draw top to bottom, footer last. The reference keeps
/// one blank row between the column header and the body.
pub fn frame_lines(
    rows: &[SessionRow],
    shown: &[usize],
    cursor: usize,
    start: usize,
    filter: &str,
    term_width: u16,
    max_rows: usize,
) -> Vec<String> {
    let mut lines = Vec::new();
    if filter.is_empty() {
        lines.push(HINT_BROWSE.to_owned());
    } else {
        lines.push(hint_filter(filter));
    }
    lines.push(column_header(term_width));
    lines.push(String::new());
    if shown.is_empty() {
        lines.push(NO_MATCH.to_owned());
    } else {
        let nw = name_width(term_width);
        let selected = shown.get(cursor).copied();
        for &i in shown.iter().skip(start).take(max_rows) {
            lines.push(format_row(&rows[i], nw, selected == Some(i)));
        }
    }
    let cursor_one_based = if shown.is_empty() { 0 } else { cursor + 1 };
    // Keep the hint aligned with the d-key guard in browse().
    let can_delete = filter.is_empty() && !shown.is_empty();
    lines.push(footer(
        cursor_one_based,
        shown.len(),
        rows.len(),
        can_delete,
    ));
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
    let term_width = term_width.max(MIN_WIDTH);
    let nw = name_width(term_width);
    write!(output, "{FALLBACK_HEADER}")?;
    // Number gutter is 7 cells (`  [ 1] `); the header is padded equally so
    // the columns stay aligned with the numbered rows.
    writeln!(output, "       {}", column_header(term_width).trim_start())?;
    for (n, r) in rows.iter().enumerate() {
        let body = format_row(r, nw, false);
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

    let (mut cols, mut term_rows) = terminal::size().context("query terminal size")?;
    let mut rows = collect_rows(store)?;
    // The reference reports an empty store before curses ever starts, so an empty
    // store on a small terminal still prints `No sessions found.`.
    if rows.is_empty() {
        println!("{NO_SESSIONS}");
        return Ok(BrowseOutcome::Empty);
    }
    let _guard = ScreenGuard::enter()?;
    if cols < PICKER_MIN_COLUMNS || term_rows < PICKER_MIN_ROWS {
        // The reference clears the screen, writes the notice and then blocks in
        // `getch()`: the notice stays visible until a key is pressed, and a narrow
        // terminal never falls through to the picker.
        let mut notice = std::io::stdout();
        execute!(
            notice,
            cursor::MoveTo(0, 0),
            terminal::Clear(terminal::ClearType::All),
            crossterm::style::Print(TOO_SMALL)
        )?;
        notice.flush()?;
        loop {
            match event::read()? {
                Event::Key(key) if key.kind == event::KeyEventKind::Press => break,
                Event::Resize(_, _) => continue,
                _ => continue,
            }
        }
        return Ok(BrowseOutcome::Cancelled);
    }

    let mut filter = String::new();
    let mut cursor_idx = 0usize;
    // Reference `_curses_browse` keeps `scroll_offset` as state: it only moves
    // when the cursor leaves the window, and every filter change resets it.
    let mut scroll_offset = 0usize;
    let mut notices: Vec<&str> = Vec::new();
    // Hint, column header, blank separator and footer occupy four rows.
    let mut max_rows = (term_rows as usize).saturating_sub(4).max(1);

    // The reference draws once and then blocks in `getch()`; repainting only when
    // something changed keeps that cadence while the poll loop stays responsive.
    let mut dirty = true;
    let outcome = loop {
        let shown = apply_filter(&rows, &filter);
        cursor_idx = cursor_idx.min(shown.len().saturating_sub(1));
        scroll_offset = clamped_offset(scroll_offset, cursor_idx, max_rows);
        if dirty {
            // Full redraw: home + clear + frame (flicker is acceptable here).
            use crossterm::terminal::ClearType;
            let mut out = std::io::stdout();
            execute!(out, cursor::MoveTo(0, 0), terminal::Clear(ClearType::All))?;
            let frame = frame_lines(
                &rows,
                &shown,
                cursor_idx,
                scroll_offset,
                &filter,
                cols,
                max_rows,
            );
            for (n, line) in frame.iter().enumerate() {
                let is_footer = n == frame.len() - 1;
                // `addnstr(..., max_x - 1, ...)` in the reference: clip, never wrap.
                let line = truncate_chars(line, (cols as usize).saturating_sub(1));
                if is_footer {
                    execute!(out, cursor::MoveTo(0, term_rows - 1))?;
                }
                // Body rows sit between the blank separator (index 2) and the
                // footer; the cursor row still gets reverse video.
                let is_cursor_row = !shown.is_empty()
                    && n >= 3
                    && n - 3 == cursor_idx.saturating_sub(scroll_offset)
                    && n < frame.len() - 1;
                if n == 0 {
                    use crossterm::style::{
                        Attribute, Color, Print, SetAttribute, SetForegroundColor,
                    };
                    let ink = if filter.is_empty() {
                        Color::DarkYellow
                    } else {
                        Color::DarkCyan
                    };
                    execute!(
                        out,
                        SetForegroundColor(ink),
                        SetAttribute(Attribute::Bold),
                        Print(line),
                        SetAttribute(Attribute::Reset)
                    )?;
                } else if n == 1 {
                    use crossterm::style::{Color, Print, SetForegroundColor};
                    execute!(
                        out,
                        SetForegroundColor(Color::DarkGrey),
                        Print(line),
                        SetForegroundColor(Color::Reset)
                    )?;
                } else if line.as_str() == NO_MATCH {
                    // The reference writes the message with the dim attribute
                    // (SGR 2) and only resets it at the footer redraw; resetting
                    // in place renders identically and never leaks dim.
                    use crossterm::style::{Attribute, Print, SetAttribute};
                    execute!(
                        out,
                        SetAttribute(Attribute::Dim),
                        Print(line),
                        SetAttribute(Attribute::Reset)
                    )?;
                } else if is_cursor_row {
                    // The reference paints the whole selected row with palette
                    // slot 2 + bold and never uses reverse video.
                    use crossterm::style::{
                        Attribute, Color, Print, SetAttribute, SetForegroundColor,
                    };
                    execute!(
                        out,
                        SetForegroundColor(Color::DarkGreen),
                        SetAttribute(Attribute::Bold),
                        Print(line),
                        SetAttribute(Attribute::Reset)
                    )?;
                } else if is_footer {
                    use crossterm::style::{Color, Print, SetForegroundColor};
                    execute!(
                        out,
                        SetForegroundColor(Color::DarkGrey),
                        Print(line),
                        SetForegroundColor(Color::Reset)
                    )?;
                } else {
                    // Only the five-cell status tag is recoloured, and only on
                    // rows that are not the cursor (pinned `_session_browse_picker`).
                    use crossterm::style::{Print, SetForegroundColor};
                    let span = status_tag_span(name_width(cols));
                    // Rows above the body (hint, header, separator) must not
                    // index into `shown`; `n - 3` would underflow there.
                    let body = if n < 3 {
                        None
                    } else {
                        shown.get(scroll_offset + (n - 3)).copied()
                    };
                    match body.filter(|_| line.chars().count() >= span.end) {
                        Some(index) => {
                            let head = char_offset(&line, span.start);
                            let tag = char_offset(&line, span.end);
                            execute!(
                                out,
                                Print(&line[..head]),
                                SetForegroundColor(status_ink(rows[index].status)),
                                Print(&line[head..tag]),
                                SetForegroundColor(Color::Reset),
                                Print(&line[tag..])
                            )?;
                        }
                        None => execute!(out, Print(line))?,
                    }
                }
                if !is_footer {
                    execute!(out, cursor::MoveToNextLine(1))?;
                }
            }
            out.flush()?;
            dirty = false;
        }
        // The reference re-reads its geometry at the top of every loop turn.
        // Crossterm surfaces SIGWINCH as `Event::Resize`; as a fallback also
        // poll the size on a wait timeout, so a missed signal costs at most
        // one 100 ms tick instead of leaving a stale frame (and the
        // redraw-on-input cadence is untouched: no size change, no repaint).
        let event = if !event::poll(std::time::Duration::from_millis(100))? {
            match terminal::size() {
                Ok(size) if size != (cols, term_rows) => Event::Resize(size.0, size.1),
                _ => continue,
            }
        } else {
            event::read()?
        };
        match event {
            Event::Resize(width, height) => {
                cols = width;
                term_rows = height;
                if cols < PICKER_MIN_COLUMNS || term_rows < PICKER_MIN_ROWS {
                    // The reference re-checks its minimum at the top of every
                    // loop turn: a terminal that shrank below 5 rows / 40
                    // columns shows the notice and exits at the next key.
                    let mut notice = std::io::stdout();
                    execute!(
                        notice,
                        cursor::MoveTo(0, 0),
                        terminal::Clear(terminal::ClearType::All),
                        crossterm::style::Print(TOO_SMALL)
                    )?;
                    notice.flush()?;
                    loop {
                        match event::read()? {
                            Event::Key(key) if key.kind == event::KeyEventKind::Press => break,
                            _ => continue,
                        }
                    }
                    break BrowseOutcome::Cancelled;
                }
                max_rows = (term_rows as usize).saturating_sub(4).max(1);
                dirty = true;
                continue;
            }
            Event::Key(key) => {
                // Like the TUI renderer: only press events drive input, so a
                // terminal that reports release/repeat never double-applies.
                if key.kind != event::KeyEventKind::Press {
                    continue;
                }
                dirty = true;
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    anyhow::bail!("interrupted");
                }
                match key.code {
                    KeyCode::Esc => {
                        if filter.is_empty() {
                            break BrowseOutcome::Cancelled;
                        }
                        // Reference: the first Esc clears the search — full
                        // list, cursor and window back to zero; only the
                        // second Esc exits.
                        filter.clear();
                        cursor_idx = 0;
                        scroll_offset = 0;
                    }
                    KeyCode::Enter => {
                        if let Some(&i) = shown.get(cursor_idx) {
                            break BrowseOutcome::Selected(rows[i].id);
                        }
                    }
                    KeyCode::Up => cursor_idx = wrapped_cursor(cursor_idx, shown.len(), false),
                    KeyCode::Down => cursor_idx = wrapped_cursor(cursor_idx, shown.len(), true),
                    KeyCode::Backspace => {
                        filter.pop();
                        cursor_idx = 0;
                        scroll_offset = 0;
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
                            use crossterm::style::{
                                Attribute, Color, Print, SetAttribute, SetForegroundColor,
                            };
                            execute!(
                                out,
                                SetForegroundColor(Color::DarkRed),
                                SetAttribute(Attribute::Bold),
                                Print(delete_prompt(&target.name)),
                                SetAttribute(Attribute::Reset)
                            )?;
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
                        scroll_offset = 0;
                    }
                    KeyCode::Char(c)
                        if !key.modifiers.contains(KeyModifiers::CONTROL)
                            && !key.modifiers.contains(KeyModifiers::ALT)
                            && c != '\n'
                            && c != '\r' =>
                    {
                        filter.push(c);
                        cursor_idx = 0;
                        scroll_offset = 0;
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

    fn row(
        name: &str,
        status: &'static str,
        msgs: usize,
        last_active: &str,
        sid: &str,
    ) -> SessionRow {
        SessionRow {
            id: "550e8400-e29b-41d4-a716-446655440000".parse().unwrap(),
            name: name.into(),
            status,
            msgs,
            last_active: last_active.into(),
            source: "cli".into(),
            sid: sid.into(),
        }
    }

    #[test]
    fn picker_no_match_counter_preserves_total() {
        let rows = fixture_rows();
        let shown = apply_filter(&rows, "zzzz");
        let frame = frame_lines(&rows, &shown, 0, 0, "zzzz", 20, 10);
        // Independent pinned Python two-session fixture, not a recomputed count.
        assert_eq!(frame.last().unwrap(), "  0/2 sessions");
    }

    #[test]
    fn picker_footer_tracks_delete_availability() {
        let mut rows = fixture_rows();
        rows.truncate(1);
        // A nonempty filter can match ALL rows; counts do not encode filter state.
        for (query, expected) in [
            ("", "  1/1 sessions   d delete"),
            ("CLI", "  1/1 sessions"),
            ("zzzz", "  0/1 sessions"),
        ] {
            let shown = apply_filter(&rows, query);
            let frame = frame_lines(&rows, &shown, 0, 0, query, 20, 10);
            assert_eq!(frame.last().unwrap(), expected, "query={query:?}");
        }
        let frame = frame_lines(&[], &[], 0, 0, "", 20, 10);
        assert_eq!(frame.last().unwrap(), "  0/0 sessions");
    }

    #[test]
    fn hint_header_row_footer_are_verbatim_shapes() {
        assert_eq!(
            HINT_BROWSE,
            "  Browse sessions — ↑↓ navigate  Enter select  Type to filter  Esc quit"
        );
        assert_eq!(hint_filter("dep"), "  Browse sessions — filter: dep█");
        assert_eq!(
            column_header(100),
            "   Title / Preview                            Stat    Msgs  Active      Src   ID"
        );
        assert_eq!(
            format_row(&row("deploy", "done", 12, "2h ago", "550e8400"), 20, false),
            "   deploy                done      12  2h ago      cli   550e8400"
        );
        assert_eq!(footer(1, 2, 2, true), "  1/2 sessions   d delete");
        assert_eq!(footer(1, 1, 2, false), "  1/1 sessions (filtered from 2)");
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
    fn too_small_threshold_follows_the_pinned_reference() {
        assert_eq!(PICKER_MIN_COLUMNS, 40);
        assert_eq!(PICKER_MIN_ROWS, 5);
        // The numbered fallback keeps its own, wider floor.
        assert_eq!(MIN_WIDTH, 60);
        assert_eq!(TOO_SMALL, "Terminal too small");
    }

    #[test]
    fn a_drawn_row_is_clipped_not_wrapped() {
        assert_eq!(truncate_chars("  Browse sessions", 39), "  Browse sessions");
        assert_eq!(truncate_chars("", 5), "");
        let wide = format_row(
            &row("deploy the thing", "done", 12, "2h ago", "550e8400"),
            name_width(40),
            false,
        );
        assert!(wide.chars().count() > 39, "{wide:?}");
        assert_eq!(truncate_chars(&wide, 39).chars().count(), 39);
        assert_eq!(
            truncate_chars(&wide, 39),
            wide.chars().take(39).collect::<String>()
        );
    }

    #[test]
    fn status_ink_follows_the_pinned_mapping() {
        assert_eq!(status_ink("done"), Color::DarkGreen);
        assert_eq!(status_ink("intr"), Color::DarkYellow);
        assert_eq!(status_ink("err"), Color::DarkRed);
        assert_eq!(status_ink("empty"), Color::DarkGrey);
        assert_eq!(status_ink("-"), Color::Reset);
    }

    #[test]
    fn status_tag_span_sits_three_plus_name_width_plus_two() {
        assert_eq!(status_tag_span(name_width(100)), 43..48);
        assert_eq!(status_tag_span(name_width(80)), 25..30);
        let wide = format_row(
            &row("deploy", "done", 12, "2h ago", "550e8400"),
            name_width(100),
            false,
        );
        assert_eq!(&wide[43..48], "done ");
        let narrow = format_row(
            &row("deploy", "done", 12, "2h ago", "550e8400"),
            name_width(80),
            false,
        );
        assert_eq!(&narrow[25..30], "done ");
        assert_eq!(char_offset(&wide, 43), 43);
        assert_eq!(char_offset("  → deploy", 4), 6);
    }

    #[test]
    fn row_truncates_long_names_to_the_column() {
        let line = format_row(
            &row("abcdefghijklmnopqrstuvwxyz", "done", 1, "now", "abcdef01"),
            10,
            false,
        );
        assert!(line.contains("abcdefghij"), "{line}");
        assert!(!line.contains("klmnop"), "{line}");
    }

    #[test]
    fn name_width_matches_the_two_pinned_widths() {
        // Body field width-62 and header field width-59, both floored at the
        // 80-column value 20, reproduce the retained 100/80 reference rows.
        assert_eq!(name_width(100), 38);
        assert_eq!(name_width(80), 20);
        assert_eq!(name_width(60), 20);
        assert_eq!(header_name_width(100), 41);
        assert_eq!(header_name_width(80), 21);
        assert_eq!(
            format_row(
                &row("second topic", "done", 1, "2023-11-14", "660f8400"),
                name_width(100),
                true,
            ),
            " → second topic                            done       1  2023-11-14  cli   660f8400"
        );
        assert_eq!(column_header(100).find("Stat"), Some(46));
        assert_eq!(column_header(80).find("Stat"), Some(26));
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
        let frame = frame_lines(&rows, &shown, 0, 0, "", 20, 10);
        assert_eq!(frame[0], HINT_BROWSE);
        assert_eq!(frame[1], column_header(20));
        assert_eq!(frame[2], "");
        assert!(frame[3].contains("deploy the thing"), "{frame:?}");
        assert!(frame[4].contains("(empty)"), "{frame:?}");
        assert_eq!(*frame.last().unwrap(), "  1/2 sessions   d delete");

        let shown = apply_filter(&rows, "zzz");
        let frame = frame_lines(&rows, &shown, 0, 0, "zzz", 20, 10);
        assert_eq!(frame[0], "  Browse sessions — filter: zzz█");
        assert!(frame.contains(&NO_MATCH.to_owned()), "{frame:?}");
        assert_eq!(*frame.last().unwrap(), "  0/2 sessions");
    }

    #[test]
    fn picker_cursor_wraps_modulo_like_the_reference() {
        assert_eq!(
            wrapped_cursor(0, 3, false),
            2,
            "Up from the first item wraps to the last"
        );
        assert_eq!(
            wrapped_cursor(2, 3, true),
            0,
            "Down from the last item wraps to the first"
        );
        assert_eq!(wrapped_cursor(1, 3, true), 2);
        assert_eq!(wrapped_cursor(1, 3, false), 0);
        assert_eq!(wrapped_cursor(0, 0, true), 0, "an empty list never moves");
        assert_eq!(wrapped_cursor(0, 0, false), 0);
    }

    #[test]
    fn picker_window_moves_only_enough_to_keep_cursor_visible() {
        // Down: the window is still until the cursor passes its bottom edge,
        // then shifts by exactly the overflow.
        assert_eq!(clamped_offset(0, 4, 5), 0);
        assert_eq!(clamped_offset(0, 5, 5), 1);
        assert_eq!(clamped_offset(1, 6, 5), 2);
        // Up: the window is still until the cursor passes its top edge —
        // moving up never recentres (the stale recompute did).
        assert_eq!(clamped_offset(3, 4, 5), 3);
        assert_eq!(clamped_offset(3, 2, 5), 2);
        // Wrapping down to item 0 snaps the window back to the top.
        assert_eq!(clamped_offset(25, 0, 26), 0);
    }

    fn long_fixture_rows(count: usize) -> Vec<SessionRow> {
        (0..count)
            .map(|i| SessionRow {
                id: "550e8400-e29b-41d4-a716-446655440000".parse().unwrap(),
                name: format!("session-{i:02}"),
                status: "done",
                msgs: 1,
                last_active: "just now".into(),
                source: "cli".into(),
                sid: "550e8400".into(),
            })
            .collect()
    }

    #[test]
    fn frame_lines_render_the_stateful_window_not_a_cursor_pinned_one() {
        let rows = long_fixture_rows(30);
        let shown: Vec<usize> = (0..30).collect();
        // After wrapping up to the last item the window clamps to the bottom:
        // offset 4 shows sessions 04..=29 with the cursor on the final row.
        let frame = frame_lines(&rows, &shown, 29, 4, "", 100, 26);
        assert!(frame[3].contains("session-04"), "{frame:?}");
        assert!(frame[28].contains("session-29"), "{frame:?}");
        assert!(
            frame[28].contains("→"),
            "cursor row keeps the arrow: {frame:?}"
        );
        assert_eq!(frame[29].trim(), "30/30 sessions   d delete");
        // Wrap down to item 0: window snapped to the top, cursor on row 3.
        let frame = frame_lines(&rows, &shown, 0, 0, "", 100, 26);
        assert!(frame[3].contains("session-00"), "{frame:?}");
        assert!(frame[3].contains("→"), "{frame:?}");
        assert!(frame[28].contains("session-25"), "{frame:?}");
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
        // The session's last (and only) row is the user turn the agent never
        // answered, which the reference classifies as `interrupted`.
        assert_eq!(rows[0].status, "intr");
        assert_eq!(rows[0].msgs, 1);
        assert_eq!(rows[0].sid.len(), 8);
    }

    #[test]
    fn a_failed_status_query_renders_a_dash_instead_of_failing_the_picker() {
        // Contract 7: `None` is a query that failed, which is not the same as
        // a session missing from the map — that one has no message row at all
        // and renders `empty`.
        assert_eq!(status_tag(None, "any-session"), "-");

        let mut map = HashMap::new();
        assert_eq!(status_tag(Some(&map), "absent"), "empty");
        map.insert("known".to_owned(), SessionStatus::Interrupted);
        assert_eq!(status_tag(Some(&map), "known"), "intr");
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
