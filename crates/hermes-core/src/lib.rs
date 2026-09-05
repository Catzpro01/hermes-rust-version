//! Hermes-RS core: provider-neutral agent domain and compatibility boundary.

pub mod config;
pub mod conversation;
pub mod provider;
pub mod search;
pub mod session;
pub mod tools;
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::config::*;
    use std::{fs, path::Path};
    use tempfile::tempdir;

    #[test]
    fn resolves_explicit_existing_home() {
        let d = tempdir().unwrap();
        assert_eq!(resolve_hermes_home(Some(d.path())).unwrap(), d.path());
    }
    #[test]
    fn returns_error_when_home_missing() {
        let e =
            resolve_hermes_home(Some(Path::new("/definitely/missing/hermes-home"))).unwrap_err();
        assert!(matches!(e, ConfigError::HomeNotFound { .. }));
    }
    #[test]
    fn parses_valid_hermes_yaml_read_only() {
        let d = tempdir().unwrap();
        fs::write(d.path().join("config.yaml"), "model:\n  default: gpt-test\n  provider: custom\n  base_url: http://localhost:8080/v1\n  api_key: secret-value\n").unwrap();
        let c = load_config(d.path()).unwrap();
        assert_eq!(c.model.default.as_deref(), Some("gpt-test"));
        assert_eq!(c.model.provider.as_deref(), Some("custom"));
        assert_eq!(c.model.api_key.as_ref().unwrap().expose(), "secret-value");
        assert_eq!(format!("{:?}", c.model), "ModelConfig { default: Some(\"gpt-test\"), provider: Some(\"custom\"), base_url: Some(\"http://localhost:8080/v1\"), api_key: Some(SecretString(***REDACTED***)) }");
        assert!(!d.path().join("config.yaml.tmp").exists());
    }
    #[test]
    fn applies_explicit_provider_override_without_writing() {
        let d = tempdir().unwrap();
        let path = d.path().join("config.yaml");
        fs::write(&path, "model:\n  provider: auto\n").unwrap();
        let c = load_config_with_provider(d.path(), Some("openai")).unwrap();
        assert_eq!(c.model.provider.as_deref(), Some("openai"));
        assert_eq!(
            fs::read_to_string(path).unwrap(),
            "model:\n  provider: auto\n"
        );
    }

    #[test]
    fn rejects_invalid_yaml() {
        let d = tempdir().unwrap();
        fs::write(d.path().join("config.yaml"), "model: [broken").unwrap();
        assert!(matches!(
            load_config(d.path()),
            Err(ConfigError::ParseFailed { .. })
        ));
    }
    #[test]
    fn reports_missing_config() {
        let d = tempdir().unwrap();
        assert!(matches!(
            load_config(d.path()),
            Err(ConfigError::ConfigFileMissing { .. })
        ));
    }
    #[test]
    fn rejects_mcp_server_with_empty_command_at_load() {
        let d = tempdir().unwrap();
        fs::write(
            d.path().join("config.yaml"),
            "mcp_servers:\n  github:\n    command: \"\"\n",
        )
        .unwrap();
        assert!(matches!(
            load_config(d.path()),
            Err(ConfigError::McpServerInvalid { server, .. }) if server == "github"
        ));
    }
    #[test]
    fn empty_or_valid_mcp_servers_load_fine() {
        let d = tempdir().unwrap();
        fs::write(
            d.path().join("config.yaml"),
            "mcp_servers:\n  gh:\n    command: npx\n    args: [\"-y\", \"pkg\"]\n    env:\n      TOK: \"abc\"\n",
        )
        .unwrap();
        let c = load_config(d.path()).unwrap();
        assert_eq!(c.mcp_servers["gh"].command, "npx");
    }
}
