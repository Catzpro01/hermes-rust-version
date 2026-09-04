use super::{tool_aware_stream, EventStream, Provider, ProviderError};
use crate::conversation::{Event, Turn};
use async_trait::async_trait;
use futures::stream;

/// Deterministic offline provider for tests and local development.
pub struct FakeProvider;
#[async_trait]
impl Provider for FakeProvider {
    async fn chat(&self, turns: &[Turn]) -> Result<EventStream, ProviderError> {
        let input = turns
            .iter()
            .rev()
            .find_map(|t| match t {
                Turn::User { content } => Some(content.as_str()),
                _ => None,
            })
            .unwrap_or("");
        if turns.iter().any(|turn| matches!(turn, Turn::Tool { .. })) {
            return Ok(tool_aware_stream(Box::pin(stream::iter([
                Ok(Event::Started),
                Ok(Event::Chunk("tool completed".into())),
                Ok(Event::Done),
            ]))));
        }
        if input == "tool" {
            return Ok(tool_aware_stream(Box::pin(stream::iter([
                Ok(Event::Started),
                Ok(Event::Chunk(
                    "<tool_call id=\"fake-1\">read_file: {\"path\":\"Cargo.toml\"}</tool_call>"
                        .into(),
                )),
                Ok(Event::Done),
            ]))));
        }
        if input == "error" {
            return Ok(tool_aware_stream(Box::pin(stream::iter([
                Ok(Event::Started),
                Err(ProviderError::Message("simulated".into())),
            ]))));
        }
        Ok(tool_aware_stream(Box::pin(stream::iter([
            Ok(Event::Started),
            Ok(Event::Chunk(format!("echo: {input}"))),
            Ok(Event::Done),
        ]))))
    }
}
