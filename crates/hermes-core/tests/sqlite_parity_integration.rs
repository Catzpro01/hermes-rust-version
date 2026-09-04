use hermes_core::{conversation::Turn, session::SessionStore};
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
