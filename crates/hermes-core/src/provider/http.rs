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

/// Retry configuration for transient, pre-stream failures. Bounded and
/// injectable so tests can force fast exhaustion/recovery without waiting on
/// real backoff delays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Maximum number of send attempts (>= 1). One attempt, then up to
    /// `max_attempts - 1` retries.
    pub max_attempts: u32,
    /// Base backoff delay, doubled after each failed attempt.
    pub base_delay: Duration,
    /// Upper bound on the backoff delay.
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(200),
            max_delay: Duration::from_millis(2000),
        }
    }
}

/// Backoff delay to wait after `failed_attempt` (1-based) has failed, before
/// attempting again. Formula: `min(base_delay * 2^(failed_attempt - 1),
/// max_delay)`. Pure so it is unit-testable without sleeping.
fn backoff_delay(policy: &RetryPolicy, failed_attempt: u32) -> Duration {
    let exponent = (failed_attempt.saturating_sub(1)).min(20);
    // Compute in u128 to avoid overflow from the shift, then cap and narrow.
    let raw_millis = policy.base_delay.as_millis().saturating_mul(1u128 << exponent);
    let capped = raw_millis.min(policy.max_delay.as_millis());
    Duration::from_millis(u64::try_from(capped).unwrap_or(u64::MAX))
}

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
    retry: RetryPolicy,
}
impl HttpProvider {
    pub fn new(base_url: Url, api_key: SecretString, model: impl Into<String>) -> Self {
        Self {
            client: default_client(),
            base_url,
            api_key,
            model: model.into(),
            api_mode: ApiMode::ChatCompletions,
            retry: RetryPolicy::default(),
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
            retry: RetryPolicy::default(),
        }
    }

    /// Selects the wire mode. `chat_completions` (the default) preserves the
    /// original behaviour; `completions` talks to `v1/completions`.
    pub fn with_api_mode(mut self, api_mode: ApiMode) -> Self {
        self.api_mode = api_mode;
        self
    }

    /// Overrides the retry policy (used by tests to force fast retries).
    pub fn with_retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
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
        instruction: Option<&str>,
    ) -> Result<reqwest::RequestBuilder, ProviderError> {
        let url = self
            .base_url
            .join(self.endpoint_path())
            .map_err(|e| ProviderError::Message(e.to_string()))?;
        let builder = self.client.post(url).bearer_auth(self.api_key.expose());
        let request = match self.api_mode {
            ApiMode::ChatCompletions => {
                let messages = build_chat_messages(turns, instruction);
                builder.json(&ChatRequest {
                    model: &self.model,
                    stream: true,
                    messages,
                })
            }
            ApiMode::Completions => {
                let prompt = render_completions_prompt_with_instruction(turns, instruction);
                builder.json(&CompletionRequest {
                    model: &self.model,
                    stream: true,
                    prompt: &prompt,
                })
            }
        };
        Ok(request)
    }

    /// One pre-stream request attempt: build, send (honouring cancellation),
    /// and return the 2xx response or a classified error. Non-2xx and send
    /// failures become a [`ProviderError`] here, before any stream is consumed,
    /// so a retry/fallback can react cleanly.
    async fn attempt(
        &self,
        turns: &[Turn],
        instruction: Option<&str>,
        cancel: &CancellationToken,
    ) -> Result<reqwest::Response, ProviderError> {
        let request = self.build_request(turns, instruction)?;
        let response = tokio::select! {
            _ = cancel.cancelled() => return Err(ProviderError::Cancelled),
            result = request.send() => match result {
                Ok(response) => response,
                // A transport timeout is distinct from a generic message error
                // so it can be classified retryable.
                Err(e) if e.is_timeout() => return Err(ProviderError::Timeout),
                Err(e) => {
                    return Err(ProviderError::Message(redact(&e.to_string(), &self.api_key)))
                }
            },
        };
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::Http {
                status: status.as_u16(),
                message: redact(&body, &self.api_key),
            });
        }
        Ok(response)
    }

    /// Sends the request with bounded exponential backoff retries for
    /// pre-stream, retryable failures (Ticket 02). Non-retryable errors and
    /// `Cancelled` return immediately on the first attempt. Between attempts we
    /// `tokio::time::sleep` inside a `tokio::select!` against the cancel token,
    /// so a SIGINT mid-backoff exits with `Cancelled` instead of waiting out
    /// the timer.
    async fn send_with_retry(
        &self,
        turns: &[Turn],
        instruction: Option<&str>,
        cancel: &CancellationToken,
    ) -> Result<reqwest::Response, ProviderError> {
        let mut attempt: u32 = 1;
        loop {
            match self.attempt(turns, instruction, cancel).await {
                Ok(response) => return Ok(response),
                Err(err) => {
                    if err.is_retryable() && attempt < self.retry.max_attempts {
                        let delay = backoff_delay(&self.retry, attempt);
                        tokio::select! {
                            _ = cancel.cancelled() => return Err(ProviderError::Cancelled),
                            _ = tokio::time::sleep(delay) => {}
                        }
                        attempt += 1;
                    } else {
                        return Err(err);
                    }
                }
            }
        }
    }

    /// Low-level send returning the **raw** SSE event stream (no tool-tag
    /// parsing). `instruction` is sent as an ephemeral system-level instruction
    /// when present. Used by normal chat (`None`, then tool-parsed by the
    /// [`Provider`] impl) and by plan generation (`Some`, read as plain text so
    /// a literal `<tool_call>` in the plan cannot trigger the tool parser).
    async fn chat_with_cancel_raw(
        &self,
        turns: &[Turn],
        instruction: Option<&str>,
        cancel: CancellationToken,
    ) -> Result<EventStream, ProviderError> {
        let response = self.send_with_retry(turns, instruction, &cancel).await?;
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

/// Builds the `chat.completions` message list. When an ephemeral `instruction`
/// is present it is prepended as a `system` message; otherwise the list is
/// exactly the conversation turns. Pure and unit-testable.
fn build_chat_messages<'a>(turns: &'a [Turn], instruction: Option<&'a str>) -> Vec<ApiMessage<'a>> {
    let mut messages = Vec::with_capacity(turns.len() + usize::from(instruction.is_some()));
    if let Some(instr) = instruction {
        messages.push(ApiMessage {
            role: "system",
            content: instr,
        });
    }
    for t in turns {
        messages.push(match t {
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
        });
    }
    messages
}

/// Renders the completions `prompt`, optionally prefixed with an ephemeral
/// instruction header (there is no per-message role in this mode).
fn render_completions_prompt_with_instruction(
    turns: &[Turn],
    instruction: Option<&str>,
) -> String {
    let transcript = render_completions_prompt(turns);
    match instruction {
        Some(instr) => format!("[Instruction] {instr}\n\n{transcript}"),
        None => transcript,
    }
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

    // Override so cancellation passes through a `Box<dyn Provider>` (the shape
    // the runner and FallbackProvider hold). The raw method honours the token
    // both before the request (send_with_retry) and during the SSE stream; here
    // we additionally apply tool-tag parsing so the result is indistinguishable
    // from `chat`. Without this override, dynamic dispatch would fall to the
    // trait default (`self.chat()`), which builds a fresh token and silently
    // ignores the caller's cancellation.
    async fn chat_with_cancel(
        &self,
        turns: &[Turn],
        cancel: CancellationToken,
    ) -> Result<EventStream, ProviderError> {
        self.chat_with_cancel_raw(turns, None, cancel)
            .await
            .map(tool_aware_stream)
    }

    // Spec 009 (Ticket 02): like `chat_with_cancel` (tool-aware), but with an
    // ephemeral `system` instruction prepended when present. The plan/execution
    // stream is still tool-parsed so the model may call tools while following a
    // plan; a stray literal `<tool_call>` inside a *plan* is never executed by
    // the runner because plan generation reads only the assistant text.
    async fn chat_with_instruction(
        &self,
        turns: &[Turn],
        instruction: Option<&str>,
        cancel: CancellationToken,
    ) -> Result<EventStream, ProviderError> {
        self.chat_with_cancel_raw(turns, instruction, cancel)
            .await
            .map(tool_aware_stream)
    }
}

/// Total per-request timeout applied to the internally-built HTTP client.
/// Prevents a request from hanging indefinitely (a DoS vector) and lets a
/// timeout surface as a retryable [`ProviderError::Timeout`].
const HTTP_CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

/// Builds the default client with the request timeout applied.
fn default_client() -> Client {
    Client::builder()
        .timeout(HTTP_CLIENT_TIMEOUT)
        .build()
        .expect("reqwest client build should not fail")
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

    #[test]
    fn chat_messages_prepend_a_system_instruction_when_present() {
        let turns = [Turn::User {
            content: "hi".into(),
        }];
        // Without instruction: exactly the turn (no system message).
        let none = build_chat_messages(&turns, None);
        assert_eq!(none.len(), 1);
        assert_eq!(none[0].role, "user");
        // With instruction: a system message leads, then the turns in order.
        let with = build_chat_messages(&turns, Some("produce a plan"));
        assert_eq!(with.len(), 2);
        assert_eq!(with[0].role, "system");
        assert_eq!(with[0].content, "produce a plan");
        assert_eq!(with[1].role, "user");
        assert_eq!(with[1].content, "hi");
    }

    #[test]
    fn completions_prompt_prepends_an_instruction_header_only_when_present() {
        let turns = [Turn::User {
            content: "hi".into(),
        }];
        let base = render_completions_prompt(&turns);
        assert_eq!(
            render_completions_prompt_with_instruction(&turns, None),
            base,
            "None must be identical to the plain transcript"
        );
        let with = render_completions_prompt_with_instruction(&turns, Some("do X"));
        assert!(with.starts_with("[Instruction] do X\n\n"), "got: {with}");
        assert!(with.ends_with(&base), "transcript must follow the header");
    }

    #[test]
    fn http_client_timeout_is_thirty_seconds() {
        // Ticket 01: a real (not dead) 30s request timeout. Changing this is a
        // deliberate, test-visible act.
        assert_eq!(HTTP_CLIENT_TIMEOUT, Duration::from_secs(30));
    }

    #[test]
    fn default_retry_policy_is_bounded_and_reasonable() {
        // Ticket 02: 3 attempts max, 200ms base, 2000ms cap.
        let p = RetryPolicy::default();
        assert_eq!(p.max_attempts, 3);
        assert_eq!(p.base_delay, Duration::from_millis(200));
        assert_eq!(p.max_delay, Duration::from_millis(2000));
    }

    #[test]
    fn backoff_delay_doubles_then_caps() {
        // With base 100ms / cap 1000ms: attempt 1 -> 100, attempt 2 -> 200,
        // attempt 3 -> 400, attempt 4 -> 800, attempt 5+ -> capped at 1000.
        let p = RetryPolicy {
            max_attempts: 6,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(1000),
        };
        let expect = [100u64, 200, 400, 800, 1000, 1000];
        for (i, ms) in expect.iter().enumerate() {
            let attempt = (i + 1) as u32;
            assert_eq!(
                backoff_delay(&p, attempt),
                Duration::from_millis(*ms),
                "attempt {attempt}"
            );
        }
    }

    #[test]
    fn backoff_delay_never_exceeds_max_delay_even_at_large_exponent() {
        let p = RetryPolicy {
            max_attempts: 100,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(1500),
        };
        assert_eq!(backoff_delay(&p, 1), Duration::from_millis(100));
        assert_eq!(backoff_delay(&p, 60), p.max_delay);
        assert_eq!(backoff_delay(&p, 200), p.max_delay);
    }
}
