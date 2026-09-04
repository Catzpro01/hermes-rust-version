#![cfg(unix)]
use assert_cmd::cargo::CommandCargoExt;
use nix::{
    sys::signal::{kill, Signal},
    unistd::Pid,
};
use std::{
    io::Write,
    process::{Command, Stdio},
    time::Duration,
};
use tempfile::TempDir;
use tokio::time::sleep;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn sigint_during_active_stream_cancels_and_exits_130() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(200)
        .set_body_raw("data: {\"choices\":[{\"delta\":{\"content\":\"chunk1\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"chunk2\"}}]}\n\ndata: [DONE]\n\n", "text/event-stream")
        .set_delay(Duration::from_millis(800))).mount(&server).await;
    let home = TempDir::new().unwrap();
    std::fs::write(
        home.path().join("config.yaml"),
        "model:\n  default: test-model
",
    )
    .unwrap();
    let mut child = Command::cargo_bin("hermes-rs")
        .unwrap()
        .env("HERMES_HOME", home.path())
        .env("HERMES_API_KEY", "test-key")
        .args(["--provider", "openai", "--api-url", &server.uri()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(b"hello\n").unwrap();
    sleep(Duration::from_millis(300)).await;
    kill(Pid::from_raw(child.id() as i32), Signal::SIGINT).unwrap();
    drop(stdin);
    let status = tokio::task::spawn_blocking(move || child.wait())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status.code(), Some(130));
    let db = home.path().join("state.db");
    if db.exists() {
        let conn = rusqlite::Connection::open(db).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM messages WHERE role = 'assistant'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        assert_eq!(count, 0);
    }
}
