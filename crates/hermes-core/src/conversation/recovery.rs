//! Spec 009 (Ticket 04) — error recovery via parameter mutation.
//!
//! After a retryable tool failure (Option 1 from review), the runner records the
//! argument set that failed and surfaces an "already tried" note in the tool
//! result so the model picks *different* parameters rather than repeating the
//! exact same request. Identical repeats are tracked and bounded.

use std::collections::HashMap;

/// Max retries per tool before the step is blocked (aligned with the bounded
/// [`crate::provider::RetryPolicy`] default of 3 attempts). Explicit constant.
pub const MAX_RETRIES: usize = 3;

/// Deterministic 64-bit FNV-1a over the trimmed argument string.
///
/// This is a deliberate deviation from a sha256 recommendation: there is no
/// hashing crate in the workspace, and we want determinism without adding a
/// dependency. The fingerprint's job is only to detect an *identical repeated*
/// argument set within one run; trimming whitespace canonicalises superficial
/// differences. If a collision-resistant digest is ever required, this is a
/// one-line swap behind the pinned tests below.
pub fn arg_fingerprint(arguments: &str) -> u64 {
    let bytes = arguments.trim().as_bytes();
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Bounded, in-memory record of tool attempts per (tool) for recovery.
#[derive(Debug, Clone, Default)]
pub struct RetryTracker {
    /// tool name -> ordered fingerprints that have been attempted.
    tried: HashMap<String, Vec<u64>>,
    /// tool name -> the raw (trimmed) argument strings attempted, for the
    /// "already tried" note the model sees (Option 1).
    raw: HashMap<String, Vec<String>>,
}
impl RetryTracker {
    pub fn new() -> Self {
        Self::default()
    }
    /// Whether this exact argument set was already attempted for `tool`.
    pub fn is_tried(&self, tool: &str, arguments: &str) -> bool {
        self.tried
            .get(tool)
            .map(|v| v.contains(&arg_fingerprint(arguments)))
            .unwrap_or(false)
    }
    /// Records an attempted argument set for `tool`. Returns `false` when it is
    /// an exact repeat (already tried), `true` when new.
    pub fn record(&mut self, tool: &str, arguments: &str) -> bool {
        let fp = arg_fingerprint(arguments);
        let entry = self.tried.entry(tool.to_owned()).or_default();
        if entry.contains(&fp) {
            return false;
        }
        entry.push(fp);
        self.raw.entry(tool.to_owned()).or_default().push(arguments.trim().to_owned());
        true
    }
    /// Number of attempts recorded for `tool`.
    pub fn attempts(&self, tool: &str) -> usize {
        self.tried.get(tool).map(Vec::len).unwrap_or(0)
    }
    /// Whether more retries remain for `tool` (attempts < [`MAX_RETRIES`]).
    pub fn can_retry(&self, tool: &str) -> bool {
        self.attempts(tool) < MAX_RETRIES
    }
    /// The raw argument sets already tried for `tool` (for the Option 1 note).
    pub fn tried_args(&self, tool: &str) -> &[String] {
        self.raw.get(tool).map(Vec::as_slice).unwrap_or(&[])
    }
    /// Builds the Option 1 annotation appended to a failed tool result so the
    /// model sees what has already been tried and picks different parameters.
    pub fn already_tried_note(&self, tool: &str) -> Option<String> {
        let args = self.tried_args(tool);
        if args.is_empty() {
            return None;
        }
        let joined = args
            .iter()
            .map(|a| {
                let preview: String = a.chars().take(80).collect();
                preview
            })
            .collect::<Vec<_>>()
            .join(" | ");
        Some(format!("[already tried {count} parameter set(s): {joined}]", count = args.len()))
    }
    /// Resets per-step recovery state (a new task / step).
    pub fn reset(&mut self) {
        self.tried.clear();
        self.raw.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_deterministic_and_whitespace_insensitive() {
        assert_eq!(arg_fingerprint("a"), arg_fingerprint(" a \n"));
        assert_eq!(arg_fingerprint("{\"path\":\"x\"}"), arg_fingerprint("{\"path\":\"x\"}"));
    }

    #[test]
    fn exact_repeat_is_detected_and_not_double_recorded() {
        let mut t = RetryTracker::new();
        assert!(!t.is_tried("read_file", "{\"path\":\"x\"}"));
        assert!(t.record("read_file", "{\"path\":\"x\"}"));
        assert!(t.is_tried("read_file", "{\"path\":\"x\"}"));
        // A different argument set is a new attempt.
        assert!(!t.is_tried("read_file", "{\"path\":\"y\"}"));
        assert!(t.record("read_file", "{\"path\":\"y\"}"));
        assert_eq!(t.attempts("read_file"), 2);
        // Re-recording an exact repeat returns false (not double counted).
        assert!(!t.record("read_file", "{\"path\":\"x\"}"));
        assert_eq!(t.attempts("read_file"), 2);
    }

    #[test]
    fn max_retries_bounds_can_retry() {
        let mut t = RetryTracker::new();
        assert!(t.can_retry("tool"));
        for i in 0..MAX_RETRIES {
            t.record("tool", &format!("args-{i}"));
        }
        assert!(!t.can_retry("tool"), "exhausted after MAX_RETRIES distinct attempts");
    }

    #[test]
    fn already_tried_note_lists_attempts_and_none_when_empty() {
        let mut t = RetryTracker::new();
        assert_eq!(t.already_tried_note("tool"), None);
        t.record("tool", "{\"path\":\"/x\"}");
        t.record("tool", "{\"path\":\"/y\"}");
        let note = t.already_tried_note("tool").unwrap();
        assert!(note.contains("already tried 2 parameter set(s)"), "got: {note}");
        assert!(note.contains("/x"));
        assert!(note.contains("/y"));
    }
}
