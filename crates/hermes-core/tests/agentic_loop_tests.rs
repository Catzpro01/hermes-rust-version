use async_trait::async_trait;
use futures::stream;
use hermes_core::{
    conversation::{AgenticResult, ConversationRunner, Event, Turn},
    provider::{EventStream, Provider, ProviderError},
    session::SessionStore,
    tools::{Tool, ToolCall, ToolExecutionStatus, ToolRegistry, ToolResponse},
};
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

struct Scenario {
    responses: Arc<Mutex<Vec<String>>>,
}
#[async_trait]
impl Provider for Scenario {
    async fn chat(&self, _: &[Turn]) -> Result<EventStream, ProviderError> {
        let text = self.responses.lock().unwrap().remove(0);
        Ok(Box::pin(stream::iter(vec![
            Ok(Event::Started),
            Ok(Event::Chunk(text)),
            Ok(Event::Done),
        ])))
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
            (0..10).map(|i| format!("<tool_call id=\"{i}\">echo: x</tool_call>").collect()),
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
