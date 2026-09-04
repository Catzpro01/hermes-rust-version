//! Deterministic, advisory-only context-size estimation.
//!
//! There is no tokenizer in this codebase and `models` values are free-form
//! `serde_yaml::Value`, so we expose a conservative, char-based heuristic
//! (`text.len() / 4`) that overestimates CJK and is safe for English. These
//! helpers exist purely to *advise* callers whether a request may exceed a
//! provider's `context_length`; they never block a request.

use crate::conversation::Turn;

/// Conservative estimate of the number of tokens in `text`.
///
/// Heuristic: `text.len() / 4` (safe for English; an overestimate for CJK
/// where one character often maps to one token). This is NOT a real tokenizer
/// — only an advisory. The divisor is explicit so tests can pin it; changing it
/// is a deliberate, test-visible act.
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() / 4).max(1)
}

/// Estimated total tokens across an entire conversation (`&[Turn]`). Tool
/// turns count both their tool name and their content.
pub fn estimate_turns_tokens(turns: &[Turn]) -> usize {
    turns
        .iter()
        .map(|turn| match turn {
            Turn::User { content } => estimate_tokens(content),
            Turn::Assistant { content } => estimate_tokens(content),
            Turn::Tool { name, content } => estimate_tokens(name) + estimate_tokens(content),
        })
        .sum()
}

/// Advisory check: does the estimated context exceed `context_length`?
///
/// Returns `Some(warning)` when the estimate is over the limit, `None` when it
/// is within it. `context_length = None` means "no limit known" and **skips**
/// the check entirely (backward compatible with configs that omit it). This is
/// non-blocking by design: the caller decides whether to warn or act.
pub fn check_context_limit(turns: &[Turn], context_length: Option<u64>) -> Option<String> {
    let limit = context_length?;
    let estimated = estimate_turns_tokens(turns) as u64;
    if estimated > limit {
        Some(format!(
            "⚠ Estimated context ({estimated} tokens) exceeds provider limit ({limit}). \
             Response may be truncated."
        ))
    } else {
        None
    }
}

/// Max turns included verbatim in a [`summarize_dropped`] digest.
const SUMMARY_MAX_TURNS: usize = 3;
/// Max characters of a turn's leading text kept in the digest.
const SUMMARY_MAX_CHARS: usize = 100;

/// Deterministic, advisory summary of turns that the sliding window dropped.
///
/// Heuristic only (no LLM): for the first [`SUMMARY_MAX_TURNS`] dropped turns,
/// keep a short lead (`role: <first line, truncated>`), then report how many
/// further turns were dropped. Tool turns are summarized by name only.
///
/// **This is NOT injected into the LLM context.** Representing a dropped
/// conversation as a message with a role not present in [`Turn`] (e.g. a
/// "summary"/"system" role) is deferred until a formal ADR decides the
/// representation, `state.db` role column, provider mapping, and STRIDE. This
/// helper only feeds human-facing display (e.g. the REPL `/info`), so it must
/// never be mistaken for an authentic user/assistant message.
pub fn summarize_dropped(dropped: &[Turn]) -> String {
    if dropped.is_empty() {
        return String::new();
    }
    let summaries: Vec<String> = dropped
        .iter()
        .take(SUMMARY_MAX_TURNS)
        .map(|turn| {
            let (role, text) = match turn {
                Turn::User { content } => ("User", content.as_str()),
                Turn::Assistant { content } => ("Asst", content.as_str()),
                Turn::Tool { name, .. } => ("Tool", name.as_str()),
            };
            format!("{role}: {}", first_line_truncated(text, SUMMARY_MAX_CHARS))
        })
        .collect();
    let remaining = dropped.len().saturating_sub(SUMMARY_MAX_TURNS);
    let suffix = if remaining > 0 {
        format!(" (+{remaining} more)")
    } else {
        String::new()
    };
    format!(
        "[{} turns dropped] {}{}",
        dropped.len(),
        summaries.join(" | "),
        suffix
    )
}

/// First line of `text`, truncated to `max_chars` characters. Uses char-safe
/// slicing so a multi-byte (e.g. CJK) boundary never panics; a truncated value
/// gets a trailing ellipsis.
fn first_line_truncated(text: &str, max_chars: usize) -> String {
    let first_line = text.lines().next().unwrap_or("");
    let lead: String = first_line.chars().take(max_chars).collect();
    if first_line.chars().count() > max_chars {
        format!("{lead}…")
    } else {
        lead
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(content: &str) -> Turn {
        Turn::User {
            content: content.into(),
        }
    }
    fn a(content: &str) -> Turn {
        Turn::Assistant {
            content: content.into(),
        }
    }
    fn tool(name: &str, content: &str) -> Turn {
        Turn::Tool {
            name: name.into(),
            content: content.into(),
        }
    }

    #[test]
    fn divisor_is_exactly_four() {
        // Pin the constant: 40 chars / 4 = 10 tokens.
        assert_eq!(estimate_tokens("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"), 10);
        // Sub-divisor fragments round down but never below 1 token.
        assert_eq!(estimate_tokens(""), 1);
        assert_eq!(estimate_tokens("a"), 1);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcdefgh"), 2);
    }

    #[test]
    fn turns_sum_content_and_tool_name() {
        let turns = [u("a".repeat(40).as_str()), a("b".repeat(20).as_str())];
        // 10 + 5 = 15
        assert_eq!(estimate_turns_tokens(&turns), 15);
    }

    #[test]
    fn tool_turn_counts_name_and_content() {
        let turns = [tool("read_file", "c".repeat(40).as_str())];
        // read_file = 9 chars/4=2, content 40/4=10 -> 12
        assert_eq!(estimate_turns_tokens(&turns), 12);
    }

    #[test]
    fn empty_turns_are_zero_tokens() {
        assert_eq!(estimate_turns_tokens(&[]), 0);
    }

    #[test]
    fn none_limit_skips_the_check_entirely() {
        // Backward compatible: absent context_length means "no limit", so no
        // warning is produced even for a very large conversation.
        let big = [u(&"x".repeat(100_000))];
        assert_eq!(check_context_limit(&big, None), None);
    }

    #[test]
    fn over_limit_yields_a_warning_naming_both_numbers() {
        let big = [u(&"y".repeat(80))]; // 80/4 = 20 estimated tokens
        let warning = check_context_limit(&big, Some(10)).expect("should warn");
        assert!(warning.contains("20"), "must name estimate: {warning}");
        assert!(warning.contains("10"), "must name limit: {warning}");
    }

    #[test]
    fn within_limit_yields_no_warning() {
        let small = [u(&"z".repeat(40))]; // 40/4 = 10 estimated tokens
        assert_eq!(check_context_limit(&small, Some(10)), None);
    }

    #[test]
    fn equal_to_limit_is_not_over() {
        let turns = [u(&"w".repeat(40))]; // 10 estimated == limit 10 -> safe
        assert_eq!(check_context_limit(&turns, Some(10)), None);
    }

    #[test]
    fn summarize_empty_returns_empty() {
        assert_eq!(summarize_dropped(&[]), "");
    }

    #[test]
    fn summarize_one_turn_labels_role_and_lead() {
        let s = summarize_dropped(&[u("hello world this is long content")]);
        assert!(s.contains("[1 turns dropped]"), "got: {s}");
        assert!(s.contains("User: hello world"), "got: {s}");
    }

    #[test]
    fn summarize_keeps_up_to_three_turns() {
        let turns = vec![u("one"), a("two"), tool("read_file", "body"), u("four")];
        let s = summarize_dropped(&turns);
        assert!(s.contains("[4 turns dropped]"), "got: {s}");
        assert!(s.contains("User: one"), "got: {s}");
        assert!(s.contains("Asst: two"), "got: {s}");
        assert!(s.contains("Tool: read_file"), "tool summarized by name: {s}");
        // 4th turn (index 3) is beyond the max 3 shown -> counted in "+N more".
        assert!(s.contains("(+1 more)"), "got: {s}");
        assert!(!s.contains("four"), "4th turn content must not be shown: {s}");
    }

    #[test]
    fn summarize_truncates_long_content_with_ellipsis() {
        let long = "x".repeat(250);
        let s = summarize_dropped(&[u(&long)]);
        assert!(s.contains('…'), "must truncate with ellipsis: {s}");
        assert!(!s.contains(&long), "must not include the full long content: {s}");
    }

    #[test]
    fn summarize_truncation_is_char_safe_for_multibyte() {
        // 120 CJK chars (> 100) - must not panic slicing mid-byte.
        let long = "你".repeat(120);
        let s = summarize_dropped(&[u(&long)]);
        assert!(s.contains('…'), "must truncate: {s}");
        // The truncated lead is at most 100 chars + ellipsis, never split bytes.
        assert!(s.contains('你'), "must preserve valid chars: {s}");
    }

    #[test]
    fn summarize_first_line_only() {
        let multi = "first line\nsecond line that should not appear";
        let s = summarize_dropped(&[u(multi)]);
        assert!(s.contains("first line"), "got: {s}");
        assert!(!s.contains("second line"), "only first line kept: {s}");
    }
}
