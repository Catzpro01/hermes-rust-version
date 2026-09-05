//! Spec 011 (MCP) closure (Ticket 05) — live end-to-end proof against a real
//! child-process MCP server (`mcp_test_server`), spawned via the stdio client:
//! handshake -> tools/list -> tools registered -> executed through Hermes tools
//! (and the agentic loop) -> confirmed/denied gating -> graceful shutdown.

use async_trait::async_trait;
use futures::stream;
use hermes_core::{
    config::McpServerConfig,
    conversation::{AgenticResult, ConversationRunner, Event, Turn},
    mcp::{McpServer, McpTool},
    provider::tool_aware_stream,
    provider::{EventStream, Provider, ProviderError},
    tools::{Confirmation, ToolCall, ToolError, ToolRegistry},
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

const BIN: &str = env!("CARGO_BIN_EXE_mcp_test_server");

/// Auto-approve or auto-deny confirmation source for tests.
#[derive(Clone)]
struct Auto(bool);
#[async_trait]
impl Confirmation for Auto {
    async fn confirm(&self, _: &str) -> bool {
        self.0
    }
}

fn server_cfg(confirm: bool) -> McpServerConfig {
    McpServerConfig {
        command: BIN.to_owned(),
        args: Vec::new(),
        env: HashMap::new(),
        confirm,
    }
}

/// Registers a live server's tools into a registry.
async fn register_server(reg: &mut ToolRegistry, confirm: bool) -> McpServer {
    let server = McpServer::spawn("demo", server_cfg(confirm)).await.unwrap();
    let descs = server.list_tools().await.unwrap();
    assert_eq!(descs.len(), 2, "demo server should expose echo + fail");
    for desc in &descs {
        reg.register(McpTool::new(&server, desc, confirm, Auto(true)));
    }
    server
}

#[tokio::test]
async fn e2e_spawn_discover_and_execute_echo() {
    let mut reg = ToolRegistry::new();
    let server = register_server(&mut reg, false).await;
    // echo succeeds and echoes the argument object.
    let call = ToolCall {
        id: Some("1".into()),
        name: "demo__echo".into(),
        arguments: r#"{"msg":"hi"}"#.into(),
    };
    let resp = reg.execute(&call, CancellationToken::new()).await.unwrap();
    assert!(resp.content.contains("hi"), "got: {}", resp.content);
    assert!(resp.success);
    server.shutdown().await;
}

#[tokio::test]
async fn e2e_failing_tool_maps_to_error() {
    let mut reg = ToolRegistry::new();
    let server = register_server(&mut reg, false).await;
    let call = ToolCall {
        id: Some("2".into()),
        name: "demo__fail".into(),
        arguments: "{}".into(),
    };
    let err = reg.execute(&call, CancellationToken::new()).await.unwrap_err();
    assert!(matches!(err, ToolError::Failed(_)), "got {err:?}");
    server.shutdown().await;
}

#[tokio::test]
async fn confirm_true_denies_when_user_declines() {
    // A server with confirm:true + an auto-NO confirmation -> Denied (never runs).
    let server = McpServer::spawn("denydemo", server_cfg(true)).await.unwrap();
    let descs = server.list_tools().await.unwrap();
    let echo = descs.iter().find(|d| d.name == "echo").unwrap();
    // Register with a deny-always confirmation.
    let mut reg = ToolRegistry::new();
    // McpTool::new takes confirm + confirmation; override confirm source to deny.
    let tool = McpTool::new(&server, echo, true, Auto(false));
    reg.register(tool);
    let call = ToolCall {
        id: Some("3".into()),
        name: "denydemo__echo".into(),
        arguments: "{}".into(),
    };
    let err = reg.execute(&call, CancellationToken::new()).await.unwrap_err();
    assert!(matches!(err, ToolError::Denied(_)), "got {err:?}");
    server.shutdown().await;
}

#[tokio::test]
async fn e2e_agentic_loop_calls_mcp_tool_transparently() {
    // A scripted provider first issues an MCP tool call, then a final answer.
    struct Scripted(Arc<Mutex<Vec<String>>>);
    #[async_trait]
    impl Provider for Scripted {
        async fn chat(&self, _: &[Turn]) -> Result<EventStream, ProviderError> {
            let text = self.0.lock().unwrap().remove(0);
            Ok(tool_aware_stream(Box::pin(stream::iter(vec![
                Ok(Event::Started),
                Ok(Event::Chunk(text)),
                Ok(Event::Done),
            ]))))
        }
    }
    let mut reg = ToolRegistry::new();
    let server = register_server(&mut reg, false).await;
    let p = Scripted(Arc::new(Mutex::new(vec![
        "<tool_call id=\"a\">demo__echo: {\"msg\":\"via agent\"}</tool_call>".into(),
        "final answer".into(),
    ])));
    let mut r = ConversationRunner::new(p);
    let out = r
        .chat_agentic("go", &reg, None, 10, CancellationToken::new())
        .await
        .unwrap();
    assert!(matches!(out, AgenticResult::Done { .. }));
    // The echoed tool content must have been recorded as a Tool turn.
    let tool_turn = r
        .turns()
        .iter()
        .find_map(|t| match t {
            Turn::Tool { name, content } if name == "demo__echo" => Some(content.clone()),
            _ => None,
        })
        .expect("MCP tool call must produce a Tool turn");
    assert!(tool_turn.contains("via agent"), "got: {tool_turn}");
    server.shutdown().await;
}

#[tokio::test]
async fn e2e_default_has_no_mcp_spawn() {
    // No mcp_servers configured -> nothing is created; the normal tool registry
    // has no MCP-namespaced tools. (Simulates the zero-config REPL path.)
    let reg = ToolRegistry::new();
    let call = ToolCall {
        id: Some("x".into()),
        name: "demo__echo".into(),
        arguments: "{}".into(),
    };
    let err = reg.execute(&call, CancellationToken::new()).await.unwrap_err();
    assert!(matches!(err, ToolError::Unknown(_)), "got {err:?}");
    // Sanity: registry still has nothing MCP.
    assert!(reg.get("demo__echo").is_none());
}
