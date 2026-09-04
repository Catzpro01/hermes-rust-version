use super::{check_fts5_available, SearchError};
use rusqlite::{params, Connection};

pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub up: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "create_fts5_message_search",
        up: r#"CREATE VIRTUAL TABLE IF NOT EXISTS message_search USING fts5(
            content,
            role,
            session_id UNINDEXED,
            message_id UNINDEXED,
            content='messages',
            content_rowid='id'
        );"#,
    },
    Migration {
        version: 2,
        name: "create_search_meta",
        up: r#"CREATE TABLE IF NOT EXISTS search_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        INSERT OR REPLACE INTO search_meta(key, value) VALUES ('fts_version', '1');"#,
    },
];

pub fn run_migrations(conn: &mut Connection) -> Result<(), SearchError> {
    if !check_fts5_available(conn)? {
        return Err(SearchError::Fts5Unavailable);
    }
    conn.execute_batch("CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at DATETIME DEFAULT CURRENT_TIMESTAMP);")
        .map_err(SearchError::MigrationFailed)?;
    let current: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )
        .map_err(SearchError::MigrationFailed)?;
    for migration in MIGRATIONS.iter().filter(|m| m.version > current) {
        let tx = conn.transaction().map_err(SearchError::MigrationFailed)?;
        tx.execute_batch(migration.up)
            .map_err(SearchError::MigrationFailed)?;
        tx.execute(
            "INSERT INTO schema_migrations(version, name) VALUES (?1, ?2)",
            params![migration.version, migration.name],
        )
        .map_err(SearchError::MigrationFailed)?;
        tx.commit().map_err(SearchError::MigrationFailed)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn canonical(conn: &Connection) -> Vec<String> {
        [
            "SELECT id, source, started_at FROM sessions ORDER BY id",
            "SELECT id, session_id, role, content, timestamp FROM messages ORDER BY id",
        ]
        .iter()
        .flat_map(|sql| {
            let mut s = conn.prepare(sql).unwrap();
            s.query_map([], |r| {
                Ok(format!(
                    "{:?}|{:?}|{:?}|{:?}|{:?}",
                    r.get::<_, String>(0),
                    r.get::<_, Option<String>>(1),
                    r.get::<_, Option<String>>(2),
                    r.get::<_, Option<String>>(3),
                    r.get::<_, Option<f64>>(4)
                ))
            })
            .unwrap()
            .map(|r| r.unwrap())
            .collect::<Vec<_>>()
        })
        .collect()
    }
    #[test]
    fn migration_fresh_and_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE sessions(id TEXT PRIMARY KEY, source TEXT NOT NULL, started_at REAL NOT NULL); CREATE TABLE messages(id INTEGER PRIMARY KEY, session_id TEXT NOT NULL, role TEXT NOT NULL, content TEXT, timestamp REAL NOT NULL); CREATE TABLE tool_calls(id TEXT PRIMARY KEY, session_id TEXT NOT NULL, turn_index INTEGER NOT NULL, tool_name TEXT NOT NULL, arguments TEXT NOT NULL, result TEXT, status TEXT NOT NULL, created_at REAL NOT NULL);").unwrap();
        run_migrations(&mut conn).unwrap();
        run_migrations(&mut conn).unwrap();
        assert_eq!(
            conn.query_row("SELECT MAX(version) FROM schema_migrations", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
    }
    #[test]
    fn migration_preserves_canonical_rows() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE sessions(id TEXT PRIMARY KEY, source TEXT NOT NULL, started_at REAL NOT NULL); CREATE TABLE messages(id INTEGER PRIMARY KEY, session_id TEXT NOT NULL, role TEXT NOT NULL, content TEXT, timestamp REAL NOT NULL); INSERT INTO sessions VALUES('s','test',1.0); INSERT INTO messages VALUES(1,'s','user','hello',2.0);").unwrap();
        let before = canonical(&conn);
        run_migrations(&mut conn).unwrap();
        assert_eq!(before, canonical(&conn));
    }
}
