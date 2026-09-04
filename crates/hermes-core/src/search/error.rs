use thiserror::Error;

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
    #[error("search unavailable: {0}")]
    Unavailable(String),
    #[error("search query failed: {0}")]
    QueryFailed(#[source] rusqlite::Error),
    #[error("query too long: {len} bytes (max {max})")]
    QueryTooLong { len: usize, max: usize },
    #[error("invalid session filter")]
    InvalidSession,
}
