//! In-memory conversation state and provider event flow.

use crate::provider::{EventStream, Provider, ProviderError};
use futures::StreamExt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Turn {
    User { content: String },
    Assistant { content: String },
    Tool { name: String, content: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Started,
    Chunk(String),
    Done,
    Error(String),
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
    pub async fn prompt(
        &mut self,
        content: impl Into<String>,
    ) -> Result<Vec<Event>, ProviderError> {
        self.turns.push(Turn::User {
            content: content.into(),
        });
        let stream: EventStream = self.provider.chat(&self.turns).await?;
        let mut events = Vec::new();
        let mut answer = String::new();
        futures::pin_mut!(stream);
        while let Some(event) = stream.next().await {
            match event {
                Ok(Event::Chunk(chunk)) => {
                    answer.push_str(&chunk);
                    events.push(Event::Chunk(chunk));
                }
                Ok(event) => events.push(event),
                Err(err) => {
                    events.push(Event::Error(err.to_string()));
                    return Err(err);
                }
            }
        }
        if !answer.is_empty() {
            self.turns.push(Turn::Assistant { content: answer });
        }
        Ok(events)
    }
}
