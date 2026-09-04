//! Hermes-RS core: provider-neutral agent domain and compatibility boundary.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub hermes_home: String,
    pub default_provider: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            hermes_home: "~/.hermes".into(),
            default_provider: None,
        }
    }
}

/// Provider-neutral message model. Provider adapters will map to/from this type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
}
