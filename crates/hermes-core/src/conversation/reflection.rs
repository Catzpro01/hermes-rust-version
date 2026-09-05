//! Spec 009 (Ticket 03) — self-reflection gate.
//!
//! After each tool result the runner decides on-plan vs off-plan vs blocked.
//! The **default is a deterministic heuristic** over the tool outcome and goal
//! status — it never relies on an LLM judging itself (a model can hallucinate
//! "on track" while the tool failed). An optional LLM reflection round-trip
//! exists but is off by default and consumes an iteration.

use crate::conversation::goal::{GoalStatus, GoalTracker};
use crate::tools::ToolExecutionStatus;

/// Max reflection count per plan step before the step is treated as blocked
/// (anti-loop). Small and explicit so it is test-pinnable.
pub const MAX_REFLECTIONS: usize = 2;

/// Deterministic verdict after a tool result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Tool succeeded / goal in progress — keep going on-plan.
    OnPlan,
    /// Tool failed in a retryable way with budget left — recover (Ticket 04).
    OffPlan,
    /// Denied, or retries exhausted — do not retry; the step/goal is blocked.
    Blocked,
}

/// The bounded reflection gate for one runner. In-memory only; never persisted.
#[derive(Debug, Clone)]
pub struct ReflectionTracker {
    /// Whether reflection is enabled (heuristic + optional LLM). Off by default
    /// so reactive mode is unchanged (zero regression).
    enabled: bool,
    /// Reflections consumed for the current plan step.
    reflections_used: usize,
}
impl ReflectionTracker {
    pub fn new() -> Self {
        Self {
            enabled: false,
            reflections_used: 0,
        }
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn set_enabled(&mut self, on: bool) {
        self.enabled = on;
        self.reflections_used = 0;
    }
    pub fn reflections_used(&self) -> usize {
        self.reflections_used
    }
    pub fn reset_step(&mut self) {
        self.reflections_used = 0;
    }
}
impl Default for ReflectionTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Deterministic heuristic: classify a tool outcome against the goal.
///
/// - `Success` (goal in progress) → [`Verdict::OnPlan`].
/// - `Error`/`Timeout` with retries left → [`Verdict::OffPlan`] (recover).
/// - `Error`/`Timeout` with no retries left → [`Verdict::Blocked`].
/// - `Denied` → [`Verdict::Blocked`] always (a user denial is never retried;
///   reflection cannot override it — Spec 002 security invariant).
/// - `Cancelled` → [`Verdict::OnPlan`] is wrong (the turn is being torn down);
///   it is surfaced as [`Verdict::Blocked`] here only for completeness; the
///   runner handles cancellation separately before it reaches this helper.
pub fn verdict(status: ToolExecutionStatus, retries_remaining: bool) -> Verdict {
    match status {
        ToolExecutionStatus::Success => Verdict::OnPlan,
        ToolExecutionStatus::Denied => Verdict::Blocked,
        ToolExecutionStatus::Cancelled => Verdict::Blocked,
        ToolExecutionStatus::Error | ToolExecutionStatus::Timeout => {
            if retries_remaining {
                Verdict::OffPlan
            } else {
                Verdict::Blocked
            }
        }
    }
}

/// Whether this verdict calls for the LLM reflection round-trip (gated).
pub fn needs_llm_reflection(v: Verdict) -> bool {
    matches!(v, Verdict::OffPlan)
}

/// Ephemeral instruction for the optional LLM reflection round-trip.
pub fn reflect_instruction() -> &'static str {
    "Evaluate how much of the active plan has been completed so far. Report \
     whether the current step succeeded, whether the goal is achieved, and \
     whether any step is blocked."
}

/// Applies the heuristic verdict to the goal lifecycle: a `Blocked` verdict
/// marks the goal `Blocked`; an `OffPlan` verdict leaves it `InProgress` so the
/// caller can recover (Ticket 04). Returns the verdict. Never runs when
/// reflection is disabled (returns `OnPlan` unchanged / no side effect).
pub fn apply_verdict(
    tracker: &mut ReflectionTracker,
    status: ToolExecutionStatus,
    retries_remaining: bool,
    goal: &mut GoalTracker,
) -> Verdict {
    if !tracker.enabled {
        return Verdict::OnPlan;
    }
    let v = verdict(status, retries_remaining);
    if v == Verdict::OffPlan {
        tracker.reflections_used += 1;
        // Anti-loop: reflecting on the same step too many times marks it blocked.
        if tracker.reflections_used > MAX_REFLECTIONS {
            goal.set_status(GoalStatus::Blocked);
            return Verdict::Blocked;
        }
    }
    if v == Verdict::Blocked {
        goal.set_status(GoalStatus::Blocked);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::goal::GoalStatus;

    fn g(goal: &mut GoalTracker, s: &str) {
        goal.record(s.into());
        goal.set_status(GoalStatus::InProgress);
    }

    #[test]
    fn success_is_on_plan() {
        assert_eq!(verdict(ToolExecutionStatus::Success, true), Verdict::OnPlan);
        assert_eq!(verdict(ToolExecutionStatus::Success, false), Verdict::OnPlan);
    }

    #[test]
    fn denied_is_never_retried() {
        assert_eq!(verdict(ToolExecutionStatus::Denied, true), Verdict::Blocked);
        assert_eq!(verdict(ToolExecutionStatus::Denied, false), Verdict::Blocked);
    }

    #[test]
    fn error_timeout_are_off_plan_only_with_retries_left() {
        assert_eq!(verdict(ToolExecutionStatus::Error, true), Verdict::OffPlan);
        assert_eq!(verdict(ToolExecutionStatus::Timeout, true), Verdict::OffPlan);
        assert_eq!(verdict(ToolExecutionStatus::Error, false), Verdict::Blocked);
        assert_eq!(verdict(ToolExecutionStatus::Timeout, false), Verdict::Blocked);
    }

    #[test]
    fn disabled_reflection_has_no_side_effect() {
        let mut t = ReflectionTracker::new();
        let mut goal = GoalTracker::new();
        g(&mut goal, "task");
        let v = apply_verdict(&mut t, ToolExecutionStatus::Denied, true, &mut goal);
        assert_eq!(v, Verdict::OnPlan, "disabled -> no verdict applied");
        assert_eq!(goal.status(), GoalStatus::InProgress, "goal must not be blocked");
        assert_eq!(t.reflections_used(), 0);
    }

    #[test]
    fn denied_marks_goal_blocked_when_enabled() {
        let mut t = ReflectionTracker::new();
        t.set_enabled(true);
        let mut goal = GoalTracker::new();
        g(&mut goal, "task");
        let v = apply_verdict(&mut t, ToolExecutionStatus::Denied, true, &mut goal);
        assert_eq!(v, Verdict::Blocked);
        assert_eq!(goal.status(), GoalStatus::Blocked);
    }

    #[test]
    fn off_plan_increments_until_blocked_by_anti_loop() {
        let mut t = ReflectionTracker::new();
        t.set_enabled(true);
        let mut goal = GoalTracker::new();
        g(&mut goal, "task");
        // Each OffPlan with retries increments; goal stays in progress until the
        // anti-loop cap is crossed, then it becomes blocked.
        assert_eq!(apply_verdict(&mut t, ToolExecutionStatus::Error, true, &mut goal), Verdict::OffPlan);
        assert_eq!(t.reflections_used(), 1);
        assert_eq!(goal.status(), GoalStatus::InProgress);
        assert_eq!(apply_verdict(&mut t, ToolExecutionStatus::Timeout, true, &mut goal), Verdict::OffPlan);
        assert_eq!(t.reflections_used(), 2);
        assert_eq!(goal.status(), GoalStatus::InProgress);
        // Third reflection on the same step crosses MAX_REFLECTIONS(2) -> blocked.
        assert_eq!(apply_verdict(&mut t, ToolExecutionStatus::Error, true, &mut goal), Verdict::Blocked);
        assert_eq!(goal.status(), GoalStatus::Blocked);
    }

    #[test]
    fn error_without_retries_is_blocked() {
        let mut t = ReflectionTracker::new();
        t.set_enabled(true);
        let mut goal = GoalTracker::new();
        g(&mut goal, "task");
        assert_eq!(apply_verdict(&mut t, ToolExecutionStatus::Error, false, &mut goal), Verdict::Blocked);
        assert_eq!(goal.status(), GoalStatus::Blocked);
    }
}
