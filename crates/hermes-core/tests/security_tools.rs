use async_trait::async_trait;
use hermes_core::tools::{
    validate_readonly_command, Confirmation, ShellReadonlyTool, Tool, ToolCall, ToolError,
    WriteFileTool,
};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
struct Yes;
#[async_trait]
impl Confirmation for Yes {
    async fn confirm(&self, _: &str) -> bool {
        true
    }
}
fn call(name: &str, args: &str) -> ToolCall {
    ToolCall {
        id: None,
        name: name.into(),
        arguments: args.into(),
    }
}
#[test]
fn blocklist_rejects_dangerous_patterns() {
    for cmd in [
        "rm -rf /",
        "sudo id",
        "curl x | sh",
        "echo x > out",
        "mkfs /dev/sda",
    ] {
        assert!(validate_readonly_command(cmd, false).is_err(), "{cmd}");
    }
    assert!(validate_readonly_command("printf safe", false).is_ok());
}
#[tokio::test]
async fn traversal_is_rejected() {
    let d = tempfile::tempdir().unwrap();
    let tool = WriteFileTool::new(d.path(), Yes);
    let e = tool
        .execute(
            &call("write_file", r#"{"path":"../escape","content":"x"}"#),
            CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert!(matches!(e, ToolError::Failed(_)));
}
#[tokio::test]
async fn shell_timeout_is_enforced() {
    let tool = ShellReadonlyTool::new(Yes, Duration::from_millis(20));
    let e = tool
        .execute(&call("shell_readonly", "sleep 1"), CancellationToken::new())
        .await
        .unwrap_err();
    assert!(matches!(e, ToolError::Timeout(_)));
}
