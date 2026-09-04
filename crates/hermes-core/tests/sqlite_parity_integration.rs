use hermes_core::{
    conversation::Turn,
    session::{SessionId, SessionStore},
};
use std::{path::PathBuf, sync::Arc};
use tempfile::tempdir;

#[test]
fn reads_real_hermes_state_db_fixture() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hermes_state.db");
    let store = SessionStore::open(&path).unwrap();
    let sessions = store.list().unwrap();
    assert!(!sessions.is_empty());
    let session = store.resume(&sessions[0]).unwrap();
    assert_eq!(session.turns.len(), 2);
    assert_eq!(
        session.turns[0],
        Turn::User {
            content: "fixture hello".into()
        }
    );
}

#[test]
fn concurrent_writes_to_same_sqlite_session_are_serialized() {
    let dir = tempdir().unwrap();
    let db = Arc::new(dir.path().join("state.db"));
    let creator = SessionStore::open(&db).unwrap();
    let id = creator.create_session("cli").unwrap();
    drop(creator);
    let mut handles = Vec::new();
    for n in 0..2 {
        let db = Arc::clone(&db);
        handles.push(std::thread::spawn(move || {
            let mut store = SessionStore::open(&db).unwrap();
            for i in 0..10 {
                store
                    .save_turn(
                        &id,
                        &Turn::User {
                            content: format!("worker-{n}-{i}"),
                        },
                    )
                    .unwrap();
            }
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
    let store = SessionStore::open(&db).unwrap();
    assert_eq!(store.resume(&id).unwrap().turns.len(), 20);
}

#[test]
fn inspection_queries_are_read_only_and_isolate_sessions() {
    use hermes_core::tools::{ToolCallRecord, ToolExecutionStatus};
    use std::fs;
    let dir = tempdir().unwrap();
    let db = dir.path().join("state.db");
    let mut store = SessionStore::open(&db).unwrap();
    let first = store.create_session("cli").unwrap();
    let second = store.create_session("cli").unwrap();
    store
        .save_turn(
            &first,
            &Turn::User {
                content: "first".into(),
            },
        )
        .unwrap();
    store
        .save_turn(
            &first,
            &Turn::Assistant {
                content: "answer".into(),
            },
        )
        .unwrap();
    store
        .save_tool_call(&ToolCallRecord {
            id: "call-fixture".into(),
            session_id: first.to_string(),
            turn_index: 1,
            tool_name: "read_file".into(),
            arguments: "{\"path\":\"README.md\"}".into(),
            result: "contents".into(),
            status: ToolExecutionStatus::Success,
        })
        .unwrap();
    store
        .save_turn(
            &second,
            &Turn::User {
                content: "second".into(),
            },
        )
        .unwrap();
    drop(store);
    let before = fs::read(&db).unwrap();
    let store = SessionStore::open(&db).unwrap();
    let details = store.session_details(&first).unwrap();
    assert_eq!((details.message_count, details.tool_call_count), (2, 1));
    assert_eq!(
        store
            .list_messages(&first)
            .unwrap()
            .iter()
            .map(|m| m.role.as_str())
            .collect::<Vec<_>>(),
        vec!["user", "assistant"]
    );
    assert_eq!(
        store.list_tool_call_details(&first).unwrap()[0].tool_name,
        "read_file"
    );
    assert_eq!(store.list_messages(&second).unwrap().len(), 1);
    assert!(matches!(
        store.list_messages(&SessionId::new()),
        Err(hermes_core::session::SessionStoreError::NotFound(_))
    ));
    drop(store);
    assert_eq!(
        before,
        fs::read(&db).unwrap(),
        "inspection queries modified canonical state.db"
    );
}
