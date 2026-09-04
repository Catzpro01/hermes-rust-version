use super::SessionId;
use crate::conversation::Turn;
use rusqlite::{params, Connection};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionStoreError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("session not found: {0}")]
    NotFound(SessionId),
}
#[derive(Debug, Clone)]
pub struct Session {
    pub id: SessionId,
    pub source: String,
    pub started_at: f64,
    pub turns: Vec<Turn>,
}

pub struct SessionStore {
    conn: Connection,
}
impl SessionStore {
    pub fn open(path: &Path) -> Result<Self, SessionStoreError> {
        let conn = Connection::open(path)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, source TEXT NOT NULL, started_at REAL NOT NULL); CREATE TABLE IF NOT EXISTS messages (id INTEGER PRIMARY KEY AUTOINCREMENT, session_id TEXT NOT NULL REFERENCES sessions(id), role TEXT NOT NULL, content TEXT, timestamp REAL NOT NULL);")?;
        Ok(Self { conn })
    }
    pub fn create_session(&self, source: &str) -> Result<SessionId, SessionStoreError> {
        let id = SessionId::new();
        self.conn.execute(
            "INSERT INTO sessions (id, source, started_at) VALUES (?1, ?2, ?3)",
            params![id.to_string(), source, now()],
        )?;
        Ok(id)
    }
    pub fn save_turn(&mut self, id: &SessionId, turn: &Turn) -> Result<(), SessionStoreError> {
        let tx = self.conn.transaction()?;
        let (role, content) = match turn {
            Turn::User { content } => ("user", content),
            Turn::Assistant { content } => ("assistant", content),
            Turn::Tool { name, content } => (name.as_str(), content),
        };
        tx.execute(
            "INSERT INTO messages (session_id, role, content, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![id.to_string(), role, content, now()],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn resume(&self, id: &SessionId) -> Result<Session, SessionStoreError> {
        let mut s = self
            .conn
            .prepare("SELECT source, started_at FROM sessions WHERE id=?1")?;
        let row = s
            .query_row(params![id.to_string()], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
            })
            .map_err(|e| {
                if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                    SessionStoreError::NotFound(*id)
                } else {
                    SessionStoreError::Sqlite(e)
                }
            })?;
        let mut q = self
            .conn
            .prepare("SELECT role, content FROM messages WHERE session_id=?1 ORDER BY id")?;
        let turns = q
            .query_map(params![id.to_string()], |r| {
                let role: String = r.get(0)?;
                let content: String = r.get::<_, Option<String>>(1)?.unwrap_or_default();
                Ok(match role.as_str() {
                    "user" => Turn::User { content },
                    "assistant" => Turn::Assistant { content },
                    name => Turn::Tool {
                        name: name.into(),
                        content,
                    },
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Session {
            id: *id,
            source: row.0,
            started_at: row.1,
            turns,
        })
    }
    pub fn list(&self) -> Result<Vec<SessionId>, SessionStoreError> {
        let mut q = self
            .conn
            .prepare("SELECT id FROM sessions ORDER BY started_at DESC")?;
        let rows = q.query_map([], |r| r.get::<_, String>(0))?;
        let mut ids = Vec::new();
        for row in rows {
            let raw = row?;
            ids.push(raw.parse().map_err(|_| rusqlite::Error::InvalidQuery)?);
        }
        Ok(ids)
    }
}
fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
