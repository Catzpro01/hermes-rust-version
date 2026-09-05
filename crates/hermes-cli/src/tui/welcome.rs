//! Welcome banner + brand strings (Spec 013 T03; Spec 017 T02 — v0.21.0 parity).
//!
//! The banner layout is a port of Python `hermes_cli/banner.py` v0.21.0
//! `build_welcome_banner` (verbatim checkout 63279301): leading blank line,
//! the 6-line `HERMES_AGENT_LOGO` (raw terminal width >= 95, word-wrapped at
//! the raw width exactly like `console.print`), a blank line, then a Rich
//! `Panel` (single-line `#CD7F32` border, panel padding (0,2)) with the
//! centered bold-gold `#FFD700` version title and a two-column
//! `Table.grid(padding=(0,2))`:
//!
//! - left: blank, the 15-line braille `HERMES_CADUCEUS`, blank, the model
//!   line (accent name + dim `· {ctx} context · Nous Research`, or the red
//!   `no model configured` state), the dim cwd, and the dim session line;
//! - right: `Available Tools` (+ toolset rows, Python's 45/42 truncation
//!   rule), the `MCP Servers` section (configured servers — the banner is
//!   printed before MCP connects), `Available Skills` (the Rust port has no
//!   skills system yet -> always the `No skills installed` empty state), and
//!   the dim summary `{n} tools · {m} skills · /help for commands`.
//!
//! Column geometry replicates Rich's table width algorithm
//! (`_collapse_widths` -> `ratio_reduce` -> re-measure) so the buffer is
//! byte-identical to the Python reference (pinned by the
//! `banner_*_matches_python_reference` unit tests, plain text, widths
//! 60/70/80/94/95/100).
//!
//! The Rust port tracks the pinned v0.21.0 upstream release, so the version
//! label is the constant [`VERSION_LABEL`] (Python derives it from git
//! state at install time).

use std::io::{self, Write};

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
};

use super::art::{
    caduceus_lines, logo_lines,
    CADUCEUS_LINES, CADUCEUS_WIDTH, LOGO_LINES, LOGO_WIDTH,
};
use super::theme::{detect_color_depth, truecolor_to_256, ColorDepth, HermesTheme};

// ---------------------------------------------------------------------------
// Branding constants (verbatim from Phase 0 extraction; see
// docs/HERMES_UI_SPEC.md v1.0.0 §0 + §017 phase-0 findings).
// ---------------------------------------------------------------------------

/// Input prompt symbol (gold `#FFD700` at render time) — `prompt.py` `PROMPT`.
pub const PROMPT_SYMBOL: &str = "❯ ";

/// Response-box label between the border corners (` ⚕ Hermes `) —
/// `response_box.py`. Rendered gold bold at runtime (spec §7).
pub const RESPONSE_LABEL: &str = " ⚕ Hermes ";

/// Tool-line prefix (`┊`, dim brown at render time) — `tool_line.py`.
pub const TOOL_PREFIX: &str = "┊";

/// Separator — `─` × 40 (spec: the misc-output separator; the /help
/// header divider, pinned by the branding E2E).
pub const SEPARATOR: &str = "────────────────────────────────────────";

/// Goodbye line on clean exit — `main.py` `GOODBYE`.
pub const GOODBYE: &str = "Goodbye! ⚕";

/// `/help` header line — `main.py` `HELP_HEADER`.
pub const HELP_HEADER: &str = "(^_^)? Available Commands";

/// Panel title / version label (bold `#FFD700`). The Rust port pins the
/// upstream v0.21.0 release, so the label is a constant.
pub const VERSION_LABEL: &str = "Hermes-RS v0.21.0 (2026.8.31) · upstream 63279301";

/// Banner data assembled by the REPL before printing (Spec 017 T02).
#[derive(Debug, Clone, Default)]
pub struct BannerInfo {
    /// Active model name; `None`, empty, or `"unknown"` (case-insensitive)
    /// renders the red `no model configured — run /model or hermes setup` line.
    pub model: Option<String>,
    /// Context window in tokens -> the `· {N} context` segment
    /// ([`format_context_length`]).
    pub context_tokens: Option<u64>,
    /// Current working directory line (dim).
    pub cwd: String,
    /// Session id -> the `Session: {id}` line (dim grey `#8B8682`).
    pub session_id: Option<String>,
    /// Registered tool names. Until T07 ports the toolset mapping every tool
    /// lands in the `other` toolset row (Python `_display_toolset_name`
    /// behavior).
    pub tools: Vec<String>,
    /// Configured MCP server names. The banner is printed before MCP
    /// connects, so each renders as `{name} (stdio) — configured`.
    pub mcp_servers: Vec<String>,
}

const BANNER_MIN_WIDTH: u16 = 60;
const BANNER_MAX_WIDTH: u16 = 120;
/// The 6-line logo is printed only at raw terminal width >= 95
/// (`len(HERMES_AGENT_LOGO.splitlines()) < 6` guard inverted in
/// `build_welcome_banner`).
const LOGO_MIN_WIDTH: u16 = 95;
/// Session line color (`#8B8682`) — `build_welcome_banner` `session_color`.
const SESSION_COLOR: Color = Color::Rgb(139, 134, 130);

// ---------------------------------------------------------------------------
// Rich algorithm ports (byte-parity, pinned by unit tests).
// ---------------------------------------------------------------------------

/// Port of `_format_context_length` (banner.py): at a million or more,
/// `{N}M`/`{N.N}M`; at a thousand or more, `{N}K`/`{N.N}K`; below that the
/// raw token count. The rounded form is used when the value is within 0.05
/// of an integer.
pub fn format_context_length(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        let val = tokens as f64 / 1_000_000.0;
        let rounded = val.round();
        if (val - rounded).abs() < 0.05 {
            format!("{rounded:.0}M")
        } else {
            format!("{val:.1}M")
        }
    } else if tokens >= 1_000 {
        let val = tokens as f64 / 1_000.0;
        let rounded = val.round();
        if (val - rounded).abs() < 0.05 {
            format!("{rounded:.0}K")
        } else {
            format!("{val:.1}K")
        }
    } else {
        tokens.to_string()
    }
}

/// Word tokens of `text` as `(byte_start, word_including_trailing_ws)` — port
/// of `rich._wrap.words` with `re_word = r"\s*\S+\s*"`. Banner content is
/// ASCII-spaced (braille blank U+2800 is NOT `\s` in Python — verified), so
/// ASCII whitespace is exact.
fn word_tokens(text: &str) -> Vec<(usize, &str)> {
    let bytes = text.as_bytes();
    let mut out: Vec<(usize, &str)> = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let word_start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        out.push((word_start, &text[word_start..i]));
    }
    out
}

/// Port of `rich._wrap.divide_line(text, width, fold=False)` (table cells
/// render with overflow `crop`): byte offsets at which `text` splits to fit
/// `width` cells. Words longer than a full line move to their own line and
/// are cropped downstream with `…` (the console line adjust).
fn divide_line(text: &str, width: usize) -> Vec<usize> {
    let mut breaks: Vec<usize> = Vec::new();
    let mut cell_offset = 0usize;
    for (start, word) in word_tokens(text) {
        let word_len = word.trim_end().chars().count();
        let remaining = width.saturating_sub(cell_offset);
        if remaining >= word_len {
            cell_offset += word.chars().count();
        } else if word_len > width {
            if start > 0 {
                breaks.push(start);
            }
            cell_offset = word.chars().count();
        } else if cell_offset > 0 && start > 0 {
            breaks.push(start);
            cell_offset = word.chars().count();
        }
    }
    breaks
}

/// Wraps `line` to `width` cells (crop mode) and crops unbreakable overflow
/// to `width-1` cells + `…` (Rich's `Segment.adjust_line_length`).
fn wrap_line(line: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![];
    }
    let breaks = divide_line(line, width);
    let mut out: Vec<String> = Vec::new();
    let mut prev = 0usize;
    for b in &breaks {
        out.push(line[prev..*b].to_owned());
        prev = *b;
    }
    out.push(line[prev..].to_owned());
    out.into_iter()
        .map(|l| {
            if l.chars().count() > width {
                let cut = byte_at_char(&l, width.saturating_sub(1));
                format!("{}…", &l[..cut])
            } else {
                l
            }
        })
        .collect()
}

/// Byte offset of the `n`-th char boundary (or `len` when `n` is beyond the
/// string).
fn byte_at_char(s: &str, n: usize) -> usize {
    s.char_indices()
        .nth(n)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

/// Python's `round()` — banker's rounding (half to even), applied to the
/// f64 produced by `ratio * remaining / total_ratio`.
fn py_round(x: f64) -> i64 {
    let fl = x.floor();
    let frac = x - fl;
    let result = if frac > 0.5 {
        fl + 1.0
    } else if frac < 0.5 || (fl as i64) % 2 == 0 {
        fl
    } else {
        fl + 1.0
    };
    result as i64
}

/// Port of Rich's `Table._collapse_widths` + `ratio_reduce` — shrinks the
/// column widths so `l + r <= max_width`. `l` is the left column including
/// its 2-cell inter-column padding (Rich's per-cell right padding; the
/// `collapse_padding` grid removes the other side), `r` the right column.
/// Returns `(left_content_width, right_content_width)`.
fn shrink_columns(l: usize, r: usize, max_width: usize) -> (usize, usize) {
    let mut widths = [l, r];
    let mut total: usize = l + r;
    let mut excess = total.saturating_sub(max_width);
    let mut guard = 0u32;
    while excess > 0 && guard < 100 {
        guard += 1;
        let max_w = widths[0].max(widths[1]);
        let second = widths
            .iter()
            .copied()
            .filter(|w| *w != max_w)
            .max()
            .unwrap_or(0);
        let diff = max_w - second;
        let ratios: [u32; 2] = [
            u32::from(widths[0] == max_w),
            u32::from(widths[1] == max_w),
        ];
        if ratios[0] + ratios[1] == 0 || diff == 0 {
            break;
        }
        let max_reduce = excess.min(diff);
        let mut remaining = excess as i64;
        let mut total_ratio = i64::from(ratios[0] + ratios[1]);
        for i in 0..2 {
            if ratios[i] > 0 && total_ratio > 0 {
                let distributed = (max_reduce as i64).min(py_round(
                    f64::from(ratios[i]) * (remaining as f64) / (total_ratio as f64),
                ));
                widths[i] = widths[i].saturating_sub(distributed as usize);
                remaining -= distributed;
                total_ratio -= i64::from(ratios[i]);
            }
        }
        total = widths[0] + widths[1];
        excess = total.saturating_sub(max_width);
    }
    (widths[0].saturating_sub(2), widths[1])
}

// ---------------------------------------------------------------------------
// Banner content (styled lines).
// ---------------------------------------------------------------------------

/// A run of text with one style.
#[derive(Debug, Clone)]
struct Run {
    text: String,
    style: Style,
}

/// A styled line (sequence of runs).
#[derive(Debug, Clone)]
struct SLine {
    runs: Vec<Run>,
}

impl SLine {
    fn new(parts: Vec<(&str, Style)>) -> Self {
        Self {
            runs: parts
                .into_iter()
                .map(|(text, style)| Run {
                    text: text.to_owned(),
                    style,
                })
                .collect(),
        }
    }

    fn blank() -> Self {
        Self { runs: Vec::new() }
    }

    fn is_blank(&self) -> bool {
        self.runs.iter().all(|r| r.text.is_empty())
    }

    fn plain(&self) -> String {
        self.runs.iter().map(|r| r.text.as_str()).collect()
    }

    fn width(&self) -> usize {
        self.runs.iter().map(|r| r.text.chars().count()).sum()
    }

    /// Slices this line to plain-text byte range `[start, end)` (re-mapping
    /// the styles) — used after wrapping.
    fn slice(&self, start: usize, end: usize) -> SLine {
        let mut out = SLine::blank();
        let mut pos = 0usize;
        for run in &self.runs {
            let run_end = pos + run.text.len();
            if run_end <= start || pos >= end {
                pos = run_end;
                continue;
            }
            let lo = start.saturating_sub(pos);
            let hi = (end - pos).min(run.text.len());
            out.runs.push(Run {
                text: run.text[lo..hi].to_owned(),
                style: run.style,
            });
            pos = run_end;
        }
        out
    }
}

/// Wraps a styled line to `width` cells (crop mode) — spans are re-mapped to
/// the wrapped pieces; unbreakable overflow is cropped with a plain `…`.
fn wrap_styled(line: &SLine, width: usize) -> Vec<SLine> {
    if width == 0 {
        return vec![SLine::blank()];
    }
    let plain = line.plain();
    let breaks = divide_line(&plain, width);
    let mut bounds: Vec<(usize, usize)> = Vec::new();
    let mut prev = 0usize;
    for b in &breaks {
        bounds.push((prev, *b));
        prev = *b;
    }
    bounds.push((prev, plain.len()));
    bounds
        .into_iter()
        .map(|(s, e)| {
            let mut sl = line.slice(s, e);
            if sl.width() > width {
                let cut = byte_at_char(&sl.plain(), width.saturating_sub(1));
                sl = sl.slice(0, cut);
                sl.runs.push(Run {
                    text: "…".to_owned(),
                    style: Style::default(),
                });
            }
            sl
        })
        .collect()
}

/// Left column lines (verbatim `build_welcome_banner` `left_lines`).
fn left_lines(info: &BannerInfo, theme: &HermesTheme) -> Vec<SLine> {
    let dim = theme.banner_dim();
    let mut left = vec![SLine::blank()];
    for row in caduceus_lines() {
        left.push(SLine::new(vec![(row.text, row.style)]));
    }
    left.push(SLine::blank());
    let model = info.model.as_deref().unwrap_or("");
    let model_t = model.trim();
    if model_t.is_empty() || model_t.eq_ignore_ascii_case("unknown") {
        left.push(SLine::new(vec![
            (
                "no model configured",
                Style::default()
                    .fg(Color::Rgb(255, 0, 0))
                    .add_modifier(Modifier::BOLD),
            ),
            (" — run /model or hermes setup", dim),
        ]));
    } else {
        let mut model_short = model_t
            .rsplit('/')
            .next()
            .map(str::to_owned)
            .unwrap_or_else(|| model_t.to_owned());
        if model_short.ends_with(".gguf") {
            model_short.truncate(model_short.len() - 5);
        }
        if model_short.chars().count() > 28 {
            model_short = format!(
                "{}...",
                model_short.chars().take(25).collect::<String>()
            );
        }
        let mut parts: Vec<(String, Style)> = vec![(
            model_short,
            Style::default().fg(theme.banner_accent),
        )];
        if let Some(ctx) = info.context_tokens {
            parts.push((" · ".to_owned(), dim));
            parts.push((
                format!("{} context", format_context_length(ctx)),
                dim,
            ));
        }
        parts.push((" · ".to_owned(), dim));
        parts.push(("Nous Research".to_owned(), dim));
        left.push(SLine {
            runs: parts
                .into_iter()
                .map(|(text, style)| Run { text, style })
                .collect(),
        });
    }
    left.push(SLine {
        runs: vec![Run {
            text: info.cwd.clone(),
            style: dim,
        }],
    });
    if let Some(sid) = &info.session_id {
        left.push(SLine {
            runs: vec![Run {
                text: format!("Session: {sid}"),
                style: Style::default().fg(SESSION_COLOR),
            }],
        });
    }
    left
}

/// Right column lines (verbatim `build_welcome_banner` `right_lines`; the
/// Rust port has no skills system, so the skills section is always the empty
/// state).
fn right_lines(info: &BannerInfo, theme: &HermesTheme) -> Vec<SLine> {
    let dim = theme.banner_dim();
    let body = theme.banner_text();
    let header = |text: &str| SLine::new(vec![(text, theme.banner_section())]);
    let mut right = vec![header("Available Tools")];
    if !info.tools.is_empty() {
        let mut names: Vec<&str> = info.tools.iter().map(String::as_str).collect();
        names.sort_unstable();
        // Python truncation rule: joined > 45 cells -> accumulate names while
        // `length + len(name) + 2 <= 42`, then append "..." and stop.
        let joined = names.join(", ");
        let tools_str = if joined.chars().count() > 45 {
            let mut short: Vec<&str> = Vec::new();
            let mut length = 0usize;
            for name in &names {
                if length + name.chars().count() + 2 > 42 {
                    short.push("...");
                    break;
                }
                short.push(name);
                length += name.chars().count() + 2;
            }
            short.join(", ")
        } else {
            joined
        };
        right.push(SLine {
            runs: vec![
                Run {
                    text: "other:".to_owned(),
                    style: dim,
                },
                Run {
                    text: format!(" {tools_str}"),
                    style: body,
                },
            ],
        });
    }
    if !info.mcp_servers.is_empty() {
        right.push(SLine::blank());
        right.push(header("MCP Servers"));
        let mut names: Vec<&str> = info
            .mcp_servers
            .iter()
            .map(String::as_str)
            .collect();
        names.sort_unstable();
        for name in names {
            right.push(SLine {
                runs: vec![
                    Run {
                        text: name.to_owned(),
                        style: dim,
                    },
                    Run {
                        text: " (stdio)".to_owned(),
                        style: dim,
                    },
                    Run {
                        text: " — configured".to_owned(),
                        style: dim,
                    },
                ],
            });
        }
    }
    right.push(SLine::blank());
    right.push(header("Available Skills"));
    right.push(SLine::new(vec![("No skills installed", dim)]));
    right.push(SLine::blank());
    let summary = [
        format!("{} tools", info.tools.len()),
        // The Rust port has no skills (T06/T07 territory) and MCP tools are
        // registered after the banner, so the connected count is 0 here.
        "0 skills".to_owned(),
        "/help for commands".to_owned(),
    ]
    .join(" · ");
    right.push(SLine::new(vec![(summary.as_str(), dim)]));
    right
}

/// Full layout of the panel content at panel width `width`: wrapped left/
/// right columns plus their measured (post-wrap) widths.
struct BannerLayout {
    left: Vec<SLine>,
    right: Vec<SLine>,
    left_w: usize,
}

fn layout_banner(width: usize, info: &BannerInfo, theme: &HermesTheme) -> BannerLayout {
    let left0 = left_lines(info, theme);
    let right0 = right_lines(info, theme);
    let l0 = left0.iter().map(SLine::width).max().unwrap_or(0);
    let r0 = right0.iter().map(SLine::width).max().unwrap_or(0);
    // Panel content area = panel width - 2 border - 2*2 padding (Rich Panel
    // padding (0,2)). The grid then gets L + 2 (gap) + R.
    let content = width.saturating_sub(6);
    let (l_alloc, r_alloc) = if l0 + 2 + r0 > content {
        shrink_columns(l0 + 2, r0, content)
    } else {
        (l0, r0)
    };
    let left: Vec<SLine> = left0
        .iter()
        .flat_map(|l| wrap_styled(l, l_alloc))
        .collect();
    let right: Vec<SLine> = right0
        .iter()
        .flat_map(|l| wrap_styled(l, r_alloc))
        .collect();
    let left_w = left.iter().map(SLine::width).max().unwrap_or(0);
    BannerLayout {
        left,
        right,
        left_w,
    }
}

/// The 6 logo rows word-wrapped at the RAW terminal width (Python prints the
/// logo with `console.print`, which wraps at the console width — rows 1-2
/// are 101 cells wide, so at 95-100 columns each breaks into two). Each
/// piece keeps its source row's color tier.
fn logo_wrapped(raw_width: usize) -> Vec<(String, Style)> {
    let mut out: Vec<(String, Style)> = Vec::new();
    for row in logo_lines() {
        for piece in wrap_line(row.text, raw_width) {
            out.push((piece, row.style));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Rendering.
// ---------------------------------------------------------------------------

/// One-shot structural sanity of the art constants (guards the art
/// contract the layout math relies on; startup cost is negligible).
fn art_contract() {
    assert_eq!(caduceus_lines().len(), CADUCEUS_LINES);
    assert_eq!(logo_lines().len(), LOGO_LINES);
    for row in caduceus_lines() {
        assert_eq!(row.text.chars().count(), CADUCEUS_WIDTH);
    }
    for row in logo_lines() {
        assert!(row.text.chars().count() <= LOGO_WIDTH);
    }
}

/// Composes the full banner (blank line, optional logo, blank line, panel)
/// into a `Buffer` of width `clamp(raw_width, 60, 120)`.
fn compose_banner(raw_width: u16, info: &BannerInfo, theme: &HermesTheme) -> Buffer {
    art_contract();
    let panel_w = raw_width.clamp(BANNER_MIN_WIDTH, BANNER_MAX_WIDTH) as usize;
    let show_logo = raw_width >= LOGO_MIN_WIDTH;
    let layout = layout_banner(panel_w, info, theme);
    let panel_h = layout.left.len().max(layout.right.len()) + 2;
    // Logo rows only matter when the logo is shown (raw width >= 95).
    let logo = if show_logo { logo_wrapped(raw_width as usize) } else { Vec::new() };
    let top = 1 + logo.len() + usize::from(show_logo);
    let total_h = top + panel_h;
    let mut buf = Buffer::empty(Rect::new(0, 0, panel_w as u16, total_h as u16));
    if show_logo {
        for (i, (piece, style)) in logo.iter().enumerate() {
            buf.set_stringn(0, 1 + i as u16, piece, panel_w, *style);
        }
    }
    draw_panel(
        Rect::new(0, top as u16, panel_w as u16, panel_h as u16),
        &mut buf,
        theme,
        &layout,
    );
    buf
}

/// Draws the panel (border + title + two-column grid) into `buf` at `area`.
fn draw_panel(area: Rect, buf: &mut Buffer, theme: &HermesTheme, layout: &BannerLayout) {
    let width = area.width as usize;
    let h = layout.left.len().max(layout.right.len());
    let border_style = theme.banner_border();
    let title_style = theme.banner_title();
    // Top border with the centered ` title ` block (Rich Panel title: one
    // space each side, remainder filled with `─`, left share floored).
    buf.set_string(area.x, area.y, "╭", border_style);
    buf.set_string(area.x + width as u16 - 1, area.y, "╮", border_style);
    let inner = width - 2;
    let title_len = VERSION_LABEL.chars().count();
    let block = title_len + 2;
    if inner >= block {
        let dashes_l = (inner - block) / 2;
        let dashes_r = inner - block - dashes_l;
        buf.set_stringn(
            area.x + 1,
            area.y,
            "─".repeat(dashes_l),
            inner,
            border_style,
        );
        buf.set_stringn(area.x + 1 + dashes_l as u16, area.y, " ", 1, border_style);
        buf.set_stringn(
            area.x + 2 + dashes_l as u16,
            area.y,
            VERSION_LABEL,
            title_len,
            title_style,
        );
        buf.set_stringn(
            area.x + 2 + (dashes_l + title_len) as u16,
            area.y,
            " ",
            1,
            border_style,
        );
        buf.set_stringn(
            area.x + 3 + (dashes_l + title_len) as u16,
            area.y,
            "─".repeat(dashes_r),
            dashes_r,
            border_style,
        );
    } else {
        // Title wider than the border (only possible below the 60-cell
        // minimum): `─ {title…} ─` with a single dash per side (Rich crops).
        let cut: String = VERSION_LABEL.chars().take(inner.saturating_sub(2)).collect();
        buf.set_stringn(area.x + 1, area.y, "─ ", 2, border_style);
        buf.set_stringn(
            area.x + 3,
            area.y,
            &cut,
            cut.chars().count(),
            title_style,
        );
        buf.set_stringn(
            area.x + 3 + cut.chars().count() as u16,
            area.y,
            " ─",
            2,
            border_style,
        );
    }
    // Content rows: left column centered in its measured width, right column
    // left-aligned at `left_w + 2` (the Rich grid gap), both top-aligned.
    for i in 0..h {
        let y = area.y + 1 + i as u16;
        buf.set_string(area.x, y, "│", border_style);
        buf.set_string(area.x + width as u16 - 1, y, "│", border_style);
        // Panel content origin = border (1) + panel padding (2) = x + 3.
        if let Some(l) = layout.left.get(i) {
            if !l.is_blank() {
                let offset = layout.left_w.saturating_sub(l.width()) / 2;
                draw_line_at(
                    buf,
                    area.x,
                    width,
                    area.x + 3 + offset as u16,
                    y,
                    l,
                );
            }
        }
        if let Some(r) = layout.right.get(i) {
            if !r.is_blank() {
                draw_line_at(
                    buf,
                    area.x,
                    width,
                    area.x + 3 + layout.left_w as u16 + 2,
                    y,
                    r,
                );
            }
        }
    }
    // Bottom border.
    let bottom_y = area.y + 1 + h as u16;
    buf.set_string(area.x, bottom_y, "╰", border_style);
    buf.set_stringn(
        area.x + 1,
        bottom_y,
        "─".repeat(inner),
        inner,
        border_style,
    );
    buf.set_string(area.x + width as u16 - 1, bottom_y, "╯", border_style);
}

/// Draws one styled line at `x`, clipped at the panel's right border.
fn draw_line_at(buf: &mut Buffer, area_x: u16, width: usize, x: u16, y: u16, line: &SLine) {
    let mut pos = x;
    for run in &line.runs {
        let room = width - (pos - area_x) as usize;
        if room > 0 {
            buf.set_stringn(pos, y, &run.text, room, run.style);
        }
        pos += run.text.chars().count() as u16;
    }
}

/// Prints the full startup banner (blank line, optional logo, blank line,
/// panel) to `w` using the detected terminal color depth. TTY gating is the
/// caller's responsibility. `width` is the RAW terminal width: the panel
/// clamps to 60-120 (Spec 013 convention) while the logo threshold (>= 95)
/// and logo wrapping use the raw value (Python parity).
pub fn print_banner(w: &mut impl Write, width: u16, info: &BannerInfo) -> io::Result<()> {
    let theme = HermesTheme::dark_canonical();
    write_banner(w, &theme, width, info, detect_color_depth())
}

/// Testable core of [`print_banner`] (explicit theme + depth, no env reads).
pub fn write_banner(
    w: &mut impl Write,
    theme: &HermesTheme,
    raw_width: u16,
    info: &BannerInfo,
    depth: ColorDepth,
) -> io::Result<()> {
    let buf = compose_banner(raw_width, info, theme);
    write_buffer_ansi(w, &buf, depth)
}

/// Writes a composed buffer as an SGR ANSI stream (Spec 013 T02): one reset
/// at the end, SGR changes only when a cell's style changes, and no
/// positioning escapes — the stream is written to stdout where the cursor
/// sits at column 0.
pub fn write_buffer_ansi(
    w: &mut impl Write,
    buf: &Buffer,
    depth: ColorDepth,
) -> io::Result<()> {
    let area = buf.area;
    for y in area.y..area.y + area.height {
        if y > area.y {
            w.write_all(b"\n")?;
        }
        let mut current = String::new();
        for x in area.x..area.x + area.width {
            let cell = &buf[(x, y)];
            let sgr = sgr_for(cell, depth);
            if cell.symbol() == " " && sgr.is_empty() {
                continue;
            }
            if sgr.is_empty() && !current.is_empty() {
                w.write_all(b"\x1b[0m")?;
                current.clear();
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
fn sgr_for(cell: &ratatui::buffer::Cell, depth: ColorDepth) -> String {
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

/// SGR for dim brown secondary text (`#B8860B`) — `hermes model` markers and
/// labels (Spec 014 T02).
pub fn sgr_dim_brown(depth: ColorDepth) -> String {
    format!("\x1b[{}m", color_code(Color::Rgb(184, 134, 11), depth))
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
    use super::super::art::{CADUCEUS_LINES, LOGO_LINES, LOGO_WIDTH};
    // AUTO-GENERATED reference constants (Spec 017 T02).
    // Source: Python v0.21.0 `build_welcome_banner` (checkout 63279301) with
    // the Rust version label monkeypatched in — plain text, no ANSI, each line
    // rstripped. Widths 60/70/80/94/95/100. Regenerate with
    // /tmp/ref_final.py on the VM (hermes-agent venv).
    
    const REF_W100_PRIMARY: &str = "\n██╗  ██╗███████╗██████╗ ███╗   ███╗███████╗███████╗       █████╗  ██████╗ ███████╗███╗\n██╗████████╗\n██║  ██║██╔════╝██╔══██╗████╗ ████║██╔════╝██╔════╝      ██╔══██╗██╔════╝ ██╔════╝████╗\n██║╚══██╔══╝\n███████║█████╗  ██████╔╝██╔████╔██║█████╗  ███████╗█████╗███████║██║  ███╗█████╗  ██╔██╗ ██║   ██║\n██╔══██║██╔══╝  ██╔══██╗██║╚██╔╝██║██╔══╝  ╚════██║╚════╝██╔══██║██║   ██║██╔══╝  ██║╚██╗██║   ██║\n██║  ██║███████╗██║  ██║██║ ╚═╝ ██║███████╗███████║      ██║  ██║╚██████╔╝███████╗██║ ╚████║   ██║\n╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝     ╚═╝╚══════╝╚══════╝      ╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚═╝  ╚═══╝   ╚═╝\n\n╭─────────────────────── Hermes-RS v0.21.0 (2026.8.31) · upstream 63279301 ────────────────────────╮\n│                                                    Available Tools                               │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⡀⠀⣀⣀⠀⢀⣀⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀           other: file_read, file_write, web_search      │\n│           ⠀⠀⠀⠀⠀⠀⢀⣠⣴⣾⣿⣿⣇⠸⣿⣿⠇⣸⣿⣿⣷⣦⣄⡀⠀⠀⠀⠀⠀⠀                                                         │\n│           ⠀⢀⣠⣴⣶⠿⠋⣩⡿⣿⡿⠻⣿⡇⢠⡄⢸⣿⠟⢿⣿⢿⣍⠙⠿⣶⣦⣄⡀⠀           Available Skills                              │\n│           ⠀⠀⠉⠉⠁⠶⠟⠋⠀⠉⠀⢀⣈⣁⡈⢁⣈⣁⡀⠀⠉⠀⠙⠻⠶⠈⠉⠉⠀⠀           No skills installed                           │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣴⣿⡿⠛⢁⡈⠛⢿⣿⣦⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠿⣿⣦⣤⣈⠁⢠⣴⣿⠿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀           3 tools · 0 skills · /help for commands       │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠉⠻⢿⣿⣦⡉⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠘⢷⣦⣈⠛⠃⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢠⣴⠦⠈⠙⠿⣦⡄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠸⣿⣤⡈⠁⢤⣿⠇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠛⠷⠄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⠑⢶⣄⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣿⠁⢰⡆⠈⡿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠳⠈⣡⠞⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│                                                                                                  │\n│  claude-sonnet-4-5 · 200K context · Nous Research                                                │\n│                  /home/user/demo                                                                 │\n╰──────────────────────────────────────────────────────────────────────────────────────────────────╯";
    
    const REF_W80_PRIMARY: &str = "\n╭───────────── Hermes-RS v0.21.0 (2026.8.31) · upstream 63279301 ──────────────╮\n│                                     Available Tools                          │\n│   ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⡀⠀⣀⣀⠀⢀⣀⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                             │\n│   ⠀⠀⠀⠀⠀⠀⢀⣠⣴⣾⣿⣿⣇⠸⣿⣿⠇⣸⣿⣿⣷⣦⣄⡀⠀⠀⠀⠀⠀⠀    Available Skills                         │\n│   ⠀⢀⣠⣴⣶⠿⠋⣩⡿⣿⡿⠻⣿⡇⢠⡄⢸⣿⠟⢿⣿⢿⣍⠙⠿⣶⣦⣄⡀⠀    No skills installed                      │\n│   ⠀⠀⠉⠉⠁⠶⠟⠋⠀⠉⠀⢀⣈⣁⡈⢁⣈⣁⡀⠀⠉⠀⠙⠻⠶⠈⠉⠉⠀⠀                                             │\n│   ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣴⣿⡿⠛⢁⡈⠛⢿⣿⣦⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀    0 tools · 0 skills · /help for commands  │\n│   ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠿⣿⣦⣤⣈⠁⢠⣴⣿⠿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                             │\n│   ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠉⠻⢿⣿⣦⡉⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                             │\n│   ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠘⢷⣦⣈⠛⠃⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                             │\n│   ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢠⣴⠦⠈⠙⠿⣦⡄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                             │\n│   ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠸⣿⣤⡈⠁⢤⣿⠇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                             │\n│   ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠛⠷⠄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                             │\n│   ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⠑⢶⣄⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                             │\n│   ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣿⠁⢰⡆⠈⡿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                             │\n│   ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠳⠈⣡⠞⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                             │\n│   ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                             │\n│                                                                              │\n│  claude-sonnet-4-5 · Nous Research                                           │\n│           /home/user/demo                                                    │\n╰──────────────────────────────────────────────────────────────────────────────╯";
    
    const REF_W94_PRIMARY: &str = "\n╭──────────────────── Hermes-RS v0.21.0 (2026.8.31) · upstream 63279301 ─────────────────────╮\n│                                        Available Tools                                     │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⡀⠀⣀⣀⠀⢀⣀⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│     ⠀⠀⠀⠀⠀⠀⢀⣠⣴⣾⣿⣿⣇⠸⣿⣿⠇⣸⣿⣿⣷⣦⣄⡀⠀⠀⠀⠀⠀⠀     Available Skills                                    │\n│     ⠀⢀⣠⣴⣶⠿⠋⣩⡿⣿⡿⠻⣿⡇⢠⡄⢸⣿⠟⢿⣿⢿⣍⠙⠿⣶⣦⣄⡀⠀     No skills installed                                 │\n│     ⠀⠀⠉⠉⠁⠶⠟⠋⠀⠉⠀⢀⣈⣁⡈⢁⣈⣁⡀⠀⠉⠀⠙⠻⠶⠈⠉⠉⠀⠀                                                         │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣴⣿⡿⠛⢁⡈⠛⢿⣿⣦⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀     0 tools · 0 skills · /help for commands             │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠿⣿⣦⣤⣈⠁⢠⣴⣿⠿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠉⠻⢿⣿⣦⡉⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠘⢷⣦⣈⠛⠃⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢠⣴⠦⠈⠙⠿⣦⡄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠸⣿⣤⡈⠁⢤⣿⠇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠛⠷⠄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⠑⢶⣄⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣿⠁⢰⡆⠈⡿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠳⠈⣡⠞⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│                                                                                            │\n│  gpt-5 · 128K context · Nous Research                                                      │\n│            /home/user/demo                                                                 │\n╰────────────────────────────────────────────────────────────────────────────────────────────╯";
    
    const REF_W95_PRIMARY: &str = "\n██╗  ██╗███████╗██████╗ ███╗   ███╗███████╗███████╗       █████╗  ██████╗ ███████╗███╗\n██╗████████╗\n██║  ██║██╔════╝██╔══██╗████╗ ████║██╔════╝██╔════╝      ██╔══██╗██╔════╝ ██╔════╝████╗\n██║╚══██╔══╝\n███████║█████╗  ██████╔╝██╔████╔██║█████╗  ███████╗█████╗███████║██║  ███╗█████╗  ██╔██╗ ██║\n██║\n██╔══██║██╔══╝  ██╔══██╗██║╚██╔╝██║██╔══╝  ╚════██║╚════╝██╔══██║██║   ██║██╔══╝  ██║╚██╗██║\n██║\n██║  ██║███████╗██║  ██║██║ ╚═╝ ██║███████╗███████║      ██║  ██║╚██████╔╝███████╗██║ ╚████║\n██║\n╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝     ╚═╝╚══════╝╚══════╝      ╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚═╝  ╚═══╝\n╚═╝\n\n╭───────────────────── Hermes-RS v0.21.0 (2026.8.31) · upstream 63279301 ─────────────────────╮\n│                                        Available Tools                                      │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⡀⠀⣀⣀⠀⢀⣀⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                          │\n│     ⠀⠀⠀⠀⠀⠀⢀⣠⣴⣾⣿⣿⣇⠸⣿⣿⠇⣸⣿⣿⣷⣦⣄⡀⠀⠀⠀⠀⠀⠀     Available Skills                                     │\n│     ⠀⢀⣠⣴⣶⠿⠋⣩⡿⣿⡿⠻⣿⡇⢠⡄⢸⣿⠟⢿⣿⢿⣍⠙⠿⣶⣦⣄⡀⠀     No skills installed                                  │\n│     ⠀⠀⠉⠉⠁⠶⠟⠋⠀⠉⠀⢀⣈⣁⡈⢁⣈⣁⡀⠀⠉⠀⠙⠻⠶⠈⠉⠉⠀⠀                                                          │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣴⣿⡿⠛⢁⡈⠛⢿⣿⣦⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀     0 tools · 0 skills · /help for commands              │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠿⣿⣦⣤⣈⠁⢠⣴⣿⠿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                          │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠉⠻⢿⣿⣦⡉⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                          │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠘⢷⣦⣈⠛⠃⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                          │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢠⣴⠦⠈⠙⠿⣦⡄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                          │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠸⣿⣤⡈⠁⢤⣿⠇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                          │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠛⠷⠄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                          │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⠑⢶⣄⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                          │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣿⠁⢰⡆⠈⡿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                          │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠳⠈⣡⠞⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                          │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                          │\n│                                                                                             │\n│  gpt-5 · 128K context · Nous Research                                                       │\n│            /home/user/demo                                                                  │\n╰─────────────────────────────────────────────────────────────────────────────────────────────╯";
    
    const REF_W100_NOMODEL: &str = "\n██╗  ██╗███████╗██████╗ ███╗   ███╗███████╗███████╗       █████╗  ██████╗ ███████╗███╗\n██╗████████╗\n██║  ██║██╔════╝██╔══██╗████╗ ████║██╔════╝██╔════╝      ██╔══██╗██╔════╝ ██╔════╝████╗\n██║╚══██╔══╝\n███████║█████╗  ██████╔╝██╔████╔██║█████╗  ███████╗█████╗███████║██║  ███╗█████╗  ██╔██╗ ██║   ██║\n██╔══██║██╔══╝  ██╔══██╗██║╚██╔╝██║██╔══╝  ╚════██║╚════╝██╔══██║██║   ██║██╔══╝  ██║╚██╗██║   ██║\n██║  ██║███████╗██║  ██║██║ ╚═╝ ██║███████╗███████║      ██║  ██║╚██████╔╝███████╗██║ ╚████║   ██║\n╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝     ╚═╝╚══════╝╚══════╝      ╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚═╝  ╚═══╝   ╚═╝\n\n╭─────────────────────── Hermes-RS v0.21.0 (2026.8.31) · upstream 63279301 ────────────────────────╮\n│                                                    Available Tools                               │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⡀⠀⣀⣀⠀⢀⣀⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│           ⠀⠀⠀⠀⠀⠀⢀⣠⣴⣾⣿⣿⣇⠸⣿⣿⠇⣸⣿⣿⣷⣦⣄⡀⠀⠀⠀⠀⠀⠀           Available Skills                              │\n│           ⠀⢀⣠⣴⣶⠿⠋⣩⡿⣿⡿⠻⣿⡇⢠⡄⢸⣿⠟⢿⣿⢿⣍⠙⠿⣶⣦⣄⡀⠀           No skills installed                           │\n│           ⠀⠀⠉⠉⠁⠶⠟⠋⠀⠉⠀⢀⣈⣁⡈⢁⣈⣁⡀⠀⠉⠀⠙⠻⠶⠈⠉⠉⠀⠀                                                         │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣴⣿⡿⠛⢁⡈⠛⢿⣿⣦⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀           0 tools · 0 skills · /help for commands       │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠿⣿⣦⣤⣈⠁⢠⣴⣿⠿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠉⠻⢿⣿⣦⡉⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠘⢷⣦⣈⠛⠃⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢠⣴⠦⠈⠙⠿⣦⡄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠸⣿⣤⡈⠁⢤⣿⠇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠛⠷⠄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⠑⢶⣄⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣿⠁⢰⡆⠈⡿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠳⠈⣡⠞⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│           ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                         │\n│                                                                                                  │\n│  no model configured — run /model or hermes setup                                                │\n│                  /home/user/demo                                                                 │\n╰──────────────────────────────────────────────────────────────────────────────────────────────────╯";
    
    const REF_W100_SESSION: &str = "\n██╗  ██╗███████╗██████╗ ███╗   ███╗███████╗███████╗       █████╗  ██████╗ ███████╗███╗\n██╗████████╗\n██║  ██║██╔════╝██╔══██╗████╗ ████║██╔════╝██╔════╝      ██╔══██╗██╔════╝ ██╔════╝████╗\n██║╚══██╔══╝\n███████║█████╗  ██████╔╝██╔████╔██║█████╗  ███████╗█████╗███████║██║  ███╗█████╗  ██╔██╗ ██║   ██║\n██╔══██║██╔══╝  ██╔══██╗██║╚██╔╝██║██╔══╝  ╚════██║╚════╝██╔══██║██║   ██║██╔══╝  ██║╚██╗██║   ██║\n██║  ██║███████╗██║  ██║██║ ╚═╝ ██║███████╗███████║      ██║  ██║╚██████╔╝███████╗██║ ╚████║   ██║\n╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝     ╚═╝╚══════╝╚══════╝      ╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚═╝  ╚═══╝   ╚═╝\n\n╭─────────────────────── Hermes-RS v0.21.0 (2026.8.31) · upstream 63279301 ────────────────────────╮\n│                                  Available Tools                                                 │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⡀⠀⣀⣀⠀⢀⣀⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                                  │\n│  ⠀⠀⠀⠀⠀⠀⢀⣠⣴⣾⣿⣿⣇⠸⣿⣿⠇⣸⣿⣿⣷⣦⣄⡀⠀⠀⠀⠀⠀⠀  Available Skills                                                │\n│  ⠀⢀⣠⣴⣶⠿⠋⣩⡿⣿⡿⠻⣿⡇⢠⡄⢸⣿⠟⢿⣿⢿⣍⠙⠿⣶⣦⣄⡀⠀  No skills installed                                             │\n│  ⠀⠀⠉⠉⠁⠶⠟⠋⠀⠉⠀⢀⣈⣁⡈⢁⣈⣁⡀⠀⠉⠀⠙⠻⠶⠈⠉⠉⠀⠀                                                                  │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣴⣿⡿⠛⢁⡈⠛⢿⣿⣦⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀  0 tools · 0 skills · /help for commands                         │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠿⣿⣦⣤⣈⠁⢠⣴⣿⠿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                                  │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠉⠻⢿⣿⣦⡉⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                                  │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠘⢷⣦⣈⠛⠃⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                                  │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢠⣴⠦⠈⠙⠿⣦⡄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                                  │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠸⣿⣤⡈⠁⢤⣿⠇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                                  │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠛⠷⠄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                                  │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⠑⢶⣄⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                                  │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣿⠁⢰⡆⠈⡿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                                  │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠳⠈⣡⠞⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                                  │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                                  │\n│                                                                                                  │\n│      gpt-5 · Nous Research                                                                       │\n│         /home/user/demo                                                                          │\n│       Session: sess_abc123                                                                       │\n╰──────────────────────────────────────────────────────────────────────────────────────────────────╯";
    
    const REF_W100_MCP: &str = "\n██╗  ██╗███████╗██████╗ ███╗   ███╗███████╗███████╗       █████╗  ██████╗ ███████╗███╗\n██╗████████╗\n██║  ██║██╔════╝██╔══██╗████╗ ████║██╔════╝██╔════╝      ██╔══██╗██╔════╝ ██╔════╝████╗\n██║╚══██╔══╝\n███████║█████╗  ██████╔╝██╔████╔██║█████╗  ███████╗█████╗███████║██║  ███╗█████╗  ██╔██╗ ██║   ██║\n██╔══██║██╔══╝  ██╔══██╗██║╚██╔╝██║██╔══╝  ╚════██║╚════╝██╔══██║██║   ██║██╔══╝  ██║╚██╗██║   ██║\n██║  ██║███████╗██║  ██║██║ ╚═╝ ██║███████╗███████║      ██║  ██║╚██████╔╝███████╗██║ ╚████║   ██║\n╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝     ╚═╝╚══════╝╚══════╝      ╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚═╝  ╚═══╝   ╚═╝\n\n╭─────────────────────── Hermes-RS v0.21.0 (2026.8.31) · upstream 63279301 ────────────────────────╮\n│                                        Available Tools                                           │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⡀⠀⣀⣀⠀⢀⣀⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀     other: file_read                                          │\n│     ⠀⠀⠀⠀⠀⠀⢀⣠⣴⣾⣿⣿⣇⠸⣿⣿⠇⣸⣿⣿⣷⣦⣄⡀⠀⠀⠀⠀⠀⠀                                                               │\n│     ⠀⢀⣠⣴⣶⠿⠋⣩⡿⣿⡿⠻⣿⡇⢠⡄⢸⣿⠟⢿⣿⢿⣍⠙⠿⣶⣦⣄⡀⠀     MCP Servers                                               │\n│     ⠀⠀⠉⠉⠁⠶⠟⠋⠀⠉⠀⢀⣈⣁⡈⢁⣈⣁⡀⠀⠉⠀⠙⠻⠶⠈⠉⠉⠀⠀     demo (stdio) — configured                                 │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣴⣿⡿⠛⢁⡈⠛⢿⣿⣦⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                               │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠿⣿⣦⣤⣈⠁⢠⣴⣿⠿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀     Available Skills                                          │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠉⠻⢿⣿⣦⡉⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀     No skills installed                                       │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠘⢷⣦⣈⠛⠃⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                               │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢠⣴⠦⠈⠙⠿⣦⡄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀     1 tools · 0 skills · /help for commands                   │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠸⣿⣤⡈⠁⢤⣿⠇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                               │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠛⠷⠄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                               │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⠑⢶⣄⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                               │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣿⠁⢰⡆⠈⡿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                               │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠳⠈⣡⠞⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                               │\n│     ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                                               │\n│                                                                                                  │\n│  gpt-5 · 128K context · Nous Research                                                            │\n│            /home/user/demo                                                                       │\n╰──────────────────────────────────────────────────────────────────────────────────────────────────╯";
    
    const REF_W60_SHRINK: &str = "\n╭─── Hermes-RS v0.21.0 (2026.8.31) · upstream 63279301 ────╮\n│                             Available Tools              │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⡀⠀⣀⣀⠀⢀⣀⡀⠀⠀⠀⠀…  other: file_read             │\n│  ⠀⠀⠀⠀⠀⠀⢀⣠⣴⣾⣿⣿⣇⠸⣿⣿⠇⣸⣿⣿⣷⣦⣄⡀…                               │\n│  ⠀⢀⣠⣴⣶⠿⠋⣩⡿⣿⡿⠻⣿⡇⢠⡄⢸⣿⠟⢿⣿⢿⣍⠙…  Available Skills             │\n│  ⠀⠀⠉⠉⠁⠶⠟⠋⠀⠉⠀⢀⣈⣁⡈⢁⣈⣁⡀⠀⠉⠀⠙⠻…  No skills installed          │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣴⣿⡿⠛⢁⡈⠛⢿⣿⣦⠀⠀⠀⠀…                               │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠿⣿⣦⣤⣈⠁⢠⣴⣿⠿⠀⠀⠀⠀…  1 tools · 0 skills · /help   │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠉⠻⢿⣿⣦⡉⠁⠀⠀⠀⠀⠀…  for commands                 │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠘⢷⣦⣈⠛⠃⠀⠀⠀⠀⠀⠀…                               │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢠⣴⠦⠈⠙⠿⣦⡄⠀⠀⠀⠀⠀…                               │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠸⣿⣤⡈⠁⢤⣿⠇⠀⠀⠀⠀⠀…                               │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠛⠷⠄⠀⠀⠀⠀⠀⠀⠀…                               │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⠑⢶⣄⡀⠀⠀⠀⠀⠀⠀…                               │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣿⠁⢰⡆⠈⡿⠀⠀⠀⠀⠀⠀…                               │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠳⠈⣡⠞⠁⠀⠀⠀⠀⠀⠀…                               │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠀⠀⠀⠀⠀⠀⠀⠀⠀…                               │\n│                                                          │\n│  claude-sonnet-4-5 · 200K                                │\n│   context · Nous Research                                │\n│       /home/user/demo                                    │\n╰──────────────────────────────────────────────────────────╯";
    
    const REF_W70_NOMODEL: &str = "\n╭──────── Hermes-RS v0.21.0 (2026.8.31) · upstream 63279301 ─────────╮\n│                                  Available Tools                   │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⡀⠀⣀⣀⠀⢀⣀⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                    │\n│  ⠀⠀⠀⠀⠀⠀⢀⣠⣴⣾⣿⣿⣇⠸⣿⣿⠇⣸⣿⣿⣷⣦⣄⡀⠀⠀⠀⠀⠀⠀  Available Skills                  │\n│  ⠀⢀⣠⣴⣶⠿⠋⣩⡿⣿⡿⠻⣿⡇⢠⡄⢸⣿⠟⢿⣿⢿⣍⠙⠿⣶⣦⣄⡀⠀  No skills installed               │\n│  ⠀⠀⠉⠉⠁⠶⠟⠋⠀⠉⠀⢀⣈⣁⡈⢁⣈⣁⡀⠀⠉⠀⠙⠻⠶⠈⠉⠉⠀⠀                                    │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣴⣿⡿⠛⢁⡈⠛⢿⣿⣦⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀  0 tools · 0 skills · /help for    │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠿⣿⣦⣤⣈⠁⢠⣴⣿⠿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀  commands                          │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠉⠻⢿⣿⣦⡉⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                    │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠘⢷⣦⣈⠛⠃⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                    │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢠⣴⠦⠈⠙⠿⣦⡄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                    │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠸⣿⣤⡈⠁⢤⣿⠇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                    │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠛⠷⠄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                    │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⠑⢶⣄⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                    │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣿⠁⢰⡆⠈⡿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                    │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠳⠈⣡⠞⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                    │\n│  ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀                                    │\n│                                                                    │\n│    no model configured — run                                       │\n│      /model or hermes setup                                        │\n│         /home/user/demo                                            │\n╰────────────────────────────────────────────────────────────────────╯";

    #[test]
    fn brand_strings_are_verbatim() {
        assert_eq!(PROMPT_SYMBOL, "❯ ");
        assert_eq!(RESPONSE_LABEL, " ⚕ Hermes ");
        assert_eq!(TOOL_PREFIX, "┊");
        assert_eq!(GOODBYE, "Goodbye! ⚕");
        assert_eq!(HELP_HEADER, "(^_^)? Available Commands");
        assert_eq!(
            VERSION_LABEL,
            "Hermes-RS v0.21.0 (2026.8.31) · upstream 63279301"
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
        assert_eq!(LOGO_LINES, 6);
        assert_eq!(CADUCEUS_LINES, 15);
        let logo = logo_lines();
        assert_eq!(
            logo[0].text,
            "██╗  ██╗███████╗██████╗ ███╗   ███╗███████╗███████╗       █████╗  ██████╗ ███████╗███╗   ██╗████████╗"
        );
        assert_eq!(logo[0].style.fg, Some(Color::Rgb(255, 215, 0)));
        assert!(logo[0].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(logo[1].style.fg, Some(Color::Rgb(255, 215, 0)));
        assert!(logo[1].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(logo[2].style.fg, Some(Color::Rgb(255, 191, 0)));
        assert!(!logo[2].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(logo[3].style.fg, Some(Color::Rgb(255, 191, 0)));
        assert!(!logo[3].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(logo[4].style.fg, Some(Color::Rgb(205, 127, 50)));
        assert!(!logo[4].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(logo[5].style.fg, Some(Color::Rgb(205, 127, 50)));
        assert!(!logo[5].style.add_modifier.contains(Modifier::BOLD));
        for (i, row) in logo.iter().enumerate() {
            assert!(
                (98..=101).contains(&row.text.chars().count()),
                "logo row {i} width drifted"
            );
        }
    }

    #[test]
    fn response_label_width_is_pinned() {
        assert_eq!(
            RESPONSE_LABEL_WIDTH,
            RESPONSE_LABEL.chars().count(),
            "RESPONSE_LABEL_WIDTH must match the constant"
        );
        assert_eq!(RESPONSE_LABEL_WIDTH, 10);
    }

    // -- Spec 017 T02: context formatting --------------------------------

    #[test]
    fn format_context_length_matches_python() {
        assert_eq!(format_context_length(999), "999");
        assert_eq!(format_context_length(1_000), "1K");
        assert_eq!(format_context_length(1_500), "1.5K");
        assert_eq!(format_context_length(99_500), "99.5K");
        assert_eq!(format_context_length(128_000), "128K");
        assert_eq!(format_context_length(200_000), "200K");
        assert_eq!(format_context_length(995_000), "995K");
        assert_eq!(format_context_length(1_000_000), "1M");
        assert_eq!(format_context_length(1_048_576), "1M");
        assert_eq!(format_context_length(1_500_000), "1.5M");
    }

    // -- Spec 017 T02: divide_line (rich._wrap port) ----------------------

    /// The pinned expectations are Python `divide_line` offsets (char space);
    /// Rust operates in byte space, so convert before comparing.
    fn byte_breaks(text: &str, char_breaks: &[usize]) -> Vec<usize> {
        char_breaks
            .iter()
            .map(|&b| text.char_indices().nth(b).map(|(i, _)| i).unwrap_or(text.len()))
            .collect()
    }

    #[test]
    fn divide_line_matches_rich() {
        assert_eq!(divide_line("a  b c", 4), vec![5]);
        assert_eq!(divide_line("a  b c", 5), vec![5]);
        let nomodel = "no model configured — run /model or hermes setup";
        assert_eq!(divide_line(nomodel, 30), byte_breaks(nomodel, &[26]));
        let summary = "1 tools · 0 skills · /help for commands";
        assert_eq!(divide_line(summary, 27), byte_breaks(summary, &[27]));
        let model_line = "claude-sonnet-4-5 · 200K context · Nous Research";
        assert_eq!(divide_line(model_line, 25), byte_breaks(model_line, &[25]));
        // Logo rows (pinned against Python `divide_line` on the verbatim
        // art): rows 1-2 break at 89; rows 3-6 break at 95 (width 95) or
        // not at all (width >= 98).
        let logo = logo_lines();
        assert_eq!(divide_line(logo[0].text, 95), byte_breaks(logo[0].text, &[89]));
        assert_eq!(divide_line(logo[1].text, 95), byte_breaks(logo[1].text, &[89]));
        for row in &logo[2..6] {
            assert_eq!(divide_line(row.text, 95), byte_breaks(row.text, &[95]));
        }
        assert_eq!(divide_line(logo[0].text, 100), byte_breaks(logo[0].text, &[89]));
        assert_eq!(divide_line(logo[1].text, 100), byte_breaks(logo[1].text, &[89]));
        for row in &logo[2..6] {
            assert_eq!(divide_line(row.text, 100), Vec::<usize>::new());
        }
        assert_eq!(divide_line(logo[0].text, 101), Vec::<usize>::new());
        assert_eq!(divide_line(logo[5].text, 120), Vec::<usize>::new());
    }

    // -- Spec 017 T02: shrink (Rich table width algorithm) ----------------

    #[test]
    fn shrink_columns_matches_rich() {
        // w60_shrink reference: L0=48 (model+ctx line), R0=39 (summary),
        // content 54 -> Rich yields (25, 27).
        assert_eq!(shrink_columns(48 + 2, 39, 54), (25, 27));
        // w70_nomodel: L0=48, R0=39, content 64 -> (30, 32).
        assert_eq!(shrink_columns(48 + 2, 39, 64), (30, 32));
    }

    // -- Spec 017 T02: byte-parity against the Python reference -----------

    fn ref_lines(reference: &str) -> Vec<String> {
        reference.lines().map(str::to_owned).collect()
    }

    fn banner_info(
        model: Option<&str>,
        ctx: Option<u64>,
        tools: &[&str],
        mcp: &[&str],
        session: Option<&str>,
    ) -> BannerInfo {
        BannerInfo {
            model: model.map(str::to_owned),
            context_tokens: ctx,
            cwd: "/home/user/demo".to_owned(),
            session_id: session.map(str::to_owned),
            tools: tools.iter().map(|s| s.to_string()).collect(),
            mcp_servers: mcp.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Renders the banner at `raw_width` and returns the plain (style-free),
    /// right-trimmed rows — the same normalization as the reference files.
    fn plain_banner(raw_width: u16, info: &BannerInfo) -> Vec<String> {
        let theme = HermesTheme::dark_canonical();
        let buf = compose_banner(raw_width, info, &theme);
        let area = buf.area;
        (0..area.height)
            .map(|y| {
                let line: String = (0..area.width)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect();
                line.trim_end().to_owned()
            })
            .collect()
    }

    #[test]
    fn banner_w100_primary_matches_python_reference() {
        let info = banner_info(
            Some("anthropic/claude-sonnet-4-5"),
            Some(200_000),
            &["file_read", "file_write", "web_search"],
            &[],
            None,
        );
        assert_eq!(
            plain_banner(100, &info),
            ref_lines(REF_W100_PRIMARY),
            "w100 primary banner must match Python v0.21.0 byte-for-byte"
        );
    }

    #[test]
    fn banner_w80_primary_matches_python_reference() {
        let info = banner_info(
            Some("anthropic/claude-sonnet-4-5"),
            None,
            &[],
            &[],
            None,
        );
        assert_eq!(
            plain_banner(80, &info),
            ref_lines(REF_W80_PRIMARY),
            "w80 banner (no logo) must match Python v0.21.0 byte-for-byte"
        );
    }

    #[test]
    fn banner_w94_primary_matches_python_reference() {
        let info = banner_info(Some("gpt-5"), Some(128_000), &[], &[], None);
        assert_eq!(
            plain_banner(94, &info),
            ref_lines(REF_W94_PRIMARY),
            "w94 banner (just under the logo threshold) must match"
        );
    }

    #[test]
    fn banner_w95_primary_matches_python_reference() {
        let info = banner_info(Some("gpt-5"), Some(128_000), &[], &[], None);
        assert_eq!(
            plain_banner(95, &info),
            ref_lines(REF_W95_PRIMARY),
            "w95 banner (logo threshold, wrapped 12-line logo) must match"
        );
    }

    #[test]
    fn banner_w100_nomodel_matches_python_reference() {
        let info = banner_info(Some(""), None, &[], &[], None);
        assert_eq!(
            plain_banner(100, &info),
            ref_lines(REF_W100_NOMODEL),
            "w100 no-model banner (red state line) must match"
        );
    }

    #[test]
    fn banner_w100_session_matches_python_reference() {
        let info = banner_info(Some("gpt-5"), None, &[], &[], Some("sess_abc123"));
        assert_eq!(
            plain_banner(100, &info),
            ref_lines(REF_W100_SESSION),
            "w100 session banner (21-row inner) must match"
        );
    }

    #[test]
    fn banner_w100_mcp_matches_python_reference() {
        let info = banner_info(Some("gpt-5"), Some(128_000), &["file_read"], &["demo"], None);
        assert_eq!(
            plain_banner(100, &info),
            ref_lines(REF_W100_MCP),
            "w100 MCP-configured banner must match"
        );
    }

    #[test]
    fn banner_w60_shrink_matches_python_reference() {
        let info = banner_info(
            Some("anthropic/claude-sonnet-4-5"),
            Some(200_000),
            &["file_read"],
            &[],
            None,
        );
        assert_eq!(
            plain_banner(60, &info),
            ref_lines(REF_W60_SHRINK),
            "w60 banner (Rich shrink zone: wrapped model, cropped art) must match"
        );
    }

    #[test]
    fn banner_w70_nomodel_matches_python_reference() {
        let info = banner_info(Some(""), None, &[], &[], None);
        assert_eq!(
            plain_banner(70, &info),
            ref_lines(REF_W70_NOMODEL),
            "w70 no-model banner (shrink zone) must match"
        );
    }

    #[test]
    fn toolset_truncation_matches_python_rule() {
        // Sorted names; joined is 50 cells (> 45) so the 42-cell budget
        // stops after `tool_one` (32 + 10 + 2 = 44 > 42 for `tool_three`).
        let info = banner_info(
            Some("gpt-5"),
            None,
            &[
                "tool_one",
                "tool_two",
                "tool_three",
                "tool_four",
                "tool_five",
            ],
            &[],
            None,
        );
        let got = plain_banner(100, &info);
        assert!(
            got.iter().any(|l| l.contains("other: tool_five, tool_four, tool_one, ...")),
            "truncated toolset row expected; got: {got:?}"
        );
    }

    // -- Spec 017 T02: colors (Phase 0 finding B/F) ------------------------

    fn banner_buffer(raw_width: u16, info: &BannerInfo) -> Buffer {
        let theme = HermesTheme::dark_canonical();
        compose_banner(raw_width, info, &theme)
    }

    #[test]
    fn banner_colors_match_phase0() {
        let info = banner_info(Some("gpt-5"), Some(128_000), &[], &[], None);
        let buf = banner_buffer(100, &info);
        // Panel top row y = 1 + 8 (wrapped logo) + 1 = 10.
        assert_eq!(buf[(0, 10)].symbol(), "╭");
        assert_eq!(buf[(0, 10)].fg, Color::Rgb(205, 127, 50), "border #CD7F32");
        // Title: inner 98, block 51, left dashes (98-51)/2 = 23 -> x = 1+23+1.
        assert_eq!(buf[(25, 10)].symbol(), "H");
        assert_eq!(buf[(25, 10)].fg, Color::Rgb(255, 215, 0), "title #FFD700");
        assert!(buf[(25, 10)].modifier.contains(Modifier::BOLD), "title bold");
        // Left column (L=36): art offset (36-30)/2 = 3 -> x = 1+2+3 = 6,
        // first art row y = 10+1+1 = 12 (bronze tier).
        assert_eq!(buf[(6, 12)].fg, Color::Rgb(205, 127, 50), "caduceus bronze");
        // Model row (inner row 17) y = 28, x = 3: accent name.
        assert_eq!(buf[(3, 28)].symbol(), "g");
        assert_eq!(buf[(3, 28)].fg, Color::Rgb(255, 191, 0), "model #FFBF00");
        // Cwd row y = 29, x = 1+2+10 = 13 (offset (36-16)/2 = 10): dim.
        assert_eq!(buf[(13, 29)].fg, Color::Rgb(184, 134, 11), "cwd #B8860B");
        // Right column header at x = 1+2+36+2 = 41, y = 11: bold accent.
        assert_eq!(buf[(41, 11)].symbol(), "A");
        assert_eq!(buf[(41, 11)].fg, Color::Rgb(255, 191, 0), "header accent");
        assert!(buf[(41, 11)].modifier.contains(Modifier::BOLD), "header bold");
    }

    #[test]
    fn banner_session_color_is_pinned() {
        let info = banner_info(Some("gpt-5"), None, &[], &[], Some("sess_abc123"));
        let buf = banner_buffer(100, &info);
        // Session row = inner row 19 -> y = 10+1+19 = 30; L=30 (art),
        // offset (30-20)/2 = 5 -> x = 1+2+5 = 8.
        assert_eq!(buf[(8, 30)].symbol(), "S");
        assert_eq!(
            buf[(8, 30)].fg,
            Color::Rgb(139, 134, 130),
            "session #8B8682"
        );
    }

    #[test]
    fn banner_no_model_line_is_bold_red() {
        let info = banner_info(Some(""), None, &[], &[], None);
        let buf = banner_buffer(100, &info);
        // No-model row = inner row 17 -> y = 28, x = 3.
        assert_eq!(buf[(3, 28)].symbol(), "n");
        assert_eq!(buf[(3, 28)].fg, Color::Rgb(255, 0, 0), "red state");
        assert!(buf[(3, 28)].modifier.contains(Modifier::BOLD), "bold red");
    }

    #[test]
    fn banner_ansi_truecolor_and_256_paths() {
        let theme = HermesTheme::dark_canonical();
        let info = banner_info(Some("gpt-5"), Some(128_000), &[], &[], None);
        let mut tc = Vec::new();
        write_banner(&mut tc, &theme, 100, &info, ColorDepth::Truecolor).unwrap();
        let tc = String::from_utf8(tc).unwrap();
        assert!(
            tc.contains("\x1b[1;38;2;255;215;0m"),
            "title must be bold #FFD700 (truecolor)"
        );
        assert!(
            tc.contains("\x1b[38;2;205;127;50m"),
            "border must be #CD7F32 (truecolor)"
        );
        assert!(
            tc.contains("\x1b[38;2;255;191;0m"),
            "accent #FFBF00 (model name + headers)"
        );
        assert!(
            tc.contains("\x1b[38;2;184;134;11m"),
            "dim #B8860B (cwd/labels)"
        );
        assert!(tc.contains("Hermes-RS v0.21.0 (2026.8.31) · upstream 63279301"));
        assert!(tc.contains("██╗"), "logo present at width 100");
        let mut c256 = Vec::new();
        write_banner(&mut c256, &theme, 100, &info, ColorDepth::Color256)
            .unwrap();
        let c256 = String::from_utf8(c256).unwrap();
        assert!(!c256.contains("38;2;"), "256-color path must not emit truecolor");
        assert!(c256.contains("38;5;"), "256-color path must use palette indices");
    }

    #[test]
    fn banner_respects_min_width_60() {
        let theme = HermesTheme::dark_canonical();
        let buf = compose_banner(40, &BannerInfo::default(), &theme);
        assert!(buf.area.width >= 60, "panel must clamp up to 60");
        let first: String = (0..buf.area.width)
            .map(|x| buf[(x, 0)].symbol().to_string())
            .collect();
        assert_eq!(first.trim_end(), "", "leading blank row");
        let second: String = (0..buf.area.width)
            .map(|x| buf[(x, 1)].symbol().to_string())
            .collect();
        assert!(second.starts_with('╭'), "panel top border at row 1");
    }

    #[test]
    fn banner_respects_max_width_120() {
        let theme = HermesTheme::dark_canonical();
        let info = banner_info(Some("gpt-5"), Some(128_000), &[], &[], None);
        let buf = compose_banner(300, &info, &theme);
        assert_eq!(buf.area.width, 120, "panel must clamp down to 120");
        // At raw width 300 the logo is not wrapped (natural width 101).
        let row2: String = (0..120)
            .map(|x| buf[(x, 1)].symbol().to_string())
            .collect();
        assert_eq!(
            row2.trim_end(),
            "██╗  ██╗███████╗██████╗ ███╗   ███╗███████╗███████╗       █████╗  ██████╗ ███████╗███╗   ██╗████████╗"
        );
    }

    /// Full plain-text dump of a buffer (borrow-based, test helper).
    fn buffer_text(buf: &Buffer) -> String {
        (0..buf.area.height)
            .flat_map(|y| {
                (0..buf.area.width)
                    .map(move |x| buf[(x, y)].symbol().to_string())
            })
            .collect()
    }

    #[test]
    fn banner_logo_appears_only_at_95_or_wider() {
        let info = banner_info(Some("gpt-5"), Some(128_000), &[], &[], None);
        let narrow = banner_buffer(94, &info);
        let w94 = buffer_text(&narrow);
        assert!(!w94.contains("██╗"), "logo must be hidden below width 95");
        let wide = banner_buffer(95, &info);
        let w95 = buffer_text(&wide);
        assert!(w95.contains("██╗"), "logo must appear at width 95");
    }

    // -- art verbatim (retained from Spec 013) -----------------------------

    #[test]
    fn caduceus_verbatim() {
        let cad = caduceus_lines();
        assert_eq!(
            cad[0].text,
            "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⡀⠀⣀⣀⠀⢀⣀⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀"
        );
        assert_eq!(
            cad[14].text,
            "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀"
        );
        assert_eq!(cad[0].style.fg, Some(Color::Rgb(205, 127, 50)));
        assert_eq!(cad[4].style.fg, Some(Color::Rgb(255, 215, 0)));
        assert_eq!(cad[10].style.fg, Some(Color::Rgb(184, 134, 11)));
        for row in cad.iter() {
            assert_eq!(row.text.chars().count(), 30, "caduceus rows are 30 cells");
        }
    }

    #[test]
    fn logo_width_constant_is_pinned() {
        assert_eq!(LOGO_WIDTH, 101);
        for row in logo_lines().iter() {
            assert!(row.text.chars().count() <= LOGO_WIDTH);
        }
    }
}
