//! Provider-neutral asynchronous chat contract.

use crate::conversation::{Event, Turn};
use crate::tools::{parse_tool_events, parser::ToolEvent};
use async_stream::try_stream;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

/// HTTP status codes that indicate a transient upstream failure worth retrying
/// (rate limit or a 5xx server error). Kept as an explicit constant so that
/// changing the set is a deliberate, test-visible act rather than scattered
/// magic numbers.
pub const RETRYABLE_HTTP_STATUS: &[u16] = &[429, 500, 502, 503, 504];

#[derive(Debug, Error, Clone)]
pub enum ProviderError {
    #[error("provider error: {0}")]
    Message(String),
    #[error("provider returned HTTP {status}: {message}")]
    Http { status: u16, message: String },
    #[error("provider request timed out")]
    Timeout,
    #[error("request cancelled")]
    Cancelled,
    #[error("all providers in the fallback chain failed (tried: {})", tried.join(", "))]
    Fallback { tried: Vec<String> },
}

impl ProviderError {
    /// Whether the failure is transient enough to retry. Only rate limits
    /// (429), 5xx server errors (500/502/503/504), and an explicit transport
    /// timeout are retried. Anything else — other 4xx, a generic transport
    /// `Message`, or a user `Cancelled` — is not retried, so a permanent
    /// rejection or a cancelled request is surfaced immediately.
    pub fn is_retryable(&self) -> bool {
        match self {
            ProviderError::Http { status, .. } => RETRYABLE_HTTP_STATUS.contains(status),
            ProviderError::Timeout => true,
            // A fallback-exhausted error is terminal for this request; each hop
            // already ran its own retry policy before the chain gave up.
            ProviderError::Message(_) | ProviderError::Cancelled | ProviderError::Fallback { .. } => {
                false
            }
        }
    }
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
pub mod fallback;
pub mod health;
pub mod http;
pub mod registry;
mod redact;
pub mod sse;
pub use fake::FakeProvider;
pub use fallback::FallbackProvider;
pub use health::{HealthTracker, DEFAULT_COOLDOWN};
pub use http::{HttpProvider, RetryPolicy};
pub use redact::redact;
pub use registry::{ProviderRegistry, RegistryError, FAKE_PROVIDER};

/// Converts streamed text containing Hermes XML tool tags into typed events.
pub fn tool_aware_stream(mut input: EventStream) -> EventStream {
    Box::pin(try_stream! {
        let mut buffer = String::new();
        while let Some(item) = input.next().await {
            let event = item?;
            match event {
                Event::Chunk(text) => {
                    buffer.push_str(&text);
                    if let Some(start) = buffer.find("<tool_call") {
                        if start > 0 { yield Event::Chunk(buffer[..start].to_owned()); buffer.drain(..start); }
                        if let Some(end) = buffer.find("</tool_call>") {
                            let end = end + "</tool_call>".len();
                            let xml = buffer[..end].to_owned(); buffer.drain(..end);
                            match parse_tool_events(&xml) {
                                Ok(events) => for parsed in events { if let ToolEvent::Call(call) = parsed { yield Event::ToolCall(call); } },
                                Err(_) => yield Event::Chunk(xml),
                            }
                            if !buffer.is_empty() { yield Event::Chunk(std::mem::take(&mut buffer)); }
                        }
                    } else {
                        let marker = "<tool_call";
                        let keep = (1..marker.len()).rev().find(|n| buffer.ends_with(&marker[..*n])).unwrap_or(0);
                        if keep == 0 { yield Event::Chunk(std::mem::take(&mut buffer)); }
                        else if buffer.len() > keep { let split=buffer.len()-keep; let text=buffer[..split].to_owned(); buffer.drain(..split); yield Event::Chunk(text); }
                    }
                }
                other => yield other,
            }
        }
        if !buffer.is_empty() { yield Event::Chunk(buffer); }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn http(status: u16) -> ProviderError {
        ProviderError::Http {
            status,
            message: "upstream body".to_owned(),
        }
    }

    #[test]
    fn retryable_status_codes_are_exact() {
        // The explicit transient set: 429 and the 5xx family 500/502/503/504.
        assert_eq!(RETRYABLE_HTTP_STATUS, &[429, 500, 502, 503, 504]);
        for status in [429u16, 500, 502, 503, 504] {
            assert!(http(status).is_retryable(), "{status} should be retryable");
        }
    }

    #[test]
    fn non_retryable_http_and_others_are_not_retried() {
        // Other 4xx (e.g. 400 bad request, 401, 404, 422) are permanent.
        for status in [400u16, 401, 403, 404, 409, 422] {
            assert!(!http(status).is_retryable(), "{status} must not retry");
        }
        // A 5xx outside the explicit set is not retried by this classification.
        assert!(!http(501).is_retryable());
        // Generic transport message, a user cancellation, and a fallback chain
        // exhaustion are not retried (each hop already retried internally).
        assert!(!ProviderError::Message("boom".into()).is_retryable());
        assert!(!ProviderError::Cancelled.is_retryable());
        assert!(!ProviderError::Fallback {
            tried: vec!["a".into(), "b".into()]
        }
        .is_retryable());
    }

    #[test]
    fn timeout_is_retryable_but_cancelled_is_not() {
        assert!(ProviderError::Timeout.is_retryable());
        assert!(!ProviderError::Cancelled.is_retryable());
    }
}
