//! Spec 013 — Hermes Python UI parity, Ticket 05: status bar (spec §8).
//!
//! The single-line bottom-chrome status bar, rendered verbatim from the
//! Python `prompt_toolkit` fragment logic (`_get_status_bar_fragments`):
//!
//! * ` ⚕ ` + model name (strong, `#FFD700` bold), segments separated by
//!   ` · ` (narrow tiers) or ` │ ` (full tier) in `#8B8682` dim;
//! * context gauge with tiered colors: `#8FBC8F` good (<50%), `#FFD700`
//!   warn (≥50), `#FF8C00` bad (>80), `#FF6B6B` critical (≥95) — the block
//!   bar `[████░░░░░░]` appears only in the full (≥76 col) tier;
//! * badges: compression `🗜️ N`, background tasks `▶ N` / processes `⚙ N` /
//!   subagents `⛓ N`, goal `⊙ goal [used/max]`, YOLO `⚠ YOLO`;
//! * three width tiers: `<52` compact, `<76` medium, `>=76` full;
//! * navy background `#1a1a2e` over the whole line, default text `#C0C0C0`,
//!   session-title badge right-aligned (gold bg, navy text).
//!
//! The module is display-only and pure: `build_fragments`/`render_line`
//! take plain data and return strings, so layout is unit-testable without a
//! TTY. Canonical state (SQLite) is never touched (invariant 5).

use crate::tui::theme::{detect_color_depth, truecolor_to_256, ColorDepth};
use ratatui::style::Color;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// One colored piece of the status bar (a Python `FormattedText` fragment).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SbarFragment {
    pub style: SbarStyle,
    pub text: String,
}

/// Status-bar style classes — names mirror the Python `status-bar*` styles
/// (spec §8, verbatim).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SbarStyle {
    /// `status-bar` — default text on navy (`#C0C0C0`).
    Bar,
    /// `status-bar-strong` — model name, badges (`#FFD700` bold).
    Strong,
    /// `status-bar-dim` — separators, duration (`#8B8682`).
    Dim,
    /// `status-bar-good` — context <50% (`#8FBC8F` bold).
    Good,
    /// `status-bar-warn` — context ≥50% (`#FFD700` bold).
    Warn,
    /// `status-bar-bad` — context >80% (`#FF8C00` bold).
    Bad,
    /// `status-bar-critical` — context ≥95% (`#FF6B6B` bold).
    Critical,
    /// `status-bar-yolo` — `⚠ YOLO` (`#FF4444` bold).
    Yolo,
    /// `status-bar-session-title` — right badge (navy on gold, bold).
    SessionTitle,
}

/// Runtime data for one status-bar render. Everything is display input;
/// constructing this never mutates canonical state.
#[derive(Debug, Clone)]
pub struct StatusBarData {
    /// Active provider/model display name (before [`model_short`]).
    pub model: String,
    /// Current context token estimate.
    pub context_tokens: u64,
    /// Advisory context limit (`None` → gauge shows `--` dim).
    pub context_limit: Option<u64>,
    /// Compression events so far (`0` hides the badge).
    pub compressions: u32,
    /// Active background tasks / processes / subagents (`0` hides).
    pub bg_tasks: u32,
    pub bg_processes: u32,
    pub bg_subagents: u32,
    /// Whether a goal is actively being pursued.
    pub goal_active: bool,
    /// Goal turns used / budget (`max == 0` → bare `⊙ goal`).
    pub goal_turns_used: u32,
    pub goal_max_turns: u32,
    /// YOLO (auto-approve) mode active.
    pub yolo: bool,
    /// Session duration in seconds.
    pub duration_secs: f64,
    /// Focus-view badge (hidden when `None`).
    pub focus_label: Option<String>,
    /// Session title badge, right-aligned (empty → no badge).
    pub session_title: String,
}

impl Default for StatusBarData {
    fn default() -> Self {
        Self {
            model: "unknown".to_owned(),
            context_tokens: 0,
            context_limit: None,
            compressions: 0,
            bg_tasks: 0,
            bg_processes: 0,
            bg_subagents: 0,
            goal_active: false,
            goal_turns_used: 0,
            goal_max_turns: 0,
            yolo: false,
            duration_secs: 0.0,
            focus_label: None,
            session_title: String::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Formatting helpers (verbatim Python formulas)
// ---------------------------------------------------------------------------

/// Python `round()` — round-half-to-even (banker's), so block-bar fill
/// counts match `_build_context_bar` byte-for-byte.
fn py_round(x: f64) -> i64 {
    // Python 3 round(): round-half-to-even (0.5 is exact in binary, so the
    // halfway test is exact for non-negative x).
    let floor = x.floor();
    let frac = x - floor;
    if frac > 0.5 || (frac == 0.5 && (floor as i64) % 2 == 1) {
        (floor + 1.0) as i64
    } else {
        floor as i64
    }
}

/// `_build_context_bar(percent, width=10)` — `[████░░░░░░]`.
pub fn context_bar(percent: Option<u32>, width: usize) -> String {
    let safe = percent.map(|p| p.min(100)).unwrap_or(0) as f64;
    let filled = py_round(safe / 100.0 * width as f64).clamp(0, width as i64) as usize;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(width - filled))
}

/// `_status_bar_context_style(percent)` — tiered gauge color.
pub fn context_style(percent: Option<u32>) -> SbarStyle {
    match percent {
        None => SbarStyle::Dim,
        Some(p) if p >= 95 => SbarStyle::Critical,
        Some(p) if p > 80 => SbarStyle::Bad,
        Some(p) if p >= 50 => SbarStyle::Warn,
        Some(_) => SbarStyle::Good,
    }
}

/// Context usage percent (integer), `None` without a limit.
pub fn context_percent(data: &StatusBarData) -> Option<u32> {
    let limit = data.context_limit?;
    if limit == 0 {
        return None;
    }
    Some(((data.context_tokens * 100) / limit).min(100) as u32)
}

/// `format_duration_compact` — `42s` / `5m` / `2h 5m` / `1.5d`.
pub fn format_duration_compact(seconds: f64) -> String {
    let seconds = seconds.max(0.0);
    if seconds < 60.0 {
        return format!("{:.0}s", seconds);
    }
    let minutes = seconds / 60.0;
    if minutes < 60.0 {
        return format!("{:.0}m", minutes);
    }
    let hours = minutes / 60.0;
    if hours < 24.0 {
        let remaining_min = (minutes % 60.0) as u32;
        let h = hours as u32;
        return if remaining_min > 0 {
            format!("{h}h {remaining_min}m")
        } else {
            format!("{h}h")
        };
    }
    format!("{:.1}d", hours / 24.0)
}

/// `format_token_count_compact` — `999` / `1.23K` / `12K` / `1.5M` / `123M`.
pub fn format_token_count_compact(value: u64) -> String {
    if value < 1_000 {
        return value.to_string();
    }
    let units = [(1_000_000_000u64, "B"), (1_000_000, "M"), (1_000, "K")];
    let v = value as f64;
    for (threshold, suffix) in units {
        if value >= threshold {
            let scaled = v / threshold as f64;
            let text = if scaled < 10.0 {
                format!("{scaled:.2}")
            } else if scaled < 100.0 {
                format!("{scaled:.1}")
            } else {
                format!("{scaled:.0}")
            };
            let text = text.trim_end_matches('0').trim_end_matches('.');
            return format!("{text}{suffix}");
        }
    }
    unreachable!()
}

/// `banner._format_context_length` — `128000 → "128K"`, `1048576 → "1M"`.
pub fn format_context_length(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        let val = tokens as f64 / 1_000_000.0;
        let rounded = val.round();
        return if (val - rounded).abs() < 0.05 {
            format!("{rounded:.0}M")
        } else {
            format!("{val:.1}M")
        };
    }
    if tokens >= 1_000 {
        let val = tokens as f64 / 1_000.0;
        let rounded = val.round();
        return if (val - rounded).abs() < 0.05 {
            format!("{rounded:.0}K")
        } else {
            format!("{val:.1}K")
        };
    }
    tokens.to_string()
}

/// `model_short` — last slash segment, `.gguf` stripped, >26 chars →
/// first 23 + `...`.
pub fn model_short(name: &str) -> String {
    let mut s = name;
    if name.contains('/') {
        s = name.rsplit('/').next().unwrap_or(name);
    }
    if s.ends_with(".gguf") {
        s = &s[..s.len() - 5];
    }
    if s.chars().count() > 26 {
        let head: String = s.chars().take(23).collect();
        format!("{head}...")
    } else {
        s.to_owned()
    }
}

/// `_status_bar_goal_segment` — `⊙ goal 3/20`, or `⊙ goal`, or `""`.
pub fn goal_segment(data: &StatusBarData) -> Option<String> {
    if !data.goal_active {
        return None;
    }
    if data.goal_max_turns > 0 {
        Some(format!(
            "⊙ goal {}/{}",
            data.goal_turns_used, data.goal_max_turns
        ))
    } else {
        Some("⊙ goal".to_owned())
    }
}

// ---------------------------------------------------------------------------
// Fragment construction (verbatim tier logic)
// ---------------------------------------------------------------------------

fn append(frags: &mut Vec<SbarFragment>, sep: &str, pieces: Vec<SbarFragment>) {
    if !frags.is_empty() {
        frags.push(SbarFragment {
            style: SbarStyle::Dim,
            text: sep.to_owned(),
        });
    }
    frags.extend(pieces);
}

/// Build the status-bar fragments for a terminal width (Python
/// `_get_status_bar_fragments` tier logic, verbatim).
pub fn build_fragments(width: usize, data: &StatusBarData) -> Vec<SbarFragment> {
    let duration = format_duration_compact(data.duration_secs);
    let goal = goal_segment(data);
    let focus = data.focus_label.clone();
    let percent = context_percent(data);
    let percent_label = percent
        .map(|p| format!("{p}%"))
        .unwrap_or_else(|| "--".to_owned());

    let mut frags: Vec<SbarFragment> = Vec::new();
    frags.push(SbarFragment {
        style: SbarStyle::Bar,
        text: " ⚕ ".to_owned(),
    });
    frags.push(SbarFragment {
        style: SbarStyle::Strong,
        text: model_short(&data.model),
    });

    let yolo_piece = || {
        vec![SbarFragment {
            style: SbarStyle::Yolo,
            text: "⚠ YOLO".to_owned(),
        }]
    };

    if width < 52 {
        // Compact tier: model · duration · goal · focus · yolo
        if !duration.is_empty() {
            append(
                &mut frags,
                " · ",
                vec![SbarFragment {
                    style: SbarStyle::Dim,
                    text: duration.clone(),
                }],
            );
        }
        if let Some(g) = &goal {
            append(
                &mut frags,
                " · ",
                vec![SbarFragment {
                    style: SbarStyle::Strong,
                    text: g.clone(),
                }],
            );
        }
        if let Some(f) = &focus {
            append(
                &mut frags,
                " · ",
                vec![SbarFragment {
                    style: SbarStyle::Strong,
                    text: f.clone(),
                }],
            );
        }
        if data.yolo {
            append(&mut frags, " · ", yolo_piece());
        }
    } else if width < 76 {
        // Medium tier: + context %, compressions, bg counters
        append(
            &mut frags,
            " · ",
            vec![SbarFragment {
                style: context_style(percent),
                text: percent_label.clone(),
            }],
        );
        if data.compressions > 0 {
            append(
                &mut frags,
                " · ",
                vec![SbarFragment {
                    style: SbarStyle::Strong,
                    text: format!("🗜️ {}", data.compressions),
                }],
            );
        }
        if data.bg_tasks > 0 {
            append(
                &mut frags,
                " · ",
                vec![SbarFragment {
                    style: SbarStyle::Strong,
                    text: format!("▶ {}", data.bg_tasks),
                }],
            );
        }
        if data.bg_processes > 0 {
            append(
                &mut frags,
                " · ",
                vec![SbarFragment {
                    style: SbarStyle::Strong,
                    text: format!("⚙ {}", data.bg_processes),
                }],
            );
        }
        if data.bg_subagents > 0 {
            append(
                &mut frags,
                " · ",
                vec![SbarFragment {
                    style: SbarStyle::Strong,
                    text: format!("⛓ {}", data.bg_subagents),
                }],
            );
        }
        if let Some(g) = &goal {
            append(
                &mut frags,
                " · ",
                vec![SbarFragment {
                    style: SbarStyle::Strong,
                    text: g.clone(),
                }],
            );
        }
        if !duration.is_empty() {
            append(
                &mut frags,
                " · ",
                vec![SbarFragment {
                    style: SbarStyle::Dim,
                    text: duration.clone(),
                }],
            );
        }
        if let Some(f) = &focus {
            append(
                &mut frags,
                " · ",
                vec![SbarFragment {
                    style: SbarStyle::Strong,
                    text: f.clone(),
                }],
            );
        }
        if data.yolo {
            append(&mut frags, " · ", yolo_piece());
        }
    } else {
        // Full tier: context detail + block bar + everything, " │ " separated
        if let Some(limit) = data.context_limit {
            if limit > 0 {
                let context_label = format!(
                    "{}/{}",
                    format_token_count_compact(data.context_tokens),
                    format_context_length(limit)
                );
                append(
                    &mut frags,
                    " │ ",
                    vec![SbarFragment {
                        style: SbarStyle::Dim,
                        text: context_label,
                    }],
                );
            }
        }
        let bar_style = context_style(percent);
        append(
            &mut frags,
            " │ ",
            vec![
                SbarFragment {
                    style: bar_style,
                    text: context_bar(percent, 10),
                },
                SbarFragment {
                    style: SbarStyle::Dim,
                    text: " ".to_owned(),
                },
                SbarFragment {
                    style: bar_style,
                    text: percent_label,
                },
            ],
        );
        if data.compressions > 0 {
            append(
                &mut frags,
                " │ ",
                vec![SbarFragment {
                    style: SbarStyle::Strong,
                    text: format!("🗜️ {}", data.compressions),
                }],
            );
        }
        if data.bg_tasks > 0 {
            append(
                &mut frags,
                " │ ",
                vec![SbarFragment {
                    style: SbarStyle::Strong,
                    text: format!("▶ {}", data.bg_tasks),
                }],
            );
        }
        if data.bg_processes > 0 {
            append(
                &mut frags,
                " │ ",
                vec![SbarFragment {
                    style: SbarStyle::Strong,
                    text: format!("⚙ {}", data.bg_processes),
                }],
            );
        }
        if data.bg_subagents > 0 {
            append(
                &mut frags,
                " │ ",
                vec![SbarFragment {
                    style: SbarStyle::Strong,
                    text: format!("⛓ {}", data.bg_subagents),
                }],
            );
        }
        if let Some(g) = &goal {
            append(
                &mut frags,
                " │ ",
                vec![SbarFragment {
                    style: SbarStyle::Strong,
                    text: g.clone(),
                }],
            );
        }
        if !duration.is_empty() {
            append(
                &mut frags,
                " │ ",
                vec![SbarFragment {
                    style: SbarStyle::Dim,
                    text: duration.clone(),
                }],
            );
        }
        if let Some(f) = &focus {
            append(
                &mut frags,
                " │ ",
                vec![SbarFragment {
                    style: SbarStyle::Strong,
                    text: f.clone(),
                }],
            );
        }
        if data.yolo {
            append(&mut frags, " │ ", yolo_piece());
        }
    }

    // Trailing one-cell right margin (Python: final `("class:status-bar", " ")`).
    frags.push(SbarFragment {
        style: SbarStyle::Bar,
        text: " ".to_owned(),
    });
    frags
}

/// `_trim_status_bar_text` — display-width-aware trim with trailing `...`.
pub fn trim_text(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if text.width() <= max_width {
        return text.to_owned();
    }
    let ellipsis = "...";
    let ell_width = ellipsis.width();
    if max_width <= ell_width {
        return ellipsis.chars().take(max_width).collect();
    }
    let mut out = String::new();
    let mut width = 0usize;
    for ch in text.chars() {
        let cw = ch.width().unwrap_or(1);
        if width + cw + ell_width > max_width {
            break;
        }
        out.push(ch);
        width += cw;
    }
    format!("{}{ellipsis}", out.trim_end())
}

/// `_right_align_status_title_fragments` — pin the session-title badge to
/// the right edge (` ─ {title}`), padding the left content in dim.
pub fn right_align_title(frags: Vec<SbarFragment>, title: &str, width: usize) -> Vec<SbarFragment> {
    let title = title.trim();
    if title.is_empty() || width < 24 {
        return frags;
    }
    let title_width = (width / 3).clamp(6, 30);
    let badge = format!(" {} ", trim_text(title, title_width - 2));
    let suffix_width = " ─".width() + badge.width();
    let left_width = width.saturating_sub(suffix_width);
    let mut trimmed: Vec<SbarFragment> = Vec::new();
    let mut used = 0usize;
    for f in frags {
        let remaining = left_width.saturating_sub(used);
        if remaining == 0 {
            break;
        }
        let w = f.text.width();
        if w <= remaining {
            used += w;
            trimmed.push(f);
            continue;
        }
        let clipped = trim_text(&f.text, remaining);
        if !clipped.is_empty() {
            used += clipped.width();
            trimmed.push(SbarFragment {
                style: f.style,
                text: clipped,
            });
        }
        break;
    }
    if used < left_width {
        trimmed.push(SbarFragment {
            style: SbarStyle::Dim,
            text: " ".repeat(left_width - used),
        });
    }
    trimmed.push(SbarFragment {
        style: SbarStyle::Dim,
        text: " ─".to_owned(),
    });
    trimmed.push(SbarFragment {
        style: SbarStyle::SessionTitle,
        text: badge,
    });
    trimmed
}

// ---------------------------------------------------------------------------
// ANSI rendering (depth-aware)
// ---------------------------------------------------------------------------

/// SGR prefix for a style class at a color depth (fg + bold + navy bg; the
/// session-title badge inverts to gold bg). Returns `""` on Basic depth.
fn sgr(style: SbarStyle, depth: ColorDepth) -> String {
    if depth == ColorDepth::Basic {
        return String::new();
    }
    let (fg, bold) = match style {
        SbarStyle::Bar => (Color::Rgb(192, 192, 192), false),
        SbarStyle::Strong | SbarStyle::Warn => (Color::Rgb(255, 215, 0), true),
        SbarStyle::Dim => (Color::Rgb(139, 134, 130), false),
        SbarStyle::Good => (Color::Rgb(143, 188, 143), true),
        SbarStyle::Bad => (Color::Rgb(255, 140, 0), true),
        SbarStyle::Critical => (Color::Rgb(255, 107, 107), true),
        SbarStyle::Yolo => (Color::Rgb(255, 68, 68), true),
        SbarStyle::SessionTitle => (Color::Rgb(26, 26, 46), true),
    };
    let bg = match style {
        SbarStyle::SessionTitle => Color::Rgb(255, 215, 0),
        _ => Color::Rgb(26, 26, 46),
    };
    let mut parts: Vec<String> = Vec::new();
    if bold {
        parts.push("1".to_owned());
    }
    parts.push(sgr_color_code(fg, depth));
    parts.push(sgr_color_code(bg, depth).replacen("38", "48", 1));
    format!("\x1b[{}m", parts.join(";"))
}

/// SGR color code (fg form) for a color at a depth (truecolor or the
/// Ticket 02 256 approximation).
fn sgr_color_code(color: Color, depth: ColorDepth) -> String {
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

/// Render the full one-line status bar at `width` columns. The line is
/// always exactly `width` display cells (padded), so it never wraps; when
/// the fragments cannot fit, the whole bar degrades to trimmed plain text
/// (Python parity: single `status-bar` fragment).
pub fn render_line(width: usize, depth: ColorDepth, data: &StatusBarData) -> String {
    let width = width.max(1);
    let mut frags = build_fragments(width, data);
    frags = right_align_title(frags, &data.session_title, width);
    let total: usize = frags.iter().map(|f| f.text.width()).sum();
    let frags = if total > width {
        let plain: String = frags.iter().map(|f| f.text.as_str()).collect();
        vec![SbarFragment {
            style: SbarStyle::Bar,
            text: trim_text(&plain, width),
        }]
    } else {
        frags
    };
    let mut out = String::new();
    let mut used = 0usize;
    for f in &frags {
        let s = sgr(f.style, depth);
        if !s.is_empty() {
            out.push_str(&s);
        }
        out.push_str(&f.text);
        used += f.text.width();
    }
    let pad = width.saturating_sub(used);
    if pad > 0 {
        let s = sgr(SbarStyle::Bar, depth);
        if !s.is_empty() {
            out.push_str(&s);
        }
        out.push_str(&" ".repeat(pad));
    }
    if depth != ColorDepth::Basic {
        out.push_str("\x1b[0m");
    }
    out
}

/// [`render_line`] with the environment-detected color depth.
pub fn render_line_tty(width: usize, data: &StatusBarData) -> String {
    render_line(width, detect_color_depth(), data)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn data_with() -> StatusBarData {
        StatusBarData {
            model: "gpt-4o".to_owned(),
            context_tokens: 50_000,
            context_limit: Some(100_000),
            ..Default::default()
        }
    }

    fn texts(frags: &[SbarFragment]) -> Vec<String> {
        frags.iter().map(|f| f.text.clone()).collect()
    }

    fn total_width(frags: &[SbarFragment]) -> usize {
        frags.iter().map(|f| f.text.width()).sum()
    }

    /// Remove SGR escape sequences so display-width measurement ignores them.
    fn strip_ansi(text: &str) -> String {
        let bytes = text.as_bytes();
        let mut out = String::new();
        let mut i = 0usize;
        while i < bytes.len() {
            if bytes[i] == 0x1b {
                // skip `\x1b[...m`
                let mut j = i + 1;
                while j < bytes.len() && bytes[j] != b'm' {
                    j += 1;
                }
                i = j + 1;
            } else {
                let c = text[i..].chars().next().unwrap();
                out.push(c);
                i += c.len_utf8();
            }
        }
        out
    }

    // -- formatting helpers -------------------------------------------------

    #[test]
    fn py_round_matches_python_bankers() {
        assert_eq!(py_round(0.5), 0);
        assert_eq!(py_round(1.5), 2);
        assert_eq!(py_round(2.5), 2);
        assert_eq!(py_round(3.5), 4);
        assert_eq!(py_round(9.5), 10);
    }

    #[test]
    fn context_bar_fills_verbatim() {
        assert_eq!(context_bar(None, 10), "[░░░░░░░░░░]");
        assert_eq!(context_bar(Some(0), 10), "[░░░░░░░░░░]");
        assert_eq!(context_bar(Some(50), 10), "[█████░░░░░]");
        assert_eq!(
            context_bar(Some(25), 10),
            "[██░░░░░░░░]",
            "Python round(2.5) = 2"
        );
        assert_eq!(
            context_bar(Some(95), 10),
            "[██████████]",
            "Python round(9.5) = 10"
        );
        assert_eq!(context_bar(Some(100), 10), "[██████████]");
    }

    #[test]
    fn context_style_thresholds_verbatim() {
        assert_eq!(context_style(None), SbarStyle::Dim);
        assert_eq!(context_style(Some(49)), SbarStyle::Good);
        assert_eq!(context_style(Some(50)), SbarStyle::Warn);
        assert_eq!(context_style(Some(80)), SbarStyle::Warn);
        assert_eq!(context_style(Some(81)), SbarStyle::Bad);
        assert_eq!(context_style(Some(94)), SbarStyle::Bad);
        assert_eq!(context_style(Some(95)), SbarStyle::Critical);
        assert_eq!(context_style(Some(100)), SbarStyle::Critical);
    }

    #[test]
    fn duration_format_compact_verbatim() {
        assert_eq!(format_duration_compact(0.0), "0s");
        assert_eq!(format_duration_compact(42.0), "42s");
        assert_eq!(format_duration_compact(3600.0), "1h");
        assert_eq!(format_duration_compact(4500.0), "1h 15m");
        assert_eq!(format_duration_compact(86_399.0), "23h 59m");
        assert_eq!(format_duration_compact(93_600.0), "1.1d");
        assert_eq!(format_duration_compact(172_800.0), "2.0d");
    }

    #[test]
    fn token_count_compact_verbatim() {
        assert_eq!(format_token_count_compact(999), "999");
        assert_eq!(format_token_count_compact(1_000), "1K");
        assert_eq!(format_token_count_compact(1_234), "1.23K");
        assert_eq!(format_token_count_compact(12_000), "12K");
        assert_eq!(format_token_count_compact(1_500_000), "1.5M");
        assert_eq!(format_token_count_compact(12_000_000), "12M");
        assert_eq!(format_token_count_compact(123_000_000), "123M");
        assert_eq!(format_token_count_compact(1_500_000_000), "1.5B");
    }

    #[test]
    fn context_length_format_verbatim() {
        assert_eq!(format_context_length(128_000), "128K");
        assert_eq!(format_context_length(1_048_576), "1M");
        assert_eq!(format_context_length(2_097_152), "2.1M");
        assert_eq!(format_context_length(1_536_000), "1.5M");
        assert_eq!(format_context_length(500_000), "500K");
        assert_eq!(format_context_length(999), "999");
    }

    #[test]
    fn model_short_rules() {
        assert_eq!(model_short("gpt-4o"), "gpt-4o");
        assert_eq!(model_short("openai/gpt-4o"), "gpt-4o");
        assert_eq!(model_short("local/model.gguf"), "model");
        let long = "a".repeat(30);
        assert_eq!(model_short(&long), format!("{}...", "a".repeat(23)));
        assert_eq!(model_short(&"b".repeat(26)), "b".repeat(26));
        assert_eq!(
            model_short(&"c".repeat(27)),
            format!("{}...", "c".repeat(23))
        );
    }

    // -- fragment tiers -----------------------------------------------------

    #[test]
    fn compact_tier_under_52() {
        let mut d = data_with();
        d.goal_active = true;
        d.yolo = true;
        let frags = build_fragments(50, &d);
        let t = texts(&frags);
        assert_eq!(
            t,
            vec![" ⚕ ", "gpt-4o", " · ", "0s", " · ", "⊙ goal", " · ", "⚠ YOLO", " "]
        );
        // no context % in the compact tier
        assert!(!t.iter().any(|x| x.contains('%')));
    }

    #[test]
    fn medium_tier_52_to_75() {
        let d = data_with(); // 50% → warn
        let frags = build_fragments(60, &d);
        let t = texts(&frags);
        assert_eq!(t[0], " ⚕ ");
        assert_eq!(t[1], "gpt-4o");
        assert_eq!(t[2], " · ");
        assert_eq!(t[3], "50%");
        assert_eq!(frags[3].style, SbarStyle::Warn);
        assert_eq!(*t.last().unwrap(), " ", "trailing pad");
        // separator is " · ", never " │ " below 76
        assert!(!t.iter().any(|x| x.contains('│')));
    }

    #[test]
    fn full_tier_76_and_above() {
        let mut d = data_with(); // 50%
        d.compressions = 2;
        d.bg_tasks = 1;
        d.bg_processes = 3;
        d.bg_subagents = 4;
        d.goal_active = true;
        d.goal_turns_used = 3;
        d.goal_max_turns = 20;
        let frags = build_fragments(100, &d);
        let t = texts(&frags);
        // context detail + bar + pct with " │ " separators
        assert!(t.contains(&"50K/100K".to_owned()));
        assert!(t.contains(&"[█████░░░░░]".to_owned()));
        assert!(t.contains(&"🗜️ 2".to_owned()));
        assert!(t.contains(&"▶ 1".to_owned()));
        assert!(t.contains(&"⚙ 3".to_owned()));
        assert!(t.contains(&"⛓ 4".to_owned()));
        assert!(t.contains(&"⊙ goal 3/20".to_owned()));
        assert!(t.iter().any(|x| x == " │ "));
        // badges come after the duration, before the trailing pad
        assert_eq!(*t.last().unwrap(), " ");
        let goal_pos = t.iter().position(|x| x.contains("⊙ goal")).unwrap();
        let dur_pos = t.iter().position(|x| x == "0s").unwrap();
        assert!(goal_pos < dur_pos, "goal precedes duration (spec order)");
    }

    #[test]
    fn zero_counts_hide_badges() {
        let d = data_with();
        let t = texts(&build_fragments(100, &d));
        assert!(!t.iter().any(|x| x.contains('🗜')));
        assert!(!t.iter().any(|x| x.contains('▶')));
        assert!(!t.iter().any(|x| x.contains('⚙')));
        assert!(!t.iter().any(|x| x.contains('⛓')));
        assert!(!t.iter().any(|x| x.contains("⊙ goal")));
    }

    #[test]
    fn no_limit_shows_dash_dash_dim() {
        let mut d = data_with();
        d.context_limit = None;
        let frags = build_fragments(60, &d);
        let pct = frags
            .iter()
            .find(|f| f.text == "--")
            .expect("-- label present");
        assert_eq!(pct.style, SbarStyle::Dim);
        let full = build_fragments(100, &d);
        assert!(full.iter().any(|f| f.text == "[░░░░░░░░░░]"));
    }

    #[test]
    fn goal_segment_variants() {
        let mut d = StatusBarData::default();
        assert_eq!(goal_segment(&d), None);
        d.goal_active = true;
        assert_eq!(goal_segment(&d), Some("⊙ goal".to_owned()));
        d.goal_turns_used = 3;
        d.goal_max_turns = 20;
        assert_eq!(goal_segment(&d), Some("⊙ goal 3/20".to_owned()));
    }

    // -- trim + title -------------------------------------------------------

    #[test]
    fn trim_text_display_width_aware() {
        assert_eq!(trim_text("hello", 10), "hello");
        assert_eq!(trim_text("hello", 5), "hello");
        assert_eq!(trim_text("hello world", 8), "hello...");
        assert_eq!(trim_text("hello", 3), "...");
        assert_eq!(trim_text("hello", 2), "..");
        // emoji width: 🗜️ is 2 cells (VS16 is zero-width)
        assert_eq!(trim_text("a🗜️b", 4), "a🗜️b");
        assert_eq!(trim_text("a🗜️bcd", 5), "a🗜️...");
    }

    #[test]
    fn title_badge_right_aligned() {
        let mut d = data_with();
        d.session_title = "My Session".to_owned();
        let frags = build_fragments(100, &d);
        let frags = right_align_title(frags, &d.session_title, 100);
        let last = frags.last().unwrap();
        assert_eq!(last.style, SbarStyle::SessionTitle);
        assert_eq!(last.text, " My Session ");
        assert_eq!(frags[frags.len() - 2].text, " ─");
        // the whole bar fills the width exactly (padded left content)
        assert_eq!(total_width(&frags), 100);
    }

    #[test]
    fn title_suppressed_below_24_cols() {
        let mut d = data_with();
        d.session_title = "T".to_owned();
        let frags = build_fragments(20, &d);
        let frags = right_align_title(frags, "T", 20);
        assert!(frags.iter().all(|f| f.style != SbarStyle::SessionTitle));
    }

    #[test]
    fn overflow_degrades_to_trimmed_plain() {
        let mut d = data_with();
        d.compressions = 2;
        d.bg_tasks = 1;
        d.bg_processes = 3;
        d.bg_subagents = 4;
        d.goal_active = true;
        d.yolo = true;
        let line = render_line(30, ColorDepth::Basic, &d);
        assert_eq!(line.width(), 30, "line stays exactly one row");
        assert!(line.ends_with("..."), "trimmed with ellipsis: {line:?}");
    }

    // -- ANSI rendering ------------------------------------------------------

    #[test]
    fn render_truecolor_verbatim_sgr() {
        let d = data_with();
        let line = render_line(60, ColorDepth::Truecolor, &d);
        assert!(
            line.contains("\x1b[1;38;2;255;215;0;48;2;26;26;46mgpt-4o"),
            "strong model on navy: {line:?}"
        );
        assert!(
            line.contains("\x1b[1;38;2;255;215;0;48;2;26;26;46m50%"),
            "warn gauge"
        );
        assert!(line.starts_with("\x1b[38;2;192;192;192;48;2;26;26;46m ⚕ "));
        assert!(line.ends_with("\x1b[0m"));
        assert_eq!(strip_ansi(&line).width(), 60, "full-width line, no wrap");
        assert!(!line.contains('\n'));
    }

    #[test]
    fn render_256_uses_indexed_codes() {
        let d = data_with();
        let line = render_line(60, ColorDepth::Color256, &d);
        assert!(!line.contains("38;2;"), "no truecolor codes");
        assert!(line.contains("38;5;"), "indexed fg");
        assert!(line.contains("48;5;"), "indexed bg");
        assert_eq!(strip_ansi(&line).width(), 60);
    }

    #[test]
    fn render_basic_has_no_ansi() {
        let d = data_with();
        let line = render_line(60, ColorDepth::Basic, &d);
        assert!(!line.contains('\x1b'));
        assert_eq!(line.width(), 60);
        assert!(line.starts_with(" ⚕ gpt-4o"));
    }

    #[test]
    fn session_title_badge_ansi() {
        let mut d = data_with();
        d.session_title = "Mine".to_owned();
        let line = render_line(80, ColorDepth::Truecolor, &d);
        // gold bg, navy fg, bold
        assert!(
            line.contains("\x1b[1;38;2;26;26;46;48;2;255;215;0m Mine "),
            "badge: {line:?}"
        );
        assert_eq!(strip_ansi(&line).width(), 80);
    }
}
