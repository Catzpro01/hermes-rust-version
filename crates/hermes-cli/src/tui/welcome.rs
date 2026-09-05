//! Spec 013 — Hermes Python UI parity, Ticket 03: Banner, Prompt, & Strings.
//!
//! Everything user-visible branding here is lifted **verbatim** from
//! `docs/HERMES_UI_SPEC.md` §9 (itself excavated from the real Python Hermes
//! default skin) with a single, deliberate adaptation: the product name in the
//! welcome line is `Hermes-RS` (this is the Rust port), while every symbol,
//! separator, label and the `(^_^)?` kawaii tone are byte-for-byte the Python
//! originals.
//!
//! Layout approach: the banner is composed into a [`Buffer`] with ratatui
//! widgets (border + title + two-column grid) and flushed to stdout as ANSI
//! by [`write_buffer_ansi`]. Rendering to a buffer keeps the layout pure and
//! unit-testable without a TTY; the ANSI writer honors the Ticket 02 color
//! depth (truecolor → 256 approximation). Piped (non-TTY) output never gets
//! the banner at all — that gating is the caller's job (see `repl.rs`) so
//! E2E invocations stay byte-stable and ANSI-free.

use std::io::{self, Write};

use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier};
use ratatui::text::{Line, Span};
use ratatui::widgets::block::{Padding, Title};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

use super::art::{
    caduceus_lines, logo_lines, CADUCEUS_LINES, CADUCEUS_WIDTH, LOGO_LINES, LOGO_WIDTH,
};
use super::theme::{detect_color_depth, truecolor_to_256, ColorDepth, HermesTheme};

// ---------------------------------------------------------------------------
// Branding strings (spec §9 — verbatim)
// ---------------------------------------------------------------------------

/// `prompt_symbol` — the composer prompt. Per spec §9 the renderer adds the
/// trailing space, so the full prompt is `❯ `.
pub const PROMPT_SYMBOL: &str = "❯ ";

/// `response_label` — the response box label, spaces on both sides
/// (spec §5.1/§9: ` ⚕ Hermes `).
pub const RESPONSE_LABEL: &str = " ⚕ Hermes ";

/// `tool_prefix` — tool-line prefix (spec §2.3: `┊`).
pub const TOOL_PREFIX: &str = "┊";

/// Separator — `─` × 40 (spec: the misc-output separator).
pub const SEPARATOR: &str = "────────────────────────────────────────";

/// `welcome` — the startup welcome line. Hermes-RS adaptation of the Python
/// default-skin welcome (product name substituted only):
/// `"Welcome to Hermes Agent! Type your message or /help for commands."`
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "verbatim brand string (spec 09); consumed by a later Spec 013 ticket (TUI welcome display)"
    )
)]
pub const WELCOME: &str = "Welcome to Hermes-RS! Type your message or /help for commands.";
/// First welcome line (banner right column, bold).
pub const WELCOME_TITLE: &str = "Welcome to Hermes-RS!";
/// Second welcome line (banner right column, dim).
pub const WELCOME_HINT: &str = "Type your message or /help for commands.";
/// Welcome panel title (ticket: gold `#FFD700`).
pub const BANNER_TITLE: &str = "Welcome to Hermes-RS!";
/// `goodbye` — clean-exit line (spec §9).
pub const GOODBYE: &str = "Goodbye! ⚕";
/// `help_header` — `/help` header (spec §9).
pub const HELP_HEADER: &str = "(^_^)? Available Commands";

// ---------------------------------------------------------------------------
// Banner layout (Buffer-based, pure)
// ---------------------------------------------------------------------------

/// The figlet logo only shows at or above this terminal width (Python rule
/// `terminal_width >= 95`, spec §3).
pub const LOGO_MIN_WIDTH: u16 = 95;
/// Panel width clamps (keeps the banner sane on tiny/huge terminals).
pub const BANNER_MIN_WIDTH: u16 = 60;
pub const BANNER_MAX_WIDTH: u16 = 120;
/// Welcome panel height: 15 caduceus rows + 2 border rows + 2 padding rows.
const BANNER_PANEL_HEIGHT: u16 = CADUCEUS_LINES as u16 + 4;

/// Renders the welcome banner into `area` of `buf` (spec §3 structure): a
/// bordered panel in `banner_border` (`#CD7F32`) titled
/// [`BANNER_TITLE`] in `banner_title` (`#FFD700`, bold), with a two-column
/// grid — left: the braille `HERMES_CADUCEUS` hero (centered), right: the
/// welcome copy (bold line + dim hint).
pub fn render_banner(area: Rect, buf: &mut Buffer, theme: &HermesTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.banner_border())
        .title(Title::from(Span::styled(
            BANNER_TITLE,
            theme.banner_title(),
        )))
        .padding(Padding::new(2, 2, 1, 1));
    //  is pure geometry — compute it before  moves the block.
    let inner = block.inner(area);
    block.render(area, buf);
    // ratatui 0.29: `split` returns `Rects` (`Rc<[Rect]>`) — index it.
    let rects = Layout::horizontal([
        Constraint::Length(CADUCEUS_WIDTH as u16),
        Constraint::Min(10),
    ])
    .split(inner);
    let left = rects[0];
    let right = rects[1];
    let caduceus: Vec<Line> = caduceus_lines()
        .into_iter()
        .map(|l| Line::from(Span::styled(l.text, l.style)))
        .collect();
    Paragraph::new(caduceus)
        .alignment(Alignment::Center)
        .render(left, buf);
    let copy: Vec<Line> = vec![
        Line::from(Span::styled(
            WELCOME_TITLE,
            theme.banner_text().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(WELCOME_HINT, theme.banner_dim())),
    ];
    // Wrap (never clip): at the 60-col minimum the right column is only
    // ~24 cells wide and the 41-char hint must break to a second line.
    Paragraph::new(copy)
        .wrap(Wrap { trim: false })
        .render(right, buf);
}

/// Renders the 6-line `HERMES_AGENT_LOGO` top-aligned in `area`, left-aligned
/// exactly as Python prints it (the source rows are ragged — verbatim).
pub fn render_logo(area: Rect, buf: &mut Buffer) {
    for (i, line) in logo_lines().iter().enumerate() {
        let y = area.y.saturating_add(i as u16);
        buf.set_stringn(area.x, y, line.text, LOGO_WIDTH, line.style);
    }
}

/// Prints the full startup banner (blank line, optional logo, blank line,
/// panel) to `w` using the detected terminal color depth. TTY gating is the
/// caller's responsibility.
pub fn print_banner(w: &mut impl Write, width: u16) -> io::Result<()> {
    let theme = HermesTheme::dark_canonical();
    write_banner(w, &theme, width, detect_color_depth())
}

/// Testable core of [`print_banner`] (explicit theme + depth, no env reads).
pub fn write_banner(
    w: &mut impl Write,
    theme: &HermesTheme,
    width: u16,
    depth: ColorDepth,
) -> io::Result<()> {
    let panel_w = width.clamp(BANNER_MIN_WIDTH, BANNER_MAX_WIDTH);
    let show_logo = width >= LOGO_MIN_WIDTH;
    let top = if show_logo {
        1 + LOGO_LINES as u16 + 1
    } else {
        0
    };
    let total_h = top + BANNER_PANEL_HEIGHT;
    let mut buf = Buffer::empty(Rect::new(0, 0, panel_w, total_h));
    let mut y = 0u16;
    if show_logo {
        y += 1; // blank line above the logo (Python: `console.print()`)
        render_logo(Rect::new(0, y, panel_w, LOGO_LINES as u16), &mut buf);
        y += LOGO_LINES as u16 + 1; // logo rows + blank line below
    }
    render_banner(
        Rect::new(0, y, panel_w, BANNER_PANEL_HEIGHT),
        &mut buf,
        theme,
    );
    write_buffer_ansi(w, &buf, depth)
}

/// Flushes a rendered [`Buffer`] to `w` as ANSI text. Rows are trimmed at
/// their last non-blank cell, trailing blank rows are dropped, and the SGR
/// state is tracked cell-to-cell to keep the byte stream compact. `depth`
/// selects truecolor codes or the Ticket 02 256-color approximation.
pub fn write_buffer_ansi(w: &mut impl Write, buf: &Buffer, depth: ColorDepth) -> io::Result<()> {
    let width = buf.area.width as usize;
    let rows: Vec<&[Cell]> = buf.content.chunks_exact(width).collect();
    let last_row = rows
        .iter()
        .rposition(|row| row.iter().any(|c| c.symbol() != " "))
        .unwrap_or(0);
    let mut first = true;
    for row in rows.iter().take(last_row + 1) {
        if !first {
            w.write_all(b"\r\n")?;
        }
        first = false;
        let last_cell = row.iter().rposition(|c| c.symbol() != " ").unwrap_or(0);
        let mut current = String::new();
        for cell in row.iter().take(last_cell + 1) {
            let sgr = sgr_for(cell, depth);
            if sgr.is_empty() {
                if !current.is_empty() {
                    w.write_all(b"\x1b[0m")?;
                    current.clear();
                }
            } else if sgr != current {
                w.write_all(sgr.as_bytes())?;
                current = sgr;
            }
            w.write_all(cell.symbol().as_bytes())?;
        }
        if !current.is_empty() {
            w.write_all(b"\x1b[0m")?;
        }
    }
    Ok(())
}

/// The SGR prefix for one cell at the given depth (`""` when the cell is
/// unstyled).
fn sgr_for(cell: &Cell, depth: ColorDepth) -> String {
    let mut parts: Vec<String> = Vec::new();
    let m = cell.modifier;
    if m.contains(Modifier::BOLD) {
        parts.push("1".to_owned());
    }
    if m.contains(Modifier::DIM) {
        parts.push("2".to_owned());
    }
    if m.contains(Modifier::ITALIC) {
        parts.push("3".to_owned());
    }
    if m.contains(Modifier::UNDERLINED) {
        parts.push("4".to_owned());
    }
    if cell.fg != Color::Reset {
        parts.push(color_code(cell.fg, depth));
    }
    if cell.bg != Color::Reset {
        parts.push(format!(
            "4{}",
            color_code(cell.bg, depth).replacen("38", "48", 1)
        ));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("\x1b[{}m", parts.join(";"))
    }
}

/// Cell width of [`RESPONSE_LABEL`] — ` ⚕ Hermes ` = space + ⚕ + space +
/// 6 letters + space = 10 narrow cells. Pinned against the constant by a
/// unit test; used by the Ticket 04 streaming box math.
pub(crate) const RESPONSE_LABEL_WIDTH: usize = 10;

/// SGR for `response_border` gold bold (`#FFD700`) — response-frame header
/// and footer (Ticket 03 frame + Ticket 04 streaming box).
pub fn sgr_bold_gold(depth: ColorDepth) -> String {
    format!("\x1b[1;{}m", color_code(Color::Rgb(255, 215, 0), depth))
}

/// SGR for `_DIM` (dim + italic) — reasoning box and tool lines (spec §5.2/§6).
pub fn sgr_dim_italic(_depth: ColorDepth) -> String {
    "\x1b[2;3m".to_owned()
}

/// SGR for `banner_text` (`#FFF8DC`) — streamed response body (spec §5.1).
pub fn sgr_banner_text(depth: ColorDepth) -> String {
    format!("\x1b[{}m", color_code(Color::Rgb(255, 248, 220), depth))
}

/// SGR reset.
pub const SGR_RESET: &str = "\x1b[0m";

/// SGR color code for an fg/bg color at the given depth (truecolor, or the
/// Ticket 02 256 approximation otherwise).
fn color_code(color: Color, depth: ColorDepth) -> String {
    match color {
        Color::Rgb(r, g, b) if depth == ColorDepth::Truecolor => format!("38;2;{r};{g};{b}"),
        _ => {
            let approx = truecolor_to_256(color);
            match approx {
                Color::Indexed(i) => format!("38;5;{i}"),
                _ => String::new(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brand_strings_are_verbatim() {
        assert_eq!(PROMPT_SYMBOL, "❯ ");
        assert_eq!(RESPONSE_LABEL, " ⚕ Hermes ");
        assert_eq!(TOOL_PREFIX, "┊");
        assert_eq!(GOODBYE, "Goodbye! ⚕");
        assert_eq!(HELP_HEADER, "(^_^)? Available Commands");
        assert_eq!(
            WELCOME,
            "Welcome to Hermes-RS! Type your message or /help for commands."
        );
        assert_eq!(BANNER_TITLE, "Welcome to Hermes-RS!");
        assert_eq!(WELCOME, format!("{WELCOME_TITLE} {WELCOME_HINT}"));
        assert_eq!(
            RESPONSE_LABEL.chars().count(),
            RESPONSE_LABEL_WIDTH,
            "label width const must track the label"
        );
    }

    #[test]
    fn separator_is_exactly_forty_box_dashes() {
        assert_eq!(SEPARATOR, "─".repeat(40));
        assert_eq!(SEPARATOR.chars().count(), 40);
        assert!(SEPARATOR.chars().all(|c| c == '─'));
    }

    #[test]
    fn logo_is_six_lines_with_three_color_tiers() {
        let rows = logo_lines();
        assert_eq!(rows.len(), 6);
        let gold = Color::Rgb(255, 215, 0);
        let accent = Color::Rgb(255, 191, 0);
        let bronze = Color::Rgb(205, 127, 50);
        let expected: [(Color, bool); 6] = [
            (gold, true),
            (gold, true),
            (accent, false),
            (accent, false),
            (bronze, false),
            (bronze, false),
        ];
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.style.fg, Some(expected[i].0), "logo row {i} fg");
            assert_eq!(
                row.style.add_modifier.contains(Modifier::BOLD),
                expected[i].1,
                "logo row {i} bold"
            );
            assert!(
                !row.text.contains('[') && !row.text.contains(']'),
                "no rich markup may leak into row {i}"
            );
        }
    }

    #[test]
    fn banner_renders_border_title_and_grid() {
        let theme = HermesTheme::dark_canonical();
        let area = Rect::new(0, 0, 80, BANNER_PANEL_HEIGHT);
        let mut buf = Buffer::empty(area);
        render_banner(area, &mut buf, &theme);
        let bronze = Color::Rgb(205, 127, 50);
        let gold = Color::Rgb(255, 215, 0);
        // corners carry the border color
        let tl = buf.cell((0, 0)).unwrap();
        assert_eq!(tl.symbol(), "┌");
        assert_eq!(tl.fg, bronze);
        let tr = buf.cell((79, 0)).unwrap();
        assert_eq!(tr.symbol(), "┐");
        assert_eq!(tr.fg, bronze);
        assert_eq!(buf.cell((0, 18)).unwrap().symbol(), "└");
        assert_eq!(buf.cell((79, 18)).unwrap().symbol(), "┘");
        // gold bold title on the top border row
        let title: String = (1..79)
            .map(|x| buf.cell((x, 0)).unwrap().symbol().to_owned())
            .collect();
        assert!(title.contains(BANNER_TITLE), "title missing: {title:?}");
        let x = (1..79)
            .find(|x| buf.cell((*x, 0)).unwrap().symbol() == "W")
            .unwrap();
        let t = buf.cell((x, 0)).unwrap();
        assert_eq!(t.fg, gold);
        assert!(t.modifier.contains(Modifier::BOLD));
        // grid: caduceus braille present, welcome copy present
        let whole: String = buf.content.iter().map(|c| c.symbol().to_owned()).collect();
        assert!(whole.contains('⣿'), "caduceus braille missing");
        assert!(whole.contains(WELCOME_TITLE));
        // the dim hint line sits in the right column (x >= caduceus width)
        let hint_y = (0..area.height)
            .find(|y| {
                let row: String = (0..area.width)
                    .map(|x| buf.cell((x, *y)).unwrap().symbol().to_owned())
                    .collect();
                row.contains(WELCOME_HINT)
            })
            .expect("welcome hint row");
        let row: String = (0..area.width)
            .map(|x| buf.cell((x, hint_y)).unwrap().symbol().to_owned())
            .collect();
        let pos = row.find('T').expect("hint starts with 'T'");
        assert!(
            pos >= CADUCEUS_WIDTH,
            "hint must sit in the right column (got col {pos})"
        );
    }

    #[test]
    fn banner_narrow_width_keeps_full_welcome_copy() {
        // Minimum panel width: the right column is only ~24 cells wide, so
        // the dim hint (41 chars) must wrap onto a second line, never clip.
        let theme = HermesTheme::dark_canonical();
        let area = Rect::new(0, 0, BANNER_MIN_WIDTH, BANNER_PANEL_HEIGHT);
        let mut buf = Buffer::empty(area);
        render_banner(area, &mut buf, &theme);
        let whole: String = buf.content.iter().map(|c| c.symbol().to_owned()).collect();
        assert!(whole.contains("Type your message or"), "hint head clipped");
        assert!(
            whole.contains("/help for commands."),
            "hint tail must survive the 60-col wrap: {whole:?}"
        );
    }

    #[test]
    fn banner_ansi_truecolor_and_256_paths() {
        let theme = HermesTheme::dark_canonical();
        let mut out = Vec::new();
        write_banner(&mut out, &theme, 100, ColorDepth::Truecolor).unwrap();
        let s = String::from_utf8_lossy(&out);
        assert!(s.contains("38;2;255;215;0"), "gold truecolor (logo/title)");
        assert!(s.contains("38;2;205;127;50"), "bronze truecolor (border)");
        assert!(
            s.contains("38;2;255;191;0"),
            "accent truecolor (logo tier 2)"
        );
        assert!(
            s.contains("38;2;184;134;11"),
            "dim truecolor (caduceus base)"
        );
        assert!(s.contains("┌") && s.contains("┘"), "panel border present");
        assert!(s.contains("██╗"), "logo art present");
        assert!(!s.contains('\0'));
        // 256 path: indexed codes only, no truecolor
        let mut out2 = Vec::new();
        write_banner(&mut out2, &theme, 100, ColorDepth::Color256).unwrap();
        let s2 = String::from_utf8_lossy(&out2);
        assert!(
            !s2.contains("38;2;"),
            "256 path must not emit truecolor codes"
        );
        assert!(s2.contains("38;5;"), "256 path uses indexed codes");
        // below 95 cols the logo is suppressed (Python rule), panel remains
        let mut out3 = Vec::new();
        write_banner(&mut out3, &theme, 80, ColorDepth::Truecolor).unwrap();
        let s3 = String::from_utf8_lossy(&out3);
        assert!(!s3.contains("██╗"), "logo hidden below 95 cols");
        assert!(s3.contains("┌"), "panel still present");
    }
}
