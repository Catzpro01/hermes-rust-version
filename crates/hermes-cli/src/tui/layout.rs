//! Spec 012 — pure, unit-testable panel layout (Ticket 02).
//!
//! [`Panels`] carves the terminal [`Rect`] into the four dashboard regions:
//! a top status/header bar, a central area split into transcript + tool log,
//! and a bottom input line. All math is deliberately **manual and saturating**
//! so the function is total: it never overflows the parent area and never
//! panics, even on absurdly small or large terminal sizes. This keeps it safe
//! to unit-test across narrow/wide geometries and to drive a live `--tui` on a
//! resized window.

use ratatui::layout::Rect;

/// Fixed reserved height for the top status bar (title + border).
pub const HEADER_HEIGHT: u16 = 3;
/// Fixed reserved height for the bottom input line (border + text row).
pub const INPUT_HEIGHT: u16 = 3;

/// The set of non-overlapping regions the renderer draws into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Panels {
    pub header: Rect,
    pub transcript: Rect,
    pub tool_log: Rect,
    pub input: Rect,
}

impl Panels {
    /// True if any panel region touches a neighbour (i.e. an overlap bug).
    /// Zero-sized panels are allowed (terminal too small to show that region).
    #[allow(dead_code)] // asserted by tests only
    pub fn has_overlap(&self) -> bool {
        overlaps(self.header, self.transcript)
            || overlaps(self.header, self.tool_log)
            || overlaps(self.header, self.input)
            || overlaps(self.transcript, self.tool_log)
            || overlaps(self.transcript, self.input)
            || overlaps(self.tool_log, self.input)
    }
}

/// Splits `area` into the four dashboard panels, guaranteeing every region is
/// fully contained within `area` and mutually non-overlapping.
pub fn split(area: Rect) -> Panels {
    // --- vertical split into header / main / input (manual & saturating) ---
    let header_h = HEADER_HEIGHT.min(area.height);
    let input_h = {
        let remaining = area.height.saturating_sub(header_h);
        INPUT_HEIGHT.min(remaining)
    };
    let main_y = area.y.saturating_add(header_h);
    let main_h = area
        .height
        .saturating_sub(header_h)
        .saturating_sub(input_h);

    let header = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: header_h,
    };
    let main = Rect {
        x: area.x,
        y: main_y,
        width: area.width,
        height: main_h,
    };
    let input = Rect {
        x: area.x,
        y: main_y.saturating_add(main_h),
        width: area.width,
        height: input_h,
    };

    // --- horizontal split of main into transcript (left) + tool log (right) --
    let transcript_w = (main.width as u32 * 70 / 100) as u16; // ~70% to transcript
    let tool_log_w = main.width.saturating_sub(transcript_w);
    let transcript = Rect {
        x: main.x,
        y: main.y,
        width: transcript_w,
        height: main.height,
    };
    let tool_log = Rect {
        x: main.x.saturating_add(transcript_w),
        y: main.y,
        width: tool_log_w,
        height: main.height,
    };

    Panels {
        header,
        transcript,
        tool_log,
        input,
    }
}

/// True when two non-empty rectangles share any cell.
#[allow(dead_code)] // used by Panels::has_overlap (tests)
fn overlaps(a: Rect, b: Rect) -> bool {
    a.width > 0
        && a.height > 0
        && b.width > 0
        && b.height > 0
        && a.x < b.x.saturating_add(b.width)
        && b.x < a.x.saturating_add(a.width)
        && a.y < b.y.saturating_add(b.height)
        && b.y < a.y.saturating_add(a.height)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Asserts every panel is inside `area` and none overlap.
    fn assert_valid(area: Rect) {
        let panels = split(area);
        for (name, rect) in [
            ("header", panels.header),
            ("transcript", panels.transcript),
            ("tool_log", panels.tool_log),
            ("input", panels.input),
        ] {
            assert!(
                rect.x >= area.x
                    && rect.y >= area.y
                    && rect.x.saturating_add(rect.width) <= area.x.saturating_add(area.width)
                    && rect.y.saturating_add(rect.height)
                        <= area.y.saturating_add(area.height),
                "{name} {rect:?} escapes area {area:?}"
            );
        }
        assert!(!panels.has_overlap(), "panels overlap: {panels:?}");
    }

    #[test]
    fn layout_typical_terminal() {
        assert_valid(Rect::new(0, 0, 100, 30));
        assert_valid(Rect::new(0, 0, 80, 24));
    }

    #[test]
    fn layout_wide_and_very_wide() {
        assert_valid(Rect::new(0, 0, 200, 50));
        assert_valid(Rect::new(0, 0, 500, 60));
        assert_valid(Rect::new(0, 0, 300, 10));
    }

    #[test]
    fn layout_narrow_column() {
        assert_valid(Rect::new(0, 0, 8, 30));
        assert_valid(Rect::new(0, 0, 4, 30));
        assert_valid(Rect::new(0, 0, 20, 8));
    }

    #[test]
    fn layout_tiny_no_panic_and_contained() {
        for (w, h) in [(5, 3), (3, 3), (3, 2), (2, 2), (1, 1), (0, 0)] {
            assert_valid(Rect::new(0, 0, w, h));
        }
    }

    #[test]
    fn offset_area_keeps_absolute_positions() {
        // Panels must honour a non-zero origin (e.g. nested regions).
        assert_valid(Rect::new(10, 20, 60, 15));
    }

    #[test]
    fn transcript_is_left_and_larger_tool_log_right() {
        let area = Rect::new(0, 0, 100, 24);
        let panels = split(area);
        assert!(panels.transcript.x < panels.tool_log.x);
        assert!(panels.transcript.width >= panels.tool_log.width);
    }
}
