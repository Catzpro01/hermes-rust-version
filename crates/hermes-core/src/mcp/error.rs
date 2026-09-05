//! Spec 011 (MCP) — error taxonomy for the MCP client.

use thiserror::Error;

/// Errors surfaced by the MCP client layer (transport, protocol, remote).
#[derive(Debug, Error)]
pub enum McpError {
    /// The underlying stdio transport failed (I/O, write, read).
    #[error("mcp transport error: {0}")]
    Transport(String),
    /// The child's stdout closed before a response was received.
    #[error("mcp server closed the stream before responding")]
    Closed,
    /// A message did not conform to JSON-RPC 2.0 / the expected protocol.
    #[error("mcp protocol error: {0}")]
    Protocol(String),
    /// The peer replied with a JSON-RPC error object.
    #[error("mcp server error {code}: {message}")]
    Remote { code: i64, message: String },
    /// A response arrived for a request id we did not send / already resolved.
    #[error("mcp unexpected response for id {id}")]
    UnexpectedResponse { id: String },
    /// The peer returned a response id different from the outstanding request.
    #[error("mcp response id mismatch: expected {expected}, got {got}")]
    IdMismatch { expected: String, got: String },
}

impl McpError {
    /// Helper to build [`McpError::Remote`] from a JSON-RPC error.
    pub fn remote(code: i64, message: impl Into<String>) -> Self {
        Self::Remote { code, message: message.into() }
    }
}
