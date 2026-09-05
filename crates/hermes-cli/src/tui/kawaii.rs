//! Spec 013 — Hermes Python UI parity, Ticket 04: spinner & kawaii constants.
//!
//! Generated **byte-for-byte** from the Python Hermes source (canonical
//! truth, Ticket 01 archaeology):
//! * `agent/display.py` — `KawaiiSpinner.SPINNERS["dots"]`, `KAWAII_WAITING`,
//!   `KAWAII_THINKING`, `THINKING_VERBS`, and the 120 ms tick.
//! * `cli.py` — the verbatim reasoning tag set (`_OPEN_TAGS`/`_CLOSE_TAGS`).
//!
//! Do not hand-edit; regenerate with `gen_kawaii.py` on the Hermes VM.
//! The generated tests pin every value.

/// Braille `dots` spinner frames (spec §7; default `spinner_type="dots"`).
pub const DOTS: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Kawaii faces shown while *waiting* (approval prompts etc., spec §7).
pub const KAWAII_WAITING: [&str; 10] = [
    "(｡◕‿◕｡)",
    "(◕‿◕✿)",
    "٩(◕‿◕｡)۶",
    "(✿◠‿◠)",
    "( ˘▽˘)っ",
    "♪(´ε` )",
    "(◕ᴗ◕✿)",
    "ヾ(＾∇＾)",
    "(≧◡≦)",
    "(★ω★)",
];

/// Kawaii faces shown while *thinking* (tool activity, spec §7).
pub const KAWAII_THINKING: [&str; 15] = [
    "(｡•́︿•̀｡)",
    "(◔_◔)",
    "(¬‿¬)",
    "( •_•)>⌐■-■",
    "(⌐■_■)",
    "(´･_･`)",
    "◉_◉",
    "(°ロ°)",
    "( ˘⌣˘)♡",
    "ヽ(>∀<☆)☆",
    "٩(๑❛ᴗ❛๑)۶",
    "(⊙_⊙)",
    "(¬_¬)",
    "( ͡° ͜ʖ ͡°)",
    "ಠ_ಠ",
];

/// Thinking verbs for the spinner message (spec §7).
pub const THINKING_VERBS: [&str; 15] = [
    "pondering",
    "contemplating",
    "musing",
    "cogitating",
    "ruminating",
    "deliberating",
    "mulling",
    "reflecting",
    "processing",
    "reasoning",
    "analyzing",
    "computing",
    "synthesizing",
    "formulating",
    "brainstorming",
];

/// Spinner tick interval in ms (`time.sleep(0.12)` — validated by the generator).
pub const TICK_MS: u64 = 120;
/// Reasoning open tags, verbatim from `cli.py` `_OPEN_TAGS`.
pub const REASONING_OPEN_TAGS: [&str; 6] = [
    "<REASONING_SCRATCHPAD>",
    "<think>",
    "<reasoning>",
    "<THINKING>",
    "<thinking>",
    "<thought>",
];

/// Reasoning close tags, verbatim from `cli.py` `_CLOSE_TAGS`.
pub const REASONING_CLOSE_TAGS: [&str; 6] = [
    "</REASONING_SCRATCHPAD>",
    "</think>",
    "</reasoning>",
    "</THINKING>",
    "</thinking>",
    "</thought>",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dots_frames_are_verbatim() {
        assert_eq!(DOTS.len(), 10);
        assert_eq!(DOTS[0], "⠋");
        assert_eq!(DOTS[9], "⠏");
        assert_eq!(DOTS.iter().copied().collect::<String>(), "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏");
    }

    #[test]
    fn kawaii_lists_are_verbatim() {
        assert_eq!(KAWAII_WAITING.len(), 10);
        assert_eq!(KAWAII_THINKING.len(), 15);
        assert_eq!(THINKING_VERBS.len(), 15);
        assert_eq!(
            KAWAII_WAITING.iter().copied().collect::<String>(),
            "(｡◕‿◕｡)(◕‿◕✿)٩(◕‿◕｡)۶(✿◠‿◠)( ˘▽˘)っ♪(´ε` )(◕ᴗ◕✿)ヾ(＾∇＾)(≧◡≦)(★ω★)"
        );
        assert_eq!(KAWAII_THINKING.iter().copied().collect::<String>(), "(｡•́︿•̀｡)(◔_◔)(¬‿¬)( •_•)>⌐■-■(⌐■_■)(´･_･`)◉_◉(°ロ°)( ˘⌣˘)♡ヽ(>∀<☆)☆٩(๑❛ᴗ❛๑)۶(⊙_⊙)(¬_¬)( ͡° ͜ʖ ͡°)ಠ_ಠ");
        assert_eq!(THINKING_VERBS.iter().copied().collect::<String>(), "ponderingcontemplatingmusingcogitatingruminatingdeliberatingmullingreflectingprocessingreasoninganalyzingcomputingsynthesizingformulatingbrainstorming");
    }

    #[test]
    fn reasoning_tags_are_verbatim() {
        assert_eq!(REASONING_OPEN_TAGS.len(), 6);
        assert_eq!(REASONING_CLOSE_TAGS.len(), 6);
        for (open, close) in REASONING_OPEN_TAGS.iter().zip(REASONING_CLOSE_TAGS.iter()) {
            assert_eq!(
                close,
                &format!("</{}>", &open[1..open.len() - 1]),
                "close tag must mirror open tag {open}"
            );
        }
        assert!(REASONING_OPEN_TAGS.contains(&"<REASONING_SCRATCHPAD>"));
        assert!(REASONING_OPEN_TAGS.contains(&"<think>"));
        assert!(REASONING_CLOSE_TAGS.contains(&"</REASONING_SCRATCHPAD>"));
    }
}
