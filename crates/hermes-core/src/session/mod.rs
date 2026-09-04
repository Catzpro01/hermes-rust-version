//! SQLite-backed session persistence compatible with Hermes `state.db`.

mod id;
mod store;
pub use id::SessionId;
pub use store::{Session, SessionStore, SessionStoreError};
