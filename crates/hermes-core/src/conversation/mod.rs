//! In-memory conversation state and provider event flow.

use crate::provider::{EventStream, Provider, ProviderError};
use crate::tools::{ToolCall, ToolError, ToolRegistry};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

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
        max_iters: usize,
        cancel: CancellationToken,
    ) -> Result<AgenticResult, ProviderError> {
        self.turns.push(Turn::User {
            content: content.into(),
        });
        for iteration in 1..=max_iters {
            let mut stream = self
                .provider
                .chat_with_cancel(&self.turns, cancel.clone())
                .await?;
            let mut text = String::new();
            let mut calls = Vec::new();
            while let Some(event) = stream.next().await {
                match event? {
                    Event::Chunk(chunk) => text.push_str(&chunk),
                    Event::ToolCall(call) => calls.push(call),
                    _ => {}
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
                    return Err(ProviderError::Cancelled);
                }
                self.turns.push(Turn::Tool {
                    name: format!("call:{}", call.name),
                    content: call.arguments.clone(),
                });
                let response =
                    registry
                        .execute(&call, cancel.clone())
                        .await
                        .map_err(|e| match e {
                            ToolError::Cancelled => ProviderError::Cancelled,
                            other => ProviderError::Message(other.to_string()),
                        })?;
                self.turns.push(Turn::Tool {
                    name: response.name.clone(),
                    content: response.content.clone(),
                });
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
