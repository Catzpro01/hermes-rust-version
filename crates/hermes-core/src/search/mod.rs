pub mod error;
pub mod migration;
pub mod query;
pub mod state;

pub use error::SearchError;
pub use query::{escape_fts5_literal, search_messages, SearchLimits, SearchResult};
pub use state::SearchState;

use rusqlite::Connection;

/// Checks the compiled SQLite feature without changing canonical tables.
pub fn check_fts5_available(conn: &Connection) -> Result<bool, SearchError> {
    let result = conn.execute_batch("CREATE VIRTUAL TABLE temp.fts5_availability_probe USING fts5(value); DROP TABLE temp.fts5_availability_probe;");
    Ok(result.is_ok())
}

const CREATE_INDEX_SQL: &str = "CREATE VIRTUAL TABLE message_search USING fts5(content, role, session_id UNINDEXED, content='messages', content_rowid='id');";

fn index_exists(conn: &Connection) -> Result<bool, SearchError> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='message_search')",
        [],
        |r| r.get(0),
    )
    .map_err(SearchError::RebuildFailed)
}
fn index_schema_is_valid(conn: &Connection) -> Result<bool, SearchError> {
    let mut stmt = conn.prepare("SELECT name FROM pragma_table_info('message_search') WHERE name IN ('content','role','session_id') ORDER BY name").map_err(SearchError::RebuildFailed)?;
    let columns = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(SearchError::RebuildFailed)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(SearchError::RebuildFailed)?;
    Ok(columns == vec!["content", "role", "session_id"])
}
pub fn repair_index(conn: &Connection) -> Result<usize, SearchError> {
    if !index_exists(conn)? {
        conn.execute_batch(CREATE_INDEX_SQL)
            .map_err(SearchError::RebuildFailed)?;
    } else if !index_schema_is_valid(conn)? {
        return Err(SearchError::IndexCorrupt {
            reason: "message_search schema does not match the versioned contract".into(),
        });
    }
    rebuild_index(conn)
}
pub fn rebuild_index(conn: &Connection) -> Result<usize, SearchError> {
    if !index_exists(conn)? {
        return Err(SearchError::IndexNotReady);
    }
    if !index_schema_is_valid(conn)? {
        return Err(SearchError::IndexCorrupt {
            reason: "message_search schema does not match the versioned contract".into(),
        });
    }
    conn.execute_batch("INSERT INTO message_search(message_search) VALUES('rebuild');")
        .map_err(SearchError::RebuildFailed)?;
    conn.query_row("SELECT COUNT(*) FROM message_search", [], |row| {
        row.get::<_, i64>(0)
    })
    .map(|count| count as usize)
    .map_err(SearchError::RebuildFailed)
}

pub fn index_count(conn: &Connection) -> Result<usize, SearchError> {
    conn.query_row("SELECT COUNT(*) FROM message_search", [], |row| {
        row.get::<_, i64>(0)
    })
    .map(|count| count as usize)
    .map_err(SearchError::RebuildFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::Turn;
    use crate::session::SessionStore;
    use tempfile::tempdir;

    #[test]
    fn fts5_availability_is_detected() {
        let conn = Connection::open_in_memory().unwrap();
        assert!(check_fts5_available(&conn).unwrap());
    }

    #[test]
    fn rebuild_empty_and_populated_index() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("state.db");
        let mut store = SessionStore::open(&db).unwrap();
        let id = store.create_session("test").unwrap();
        assert_eq!(rebuild_index(store.connection_for_tests()).unwrap(), 0);
        store
            .save_turn(
                &id,
                &Turn::User {
                    content: "hello search".into(),
                },
            )
            .unwrap();
        assert_eq!(rebuild_index(store.connection_for_tests()).unwrap(), 1);
    }
}

#[cfg(test)]
mod hardening_tests {
    use super::*;
    use crate::{conversation::Turn, session::SessionStore};
    use tempfile::tempdir;

    #[test]
    fn rebuild_is_idempotent_and_isolates_sessions() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("state.db");
        let mut store = SessionStore::open(&db).unwrap();
        let a = store.create_session("a").unwrap();
        let b = store.create_session("b").unwrap();
        store
            .save_turn(
                &a,
                &Turn::User {
                    content: "alpha".into(),
                },
            )
            .unwrap();
        store
            .save_turn(
                &b,
                &Turn::User {
                    content: "beta".into(),
                },
            )
            .unwrap();
        let first = rebuild_index(store.connection_for_tests()).unwrap();
        let second = rebuild_index(store.connection_for_tests()).unwrap();
        assert_eq!(first, 2);
        assert_eq!(first, second);
        assert_eq!(store.resume(&a).unwrap().turns.len(), 1);
        assert_eq!(store.resume(&b).unwrap().turns.len(), 1);
    }

    #[test]
    fn missing_index_is_not_silently_rebuilt() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("state.db");
        let store = SessionStore::open(&db).unwrap();
        store
            .connection_for_tests()
            .execute_batch("DROP TABLE message_search")
            .unwrap();
        assert!(matches!(
            rebuild_index(store.connection_for_tests()),
            Err(SearchError::IndexNotReady)
        ));
    }
}

#[cfg(test)]
mod repair_tests {
    use super::*;
    use crate::{conversation::Turn, session::SessionStore};
    use std::time::Instant;
    use tempfile::tempdir;

    #[test]
    fn missing_index_is_repaired_without_canonical_mutation() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("state.db");
        let mut store = SessionStore::open(&db).unwrap();
        let id = store.create_session("test").unwrap();
        store
            .save_turn(
                &id,
                &Turn::User {
                    content: "repair me".into(),
                },
            )
            .unwrap();
        store
            .connection_for_tests()
            .execute_batch("DROP TABLE message_search")
            .unwrap();
        let before = store.resume(&id).unwrap().turns;
        assert_eq!(repair_index(store.connection_for_tests()).unwrap(), 1);
        assert_eq!(store.resume(&id).unwrap().turns, before);
    }
    #[test]
    fn wrong_index_schema_is_classified_as_corrupt() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("state.db");
        let store = SessionStore::open(&db).unwrap();
        store.connection_for_tests().execute_batch("DROP TABLE message_search; CREATE VIRTUAL TABLE message_search USING fts5(wrong, content='messages', content_rowid='id');").unwrap();
        assert!(matches!(
            repair_index(store.connection_for_tests()),
            Err(SearchError::IndexCorrupt { .. })
        ));
        assert!(matches!(
            rebuild_index(store.connection_for_tests()),
            Err(SearchError::IndexCorrupt { .. })
        ));
    }
    #[test]
    fn rebuild_large_dataset_is_bounded_and_complete() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("state.db");
        let store = SessionStore::open(&db).unwrap();
        let id = store.create_session("load-test").unwrap();
        let mut inserts = String::from("BEGIN;");
        for n in 0..5_000 {
            inserts.push_str(&format!("INSERT INTO messages(session_id, role, content, timestamp) VALUES ('{}', 'user', 'search fixture message {}', {});", id, n, n));
        }
        inserts.push_str("COMMIT;");
        store
            .connection_for_tests()
            .execute_batch(&inserts)
            .unwrap();
        let started = Instant::now();
        assert_eq!(rebuild_index(store.connection_for_tests()).unwrap(), 5_000);
        assert!(started.elapsed().as_secs() < 15);
    }
}
