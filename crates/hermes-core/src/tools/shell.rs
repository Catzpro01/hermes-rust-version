use super::sandbox::{run_shell, SandboxPolicy};
use super::{Tool, ToolCall, ToolError, ToolResponse};
use async_trait::async_trait;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
#[async_trait]
pub trait Confirmation: Send + Sync {
    async fn confirm(&self, command: &str) -> bool;
}
pub struct ShellTool<C> {
    confirmation: C,
    timeout: Duration,
    sandbox: SandboxPolicy,
}
impl<C> ShellTool<C> {
    /// Legacy constructor: inherits the process environment (Spec 002
    /// behaviour, `SandboxPolicy::inherit`).
    pub fn new(confirmation: C, timeout: Duration) -> Self {
        Self {
            confirmation,
            timeout,
            sandbox: SandboxPolicy::inherit(),
        }
    }
    /// Spec 007: run every command inside `policy`.
    pub fn with_sandbox(mut self, policy: SandboxPolicy) -> Self {
        self.sandbox = policy;
        self
    }
    pub fn sandbox(&self) -> &SandboxPolicy {
        &self.sandbox
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
        let out = run_shell(&self.sandbox, &call.arguments, self.timeout, cancel).await?;
        Ok(ToolResponse {
            id: call.id.clone(),
            name: call.name.clone(),
            success: out.success,
            content: out.content,
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
    sandbox: SandboxPolicy,
}
impl<C> ShellReadonlyTool<C> {
    /// Legacy constructor: inherits the process environment (Spec 002
    /// behaviour, `SandboxPolicy::inherit`).
    pub fn new(confirmation: C, timeout: Duration) -> Self {
        Self {
            confirmation,
            timeout,
            unsafe_mode: false,
            sandbox: SandboxPolicy::inherit(),
        }
    }
    pub fn with_unsafe(mut self, enabled: bool) -> Self {
        self.unsafe_mode = enabled;
        self
    }
    /// Spec 007: run every command inside `policy`. The blocklist still
    /// applies first — the sandbox is defense in depth, not a replacement.
    pub fn with_sandbox(mut self, policy: SandboxPolicy) -> Self {
        self.sandbox = policy;
        self
    }
    pub fn sandbox(&self) -> &SandboxPolicy {
        &self.sandbox
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
        let out = run_shell(&self.sandbox, &call.arguments, self.timeout, cancel).await?;
        Ok(ToolResponse {
            id: call.id.clone(),
            name: call.name.clone(),
            content: out.content,
            success: out.success,
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
    #[tokio::test]
    async fn sandboxed_readonly_still_applies_blocklist_first() {
        let t = ShellReadonlyTool::new(Yes, Duration::from_secs(2))
            .with_sandbox(SandboxPolicy::strict(std::env::temp_dir()));
        let c = ToolCall {
            id: None,
            name: "shell_readonly".into(),
            arguments: "rm -rf /".into(),
        };
        assert!(matches!(
            t.execute(&c, CancellationToken::new()).await.unwrap_err(),
            ToolError::Denied(_)
        ));
        assert!(t.sandbox().enabled);
    }
}
