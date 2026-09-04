#![cfg(unix)]
use assert_cmd::cargo::CommandCargoExt;
use nix::{
    sys::signal::{kill, Signal},
    unistd::Pid,
};
use std::{
    io::Write,
    net::SocketAddr,
    process::{Command, Stdio},
    sync::Arc,
    time::Duration,
};
use tempfile::TempDir;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::Notify,
    time::sleep,
};

#[tokio::test]
async fn sigint_during_active_stream_cancels_and_exits_130() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let first_chunk_sent = Arc::new(Notify::new());
    let notified = Arc::clone(&first_chunk_sent);
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = vec![0u8; 4096];
        let _ = socket.read(&mut request).await.unwrap();
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: keep-alive\r\n\r\n").await.unwrap();
        let chunk1 = b"data: {\"choices\":[{\"delta\":{\"content\":\"chunk1\"}}]}\n\n";
        socket
            .write_all(format!("{:x}\r\n", chunk1.len()).as_bytes())
            .await
            .unwrap();
        socket.write_all(chunk1).await.unwrap();
        socket.write_all(b"\r\n").await.unwrap();
        notified.notify_one();
        sleep(Duration::from_secs(10)).await;
    });

    let home = TempDir::new().unwrap();
    std::fs::write(
        home.path().join("config.yaml"),
        "model:\n  default: test-model\n",
    )
    .unwrap();
    let mut child = Command::cargo_bin("hermes-rs")
        .unwrap()
        .env("HERMES_HOME", home.path())
        .env("HERMES_API_KEY", "test-key")
        .args([
            "--provider",
            "openai",
            "--api-url",
            &format!("http://{}", addr),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(b"hello\n").unwrap();
    first_chunk_sent.notified().await;
    sleep(Duration::from_millis(100)).await;
    kill(Pid::from_raw(child.id() as i32), Signal::SIGINT).unwrap();
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
