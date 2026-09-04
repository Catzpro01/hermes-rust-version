//! Read-only compatibility access to Hermes home and `config.yaml`.

mod error;
mod home;
mod schema;

pub use error::ConfigError;
pub use home::{load_config, resolve_hermes_home, HermesHome};
pub use schema::{HermesConfig, ModelConfig, SecretString};
