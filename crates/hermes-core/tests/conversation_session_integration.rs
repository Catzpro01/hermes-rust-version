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

/// Ticket 02 invariant: the sliding window trims only what is *sent* to the
/// provider (`turns_to_send`), never `self.turns` nor `state.db`. A resumed
/// session and `/messages` (both read `state.db`) therefore still show the full
/// history, and read/resume leaves the file byte-identical.
#[test]
fn sliding_window_trims_send_but_state_db_keeps_full_history() {
    use hermes_core::conversation::context::estimate_turns_tokens;
    use std::fs;

    let dir = tempdir().unwrap();
    let db = dir.path().join("state.db");
    let mut store = SessionStore::open(&db).unwrap();
    let id = store.create_session("cli").unwrap();

    // 100 user turns, each 40 chars => ~10 tokens each => ~1000 tokens total.
    let history: Vec<Turn> = (0..100)
        .map(|i| Turn::User {
            content: format!("{i}-{}", "x".repeat(40)),
        })
        .collect();
    let mut runner = ConversationRunner::from_turns(FakeProvider, history.clone());
    runner.set_context_limit(Some(150)); // fits ~15 turns

    // The window trims the send-side copy...
    let sent = runner.turns_to_send();
    assert!(sent.len() < 100, "window must drop old turns");
    assert!(
        estimate_turns_tokens(&sent) <= 150,
        "send window must fit the limit"
    );
    assert_eq!(sent.last(), history.last(), "newest turn always sent");
    // ...but leaves self.turns (the full history) untouched.
    assert_eq!(runner.turns().len(), 100, "self.turns must not be mutated");

    // REPL persists the FULL history from self.turns into state.db.
    let turns = runner.turns().to_vec();
    for t in turns {
        store.save_turn(&id, &t).unwrap();
    }
    drop(store);

    // state.db holds all 100 turns and read/resume does not mutate it.
    let before = fs::read(&db).unwrap();
    let store2 = SessionStore::open(&db).unwrap();
    assert_eq!(store2.resume(&id).unwrap().turns.len(), 100);
    assert_eq!(store2.list_messages(&id).unwrap().len(), 100);
    drop(store2);
    assert_eq!(
        before,
        fs::read(&db).unwrap(),
        "read/resume must not alter state.db (window does not touch storage)"
    );
}
