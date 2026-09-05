//! In-memory conversation state and provider event flow.

use crate::provider::{EventStream, Provider, ProviderError};
/// Ephemeral instruction that asks the model to emit a delimited plan
/// (Spec 009 Ticket 02). Sent only via the instruction channel; never persisted
/// and never a user turn.
const PLAN_INSTRUCTION: &str = "You are a planning assistant. Given the user's \
goal, produce a concise step-by-step plan wrapped in [[plan]]...[[/plan]]. Only \
output the plan.";
use crate::session::{SessionId, SessionStore};
use crate::tools::{ToolCall, ToolCallRecord, ToolError, ToolExecutionStatus, ToolRegistry};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio_util::sync::CancellationToken;

pub mod context;
pub mod goal;
pub mod plan;

use goal::{GoalStatus, GoalTracker};
use plan::Plan;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Turn {
    User { content: String },
    Assistant { content: String },
    Tool { name: String, content: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgenticResult {
    Done { text: String, iterations: usize },
    MaxIterations(usize),
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Started,
    Chunk(String),
    Done,
    Error(String),
    ToolCall(ToolCall),
}

pub struct ConversationRunner<P> {
    provider: P,
    turns: Vec<Turn>,
    /// Advisory context window this runner should stay under, when known
    /// (from the active provider's `context_length`/compression config). `None`
    /// means "no known limit" -> no truncation/warning (backward compatible).
    /// Ticket 01 only reads it for accounting + a non-blocking warning;
    /// truncation (sliding window) is Ticket 02.
    context_limit: Option<u64>,
    /// Indices into `self.turns` that must always be sent (never dropped by the
    /// sliding window). In-memory only, per-session (Ticket 04). Indices refer
    /// to `self.turns` positions; they are cleared when turns are replaced
    /// (`/new`, `/resume`) so stale indices never dangle.
    pinned: HashSet<usize>,
    /// Spec 009 (Ticket 01) — advisory, in-memory goal state. Inactive by
    /// default so behavior is unchanged unless tracking is enabled. Never
    /// persisted; never introduces a new role/Turn variant.
    goal: GoalTracker,
    /// Spec 009 (Ticket 02) — whether the agent is in planned mode. Off by
    /// default so `chat_agentic` stays reactive (zero regression).
    plan_mode: bool,
    /// The active in-memory plan (Ticket 02). `None` until generated.
    plan: Option<Plan>,
}
impl<P: Provider> ConversationRunner<P> {
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            turns: Vec::new(),
            context_limit: None,
            pinned: HashSet::new(),
            goal: GoalTracker::new(),
            plan_mode: false,
            plan: None,
        }
    }
    pub fn turns(&self) -> &[Turn] {
        &self.turns
    }
    pub fn from_turns(provider: P, turns: Vec<Turn>) -> Self {
        Self {
            provider,
            turns,
            context_limit: None,
            pinned: HashSet::new(),
            goal: GoalTracker::new(),
            plan_mode: false,
            plan: None,
        }
    }

    /// Sets the advisory context limit (e.g. from config precedence at REPL
    /// startup or after a `/provider` switch). Does not change stored turns.
    pub fn set_context_limit(&mut self, limit: Option<u64>) {
        self.context_limit = limit;
    }

    /// The advisory context limit this runner is told to respect.
    pub fn context_limit(&self) -> Option<u64> {
        self.context_limit
    }

    /// Estimated tokens across all current turns plus any active in-memory
    /// plan (Spec 009), delegating to the Spec 006 helper so the char/4
    /// heuristic lives in exactly one place.
    pub fn estimated_tokens(&self) -> usize {
        crate::conversation::context::estimate_turns_tokens(&self.turns)
            + self.plan.as_ref().map(Plan::tokens).unwrap_or(0)
    }

    /// Advisory warning when current context is estimated to exceed the limit.
    /// `None` when within limit or when no limit is configured. Never blocks.
    pub fn context_warning(&self) -> Option<String> {
        crate::conversation::context::check_context_limit(&self.turns, self.context_limit)
    }

    /// Pins the turn at `index` (0-based into the current history) so the
    /// sliding window never drops it. Returns `Ok(())` on success, or an error
    /// naming the problem when the index is out of range or already pinned.
    pub fn pin(&mut self, index: usize) -> Result<(), String> {
        if index >= self.turns.len() {
            return Err(format!(
                "no turn at index {index} (session has {} turns)",
                self.turns.len()
            ));
        }
        if !self.pinned.insert(index) {
            return Err(format!("turn {index} is already pinned"));
        }
        Ok(())
    }

    /// Removes the pin on the turn at `index`. Idempotent: returns `Ok(())` if
    /// it was pinned (and now isn't), and an error if there is no such pin or
    /// the index is out of range.
    pub fn unpin(&mut self, index: usize) -> Result<(), String> {
        if index >= self.turns.len() {
            return Err(format!(
                "no turn at index {index} (session has {} turns)",
                self.turns.len()
            ));
        }
        if !self.pinned.remove(&index) {
            return Err(format!("turn {index} is not pinned"));
        }
        Ok(())
    }

    /// Indices of all currently pinned turns, ascending.
    pub fn pinned(&self) -> Vec<usize> {
        let mut v: Vec<usize> = self.pinned.iter().copied().collect();
        v.sort_unstable();
        v
    }

    /// Whether the turn at `index` is currently pinned.
    pub fn is_pinned(&self, index: usize) -> bool {
        self.pinned.contains(&index)
    }

    // -- Spec 009 goal tracking (Ticket 01) --------------------------------

    /// Enables/disables auto-recording a goal from the initiating user turn of
    /// an agentic session.
    pub fn set_goal_tracking(&mut self, on: bool) {
        self.goal.set_tracking(on);
    }
    pub fn goal_tracking(&self) -> bool {
        self.goal.tracking()
    }
    /// The currently tracked goal text, if any.
    pub fn goal(&self) -> Option<&str> {
        self.goal.goal()
    }
    pub fn goal_status(&self) -> GoalStatus {
        self.goal.status()
    }
    /// Records `text` as the active goal (marks it in progress).
    pub fn set_goal(&mut self, text: String) -> bool {
        self.goal.record(text)
    }
    /// Explicitly sets the goal lifecycle status (used by later tickets on
    /// completion/blocking). No-op when no goal is active.
    pub fn set_goal_status(&mut self, status: GoalStatus) {
        self.goal.set_status(status);
    }
    pub fn reset_goal(&mut self) {
        self.goal.reset();
    }
    /// Clears goal/pin state when a session's turns are replaced.
    fn clear_goal(&mut self) {
        self.goal.reset();
    }

    // -- Spec 009 plan-then-execute (Ticket 02) -----------------------------

    /// Enables/disables planned mode. Turning it off clears any active plan so
    /// execution returns to reactive.
    pub fn set_plan_mode(&mut self, on: bool) {
        self.plan_mode = on;
        if !on {
            self.plan = None;
        }
    }
    pub fn plan_mode(&self) -> bool {
        self.plan_mode
    }
    /// The active in-memory plan, if any.
    pub fn plan(&self) -> Option<&Plan> {
        self.plan.as_ref()
    }
    /// Clears the active plan (keeps plan_mode).
    pub fn clear_plan(&mut self) {
        self.plan = None;
    }
    /// An ephemeral re-supply of the active plan (when planned and present) to
    /// carry on execution sends, or `None` otherwise. Never persisted.
    fn active_plan_instruction(&self) -> Option<String> {
        if !self.plan_mode {
            return None;
        }
        self.plan.as_ref().map(Plan::instruction_text)
    }
    /// Generates a plan for the current conversation via one ephemeral
    /// instruction round-trip (Ticket 02). In planned mode only: sends the
    /// current window plus the [`PLAN_INSTRUCTION`], reads the raw assistant
    /// text, and stores the parsed plan. No-op (returns the existing plan)
    /// when not in planned mode. The round-trip shares the caller's iteration
    /// budget and is never persisted.
    pub async fn ensure_plan(
        &mut self,
        cancel: CancellationToken,
    ) -> Result<Option<Plan>, ProviderError> {
        if !self.plan_mode {
            return Ok(self.plan.clone());
        }
        if self.plan.is_some() {
            return Ok(self.plan.clone());
        }
        // A plan needs a task; with no user turn there is nothing to plan.
        if !self
            .turns
            .iter()
            .any(|t| matches!(t, Turn::User { .. }))
        {
            return Ok(None);
        }
        let to_send = self.turns_to_send();
        let mut stream = self
            .provider
            .chat_with_instruction(&to_send, Some(PLAN_INSTRUCTION), cancel)
            .await?;
        let mut text = String::new();
        while let Some(event) = stream.next().await {
            match event {
                Ok(Event::Chunk(chunk)) => text.push_str(&chunk),
                Ok(_) => {}
                Err(ProviderError::Cancelled) => return Err(ProviderError::Cancelled),
                Err(err) => return Err(err),
            }
        }
        // If the model did not emit a delimited plan, fall back to no plan
        // (reactive) rather than erroring.
        if let Some(plan) = crate::conversation::plan::parse_plan(&text) {
            self.plan = Some(plan.clone());
        }
        Ok(self.plan.clone())
    }

    /// Indices (ascending) that the sliding window would send next. Always
    /// includes every pinned index and the most recent turn; then fills from
    /// newest backward while the token budget allows. Sorted ascending so the
    /// emitted context keeps chronological order.
    fn keep_indices(&self) -> Vec<usize> {
        let n = self.turns.len();
        let limit = match self.context_limit {
            None => return (0..n).collect(),
            Some(limit) => limit,
        };
        let turn_tokens = |i: usize| crate::conversation::context::turn_tokens(&self.turns[i]) as u64;

        // Must-keep: every pinned turn + the newest (active) turn.
        let mut keep: Vec<usize> = self
            .pinned
            .iter()
            .copied()
            .filter(|&i| i < n)
            .collect();
        if !keep.contains(&(n - 1)) {
            keep.push(n - 1);
        }
        let mut used: u64 = keep.iter().map(|&i| turn_tokens(i)).sum();

        // Fill from newest toward oldest while the budget allows.
        for i in (0..n).rev() {
            if keep.contains(&i) {
                continue;
            }
            let t = turn_tokens(i);
            if used + t <= limit {
                keep.push(i);
                used += t;
            }
            // Stop filling once a recent turn can't fit: older ones are even
            // less desirable, so leaving them out keeps context most recent.
            if used + t > limit {
                break;
            }
        }
        keep.sort_unstable();
        keep
    }

    /// Returns the turns to send to the provider (Ticket 02 + Ticket 04).
    ///
    /// Sliding-window invariant: this returns a **copy** of a subset trimmed to
    /// fit `context_limit`, and never mutates `self.turns`. Full history stays
    /// in `self.turns` (and `state.db`); only what is handed to the model is
    /// shortened.
    ///
    /// - `context_limit = None` → full history unchanged (backward compatible).
    /// - Pinned turns and the most recent turn are always included; if even
    ///   they alone exceed the budget they are still sent (a warning is emitted
    ///   separately — never dropped). Additional turns are taken from newest to
    ///   oldest until the budget is reached.
    pub fn turns_to_send(&self) -> Vec<Turn> {
        self.keep_indices()
            .into_iter()
            .map(|i| self.turns[i].clone())
            .collect()
    }

    /// The turns the sliding window would drop from the next send, in original
    /// order (complement of what is sent). Empty when within the limit or when
    /// no limit is configured. Pinned/newest turns are never here. Used only
    /// for human-facing display (e.g. `/info`); never mutates `self.turns`.
    pub fn dropped_turns(&self) -> Vec<Turn> {
        let keep: HashSet<usize> = self.keep_indices().into_iter().collect();
        self.turns
            .iter()
            .enumerate()
            .filter(|(i, _)| !keep.contains(i))
            .map(|(_, t)| t.clone())
            .collect()
    }
    pub fn replace_turns(&mut self, turns: Vec<Turn>) {
        self.turns = turns;
        // Pins are indices into the previous history; a replaced history
        // (/new, /resume) invalidates them, so clear to avoid dangling pins.
        self.pinned.clear();
        // Goal/plan state belongs to the previous session too.
        self.clear_goal();
        self.plan = None;
    }

    /// Swaps the provider backing this runner. The conversation history in
    /// `self.turns` is untouched, so a mid-session `/provider <name>` switch
    /// keeps the session and all prior turns. Callers must only invoke this at
    /// a turn boundary (i.e. while no stream from the old provider is active),
    /// otherwise a single turn could span two providers.
    pub fn replace_provider(&mut self, provider: P) {
        self.provider = provider;
    }
    pub async fn chat(&mut self, content: impl Into<String>) -> Result<EventStream, ProviderError> {
        self.turns.push(Turn::User {
            content: content.into(),
        });
        // Send only what fits the window; self.turns keeps the full history.
        let to_send = self.turns_to_send();
        self.provider.chat(&to_send).await
    }

    pub async fn chat_with_cancel(
        &mut self,
        content: impl Into<String>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<EventStream, ProviderError> {
        self.turns.push(Turn::User {
            content: content.into(),
        });
        let to_send = self.turns_to_send();
        self.provider.chat_with_cancel(&to_send, cancel).await
    }

    pub fn push_assistant(&mut self, content: String) {
        if !content.is_empty() {
            self.turns.push(Turn::Assistant { content });
        }
    }

    pub fn discard_pending_user(&mut self) {
        if matches!(self.turns.last(), Some(Turn::User { .. })) {
            self.turns.pop();
        }
    }

    pub async fn chat_agentic(
        &mut self,
        content: impl Into<String>,
        registry: &ToolRegistry,
        store_ctx: Option<(&SessionStore, &SessionId)>,
        max_iters: usize,
        cancel: CancellationToken,
    ) -> Result<AgenticResult, ProviderError> {
        let content: String = content.into();
        self.turns.push(Turn::User {
            content: content.clone(),
        });
        // Spec 009 (Ticket 01): when goal tracking is on and no goal is set
        // yet, treat this initiating user prompt as the goal. Off by default,
        // so this never fires unless the user enables it (zero regression).
        self.goal.record_if_tracking_empty(&content);
        // Spec 009 (Ticket 02): in planned mode, generate a plan (one ephemeral
        // instruction round) before executing tools. Sharing the iteration
        // budget: a planning round consumes one of `max_iters`, so the
        // execution loop below runs with one fewer attempt. Off by default, so
        // this branch is never taken in reactive mode (zero regression).
        let plan_round = self.plan_mode && self.plan.is_none();
        if plan_round {
            match self.ensure_plan(cancel.clone()).await {
                Ok(_) => {}
                Err(ProviderError::Cancelled) => {
                    self.discard_pending_user();
                    return Ok(AgenticResult::Cancelled);
                }
                Err(err) => return Err(err),
            }
        }
        // Advisory (never blocking): warn if the full context is estimated to
        // exceed the runner's context limit before we send it. Truncation is a
        // later ticket (sliding window); here we only surface a warning.
        if let Some(warning) = self.context_warning() {
            tracing::warn!("{warning}");
        }
        // Planning shares the budget: when a planning round was taken, one fewer
        // iteration remains for execution (total <= max_iters).
        let exec_budget = if plan_round {
            max_iters.saturating_sub(1).max(1)
        } else {
            max_iters
        };
        for iteration in 1..=exec_budget {
            if cancel.is_cancelled() {
                self.discard_pending_user();
                return Ok(AgenticResult::Cancelled);
            }
            // Recompute the window each iteration (tool results may have grown
            // the context since the last send). self.turns stays untouched. In
            // planned mode the active plan is re-supplied as an ephemeral
            // instruction so the model keeps it in view while executing.
            let to_send = self.turns_to_send();
            let instruction = self.active_plan_instruction();
            let mut stream = match self
                .provider
                .chat_with_instruction(&to_send, instruction.as_deref(), cancel.clone())
                .await
            {
                Ok(stream) => stream,
                Err(ProviderError::Cancelled) => {
                    self.discard_pending_user();
                    return Ok(AgenticResult::Cancelled);
                }
                Err(err) => return Err(err),
            };
            let mut text = String::new();
            let mut calls = Vec::new();
            while let Some(event_res) = stream.next().await {
                match event_res {
                    Ok(Event::Chunk(c)) => text.push_str(&c),
                    Ok(Event::ToolCall(c)) => calls.push(c),
                    Ok(_) => {}
                    Err(ProviderError::Cancelled) => {
                        self.discard_pending_user();
                        return Ok(AgenticResult::Cancelled);
                    }
                    Err(err) => return Err(err),
                }
            }
            if calls.is_empty() {
                self.push_assistant(text.clone());
                return Ok(AgenticResult::Done {
                    text,
                    iterations: iteration,
                });
            }
            for call in calls {
                if cancel.is_cancelled() {
                    self.discard_pending_user();
                    return Ok(AgenticResult::Cancelled);
                }
                let result = registry.execute(&call, cancel.clone()).await;
                let (content, status) = match result {
                    Ok(r) => (r.content, ToolExecutionStatus::Success),
                    Err(ToolError::Denied(e)) => (e, ToolExecutionStatus::Denied),
                    Err(ToolError::Timeout(d)) => {
                        (format!("timeout: {d:?}"), ToolExecutionStatus::Timeout)
                    }
                    Err(ToolError::Cancelled) => {
                        ("cancelled".into(), ToolExecutionStatus::Cancelled)
                    }
                    Err(e) => (e.to_string(), ToolExecutionStatus::Error),
                };
                self.turns.push(Turn::Tool {
                    name: call.name.clone(),
                    content: content.clone(),
                });
                if let Some((store, id)) = store_ctx {
                    let record = ToolCallRecord {
                        id: call
                            .id
                            .clone()
                            .unwrap_or_else(|| format!("{}-{}", iteration, self.turns.len())),
                        session_id: id.to_string(),
                        turn_index: self.turns.len(),
                        tool_name: call.name.clone(),
                        arguments: call.arguments.clone(),
                        result: content,
                        status,
                    };
                    store
                        .save_tool_call(&record)
                        .map_err(|e| ProviderError::Message(e.to_string()))?;
                }
            }
        }
        Ok(AgenticResult::MaxIterations(max_iters))
    }

    /// Compatibility helper for callers that still need a fully collected response.
    pub async fn prompt(
        &mut self,
        content: impl Into<String>,
    ) -> Result<Vec<Event>, ProviderError> {
        let mut stream = self.chat(content).await?;
        let mut events = Vec::new();
        let mut answer = String::new();
        while let Some(event) = stream.next().await {
            match event {
                Ok(Event::Chunk(chunk)) => {
                    answer.push_str(&chunk);
                    events.push(Event::Chunk(chunk));
                }
                Ok(event) => events.push(event),
                Err(err) => {
                    self.discard_pending_user();
                    return Err(err);
                }
            }
        }
        self.push_assistant(answer);
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::FakeProvider;

    fn runner_with(turns: Vec<Turn>) -> ConversationRunner<FakeProvider> {
        ConversationRunner::from_turns(FakeProvider, turns)
    }

    #[test]
    fn estimated_tokens_delegates_to_estimate_turns_tokens() {
        // 40-char content => 10 tokens (char/4 heuristic pinned in context.rs).
        let mut r = runner_with(vec![Turn::User {
            content: "a".repeat(40),
        }]);
        assert_eq!(r.estimated_tokens(), 10);
        // Adding another 40-char user turn raises the count.
        r.turns
            .push(Turn::User {
                content: "b".repeat(40),
            });
        assert_eq!(r.estimated_tokens(), 20);
    }

    #[test]
    fn empty_runner_reports_zero_tokens() {
        let r = runner_with(vec![]);
        assert_eq!(r.estimated_tokens(), 0);
        assert_eq!(r.context_limit(), None);
        assert_eq!(r.context_warning(), None);
    }

    #[test]
    fn context_limit_defaults_to_none_and_is_settable() {
        let mut r = runner_with(vec![]);
        assert_eq!(r.context_limit(), None);
        r.set_context_limit(Some(100));
        assert_eq!(r.context_limit(), Some(100));
        r.set_context_limit(None);
        assert_eq!(r.context_limit(), None);
    }

    #[test]
    fn context_warning_appears_only_when_over_limit() {
        // 200-char user turn => 50 estimated tokens.
        let mut r = runner_with(vec![Turn::User {
            content: "x".repeat(200),
        }]);
        assert_eq!(r.estimated_tokens(), 50);
        // No limit configured -> no warning.
        assert_eq!(r.context_warning(), None);
        // Limit large enough -> no warning.
        r.set_context_limit(Some(100));
        assert_eq!(r.context_warning(), None);
        // Limit below estimate -> warning naming both numbers.
        r.set_context_limit(Some(40));
        let w = r.context_warning().expect("should warn");
        assert!(w.contains("50"), "must name estimate: {w}");
        assert!(w.contains("40"), "must name limit: {w}");
    }

    #[tokio::test]
    async fn chat_records_the_user_turn_before_sending() {
        let mut r = runner_with(vec![]);
        // FakeProvider always returns a stream; just ensure a user turn was
        // recorded (context accounting reflects it).
        let content = "z".repeat(80);
        let stream = r.chat(content.clone()).await.unwrap();
        use futures::StreamExt;
        stream.collect::<Vec<_>>().await;
        assert_eq!(r.turns().len(), 1);
        assert_eq!(r.estimated_tokens(), crate::conversation::context::estimate_tokens(&content));
    }

    fn many_turns(n: usize, len: usize) -> Vec<Turn> {
        (0..n)
            .map(|i| Turn::User {
                content: format!("{i}-{}", "x".repeat(len)),
            })
            .collect()
    }

    #[test]
    fn turns_to_send_returns_full_when_no_limit() {
        // 100-turn history, but no context_limit -> no trimming (backward compat).
        let r = runner_with(many_turns(100, 40));
        assert_eq!(r.context_limit(), None);
        let sent = r.turns_to_send();
        assert_eq!(sent.len(), 100, "no limit must not trim");
    }

    #[test]
    fn turns_to_send_returns_full_when_within_limit() {
        // 10 turns x 40 chars = 10 tokens each => 100 tokens, within 200 limit.
        let r = runner_with(many_turns(10, 40));
        let mut r = r;
        r.set_context_limit(Some(200));
        assert_eq!(r.turns_to_send().len(), 10);
    }

    #[test]
    fn turns_to_send_trims_oldest_and_preserves_most_recent() {
        // 100 turns x 40 chars = 10 tokens each => 1000 tokens, limit 120.
        let original = many_turns(100, 40);
        let last = original.last().unwrap().clone();
        let mut r = runner_with(original.clone());
        r.set_context_limit(Some(120));

        let sent = r.turns_to_send();
        // Must fit the budget.
        let est = crate::conversation::context::estimate_turns_tokens(&sent);
        assert!(
            est <= 120,
            "trimmed context must fit limit, got {est} tokens"
        );
        // Fewer than the full history, and the newest turn is kept.
        assert!(sent.len() < 100, "must drop old turns: {}", sent.len());
        assert_eq!(sent.last(), Some(&last), "must keep the newest turn");
        // self.turns is untouched (full history preserved for state.db).
        assert_eq!(r.turns().len(), 100, "self.turns must not be mutated");
    }

    #[test]
    fn turns_to_send_keeps_at_least_one_turn_even_if_single_is_over() {
        // One huge turn that alone exceeds the limit: still sent (never empty).
        let mut r = runner_with(vec![Turn::User {
            content: "y".repeat(10_000), // 2500 tokens
        }]);
        r.set_context_limit(Some(100));
        let sent = r.turns_to_send();
        assert_eq!(sent.len(), 1, "must never send an empty window");
    }

    #[test]
    fn dropped_turns_reports_the_trimmed_complement() {
        let history = many_turns(100, 40);
        let mut r = runner_with(history.clone());
        // No limit -> nothing dropped.
        assert_eq!(r.dropped_turns().len(), 0);
        // Tight limit -> old turns reported dropped, newest preserved.
        r.set_context_limit(Some(120));
        let dropped = r.dropped_turns();
        assert!(!dropped.is_empty(), "tight limit must drop some turns");
        let sent = r.turns_to_send();
        // dropped + sent == full history (disjoint partition, no loss).
        assert_eq!(dropped.len() + sent.len(), history.len());
        // self.turns remains the full history.
        assert_eq!(r.turns().len(), history.len());
        // The most recent turn is never dropped.
        assert!(!dropped.contains(&history.last().unwrap().clone()));
    }

    #[test]
    fn pin_protects_a_turn_from_the_sliding_window() {
        // Equal turns so order/dropping is deterministic.
        let mut history = many_turns(100, 40);
        // Make turn 0 (index 0) distinctive so we can find it in the output.
        history[0] = Turn::User {
            content: format!("PINNED-{}", "x".repeat(40)),
        };
        let mut r = runner_with(history.clone());
        r.set_context_limit(Some(120));
        // Without a pin, index 0 is dropped.
        assert!(!r.dropped_turns().is_empty(), "tight limit drops turns");

        // Pin turn 0 -> it must now be sent even though it is the oldest.
        r.pin(0).unwrap();
        let sent = r.turns_to_send();
        assert!(
            sent.iter().any(|t| matches!(t, Turn::User { content } if content.starts_with("PINNED-"))),
            "pinned oldest turn must be sent"
        );
        // And it must not appear in dropped.
        assert!(
            !r.dropped_turns()
                .iter()
                .any(|t| matches!(t, Turn::User { content } if content.starts_with("PINNED-"))),
            "pinned turn must not be dropped"
        );
    }

    #[test]
    fn pin_newest_is_kept_and_pins_survive_replacement_rules() {
        let mut r = runner_with(many_turns(5, 40));
        // 5 turns within no limit; pin two distinct indices.
        r.pin(1).unwrap();
        r.pin(3).unwrap();
        assert_eq!(r.pinned(), vec![1, 3]);
        assert!(r.is_pinned(1));
        assert!(!r.is_pinned(2));
        // Replacing the history clears pins (no dangling indices).
        r.replace_turns(many_turns(3, 40));
        assert!(r.pinned().is_empty(), "pins must clear when history is replaced");
    }

    #[test]
    fn pin_rejects_out_of_range_and_duplicates() {
        let mut r = runner_with(many_turns(5, 40));
        assert!(r.pin(5).is_err(), "index out of range must error");
        assert!(r.pin(0).is_ok());
        assert!(r.pin(0).is_err(), "double pin must error");
        assert!(r.unpin(0).is_ok());
        assert!(r.unpin(0).is_err(), "unpinning a non-pinned turn must error");
        assert!(r.unpin(9).is_err(), "out-of-range unpin must error");
    }

    #[test]
    fn pinned_turns_count_against_the_token_budget() {
        // 5 turns x 10 tokens = 50 tokens total, limit 40. Pin the newest + one
        // old turn so they alone exceed; they are still sent (warn, not drop).
        let mut r = runner_with(many_turns(5, 40));
        r.set_context_limit(Some(20)); // newest(10)+nothing else fits
        // newest is index 4 (always kept). Pin it plus index 3 -> 20, fits.
        r.pin(4).unwrap();
        r.pin(3).unwrap();
        let sent = r.turns_to_send();
        assert!(
            sent.iter().any(|t| matches!(t, Turn::User { content } if content.starts_with("4-"))),
            "newest pinned kept"
        );
        // self.turns not mutated.
        assert_eq!(r.turns().len(), 5);
    }

    #[tokio::test]
    async fn goal_tracking_defaults_off_and_records_nothing() {
        let mut r = runner_with(vec![]);
        assert!(!r.goal_tracking(), "tracking must default off");
        // A normal turn with tracking off leaves no goal (zero regression).
        let reg = ToolRegistry::new();
        let _ = r
            .chat_agentic("do the task", &reg, None, 10, CancellationToken::new())
            .await;
        assert_eq!(r.goal(), None);
        assert_eq!(r.goal_status(), GoalStatus::NotStarted);
    }

    #[tokio::test]
    async fn goal_tracking_records_first_user_turn_when_enabled() {
        let mut r = ConversationRunner::new(FakeProvider);
        r.set_goal_tracking(true);
        let reg = ToolRegistry::new();
        let _ = r
            .chat_agentic("fetch the monthly report", &reg, None, 10, CancellationToken::new())
            .await;
        assert_eq!(r.goal(), Some("fetch the monthly report"));
        assert_eq!(r.goal_status(), GoalStatus::InProgress);
        // A later turn does not overwrite the first goal.
        let _ = r
            .chat_agentic("also email it", &reg, None, 10, CancellationToken::new())
            .await;
        assert_eq!(r.goal(), Some("fetch the monthly report"));
    }

    #[test]
    fn goal_api_exposes_set_status_and_reset() {
        let mut r = runner_with(vec![]);
        assert!(r.set_goal("build x".into()));
        assert_eq!(r.goal_status(), GoalStatus::InProgress);
        r.set_goal_status(GoalStatus::Achieved);
        assert_eq!(r.goal_status(), GoalStatus::Achieved);
        // replace_turns (e.g. /new, /resume) clears goal + pins together.
        r.replace_turns(vec![Turn::User { content: "y".into() }]);
        assert_eq!(r.goal(), None);
        assert_eq!(r.goal_status(), GoalStatus::NotStarted);
    }

    // -- Spec 009 plan (Ticket 02) -----------------------------------------

    /// A provider that echoes a fixed assistant reply (no tool calls), so plan
    /// generation can be tested deterministically through the runner.
    struct Reply(&'static str);
    #[async_trait::async_trait]
    impl Provider for Reply {
        async fn chat(
            &self,
            _turns: &[Turn],
        ) -> Result<crate::provider::EventStream, ProviderError> {
            use futures::stream;
            Ok(Box::pin(stream::iter([
                Ok(Event::Started),
                Ok(Event::Chunk(self.0.to_owned())),
                Ok(Event::Done),
            ])))
        }
    }

    #[test]
    fn plan_mode_defaults_off_and_turning_it_off_clears_the_plan() {
        let mut r = runner_with(vec![]);
        assert!(!r.plan_mode());
        assert!(r.plan().is_none());
        r.set_plan_mode(true);
        assert!(r.plan_mode());
        r.set_plan_mode(false);
        assert!(!r.plan_mode());
        assert!(r.plan().is_none());
    }

    #[tokio::test]
    async fn ensure_plan_generates_and_stores_a_plan_in_planned_mode() {
        let mut r = ConversationRunner::new(Reply(
            "[[plan]]\n1. list files\n2. read config\n[[/plan]]",
        ));
        r.turns
            .push(Turn::User { content: "inspect the repo".into() });
        r.set_plan_mode(true);
        assert!(r.plan().is_none());
        let plan = r.ensure_plan(CancellationToken::new()).await.unwrap();
        let plan = plan.expect("plan must be generated");
        assert_eq!(plan.steps(), &["1. list files", "2. read config"]);
        assert_eq!(r.plan().unwrap().steps(), &["1. list files", "2. read config"]);
        // estimated tokens now include the plan's token contribution.
        assert_eq!(
            r.estimated_tokens(),
            crate::conversation::context::estimate_turns_tokens(&r.turns) + plan.tokens()
        );
    }

    #[tokio::test]
    async fn ensure_plan_is_a_noop_outside_planned_mode() {
        let mut r = ConversationRunner::new(Reply(
            "[[plan]]\n1. x\n[[/plan]]",
        ));
        r.turns.push(Turn::User { content: "task".into() });
        // Reactive (plan_mode off): no plan is produced.
        let plan = r.ensure_plan(CancellationToken::new()).await.unwrap();
        assert!(plan.is_none());
        assert!(r.plan().is_none());
    }

    #[tokio::test]
    async fn ensure_plan_ignores_literal_tool_tags_in_plan_text() {
        // A plan that quotes a <tool_call> still parses as a plan and is never
        // executed: ensure_plan only reads assistant text.
        let mut r = ConversationRunner::new(Reply(
            "[[plan]]\nnote: do not run <tool_call> yet\n[[/plan]]",
        ));
        r.turns.push(Turn::User { content: "task".into() });
        r.set_plan_mode(true);
        let plan = r.ensure_plan(CancellationToken::new()).await.unwrap();
        let plan = plan.expect("plan must be generated");
        assert!(
            plan.steps().iter().any(|s| s.contains("<tool_call>")),
            "literal tool tag must be preserved as text, got {plan:?}"
        );
    }

    #[tokio::test]
    async fn replace_turns_clears_the_plan() {
        let mut r = ConversationRunner::new(Reply("[[plan]]\n1. a\n[[/plan]]"));
        r.turns.push(Turn::User { content: "task".into() });
        r.set_plan_mode(true);
        let _ = r.ensure_plan(CancellationToken::new()).await.unwrap();
        assert!(r.plan().is_some(), "plan should be generated");
        r.replace_turns(vec![Turn::User { content: "x".into() }]);
        assert!(r.plan().is_none(), "replace_turns must clear the plan");
    }
}
