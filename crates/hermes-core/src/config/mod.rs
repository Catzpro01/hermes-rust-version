//! Read-only compatibility access to Hermes home and `config.yaml`.

mod error;
mod home;
mod schema;

pub use error::ConfigError;
pub use home::{load_config, load_config_with_provider, resolve_hermes_home, HermesHome};
pub use schema::{ApiMode, CompressionConfig, HermesConfig, ModelConfig, ProviderConfig, SecretString};
