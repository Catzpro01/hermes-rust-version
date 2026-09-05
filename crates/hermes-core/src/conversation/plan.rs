//! Spec 009 (Ticket 02) — structured plan parsing.
//!
//! A plan is a step list the model produces inside a delimited block. It is
//! **in-memory only** (never persisted, never a `Turn` variant / db role),
//! consistent with ADR 0003 and the no-fake-user invariant.
//!
//! The delimiters are bracket-paired (not XML) so they are disjoint from the
//! `<tool_call>` tag parser and can never be confused with it.

use crate::conversation::context::estimate_tokens;

/// Opening delimiter of a plan block.
pub const PLAN_OPEN: &str = "[[plan]]";
/// Closing delimiter of a plan block.
pub const PLAN_CLOSE: &str = "[[/plan]]";

/// A parsed step-by-step plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The inner text between the delimiters (trimmed), used for display and
    /// token accounting.
    raw: String,
    /// Non-empty, trimmed lines of the plan body.
    steps: Vec<String>,
}
impl Plan {
    pub fn steps(&self) -> &[String] {
        &self.steps
    }
    pub fn raw(&self) -> &str {
        &self.raw
    }
    /// Estimated tokens the plan contributes to the context budget.
    pub fn tokens(&self) -> usize {
        estimate_tokens(&self.raw)
    }
    /// A re-suppliable rendering of the active plan (sent as an ephemeral
    /// instruction so the model keeps it in view while executing).
    pub fn instruction_text(&self) -> String {
        if self.steps.is_empty() {
            return self.raw.clone();
        }
        let mut out = String::from("Active plan:\n");
        for (i, step) in self.steps.iter().enumerate() {
            out.push_str(&format!("{}. {step}\n", i + 1));
        }
        out
    }
}

/// Parses a plan block from `response`. Returns `Some(plan)` only when both
/// delimiters appear (open before close); otherwise `None`. The body is split
/// into non-empty trimmed lines (steps). Byte offsets of the ASCII delimiters
/// always fall on character boundaries, so the slice is safe for multi-byte
/// content between the markers.
pub fn parse_plan(response: &str) -> Option<Plan> {
    let open = response.find(PLAN_OPEN)?;
    let body_start = open + PLAN_OPEN.len();
    let close = response[body_start..].find(PLAN_CLOSE)?;
    let body_end = body_start + close;
    let body = response[body_start..body_end].trim();
    let steps: Vec<String> = body
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect();
    Some(Plan {
        raw: body.to_owned(),
        steps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_numbered_plan_into_steps() {
        let plan = parse_plan(
            "Sure. [[plan]]\n1. list files\n2. read config\n[[/plan]] done",
        )
        .unwrap();
        assert_eq!(plan.raw(), "1. list files\n2. read config");
        assert_eq!(plan.steps(), &["1. list files", "2. read config"]);
        assert_eq!(plan.tokens(), estimate_tokens("1. list files\n2. read config"));
    }

    #[test]
    fn missing_markers_yield_no_plan() {
        assert_eq!(parse_plan("just an answer, no plan"), None);
        // Only an opening marker, no close -> not a plan.
        assert_eq!(parse_plan("[[plan]] unfinished"), None);
    }

    #[test]
    fn quoted_user_plan_marker_is_not_an_output_plan() {
        // A response that merely quotes user text containing [[plan]] but has no
        // enclosing close marker must not parse as a plan.
        assert_eq!(parse_plan("the user wrote [[plan]] in their prompt"), None);
    }

    #[test]
    fn empty_body_plan_is_allowed_with_no_steps() {
        let plan = parse_plan("[[plan]]   [[/plan]]").unwrap();
        assert!(plan.steps().is_empty());
        assert_eq!(plan.raw(), "");
    }

    #[test]
    fn multi_byte_content_between_markers_is_char_safe() {
        // CJK between the ASCII markers must not panic and is preserved.
        let plan = parse_plan("[[plan]]\n你 你\n读文件\n[[/plan]]").unwrap();
        assert_eq!(plan.steps().len(), 2);
        assert!(plan.steps().iter().any(|s| s.contains('你')));
        assert!(plan.raw().contains('你'));
    }

    #[test]
    fn literal_tool_tag_inside_plan_does_not_break_parsing() {
        // A plan that mentions a tool name is still a plan; parsing only looks
        // for the bracket delimiters, never XML.
        let plan = parse_plan(
            "[[plan]]\ncall read_file on the path\n[[/plan]]",
        )
        .unwrap();
        assert_eq!(plan.steps(), &["call read_file on the path"]);
    }

    #[test]
    fn instruction_text_numbers_the_steps() {
        let plan = parse_plan("[[plan]]\na\nb\n[[/plan]]").unwrap();
        let text = plan.instruction_text();
        assert!(text.contains("Active plan:"));
        assert!(text.contains("1. a"));
        assert!(text.contains("2. b"));
    }
}
