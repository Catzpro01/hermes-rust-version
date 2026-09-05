//! Spec 013 — Hermes Python UI parity, Ticket 02: Color palette & theme system.
//!
//! This module is the single source of truth for the Hermes "gold & kawaii"
//! visual identity inside the Hermes-RS TUI. Every value is lifted **verbatim**
//! from `docs/HERMES_UI_SPEC.md` (itself excavated from the real Python Hermes
//! skin `default` / prompt_toolkit style dict), not guessed.
//!
//! Design decisions locked by Matt (Ticket 01 review):
//! * **Dark-canonical only.** Light mode (`light_colors`) is deliberately out of
//!   scope and deferred to Spec 013b.
//! * Accent hex = `#FFBF00` (`banner_accent`); `#FFD700` is reserved for
//!   `banner_title` and `response_border`.
//! * Reasoning = dim + italic (`_DIM`), not the unrealized `ui_thinking`.
//! * Palate + branding + visual hierarchy only — no per-widget replication.
//!
//! Scope (Ticket 02): this module is **helpers only**. Nothing is wired into
//! the panels yet; panels consume it from Ticket 03 onward.

#![allow(dead_code)]
// Ticket 02 scope is a standalone, unwired palette/theme module. Every helper
// and field below is intentionally un-referenced until Tickets 03-05 wire it
// into the panels, so `dead_code` is allowed here (mirroring the precedent
// already used in `worker.rs`).

use ratatui::style::{Color, Modifier, Style};

// ---------------------------------------------------------------------------
// hex → Color
// ---------------------------------------------------------------------------

/// Parse a 6-digit hex color string (`#RRGGBB` or `RRGGBB`) into a
/// [`Color::Rgb`]. Leading `#` is optional. Non-hex or wrong-length input
/// returns `None` so callers can degrade gracefully instead of panicking.
pub fn hex_to_rgb(hex: &str) -> Option<Color> {
    let s = hex.strip_prefix('#').unwrap_or(hex);
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

// ---------------------------------------------------------------------------
// HermesTheme
// ---------------------------------------------------------------------------

/// The complete Hermes `default` (dark canonical) palette.
///
/// Field names mirror the Python skin keys from `HERMES_UI_SPEC.md` §2.1 so a
/// human can cross-check a color against the spec by name alone. The 256-color
/// TUI chrome colors from §8 (placeholder, image badge, menu meta, clarify /
/// sudo / approval / voice) are represented by `Style` helper methods rather
/// than duplicate palette fields, because they are plain-color roles, not
/// palette slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HermesTheme {
    /// `banner_border` — panel border of the welcome banner (bronze).
    pub banner_border: Color,
    /// `banner_title` — panel title / version label (gold, bold).
    pub banner_title: Color,
    /// `banner_accent` — section headers (Available Tools / Skills …).
    pub banner_accent: Color,
    /// `banner_dim` — dim / muted text (separators `·`, labels, cwd).
    pub banner_dim: Color,
    /// `banner_text` — body text (tool / skill names).
    pub banner_text: Color,
    /// `ui_accent` — generic accent.
    pub ui_accent: Color,
    /// `ui_label` — generic label.
    pub ui_label: Color,
    /// `ui_ok` — success status.
    pub ui_ok: Color,
    /// `ui_error` — errors.
    pub ui_error: Color,
    /// `ui_warn` — warnings.
    pub ui_warn: Color,
    /// `prompt` — prompt text.
    pub prompt: Color,
    /// `input_rule` — horizontal `─` rules above/below the composer.
    pub input_rule: Color,
    /// `response_border` — response box frame (gold, bold).
    pub response_border: Color,
    /// `status_bar_bg` — navy status-bar background.
    pub status_bar_bg: Color,
    /// `status_bar_text` — status-bar default foreground.
    pub status_bar_text: Color,
    /// `status_bar_strong` — strong segments (model name), bold.
    pub status_bar_strong: Color,
    /// `status_bar_dim` — status-bar separators (` · `).
    pub status_bar_dim: Color,
    /// `status_bar_good` — healthy state, bold.
    pub status_bar_good: Color,
    /// `status_bar_warn` — warning state, bold.
    pub status_bar_warn: Color,
    /// `status_bar_bad` — bad / near-limit state, bold.
    pub status_bar_bad: Color,
    /// `status_bar_critical` — critical state, bold.
    pub status_bar_critical: Color,
    /// `status-bar-yolo` (TUI style) — YOLO mode, bold.
    pub status_bar_yolo: Color,
    /// `session_label` — session label.
    pub session_label: Color,
    /// `session_border` — dim session id.
    pub session_border: Color,
    /// `completion_menu_bg` — autocomplete background (navy).
    pub completion_menu_bg: Color,
    /// `completion_menu_current_bg` — active autocomplete item background.
    pub completion_menu_current_bg: Color,
    /// `selection_bg` — generic selection background.
    pub selection_bg: Color,
    /// `shell_dollar` — shell/dollar prompt color.
    pub shell_dollar: Color,
    /// `voice_status_bg` — voice bar background.
    pub voice_status_bg: Color,
    /// Placeholder / hint foreground (`#888888`, italic) — TUI §8.
    pub placeholder: Color,
    /// Image attachment badge (`#87CEEB`, bold) — TUI §8.
    pub image_badge: Color,
}

impl HermesTheme {
    /// The single theme used by Spec 013: dark-canonical Hermes `default`.
    /// No `light_colors` overlay is applied (deferred to Spec 013b).
    pub fn dark_canonical() -> Self {
        fn c(hex: &str) -> Color {
            hex_to_rgb(hex).expect("HERMES_UI_SPEC palette hex must be valid")
        }
        Self {
            banner_border: c("#CD7F32"),
            banner_title: c("#FFD700"),
            banner_accent: c("#FFBF00"),
            banner_dim: c("#B8860B"),
            banner_text: c("#FFF8DC"),
            ui_accent: c("#FFBF00"),
            ui_label: c("#DAA520"),
            ui_ok: c("#4caf50"),
            ui_error: c("#ef5350"),
            ui_warn: c("#ffa726"),
            prompt: c("#FFF8DC"),
            input_rule: c("#CD7F32"),
            response_border: c("#FFD700"),
            status_bar_bg: c("#1a1a2e"),
            status_bar_text: c("#C0C0C0"),
            status_bar_strong: c("#FFD700"),
            status_bar_dim: c("#8A7A4A"),
            status_bar_good: c("#8FBC8F"),
            status_bar_warn: c("#FFD700"),
            status_bar_bad: c("#FF8C00"),
            status_bar_critical: c("#FF6B6B"),
            status_bar_yolo: c("#FF4444"),
            session_label: c("#DAA520"),
            session_border: c("#8B8682"),
            completion_menu_bg: c("#1a1a2e"),
            completion_menu_current_bg: c("#333355"),
            selection_bg: c("#3a3a55"),
            shell_dollar: c("#4dabf7"),
            voice_status_bg: c("#1a1a2e"),
            placeholder: c("#888888"),
            image_badge: c("#87CEEB"),
        }
    }
}

// ---------------------------------------------------------------------------
// Style helpers — mapped from the prompt_toolkit TUI style dict (§8).
// ---------------------------------------------------------------------------

impl HermesTheme {
    /// Banner section-header style (`banner_accent`, bold).
    pub fn banner_section(&self) -> Style {
        Style::default()
            .fg(self.banner_accent)
            .add_modifier(Modifier::BOLD)
    }
    /// Banner title / version label (`banner_title`, bold).
    pub fn banner_title(&self) -> Style {
        Style::default()
            .fg(self.banner_title)
            .add_modifier(Modifier::BOLD)
    }
    /// Banner panel border (`banner_border`).
    pub fn banner_border(&self) -> Style {
        Style::default().fg(self.banner_border)
    }
    /// Banner dim / muted text (`banner_dim`).
    pub fn banner_dim(&self) -> Style {
        Style::default().fg(self.banner_dim)
    }
    /// Banner body text (`banner_text`).
    pub fn banner_text(&self) -> Style {
        Style::default().fg(self.banner_text)
    }

    /// `response_border` — response box frame (`#FFD700` bold).
    pub fn response_border(&self) -> Style {
        Style::default()
            .fg(self.response_border)
            .add_modifier(Modifier::BOLD)
    }
    /// Reasoning dim style — `_DIM` = dim + italic (Matt decision B).
    pub fn reasoning_dim(&self) -> Style {
        Style::default().add_modifier(Modifier::DIM | Modifier::ITALIC)
    }
    /// Streamed response body text — Python streams in `banner_text #FFF8DC`.
    pub fn response_text(&self) -> Style {
        Style::default().fg(self.banner_text)
    }
    /// User bullet prompt accent (`ui_accent`, bold).
    pub fn user_bullet(&self) -> Style {
        Style::default()
            .fg(self.ui_accent)
            .add_modifier(Modifier::BOLD)
    }
    /// Generic error style (`ui_error`).
    pub fn error(&self) -> Style {
        Style::default().fg(self.ui_error)
    }
    /// Generic warning style (`ui_warn`).
    pub fn warn(&self) -> Style {
        Style::default().fg(self.ui_warn)
    }
    /// Generic success style (`ui_ok`).
    pub fn ok(&self) -> Style {
        Style::default().fg(self.ui_ok)
    }

    /// `input-rule` — the `─` rules above/below the composer.
    pub fn input_rule(&self) -> Style {
        Style::default().fg(self.input_rule)
    }
    /// Composer prompt symbol / typed text. Python leaves this un-styled so it
    /// inherits the terminal foreground; we expose it explicitly for callers
    /// that need it, defaulting to the skin `prompt` color.
    pub fn input_text(&self) -> Style {
        Style::default().fg(self.prompt)
    }
    /// Placeholder / hint text (`#888888` italic).
    pub fn placeholder(&self) -> Style {
        Style::default()
            .fg(self.placeholder)
            .add_modifier(Modifier::ITALIC)
    }
    /// `image-badge` — attached-image badge (`#87CEEB` bold).
    pub fn image_badge(&self) -> Style {
        Style::default()
            .fg(self.image_badge)
            .add_modifier(Modifier::BOLD)
    }

    // --- status bar ---
    /// `status-bar` — default status-bar cell (navy bg, `#C0C0C0` fg).
    pub fn status_bar(&self) -> Style {
        Style::default().bg(self.status_bar_bg).fg(self.status_bar_text)
    }
    /// `status-bar-strong` — strong segment (gold bold).
    pub fn status_bar_strong(&self) -> Style {
        Style::default()
            .bg(self.status_bar_bg)
            .fg(self.status_bar_strong)
            .add_modifier(Modifier::BOLD)
    }
    /// `status-bar-dim` — separators.
    pub fn status_bar_dim(&self) -> Style {
        Style::default().bg(self.status_bar_bg).fg(self.status_bar_dim)
    }
    /// `status-bar-good` — healthy (bold).
    pub fn status_bar_good(&self) -> Style {
        Style::default()
            .bg(self.status_bar_bg)
            .fg(self.status_bar_good)
            .add_modifier(Modifier::BOLD)
    }
    /// `status-bar-warn` — warning (gold bold).
    pub fn status_bar_warn(&self) -> Style {
        Style::default()
            .bg(self.status_bar_bg)
            .fg(self.status_bar_warn)
            .add_modifier(Modifier::BOLD)
    }
    /// `status-bar-bad` — bad / near-limit (orange bold).
    pub fn status_bar_bad(&self) -> Style {
        Style::default()
            .bg(self.status_bar_bg)
            .fg(self.status_bar_bad)
            .add_modifier(Modifier::BOLD)
    }
    /// `status-bar-critical` — critical (bold).
    pub fn status_bar_critical(&self) -> Style {
        Style::default()
            .bg(self.status_bar_bg)
            .fg(self.status_bar_critical)
            .add_modifier(Modifier::BOLD)
    }
    /// `status-bar-yolo` — YOLO mode (`#FF4444` bold).
    pub fn status_bar_yolo(&self) -> Style {
        Style::default()
            .bg(self.status_bar_bg)
            .fg(self.status_bar_yolo)
            .add_modifier(Modifier::BOLD)
    }

    // --- completion menu ---
    /// `completion-menu` — autocomplete (navy bg, `#FFF8DC` fg).
    pub fn completion_menu(&self) -> Style {
        Style::default()
            .bg(self.completion_menu_bg)
            .fg(self.banner_text)
    }
    /// Active completion item (`bg:#333355 fg:#FFD700`).
    pub fn completion_menu_current(&self) -> Style {
        Style::default()
            .bg(self.completion_menu_current_bg)
            .fg(self.banner_title)
    }

    /// Modal helper style builders shared by clarify/sudo/approval panels.
    fn bordered(&self, title: Color) -> Style {
        Style::default()
            .fg(title)
            .add_modifier(Modifier::BOLD)
    }
    fn selected(&self) -> Style {
        Style::default()
            .fg(self.banner_title)
            .add_modifier(Modifier::BOLD)
    }
    fn choice(&self) -> Style {
        Style::default().fg(Color::Rgb(0xAA, 0xAA, 0xAA))
    }

    /// `clarify-title` (gold bold).
    pub fn clarify_title(&self) -> Style {
        self.bordered(self.banner_title)
    }
    /// `clarify-question` (`#FFF8DC` bold).
    pub fn clarify_question(&self) -> Style {
        Style::default()
            .fg(self.banner_text)
            .add_modifier(Modifier::BOLD)
    }
    /// `clarify-answer` (`#98FB98`).
    pub fn clarify_answer(&self) -> Style {
        Style::default().fg(Color::Rgb(0x98, 0xFB, 0x98))
    }
    /// `clarify-choice`.
    pub fn clarify_choice(&self) -> Style {
        self.choice()
    }
    /// `clarify-selected`.
    pub fn clarify_selected(&self) -> Style {
        self.selected()
    }
    /// `sudo-prompt` / `sudo-title` (`#FF6B6B` bold).
    pub fn sudo_title(&self) -> Style {
        Style::default()
            .fg(self.status_bar_critical)
            .add_modifier(Modifier::BOLD)
    }
    /// `approval-title` (`#FF8C00` bold).
    pub fn approval_title(&self) -> Style {
        Style::default()
            .fg(self.status_bar_bad)
            .add_modifier(Modifier::BOLD)
    }
    /// `approval-desc` (`#FFF8DC` bold).
    pub fn approval_desc(&self) -> Style {
        Style::default()
            .fg(self.banner_text)
            .add_modifier(Modifier::BOLD)
    }
    /// `approval-choice`.
    pub fn approval_choice(&self) -> Style {
        self.choice()
    }
    /// `approval-selected`.
    pub fn approval_selected(&self) -> Style {
        self.selected()
    }
}

// ---------------------------------------------------------------------------
// Terminal color-depth detection
// ---------------------------------------------------------------------------

/// How many colors the terminal advertises. Used only to decide whether to
/// warn + approximate at startup (a warning is emitted **once**, by the caller
/// in a later ticket — never on every render).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorDepth {
    /// 24-bit true color (24-bit `COLORTERM`), or unknown-but-modern default.
    Truecolor,
    /// 256-color mode — true-color values must be approximated once.
    Color256,
    /// Basic 16/8 color — degrade to the ANSI 16 palette.
    Basic,
}

/// Inspect `COLORTERM` (and `TERM`) to estimate the terminal color depth.
///
/// `COLORTERM=truecolor|24bit` → [`ColorDepth::Truecolor`]. `TERM` containing
/// `256color` → [`ColorDepth::Color256`]. Anything else falls back to
/// [`ColorDepth::Truecolor`] because most modern terminals (and ratatui's
/// own crossterm backend) negotiate 24-bit color on demand.
pub fn detect_color_depth() -> ColorDepth {
    detect_color_depth_for(std::env::var("COLORTERM").ok(), std::env::var("TERM").ok())
}

fn detect_color_depth_for(colorterm: Option<String>, term: Option<String>) -> ColorDepth {
    if let Some(ct) = colorterm {
        let ct = ct.to_ascii_lowercase();
        if ct.contains("truecolor") || ct.contains("24bit") {
            return ColorDepth::Truecolor;
        }
        if ct.contains("256") {
            return ColorDepth::Color256;
        }
    }
    if let Some(t) = term {
        let tl = t.to_ascii_lowercase();
        if tl.contains("256color") {
            return ColorDepth::Color256;
        }
        if t.eq_ignore_ascii_case("xterm") || tl.contains("linux") || tl.contains("ansi") {
            return ColorDepth::Basic;
        }
    }
    ColorDepth::Truecolor
}

/// Whether the environment advertises true color.
pub fn truecolor_supported() -> bool {
    detect_color_depth() == ColorDepth::Truecolor
}

// ---------------------------------------------------------------------------
// 256-color approximation
// ---------------------------------------------------------------------------

fn cube_index(v: u8) -> usize {
    if v < 48 {
        0
    } else if v < 115 {
        1
    } else {
        ((v as usize) - 35) / 40
    }
}

/// Approximate an RGB color with the closest 6×6×6 ANSI-256 cube entry,
/// returning a [`Color::Indexed`]. Grayscales are left to the caller to
/// special-case if desired; the cube path covers the Hermes palette well.
pub fn truecolor_to_256(color: Color) -> Color {
    let (r, g, b) = match color {
        Color::Rgb(r, g, b) => (r, g, b),
        other => return other,
    };
    // Pure grayscale ramp 232..=255 (24 gray steps: 8 + 10*i for i in 0..=23).
    let mx = r.max(g).max(b) as i16;
    let mn = r.min(g).min(b) as i16;
    if mx - mn < 8 {
        // Very low chroma → nearest grayscale entry. Step `i` is centered at
        // value 8 + 10*i, so invert: i = round((v - 8) / 10), clamped to 0..=23.
        let v = mx as f32;
        let i = (((v - 8.0) / 10.0).round() as i16).clamp(0, 23) as u8;
        return Color::Indexed(232 + i);
    }
    let ri = cube_index(r);
    let gi = cube_index(g);
    let bi = cube_index(b);
    Color::Indexed((16 + 36 * ri + 6 * gi + bi) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(color: Color) -> (u8, u8, u8) {
        match color {
            Color::Rgb(r, g, b) => (r, g, b),
            other => panic!("expected Rgb, got {other:?}"),
        }
    }

    #[test]
    fn hex_to_rgb_parses_hash_and_bare() {
        assert_eq!(hex_to_rgb("#FFD700"), Some(Color::Rgb(255, 215, 0)));
        assert_eq!(hex_to_rgb("FFD700"), Some(Color::Rgb(255, 215, 0)));
        assert_eq!(hex_to_rgb("#1a1a2e"), Some(Color::Rgb(0x1a, 0x1a, 0x2e)));
        assert_eq!(hex_to_rgb("#CD7F32"), Some(Color::Rgb(0xcd, 0x7f, 0x32)));
    }

    #[test]
    fn hex_to_rgb_rejects_bad_input() {
        assert_eq!(hex_to_rgb(""), None);
        assert_eq!(hex_to_rgb("#FFD"), None); // 3 digits
        assert_eq!(hex_to_rgb("#GGGGGG"), None);
        assert_eq!(hex_to_rgb("FFF"), None);
        assert_eq!(hex_to_rgb("#FFD7000"), None); // 7 digits
    }

    #[test]
    fn every_palette_color_is_verbatim() {
        let t = HermesTheme::dark_canonical();
        // §2.1 — spot-check the "gold & kawaii" hierarchy by name.
        assert_eq!(rgb(t.banner_title), (0xFF, 0xD7, 0x00)); // #FFD700
        assert_eq!(rgb(t.banner_accent), (0xFF, 0xBF, 0x00)); // #FFBF00
        assert_eq!(rgb(t.banner_dim), (0xB8, 0x86, 0x0B)); // #B8860B
        assert_eq!(rgb(t.banner_border), (0xCD, 0x7F, 0x32)); // #CD7F32
        assert_eq!(rgb(t.banner_text), (0xFF, 0xF8, 0xDC)); // #FFF8DC
        assert_eq!(rgb(t.response_border), (0xFF, 0xD7, 0x00)); // #FFD700
        assert_eq!(rgb(t.input_rule), (0xCD, 0x7F, 0x32)); // #CD7F32
        assert_eq!(rgb(t.status_bar_bg), (0x1a, 0x1a, 0x2e)); // #1a1a2e
        assert_eq!(rgb(t.status_bar_critical), (0xFF, 0x6B, 0x6B)); // #FF6B6B
        assert_eq!(rgb(t.session_border), (0x8B, 0x86, 0x82)); // #8B8682
        // #FFD700 is *not* the accent (Matt decision A: accent is #FFBF00).
        assert_ne!(rgb(t.banner_accent), rgb(t.banner_title));
    }

    #[test]
    fn style_helpers_carry_expected_colors_and_bold() {
        let t = HermesTheme::dark_canonical();
        assert_eq!(t.status_bar().bg, Some(t.status_bar_bg));
        assert_eq!(t.status_bar().fg, Some(t.status_bar_text));
        assert_eq!(t.status_bar_strong().fg, Some(t.status_bar_strong));
        assert!(t
            .status_bar_strong()
            .add_modifier
            .contains(Modifier::BOLD));
        assert_eq!(t.response_border().fg, Some(t.response_border));
        assert_eq!(t.input_rule().fg, Some(t.input_rule));
        assert!(t.response_border().add_modifier.contains(Modifier::BOLD));
        // Reasoning is dim+italic, never a hard-coded hue (Matt decision B).
        let r = t.reasoning_dim();
        assert!(r.add_modifier.contains(Modifier::DIM));
        assert!(r.add_modifier.contains(Modifier::ITALIC));
        assert_eq!(r.fg, None);
    }

    #[test]
    fn status_bar_variants_are_distinct() {
        let t = HermesTheme::dark_canonical();
        let sb = t.status_bar();
        let strong = t.status_bar_strong();
        let good = t.status_bar_good();
        let bad = t.status_bar_bad();
        let crit = t.status_bar_critical();
        // Background is constant navy for all variants.
        assert_eq!(sb.bg, strong.bg);
        assert_eq!(sb.bg, good.bg);
        assert_eq!(sb.bg, bad.bg);
        assert_eq!(sb.bg, crit.bg);
        // Foregrounds are role-distinct.
        assert_ne!(good.fg, bad.fg);
        assert_ne!(bad.fg, crit.fg);
        // Strong/warn both use gold but strong is the documented name.
        assert_eq!(strong.fg, Some(t.status_bar_strong));
    }

    #[test]
    fn color_depth_detection_reads_env() {
        assert_eq!(
            detect_color_depth_for(Some("truecolor".into()), Some("xterm-256color".into())),
            ColorDepth::Truecolor
        );
        assert_eq!(
            detect_color_depth_for(Some("24bit".into()), None),
            ColorDepth::Truecolor
        );
        assert_eq!(
            detect_color_depth_for(None, Some("xterm-256color".into())),
            ColorDepth::Color256
        );
        assert_eq!(
            detect_color_depth_for(Some("16".into()), Some("linux".into())),
            ColorDepth::Basic
        );
        assert_eq!(
            detect_color_depth_for(None, Some("xterm-kitty".into())),
            ColorDepth::Truecolor
        );
    }

    #[test]
    fn truecolor_to_256_maps_known_entries() {
        assert_eq!(truecolor_to_256(Color::Rgb(0, 0, 0)), Color::Indexed(232));
        assert_eq!(truecolor_to_256(Color::Rgb(255, 255, 255)), Color::Indexed(255));
        assert_eq!(truecolor_to_256(Color::Rgb(255, 0, 0)), Color::Indexed(196));
        assert_eq!(truecolor_to_256(Color::Rgb(0, 255, 0)), Color::Indexed(46));
        assert_eq!(truecolor_to_256(Color::Rgb(0, 0, 255)), Color::Indexed(21));
        // Non-RGB passes through untouched.
        assert_eq!(truecolor_to_256(Color::Reset), Color::Reset);
    }
}
