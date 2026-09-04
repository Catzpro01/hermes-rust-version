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
        ]))));
    }
}
