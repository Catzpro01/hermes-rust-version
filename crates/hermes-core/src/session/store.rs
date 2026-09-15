use super::SessionId;
use crate::conversation::Turn;
use crate::tools::{ToolCallRecord, ToolExecutionStatus};
use rusqlite::{params, params_from_iter, Connection};
use std::{
    collections::HashMap,
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

#[derive(Debug, Clone)]
pub struct SessionDetails {
    pub id: SessionId,
    pub source: String,
    pub started_at: f64,
    pub message_count: usize,
    pub tool_call_count: usize,
}
#[derive(Debug, Clone)]
pub struct MessageDetails {
    pub sequence: usize,
    pub role: String,
    pub content: String,
    pub timestamp: f64,
}

/// Lifecycle status of a session, as the pinned reference derives it
/// (`hermes_state.classify_session_status`, retained verbatim at
/// `docs/hermes-ui-spec/017/evidence/upstream-lifecycle-status/`).
///
/// It is derived from a session's **last message row only** — never from a
/// transcript scan — which is the same O(1)-per-session property the reference
/// buys with its `MAX(id)` join.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    /// The last row's `finish_reason` is an error. The reference checks this
    /// *before* the role, so it wins over every other branch.
    Error,
    /// The last row is a turn that never got an answer: a user row, a
    /// tool-result row, or an assistant row whose tool call has no result row.
    /// ADR 0007: a tool-result row is any row whose role is not `user`,
    /// `assistant` or `system`, because this crate stores those under the
    /// tool's own name rather than under a `tool` role.
    Interrupted,
    /// The last row is a speaker that got its answer: an assistant reply with
    /// nothing pending, or a system row. This is a narrower set than the
    /// reference's benign default — see ADR 0007.
    Complete,
    /// The session has no message row at all.
    Empty,
    /// The status could not be derived because the one grouped query failed.
    /// The reference swallows that error (`_annotate_session_statuses`) and
    /// `_session_status_tag` then renders `-`, so a status column that cannot
    /// be filled never takes the picker down with it.
    Unknown,
}

impl SessionStatus {
    /// The word the picker's five-cell `Stat` column renders, i.e.
    /// `_session_status_tag` in the pinned upstream source.
    pub fn tag(self) -> &'static str {
        match self {
            SessionStatus::Error => "err",
            SessionStatus::Interrupted => "intr",
            SessionStatus::Complete => "done",
            SessionStatus::Empty => "empty",
            SessionStatus::Unknown => "-",
        }
    }
}

/// `finish_reason` values the reference treats as an error
/// (`_ERROR_FINISH_REASONS` in the pinned source).
const ERROR_FINISH_REASONS: [&str; 3] = ["error", "agent_error", "content_filter"];

/// Classify one session from its last message row.
///
/// `tool_calls` and `finish_reason` exist only in databases written by Hermes
/// Python; a database this crate creates has neither column (see the schema in
/// [`SessionStore::open`]). Both are optional here so that one classifier
/// serves both shapes: with the columns absent only the role rules apply and
/// `err` therefore cannot arise — which is equally true of the reference,
/// since it has no other source for it either.
///
/// The order of the checks is the reference's: an error `finish_reason` wins
/// over the role, then the role decides. The role rule itself departs from the
/// reference on one point, recorded in ADR 0007: where the reference treats an
/// unrecognised role as `complete`, this classifier treats it as a
/// tool-result row and reports `intr`.
pub fn classify_session_status(
    role: &str,
    tool_calls: Option<&str>,
    finish_reason: Option<&str>,
) -> SessionStatus {
    if let Some(reason) = finish_reason {
        let reason = reason.trim().to_ascii_lowercase();
        if ERROR_FINISH_REASONS.iter().any(|r| *r == reason) {
            return SessionStatus::Error;
        }
    }
    match role {
        // ADR 0007: the reference lists `user` and `tool` as the interrupted
        // roles, but this crate stores a tool result under the tool's own name
        // (`Turn::Tool { name, .. }` -> `role = name`), so `tool` is only one
        // spelling of a tool-result row. Anything that is not a speaker is one.
        "assistant" if tool_calls.is_some() => SessionStatus::Interrupted,
        "assistant" | "system" => SessionStatus::Complete,
        _ => SessionStatus::Interrupted,
    }
}

pub struct Session {
    pub id: SessionId,
    pub source: String,
    pub started_at: f64,
    pub turns: Vec<Turn>,
}

pub struct SessionStore {
    conn: Connection,
    pub(crate) search_state: crate::search::SearchState,
}

#[cfg(test)]
impl SessionStore {
    pub(crate) fn connection_for_tests(&self) -> &Connection {
        &self.conn
    }
}
impl SessionStore {
    pub fn repair_search_index(&self) -> Result<usize, crate::search::SearchError> {
        crate::search::repair_index(&self.conn)
    }

    pub fn search_messages(
        &self,
        query: &str,
        session: Option<&SessionId>,
        limits: crate::search::SearchLimits,
    ) -> Result<Vec<crate::search::SearchResult>, crate::search::SearchError> {
        crate::search::search_messages(&self.conn, &self.search_state, query, session, limits)
    }

    pub fn search_state(&self) -> &crate::search::SearchState {
        &self.search_state
    }

    pub fn open(path: &Path) -> Result<Self, SessionStoreError> {
        let mut conn = Connection::open(path)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, source TEXT NOT NULL, started_at REAL NOT NULL); CREATE TABLE IF NOT EXISTS messages (id INTEGER PRIMARY KEY AUTOINCREMENT, session_id TEXT NOT NULL REFERENCES sessions(id), role TEXT NOT NULL, content TEXT, timestamp REAL NOT NULL); CREATE TABLE IF NOT EXISTS tool_calls (id TEXT PRIMARY KEY, session_id TEXT NOT NULL REFERENCES sessions(id), turn_index INTEGER NOT NULL, tool_name TEXT NOT NULL, arguments TEXT NOT NULL, result TEXT, status TEXT NOT NULL CHECK(status IN ('success','error','denied','timeout','cancelled')), created_at REAL NOT NULL);")?;
        let search_state = match crate::search::migration::run_migrations(&mut conn) {
            Ok(()) => crate::search::SearchState::Ready,
            Err(crate::search::SearchError::Fts5Unavailable) => {
                tracing::warn!("FTS5 not available; search disabled");
                crate::search::SearchState::Unavailable
            }
            Err(crate::search::SearchError::MigrationFailed(error)) => {
                tracing::error!(error = %error, "FTS5 migration failed; search disabled");
                crate::search::SearchState::Corrupt(error.to_string())
            }
            Err(error) => {
                tracing::error!(error = %error, "FTS5 search disabled");
                crate::search::SearchState::Unavailable
            }
        };
        Ok(Self { conn, search_state })
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
        // Begin IMMEDIATE (not the rusqlite default DEFERRED) so the write lock
        // is taken up-front rather than lazily on the first INSERT. With a
        // DEFERRED transaction a second concurrent connection can first take a
        // SHARED read lock and then try to upgrade to RESERVED, which deadlocks
        // against the first writer holding RESERVED but unable to COMMIT while
        // a reader holds SHARED — SQLite breaks that with an untreatable
        // SQLITE_BUSY that `busy_timeout` cannot resolve. IMMEDIATE removes the
        // read-lock holder entirely, so concurrent writers are cleanly
        // serialized by `busy_timeout`. See Spec 006 #06.
        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
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
        if !self.session_exists(id)? {
            return Err(SessionStoreError::NotFound(*id));
        }
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
        if !self.session_exists(id)? {
            return Err(SessionStoreError::NotFound(*id));
        }
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
    pub fn session_details(&self, id: &SessionId) -> Result<SessionDetails, SessionStoreError> {
        let session = self.resume(id)?;
        let message_count = session.turns.len();
        let tool_call_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tool_calls WHERE session_id=?1",
            params![id.to_string()],
            |r| r.get(0),
        )?;
        Ok(SessionDetails {
            id: *id,
            source: session.source,
            started_at: session.started_at,
            message_count,
            tool_call_count: tool_call_count as usize,
        })
    }
    pub fn list_messages(&self, id: &SessionId) -> Result<Vec<MessageDetails>, SessionStoreError> {
        if !self.session_exists(id)? {
            return Err(SessionStoreError::NotFound(*id));
        }
        let mut q=self.conn.prepare("SELECT id,role,COALESCE(content,''),timestamp FROM messages WHERE session_id=?1 ORDER BY id")?;
        let rows = q.query_map(params![id.to_string()], |r| {
            Ok(MessageDetails {
                sequence: r.get::<_, i64>(0)? as usize,
                role: r.get(1)?,
                content: r.get(2)?,
                timestamp: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
    fn session_exists(&self, id: &SessionId) -> Result<bool, SessionStoreError> {
        Ok(self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sessions WHERE id=?1)",
            params![id.to_string()],
            |r| r.get(0),
        )?)
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

    /// Lifecycle status of each listed session, keyed by session id.
    ///
    /// One query that resolves every listed session's newest message id with
    /// `MAX(id)` and joins back for that single row — the reference's
    /// `session_lifecycle_statuses`, so no transcript is ever scanned for one
    /// session's status. Every session asked about is present in the map: the
    /// ones with no message row are seeded with [`SessionStatus::Empty`], the
    /// way the reference seeds `{sid: "empty" for sid in ids}`. A session
    /// *absent* from the map is one that was not asked about.
    pub fn lifecycle_statuses(
        &self,
        ids: &[SessionId],
    ) -> Result<HashMap<String, SessionStatus>, SessionStoreError> {
        let mut out: HashMap<String, SessionStatus> = ids
            .iter()
            .map(|id| (id.to_string(), SessionStatus::Empty))
            .collect();
        if ids.is_empty() {
            return Ok(out);
        }
        // A database may carry one of the two columns without the other, so
        // they are probed separately: a partially migrated schema has to stay a
        // normal case rather than become a query error.
        let (tool_calls, finish_reason) = self.lifecycle_column_expressions()?;
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            "SELECT m.session_id, m.role, {tool_calls}, {finish_reason} \
             FROM messages AS m \
             JOIN (SELECT session_id, MAX(id) AS max_id FROM messages \
                   WHERE session_id IN ({placeholders}) GROUP BY session_id) AS l \
             ON l.session_id = m.session_id AND l.max_id = m.id"
        );
        let params = ids.iter().map(|id| id.to_string());
        let mut q = self.conn.prepare(&sql)?;
        let rows = q.query_map(params_from_iter(params), |r| {
            let sid: String = r.get(0)?;
            let role: String = r.get(1)?;
            let calls: Option<String> = r.get(2)?;
            let reason: Option<String> = r.get(3)?;
            Ok((sid, role, calls, reason))
        })?;
        for row in rows {
            let (sid, role, calls, reason) = row?;
            let status = classify_session_status(&role, calls.as_deref(), reason.as_deref());
            out.insert(sid, status);
        }
        Ok(out)
    }

    /// SQL expressions reading `tool_calls` and `finish_reason`, or `NULL` for
    /// whichever column this database does not carry. Both are written only by
    /// Hermes Python; the schema in [`SessionStore::open`] has neither.
    /// `PRAGMA table_info` returns `cid, name, type, ...`, so the name is the
    /// second column.
    fn lifecycle_column_expressions(
        &self,
    ) -> Result<(&'static str, &'static str), SessionStoreError> {
        let mut q = self.conn.prepare("PRAGMA table_info(messages)")?;
        let columns = q
            .query_map([], |r| r.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?;
        let has = |name: &str| columns.iter().any(|column| column == name);
        Ok((
            if has("tool_calls") {
                "m.tool_calls"
            } else {
                "NULL"
            },
            if has("finish_reason") {
                "m.finish_reason"
            } else {
                "NULL"
            },
        ))
    }

    /// Delete a session and all of its messages and tool calls (Spec 017 T09:
    /// the browse picker's `d` key). Children are deleted before the parent
    /// row because `PRAGMA foreign_keys=ON` has no cascading delete.
    /// Returns `false` when the session did not exist (nothing was deleted).
    pub fn delete_session(&self, id: &SessionId) -> Result<bool, SessionStoreError> {
        let raw = id.to_string();
        self.conn
            .execute("DELETE FROM tool_calls WHERE session_id=?1", params![raw])?;
        self.conn
            .execute("DELETE FROM messages WHERE session_id=?1", params![raw])?;
        let removed = self
            .conn
            .execute("DELETE FROM sessions WHERE id=?1", params![raw])?;
        Ok(removed > 0)
    }
}
fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

#[cfg(test)]
mod tests {
    use super::{classify_session_status, SessionStatus, SessionStore, Turn};

    #[test]
    fn error_finish_reason_outranks_every_role() {
        assert_eq!(
            classify_session_status("user", None, Some("error")),
            SessionStatus::Error
        );
        assert_eq!(
            classify_session_status("assistant", None, Some("agent_error")),
            SessionStatus::Error
        );
        assert_eq!(
            classify_session_status("tool", None, Some(" Content_Filter ")),
            SessionStatus::Error
        );
    }

    #[test]
    fn unanswered_turns_are_interrupted() {
        assert_eq!(
            classify_session_status("user", None, None),
            SessionStatus::Interrupted
        );
        assert_eq!(
            classify_session_status("tool", None, None),
            SessionStatus::Interrupted
        );
        assert_eq!(
            classify_session_status("assistant", Some("[{\"id\":\"call-1\"}]"), None),
            SessionStatus::Interrupted
        );
    }

    #[test]
    fn answered_turns_are_complete() {
        assert_eq!(
            classify_session_status("assistant", None, Some("stop")),
            SessionStatus::Complete
        );
        assert_eq!(
            classify_session_status("assistant", None, None),
            SessionStatus::Complete
        );
    }

    #[test]
    fn system_rows_stay_complete() {
        // ADR 0007: `system` is a speaker, so it is not read as a tool row.
        assert_eq!(
            classify_session_status("system", None, Some("length")),
            SessionStatus::Complete
        );
    }

    #[test]
    fn a_tool_result_row_is_interrupted_under_the_tools_own_name() {
        // ADR 0007: this crate stores a tool result under the tool's name, not
        // under a `tool` role, so both spellings must classify the same way.
        assert_eq!(
            classify_session_status("shell", None, None),
            SessionStatus::Interrupted
        );
        assert_eq!(
            classify_session_status("tool", None, None),
            SessionStatus::Interrupted
        );
        // The error check still outranks the role.
        assert_eq!(
            classify_session_status("shell", None, Some("error")),
            SessionStatus::Error
        );
    }

    #[test]
    fn an_unrecognised_role_is_read_as_a_tool_result_row() {
        // ADR 0007: the reference's benign default for an unknown shape is
        // `complete`; this crate deliberately reads it as an unanswered turn
        // instead, so the status column cannot hide an interruption.
        assert_eq!(
            classify_session_status("developer", None, None),
            SessionStatus::Interrupted
        );
        assert_eq!(
            classify_session_status("", None, None),
            SessionStatus::Interrupted
        );
    }

    #[test]
    fn lifecycle_statuses_reads_only_the_sessions_it_was_asked_about() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut store = SessionStore::open(&dir.path().join("state.db")).unwrap();
        let asked = store.create_session("cli").unwrap();
        store
            .save_turn(
                &asked,
                &Turn::User {
                    content: "hi".into(),
                },
            )
            .unwrap();
        let other = store.create_session("cli").unwrap();

        let statuses = store.lifecycle_statuses(&[asked]).unwrap();
        assert_eq!(
            statuses.get(&asked.to_string()),
            Some(&SessionStatus::Interrupted),
            "one user row and no answer -> interrupted"
        );
        assert!(
            !statuses.contains_key(&other.to_string()),
            "a session that was not asked about must not be read"
        );

        // Asked about, but no message row: seeded empty, like the reference.
        let seeded = store.lifecycle_statuses(&[other]).unwrap();
        assert_eq!(seeded.get(&other.to_string()), Some(&SessionStatus::Empty));
    }

    #[test]
    fn tags_are_the_pinned_words() {
        assert_eq!(SessionStatus::Error.tag(), "err");
        assert_eq!(SessionStatus::Interrupted.tag(), "intr");
        assert_eq!(SessionStatus::Complete.tag(), "done");
        assert_eq!(SessionStatus::Empty.tag(), "empty");
        // `_session_status_tag`'s fallback for a status it cannot name.
        assert_eq!(SessionStatus::Unknown.tag(), "-");
    }
}
