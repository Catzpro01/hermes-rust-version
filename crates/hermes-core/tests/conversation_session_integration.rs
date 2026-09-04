use futures::StreamExt;
use hermes_core::{
    conversation::{ConversationRunner, Event, Turn},
    provider::{FakeProvider, Provider},
    session::{SessionId, SessionStore},
};
use tempfile::tempdir;

#[tokio::test]
async fn fake_provider_emits_started_chunk_done_and_runner_keeps_history() {
    let mut runner = ConversationRunner::new(FakeProvider);
    let events = runner.prompt("hello").await.unwrap();
    assert_eq!(
        events,
        vec![
            Event::Started,
            Event::Chunk("echo: hello".into()),
            Event::Done
        ]
    );
    assert_eq!(
        runner.turns(),
        &[
            Turn::User {
                content: "hello".into()
            },
            Turn::Assistant {
                content: "echo: hello".into()
            }
        ]
    );
}

#[tokio::test]
async fn fake_provider_error_is_deterministic() {
    let stream = FakeProvider
        .chat(&[Turn::User {
            content: "error".into(),
        }])
        .await
        .unwrap();
    let events: Vec<_> = stream.collect().await;
    assert!(events[0].as_ref().is_ok());
    assert!(events[1].is_err());
}

#[test]
fn sqlite_session_roundtrip_and_time_sorted_list() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("state.db");
    let mut store = SessionStore::open(&db).unwrap();
    let id = store.create_session("cli").unwrap();
    store
        .save_turn(
            &id,
            &Turn::User {
                content: "hello".into(),
            },
        )
        .unwrap();
    store
        .save_turn(
            &id,
            &Turn::Assistant {
                content: "world".into(),
            },
        )
        .unwrap();
    let resumed = store.resume(&id).unwrap();
    assert_eq!(resumed.id, id);
    assert_eq!(resumed.source, "cli");
    assert_eq!(resumed.turns.len(), 2);
    assert_eq!(store.list().unwrap(), vec![id]);
}

#[test]
fn unknown_session_is_rejected() {
    let dir = tempdir().unwrap();
    let store = SessionStore::open(&dir.path().join("state.db")).unwrap();
    let missing = SessionId::new();
    assert!(store.resume(&missing).is_err());
}
