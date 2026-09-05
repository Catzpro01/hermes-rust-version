//! Spec 011 (MCP) — a tiny controllable MCP server for integration tests.
//!
//! Speaks newline-delimited JSON-RPC over stdio like a real MCP server, but is
//! fully offline/deterministic: it answers `initialize`, `tools/list`
//! (two tools: `echo` and `fail`), and `tools/call`. It is built as a normal
//! `cargo` bin so integration tests can spawn it via
//! `env!("CARGO_BIN_EXE_mcp_test_server")` and prove the real child-process
//! path without network access or `npx`.

use serde_json::{json, Value};
use std::io::{BufRead, Write};

fn reply_for(method: &str, params: Option<&Value>) -> Value {
    match method {
        "initialize" => json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "serverInfo": { "name": "hermes-test-server", "version": "1.0" }
        }),
        "tools/list" => json!({
            "tools": [
                { "name": "echo", "description": "echo the arguments as text" },
                { "name": "fail", "description": "always reports an error" }
            ]
        }),
        "tools/call" => {
            let name = params
                .and_then(|p| p.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let args = params.and_then(|p| p.get("arguments")).cloned().unwrap_or(Value::Null);
            if name == "fail" {
                json!({
                    "content": [{"type": "text", "text": "intentional failure"}],
                    "isError": true
                })
            } else {
                json!({
                    "content": [{"type": "text", "text": format!("echo: {}", args)}]
                })
            }
        }
        _ => json!({}),
    }
}

fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let v: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue, // skip malformed
        };
        let Some(obj) = v.as_object() else { continue };
        // Notifications have a method but no id -> no response expected.
        let Some(method) = obj.get("method").and_then(Value::as_str) else {
            continue;
        };
        let Some(id) = obj.get("id") else {
            continue;
        };
        let params = obj.get("params");
        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": reply_for(method, params)
        });
        if let Ok(s) = serde_json::to_string(&body) {
            let _ = writeln!(out, "{s}");
            let _ = out.flush();
        }
    }
}
