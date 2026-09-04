//! Provider-neutral asynchronous chat contract.

use crate::conversation::{Event, Turn};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use thiserror::Error;

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
}

pub mod fake;
pub mod http;
mod redact;
pub mod sse;
pub use fake::FakeProvider;
pub use http::HttpProvider;
pub use redact::redact;
