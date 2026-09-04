//! Spec 002 tool calling primitives. Execution is explicit and policy-gated.
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};
use thiserror::Error;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCall {
    pub id: Option<String>,
    pub name: String,
    pub arguments: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolResponse {
    pub id: Option<String>,
    pub name: String,
    pub content: String,
    pub success: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolExecutionStatus {
    Success,
    Error,
    Denied,
    Timeout,
    Cancelled,
}
impl ToolExecutionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Error => "error",
            Self::Denied => "denied",
            Self::Timeout => "timeout",
            Self::Cancelled => "cancelled",
        }
    }
}
#[derive(Debug, Clone)]
pub struct ToolCallRecord {
    pub id: String,
    pub session_id: String,
    pub turn_index: usize,
    pub tool_name: String,
    pub arguments: String,
    pub result: String,
    pub status: ToolExecutionStatus,
}
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("unknown tool: {0}")]
    Unknown(String),
    #[error("tool execution cancelled")]
    Cancelled,
    #[error("tool timed out after {0:?}")]
    Timeout(Duration),
    #[error("tool denied: {0}")]
    Denied(String),
    #[error("tool failed: {0}")]
    Failed(String),
    #[error("invalid tool XML: {0}")]
    InvalidXml(String),
}
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(
        &self,
        call: &ToolCall,
        cancel: CancellationToken,
    ) -> Result<ToolResponse, ToolError>;
}
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}
impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }
    pub fn register(&mut self, tool: impl Tool + 'static) {
        self.tools.insert(tool.name().to_owned(), Box::new(tool));
    }
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }
    pub async fn execute(
        &self,
        call: &ToolCall,
        cancel: CancellationToken,
    ) -> Result<ToolResponse, ToolError> {
        self.get(&call.name)
            .ok_or_else(|| ToolError::Unknown(call.name.clone()))?
            .execute(call, cancel)
            .await
    }
}
impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub mod parser;
pub mod readonly;
pub mod shell;
pub mod write;
pub use parser::parse_tool_events;
pub use readonly::{ListDirTool, ReadFileTool};
pub use shell::{validate_readonly_command, Confirmation, ShellReadonlyTool, ShellTool};
pub use write::WriteFileTool;
