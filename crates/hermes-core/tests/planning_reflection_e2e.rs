//! Spec 009 closure (Ticket 05) — end-to-end proof of the full pipeline.
//!
//! A scripted, deterministic provider drives `ConversationRunner` through the
//! whole guided flow: goal extracted -> plan generated -> tools executed ->
//! reflection judges -> recovery mutates parameters after a retryable failure
//! -> Done with the goal Achieved — all within the iteration budget. Companion
//! negative tests pin the invariants: an identical failing argument set is
//! never re-executed, a `Denied` tool is never retried, plan/reflection never
//! fabricate a `User` turn (or a new db role), and reactive mode stays a
//! zero-regression Spec 002 loop.

use async_trait::async_trait;
use futures::stream;
use hermes_core::{
    conversation::{AgenticResult, ConversationRunner, Event, Turn},
    provider::tool_aware_stream,
    provider::{EventStream, Provider, ProviderError},
    tools::{Tool, ToolCall, ToolError, ToolRegistry, ToolResponse},
};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// Shared ordered log of executed tool arguments.
type SharedLog = Arc<Mutex<Vec<String>>>;

/// Pops one scripted response text per provider call, in order, wrapped in the
/// tool-aware stream so `<tool_call>` tags become `Event::ToolCall` and plain
/// text becomes `Event::Chunk`. Only overrides `chat`; the `chat_with_instruction`
/// default chain (plan generation) flows through it too.
struct Scripted {
    responses: Arc<Mutex<Vec<String>>>,
}
#[async_trait]
impl Provider for Scripted {
    async fn chat(&self, _: &[Turn]) -> Result<EventStream, ProviderError> {
        let text = self.responses.lock().unwrap().remove(0);
        Ok(tool_aware_stream(Box::pin(stream::iter(vec![
            Ok(Event::Started),
            Ok(Event::Chunk(text)),
            Ok(Event::Done),
        ]))))
    }
}

/// Records every argument it executed (so a repeat can be proven skipped) and
/// fails for "bad"/"fail" (retryable timeout), denies for "deny", else succeeds.
struct Fetch {
    log: SharedLog,
}
#[async_trait]
impl Tool for Fetch {
    fn name(&self) -> &str {
        "fetch"
    }
    fn description(&self) -> &str {
        "test fetch that fails on 'bad', denies on 'deny', else succeeds"
    }
    async fn execute(
        &self,
        c: &ToolCall,
        _: CancellationToken,
    ) -> Result<ToolResponse, ToolError> {
        self.log.lock().unwrap().push(c.arguments.clone());
        match c.arguments.as_str() {
            "bad" | "fail" => Err(ToolError::Timeout(Duration::from_secs(1))),
            "deny" => Err(ToolError::Denied("operator declined".into())),
            a => Ok(ToolResponse {
                id: c.id.clone(),
                name: c.name.clone(),
                content: format!("data for {a}"),
                success: true,
            }),
        }
    }
}

/// Always succeeds; represents a second tool step in a multi-step goal.
struct Note {
    log: SharedLog,
}
#[async_trait]
impl Tool for Note {
    fn name(&self) -> &str {
        "note"
    }
    fn description(&self) -> &str {
        "test note that always succeeds"
    }
    async fn execute(
        &self,
        c: &ToolCall,
        _: CancellationToken,
    ) -> Result<ToolResponse, ToolError> {
        self.log.lock().unwrap().push(c.arguments.clone());
        Ok(ToolResponse {
            id: c.id.clone(),
            name: c.name.clone(),
            content: format!("noted: {}", c.arguments),
            success: true,
        })
    }
}

fn guided_registry() -> (ToolRegistry, SharedLog, SharedLog) {
    let fetch_log = Arc::new(Mutex::new(Vec::new()));
    let note_log = Arc::new(Mutex::new(Vec::new()));
    let mut reg = ToolRegistry::new();
    reg.register(Fetch { log: fetch_log.clone() });
    reg.register(Note { log: note_log.clone() });
    (reg, fetch_log, note_log)
}

fn count_user_turns(r: &ConversationRunner<Scripted>) -> usize {
    r.turns()
        .iter()
        .filter(|t| matches!(t, Turn::User { .. }))
        .count()
}

/// Positive: goal -> plan -> execute (one retryable failure) -> mutate param ->
/// second tool step -> Done with the goal Achieved. The repeated identical
/// failing argument is intercepted, not executed a second time.
#[tokio::test]
async fn full_pipeline_plans_reflects_recovers_and_marks_goal_achieved() {
    let (reg, fetch_log, note_log) = guided_registry();
    let p = Scripted {
        responses: Arc::new(Mutex::new(vec![
            // plan round
            "[[plan]]\n1. fetch the dataset\n2. note the result\n[[/plan]]".into(),
            // exec 1: fetch fails retryably on "bad"
            "<tool_call id=\"1\">fetch: bad</tool_call>".into(),
            // exec 2: identical repeat is rejected by recovery (not executed)
            "<tool_call id=\"2\">fetch: bad</tool_call>".into(),
            // exec 3: recovery drove a parameter mutation -> fetch "good"
            "<tool_call id=\"3\">fetch: good</tool_call>".into(),
            // exec 4: second tool step
            "<tool_call id=\"4\">note: stored</tool_call>".into(),
            // exec 5: final answer, no tool -> Done + goal Achieved
            "final: dataset fetched and noted.".into(),
        ])),
    };
    let mut r = ConversationRunner::new(p);
    r.set_goal_tracking(true);
    r.set_plan_mode(true);
    r.set_reflection(true); // recovery_enabled == reflection_enabled
    let out = r
        .chat_agentic("task", &reg, None, 10, CancellationToken::new())
        .await
        .unwrap();
    match out {
        AgenticResult::Done { text, .. } => assert!(text.contains("final"), "got: {text}"),
        other => panic!("expected Done, got {other:?}"),
    }
    // Goal lifecycle closed as Achieved.
    assert_eq!(r.goal(), Some("task"));
    assert_eq!(r.goal_status(), hermes_core::conversation::goal::GoalStatus::Achieved);
    // A plan was generated and retained in memory.
    let plan = r.plan().expect("plan must exist");
    assert_eq!(plan.steps().len(), 2);
    // Reflection actually ran.
    assert!(r.reflections_used() >= 1);
    // fetch executed exactly once per distinct argument; the identical "bad"
    // repeat was NOT executed.
    let fetches = fetch_log.lock().unwrap().clone();
    assert_eq!(fetches, vec!["bad".to_string(), "good".to_string()]);
    // second tool step executed once.
    assert_eq!(*note_log.lock().unwrap(), vec!["stored".to_string()]);
    // No fabricated user turn: only the single initiating user turn exists.
    assert_eq!(count_user_turns(&r), 1, "plan/reflection must not add a User turn");
}

/// Negative: in a planned, reflective session a `Denied` tool immediately
/// Blocks the goal and is never retried.
#[tokio::test]
async fn denied_in_planned_session_blocks_and_is_never_retried() {
    let (reg, fetch_log, _note_log) = guided_registry();
    let p = Scripted {
        responses: Arc::new(Mutex::new(vec![
            "[[plan]]\n1. fetch it\n[[/plan]]".into(),
            "<tool_call id=\"5\">fetch: deny</tool_call>".into(),
        ])),
    };
    let mut r = ConversationRunner::new(p);
    r.set_goal_tracking(true);
    r.set_plan_mode(true);
    r.set_reflection(true);
    let out = r
        .chat_agentic("task", &reg, None, 10, CancellationToken::new())
        .await
        .unwrap();
    assert!(
        matches!(out, AgenticResult::Blocked { .. }),
        "a denied tool must Block, got {out:?}"
    );
    assert_eq!(r.goal_status(), hermes_core::conversation::goal::GoalStatus::Blocked);
    // The denied call was attempted exactly once and never retried.
    assert_eq!(*fetch_log.lock().unwrap(), vec!["deny".to_string()]);
    // No fabricated user turn (plan/reflection add none).
    assert_eq!(count_user_turns(&r), 1);
}

/// Reactive mode (no /plan, no /reflect, no /goal) remains a plain Spec 002
/// loop: Done, no plan, no goal, no block.
#[tokio::test]
async fn reactive_mode_is_zero_regression_spec002() {
    let (reg, fetch_log, _note_log) = guided_registry();
    let p = Scripted {
        responses: Arc::new(Mutex::new(vec![
            // no plan round; straight to a final answer with a harmless tool use
            "<tool_call id=\"9\">fetch: good</tool_call>".into(),
            "plain reactive answer".into(),
        ])),
    };
    let mut r = ConversationRunner::new(p);
    // No /plan, no /reflect, no /goal.
    let out = r
        .chat_agentic("hi", &reg, None, 10, CancellationToken::new())
        .await
        .unwrap();
    assert!(matches!(out, AgenticResult::Done { .. }), "got {out:?}");
    assert!(r.plan().is_none(), "reactive mode must not plan");
    assert_eq!(r.goal(), None, "reactive mode must not track a goal");
    assert_eq!(r.goal_status(), hermes_core::conversation::goal::GoalStatus::NotStarted);
    assert_eq!(*fetch_log.lock().unwrap(), vec!["good".to_string()]);
}
