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
}
