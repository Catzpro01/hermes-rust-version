//! Spec 011 — Model Context Protocol (MCP) client.
//!
//! Hermes-RS connects to MCP servers as a client: it spawns a child process
//! (`McpServerConfig` from Ticket 01), speaks JSON-RPC 2.0 over newline-delimited
//! stdio, discovers tools via `tools/list`, and wraps each as a Hermes `Tool`.
//! See `.scratch/hermes-rs-mcp-client/issues/` for the ticket breakdown.

pub mod client;
pub mod error;
pub mod jsonrpc;
pub mod transport;

pub use client::{McpClient, CLIENT_NAME, CLIENT_VERSION};
pub use error::McpError;
pub use transport::{McpTransport, StdioTransport};
