//! Spec 013 — Hermes Python UI parity, Ticket 03: hardcoded art constants.
//!
//! `HERMES_AGENT_LOGO` (6-line figlet, 3 color tiers) and `HERMES_CADUCEUS`
//! (braille hero, gradient) are lifted **verbatim, byte-for-byte** from the
//! Python Hermes source `hermes_cli/banner.py` (the canonical truth from the
//! Ticket 01 archaeology). This file is generated from that source by
//! `gen_art.py` (run on the Hermes VM) — do not hand-edit the art text;
//! regenerate instead. The generated tests below pin the exact bytes.
//!
//! Color tiers (verbatim from the Python markup):
//! * logo: lines 1-2 `bold #FFD700`, lines 3-4 `#FFBF00`, lines 5-6 `#CD7F32`.
//! * caduceus: bronze `#CD7F32` → accent `#FFBF00` → gold `#FFD700` →
//!   accent → bronze → dim `#B8860B` (head-to-base gradient).

use ratatui::style::{Color, Modifier, Style};

pub const LOGO_BRONZE: Color = Color::Rgb(205, 127, 50);
pub const LOGO_ACCENT: Color = Color::Rgb(255, 191, 0);
pub const LOGO_GOLD: Color = Color::Rgb(255, 215, 0);
pub const CADUCEUS_DIM: Color = Color::Rgb(184, 134, 11);
pub const CADUCEUS_BRONZE: Color = Color::Rgb(205, 127, 50);
pub const CADUCEUS_ACCENT: Color = Color::Rgb(255, 191, 0);
pub const CADUCEUS_GOLD: Color = Color::Rgb(255, 215, 0);

/// One art row: the verbatim text plus its skin color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtLine {
    pub style: Style,
    pub text: &'static str,
}

/// Row count of `HERMES_AGENT_LOGO` (figlet two-line: 3 rows per word tier).
pub const LOGO_LINES: usize = 6;
/// Max display width (cells) of a logo row. The source rows are ragged
/// (trailing spaces lost in the original authoring) — per-row widths are
/// pinned by the generated test.
pub const LOGO_WIDTH: usize = 101;
/// Row count of `HERMES_CADUCEUS`.
pub const CADUCEUS_LINES: usize = 15;
/// Max display width (cells) of a caduceus row.
pub const CADUCEUS_WIDTH: usize = 30;

/// `HERMES_AGENT_LOGO` — the 6-line ASCII `HERMES-AGENT` figlet, one entry per
/// row, top to bottom, with its verbatim color tier.
pub fn logo_lines() -> [ArtLine; 6] {
    [
        ArtLine { style: Style::default().fg(LOGO_GOLD).add_modifier(Modifier::BOLD), text: "██╗  ██╗███████╗██████╗ ███╗   ███╗███████╗███████╗       █████╗  ██████╗ ███████╗███╗   ██╗████████╗" },

        ArtLine { style: Style::default().fg(LOGO_GOLD).add_modifier(Modifier::BOLD), text: "██║  ██║██╔════╝██╔══██╗████╗ ████║██╔════╝██╔════╝      ██╔══██╗██╔════╝ ██╔════╝████╗  ██║╚══██╔══╝" },

        ArtLine { style: Style::default().fg(LOGO_ACCENT), text: "███████║█████╗  ██████╔╝██╔████╔██║█████╗  ███████╗█████╗███████║██║  ███╗█████╗  ██╔██╗ ██║   ██║" },

        ArtLine { style: Style::default().fg(LOGO_ACCENT), text: "██╔══██║██╔══╝  ██╔══██╗██║╚██╔╝██║██╔══╝  ╚════██║╚════╝██╔══██║██║   ██║██╔══╝  ██║╚██╗██║   ██║" },

        ArtLine { style: Style::default().fg(LOGO_BRONZE), text: "██║  ██║███████╗██║  ██║██║ ╚═╝ ██║███████╗███████║      ██║  ██║╚██████╔╝███████╗██║ ╚████║   ██║" },

        ArtLine { style: Style::default().fg(LOGO_BRONZE), text: "╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝     ╚═╝╚══════╝╚══════╝      ╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚═╝  ╚═══╝   ╚═╝" },
    ]
}

/// `HERMES_CADUCEUS` — the braille hero art from the welcome banner's left
/// column, one entry per row, top to bottom, with its verbatim gradient color.
pub fn caduceus_lines() -> [ArtLine; 15] {
    [
        ArtLine {
            style: Style::default().fg(CADUCEUS_BRONZE),
            text: "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⡀⠀⣀⣀⠀⢀⣀⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        },
        ArtLine {
            style: Style::default().fg(CADUCEUS_BRONZE),
            text: "⠀⠀⠀⠀⠀⠀⢀⣠⣴⣾⣿⣿⣇⠸⣿⣿⠇⣸⣿⣿⣷⣦⣄⡀⠀⠀⠀⠀⠀⠀",
        },
        ArtLine {
            style: Style::default().fg(CADUCEUS_ACCENT),
            text: "⠀⢀⣠⣴⣶⠿⠋⣩⡿⣿⡿⠻⣿⡇⢠⡄⢸⣿⠟⢿⣿⢿⣍⠙⠿⣶⣦⣄⡀⠀",
        },
        ArtLine {
            style: Style::default().fg(CADUCEUS_ACCENT),
            text: "⠀⠀⠉⠉⠁⠶⠟⠋⠀⠉⠀⢀⣈⣁⡈⢁⣈⣁⡀⠀⠉⠀⠙⠻⠶⠈⠉⠉⠀⠀",
        },
        ArtLine {
            style: Style::default().fg(CADUCEUS_GOLD),
            text: "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣴⣿⡿⠛⢁⡈⠛⢿⣿⣦⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        },
        ArtLine {
            style: Style::default().fg(CADUCEUS_GOLD),
            text: "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠿⣿⣦⣤⣈⠁⢠⣴⣿⠿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        },
        ArtLine {
            style: Style::default().fg(CADUCEUS_ACCENT),
            text: "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠉⠻⢿⣿⣦⡉⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        },
        ArtLine {
            style: Style::default().fg(CADUCEUS_ACCENT),
            text: "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠘⢷⣦⣈⠛⠃⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        },
        ArtLine {
            style: Style::default().fg(CADUCEUS_BRONZE),
            text: "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢠⣴⠦⠈⠙⠿⣦⡄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        },
        ArtLine {
            style: Style::default().fg(CADUCEUS_BRONZE),
            text: "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠸⣿⣤⡈⠁⢤⣿⠇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        },
        ArtLine {
            style: Style::default().fg(CADUCEUS_DIM),
            text: "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠛⠷⠄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        },
        ArtLine {
            style: Style::default().fg(CADUCEUS_DIM),
            text: "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⠑⢶⣄⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        },
        ArtLine {
            style: Style::default().fg(CADUCEUS_DIM),
            text: "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣿⠁⢰⡆⠈⡿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        },
        ArtLine {
            style: Style::default().fg(CADUCEUS_DIM),
            text: "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠳⠈⣡⠞⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        },
        ArtLine {
            style: Style::default().fg(CADUCEUS_DIM),
            text: "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Byte-for-byte pin of the logo rows (generated from banner.py).
    const EXPECTED_LOGO: [&str; 6] = [
        "██╗  ██╗███████╗██████╗ ███╗   ███╗███████╗███████╗       █████╗  ██████╗ ███████╗███╗   ██╗████████╗",
        "██║  ██║██╔════╝██╔══██╗████╗ ████║██╔════╝██╔════╝      ██╔══██╗██╔════╝ ██╔════╝████╗  ██║╚══██╔══╝",
        "███████║█████╗  ██████╔╝██╔████╔██║█████╗  ███████╗█████╗███████║██║  ███╗█████╗  ██╔██╗ ██║   ██║",
        "██╔══██║██╔══╝  ██╔══██╗██║╚██╔╝██║██╔══╝  ╚════██║╚════╝██╔══██║██║   ██║██╔══╝  ██║╚██╗██║   ██║",
        "██║  ██║███████╗██║  ██║██║ ╚═╝ ██║███████╗███████║      ██║  ██║╚██████╔╝███████╗██║ ╚████║   ██║",
        "╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝     ╚═╝╚══════╝╚══════╝      ╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚═╝  ╚═══╝   ╚═╝",
    ];

    /// Byte-for-byte pin of the caduceus rows (generated from banner.py).
    const EXPECTED_CADUCEUS: [&str; 15] = [
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⡀⠀⣀⣀⠀⢀⣀⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        "⠀⠀⠀⠀⠀⠀⢀⣠⣴⣾⣿⣿⣇⠸⣿⣿⠇⣸⣿⣿⣷⣦⣄⡀⠀⠀⠀⠀⠀⠀",
        "⠀⢀⣠⣴⣶⠿⠋⣩⡿⣿⡿⠻⣿⡇⢠⡄⢸⣿⠟⢿⣿⢿⣍⠙⠿⣶⣦⣄⡀⠀",
        "⠀⠀⠉⠉⠁⠶⠟⠋⠀⠉⠀⢀⣈⣁⡈⢁⣈⣁⡀⠀⠉⠀⠙⠻⠶⠈⠉⠉⠀⠀",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣴⣿⡿⠛⢁⡈⠛⢿⣿⣦⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠿⣿⣦⣤⣈⠁⢠⣴⣿⠿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠉⠻⢿⣿⣦⡉⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠘⢷⣦⣈⠛⠃⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢠⣴⠦⠈⠙⠿⣦⡄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠸⣿⣤⡈⠁⢤⣿⠇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠛⠷⠄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⠑⢶⣄⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣿⠁⢰⡆⠈⡿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠳⠈⣡⠞⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
    ];

    /// Per-row char counts, pinned from banner.py (the source rows are ragged:
    /// trailing spaces were lost in the original authoring — verbatim parity
    /// keeps them ragged).
    const LOGO_ROW_CHARS: [usize; 6] = [101, 101, 98, 98, 98, 98];

    #[test]
    fn logo_is_verbatim_bytes() {
        let rows = logo_lines();
        assert_eq!(rows.len(), 6);
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(
                row.text, EXPECTED_LOGO[i],
                "logo row {i} diverged from banner.py"
            );
            assert_eq!(
                row.text.chars().count(),
                LOGO_ROW_CHARS[i],
                "logo row {i} char count"
            );
        }
    }

    #[test]
    fn caduceus_is_verbatim_bytes() {
        let rows = caduceus_lines();
        assert_eq!(rows.len(), 15);
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(
                row.text, EXPECTED_CADUCEUS[i],
                "caduceus row {i} diverged from banner.py"
            );
        }
    }

    #[test]
    fn caduceus_row_byte_lengths_are_pinned() {
        let rows = caduceus_lines();
        let expected: [usize; 15] = [90, 90, 90, 90, 90, 90, 90, 90, 90, 90, 90, 90, 90, 90, 90];
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.text.len(), expected[i], "caduceus row {i} byte length");
        }
    }

    #[test]
    fn caduceus_gradient_color_sequence_is_verbatim() {
        let rows = caduceus_lines();
        let expected_fgs: [Color; 15] = [
            Color::Rgb(205, 127, 50),
            Color::Rgb(205, 127, 50),
            Color::Rgb(255, 191, 0),
            Color::Rgb(255, 191, 0),
            Color::Rgb(255, 215, 0),
            Color::Rgb(255, 215, 0),
            Color::Rgb(255, 191, 0),
            Color::Rgb(255, 191, 0),
            Color::Rgb(205, 127, 50),
            Color::Rgb(205, 127, 50),
            Color::Rgb(184, 134, 11),
            Color::Rgb(184, 134, 11),
            Color::Rgb(184, 134, 11),
            Color::Rgb(184, 134, 11),
            Color::Rgb(184, 134, 11),
        ];
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.style.fg, Some(expected_fgs[i]), "caduceus row {i} fg");
            assert!(
                !row.style.add_modifier.contains(Modifier::BOLD),
                "caduceus rows carry no bold tier"
            );
        }
    }
}
