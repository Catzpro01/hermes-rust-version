//! Provider-neutral asynchronous chat contract.

use crate::conversation::{Event, Turn};
use async_trait::async_trait;
use futures::Stream;
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
