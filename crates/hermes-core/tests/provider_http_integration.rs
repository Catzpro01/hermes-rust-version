use futures::StreamExt;
use hermes_core::{
    config::{ApiMode, SecretString},
    conversation::{Event, Turn},
    provider::{HttpProvider, Provider},
};
use url::Url;
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, ResponseTemplate,
};

// Builds a `data:` SSE line whose JSON payload carries `content`, JSON-escaped
// so arbitrary token text (including `<tool_call ...>` markup) stays valid.
fn chat_sse(tokens: &[&str]) -> String {
    let mut out = String::new();
    for t in tokens {
        let payload =
            serde_json::json!({ "choices": [{ "delta": { "content": t } }] }).to_string();
        out.push_str(&format!("data: {payload}\n\n"));
    }
    out.push_str("data: [DONE]\n\n");
    out
}

// Same framing for the completions `text` field.
fn completions_sse(tokens: &[&str]) -> String {
    let mut out = String::new();
    for t in tokens {
        let payload = serde_json::json!({ "choices": [{ "text": t }] }).to_string();
        out.push_str(&format!("data: {payload}\n\n"));
    }
    out.push_str("data: [DONE]\n\n");
    out
}

#[tokio::test]
async fn chat_mode_streams_openai_sse_and_authenticates() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(chat_sse(&["hello", " world"])),
        )
        .mount(&server)
        .await;
    // `with_api_mode` is the explicit mode; default (not set) is also chat.
    let provider = HttpProvider::new(
        Url::parse(&(server.uri() + "/")).unwrap(),
        SecretString::from("test-key"),
        "test-model",
    );
    let stream = provider
        .chat(&[Turn::User {
            content: "hi".into(),
        }])
        .await
        .unwrap();
    let raw: Vec<Result<Event, hermes_core::provider::ProviderError>> = stream.collect().await;
    let events: Vec<Event> = raw.into_iter().map(Result::unwrap).collect();
    assert_eq!(
        events,
        vec![
            Event::Started,
            Event::Chunk("hello".into()),
            Event::Chunk(" world".into()),
            Event::Done
        ]
    );
}

#[tokio::test]
async fn completions_mode_targets_v1_completions_with_a_prompt_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/completions"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(completions_sse(&["hello", " world"])),
        )
        .mount(&server)
        .await;
    let provider = HttpProvider::new(
        Url::parse(&(server.uri() + "/")).unwrap(),
        SecretString::from("test-key"),
        "test-model",
    )
    .with_api_mode(ApiMode::Completions);
    let stream = provider
        .chat(&[Turn::User {
            content: "hi".into(),
        }])
        .await
        .unwrap();
    let raw: Vec<Result<Event, hermes_core::provider::ProviderError>> = stream.collect().await;
    let events: Vec<Event> = raw.into_iter().map(Result::unwrap).collect();
    assert_eq!(
        events,
        vec![
            Event::Started,
            Event::Chunk("hello".into()),
            Event::Chunk(" world".into()),
            Event::Done
        ]
    );

    // The legacy endpoint gets a single `prompt` (not role messages).
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let body = String::from_utf8_lossy(&requests[0].body).into_owned();
    assert!(body.contains("\"model\":\"test-model\""), "body: {body}");
    assert!(body.contains("\"stream\":true"), "body: {body}");
    assert!(body.contains("User: hi\\nAssistant:"), "body: {body}");
    assert!(!body.contains("\"messages\""), "no role messages expected: {body}");
}

#[tokio::test]
async fn both_modes_normalize_to_identical_event_streams() {
    // Equivalent token text must produce the same provider-neutral Event
    // sequence whether it arrived as `delta.content` (chat) or `text`
    // (completions).
    let tokens: &[&str] = &["alpha ", "beta ", "gamma"];

    let chat_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(chat_sse(tokens)),
        )
        .mount(&chat_server)
        .await;
    let chat_provider = HttpProvider::new(
        Url::parse(&(chat_server.uri() + "/")).unwrap(),
        SecretString::from("k"),
        "m",
    );

    let completions_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/completions"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(completions_sse(tokens)),
        )
        .mount(&completions_server)
        .await;
    let completions_provider = HttpProvider::new(
        Url::parse(&(completions_server.uri() + "/")).unwrap(),
        SecretString::from("k"),
        "m",
    )
    .with_api_mode(ApiMode::Completions);

    let turns = [Turn::User {
        content: "hi".into(),
    }];
    let chat_events: Vec<Event> = chat_provider
        .chat(&turns)
        .await
        .unwrap()
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .map(Result::unwrap)
        .collect();
    let completions_events: Vec<Event> = completions_provider
        .chat(&turns)
        .await
        .unwrap()
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .map(Result::unwrap)
        .collect();
    assert_eq!(chat_events, completions_events);
    assert_eq!(
        completions_events,
        vec![
            Event::Started,
            Event::Chunk("alpha ".into()),
            Event::Chunk("beta ".into()),
            Event::Chunk("gamma".into()),
            Event::Done
        ]
    );
}

#[tokio::test]
async fn tool_calls_are_parsed_in_completions_mode() {
    // The `text` field can carry Hermes XML tool tags; `tool_aware_stream` must
    // still surface a ToolCall event, proving the buffer is mode-agnostic.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(completions_sse(&[
                    "Let me check ",
                    "<tool_call id=\"7\">echo: x</tool_call>",
                    " done",
                ])),
        )
        .mount(&server)
        .await;
    let provider = HttpProvider::new(
        Url::parse(&(server.uri() + "/")).unwrap(),
        SecretString::from("k"),
        "m",
    )
    .with_api_mode(ApiMode::Completions);
    let events: Vec<Event> = provider
        .chat(&[Turn::User {
            content: "hi".into(),
        }])
        .await
        .unwrap()
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .map(Result::unwrap)
        .collect();
    let call = events
        .iter()
        .find_map(|e| match e {
            Event::ToolCall(c) if c.name == "echo" => Some(c),
            _ => None,
        })
        .expect("expected a parsed echo ToolCall in completions mode");
    assert_eq!(call.arguments, "x");
}

/// Runs a chat request and returns the error it produces (panicking on success).
/// `chat` returns a non-`Debug` stream on `Ok`, so errors must be extracted via
/// `match` rather than `Result::unwrap_err`.
async fn chat_error(
    provider: &HttpProvider,
    content: &str,
) -> hermes_core::provider::ProviderError {
    match provider
        .chat(&[Turn::User {
            content: content.into(),
        }])
        .await
    {
        Ok(_stream) => panic!("expected an error, got a stream"),
        Err(err) => err,
    }
}

#[tokio::test]
async fn transient_http_errors_are_retryable() {
    // Ticket 01: a rate-limit (429) and a 5xx must surface as an Http error
    // that is_retryable() is true, so retry/fallback can react.
    for status in [429u16, 503] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(reqwest::StatusCode::from_u16(status).unwrap())
                    .set_body_string("upstream busy"),
            )
            .mount(&server)
            .await;
        let provider = HttpProvider::new(
            Url::parse(&(server.uri() + "/")).unwrap(),
            SecretString::from("k"),
            "m",
        );
        let err = chat_error(&provider, "hi").await;
        assert!(matches!(
            err,
            hermes_core::provider::ProviderError::Http { status: s, .. } if s == status
        ));
        assert!(err.is_retryable(), "{status} error must be retryable: {err}");
    }
}

#[tokio::test]
async fn permanent_http_errors_are_not_retryable() {
    // Ticket 01: a 4xx (e.g. 400/401) is permanent and must not be retried.
    for status in [400u16, 401] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(reqwest::StatusCode::from_u16(status).unwrap()))
            .mount(&server)
            .await;
        let provider = HttpProvider::new(
            Url::parse(&(server.uri() + "/")).unwrap(),
            SecretString::from("k"),
            "m",
        );
        let err = chat_error(&provider, "hi").await;
        assert!(matches!(
            err,
            hermes_core::provider::ProviderError::Http { status: s, .. } if s == status
        ));
        assert!(!err.is_retryable(), "{status} must not be retryable: {err}");
    }
}

#[tokio::test]
async fn transport_timeout_yields_a_retryable_timeout() {
    // Ticket 01: a request that exceeds the client timeout must surface as
    // ProviderError::Timeout (not a generic Message), which is_retryable().
    // The default client uses a 30s timeout, which is too slow for a unit
    // test, so we inject a client with a short timeout via with_client.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(800)),
        )
        .mount(&server)
        .await;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(200))
        .build()
        .unwrap();
    let provider = HttpProvider::with_client(
        client,
        Url::parse(&(server.uri() + "/")).unwrap(),
        SecretString::from("k"),
        "m",
    );
    let err = chat_error(&provider, "hi").await;
    assert!(
        matches!(err, hermes_core::provider::ProviderError::Timeout),
        "expected a Timeout error, got: {err}"
    );
    assert!(err.is_retryable(), "timeout must be retryable");
}

/// Responder that fails the first request with `status` and then returns a 200
/// SSE success. Used to prove a single retry recovers.
struct FailOnceThenOk {
    count: std::sync::atomic::AtomicUsize,
    fail_status: u16,
    ok_body: String,
}
impl wiremock::Respond for FailOnceThenOk {
    fn respond(&self, _request: &wiremock::Request) -> ResponseTemplate {
        let n = self
            .count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if n == 0 {
            ResponseTemplate::new(reqwest::StatusCode::from_u16(self.fail_status).unwrap())
                .set_body_string("busy")
        } else {
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(self.ok_body.clone())
        }
    }
}

#[tokio::test]
async fn retry_recovers_after_a_transient_503() {
    // Ticket 02 recovery proof: attempt 1 returns 503, attempt 2 returns 200,
    // and the resulting SSE stream is consumed successfully.
    use hermes_core::provider::RetryPolicy;
    use std::time::Duration as StdDuration;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(FailOnceThenOk {
            count: std::sync::atomic::AtomicUsize::new(0),
            fail_status: 503,
            ok_body: chat_sse(&["recovered"]),
        })
        .mount(&server)
        .await;

    let provider = HttpProvider::new(
        Url::parse(&(server.uri() + "/")).unwrap(),
        SecretString::from("k"),
        "m",
    )
    .with_retry(RetryPolicy {
        max_attempts: 3,
        base_delay: StdDuration::from_millis(10),
        max_delay: StdDuration::from_millis(50),
    });

    let stream = provider
        .chat(&[Turn::User {
            content: "hi".into(),
        }])
        .await
        .unwrap();
    let events: Vec<Event> = stream
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .map(Result::unwrap)
        .collect();
    assert!(events.contains(&Event::Chunk("recovered".into())), "{events:?}");

    let received = server.received_requests().await.unwrap();
    assert_eq!(received.len(), 2, "one retry after the 503");
}

#[tokio::test]
async fn retry_exhausts_after_max_attempts_on_persistent_500() {
    // Ticket 02 exhaustion proof: server returns 500 every time; after the
    // configured max attempts the last error surfaces (no infinite loop).
    use hermes_core::provider::RetryPolicy;
    use std::time::Duration as StdDuration;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    let provider = HttpProvider::new(
        Url::parse(&(server.uri() + "/")).unwrap(),
        SecretString::from("k"),
        "m",
    )
    .with_retry(RetryPolicy {
        max_attempts: 4,
        base_delay: StdDuration::from_millis(1),
        max_delay: StdDuration::from_millis(5),
    });

    let err = chat_error(&provider, "hi").await;
    assert!(
        matches!(err, hermes_core::provider::ProviderError::Http { status: 500, .. }),
        "expected final Http 500, got {err}"
    );

    let received = server.received_requests().await.unwrap();
    assert_eq!(received.len(), 4, "must make exactly max_attempts requests");
}

#[tokio::test]
async fn non_retryable_error_does_not_retry() {
    // Ticket 02: a 400 is permanent -> exactly one request, immediate error.
    use hermes_core::provider::RetryPolicy;
    use std::time::Duration as StdDuration;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(400).set_body_string("bad"))
        .mount(&server)
        .await;

    let provider = HttpProvider::new(
        Url::parse(&(server.uri() + "/")).unwrap(),
        SecretString::from("k"),
        "m",
    )
    .with_retry(RetryPolicy {
        max_attempts: 5,
        base_delay: StdDuration::from_millis(1),
        max_delay: StdDuration::from_millis(5),
    });

    let err = chat_error(&provider, "hi").await;
    assert!(matches!(
        err,
        hermes_core::provider::ProviderError::Http { status: 400, .. }
    ));
    let received = server.received_requests().await.unwrap();
    assert_eq!(received.len(), 1, "non-retryable error must not retry");
}
