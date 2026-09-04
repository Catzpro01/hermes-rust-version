//! In-memory conversation state and provider event flow.

use crate::provider::{EventStream, Provider, ProviderError};
use crate::session::{SessionId, SessionStore};
use crate::tools::{ToolCall, ToolCallRecord, ToolError, ToolExecutionStatus, ToolRegistry};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

pub mod context;

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
}
impl<P: Provider> ConversationRunner<P> {
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            turns: Vec::new(),
            context_limit: None,
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

    /// Estimated tokens across all current turns, delegating to the Spec 006
    /// helper so the char/4 heuristic lives in exactly one place.
    pub fn estimated_tokens(&self) -> usize {
        crate::conversation::context::estimate_turns_tokens(&self.turns)
    }

    /// Advisory warning when current context is estimated to exceed the limit.
    /// `None` when within limit or when no limit is configured. Never blocks.
    pub fn context_warning(&self) -> Option<String> {
        crate::conversation::context::check_context_limit(&self.turns, self.context_limit)
    }

    /// Returns the slice of turns to send to the provider (Ticket 02).
    ///
    /// Sliding-window invariant: this returns a **copy** trimmed to fit the
    /// runner's `context_limit` and never mutates `self.turns`. The full
    /// history stays in `self.turns` (and thus in `state.db`, which the REPL
    /// persists from here); only what is handed to the model is shortened.
    ///
    /// - `context_limit = None` → returns the full history unchanged
    ///   (backward compatible: no window).
    /// - Already within budget → full history.
    /// - Otherwise oldest turns are dropped from the front until the estimate
    ///   fits or only one turn remains. The most recent turn (the active
    ///   question) is never dropped.
    pub fn turns_to_send(&self) -> Vec<Turn> {
        let limit = match self.context_limit {
            None => return self.turns.clone(),
            Some(limit) => limit,
        };
        let over = |turns: &[Turn]| -> bool {
            crate::conversation::context::estimate_turns_tokens(turns) as u64 > limit
        };
        if !over(&self.turns) {
            return self.turns.clone();
        }
        let mut trimmed = self.turns.clone();
        while trimmed.len() > 1 && over(&trimmed) {
            trimmed.remove(0);
        }
        trimmed
    }
    pub fn replace_turns(&mut self, turns: Vec<Turn>) {
        self.turns = turns;
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
        self.turns.push(Turn::User {
            content: content.into(),
        });
        // Advisory (never blocking): warn if the full context is estimated to
        // exceed the runner's context limit before we send it. Truncation is a
        // later ticket (sliding window); here we only surface a warning.
        if let Some(warning) = self.context_warning() {
            tracing::warn!("{warning}");
        }
        for iteration in 1..=max_iters {
            if cancel.is_cancelled() {
                self.discard_pending_user();
                return Ok(AgenticResult::Cancelled);
            }
            // Recompute the window each iteration (tool results may have grown
            // the context since the last send). self.turns stays untouched.
            let to_send = self.turns_to_send();
            let mut stream = match self
                .provider
                .chat_with_cancel(&to_send, cancel.clone())
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
}
