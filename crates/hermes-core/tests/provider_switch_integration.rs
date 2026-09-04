//! Verifies the mid-session provider switch primitive: `ConversationRunner`
//! can replace its backing provider without losing conversation history, and
//! the replacement takes effect on the very next turn.

use async_trait::async_trait;
use futures::StreamExt;
use hermes_core::{
    conversation::{ConversationRunner, Event, Turn},
    provider::{EventStream, FakeProvider, Provider, ProviderError},
};

/// A tiny distinguishable provider so a test can tell which provider answered.
struct TaggedProvider(&'static str);
#[async_trait]
impl Provider for TaggedProvider {
    async fn chat(&self, _turns: &[Turn]) -> Result<EventStream, ProviderError> {
        Ok(Box::pin(futures::stream::iter(vec![
            Ok(Event::Chunk(self.0.into())),
            Ok(Event::Done),
        ])))
    }
}

async fn collect_chunks(stream: EventStream) -> String {
    let mut out = String::new();
    let mut stream = stream;
    while let Some(event) = stream.next().await {
        if let Ok(Event::Chunk(c)) = event {
            out.push_str(&c);
        }
    }
    out
}

#[tokio::test]
async fn replacing_provider_preserves_history_and_applies_to_next_turn() {
    let mut runner: ConversationRunner<Box<dyn Provider>> =
        ConversationRunner::from_turns(Box::new(FakeProvider), Vec::new());

    // First turn answered by the fake provider.
    let first = collect_chunks(runner.chat("hello").await.unwrap()).await;
    assert!(first.contains("echo: hello"), "got: {first}");
    runner.push_assistant(first.clone());

    let history_before = runner.turns().len();
    assert!(history_before >= 2, "user + assistant turns should exist");

    // Switch providers mid-session: history must be preserved verbatim.
    runner.replace_provider(Box::new(TaggedProvider("second-answered")));
    assert_eq!(
        runner.turns().len(),
        history_before,
        "switching provider must not mutate history"
    );

    // The next turn must be served by the new provider.
    let second = collect_chunks(runner.chat("world").await.unwrap()).await;
    assert!(second.contains("second-answered"), "got: {second}");

    // Both user turns are still part of the conversation.
    let user_msgs: Vec<&str> = runner
        .turns()
        .iter()
        .filter_map(|t| match t {
            Turn::User { content } => Some(content.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(user_msgs, vec!["hello", "world"]);
}

#[tokio::test]
async fn replacing_provider_does_not_lose_prior_assistant_history() {
    let mut runner: ConversationRunner<Box<dyn Provider>> = ConversationRunner::new(Box::new(
        TaggedProvider("alpha"),
    ));
    let first = collect_chunks(runner.chat("msg1").await.unwrap()).await;
    assert!(first.contains("alpha"));
    runner.push_assistant(first);
    let history_before = runner.turns().to_vec();

    runner.replace_provider(Box::new(FakeProvider));
    assert!(
        runner.turns().iter().eq(history_before.iter()),
        "history changed on provider replacement"
    );

    // The next turn is answered by the new provider, with the full prior
    // conversation (including the stored assistant text) still intact.
    let second = collect_chunks(runner.chat("hi again").await.unwrap()).await;
    assert!(second.contains("echo: hi again"), "got: {second}");
    let assistant_msgs: Vec<String> = runner
        .turns()
        .iter()
        .filter_map(|t| match t {
            Turn::Assistant { content } => Some(content.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(assistant_msgs, vec!["alpha"]);
}
