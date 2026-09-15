//! SQLite-backed session persistence compatible with Hermes `state.db`.

mod id;
mod store;
pub use id::SessionId;
pub use store::{classify_session_status, Session, SessionStatus, SessionStore, SessionStoreError};
