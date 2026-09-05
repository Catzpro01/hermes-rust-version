use async_trait::async_trait;
use futures::stream;
use hermes_core::{
    conversation::{AgenticResult, ConversationRunner, Event, Turn},
    provider::tool_aware_stream,
    provider::{EventStream, Provider, ProviderError},
    session::SessionStore,
    tools::{Tool, ToolCall, ToolError, ToolExecutionStatus, ToolRegistry, ToolResponse},
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
struct Scenario {
    responses: Arc<Mutex<Vec<String>>>,
}
#[async_trait]
impl Provider for Scenario {
    async fn chat(&self, _: &[Turn]) -> Result<EventStream, ProviderError> {
        let text = self.responses.lock().unwrap().remove(0);
        Ok(tool_aware_stream(Box::pin(stream::iter(vec![
            Ok(Event::Started),
            Ok(Event::Chunk(text)),
            Ok(Event::Done),
        ]))))
    }
}
struct Echo;
#[async_trait]
impl Tool for Echo {
    fn name(&self) -> &str {
        "echo"
    }
    fn description(&self) -> &str {
        "test echo"
    }
    async fn execute(
        &self,
        c: &ToolCall,
        _: CancellationToken,
    ) -> Result<ToolResponse, hermes_core::tools::ToolError> {
        Ok(ToolResponse {
            id: c.id.clone(),
            name: c.name.clone(),
            content: c.arguments.clone(),
            success: true,
        })
    }
}
fn registry() -> ToolRegistry {
    let mut r = ToolRegistry::new();
    r.register(Echo);
    r
}
#[tokio::test]
async fn agentic_three_iterations_persist_tool_calls() {
    let d = tempfile::tempdir().unwrap();
    let store = SessionStore::open(&d.path().join("state.db")).unwrap();
    let id = store.create_session("test").unwrap();
    let p = Scenario {
        responses: Arc::new(Mutex::new(vec![
            "<tool_call id=\"1\">echo: one</tool_call>".into(),
            "<tool_call id=\"2\">echo: two</tool_call>".into(),
            "final".into(),
        ])),
    };
    let mut r = ConversationRunner::new(p);
    let out = r
        .chat_agentic(
            "go",
            &registry(),
            Some((&store, &id)),
            10,
            CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(
        out,
        AgenticResult::Done {
            text: "final".into(),
            iterations: 3
        }
    );
    assert_eq!(store.list_tool_calls(&id).unwrap().len(), 2);
    assert_eq!(
        store.list_tool_calls(&id).unwrap()[0].1,
        ToolExecutionStatus::Success
    );
}
#[tokio::test]
async fn agentic_stops_at_ten_iterations() {
    let d = tempfile::tempdir().unwrap();
    let store = SessionStore::open(&d.path().join("state.db")).unwrap();
    let id = store.create_session("test").unwrap();
    let p = Scenario {
        responses: Arc::new(Mutex::new(
            (0..10)
                .map(|i| format!("<tool_call id=\"{i}\">echo: x</tool_call>"))
                .collect(),
        )),
    };
    let mut r = ConversationRunner::new(p);
    let out = r
        .chat_agentic(
            "go",
            &registry(),
            Some((&store, &id)),
            10,
            CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(out, AgenticResult::MaxIterations(10));
    assert_eq!(store.list_tool_calls(&id).unwrap().len(), 10);
}

// -- Spec 009 recovery (Ticket 04) -----------------------------------------

/// A tool that always fails retryably (timeout), counting executions so a test
/// can assert an identical repeat is NOT re-executed by recovery.
struct Flaky {
    calls: Arc<AtomicUsize>,
}
#[async_trait]
impl Tool for Flaky {
    fn name(&self) -> &str {
        "flaky"
    }
    fn description(&self) -> &str {
        "always times out (retryable failure)"
    }
    async fn execute(
        &self,
        _: &ToolCall,
        _: CancellationToken,
    ) -> Result<ToolResponse, ToolError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(ToolError::Timeout(Duration::from_secs(1)))
    }
}
fn flaky_registry() -> (ToolRegistry, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut r = ToolRegistry::new();
    r.register(Flaky { calls: calls.clone() });
    (r, calls)
}

/// A tool that is always denied (a user/human decision that must never be
/// retried). Counts executions to prove a denial is not re-attempted.
struct Deny {
    calls: Arc<AtomicUsize>,
}
#[async_trait]
impl Tool for Deny {
    fn name(&self) -> &str {
        "deny"
    }
    fn description(&self) -> &str {
        "always denied"
    }
    async fn execute(
        &self,
        _: &ToolCall,
        _: CancellationToken,
    ) -> Result<ToolResponse, ToolError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(ToolError::Denied("operator declined".into()))
    }
}
fn deny_registry() -> (ToolRegistry, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut r = ToolRegistry::new();
    r.register(Deny { calls: calls.clone() });
    (r, calls)
}

/// Recovery is opt-in through the reflection gate. Each model turn either
/// executes a failing tool (recording its argument set) or is intercepted when
/// it repeats an already-failed argument set (NOT re-executed). Distinct
/// failures bound at 3 -> the step is Blocked and the loop early-stops.
#[tokio::test]
async fn recovery_bounds_retryable_failures_and_does_not_reexecute_identical() {
    let (reg, calls) = flaky_registry();
    // Iterations: a fails, a (repeat) intercepted, b fails, b (repeat)
    // intercepted, c fails (3 distinct -> blocked), then early-stop.
    let p = Scenario {
        responses: Arc::new(Mutex::new(vec![
            "<tool_call id=\"1\">flaky: a</tool_call>".into(),
            "<tool_call id=\"2\">flaky: a</tool_call>".into(),
            "<tool_call id=\"3\">flaky: b</tool_call>".into(),
            "<tool_call id=\"4\">flaky: b</tool_call>".into(),
            "<tool_call id=\"5\">flaky: c</tool_call>".into(),
        ])),
    };
    let mut r = ConversationRunner::new(p);
    r.set_reflection(true); // recovery_enabled == reflection_enabled
    r.set_goal_tracking(true);
    let out = r
        .chat_agentic("task", &reg, None, 10, CancellationToken::new())
        .await
        .unwrap();
    assert!(
        matches!(out, AgenticResult::Blocked { .. }),
        "retries exhausted must Block, got {out:?}"
    );
    // Only the three *distinct* argument sets executed; the two exact repeats
    // were intercepted and NOT re-executed.
    assert_eq!(calls.load(Ordering::SeqCst), 3, "identical repeats must be skipped");
}

/// A user denial must never be retried: it immediately blocks the goal and the
/// loop early-stops rather than attempting the denied call again.
#[tokio::test]
async fn recovery_never_retries_a_denied_tool() {
    let (reg, calls) = deny_registry();
    let p = Scenario {
        responses: Arc::new(Mutex::new(vec![
            "<tool_call id=\"1\">deny: x</tool_call>".into(),
        ])),
    };
    let mut r = ConversationRunner::new(p);
    r.set_reflection(true);
    r.set_goal_tracking(true);
    let out = r
        .chat_agentic("task", &reg, None, 10, CancellationToken::new())
        .await
        .unwrap();
    assert!(
        matches!(out, AgenticResult::Blocked { .. }),
        "a denied tool must Block immediately, got {out:?}"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1, "a denied call must never be retried");
}
