use rusqlite::Connection;
use thiserror::Error;

pub mod migration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchState {
    Ready,
    Unavailable,
    Corrupt(String),
}

#[derive(Debug, Error)]
pub enum SearchError {
    #[error("FTS5 is not available in this SQLite build")]
    Fts5Unavailable,
    #[error("migration failed: {0}")]
    MigrationFailed(#[source] rusqlite::Error),
    #[error("index rebuild failed: {0}")]
    RebuildFailed(#[source] rusqlite::Error),
    #[error("index not ready: run rebuild first")]
    IndexNotReady,
    #[error("index corrupt: {reason}")]
    IndexCorrupt { reason: String },
}

/// Checks the compiled SQLite feature without changing canonical tables.
pub fn check_fts5_available(conn: &Connection) -> Result<bool, SearchError> {
    let result = conn.execute_batch(
        "CREATE VIRTUAL TABLE temp.fts5_availability_probe USING fts5(value); DROP TABLE temp.fts5_availability_probe;",
    );
    Ok(result.is_ok())
}

pub fn rebuild_index(conn: &Connection) -> Result<usize, SearchError> {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='message_search')",
            [],
            |r| r.get(0),
        )
        .map_err(SearchError::RebuildFailed)?;
    if !exists {
        return Err(SearchError::IndexNotReady);
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
