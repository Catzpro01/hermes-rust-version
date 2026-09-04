//! Provider-neutral asynchronous chat contract.

use crate::conversation::{Event, Turn};
use crate::tools::{parse_tool_events, parser::ToolEvent};
use async_stream::try_stream;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Error, Clone)]
pub enum ProviderError {
    #[error("provider error: {0}")]
    Message(String),
    #[error("provider returned HTTP {status}: {message}")]
    Http { status: u16, message: String },
    #[error("request cancelled")]
    Cancelled,
}
pub type EventStream = Pin<Box<dyn Stream<Item = Result<Event, ProviderError>> + Send>>;

#[async_trait]
pub trait Provider: Send + Sync {
    async fn chat(&self, turns: &[Turn]) -> Result<EventStream, ProviderError>;

    async fn chat_with_cancel(
        &self,
        turns: &[Turn],
        _cancel: CancellationToken,
    ) -> Result<EventStream, ProviderError> {
        self.chat(turns).await
    }
}

#[async_trait]
impl<T: Provider + ?Sized> Provider for Box<T> {
    async fn chat(&self, turns: &[Turn]) -> Result<EventStream, ProviderError> {
        (**self).chat(turns).await
    }
    async fn chat_with_cancel(
        &self,
        turns: &[Turn],
        cancel: CancellationToken,
    ) -> Result<EventStream, ProviderError> {
        (**self).chat_with_cancel(turns, cancel).await
    }
}

pub mod fake;
pub mod http;
mod redact;
pub mod sse;
pub use fake::FakeProvider;
pub use http::HttpProvider;
pub use redact::redact;

/// Converts streamed text containing Hermes XML tool tags into typed events.
pub fn tool_aware_stream(mut input: EventStream) -> EventStream {
    Box::pin(try_stream! {
        let mut buffer = String::new();
        while let Some(item) = input.next().await {
            let event = item?;
            match event {
                Event::Chunk(text) => {
                    buffer.push_str(&text);
                    loop {
                        if let Some(start) = buffer.find("<tool_call") {
                            if start > 0 { yield Event::Chunk(buffer[..start].to_owned()); buffer.drain(..start); }
                            if let Some(end) = buffer.find("</tool_call>") {
                                let end = end + "</tool_call>".len();
                                let xml = buffer[..end].to_owned(); buffer.drain(..end);
                                match parse_tool_events(&xml) {
                                    Ok(events) => for parsed in events { if let ToolEvent::Call(call) = parsed { yield Event::ToolCall(call); } },
                                    Err(_) => yield Event::Chunk(xml),
                                }
                                continue;
                            }
                            break;
                        }
                        if buffer.contains("<tool_") { break; }
                        if buffer.len() > 128 { let split = buffer.len() - 64; let text = buffer[..split].to_owned(); buffer.drain(..split); yield Event::Chunk(text); }
                        break;
                    }
                }
                other => yield other,
            }
        }
        if !buffer.is_empty() { yield Event::Chunk(buffer); }
    })
}
