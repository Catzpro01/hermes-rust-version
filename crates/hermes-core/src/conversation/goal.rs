//! Spec 009 — goal extraction & tracking (Ticket 01).
//!
//! Goal state is **in-memory, per-runner** and advisory: it is never persisted
//! to `state.db` and never introduces a new role/`Turn` variant, consistent
//! with ADR 0003 (no new persisted role without a dedicated ADR). Default is
//! inactive so `ConversationRunner` behaves exactly as before when goal
//! tracking is off (zero regression).

/// Lifecycle of the active goal, updated across agentic iterations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalStatus {
    /// No goal recorded yet.
    NotStarted,
    /// A goal exists and work toward it is ongoing.
    InProgress,
    /// The goal has been reached (set by a later reflection/closure ticket).
    Achieved,
    /// The goal cannot be met (e.g. retries exhausted, denied tool).
    Blocked,
}
impl GoalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not started",
            Self::InProgress => "in progress",
            Self::Achieved => "achieved",
            Self::Blocked => "blocked",
        }
    }
}

/// Upper bound (chars) on the goal text kept by [`extract_goal`], so a long
/// prompt does not balloon into the tracked goal. Char-counted to stay safe on
/// multi-byte text.
pub const GOAL_MAX_CHARS: usize = 120;

/// Deterministic, advisory extraction of a short goal statement from the first
/// turn of a prompt. Heuristic only: returns the first non-empty line trimmed,
/// capped at [`GOAL_MAX_CHARS`] characters using char-safe slicing (a
/// multi-byte boundary never panics; a truncated value gets an ellipsis).
pub fn extract_goal(content: &str) -> String {
    let first_line = content.lines().next().unwrap_or("").trim();
    let lead: String = first_line.chars().take(GOAL_MAX_CHARS).collect();
    if first_line.chars().count() > GOAL_MAX_CHARS {
        format!("{lead}…")
    } else {
        lead
    }
}

/// In-memory goal state for one `ConversationRunner`.
#[derive(Debug, Clone)]
pub struct GoalTracker {
    goal: Option<String>,
    status: GoalStatus,
    /// Whether new user turns should auto-record a goal when none exists.
    tracking: bool,
}
impl GoalTracker {
    pub fn new() -> Self {
        Self {
            goal: None,
            status: GoalStatus::NotStarted,
            tracking: false,
        }
    }
    /// Enables/disables auto-recording a goal from the initiating user turn.
    pub fn set_tracking(&mut self, on: bool) {
        self.tracking = on;
    }
    pub fn tracking(&self) -> bool {
        self.tracking
    }
    pub fn goal(&self) -> Option<&str> {
        self.goal.as_deref()
    }
    pub fn status(&self) -> GoalStatus {
        self.status
    }
    /// Records `goal` as the active goal and marks it in progress. Returns
    /// `false` and leaves state untouched if `goal` is empty.
    pub fn record(&mut self, goal: String) -> bool {
        if goal.trim().is_empty() {
            return false;
        }
        self.goal = Some(goal);
        self.status = GoalStatus::InProgress;
        true
    }
    /// Auto-record the goal from the initiating user prompt, but only when
    /// tracking is enabled and no goal is set yet (the first user turn wins).
    pub fn record_if_tracking_empty(&mut self, content: &str) {
        if self.tracking && self.goal.is_none() {
            let extracted = extract_goal(content);
            let _ = self.record(extracted);
        }
    }
    pub fn set_status(&mut self, status: GoalStatus) {
        // Status changes are only meaningful while a goal is active.
        if self.goal.is_some() {
            self.status = status;
        }
    }
    pub fn reset(&mut self) {
        self.goal = None;
        self.status = GoalStatus::NotStarted;
    }
}
impl Default for GoalTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_takes_first_line_and_trims() {
        let goal = extract_goal("  build a report  \nsecond line ignored");
        assert_eq!(goal, "build a report");
    }

    #[test]
    fn extract_is_char_safe_and_truncates_long_cjk() {
        // 200 CJK chars > GOAL_MAX_CHARS(120) — must not panic mid-byte and
        // must be truncated with an ellipsis.
        let long = "你".repeat(200);
        let goal = extract_goal(&long);
        assert!(goal.contains('…'), "must truncate: {goal}");
        assert!(goal.contains('你'), "must keep valid chars: {goal}");
        assert!(goal.chars().count() <= GOAL_MAX_CHARS + 1, "over budget: {}", goal.chars().count());
    }

    #[test]
    fn tracker_defaults_to_not_started_and_no_goal() {
        let t = GoalTracker::new();
        assert_eq!(t.status(), GoalStatus::NotStarted);
        assert_eq!(t.goal(), None);
        assert!(!t.tracking());
    }

    #[test]
    fn record_sets_goal_and_marks_in_progress() {
        let mut t = GoalTracker::new();
        assert!(t.record("fetch the data".into()));
        assert_eq!(t.goal(), Some("fetch the data"));
        assert_eq!(t.status(), GoalStatus::InProgress);
        // Empty goal is rejected without changing state.
        assert!(!t.record("   ".into()));
        assert_eq!(t.goal(), Some("fetch the data"));
    }

    #[test]
    fn status_changes_only_when_a_goal_is_active() {
        let mut t = GoalTracker::new();
        t.set_status(GoalStatus::Achieved); // no goal yet -> ignored
        assert_eq!(t.status(), GoalStatus::NotStarted);
        t.record("goal".into());
        t.set_status(GoalStatus::Blocked);
        assert_eq!(t.status(), GoalStatus::Blocked);
        t.set_status(GoalStatus::Achieved);
        assert_eq!(t.status(), GoalStatus::Achieved);
    }

    #[test]
    fn reset_clears_goal_and_status() {
        let mut t = GoalTracker::new();
        t.record("x".into());
        t.set_status(GoalStatus::InProgress);
        t.reset();
        assert_eq!(t.goal(), None);
        assert_eq!(t.status(), GoalStatus::NotStarted);
    }

    #[test]
    fn record_if_tracking_empty_only_fires_when_enabled_and_empty() {
        let mut t = GoalTracker::new();
        // Disabled: no record.
        t.record_if_tracking_empty("hello goal");
        assert_eq!(t.goal(), None);
        // Enabled and empty -> records first user turn.
        t.set_tracking(true);
        t.record_if_tracking_empty("first user turn");
        assert_eq!(t.goal(), Some("first user turn"));
        // Already set -> a later turn does not overwrite (first wins).
        t.record_if_tracking_empty("second turn ignored");
        assert_eq!(t.goal(), Some("first user turn"));
    }

    #[test]
    fn status_strings_are_covered() {
        assert_eq!(GoalStatus::NotStarted.as_str(), "not started");
        assert_eq!(GoalStatus::InProgress.as_str(), "in progress");
        assert_eq!(GoalStatus::Achieved.as_str(), "achieved");
        assert_eq!(GoalStatus::Blocked.as_str(), "blocked");
    }
}
