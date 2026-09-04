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
}
impl<P: Provider> ConversationRunner<P> {
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            turns: Vec::new(),
        }
    }
    pub fn turns(&self) -> &[Turn] {
        &self.turns
    }
    pub fn from_turns(provider: P, turns: Vec<Turn>) -> Self {
        Self { provider, turns }
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
        self.provider.chat(&self.turns).await
    }

    pub async fn chat_with_cancel(
        &mut self,
        content: impl Into<String>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<EventStream, ProviderError> {
        self.turns.push(Turn::User {
            content: content.into(),
        });
        self.provider.chat_with_cancel(&self.turns, cancel).await
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
        for iteration in 1..=max_iters {
            if cancel.is_cancelled() {
                self.discard_pending_user();
                return Ok(AgenticResult::Cancelled);
            }
            let mut stream = match self
                .provider
                .chat_with_cancel(&self.turns, cancel.clone())
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
