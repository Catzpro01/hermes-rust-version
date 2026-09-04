use super::{redact::redact, sse::parse_chunk, EventStream, Provider, ProviderError};
use crate::{
    config::SecretString,
    conversation::{Event, Turn},
};
use async_stream::try_stream;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::Serialize;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use url::Url;

#[derive(Clone)]
pub struct HttpProvider {
    client: Client,
    base_url: Url,
    api_key: SecretString,
    model: String,
}
impl HttpProvider {
    pub fn new(base_url: Url, api_key: SecretString, model: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url,
            api_key,
            model: model.into(),
        }
    }
    pub fn with_client(
        client: Client,
        base_url: Url,
        api_key: SecretString,
        model: impl Into<String>,
    ) -> Self {
        Self {
            client,
            base_url,
            api_key,
            model: model.into(),
        }
    }
    pub async fn chat_with_cancel(
        &self,
        turns: &[Turn],
        cancel: CancellationToken,
    ) -> Result<EventStream, ProviderError> {
        let url = self
            .base_url
            .join("v1/chat/completions")
            .map_err(|e| ProviderError::Message(e.to_string()))?;
        let messages: Vec<_> = turns
            .iter()
            .map(|t| match t {
                Turn::User { content } => ApiMessage {
                    role: "user",
                    content,
                },
                Turn::Assistant { content } => ApiMessage {
                    role: "assistant",
                    content,
                },
                Turn::Tool { name: _, content } => ApiMessage {
                    role: "tool",
                    content,
                },
            })
            .collect();
        let request = self
            .client
            .post(url)
            .bearer_auth(self.api_key.expose())
            .json(&ChatRequest {
                model: &self.model,
                stream: true,
                messages,
            });
        let response = tokio::select! {
            _ = cancel.cancelled() => return Err(ProviderError::Cancelled),
            result = request.send() => result.map_err(|e| ProviderError::Message(redact(&e.to_string(), &self.api_key)))?,
        };
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::Http {
                status: status.as_u16(),
                message: redact(&body, &self.api_key),
            });
        }
        let mut bytes = response.bytes_stream();
        let api_key = self.api_key.clone();
        let stream = try_stream! { yield Event::Started; let mut remainder=Vec::new(); loop { let next=tokio::select! { _=cancel.cancelled()=>{ Err(ProviderError::Cancelled) }, item=bytes.next()=>Ok(item) }; let chunk=match next { Err(e)=>Err(e)?, Ok(None)=>break, Ok(Some(Err(e)))=>Err(ProviderError::Message(redact(&e.to_string(), &api_key)))?, Ok(Some(Ok(b)))=>b }; for event in parse_chunk(&chunk,&mut remainder)? { let done=matches!(event,Event::Done); yield event; if done { return; } } } if !remainder.is_empty() { for event in parse_chunk(b"\n",&mut remainder)? { yield event; } } };
        Ok(Box::pin(stream))
    }
}
#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    stream: bool,
    messages: Vec<ApiMessage<'a>>,
}
#[derive(Serialize)]
struct ApiMessage<'a> {
    role: &'a str,
    content: &'a str,
}
#[async_trait]
impl Provider for HttpProvider {
    async fn chat(&self, turns: &[Turn]) -> Result<EventStream, ProviderError> {
        self.chat_with_cancel(turns, CancellationToken::new()).await
    }
}

#[allow(dead_code)]
fn _timeout() -> Duration {
    Duration::from_secs(30)
}
