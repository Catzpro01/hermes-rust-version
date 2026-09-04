use super::{
    redact::redact, sse::parse_chunk, tool_aware_stream, EventStream, Provider, ProviderError,
};
use crate::{
    config::{ApiMode, SecretString},
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

/// An OpenAI-compatible provider, speaking either the chat-completions or the
/// (legacy) completions wire format depending on [`ApiMode`].
///
/// Routing is decided once per provider at construction time via
/// [`HttpProvider::with_api_mode`]; the selected mode fixes the request
/// endpoint, the request body shape, and which SSE field carries the token
/// text (`delta.content` for chat, `text` for completions). Both modes still
/// yield the same provider-neutral [`Event`] sequence, so downstream code
/// (e.g. `tool_aware_stream`) is mode-agnostic.
#[derive(Clone)]
pub struct HttpProvider {
    client: Client,
    base_url: Url,
    api_key: SecretString,
    model: String,
    api_mode: ApiMode,
}
impl HttpProvider {
    pub fn new(base_url: Url, api_key: SecretString, model: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url,
            api_key,
            model: model.into(),
            api_mode: ApiMode::ChatCompletions,
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
            api_mode: ApiMode::ChatCompletions,
        }
    }

    /// Selects the wire mode. `chat_completions` (the default) preserves the
    /// original behaviour; `completions` talks to `v1/completions`.
    pub fn with_api_mode(mut self, api_mode: ApiMode) -> Self {
        self.api_mode = api_mode;
        self
    }

    fn endpoint_path(&self) -> &'static str {
        match self.api_mode {
            ApiMode::ChatCompletions => "v1/chat/completions",
            ApiMode::Completions => "v1/completions",
        }
    }

    /// Builds the request: URL + bearer auth + body already serialized by
    /// `reqwest::RequestBuilder::json`, which buffers the payload immediately.
    fn build_request(
        &self,
        turns: &[Turn],
    ) -> Result<reqwest::RequestBuilder, ProviderError> {
        let url = self
            .base_url
            .join(self.endpoint_path())
            .map_err(|e| ProviderError::Message(e.to_string()))?;
        let builder = self.client.post(url).bearer_auth(self.api_key.expose());
        let request = match self.api_mode {
            ApiMode::ChatCompletions => {
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
                builder.json(&ChatRequest {
                    model: &self.model,
                    stream: true,
                    messages,
                })
            }
            ApiMode::Completions => {
                let prompt = render_completions_prompt(turns);
                builder.json(&CompletionRequest {
                    model: &self.model,
                    stream: true,
                    prompt: &prompt,
                })
            }
        };
        Ok(request)
    }

    pub async fn chat_with_cancel(
        &self,
        turns: &[Turn],
        cancel: CancellationToken,
    ) -> Result<EventStream, ProviderError> {
        let request = self.build_request(turns)?;
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

/// The two request bodies a provider can send, selected by `api_mode`. Each is
/// an OpenAI-compatible streaming request.
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
#[derive(Serialize)]
struct CompletionRequest<'a> {
    model: &'a str,
    stream: bool,
    prompt: &'a str,
}

/// Turns the provider-neutral conversation into a single `prompt` for the
/// legacy completions endpoint, which has no per-message roles.
///
/// The transcript is a deterministic linear rendering that keeps each turn's
/// author and content visible to the model, including the result of previous
/// tool executions (the model observed no assistant tool-call text in the
/// stored history, so only tool results are surfaced). The transcript ends
/// with an unclosed `Assistant:` cue so the model knows it is its turn to
/// speak. Absent a stricter contract this format is deliberately simple and
/// documented; changing it only affects `completions` mode.
fn render_completions_prompt(turns: &[Turn]) -> String {
    let mut out = String::new();
    for turn in turns {
        match turn {
            Turn::User { content } => {
                out.push_str("User: ");
                out.push_str(content);
            }
            Turn::Assistant { content } => {
                out.push_str("Assistant: ");
                out.push_str(content);
            }
            Turn::Tool { name, content } => {
                out.push_str("Tool result (");
                out.push_str(name);
                out.push_str("): ");
                out.push_str(content);
            }
        }
        out.push('\n');
    }
    out.push_str("Assistant:");
    out
}

#[async_trait]
impl Provider for HttpProvider {
    async fn chat(&self, turns: &[Turn]) -> Result<EventStream, ProviderError> {
        self.chat_with_cancel(turns, CancellationToken::new())
            .await
            .map(tool_aware_stream)
    }
}

#[allow(dead_code)]
fn _timeout() -> Duration {
    Duration::from_secs(30)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completions_prompt_renders_a_linear_transcript() {
        let prompt = render_completions_prompt(&[
            Turn::User {
                content: "list files".into(),
            },
            Turn::Tool {
                name: "read_dir".into(),
                content: "src".into(),
            },
            Turn::Assistant {
                content: "Done.".into(),
            },
        ]);
        assert_eq!(
            prompt,
            "User: list files\nTool result (read_dir): src\nAssistant: Done.\nAssistant:"
        );
    }

    #[test]
    fn completions_prompt_ends_with_an_assistant_cue() {
        let prompt = render_completions_prompt(&[Turn::User {
            content: "hi".into(),
        }]);
        assert_eq!(prompt, "User: hi\nAssistant:");
    }

    #[test]
    fn empty_turns_still_close_with_an_assistant_cue() {
        assert_eq!(render_completions_prompt(&[]), "Assistant:");
    }
}
