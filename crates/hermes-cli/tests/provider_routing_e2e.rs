//! Spec 005 closure E2E: two providers driven through the real binary.
//!
//! Both are wiremock-backed HTTP providers with distinct response text and
//! distinct credentials. The run starts on provider `alpha`, switches
//! mid-session to `beta`, and we then verify that:
//!   1. both responses are recorded in the same `state.db` session, in order;
//!   2. provider `alpha`'s credential never appears on any output path once
//!      `beta` is the active provider (and never leaks at all);
//!   3. the session survives the switch (turns are not mixed across providers).
//!
//! The mock servers run inside the test process on a multi-thread Tokio
//! runtime while the `hermes-rs` binary is spawned as a child that connects to
//! them over localhost — no real network is used.

use assert_cmd::Command;
use rusqlite::Connection;
use std::fs;
use tempfile::TempDir;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

const ALPHA_ENV: &str = "HERMES_E2E_ALPHA_KEY";
const BETA_ENV: &str = "HERMES_E2E_BETA_KEY";
const ALPHA_SECRET: &str = "sk-alpha-e2e-secret-1111111111";
const BETA_SECRET: &str = "sk-beta-e2e-secret-2222222222";

/// Builds one `data:` SSE line with the given token text, then `[DONE]`.
fn sse(body_lines: &[&str]) -> String {
    let mut out = String::new();
    for t in body_lines {
        // Test tokens are plain ASCII without quotes/backslashes, so this
        // inline JSON is safe.
        let payload = format!("{{\"choices\":[{{\"delta\":{{\"content\":\"{t}\"}}}}]}}");
        out.push_str(&format!("data: {payload}\n\n"));
    }
    out.push_str("data: [DONE]\n\n");
    out
}

fn canonical_messages(path: &std::path::Path) -> Vec<String> {
    let c = Connection::open(path).unwrap();
    let mut q = c
        .prepare("SELECT role, content FROM messages ORDER BY id")
        .unwrap();
    q.query_map([], |r| {
        Ok(format!(
            "{}|{}",
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?
        ))
    })
    .unwrap()
    .map(|r| r.unwrap())
    .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_providers_switch_mid_session_and_both_are_recorded() {
    // Two distinct, reachable HTTP endpoints.
    let alpha = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse(&["hello-from-alpha"])),
        )
        .mount(&alpha)
        .await;
    let beta = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse(&["hello-from-beta"])),
        )
        .mount(&beta)
        .await;

    let home = TempDir::new().unwrap();
    let config = format!(
        "providers:\n  alpha:\n    api: {}/v1\n    key_env: {ALPHA_ENV}\n    models:\n      m: {{}}\n  beta:\n    api: {}/v1\n    key_env: {BETA_ENV}\n    models:\n      m: {{}}\n",
        alpha.uri(),
        beta.uri()
    );
    fs::write(home.path().join("config.yaml"), config).unwrap();

    // Start on alpha, ask, switch to beta mid-session, ask again.
    let out = Command::cargo_bin("hermes-rs")
        .unwrap()
        .env(ALPHA_ENV, ALPHA_SECRET)
        .env(BETA_ENV, BETA_SECRET)
        .args(["--provider", "alpha", "--hermes-home", home.path().to_str().unwrap()])
        .write_stdin("first\n/provider beta\nsecond\n/exit\n")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("hello-from-alpha"),
        "alpha response not seen: {stdout:?}"
    );
    assert!(
        stdout.contains("hello-from-beta"),
        "beta response not seen: {stdout:?}"
    );

    // Neither provider's credential may appear on any output path.
    assert!(
        !stdout.contains(ALPHA_SECRET) && !stdout.contains(BETA_SECRET),
        "credential leaked: {stdout:?}"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains(ALPHA_SECRET) && !stderr.contains(BETA_SECRET),
        "credential leaked to stderr: {stderr:?}"
    );

    // Both responses live in the same session, recorded in turn order: the
    // alpha turn and the beta turn, each with its assistant reply. The switch
    // did not merge or drop a turn.
    let messages = canonical_messages(&home.path().join("state.db"));
    let joined = messages.join("\n");
    assert!(joined.contains("user|first"), "msgs={messages:?}");
    assert!(
        joined.contains("assistant|hello-from-alpha"),
        "msgs={messages:?}"
    );
    assert!(joined.contains("user|second"), "msgs={messages:?}");
    assert!(
        joined.contains("assistant|hello-from-beta"),
        "msgs={messages:?}"
    );
    // Sanity: alpha assistant turn precedes the beta user turn.
    let a_pos = joined.find("hello-from-alpha").unwrap();
    let b_pos = joined.find("hello-from-beta").unwrap();
    let second_pos = joined.find("user|second").unwrap();
    assert!(a_pos < second_pos && second_pos < b_pos, "msgs={messages:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switching_to_unavailable_provider_keeps_active_one_and_its_credential() {
    // Start on `alpha`, try to switch to `beta` whose env var is UNSET, and
    // confirm: build fails -> active provider unchanged -> the next prompt is
    // still served by alpha, and alpha's credential is the only one used.
    let alpha = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse(&["still-alpha"])),
        )
        .mount(&alpha)
        .await;

    let home = TempDir::new().unwrap();
    let config = format!(
        "providers:\n  alpha:\n    api: {}/v1\n    key_env: {ALPHA_ENV}\n    models:\n      m: {{}}\n  beta:\n    api: http://127.0.0.1:9/\n    key_env: {BETA_ENV}\n    models:\n      m: {{}}\n",
        alpha.uri()
    );
    fs::write(home.path().join("config.yaml"), config).unwrap();

    let out = Command::cargo_bin("hermes-rs")
        .unwrap()
        .env(ALPHA_ENV, ALPHA_SECRET)
        .env_remove(BETA_ENV) // beta key deliberately missing
        .args(["--provider", "alpha", "--hermes-home", home.path().to_str().unwrap()])
        .write_stdin("/provider beta\nafter-rollback\n/exit\n")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    // The failed switch names the missing variable, not a value, and reports
    // the active provider was kept.
    assert!(
        stderr.contains(BETA_ENV),
        "must name the missing env var: {stderr:?}"
    );
    assert!(
        stderr.contains("keeping provider alpha"),
        "rollback message expected: {stderr:?}"
    );

    // The next prompt is still served by alpha.
    assert!(
        stdout.contains("still-alpha"),
        "alpha should still answer after a failed switch: {stdout:?}"
    );

    // Alpha's credential never appears on output paths.
    assert!(!stdout.contains(ALPHA_SECRET) && !stderr.contains(ALPHA_SECRET));
    assert!(!stdout.contains(BETA_SECRET) && !stderr.contains(BETA_SECRET));
}
