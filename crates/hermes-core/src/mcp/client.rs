//! Spec 011 (MCP) — JSON-RPC 2.0 client over the stdio transport.
//!
//! [`McpClient`] owns a [`McpTransport`] and offers request/response plus
//! notification primitives, including the MCP `initialize` /
//! `notifications/initialized` handshake. Requests use incrementing integer
//! ids and, for simplicity and determinism, only **one request is outstanding at
//! a time** (the caller serializes via `&mut self`). Unsolicited peer
//! notifications are read and ignored while awaiting a response.

use super::error::McpError;
use super::jsonrpc::{self, method, Inbound, Reply};
use super::transport::McpTransport;
use serde_json::{json, Value};

/// Client identifier reported to the server during `initialize`.
pub const CLIENT_NAME: &str = "hermes-rs";
pub const CLIENT_VERSION: &str = "0.1.0";

/// JSON-RPC 2.0 client over an [`McpTransport`]. Not `Clone`; one client owns
/// one transport/process.
pub struct McpClient<T: McpTransport> {
    transport: T,
    next_id: u64,
    initialized: bool,
}
impl<T: McpTransport> McpClient<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            next_id: 1,
            initialized: false,
        }
    }

    /// True once [`initialize`](Self::initialize) succeeded.
    pub fn initialized(&self) -> bool {
        self.initialized
    }

    /// MCP handshake: `initialize` request → server `result`, then
    /// `notifications/initialized`. Returns the server's `initialize` result
    /// (protocolVersion/capabilities/serverInfo). Subsequent protocol calls
    /// assume the handshake happened.
    pub async fn initialize(&mut self) -> Result<Value, McpError> {
        let params = json!({
            "protocolVersion": jsonrpc::MCP_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": { "name": CLIENT_NAME, "version": CLIENT_VERSION },
        });
        let result = self.request(method::INITIALIZE, Some(params)).await?;
        self.notify(method::INITIALIZED, None).await?;
        self.initialized = true;
        Ok(result)
    }

    /// Sends a request and awaits its response `result`.
    pub async fn request(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, McpError> {
        if !self.initialized && method != method::INITIALIZE {
            return Err(McpError::Protocol(
                "cannot send request before initialize handshake".into(),
            ));
        }
        let id = self.next_id;
        self.next_id += 1;
        let req = jsonrpc::request(id, method, params);
        let line = serde_json::to_string(&req)
            .map_err(|e| McpError::Protocol(format!("serialize request: {e}")))?;
        self.transport
            .write_line(line.as_bytes())
            .await?;
        self.transport.flush().await?;
        self.await_response(id).await
    }

    /// Sends a fire-and-forget notification (no id, no response expected).
    pub async fn notify(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<(), McpError> {
        let note = jsonrpc::notification(method, params);
        let line = serde_json::to_string(&note)
            .map_err(|e| McpError::Protocol(format!("serialize notification: {e}")))?;
        self.transport.write_line(line.as_bytes()).await?;
        self.transport.flush().await?;
        Ok(())
    }

    /// Reads inbound lines until the response matching `expected_id` arrives.
    /// Peer notifications (server → client) are read and ignored while waiting.
    async fn await_response(&mut self, expected_id: u64) -> Result<Value, McpError> {
        loop {
            let line = match self.transport.read_line().await? {
                Some(l) => l,
                None => return Err(McpError::Closed),
            };
            match jsonrpc::parse_inbound(&line) {
                None => {
                    return Err(McpError::Protocol(format!("malformed inbound line: {line}")));
                }
                Some(Inbound::PeerMessage { .. }) => {
                    // Server → client notification/request; ignore for now.
                    continue;
                }
                Some(Inbound::Response { id, reply }) => {
                    let got = serde_json::to_string(&id).unwrap_or_default();
                    if !matches_jsonrpc_id(&id, expected_id) {
                        return Err(McpError::IdMismatch {
                            expected: expected_id.to_string(),
                            got,
                        });
                    }
                    return match reply {
                        Reply::Result(v) => Ok(v),
                        Reply::Error(e) => Err(McpError::Remote { code: e.code, message: e.message }),
                    };
                }
            }
        }
    }
}

/// Compares an inbound response id (`Value`) to an expected numeric id.
fn matches_jsonrpc_id(id: &Value, expected: u64) -> bool {
    match id {
        Value::Number(n) => n.as_u64() == Some(expected),
        Value::String(s) => s.parse::<u64>().map(|v| v == expected).unwrap_or(false),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::jsonrpc;
    use crate::mcp::McpTransport;
    use std::collections::VecDeque;

    /// A scripted transport: inbound lines are queued up front; outbound writes
    /// are captured so a test can assert exactly what the client sent.
    struct FakeTransport {
        inbound: VecDeque<String>,
        outbound: Vec<String>,
    }
    impl FakeTransport {
        fn new(inbound: Vec<String>) -> Self {
            Self {
                inbound: inbound.into(),
                outbound: Vec::new(),
            }
        }
    }
    #[async_trait::async_trait]
    impl McpTransport for FakeTransport {
        async fn write_line(&mut self, buf: &[u8]) -> Result<(), McpError> {
            self.outbound.push(String::from_utf8_lossy(buf).into_owned());
            Ok(())
        }
        async fn flush(&mut self) -> Result<(), McpError> {
            Ok(())
        }
        async fn read_line(&mut self) -> Result<Option<String>, McpError> {
            Ok(self.inbound.pop_front())
        }
    }

    fn server_init_result() -> String {
        serde_json::to_string(&json!({
            "jsonrpc":"2.0","id":1,
            "result": {
                "protocolVersion":"2024-11-05",
                "capabilities":{},
                "serverInfo":{"name":"fake","version":"1"}
            }
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn initialize_sends_initialize_and_initialized() {
        // One scripted reply to the initialize request (id 1).
        let t = FakeTransport::new(vec![server_init_result()]);
        let mut c = McpClient::new(t);
        let result = c.initialize().await.unwrap();
        assert_eq!(result["serverInfo"]["name"], "fake");
        assert!(c.initialized());
        assert_eq!(c.transport.outbound.len(), 2, "initialize request + initialized notification");
        let req: Value = serde_json::from_str(&c.transport.outbound[0]).unwrap();
        assert_eq!(req["method"], "initialize");
        assert_eq!(req["id"], 1);
        assert_eq!(req["params"]["protocolVersion"], jsonrpc::MCP_PROTOCOL_VERSION);
        assert_eq!(req["params"]["clientInfo"]["name"], CLIENT_NAME);
        let note: Value = serde_json::from_str(&c.transport.outbound[1]).unwrap();
        assert_eq!(note["method"], "notifications/initialized");
        assert!(note.get("id").is_none(), "notification must have no id");
    }

    #[tokio::test]
    async fn request_maps_server_error_to_remote() {
        let reply = serde_json::to_string(&json!({
            "jsonrpc":"2.0","id":1,
            "error":{"code":-32602,"message":"bad params"}
        }))
        .unwrap();
        let mut c = McpClient::new(FakeTransport::new(vec![reply]));
        // Force initialized so a request method is allowed.
        c.initialized = true;
        let err = c.request("x", None).await.unwrap_err();
        match err {
            McpError::Remote { code, message } => {
                assert_eq!(code, -32602);
                assert_eq!(message, "bad params");
            }
            other => panic!("expected Remote, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ignores_peer_notification_while_awaiting_response() {
        let notif = serde_json::to_string(&json!({
            "jsonrpc":"2.0","method":"notifications/message",
            "params":{"level":"info","data":"hi"}
        }))
        .unwrap();
        let resp = serde_json::to_string(&json!({
            "jsonrpc":"2.0","id":1,"result":{"ok":true}
        }))
        .unwrap();
        let mut c = McpClient::new(FakeTransport::new(vec![notif, resp]));
        c.initialized = true;
        let v = c.request("tools/list", None).await.unwrap();
        assert_eq!(v["ok"], true);
    }

    #[tokio::test]
    async fn closed_stream_yields_closed_error() {
        // No inbound lines -> read_line returns None -> Closed.
        let mut c = McpClient::new(FakeTransport::new(vec![]));
        c.initialized = true;
        assert!(matches!(c.request("x", None).await, Err(McpError::Closed)));
    }

    #[tokio::test]
    async fn mismatched_response_id_is_an_error() {
        let resp = serde_json::to_string(&json!({
            "jsonrpc":"2.0","id":99,"result":{"ok":true}
        }))
        .unwrap();
        let mut c = McpClient::new(FakeTransport::new(vec![resp]));
        c.initialized = true;
        assert!(matches!(c.request("x", None).await, Err(McpError::IdMismatch { .. })));
    }
}
