//! Wiremock end-to-end coverage for the provider fallback chain (Spec 006 #03).
//!
//! Proves, over real two-server HTTP:
//! 1. When provider A persistently fails (5xx) the same turn is transparently
//!    retried on provider B and B's response is the one consumed.
//! 2. Credential isolation: provider A's key never reaches B's endpoint and
//!    vice-versa (each hop authenticates with its own key only).
//! 3. A successful answer flows through the `ConversationRunner` into the
//!    session store, so B's text is what ends up in `state.db`.

use futures::StreamExt;
use hermes_core::{
    config::SecretString,
    conversation::{ConversationRunner, Event, Turn},
    provider::{FallbackProvider, HttpProvider, Provider, ProviderError, RetryPolicy},
    session::SessionStore,
};
use std::time::Duration as StdDuration;
use url::Url;
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

/// A fast retry policy so multi-hop tests finish in milliseconds, not seconds.
fn fast_retry() -> RetryPolicy {
    RetryPolicy {
        max_attempts: 2,
        base_delay: StdDuration::from_millis(1),
        max_delay: StdDuration::from_millis(5),
    }
}

fn chat_sse(tokens: &[&str]) -> String {
    let mut out = String::new();
    for t in tokens {
        let payload = serde_json::json!({ "choices": [{ "delta": { "content": t } }] }).to_string();
        out.push_str(&format!("data: {payload}\n\n"));
    }
    out.push_str("data: [DONE]\n\n");
    out
}

fn http_provider(uri: &str, key: &str) -> HttpProvider {
    HttpProvider::new(
        Url::parse(&(uri.to_owned() + "/")).unwrap(),
        SecretString::from(key),
        "m",
    )
    .with_retry(fast_retry())
}

#[tokio::test]
async fn falls_back_to_b_when_a_is_persistently_down() {
    // Server A is permanently failing (500 on every request).
    let server_a = MockServer::start().await;
    Mock::given(wiremock::matchers::method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("A is down"))
        .mount(&server_a)
        .await;

    // Server B is healthy and requires B's key.
    let server_b = MockServer::start().await;
    Mock::given(wiremock::matchers::method("POST"))
        .and(path("/v1/chat/completions"))
        .and(wiremock::matchers::header("authorization", "Bearer b-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(chat_sse(&["hello-from-b"])),
        )
        .mount(&server_b)
        .await;

    let chain = FallbackProvider::new(vec![
        ("a".into(), Box::new(http_provider(&server_a.uri(), "a-key"))),
        ("b".into(), Box::new(http_provider(&server_b.uri(), "b-key"))),
    ]);

    let stream = chain
        .chat(&[Turn::User {
            content: "ping".into(),
        }])
        .await
        .expect("fallback must recover via B");

    let raw: Vec<Result<Event, ProviderError>> = stream.collect().await;
    let events: Vec<Event> = raw.into_iter().map(Result::unwrap).collect();
    assert!(
        events.contains(&Event::Chunk("hello-from-b".into())),
        "must serve B's content: {events:?}"
    );

    // Credential isolation: A saw only A's key; B saw only B's key.
    let requests_a = server_a.received_requests().await.unwrap();
    assert!(!requests_a.is_empty(), "fallback must have actually tried A");
    for req in &requests_a {
        let auth = req
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(auth, "Bearer a-key", "A must only see its own key");
        assert_ne!(auth, "Bearer b-key", "B's key must never reach A");
    }
    let requests_b = server_b.received_requests().await.unwrap();
    assert!(!requests_b.is_empty(), "fallback must have reached B");
    for req in &requests_b {
        let auth = req
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(auth, "Bearer b-key", "B must only see its own key");
        assert_ne!(auth, "Bearer a-key", "A's key must never reach B");
    }
}

#[tokio::test]
async fn b_response_is_what_gets_stored_in_state_db() {
    // Drive the same two-server fallback through the real ConversationRunner
    // and SessionStore, then confirm the persisted assistant text is B's.
    let server_a = MockServer::start().await;
    Mock::given(wiremock::matchers::method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("A down"))
        .mount(&server_a)
        .await;
    let server_b = MockServer::start().await;
    Mock::given(wiremock::matchers::method("POST"))
        .and(path("/v1/chat/completions"))
        .and(wiremock::matchers::header("authorization", "Bearer b-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(chat_sse(&["stored-from-b"])),
        )
        .mount(&server_b)
        .await;

    let chain = FallbackProvider::new(vec![
        ("a".into(), Box::new(http_provider(&server_a.uri(), "a-key"))),
        ("b".into(), Box::new(http_provider(&server_b.uri(), "b-key"))),
    ]);

    let dir = tempfile::tempdir().unwrap();
    let mut store = SessionStore::open(&dir.path().join("state.db")).unwrap();
    let session_id = store.create_session("fallback-e2e").unwrap();

    let mut runner = ConversationRunner::from_turns(chain, Vec::new());
    // `chat` records the user turn and streams the provider's answer.
    let stream = runner
        .chat("ping")
        .await
        .expect("fallback must answer via B");
    let raw: Vec<Result<Event, ProviderError>> = stream.collect().await;
    let text: String = raw
        .into_iter()
        .filter_map(|item| match item {
            Ok(Event::Chunk(c)) => Some(c),
            _ => None,
        })
        .collect();
    assert_eq!(text, "stored-from-b", "assistant text must come from B");
    runner.push_assistant(text);

    for turn in runner.turns() {
        store.save_turn(&session_id, turn).unwrap();
    }

    // Reopen from disk to prove it was actually persisted, and assert B's text.
    let store2 = SessionStore::open(&dir.path().join("state.db")).unwrap();
    let session = store2.resume(&session_id).unwrap();
    let assistant: Vec<&String> = session
        .turns
        .iter()
        .filter_map(|t| match t {
            Turn::Assistant { content } => Some(content),
            _ => None,
        })
        .collect();
    assert_eq!(assistant, vec!["stored-from-b"], "B's answer must be in state.db");

    // And confirm the persisted session never surfaced A's text.
    assert!(
        !session
            .turns
            .iter()
            .any(|t| matches!(t, Turn::Assistant { content } if content == "a-failed")),
        "nothing from the failing provider may be persisted"
    );
}

#[tokio::test]
async fn all_hops_down_yields_aggregate_error_naming_each_provider() {
    let server_a = MockServer::start().await;
    Mock::given(wiremock::matchers::method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server_a)
        .await;
    let server_b = MockServer::start().await;
    Mock::given(wiremock::matchers::method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server_b)
        .await;

    let chain = FallbackProvider::new(vec![
        ("a".into(), Box::new(http_provider(&server_a.uri(), "a-key"))),
        ("b".into(), Box::new(http_provider(&server_b.uri(), "b-key"))),
    ]);

    let err = match chain
        .chat(&[Turn::User {
            content: "ping".into(),
        }])
        .await
    {
        Ok(_) => panic!("expected aggregate error when every provider is down"),
        Err(err) => err,
    };
    assert!(
        matches!(
            &err,
            ProviderError::Fallback { tried } if tried == &vec!["a".to_owned(), "b".to_owned()]
        ),
        "aggregate must name the tried providers: {err}"
    );
}
