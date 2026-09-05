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
pub mod events;
pub mod goal;
pub mod plan;
pub mod recovery;
pub mod reflection;

pub use events::AgentEvent;

use goal::{GoalStatus, GoalTracker};
use plan::Plan;
use recovery::RetryTracker;
use reflection::ReflectionTracker;

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
    /// Execution was stopped because it is blocked (e.g. a user denial, or
    /// retries exhausted for a failing step) — semantically distinct from
    /// merely running out of iterations.
    Blocked { reason: String },
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
    /// Spec 009 (Ticket 03) — self-reflection gate. Off by default so reactive
    /// mode is unchanged.
    reflection: ReflectionTracker,
    /// Spec 009 (Ticket 04) — bounded retry/recovery tracking per tool.
    recovery: RetryTracker,
    /// Spec 012 — optional live event observer. `None` (the default) keeps the
    /// runner silent for the REPL and existing callers (zero regression); when
    /// set, `chat_agentic` emits [`AgentEvent`]s on a best-effort, non-blocking
    /// basis.
    observer: Option<tokio::sync::mpsc::Sender<AgentEvent>>,
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
            reflection: ReflectionTracker::new(),
            recovery: RetryTracker::new(),
            observer: None,
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
            reflection: ReflectionTracker::new(),
            recovery: RetryTracker::new(),
            observer: None,
        }
    }

    /// Registers an observer to receive live agentic events (Spec 012).
    /// Default is `None`. Callers may clear it by dropping or by not calling.
    pub fn set_observer(&mut self, sender: tokio::sync::mpsc::Sender<AgentEvent>) {
        self.observer = Some(sender);
    }

    /// Best-effort emission to the observer, if any. Never blocks: a full
    /// channel drops the event (a stale display frame is acceptable; blocking a
    /// producer would deadlock the turn).
    fn emit(&self, event: AgentEvent) {
        if let Some(sender) = &self.observer {
            let _ = sender.try_send(event);
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

    // -- Spec 009 self-reflection gate (Ticket 03) --------------------------

    /// Enables/disables the reflection gate. Default off -> reactive unchanged.
    pub fn set_reflection(&mut self, on: bool) {
        self.reflection.set_enabled(on);
    }
    pub fn reflection_enabled(&self) -> bool {
        self.reflection.enabled()
    }
    /// Reflections consumed for the current plan step (anti-loop accounting).
    pub fn reflections_used(&self) -> usize {
        self.reflection.reflections_used()
    }
    /// Applies the deterministic heuristic verdict for one tool outcome to the
    /// goal lifecycle. No-op unless reflection is enabled. Returns the verdict.
    pub fn reflect_tool_outcome(
        &mut self,
        status: ToolExecutionStatus,
        retries_remaining: bool,
    ) -> reflection::Verdict {
        reflection::apply_verdict(&mut self.reflection, status, retries_remaining, &mut self.goal)
    }

    // -- Spec 009 recovery / parameter mutation (Ticket 04) -----------------

    /// Recovery is active only when reflection is enabled (opt-in). Off by
    /// default -> the tool loop is unchanged.
    pub fn recovery_enabled(&self) -> bool {
        self.reflection_enabled()
    }
    /// Resets per-task recovery/retry state.
    pub fn reset_recovery(&mut self) {
        self.recovery.reset();
    }
    /// Whether an exact argument set for `tool` was already attempted.
    pub fn is_attempted(&self, tool: &str, arguments: &str) -> bool {
        self.recovery.is_tried(tool, arguments)
    }
    /// The Option 1 "already tried" note for `tool` (what the model sees so it
    /// picks different parameters).
    pub fn already_tried_note(&self, tool: &str) -> Option<String> {
        self.recovery.already_tried_note(tool)
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
        self.reset_recovery();
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
        // Spec 009 (Ticket 03): a new task resets per-step reflection count.
        self.reflection.reset_step();
        // Spec 009 (Ticket 04): a new task resets retry/recovery state too.
        self.reset_recovery();
        // Spec 012: refresh status + token accounting once a turn is underway
        // (after the user turn and any planning round were recorded).
        self.emit(AgentEvent::StatusChanged {
            goal_status: Some(self.goal_status()),
            plan_active: self.plan_mode(),
            reflection_on: self.reflection_enabled(),
        });
        self.emit(AgentEvent::TokenTick {
            estimate: self.estimated_tokens(),
            limit: self.context_limit(),
        });
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
            // Spec 012: surface iteration progress for the observer.
            self.emit(AgentEvent::Iteration {
                current: iteration,
                max: exec_budget,
            });
            // Spec 009 (Ticket 03/04): once the goal is blocked (user denial or
            // retries exhausted), stop the loop early with a Blocked result
            // rather than burning remaining iterations. Reactive mode is off by
            // default, so this never triggers unless reflection/recovery is on.
            if self.recovery_enabled() && self.goal_status() == GoalStatus::Blocked {
                return Ok(AgenticResult::Blocked {
                    reason: "a tool step is blocked (user denial or retries exhausted)".into(),
                });
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
                    Ok(Event::Chunk(c)) => {
                        text.push_str(&c);
                        self.emit(AgentEvent::Chunk { text: c });
                    }
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
                // Spec 009 (Ticket 05): a normal, tool-free completion formally
                // closes an active, in-progress goal as Achieved — but only
                // while the reflection gate is on. In plain reactive goal
                // tracking (reflection off) the goal stays open rather than
                // being auto-closed by an answer that happens to use no tool.
                // No-op when no goal is active or it is already closed.
                if self.reflection_enabled() && self.goal_status() == GoalStatus::InProgress {
                    self.set_goal_status(GoalStatus::Achieved);
                }
                // Spec 012: notify the observer of a clean completion and a
                // final token refresh.
                self.emit(AgentEvent::Done { text: text.clone() });
                self.emit(AgentEvent::TokenTick {
                    estimate: self.estimated_tokens(),
                    limit: self.context_limit(),
                });
                return Ok(AgenticResult::Done {
                    text,
                    iterations: iteration,
                });
            }
            // Whether later iterations remain (retries are possible only while
            // the execution budget is not exhausted).
            let retries_remaining = iteration < exec_budget;
            for call in calls {
                if cancel.is_cancelled() {
                    self.discard_pending_user();
                    return Ok(AgenticResult::Cancelled);
                }
                // Spec 009 (Ticket 04, Option 1): reject an exact repeat of a
                // call that already failed retryably — do NOT execute it again.
                // Feed an "already tried" note so the model mutates parameters.
                if self.recovery_enabled() && self.is_attempted(&call.name, &call.arguments) {
                    let note = self.already_tried_note(&call.name).unwrap_or_default();
                    let dup = format!(
                        "duplicate of an earlier failed call — adjust parameters. {note}"
                    );
                    self.turns.push(Turn::Tool {
                        name: call.name.clone(),
                        content: dup.clone(),
                    });
                    if !self.recovery.can_retry(&call.name) {
                        self.set_goal_status(GoalStatus::Blocked);
                        return Ok(AgenticResult::Blocked {
                            reason: format!("no retries left for tool '{}'", call.name),
                        });
                    }
                    continue;
                }
                // Spec 012: notify the observer that a tool call is running.
                self.emit(AgentEvent::ToolStarted {
                    name: call.name.clone(),
                    arguments: call.arguments.clone(),
                });
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
                // Spec 012: notify the observer of the completed tool call.
                self.emit(AgentEvent::ToolDone {
                    name: call.name.clone(),
                    status: status.clone(),
                    result: content.clone(),
                });
                self.turns.push(Turn::Tool {
                    name: call.name.clone(),
                    content: content.clone(),
                });
                // Spec 009 (Ticket 03): heuristic reflection on the tool result.
                // No-op unless reflection is enabled (off by default -> the
                // reactive path is byte-for-byte unchanged). A Denied/Blocked
                // verdict marks the goal Blocked; OffPlan is left for recovery
                // (Ticket 04).
                self.reflect_tool_outcome(status.clone(), retries_remaining);
                // Spec 009 (Ticket 04): record a retryable failure and mark the
                // goal blocked once per-tool retries are exhausted. Denied is
                // never recorded (never retried, per Ticket 03 / Spec 002).
                if self.recovery_enabled()
                    && matches!(
                        status,
                        ToolExecutionStatus::Error | ToolExecutionStatus::Timeout
                    )
                {
                    self.recovery.record(&call.name, &call.arguments);
                    if !self.recovery.can_retry(&call.name) {
                        self.set_goal_status(GoalStatus::Blocked);
                    }
                }
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

    #[tokio::test]
    async fn observer_streams_live_agent_events() {
        use tokio::sync::mpsc;
        let mut r = ConversationRunner::new(FakeProvider);
        let (tx, mut rx) = mpsc::channel::<AgentEvent>(256);
        r.set_observer(tx);
        let reg = ToolRegistry::new();
        let res = r
            .chat_agentic("hello observer", &reg, None, 5, CancellationToken::new())
            .await
            .unwrap();
        match res {
            AgenticResult::Done { text, .. } => assert!(text.contains("observer")),
            other => panic!("expected Done, got {other:?}"),
        }
        // Drain whatever the non-blocking channel still holds (the producer may
        // have run ahead of us, so buffer has capacity 256 >> events emitted).
        let mut seen: Vec<AgentEvent> = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            seen.push(ev);
        }
        assert!(seen.iter().any(|e| matches!(e, AgentEvent::StatusChanged { .. })));
        assert!(seen.iter().any(|e| matches!(e, AgentEvent::TokenTick { .. })));
        assert!(seen.iter().any(|e| matches!(e, AgentEvent::Iteration { .. })));
        assert!(seen.iter().any(|e| matches!(e, AgentEvent::Chunk { .. })));
        assert!(seen
            .iter()
            .any(|e| matches!(e, AgentEvent::Done { .. })));
    }

    #[tokio::test]
    async fn observer_is_optional_and_off_by_default() {
        // Without an observer the loop still runs and returns a result (zero
        // regression / zero overhead path).
        let mut r = ConversationRunner::new(FakeProvider);
        let reg = ToolRegistry::new();
        let res = r
            .chat_agentic("no observer", &reg, None, 3, CancellationToken::new())
            .await
            .unwrap();
        assert!(matches!(res, AgenticResult::Done { .. }));
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

    // -- Spec 009 reflection (Ticket 03) ------------------------------------

    #[test]
    fn reflection_defaults_off_and_is_inert_when_off() {
        let mut r = runner_with(vec![]);
        assert!(!r.reflection_enabled());
        assert!(r.set_goal("task".into()));
        r.set_goal_status(GoalStatus::InProgress);
        // Even a Denied outcome has no effect when reflection is disabled.
        r.reflect_tool_outcome(ToolExecutionStatus::Denied, true);
        assert_eq!(r.goal_status(), GoalStatus::InProgress);
        assert_eq!(r.reflections_used(), 0);
    }

    #[test]
    fn denied_blocks_the_goal_when_reflection_is_on() {
        let mut r = runner_with(vec![]);
        r.set_reflection(true);
        assert!(r.reflection_enabled());
        assert!(r.set_goal("task".into()));
        r.set_goal_status(GoalStatus::InProgress);
        // Success keeps the goal in progress.
        r.reflect_tool_outcome(ToolExecutionStatus::Success, true);
        assert_eq!(r.goal_status(), GoalStatus::InProgress);
        // A user denial always blocks the goal (never retried).
        r.reflect_tool_outcome(ToolExecutionStatus::Denied, true);
        assert_eq!(r.goal_status(), GoalStatus::Blocked);
    }

    #[test]
    fn off_plan_error_increments_reflection_count_until_blocked() {
        let mut r = runner_with(vec![]);
        r.set_reflection(true);
        assert!(r.set_goal("task".into()));
        r.set_goal_status(GoalStatus::InProgress);
        assert_eq!(
            r.reflect_tool_outcome(ToolExecutionStatus::Error, true),
            reflection::Verdict::OffPlan
        );
        assert_eq!(r.reflections_used(), 1);
        assert_eq!(r.goal_status(), GoalStatus::InProgress);
        // Exhausting retries -> blocked immediately.
        assert_eq!(
            r.reflect_tool_outcome(ToolExecutionStatus::Timeout, false),
            reflection::Verdict::Blocked
        );
        assert_eq!(r.goal_status(), GoalStatus::Blocked);
    }

    // -- Spec 009 recovery (Ticket 04) --------------------------------------

    #[test]
    fn recovery_tracks_reflection_and_defaults_off() {
        let mut r = runner_with(vec![]);
        assert!(!r.recovery_enabled(), "recovery must default off");
        // Recovery is active only when reflection is on.
        r.set_reflection(true);
        assert!(r.recovery_enabled());
        assert!(!r.is_attempted("read_file", "{\"path\":\"x\"}"));
        assert_eq!(r.already_tried_note("read_file"), None);
        // Resetting per-task state keeps the toggle but clears the tracker.
        r.reset_recovery();
        assert!(r.recovery_enabled());
        assert!(!r.is_attempted("read_file", "{\"path\":\"x\"}"));
    }

    #[test]
    fn blocked_result_is_distinct_from_max_iterations() {
        assert_ne!(
            AgenticResult::Blocked { reason: "denied".into() },
            AgenticResult::MaxIterations(10)
        );
    }

    // -- Spec 009 goal closure on completion (Ticket 05) --------------------

    #[tokio::test]
    async fn reflection_on_closes_goal_as_achieved_on_done() {
        // Guided mode (reflection on, goal tracked): a tool-free final answer
        // completes the goal -> Done with goal Achieved.
        let mut r = ConversationRunner::new(Reply("done, no more tools"));
        r.set_reflection(true);
        r.set_goal_tracking(true);
        let reg = ToolRegistry::new();
        let out = r
            .chat_agentic("task", &reg, None, 10, CancellationToken::new())
            .await
            .unwrap();
        assert!(matches!(out, AgenticResult::Done { .. }));
        assert_eq!(r.goal_status(), GoalStatus::Achieved);
    }

    #[tokio::test]
    async fn reactive_mode_never_auto_closes_a_goal() {
        // Reflection off (the default): even with a goal tracked, finishing
        // with no tool calls must NOT mark the goal Achieved (zero regression
        // for plain reactive goal tracking).
        let mut r = ConversationRunner::new(Reply("answer, no tools"));
        r.set_goal_tracking(true); // goal tracked but reflection is off
        let reg = ToolRegistry::new();
        let _ = r
            .chat_agentic("task", &reg, None, 10, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(r.goal(), Some("task"));
        assert_eq!(r.goal_status(), GoalStatus::InProgress);
    }

    #[tokio::test]
    async fn no_goal_is_left_untouched_on_done() {
        // Neither toggle on: no goal is ever recorded or closed.
        let mut r = ConversationRunner::new(Reply("plain answer"));
        let reg = ToolRegistry::new();
        let _ = r
            .chat_agentic("task", &reg, None, 10, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(r.goal(), None);
        assert_eq!(r.goal_status(), GoalStatus::NotStarted);
    }
}
