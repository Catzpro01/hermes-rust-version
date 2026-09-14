//! Spec 007 closure proof — tool execution sandbox through the real agentic
//! loop. A scripted provider issues `shell_readonly` calls; the tool runs
//! inside a `SandboxPolicy` and the assertions pin the boundary:
//! parent secrets are invisible, cwd is jailed, output is capped, rlimits
//! bite, the blocklist/confirmation gates still run first, and the default
//! (inherit) path behaves exactly as Spec 002.
use async_trait::async_trait;
use futures::stream;
use hermes_core::{
    conversation::{AgenticResult, ConversationRunner, Event, Turn},
    provider::{tool_aware_stream, EventStream, Provider, ProviderError},
    session::SessionStore,
    tools::{
        sandbox::{OUTPUT_TRUNCATED_MARKER, SANDBOX_ENV_FLAG},
        Confirmation, ResourceLimits, SandboxPolicy, ShellReadonlyTool, ShellTool, Tool, ToolCall,
        ToolError, ToolExecutionStatus, ToolRegistry,
    },
};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

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
#[derive(Clone)]
struct Yes;
#[async_trait]
impl Confirmation for Yes {
    async fn confirm(&self, _: &str) -> bool {
        true
    }
}
#[derive(Clone)]
struct No;
#[async_trait]
impl Confirmation for No {
    async fn confirm(&self, _: &str) -> bool {
        false
    }
}
fn call(name: &str, args: &str) -> ToolCall {
    ToolCall {
        id: None,
        name: name.into(),
        arguments: args.into(),
    }
}
const SECRET_VAR: &str = "HERMES_E2E_SECRET_007";
const SECRET_VAL: &str = "sk-proj-e2e-sandbox-secret-0007";

#[tokio::test]
async fn spec007_sandboxed_shell_through_agentic_loop_hides_secrets_and_jails_cwd() {
    std::env::set_var(SECRET_VAR, SECRET_VAL);
    let jail = tempfile::tempdir().unwrap();
    let d = tempfile::tempdir().unwrap();
    let store = SessionStore::open(&d.path().join("state.db")).unwrap();
    let id = store.create_session("test").unwrap();

    let mut registry = ToolRegistry::new();
    registry.register(
        ShellReadonlyTool::new(Yes, Duration::from_secs(10))
            .with_sandbox(SandboxPolicy::strict(jail.path())),
    );
    let script = format!(
        "<tool_call id=\"1\">shell_readonly: printf 'secret=%s flag=%s cwd=' \"${SECRET_VAR}\" \"${SANDBOX_ENV_FLAG}\"; pwd</tool_call>"
    );
    let p = Scripted(Arc::new(Mutex::new(vec![script, "final".into()])));
    let mut r = ConversationRunner::new(p);
    let out = r
        .chat_agentic(
            "go",
            &registry,
            Some((&store, &id)),
            5,
            CancellationToken::new(),
        )
        .await
        .unwrap();
    assert!(matches!(out, AgenticResult::Done { .. }), "{out:?}");

    let calls = store.list_tool_call_details(&id).unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].status, ToolExecutionStatus::Success.as_str());
    let canon = jail.path().canonicalize().unwrap();
    assert_eq!(
        calls[0].result,
        format!("secret= flag=1 cwd={}\n", canon.display()),
        "secret invisible, sandbox flag set, cwd jailed"
    );
    // The secret value never reaches the transcript either.
    for t in r.turns() {
        if let Turn::Tool { content, .. } = t {
            assert!(!content.contains(SECRET_VAL), "leak: {content}");
        }
    }
    std::env::remove_var(SECRET_VAR);
}

#[tokio::test]
async fn spec007_inherit_default_behaves_like_spec002() {
    std::env::set_var("HERMES_E2E_INHERIT_007", "visible");
    // Legacy constructor == inherit policy: the parent env is visible.
    let legacy = ShellReadonlyTool::new(Yes, Duration::from_secs(5));
    assert!(!legacy.sandbox().enabled);
    let out = legacy
        .execute(
            &call(
                "shell_readonly",
                "printf \"$HERMES_E2E_INHERIT_007|$HERMES_SANDBOX\"",
            ),
            CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(out.content, "visible|");
    std::env::remove_var("HERMES_E2E_INHERIT_007");
}

#[tokio::test]
async fn spec007_output_cap_and_rlimits_apply_to_both_shell_tools() {
    let jail = tempfile::tempdir().unwrap();
    let mut policy = SandboxPolicy::strict(jail.path());
    policy.max_output_bytes = 16;
    policy.limits = ResourceLimits {
        max_file_size_kb: Some(1),
        ..Default::default()
    };
    let ro = ShellReadonlyTool::new(Yes, Duration::from_secs(10)).with_sandbox(policy.clone());
    let rw = ShellTool::new(Yes, Duration::from_secs(10)).with_sandbox(policy);

    // Output cap (readonly tool: no redirect, so the blocklist passes).
    let out = ro
        .execute(
            &call("shell_readonly", "printf '%032d' 7"),
            CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(
        out.content,
        format!("{}{OUTPUT_TRUNCATED_MARKER}", "0".repeat(16))
    );

    // fsize rlimit (full shell: redirects allowed, still confined).
    let out = rw
        .execute(
            &call("shell", "head -c 8192 /dev/zero > big.bin; ls -l big.bin"),
            CancellationToken::new(),
        )
        .await
        .unwrap();
    let size = std::fs::metadata(jail.path().join("big.bin"))
        .map(|m| m.len())
        .unwrap_or(0);
    assert!(
        size <= 1024,
        "fsize limit must cap the file, got {size}: {out:?}"
    );
    // And the file landed inside the jail, not in the test's cwd.
    assert!(!std::path::Path::new("big.bin").exists());
}

#[tokio::test]
async fn spec007_gates_still_run_before_the_sandbox() {
    let jail = tempfile::tempdir().unwrap();
    let policy = SandboxPolicy::strict(jail.path());
    // Blocklist first.
    let ro = ShellReadonlyTool::new(Yes, Duration::from_secs(5)).with_sandbox(policy.clone());
    let e = ro
        .execute(
            &call("shell_readonly", "curl http://x"),
            CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert!(matches!(e, ToolError::Denied(_)));
    // Confirmation second — a decline never spawns anything.
    let marker = jail.path().join("spawned");
    let rw = ShellTool::new(No, Duration::from_secs(5)).with_sandbox(policy);
    let e = rw
        .execute(
            &call("shell", &format!("touch {}", marker.display())),
            CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert!(matches!(e, ToolError::Denied(_)));
    assert!(!marker.exists(), "denied command must not run");
}

#[tokio::test]
async fn spec007_hostile_command_cannot_escape_the_ulimit_wrapper() {
    let jail = tempfile::tempdir().unwrap();
    let mut policy = SandboxPolicy::strict(jail.path());
    policy.limits.cpu_seconds = Some(30);
    let rw = ShellTool::new(Yes, Duration::from_secs(10)).with_sandbox(policy);
    // Quotes / `$1` / `"` in the model's string are inert: it is argv[3], and
    // the wrapper only ever runs `sh -c "$1"`.
    let out = rw
        .execute(
            &call("shell", "printf '%s' \"a\\\"b'c $1 $0\""),
            CancellationToken::new(),
        )
        .await
        .unwrap();
    assert!(out.success, "{out:?}");
    assert_eq!(out.content, "a\"b'c  sh");
}
