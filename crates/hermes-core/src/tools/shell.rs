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

pub const BLOCKED_PATTERNS: &[&str] = &[
    "rm ", "rm\t", "sudo ", "chmod ", "chown ", "curl ", "wget ", "dd ", "mkfs", " >", ">>", " |",
    "| ",
];
pub struct ShellReadonlyTool<C> {
    confirmation: C,
    timeout: Duration,
    unsafe_mode: bool,
}
impl<C> ShellReadonlyTool<C> {
    pub fn new(confirmation: C, timeout: Duration) -> Self {
        Self {
            confirmation,
            timeout,
            unsafe_mode: false,
        }
    }
    pub fn with_unsafe(mut self, enabled: bool) -> Self {
        self.unsafe_mode = enabled;
        self
    }
}
pub fn validate_readonly_command(command: &str, unsafe_mode: bool) -> Result<(), ToolError> {
    if unsafe_mode {
        return Ok(());
    }
    let lower = command.to_ascii_lowercase();
    if BLOCKED_PATTERNS.iter().any(|p| lower.contains(p)) {
        return Err(ToolError::Denied(
            "command matches shell readonly blocklist".into(),
        ));
    }
    Ok(())
}
#[async_trait]
impl<C: Confirmation> Tool for ShellReadonlyTool<C> {
    fn name(&self) -> &str {
        "shell_readonly"
    }
    fn description(&self) -> &str {
        "Run a command under the readonly shell blocklist."
    }
    async fn execute(
        &self,
        call: &ToolCall,
        cancel: CancellationToken,
    ) -> Result<ToolResponse, ToolError> {
        validate_readonly_command(&call.arguments, self.unsafe_mode)?;
        if !self
            .confirmation
            .confirm(&format!("Run readonly command: {}? [y/N]", call.arguments))
            .await
        {
            return Err(ToolError::Denied("confirmation declined".into()));
        }
        let child = Command::new("sh").arg("-c").arg(&call.arguments).output();
        let output = tokio::select! {_=cancel.cancelled()=>return Err(ToolError::Cancelled),r=timeout(self.timeout,child)=>r.map_err(|_|ToolError::Timeout(self.timeout))?.map_err(|e|ToolError::Failed(e.to_string()))?};
        let content =
            String::from_utf8_lossy(&[output.stdout, output.stderr].concat()).into_owned();
        Ok(ToolResponse {
            id: call.id.clone(),
            name: call.name.clone(),
            content,
            success: output.status.success(),
        })
    }
}
#[cfg(test)]
mod readonly_tests {
    use super::*;
    struct Yes;
    #[async_trait]
    impl Confirmation for Yes {
        async fn confirm(&self, _: &str) -> bool {
            true
        }
    }
    #[test]
    fn blocks_dangerous() {
        assert!(validate_readonly_command("rm -rf /", false).is_err());
        assert!(validate_readonly_command("echo hi", false).is_ok());
        assert!(validate_readonly_command("echo hi | cat", false).is_err());
        assert!(validate_readonly_command("rm -rf /", true).is_ok());
    }
    #[tokio::test]
    async fn executes_safe_command() {
        let t = ShellReadonlyTool::new(Yes, Duration::from_secs(2));
        let c = ToolCall {
            id: None,
            name: "shell_readonly".into(),
            arguments: "printf ok".into(),
        };
        assert_eq!(
            t.execute(&c, CancellationToken::new())
                .await
                .unwrap()
                .content,
            "ok"
        );
    }
}
