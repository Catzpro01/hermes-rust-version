use super::{Tool, ToolCall, ToolError, ToolResponse};
use async_trait::async_trait;
use std::time::Duration;
use tokio::{process::Command, time::timeout};
use tokio_util::sync::CancellationToken;
#[async_trait]
pub trait Confirmation: Send + Sync {
    async fn confirm(&self, command: &str) -> bool;
}
pub struct ShellTool<C> {
    confirmation: C,
    timeout: Duration,
}
impl<C> ShellTool<C> {
    pub fn new(confirmation: C, timeout: Duration) -> Self {
        Self {
            confirmation,
            timeout,
        }
    }
}
#[async_trait]
impl<C: Confirmation> Tool for ShellTool<C> {
    fn name(&self) -> &str {
        "shell"
    }
    fn description(&self) -> &str {
        "Execute a shell command after explicit confirmation."
    }
    async fn execute(
        &self,
        call: &ToolCall,
        cancel: CancellationToken,
    ) -> Result<ToolResponse, ToolError> {
        if !self.confirmation.confirm(&call.arguments).await {
            return Err(ToolError::Denied("confirmation declined".into()));
        }
        let child = Command::new("sh").arg("-c").arg(&call.arguments).output();
        let output = tokio::select! { _=cancel.cancelled()=>return Err(ToolError::Cancelled), result=timeout(self.timeout,child)=>result.map_err(|_|ToolError::Timeout(self.timeout))?.map_err(|e|ToolError::Failed(e.to_string()))? };
        let content =
            String::from_utf8_lossy(&[output.stdout, output.stderr].concat()).into_owned();
        Ok(ToolResponse {
            id: call.id.clone(),
            name: call.name.clone(),
            success: output.status.success(),
            content,
        })
    }
}
