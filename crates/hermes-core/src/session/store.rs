use super::SessionId;
use crate::conversation::Turn;
use crate::tools::{ToolCallRecord, ToolExecutionStatus};
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
pub struct ToolCallDetail {
    pub id: String,
    pub tool_name: String,
    pub arguments: String,
    pub result: String,
    pub status: String,
}

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
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, source TEXT NOT NULL, started_at REAL NOT NULL); CREATE TABLE IF NOT EXISTS messages (id INTEGER PRIMARY KEY AUTOINCREMENT, session_id TEXT NOT NULL REFERENCES sessions(id), role TEXT NOT NULL, content TEXT, timestamp REAL NOT NULL); CREATE TABLE IF NOT EXISTS tool_calls (id TEXT PRIMARY KEY, session_id TEXT NOT NULL REFERENCES sessions(id), turn_index INTEGER NOT NULL, tool_name TEXT NOT NULL, arguments TEXT NOT NULL, result TEXT, status TEXT NOT NULL CHECK(status IN ('success','error','denied','timeout','cancelled')), created_at REAL NOT NULL);")?;
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
    pub fn save_tool_call(&self, record: &ToolCallRecord) -> Result<(), SessionStoreError> {
        self.conn.execute("INSERT OR REPLACE INTO tool_calls (id,session_id,turn_index,tool_name,arguments,result,status,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)", params![record.id, record.session_id, record.turn_index as i64, record.tool_name, record.arguments, record.result, record.status.as_str(), now()])?;
        Ok(())
    }
    pub fn list_tool_call_details(
        &self,
        id: &SessionId,
    ) -> Result<Vec<ToolCallDetail>, SessionStoreError> {
        let mut q=self.conn.prepare("SELECT id,tool_name,arguments,COALESCE(result,''),status FROM tool_calls WHERE session_id=?1 ORDER BY turn_index")?;
        let rows = q.query_map(params![id.to_string()], |r| {
            Ok(ToolCallDetail {
                id: r.get(0)?,
                tool_name: r.get(1)?,
                arguments: r.get(2)?,
                result: r.get(3)?,
                status: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn list_tool_calls(
        &self,
        id: &SessionId,
    ) -> Result<Vec<(String, ToolExecutionStatus)>, SessionStoreError> {
        let mut q = self
            .conn
            .prepare("SELECT id,status FROM tool_calls WHERE session_id=?1 ORDER BY turn_index")?;
        let rows = q.query_map(params![id.to_string()], |r| {
            let status: String = r.get(1)?;
            let s = match status.as_str() {
                "success" => ToolExecutionStatus::Success,
                "error" => ToolExecutionStatus::Error,
                "denied" => ToolExecutionStatus::Denied,
                "timeout" => ToolExecutionStatus::Timeout,
                "cancelled" => ToolExecutionStatus::Cancelled,
                _ => return Err(rusqlite::Error::InvalidQuery),
            };
            Ok((r.get(0)?, s))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
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
