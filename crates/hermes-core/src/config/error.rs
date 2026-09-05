use std::{io, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Hermes home not found: {path}. Set HERMES_HOME or provide an explicit home path")]
    HomeNotFound { path: PathBuf },
    #[error("config file not found at {path}")]
    ConfigFileMissing { path: PathBuf },
    #[error("failed to read config file {path}: {source}")]
    ReadFailed {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("invalid config format at {path}: {source}")]
    ParseFailed {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("provider '{name}' is not configured")]
    ProviderNotConfigured { name: String },
    #[error("invalid override for field '{field}': {reason}")]
    InvalidOverride { field: String, reason: String },
    #[error("invalid MCP server '{server}': {reason}")]
    McpServerInvalid { server: String, reason: String },
}
