//! Spec 011 (MCP) — the stdio transport seam.
//!
//! [`McpTransport`] abstracts "write a buffer + flush" and "read one NDJSON
//! line" so protocol logic in the client is testable without a real child
//! process. [`StdioTransport`] is the production implementation over a spawned
//! child's stdin/stdout; tests inject a scripted in-memory transport.

use super::error::McpError;
use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};

/// Maximum size (bytes) of a single inbound JSON-RPC message line. Guards
/// against an MCP server sending an oversized response (DoS). A message over
/// this bound is rejected rather than processed.
pub const MAX_MESSAGE_BYTES: usize = 10 * 1024 * 1024; // 10 MB

/// Byte-oriented transport over which NDJSON JSON-RPC messages flow.
#[async_trait]
pub trait McpTransport: Send {
    /// Writes `buf` (one JSON-RPC message plus trailing newline) to the peer.
    async fn write_line(&mut self, buf: &[u8]) -> Result<(), McpError>;
    /// Flushes buffered output so the peer receives it promptly.
    async fn flush(&mut self) -> Result<(), McpError>;
    /// Reads one line (without trailing newline). `Ok(None)` on a clean EOF
    /// (peer closed the stream).
    async fn read_line(&mut self) -> Result<Option<String>, McpError>;
}

/// Production transport bound to a spawned MCP child process's stdin/stdout.
pub struct StdioTransport {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}
impl StdioTransport {
    /// Splits a spawned `Child` into a transport. The caller keeps the `Child`
    /// itself (for exit id / kill-on-drop in the lifecycle layer).
    pub fn take(child: &mut Child) -> Result<Self, McpError> {
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpError::Transport("child has no stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpError::Transport("child has no stdout".into()))?;
        Ok(Self {
            stdin,
            stdout: BufReader::new(stdout),
        })
    }
}
#[async_trait]
impl McpTransport for StdioTransport {
    async fn write_line(&mut self, buf: &[u8]) -> Result<(), McpError> {
        self.stdin
            .write_all(buf)
            .await
            .map_err(|e| McpError::Transport(format!("write stdin: {e}")))?;
        self.stdin
            .write_all(b"\n")
            .await
            .map_err(|e| McpError::Transport(format!("write stdin newline: {e}")))?;
        Ok(())
    }
    async fn flush(&mut self) -> Result<(), McpError> {
        self.stdin
            .flush()
            .await
            .map_err(|e| McpError::Transport(format!("flush stdin: {e}")))
    }
    async fn read_line(&mut self) -> Result<Option<String>, McpError> {
        let mut buf = String::new();
        let n = self
            .stdout
            .read_line(&mut buf)
            .await
            .map_err(|e| McpError::Transport(format!("read stdout: {e}")))?;
        if n == 0 {
            return Ok(None); // EOF
        }
        let trimmed = buf.trim_end_matches(['\n', '\r']);
        Ok(Some(trimmed.to_owned()))
    }
}
