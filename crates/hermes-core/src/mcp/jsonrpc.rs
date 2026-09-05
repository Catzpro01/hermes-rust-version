//! Spec 011 (MCP) — JSON-RPC 2.0 value model for the stdio transport.
//!
//! MCP speaks JSON-RPC 2.0 with **newline-delimited JSON (NDJSON)** framing: one
//! JSON-RPC message per line on the child's stdout, and we write one message
//! per line to its stdin. There is no `Content-Length` header (that is the LSP
//! framing; MCP stdio transport uses NDJSON). This module only defines the wire
//! shapes and tiny encode/decode helpers — no I/O.

use serde_json::Value;

/// JSON-RPC protocol marker string.
pub const JSONRPC: &str = "2.0";
/// MCP protocol version we advertise during `initialize` (pinned; the server
/// decides which version it actually runs).
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// Builds a JSON-RPC 2.0 `Request` object (`id`, `method`, `params`).
pub fn request(id: u64, method: &str, params: Option<Value>) -> Value {
    let mut o = serde_json::Map::new();
    o.insert("jsonrpc".into(), Value::String(JSONRPC.into()));
    o.insert("id".into(), Value::Number(id.into()));
    o.insert("method".into(), Value::String(method.into()));
    if let Some(p) = params {
        o.insert("params".into(), p);
    }
    Value::Object(o)
}

/// Builds a JSON-RPC 2.0 `Notification` object (`method`, `params`, **no id**).
pub fn notification(method: &str, params: Option<Value>) -> Value {
    let mut o = serde_json::Map::new();
    o.insert("jsonrpc".into(), Value::String(JSONRPC.into()));
    o.insert("method".into(), Value::String(method.into()));
    if let Some(p) = params {
        o.insert("params".into(), p);
    }
    Value::Object(o)
}

/// A JSON-RPC `error` object returned by the peer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

/// The reply a client cares about when awaiting one outstanding request.
#[derive(Debug, Clone)]
pub enum Reply {
    /// Successful `result`.
    Result(Value),
    /// JSON-RPC `error`.
    Error(RpcError),
}

/// Classifies one inbound line into what a request/response client can act on.
///
/// - A **Response** carries `id` and no `method`; the client matches it to the
///   outstanding request id.
/// - Anything carrying `method` is a server → client `Notification` (no `id`)
///   or `Request` (has `id`). Basic MCP clients do not need to answer server
///   requests, so both are surfaced as `PeerMessage` for the caller to ignore or
///   log.
#[derive(Debug, Clone)]
pub enum Inbound {
    Response { id: Value, reply: Reply },
    PeerMessage { method: String, id: Option<Value> },
}

/// Parses a raw NDJSON line into an [`Inbound`]. `None` on malformed input
/// (not valid JSON-RPC 2.0) — the caller decides whether to skip or error.
pub fn parse_inbound(line: &str) -> Option<Inbound> {
    let v: Value = serde_json::from_str(line).ok()?;
    let obj = v.as_object()?;
    if obj.get("jsonrpc")?.as_str()? != JSONRPC {
        return None;
    }
    let has_method = obj.get("method").and_then(Value::as_str);
    let id = obj.get("id").cloned();
    match has_method {
        Some(method) => Some(Inbound::PeerMessage { method: method.to_owned(), id }),
        None => {
            let id = id?;
            if let Some(err) = obj.get("error") {
                let code = err.get("code").and_then(Value::as_i64).unwrap_or(0);
                let message = err
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("json-rpc error")
                    .to_owned();
                let data = err.get("data").cloned();
                Some(Inbound::Response {
                    id,
                    reply: Reply::Error(RpcError { code, message, data }),
                })
            } else {
                let result = obj.get("result").cloned().unwrap_or(Value::Null);
                Some(Inbound::Response { id, reply: Reply::Result(result) })
            }
        }
    }
}

/// Standard MCP methods.
pub mod method {
    pub const INITIALIZE: &str = "initialize";
    pub const INITIALIZED: &str = "notifications/initialized";
    pub const TOOLS_LIST: &str = "tools/list";
    pub const TOOLS_CALL: &str = "tools/call";
    pub const SHUTDOWN: &str = "shutdown";
}
