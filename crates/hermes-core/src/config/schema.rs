use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct SecretString(String);
impl SecretString {
    pub fn expose(&self) -> &str {
        &self.0
    }
}
impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString(***REDACTED***)")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HermesConfig {
    #[serde(default)]
    pub model: ModelConfig,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct ModelConfig {
    pub default: Option<String>,
    pub provider: Option<String>,
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<SecretString>,
}
impl fmt::Debug for ModelConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModelConfig")
            .field("default", &self.default)
            .field("provider", &self.provider)
            .field("base_url", &self.base_url)
            .field("api_key", &self.api_key)
            .finish()
    }
}
