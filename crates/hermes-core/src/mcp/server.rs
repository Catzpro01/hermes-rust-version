//! Spec 011 (MCP) — a live connection to one MCP server child process.
//!
//! [`McpServer`] spawns the child (from a [`McpServerConfig`]), performs the
//! `initialize` handshake, and keeps the [`McpClient`] behind an `Arc` mutex so
//! many [`crate::mcp::tool::McpTool`] wrappers (one per discovered tool) can
//! share it. Tool calls are serialized on the mutex (the MCP client supports one
//! outstanding request at a time).

use crate::config::McpServerConfig;
use crate::mcp::client::McpClient;
use crate::mcp::error::McpError;
use crate::mcp::jsonrpc::method;
use crate::mcp::transport::StdioTransport;
use crate::mcp::McpToolDescriptor;
use serde_json::Value;
use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// A live, initialized connection to one MCP server.
pub struct McpServer {
    /// Configured server name (namespace prefix for its tools).
    pub name: String,
    /// Whether this server's tools require the Spec 002 confirmation gate.
    pub confirm: bool,
    child: Child,
    /// The JSON-RPC client, shared by all this server's tool wrappers.
    client: Arc<Mutex<McpClient<StdioTransport>>>,
}

impl McpServer {
    /// Spawns `cfg.command` (with `args`/`env`), performs the `initialize`
    /// handshake, and returns a live server. On any failure the child is
    /// killed and the error returned (the caller keeps other servers alive).
    pub async fn spawn(name: &str, cfg: McpServerConfig) -> Result<Self, McpError> {
        let mut cmd = Command::new(&cfg.command);
        cmd.args(&cfg.args)
            .envs(&cfg.env)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        let mut child = cmd
            .spawn()
            .map_err(|e| McpError::Transport(format!("spawn '{}': {e}", cfg.command)))?;

        // Drain stderr to a tracing task so the child never blocks on a full
        // stderr pipe; server logs never carry our secrets.
        if let Some(stderr) = child.stderr.take() {
            let tag = name.to_owned();
            tokio::spawn(async move {
                use tokio::io::AsyncBufReadExt;
                let mut lines = tokio::io::BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::debug!("mcp[{tag}] stderr: {line}");
                }
            });
        }

        let transport = StdioTransport::take(&mut child)?;
        let mut client = McpClient::new(transport);
        if let Err(e) = client.initialize().await {
            // Best effort: make sure the child is gone before returning.
            let _ = child.kill().await;
            return Err(e);
        }
        Ok(Self {
            name: name.to_owned(),
            confirm: cfg.confirm,
            child,
            client: Arc::new(Mutex::new(client)),
        })
    }

    /// The shared JSON-RPC client.
    pub fn client(&self) -> &Arc<Mutex<McpClient<StdioTransport>>> {
        &self.client
    }

    /// Runs `tools/list` and parses the discovered tool descriptors.
    pub async fn list_tools(&self) -> Result<Vec<McpToolDescriptor>, McpError> {
        let mut client = self.client.lock().await;
        let result = client.request(method::TOOLS_LIST, None).await?;
        Self::parse_tool_list(&self.name, &result)
    }

    fn parse_tool_list(server: &str, result: &Value) -> Result<Vec<McpToolDescriptor>, McpError> {
        let tools = result
            .get("tools")
            .and_then(Value::as_array)
            .ok_or_else(|| McpError::Protocol("tools/list result has no 'tools' array".into()))?;
        let mut out = Vec::new();
        for t in tools {
            let name = t
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| McpError::Protocol("tool entry missing 'name'".into()))?;
            let description = t
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            out.push(McpToolDescriptor {
                server: server.to_owned(),
                name: name.to_owned(),
                description,
            });
        }
        Ok(out)
    }

    /// Closes the child process (graceful best-effort kill + reap). Further
    /// tool calls on this server will fail because the child is gone.
    pub async fn shutdown(&mut self) {
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
    }
}

/// Convenience for tests/registry that need a no-op server pool.
pub async fn spawn_configured(
    servers: Vec<(String, McpServerConfig)>,
    cancel: CancellationToken,
) -> (Vec<McpServer>, Vec<(String, McpError)>) {
    let mut ok = Vec::new();
    let mut failed = Vec::new();
    for (name, cfg) in servers {
        if cancel.is_cancelled() {
            failed.push((name, McpError::Transport("cancelled during startup".into())));
            continue;
        }
        match McpServer::spawn(&name, cfg).await {
            Ok(s) => ok.push(s),
            Err(e) => failed.push((name, e)),
        }
    }
    (ok, failed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_tool_list() {
        let result = json!({
            "tools": [
                {"name":"echo","description":"echo back"},
                {"name":"no_desc"},
                {"name":"other","description":"","inputSchema":{"type":"object"}}
            ]
        });
        let descs = McpServer::parse_tool_list("gh", &result).unwrap();
        assert_eq!(descs.len(), 3);
        assert_eq!(descs[0].server, "gh");
        assert_eq!(descs[0].name, "echo");
        assert_eq!(descs[0].description, "echo back");
        assert_eq!(descs[1].description, "");
        assert_eq!(descs[2].hermes_name(), "gh__other");
    }

    #[test]
    fn missing_tools_array_is_protocol_error() {
        let result = json!({ "capabilities": {} });
        assert!(matches!(McpServer::parse_tool_list("s", &result), Err(McpError::Protocol(_))));
    }

    #[test]
    fn tool_without_name_is_protocol_error() {
        let result = json!({ "tools": [ { "description": "nameless" } ] });
        assert!(matches!(McpServer::parse_tool_list("s", &result), Err(McpError::Protocol(_))));
    }
}
