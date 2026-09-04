use super::{SearchError, SearchState};
use crate::session::SessionId;
use rusqlite::{params, Connection};

#[derive(Debug, Clone, Copy)]
pub struct SearchLimits {
    pub max_query_bytes: usize,
    pub max_results: usize,
    pub max_snippet_bytes: usize,
}
impl Default for SearchLimits {
    fn default() -> Self {
        Self {
            max_query_bytes: 4 * 1024,
            max_results: 50,
            max_snippet_bytes: 4 * 1024,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub session_id: SessionId,
    pub message_id: i64,
    pub role: String,
    pub snippet: String,
    pub rank: f64,
}

pub fn escape_fts5_literal(input: &str) -> String {
    format!("\"{}\"", input.replace('"', "\"\""))
}

pub fn search_messages(
    conn: &Connection,
    state: &SearchState,
    query: &str,
    session: Option<&SessionId>,
    limits: SearchLimits,
) -> Result<Vec<SearchResult>, SearchError> {
    match state {
        SearchState::Ready => {}
        SearchState::Unavailable => {
            return Err(SearchError::Unavailable("FTS5 is unavailable".into()))
        }
        SearchState::Corrupt(reason) => {
            return Err(SearchError::Unavailable(format!("index corrupt: {reason}")))
        }
        SearchState::NotReady => return Err(SearchError::IndexNotReady),
    }
    if query.len() > limits.max_query_bytes {
        return Err(SearchError::QueryTooLong {
            len: query.len(),
            max: limits.max_query_bytes,
        });
    }
    if limits.max_results == 0 || query.is_empty() {
        return Ok(Vec::new());
    }
    let literal = escape_fts5_literal(query);
    let session_filter = session.map(SessionId::to_string);
    let mut stmt = conn.prepare("SELECT m.session_id, m.id, m.role, substr(COALESCE(m.content, ''), 1, ?1), bm25(message_search) FROM message_search JOIN messages AS m ON m.id = message_search.rowid WHERE message_search MATCH ?2 AND (?3 IS NULL OR m.session_id = ?3) ORDER BY bm25(message_search), m.id LIMIT ?4").map_err(SearchError::QueryFailed)?;
    let rows = stmt
        .query_map(
            params![
                limits.max_snippet_bytes as i64,
                literal,
                session_filter,
                limits.max_results as i64
            ],
            |row| {
                let raw: String = row.get(0)?;
                let id: i64 = row.get(1)?;
                let role: String = row.get(2)?;
                let snippet: String = row.get(3)?;
                let rank: f64 = row.get(4)?;
                let session_id = raw.parse().map_err(|_| rusqlite::Error::InvalidQuery)?;
                Ok(SearchResult {
                    session_id,
                    message_id: id,
                    role,
                    snippet,
                    rank,
                })
            },
        )
        .map_err(SearchError::QueryFailed)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(SearchError::QueryFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{conversation::Turn, search::rebuild_index, session::SessionStore};
    use tempfile::tempdir;
    fn setup() -> (tempfile::TempDir, SessionStore, SessionId) {
        let d = tempdir().unwrap();
        let mut s = SessionStore::open(&d.path().join("state.db")).unwrap();
        let id = s.create_session("test").unwrap();
        s.save_turn(
            &id,
            &Turn::User {
                content: "hello alpha deploy".into(),
            },
        )
        .unwrap();
        s.save_turn(
            &id,
            &Turn::Assistant {
                content: "safe result".into(),
            },
        )
        .unwrap();
        rebuild_index(s.connection_for_tests()).unwrap();
        (d, s, id)
    }
    #[test]
    fn finds_literal_message() {
        let (_d, s, _id) = setup();
        let r = search_messages(
            s.connection_for_tests(),
            s.search_state(),
            "alpha",
            None,
            SearchLimits::default(),
        )
        .unwrap();
        assert_eq!(r.len(), 1);
    }
    #[test]
    fn preserves_literal_quotes() {
        assert_eq!(escape_fts5_literal("a\"b"), "\"a\"\"b\"");
    }
    #[test]
    fn sql_injection_is_data() {
        let (_d, s, _id) = setup();
        for q in ["' OR 1=1 --", "x); DROP TABLE messages;--", "\" OR \""] {
            assert!(search_messages(
                s.connection_for_tests(),
                s.search_state(),
                q,
                None,
                SearchLimits::default()
            )
            .is_ok());
        }
        assert!(s.resume(&_id).is_ok());
    }
    #[test]
    fn fts_operators_are_literal() {
        let (_d, s, _id) = setup();
        for q in ["alpha OR beta", "NEAR(alpha beta)", "*"] {
            assert!(search_messages(
                s.connection_for_tests(),
                s.search_state(),
                q,
                None,
                SearchLimits::default()
            )
            .is_ok());
        }
    }
    #[test]
    fn shell_and_tool_payloads_are_not_executed() {
        let (_d, s, _id) = setup();
        for q in ["rm -rf /", "<tool_call>", "curl https://evil.example"] {
            assert!(search_messages(
                s.connection_for_tests(),
                s.search_state(),
                q,
                None,
                SearchLimits::default()
            )
            .is_ok());
        }
    }
    #[test]
    fn query_limit_enforced() {
        let (_d, s, _id) = setup();
        let l = SearchLimits {
            max_query_bytes: 3,
            ..Default::default()
        };
        assert!(matches!(
            search_messages(s.connection_for_tests(), s.search_state(), "abcd", None, l),
            Err(SearchError::QueryTooLong { .. })
        ));
    }
    #[test]
    fn result_limit_enforced() {
        let (_d, s, id) = setup();
        let l = SearchLimits {
            max_results: 1,
            ..Default::default()
        };
        assert!(
            search_messages(
                s.connection_for_tests(),
                s.search_state(),
                "hello",
                Some(&id),
                l
            )
            .unwrap()
            .len()
                <= 1
        );
    }
    #[test]
    fn snippet_limit_bound_is_bound() {
        let (_d, s, _id) = setup();
        let l = SearchLimits {
            max_snippet_bytes: 2,
            ..Default::default()
        };
        let r =
            search_messages(s.connection_for_tests(), s.search_state(), "hello", None, l).unwrap();
        assert!(r[0].snippet.len() <= 2);
    }
    #[test]
    fn session_filter_isolated() {
        let (_d, mut s, id) = setup();
        let other = s.create_session("other").unwrap();
        s.save_turn(
            &other,
            &Turn::User {
                content: "hello alpha".into(),
            },
        )
        .unwrap();
        rebuild_index(s.connection_for_tests()).unwrap();
        let r = search_messages(
            s.connection_for_tests(),
            s.search_state(),
            "alpha",
            Some(&id),
            Default::default(),
        )
        .unwrap();
        assert!(r.iter().all(|x| x.session_id == id));
    }
    #[test]
    fn empty_query_is_empty() {
        let (_d, s, _id) = setup();
        assert!(search_messages(
            s.connection_for_tests(),
            s.search_state(),
            "",
            None,
            Default::default()
        )
        .unwrap()
        .is_empty());
    }
    #[test]
    fn state_gate_blocks_non_ready() {
        let (_d, s, _id) = setup();
        assert!(matches!(
            search_messages(
                s.connection_for_tests(),
                &SearchState::NotReady,
                "hello",
                None,
                Default::default()
            ),
            Err(SearchError::IndexNotReady)
        ));
        assert!(matches!(
            search_messages(
                s.connection_for_tests(),
                &SearchState::Unavailable,
                "hello",
                None,
                Default::default()
            ),
            Err(SearchError::Unavailable(_))
        ));
    }
    #[test]
    fn no_canonical_write_from_query() {
        let (_d, s, id) = setup();
        let before = s.resume(&id).unwrap().turns;
        let _ = search_messages(
            s.connection_for_tests(),
            s.search_state(),
            "hello",
            None,
            Default::default(),
        )
        .unwrap();
        assert_eq!(before, s.resume(&id).unwrap().turns);
    }
}
