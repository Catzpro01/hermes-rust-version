//! Spec 011 (MCP) — wrapping a discovered MCP tool as a Hermes [`Tool`].

use crate::mcp::client::McpClient;
use crate::mcp::error::McpError;
use crate::mcp::jsonrpc::method;
use crate::mcp::server::McpServer;
use crate::mcp::transport::StdioTransport;
use crate::tools::{Confirmation, Tool, ToolCall, ToolError, ToolResponse};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// A tool exposed by an MCP server via `tools/list`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpToolDescriptor {
    /// Server name this tool belongs to (namespace prefix).
    pub server: String,
    /// The tool's name as reported by the server.
    pub name: String,
    /// The tool's description as reported by the server.
    pub description: String,
}

impl McpToolDescriptor {
    /// The registry-unique Hermes name: `"{server}__{name}"`.
    pub fn hermes_name(&self) -> String {
        format!("{}__{}", self.server, self.name)
    }
}

/// Default per-call timeout for MCP tool execution (a hanging server must not
/// wedge the agent forever).
pub const MCP_TOOL_TIMEOUT: Duration = Duration::from_secs(60);

/// A Hermes `Tool` that forwards execution to an MCP server over its shared
/// client. Generic over the confirmation source so the CLI can pass its real
/// [`Confirmation`] and tests can pass an auto-yes/no.
pub struct McpTool<C> {
    client: Arc<Mutex<McpClient<StdioTransport>>>,
    server: String,
    /// The registry name (already namespaced `"{server}__{name}"`).
    hermes_name: String,
    /// The tool name sent in `tools/call`.
    mcp_name: String,
    description: String,
    confirm: bool,
    confirmation: C,
    timeout: Duration,
}
impl<C: Confirmation> McpTool<C> {
    pub fn new(
        server: &McpServer,
        desc: &McpToolDescriptor,
        confirm: bool,
        confirmation: C,
    ) -> Self {
        Self {
            client: Arc::clone(server.client()),
            server: server.name.clone(),
            hermes_name: desc.hermes_name(),
            mcp_name: desc.name.clone(),
            description: desc.description.clone(),
            confirm,
            confirmation,
            timeout: MCP_TOOL_TIMEOUT,
        }
    }

    /// Executes one `tools/call`, honoring confirm (→ `Denied`) and a per-call
    /// timeout (→ `ToolError::Timeout`).
    async fn call(
        &self,
        mcp_name: &str,
        arguments: Value,
        cancel: CancellationToken,
    ) -> Result<ToolResponse, ToolError> {
        let fut = async {
            if cancel.is_cancelled() {
                return Err(ToolError::Cancelled);
            }
            let mut client = self.client.lock().await;
            let params = json!({ "name": mcp_name, "arguments": arguments });
            client
                .request(method::TOOLS_CALL, Some(params))
                .await
                .map_err(map_mcp_err)
        };
        let result = match tokio::time::timeout(self.timeout, fut).await {
            Err(_) => return Err(ToolError::Timeout(self.timeout)),
            Ok(r) => r?,
        };
        parse_call_result(mcp_name, result)
    }
}

/// Maps an MCP-layer error onto the Hermes tool error taxonomy. A `Denied`
/// never originates here (denial is the confirmation gate's decision); anything
/// from the server is a retryable `Failed` (Spec 009 recovery treats
/// `Error`/`Timeout` as retryable) except transport/protocol faults, which
/// surface as `Failed` too so the agent sees a recoverable tool failure.
fn map_mcp_err(e: McpError) -> ToolError {
    match e {
        McpError::Remote { message, .. } => ToolError::Failed(message),
        other => ToolError::Failed(other.to_string()),
    }
}

/// Turns a `tools/call` result into a Hermes [`ToolResponse`].
///
/// - `result.content` (MCP content items) is flattened: text items are joined;
///   if there is no text, `structuredContent` is serialized as JSON; otherwise
///   a human-readable summary of content types is produced.
/// - `result.isError == true` → [`ToolError::Failed`] (an execution failure).
fn parse_call_result(name: &str, result: Value) -> Result<ToolResponse, ToolError> {
    if result.get("isError").and_then(Value::as_bool).unwrap_or(false) {
        let msg = flatten_text(&result);
        return Err(ToolError::Failed(if msg.is_empty() {
            format!("MCP tool '{name}' reported an error")
        } else {
            msg
        }));
    }
    let content = flatten_text(&result);
    Ok(ToolResponse {
        id: Some(name.to_owned()),
        name: name.to_owned(),
        content,
        success: true,
    })
}

fn flatten_text(result: &Value) -> String {
    if let Some(items) = result.get("content").and_then(Value::as_array) {
        let mut parts = Vec::new();
        for item in items {
            if let Some(t) = item.get("text").and_then(Value::as_str) {
                parts.push(t.to_owned());
            } else if let Some(s) = item.get("type").and_then(Value::as_str) {
                // e.g. image/resource — represent as a short note.
                parts.push(format!("[{s} content omitted]"));
            }
        }
        if !parts.is_empty() {
            return parts.join("\n");
        }
    }
    if let Some(sc) = result.get("structuredContent") {
        return serde_json::to_string(sc).unwrap_or_default();
    }
    String::new()
}

#[async_trait]
impl<C: Confirmation + Clone + Send + Sync> Tool for McpTool<C> {
    fn name(&self) -> &str {
        &self.hermes_name
    }
    fn description(&self) -> &str {
        &self.description
    }
    async fn execute(
        &self,
        call: &ToolCall,
        cancel: CancellationToken,
    ) -> Result<ToolResponse, ToolError> {
        if self.confirm {
            let prompt = format!(
                "Run MCP tool '{}:{}' with arguments: {}?",
                self.server, self.mcp_name, call.arguments
            );
            let ok = self.confirmation.confirm(&prompt).await;
            if !ok {
                return Err(ToolError::Denied(format!(
                    "MCP tool '{}:{}' declined by user",
                    self.server, self.mcp_name
                )));
            }
        }
        let args = parse_arguments(&call.arguments)?;
        self.call(&self.mcp_name, args, cancel).await
    }
}

/// Parses the model-supplied arguments string as a JSON object for `tools/call`.
/// An empty string is treated as `{}`; a non-object JSON is an error (MCP
/// requires an object) — the call is not executed.
fn parse_arguments(arguments: &str) -> Result<Value, ToolError> {
    let trimmed = arguments.trim();
    if trimmed.is_empty() {
        return Ok(json!({}));
    }
    let v: Value = serde_json::from_str(trimmed).map_err(|e| {
        ToolError::Failed(format!("MCP tool arguments are not valid JSON: {e}"))
    })?;
    match v {
        Value::Object(_) => Ok(v),
        _ => Err(ToolError::Failed(
            "MCP tool arguments must be a JSON object".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hermes_name_namespaces_by_server() {
        let d = McpToolDescriptor {
            server: "github".into(),
            name: "create_issue".into(),
            description: "x".into(),
        };
        assert_eq!(d.hermes_name(), "github__create_issue");
    }

    #[test]
    fn parse_arguments_accepts_empty_and_object() {
        assert_eq!(parse_arguments("").unwrap(), json!({}));
        assert_eq!(parse_arguments("  ").unwrap(), json!({}));
        assert_eq!(parse_arguments(r#"{"path":"/a"}"#).unwrap(), json!({"path":"/a"}));
    }

    #[test]
    fn parse_arguments_rejects_non_object() {
        assert!(matches!(
            parse_arguments(r#"[1,2]"#),
            Err(ToolError::Failed(_))
        ));
        assert!(matches!(
            parse_arguments("not json"),
            Err(ToolError::Failed(_))
        ));
    }

    #[test]
    fn call_result_flattens_text_content() {
        let result = json!({
            "content": [{"type":"text","text":"hello"},{"type":"text","text":"world"}]
        });
        let r = parse_call_result("t", result).unwrap();
        assert_eq!(r.content, "hello\nworld");
        assert!(r.success);
    }

    #[test]
    fn call_result_is_error_maps_to_failed() {
        let result = json!({
            "isError": true,
            "content": [{"type":"text","text":"boom"}]
        });
        assert!(matches!(parse_call_result("t", result), Err(ToolError::Failed(_))));
    }

    #[test]
    fn call_result_structured_content_when_no_text() {
        let result = json!({ "structuredContent": { "rows": 3 } });
        let r = parse_call_result("t", result).unwrap();
        assert!(r.content.contains("rows"), "got: {}", r.content);
    }
}
