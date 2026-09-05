//! Spec 013 — Hermes Python UI parity, Ticket 04: streaming display.
//!
//! Live streaming of model text with the verbatim response frame (spec §5.1),
//! the dim reasoning box (spec §5.2), tool-activity lines (spec §6), and the
//! braille/kawaii spinner (spec §7).
//!
//! Security invariants (spec §2):
//! * **Invariant 4** — every streamed chunk passes through the same CLI-stdout
//!   boundary sanitizer as the final frame (`sanitize_untrusted_output` +
//!   `redact_credentials`). Raw escapes and leaked tool-call markup are
//!   impossible in TTY or piped mode.
//! * **Invariant 5** — this module is *display only*: it never mutates the
//!   canonical conversation bytes (SQLite).
//!
//! Every state machine here is plain (no I/O in constructors; I/O methods
//! take `&mut dyn Write`), so the full rendering is unit-testable against
//! `Vec<u8>`.

use std::io::{self, Write};
use std::time::{Duration, Instant};

use hermes_core::conversation::AgentEvent;
use hermes_core::tools::ToolExecutionStatus;

use crate::output::sanitize_untrusted_output;
use crate::tui::event::redact;
use crate::tui::kawaii::{
    DOTS, KAWAII_THINKING, REASONING_CLOSE_TAGS, REASONING_OPEN_TAGS, THINKING_VERBS, TICK_MS,
};
use crate::tui::theme::{detect_color_depth, ColorDepth};
use crate::tui::welcome::{
    sgr_banner_text, sgr_bold_gold, sgr_dim_italic, RESPONSE_LABEL, RESPONSE_LABEL_WIDTH, SGR_RESET,
};

/// Cell width of the reasoning label ` Reasoning ` (spec §5.2).
const REASONING_LABEL_WIDTH: usize = 11;
/// The reasoning box label, spaces on both sides (spec §5.2 verbatim).
const REASONING_LABEL: &str = " Reasoning ";

// ---------------------------------------------------------------------------
// Reasoning tag splitter (spec §5.2)
// ---------------------------------------------------------------------------

/// One finalized slice of the stream produced by the reasoning splitter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    /// Text inside a reasoning tag (rendered into the dim reasoning box).
    Reasoning(String),
    /// Plain model text (rendered into the response box).
    Normal(String),
}

/// Byte-level state machine that peels reasoning tags (`` … ``,
/// `<REASONING_SCRATCHPAD>`, `<think>`, …) out of a streamed text so they are
/// displayed in the dim reasoning box and never in the response box or the
/// canonical transcript (spec §5.2).
///
/// Chunks may split **anywhere** (even mid-tag), so the splitter holds a
/// trailing tag-prefix between `feed` calls and only emits text that a later
/// chunk cannot re-interpret. Matching is byte-exact: the tag set is ASCII,
/// and holding a trailing ASCII prefix can never split a multi-byte UTF-8
/// character.
pub struct ReasoningSplitter {
    pending: Vec<u8>,
    in_reasoning: bool,
    tags: Vec<&'static str>,
}

impl Default for ReasoningSplitter {
    fn default() -> Self {
        Self::new()
    }
}

impl ReasoningSplitter {
    pub fn new() -> Self {
        let mut tags: Vec<&'static str> = REASONING_OPEN_TAGS.to_vec();
        tags.extend_from_slice(&REASONING_CLOSE_TAGS);
        Self {
            pending: Vec::new(),
            in_reasoning: false,
            tags,
        }
    }

    /// True while the stream is inside an unclosed reasoning tag.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "test-only state query; the renderer tracks box state itself"
        )
    )]
    pub fn in_reasoning(&self) -> bool {
        self.in_reasoning
    }

    /// Feed the next chunk and return the finalized segments (in order).
    pub fn feed(&mut self, chunk: &str) -> Vec<Segment> {
        let mut buf = std::mem::take(&mut self.pending);
        buf.extend_from_slice(chunk.as_bytes());
        let mut out: Vec<Segment> = Vec::new();
        let mut start = 0usize;
        while let Some((pos, len)) = find_earliest(&buf, start, &self.tags) {
            self.push_segment(&mut out, &buf[start..pos]);
            self.in_reasoning = tag_state(&buf, pos, len, self.in_reasoning);
            start = pos + len;
        }
        let hold = max_holdback(&buf[start..], &self.tags);
        let end = buf.len() - hold;
        self.push_segment(&mut out, &buf[start..end]);
        self.pending = buf[end..].to_vec();
        out
    }

    /// Finalize the stream: flush any held tail (a stream that ended inside an
    /// unclosed tag keeps its tail in the current mode).
    pub fn finish(&mut self) -> Vec<Segment> {
        let tail = std::mem::take(&mut self.pending);
        let text = String::from_utf8_lossy(&tail).into_owned();
        if text.is_empty() {
            Vec::new()
        } else if self.in_reasoning {
            vec![Segment::Reasoning(text)]
        } else {
            vec![Segment::Normal(text)]
        }
    }

    fn push_segment(&self, out: &mut Vec<Segment>, raw: &[u8]) {
        if raw.is_empty() {
            return;
        }
        let text = String::from_utf8_lossy(raw).into_owned();
        if text.is_empty() {
            return;
        }
        if self.in_reasoning {
            out.push(Segment::Reasoning(text));
        } else {
            out.push(Segment::Normal(text));
        }
    }
}

/// The reasoning state after the tag at `buf[pos..pos+len]`, given the
/// current state.
///
/// Tags that appear in *both* lists (the backtick scratchpad: open and close
/// are the same string) toggle, matching the Python stream consumer, which
/// only looks at open tags while closed and at close tags while open.
fn tag_state(buf: &[u8], pos: usize, len: usize, current: bool) -> bool {
    let tag = &buf[pos..pos + len];
    let is_open = REASONING_OPEN_TAGS.iter().any(|t| t.as_bytes() == tag);
    let is_close = REASONING_CLOSE_TAGS.iter().any(|t| t.as_bytes() == tag);
    match (is_open, is_close) {
        (true, false) => true,
        (false, true) => false,
        _ => !current, // shared tag (```) toggles
    }
}

/// Index/length of the earliest full tag occurrence in `buf[start..]`.
fn find_earliest(buf: &[u8], start: usize, tags: &[&str]) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize)> = None;
    for t in tags {
        let tb = t.as_bytes();
        if start + tb.len() > buf.len() {
            continue;
        }
        let mut i = start;
        while i + tb.len() <= buf.len() {
            if &buf[i..i + tb.len()] == tb {
                if best.is_none_or(|(bp, _)| i < bp) {
                    best = Some((i, tb.len()));
                }
                break;
            }
            i += 1;
        }
    }
    best
}

/// Longest suffix of `tail` that is a prefix of any tag (holdback length).
fn max_holdback(tail: &[u8], tags: &[&str]) -> usize {
    let mut max_hold = 0usize;
    for t in tags {
        let tb = t.as_bytes();
        let limit = tb.len().min(tail.len());
        for l in (1..=limit).rev() {
            if tail[tail.len() - l..] == tb[..l] && l > max_hold {
                max_hold = l;
                break;
            }
        }
    }
    max_hold
}

// ---------------------------------------------------------------------------
// Tool-call markup filter (spec §6 boundary hygiene)
// ---------------------------------------------------------------------------

/// Tool-call XML tag roots that open models may leak into visible content
/// (parity with the Python stream consumer / `_strip_think_blocks`, which
/// strips these live before display). The Gemma-style bare `<function …>`
/// root is included only with Python's `name=` + line-boundary gates.
const TOOL_TAG_ROOTS: [&str; 6] = [
    "tool_call",
    "tool_calls",
    "tool_result",
    "function_call",
    "function_calls",
    "function",
];

/// Strips tool-call XML blocks (`<tool_call …>…</tool_call>` and the sibling
/// roots) from streamed text, handling blocks that span chunk boundaries
/// (holdback) and orphan close tags. Case-insensitive, like the Python
/// regexes. The result is only ever *displayed* — the canonical transcript
/// bytes are untouched (invariant 5).
pub struct ToolCallFilter {
    pending: Vec<u8>,
}

impl Default for ToolCallFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolCallFilter {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
        }
    }

    /// Feed the next chunk; returns the display-clean text for it.
    pub fn feed(&mut self, chunk: &str) -> String {
        let mut buf = std::mem::take(&mut self.pending);
        buf.extend_from_slice(chunk.as_bytes());
        let mut out = String::new();
        let mut start = 0usize;
        loop {
            let opener = find_earliest_opener(&buf, start);
            let closer = find_earliest_closer(&buf, start);
            match (opener, closer) {
                (Some(op), Some(cl)) if cl.0 < op.0 => {
                    // Orphan close tag first — strip it (and the following
                    // whitespace run, parity with the Python `\s*`).
                    out.push_str(&decode(&buf[start..cl.0]));
                    start = eat_whitespace_after(&buf, cl.0 + cl.1);
                }
                (Some((op_pos, op_len)), _) => {
                    let root = opener_root(&buf, op_pos);
                    match find_closer_for(&buf, op_pos + op_len, &root) {
                        Some((c_pos, c_len)) => {
                            out.push_str(&decode(&buf[start..op_pos]));
                            start = eat_whitespace_after(&buf, c_pos + c_len);
                        }
                        None => {
                            // Complete opener, no closer yet: strip so far and
                            // hold from the opener onward (the block may
                            // continue in a later chunk).
                            out.push_str(&decode(&buf[start..op_pos]));
                            self.pending = buf[op_pos..].to_vec();
                            return out;
                        }
                    }
                }
                (None, Some((c_pos, c_len))) => {
                    out.push_str(&decode(&buf[start..c_pos]));
                    start = eat_whitespace_after(&buf, c_pos + c_len);
                }
                (None, None) => break,
            }
        }
        // Hold back a trailing opener prefix (the opener may continue in a
        // later chunk).
        let hold = max_opener_holdback(&buf[start..]);
        let end = buf.len() - hold;
        out.push_str(&decode(&buf[start..end]));
        self.pending = buf[end..].to_vec();
        out
    }

    /// Finalize: a stream that ends inside a tool-call block drops the held
    /// tail (parity with the Python "unterminated opener" strip-to-end). A
    /// mere incomplete opener *prefix* (never a real tag) is flushed as
    /// prose.
    pub fn finish(&mut self) -> String {
        let tail = std::mem::take(&mut self.pending);
        let complete_opener = TOOL_TAG_ROOTS.iter().any(|root| {
            let pat = format!("<{root}").into_bytes();
            tail.starts_with(&pat)
                && is_word_boundary(&tail, pat.len())
                && tail[pat.len()..].contains(&b'>')
        });
        if complete_opener {
            String::new()
        } else {
            decode(&tail)
        }
    }
}

/// Decode bytes to a String (lossy — chunks are valid UTF-8 and holdback
/// tails are ASCII prefixes, so lossy is a no-op in practice).
fn decode(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// End index after the whitespace run starting at `idx` (the Python strip
/// regexes eat the `\s*` following a removed block).
fn eat_whitespace_after(buf: &[u8], idx: usize) -> usize {
    let mut end = idx;
    while end < buf.len() && matches!(buf[end], b' ' | b'\t' | b'\n' | b'\r') {
        end += 1;
    }
    end
}

/// Find the earliest valid tool opener at/after `start`. Returns
/// `(position, full-opener-length)` where the opener ends at its `>`.
fn find_earliest_opener(buf: &[u8], start: usize) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize)> = None;
    for root in TOOL_TAG_ROOTS {
        let pat = format!("<{root}").into_bytes();
        let mut i = start;
        while i + pat.len() <= buf.len() {
            if ci_eq(&buf[i..i + pat.len()], &pat)
                && is_word_boundary(buf, i + pat.len())
                && opener_gates_valid(buf, i, root)
            {
                if let Some(gt) = buf[i + pat.len()..].iter().position(|&b| b == b'>') {
                    let op_len = pat.len() + gt + 1;
                    if best.is_none_or(|(bp, _)| i < bp) {
                        best = Some((i, op_len));
                    }
                    break;
                }
            }
            i += 1;
        }
    }
    best
}

/// Gates for the `<function …>` root (parity with Python): `name=` must be
/// present in the attributes, and the opener must sit at the start of the
/// buffer or after one of `[\n\r.!?:]` (spaces/tabs in between). The other
/// roots have no gate.
fn opener_gates_valid(buf: &[u8], pos: usize, root: &str) -> bool {
    if root != "function" {
        return true;
    }
    let gt = match buf[pos..].iter().position(|&b| b == b'>') {
        Some(o) => pos + o,
        None => return false,
    };
    if !contains_ci(&buf[pos..=gt], b"name=") {
        return false;
    }
    let mut i = pos;
    while i > 0 && matches!(buf[i - 1], b' ' | b'\t') {
        i -= 1;
    }
    if i == 0 {
        return true;
    }
    matches!(buf[i - 1], b'\n' | b'\r' | b'.' | b'!' | b'?')
}

/// Word boundary after `<root`: next byte is not `[A-Za-z0-9_]`.
fn is_word_boundary(buf: &[u8], idx: usize) -> bool {
    match buf.get(idx) {
        None => true,
        Some(&b) => !(b.is_ascii_alphanumeric() || b == b'_'),
    }
}

/// Root of the opener at `pos` (for matching its closer). Longest-root-first
/// (`function_call` before `function`) is guaranteed by `TOOL_TAG_ROOTS`
/// order.
fn opener_root(buf: &[u8], pos: usize) -> String {
    let gt = buf[pos..]
        .iter()
        .position(|&b| b == b'>')
        .unwrap_or(buf.len() - pos);
    let tag = &buf[pos + 1..pos + gt]; // strip leading '<'
    for root in TOOL_TAG_ROOTS {
        let rb = root.as_bytes();
        if tag.len() >= rb.len() && ci_eq(&tag[..rb.len()], rb) && is_word_boundary(tag, rb.len()) {
            return root.to_owned();
        }
    }
    String::new()
}

/// Find a close tag `</root>` at/after `start` (root-exact match).
fn find_closer_for(buf: &[u8], start: usize, root: &str) -> Option<(usize, usize)> {
    let pat = format!("</{root}>").into_bytes();
    let mut i = start;
    while i + pat.len() <= buf.len() {
        if ci_eq(&buf[i..i + pat.len()], &pat) {
            return Some((i, pat.len()));
        }
        i += 1;
    }
    None
}

/// Find the earliest orphan close tag of any root at/after `start`.
fn find_earliest_closer(buf: &[u8], start: usize) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize)> = None;
    for root in TOOL_TAG_ROOTS {
        let pat = format!("</{root}>").into_bytes();
        let mut i = start;
        while i + pat.len() <= buf.len() {
            if ci_eq(&buf[i..i + pat.len()], &pat) {
                if best.is_none_or(|(bp, _)| i < bp) {
                    best = Some((i, pat.len()));
                }
                break;
            }
            i += 1;
        }
    }
    best
}

/// Case-insensitive byte equality.
fn ci_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.eq_ignore_ascii_case(y))
}

/// Case-insensitive substring search.
fn contains_ci(hay: &[u8], needle: &[u8]) -> bool {
    if needle.len() > hay.len() {
        return false;
    }
    (0..=hay.len() - needle.len()).any(|i| ci_eq(&hay[i..i + needle.len()], needle))
}

/// Longest suffix of `tail` that is a prefix of `<root` for any root.
fn max_opener_holdback(tail: &[u8]) -> usize {
    let mut max_hold = 0usize;
    for root in TOOL_TAG_ROOTS {
        let pat = format!("<{root}").into_bytes();
        let limit = pat.len().min(tail.len());
        for l in (1..=limit).rev() {
            if ci_eq(&tail[tail.len() - l..], &pat[..l]) && l > max_hold {
                max_hold = l;
                break;
            }
        }
    }
    max_hold
}

// ---------------------------------------------------------------------------
// Streaming renderer (spec §5)
// ---------------------------------------------------------------------------

/// Live streaming response renderer.
///
/// TTY: gold-bold response frame (header on the first text chunk, footer at
/// finish), dim-italic reasoning box (always before the response box), dim
/// `  ┊ ◇`/`  ┊ ✅`/`  ┊ ❌` tool lines. Piped: plain frame plus `  [tool]`
/// lines, no ANSI at all (E2E byte-stable).
pub struct StreamRenderer {
    tty: bool,
    width: usize,
    depth: ColorDepth,
    split: ReasoningSplitter,
    tool: ToolCallFilter,
    reasoning_open: bool,
    response_open: bool,
    any_text: bool,
    started: bool,
    /// Whether the last byte written ended with a newline (Python's
    /// `_cprint` puts the footer on its own line even for unterminated
    /// final chunks).
    ends_nl: bool,
}

impl StreamRenderer {
    /// Create a renderer for `tty` (true → styled frames) at `width` columns,
    /// with the environment-detected color depth.
    pub fn new(tty: bool, width: u16) -> Self {
        Self::with_depth(tty, width.max(20), detect_color_depth())
    }

    /// Test/deterministic constructor with an explicit depth.
    pub fn with_depth(tty: bool, width: u16, depth: ColorDepth) -> Self {
        Self {
            tty,
            width: width.max(20) as usize,
            depth,
            split: ReasoningSplitter::new(),
            tool: ToolCallFilter::new(),
            reasoning_open: false,
            response_open: false,
            any_text: false,
            started: false,
            ends_nl: true,
        }
    }

    /// True once any streamed text (reasoning or response) has been rendered
    /// — the REPL uses this to stop the spinner and to de-duplicate the
    /// final `AgenticResult::Done` text.
    pub fn any_text(&self) -> bool {
        self.any_text
    }

    /// Feed one raw model chunk. Returns true when text was rendered.
    pub fn on_chunk(&mut self, w: &mut dyn Write, raw: &str) -> io::Result<bool> {
        let scrubbed = redact(&sanitize_untrusted_output(raw));
        let clean = self.tool.feed(&scrubbed);
        let segments = self.split.feed(&clean);
        let mut rendered = false;
        for seg in segments {
            rendered |= self.write_segment(w, seg)?;
        }
        if rendered {
            self.any_text = true;
        }
        Ok(rendered)
    }

    /// A tool call is starting (spec §6): close any open box, print the tool
    /// line, so the next streamed answer resumes in a fresh box.
    pub fn on_tool_started(&mut self, w: &mut dyn Write, name: &str) -> io::Result<()> {
        let name = sanitize_untrusted_output(name);
        self.close_reasoning(w)?;
        self.close_response(w)?;
        self.first_clear(w)?;
        if self.tty {
            writeln!(w, "  {}┊ ◇ {name}{SGR_RESET}", sgr_dim_italic(self.depth))?;
        } else {
            writeln!(w, "  [tool] {name}")?;
        }
        self.ends_nl = true;
        Ok(())
    }

    /// A tool call completed (spec §6): TTY prints the ✅/❌ line.
    pub fn on_tool_done(
        &mut self,
        w: &mut dyn Write,
        name: &str,
        status: &ToolExecutionStatus,
    ) -> io::Result<()> {
        if !self.tty {
            return Ok(());
        }
        let name = sanitize_untrusted_output(name);
        let mark = if *status == ToolExecutionStatus::Success {
            "✅"
        } else {
            "❌"
        };
        writeln!(
            w,
            "  {}┊ {mark} {name}{SGR_RESET}",
            sgr_dim_italic(self.depth)
        )?;
        self.ends_nl = true;
        Ok(())
    }

    /// Finalize the stream: flush held tails and close any open boxes.
    pub fn finish(&mut self, w: &mut dyn Write) -> io::Result<()> {
        let tail_text = self.tool.finish();
        let mut segments = self.split.feed(&tail_text);
        segments.extend(self.split.finish());
        for seg in segments {
            self.write_segment(w, seg)?;
        }
        self.close_reasoning(w)?;
        self.close_response(w)?;
        Ok(())
    }

    /// Non-streaming fallback: render the final answer through the same
    /// scrub → filter → split → box pipeline.
    pub fn emit_final(&mut self, w: &mut dyn Write, text: &str) -> io::Result<()> {
        self.on_chunk(w, text)?;
        self.finish(w)
    }

    // -- internals ---------------------------------------------------------

    fn write_segment(&mut self, w: &mut dyn Write, seg: Segment) -> io::Result<bool> {
        let reasoning = matches!(seg, Segment::Reasoning(_));
        let text = match seg {
            Segment::Reasoning(t) | Segment::Normal(t) => t,
        };
        if text.is_empty() {
            return Ok(false);
        }
        if reasoning {
            self.close_response(w)?;
            if !self.reasoning_open {
                self.open_reasoning(w)?;
            }
        } else {
            self.close_reasoning(w)?;
            if !self.response_open {
                self.open_response(w)?;
            }
        }
        let (fg, reset) = if reasoning {
            (sgr_dim_italic(self.depth), SGR_RESET)
        } else if self.tty {
            (sgr_banner_text(self.depth), SGR_RESET)
        } else {
            (String::new(), "")
        };
        if self.tty {
            write!(w, "{fg}{text}{reset}")?;
        } else {
            write!(w, "{text}")?;
        }
        w.flush()?;
        self.ends_nl = text.ends_with('\n');
        Ok(true)
    }

    fn first_clear(&mut self, w: &mut dyn Write) -> io::Result<()> {
        if self.tty && !self.started {
            // Erase the spinner line before the first output (no-op when the
            // spinner was not visible).
            write!(w, "\r\x1b[2K\r")?;
        }
        self.started = true;
        Ok(())
    }

    fn open_reasoning(&mut self, w: &mut dyn Write) -> io::Result<()> {
        self.first_clear(w)?;
        let fill = self
            .width
            .saturating_sub(2)
            .saturating_sub(REASONING_LABEL_WIDTH);
        if self.tty {
            writeln!(
                w,
                "\n{}┌─{REASONING_LABEL}{}┐{SGR_RESET}",
                sgr_dim_italic(self.depth),
                "─".repeat(fill.saturating_sub(1))
            )?;
        } else {
            writeln!(
                w,
                "\n┌─{REASONING_LABEL}{}┐",
                "─".repeat(fill.saturating_sub(1))
            )?;
        }
        self.reasoning_open = true;
        self.ends_nl = true;
        Ok(())
    }

    fn open_response(&mut self, w: &mut dyn Write) -> io::Result<()> {
        self.first_clear(w)?;
        let fill = self
            .width
            .saturating_sub(2)
            .saturating_sub(RESPONSE_LABEL_WIDTH);
        if self.tty {
            writeln!(
                w,
                "\n{}╭─{RESPONSE_LABEL}{}╮{SGR_RESET}",
                sgr_bold_gold(self.depth),
                "─".repeat(fill.saturating_sub(1))
            )?;
        } else {
            writeln!(
                w,
                "\n╭─{RESPONSE_LABEL}{}╮",
                "─".repeat(fill.saturating_sub(1))
            )?;
        }
        self.response_open = true;
        self.ends_nl = true;
        Ok(())
    }

    fn close_reasoning(&mut self, w: &mut dyn Write) -> io::Result<()> {
        if !self.reasoning_open {
            return Ok(());
        }
        if !self.ends_nl {
            writeln!(w)?;
        }
        let footer = "─".repeat(self.width.saturating_sub(2));
        if self.tty {
            writeln!(w, "{}└{footer}┘{SGR_RESET}", sgr_dim_italic(self.depth))?;
        } else {
            writeln!(w, "└{footer}┘")?;
        }
        self.reasoning_open = false;
        self.ends_nl = true;
        Ok(())
    }

    fn close_response(&mut self, w: &mut dyn Write) -> io::Result<()> {
        if !self.response_open {
            return Ok(());
        }
        if !self.ends_nl {
            writeln!(w)?;
        }
        let footer = "─".repeat(self.width.saturating_sub(2));
        if self.tty {
            writeln!(w, "{}╰{footer}╯{SGR_RESET}", sgr_bold_gold(self.depth))?;
        } else {
            writeln!(w, "╰{footer}╯")?;
        }
        self.response_open = false;
        self.ends_nl = true;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Spinner (spec §7)
// ---------------------------------------------------------------------------

/// What the spinner represents while the model/tool is working.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpinnerMode {
    /// Model thinking — braille `dots` + rotating thinking verb.
    Thinking,
    /// A tool is executing — rotating kawaii face + `tool: {name}`.
    Tool(String),
}

/// Pure spinner state; the owning task advances one frame every
/// [`TICK_INTERVAL`] (120 ms, parity with `KawaiiSpinner`).
pub struct SpinnerState {
    mode: SpinnerMode,
    started: Instant,
    tick: u64,
}

impl SpinnerState {
    pub fn new(mode: SpinnerMode) -> Self {
        Self {
            mode,
            started: Instant::now(),
            tick: 0,
        }
    }

    /// Advance one frame and return the line (caller prefixes `\r`).
    pub fn advance(&mut self) -> String {
        self.tick += 1;
        self.line_at(self.started.elapsed().as_secs_f32())
    }

    /// The line at an explicit elapsed time — pure and unit-testable
    /// (format parity: `  {frame} {message} ({elapsed:.1}s)`).
    pub fn line_at(&self, elapsed: f32) -> String {
        let frame = match &self.mode {
            SpinnerMode::Thinking => DOTS[(self.tick % DOTS.len() as u64) as usize],
            SpinnerMode::Tool(_) => {
                KAWAII_THINKING[((self.tick / 5) % KAWAII_THINKING.len() as u64) as usize]
            }
        };
        let message = match &self.mode {
            SpinnerMode::Thinking => {
                THINKING_VERBS[((self.tick / 10) % THINKING_VERBS.len() as u64) as usize].to_owned()
            }
            SpinnerMode::Tool(name) => {
                return format!("  {frame} tool: {name} ({elapsed:.1}s)");
            }
        };
        format!("  {frame} {message} ({elapsed:.1}s)")
    }
}

/// The 120 ms tick interval (`KawaiiSpinner` `time.sleep(0.12)`).
pub const TICK_INTERVAL: Duration = Duration::from_millis(TICK_MS);

// ---------------------------------------------------------------------------
// Event → display mapping
// ---------------------------------------------------------------------------

/// Map one agentic event onto the streaming display (spec §5/§7).
///
/// All untrusted strings (chunk text, tool names) are scrubbed + redacted at
/// this boundary (invariant 4); the canonical bytes are untouched.
pub fn apply_event(
    renderer: &mut StreamRenderer,
    spinner: &mut Option<SpinnerState>,
    out: &mut dyn Write,
    ev: &AgentEvent,
) -> io::Result<()> {
    match ev {
        AgentEvent::Chunk { text } => {
            renderer.on_chunk(out, text)?;
            // Streaming text is flowing — the spinner steps aside.
            *spinner = None;
        }
        AgentEvent::ToolStarted { name, .. } => {
            renderer.on_tool_started(out, name)?;
            *spinner = Some(SpinnerState::new(SpinnerMode::Tool(name.clone())));
        }
        AgentEvent::ToolDone { name, status, .. } => {
            renderer.on_tool_done(out, name, status)?;
            *spinner = Some(SpinnerState::new(SpinnerMode::Thinking));
        }
        AgentEvent::Done { .. } => {
            renderer.finish(out)?;
            *spinner = None;
        }
        // Iteration / TokenTick / StatusChanged / Error: no dedicated display
        // line in the spec (the status bar is Ticket 05).
        _ => {}
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    type S = Segment;

    fn segs(v: &[S]) -> String {
        v.iter()
            .map(|s| match s {
                S::Reasoning(t) => format!("[R]{t}"),
                S::Normal(t) => format!("[N]{t}"),
            })
            .collect::<Vec<_>>()
            .join("|")
    }

    // -- ReasoningSplitter --------------------------------------------------

    #[test]
    fn splitter_passes_plain_text() {
        let mut sp = ReasoningSplitter::new();
        assert_eq!(segs(&sp.feed("Hello ")), "[N]Hello ");
        assert_eq!(segs(&sp.feed("world")), "[N]world");
        assert!(sp.finish().is_empty());
    }

    #[test]
    fn splitter_basic_reasoning_block() {
        let mut sp = ReasoningSplitter::new();
        let out = sp.feed("pre<think>reason here</think>post");
        assert_eq!(segs(&out), "[N]pre|[R]reason here|[N]post");
    }

    #[test]
    fn splitter_tag_split_across_chunks() {
        let mut sp = ReasoningSplitter::new();
        assert_eq!(segs(&sp.feed("a<thi")), "[N]a");
        assert_eq!(segs(&sp.feed("nking>x")), "[R]x");
        assert!(sp.finish().is_empty());
    }

    #[test]
    fn splitter_close_tag_split_across_chunks() {
        // a stray close tag outside a reasoning block is inert (parity:
        // the consumer only reacts to close tags while in-reasoning)
        let mut sp = ReasoningSplitter::new();
        assert_eq!(segs(&sp.feed("hi</thin")), "[N]hi");
        assert_eq!(segs(&sp.feed("k>!")), "[N]!");
        // ...and a close tag split mid-way inside reasoning ends the block
        let mut sp = ReasoningSplitter::new();
        assert_eq!(segs(&sp.feed("<think>hi</thin")), "[R]hi");
        assert_eq!(segs(&sp.feed("k>!")), "[N]!");
    }

    #[test]
    fn splitter_all_six_tag_families() {
        let cases = [
            ("<REASONING_SCRATCHPAD>", "</REASONING_SCRATCHPAD>"),
            ("<reasoning>", "</reasoning>"),
            ("<THINKING>", "</THINKING>"),
            ("<thinking>", "</thinking>"),
            ("<thought>", "</thought>"),
        ];
        for (open, close) in cases {
            let mut sp = ReasoningSplitter::new();
            let out = sp.feed(&format!("A{open}B{close}C"));
            assert_eq!(segs(&out), "[N]A|[R]B|[N]C", "tag {open}");
        }
    }

    #[test]
    fn splitter_unclosed_tag_keeps_mode() {
        let mut sp = ReasoningSplitter::new();
        assert_eq!(segs(&sp.feed("<REASONING_SCRATCHPAD>plan")), "[R]plan");
        assert!(sp.in_reasoning());
        // tail text stays reasoning-mode at the end of the stream
        let out = sp.feed(" more");
        assert_eq!(segs(&out), "[R] more");
        assert!(sp.finish().is_empty());
    }

    #[test]
    fn splitter_lookalike_mid_text_stays_literal() {
        let mut sp = ReasoningSplitter::new();
        // `<RE...` continued by non-tag bytes mid-text is prose.
        assert_eq!(segs(&sp.feed("x <REally y")), "[N]x <REally y");
        assert_eq!(segs(&sp.feed(" more")), "[N] more");
    }

    #[test]
    fn splitter_trailing_tag_prefix_held() {
        let mut sp = ReasoningSplitter::new();
        assert_eq!(segs(&sp.feed("x <RE")), "[N]x ");
        assert_eq!(segs(&sp.feed("ally")), "[N]<REally");
        assert!(sp.finish().is_empty());
    }

    // -- ToolCallFilter -----------------------------------------------------

    #[test]
    fn toolfilter_strips_complete_block() {
        let mut f = ToolCallFilter::new();
        let out = f.feed("before <tool_call id=\"1\">read_file: {\"p\":\"x\"}</tool_call> after");
        assert_eq!(out, "before after");
    }

    #[test]
    fn toolfilter_strips_orphan_close() {
        let mut f = ToolCallFilter::new();
        let out = f.feed("text </tool_call> more");
        assert_eq!(out, "text more");
    }

    #[test]
    fn toolfilter_block_split_across_chunks() {
        let mut f = ToolCallFilter::new();
        let a = f.feed("lead <tool_call id=\"1\">re");
        assert_eq!(a, "lead ");
        let b = f.feed("ad_file</tool_call> trail");
        assert_eq!(b, "trail");
    }

    #[test]
    fn toolfilter_unterminated_at_end_drops_tail() {
        let mut f = ToolCallFilter::new();
        let a = f.feed("hello <tool_call id=\"1\">re");
        assert_eq!(a, "hello ");
        assert_eq!(f.finish(), "");
    }

    #[test]
    fn toolfilter_incomplete_opener_prefix_is_prose() {
        let mut f = ToolCallFilter::new();
        let a = f.feed("note: <tool_ca");
        assert_eq!(a, "note: ");
        // no closing '>' ever arrives -> the held prefix is flushed as prose
        assert_eq!(f.finish(), "<tool_ca");
    }

    #[test]
    fn toolfilter_word_boundary_no_false_positive() {
        let mut f = ToolCallFilter::new();
        // `<tools>` and `<tool>` are NOT roots -> prose stays.
        let out = f.feed("use <tools> and <tool> here");
        assert_eq!(out, "use <tools> and <tool> here");
    }

    #[test]
    fn toolfilter_function_root_needs_name_gate() {
        let mut f = ToolCallFilter::new();
        // prose `<function>` without `name=` is not a tool opener
        let out = f.feed("the <function> concept");
        assert_eq!(out, "the <function> concept");
        // Gemma-style with name= at line start is stripped
        let mut f2 = ToolCallFilter::new();
        let out2 = f2.feed("ok\n<function name=\"f\">args</function>\ntail");
        assert_eq!(out2, "ok\ntail");
    }

    #[test]
    fn toolfilter_case_insensitive() {
        let mut f = ToolCallFilter::new();
        let out = f.feed("a <TOOL_CALL x=\"1\">b</TOOL_CALL> c");
        assert_eq!(out, "a c");
    }

    // -- SpinnerState -------------------------------------------------------

    #[test]
    fn spinner_thinking_line_format() {
        let mut sp = SpinnerState::new(SpinnerMode::Thinking);
        assert_eq!(
            sp.line_at(0.1),
            format!("  {} {} (0.1s)", DOTS[0], THINKING_VERBS[0])
        );
        sp.tick = 1;
        assert_eq!(
            sp.line_at(0.2),
            format!("  {} {} (0.2s)", DOTS[1], THINKING_VERBS[0])
        );
        sp.tick = 10;
        assert_eq!(
            sp.line_at(1.0),
            format!("  {} {} (1.0s)", DOTS[0], THINKING_VERBS[1])
        );
    }

    #[test]
    fn spinner_tool_line_format() {
        let sp = SpinnerState {
            mode: SpinnerMode::Tool("read_file".into()),
            started: Instant::now(),
            tick: 3,
        };
        assert_eq!(
            sp.line_at(2.5),
            format!("  {} tool: read_file (2.5s)", KAWAII_THINKING[0])
        );
    }

    #[test]
    fn spinner_cycles_all_frames() {
        let mut sp = SpinnerState::new(SpinnerMode::Thinking);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..10 {
            sp.advance();
            seen.insert(sp.line_at(9.9).chars().nth(2).unwrap());
        }
        assert_eq!(seen.len(), 10, "all 10 braille frames within one cycle");
    }

    // -- StreamRenderer -----------------------------------------------------

    fn piped(width: u16) -> StreamRenderer {
        StreamRenderer::with_depth(false, width, ColorDepth::Truecolor)
    }

    fn tty(width: u16) -> StreamRenderer {
        StreamRenderer::with_depth(true, width, ColorDepth::Truecolor)
    }

    #[test]
    fn renderer_width_math_matches_python() {
        // spec §5.1: fill = w - 2 - width(label); header/footer span `w`.
        let mut r = piped(60);
        let mut buf: Vec<u8> = Vec::new();
        r.on_chunk(&mut buf, "hello").unwrap();
        r.finish(&mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        // line 0 is blank: the Python header prints a leading "\n"
        assert!(lines[0].is_empty());
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[1].chars().count(), 60, "header spans full width");
        assert!(lines[1].starts_with("╭─ ⚕ Hermes "));
        assert!(lines[1].ends_with('╮'));
        assert_eq!(lines[2], "hello");
        assert!(lines[3].starts_with('╰'));
        assert!(lines[3].ends_with('╯'));
        assert_eq!(lines[3].chars().count(), 60);
    }

    #[test]
    fn renderer_is_total_on_tiny_widths() {
        // width clamps at 20 — no panic, still a valid box
        let mut r = piped(8);
        let mut buf: Vec<u8> = Vec::new();
        r.on_chunk(&mut buf, "x").unwrap();
        r.finish(&mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("╭─ ⚕ Hermes "));
        assert!(out.ends_with('\n'));
        assert!(out.contains('╯'));
    }

    #[test]
    fn renderer_piped_box_exact() {
        let mut r = piped(60);
        let mut buf: Vec<u8> = Vec::new();
        r.on_chunk(&mut buf, "echo: hello").unwrap();
        r.finish(&mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        let header = format!("╭─{RESPONSE_LABEL}{}╮", "─".repeat(60 - 2 - 10 - 1));
        let footer = format!("╰{}╯", "─".repeat(60 - 2));
        assert_eq!(out, format!("\n{header}\necho: hello\n{footer}\n"));
        assert!(!out.contains('\x1b'), "piped output must be ANSI-free");
    }

    #[test]
    fn renderer_piped_box_matches_t03_frame_math() {
        // Byte-parity with the Ticket 03 `response_frame` box (the streaming
        // path additionally prints a leading blank line, like the Python
        // header `"\n..."`).
        let mut r = piped(60);
        let mut buf: Vec<u8> = Vec::new();
        r.on_chunk(&mut buf, "echo: hello").unwrap();
        r.finish(&mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        let body = out.strip_prefix('\n').unwrap();
        assert!(body.starts_with("╭─ ⚕ Hermes "));
        assert!(body.contains("echo: hello"));
        assert!(body.ends_with(&format!("╰{}╯\n", "─".repeat(58))));
    }

    #[test]
    fn renderer_reasoning_before_response() {
        let mut r = piped(70);
        let mut buf: Vec<u8> = Vec::new();
        r.on_chunk(&mut buf, "<think>plan it</think>answer")
            .unwrap();
        r.finish(&mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        let reasoning_at = out.find("┌─ Reasoning ").unwrap();
        let response_at = out.find("╭─ ⚕ Hermes ").unwrap();
        assert!(
            reasoning_at < response_at,
            "reasoning box must precede response box"
        );
        assert!(out.contains("plan it"));
        assert!(out.contains("answer"));
        assert!(!out.contains("<think>"), "raw tags must not be displayed");
    }

    #[test]
    fn renderer_reasoning_tag_split_across_chunks() {
        let mut r = piped(70);
        let mut buf: Vec<u8> = Vec::new();
        r.on_chunk(&mut buf, "x<th").unwrap();
        r.on_chunk(&mut buf, "ink>reason").unwrap();
        r.on_chunk(&mut buf, "</thin").unwrap();
        r.on_chunk(&mut buf, "k>done").unwrap();
        r.finish(&mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("x"));
        assert!(out.contains("reason"));
        assert!(out.contains("done"));
        assert!(!out.contains("ink>"), "partial tag must not leak");
    }

    #[test]
    fn renderer_tty_uses_ansi_frames() {
        let mut r = tty(60);
        let mut buf: Vec<u8> = Vec::new();
        r.on_chunk(&mut buf, "hi").unwrap();
        r.finish(&mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("\x1b[1;38;2;255;215;0m"), "gold bold header");
        assert!(out.contains("\x1b[38;2;255;248;220m"), "banner_text body");
        assert!(out.contains("\x1b[0m"), "reset");
    }

    #[test]
    fn renderer_scrubs_ansi_and_split_escape() {
        let mut r = piped(60);
        let mut buf: Vec<u8> = Vec::new();
        r.on_chunk(&mut buf, "safe\x1b[3").unwrap();
        r.on_chunk(&mut buf, "1mred").unwrap();
        r.finish(&mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(!out.contains('\x1b'), "raw escape must never reach stdout");
        assert!(out.contains("safe"), "preceding text is kept");
    }

    #[test]
    fn renderer_redacts_credentials() {
        let mut r = piped(60);
        let mut buf: Vec<u8> = Vec::new();
        r.on_chunk(&mut buf, "key=sk-abcdefghijklmnopqrs").unwrap();
        r.finish(&mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(
            !out.contains("sk-abcdefghijklmnopqrs"),
            "secret must be redacted"
        );
    }

    #[test]
    fn renderer_tool_call_markup_not_displayed() {
        let mut r = piped(60);
        let mut buf: Vec<u8> = Vec::new();
        r.on_chunk(
            &mut buf,
            "<tool_call id=\"fake-1\">read_file: {\"path\":\"Cargo.toml\"}</tool_call>",
        )
        .unwrap();
        r.on_chunk(&mut buf, "tool completed").unwrap();
        r.finish(&mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(
            !out.contains("tool_call"),
            "raw tool_call markup must not display"
        );
        assert!(!out.contains("read_file: {"));
        assert!(out.contains("tool completed"));
        assert!(out.contains("╭─ ⚕ Hermes "));
    }

    #[test]
    fn renderer_tool_lines_piped() {
        let mut r = piped(60);
        let mut buf: Vec<u8> = Vec::new();
        r.on_tool_started(&mut buf, "read_file").unwrap();
        r.on_tool_done(&mut buf, "read_file", &ToolExecutionStatus::Success)
            .unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("  [tool] read_file"));
        assert!(!out.contains('✅'), "done line is TTY-only");
        assert!(!out.contains('\x1b'));
    }

    #[test]
    fn renderer_tool_lines_tty() {
        let mut r = tty(60);
        let mut buf: Vec<u8> = Vec::new();
        r.on_tool_started(&mut buf, "read_file").unwrap();
        r.on_tool_done(&mut buf, "read_file", &ToolExecutionStatus::Success)
            .unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("\x1b[2;3m┊ ◇ read_file\x1b[0m"));
        assert!(out.contains("\x1b[2;3m┊ ✅ read_file\x1b[0m"));
    }

    #[test]
    fn renderer_tool_line_closes_open_box() {
        let mut r = piped(60);
        let mut buf: Vec<u8> = Vec::new();
        r.on_chunk(&mut buf, "partial answer").unwrap();
        r.on_tool_started(&mut buf, "shell_readonly").unwrap();
        r.on_chunk(&mut buf, "final").unwrap();
        r.finish(&mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        let headers = out.matches("╭─ ⚕ Hermes ").count();
        assert_eq!(
            headers, 2,
            "streaming answer resumes in a new box after a tool"
        );
        assert!(out.contains("  [tool] shell_readonly"));
    }

    #[test]
    fn renderer_unclosed_reasoning_closes_at_finish() {
        let mut r = piped(70);
        let mut buf: Vec<u8> = Vec::new();
        r.on_chunk(&mut buf, "<think>never closed").unwrap();
        r.finish(&mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("┌─ Reasoning "));
        assert!(out.contains("└"), "reasoning footer must be printed");
    }

    // -- apply_event --------------------------------------------------------

    #[test]
    fn apply_event_chunk_stops_spinner() {
        let mut r = piped(60);
        let mut spinner = Some(SpinnerState::new(SpinnerMode::Thinking));
        let mut buf: Vec<u8> = Vec::new();
        let ev = AgentEvent::Chunk {
            text: "hello".into(),
        };
        apply_event(&mut r, &mut spinner, &mut buf, &ev).unwrap();
        assert!(spinner.is_none());
        assert!(r.any_text());
    }

    #[test]
    fn apply_event_tool_starts_spinner() {
        let mut r = piped(60);
        let mut spinner: Option<SpinnerState> = None;
        let mut buf: Vec<u8> = Vec::new();
        let ev = AgentEvent::ToolStarted {
            name: "read_file".into(),
            arguments: "{}".into(),
        };
        apply_event(&mut r, &mut spinner, &mut buf, &ev).unwrap();
        assert!(spinner.is_some());
        assert!(buf.windows(4).any(|w| w == b"[too"));
    }

    #[test]
    fn apply_event_done_finishes() {
        let mut r = piped(60);
        let mut spinner = Some(SpinnerState::new(SpinnerMode::Thinking));
        let mut buf: Vec<u8> = Vec::new();
        r.on_chunk(&mut buf, "x").unwrap();
        let ev = AgentEvent::Done { text: "x".into() };
        apply_event(&mut r, &mut spinner, &mut buf, &ev).unwrap();
        assert!(spinner.is_none());
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("╯"), "footer printed on Done");
    }
}
