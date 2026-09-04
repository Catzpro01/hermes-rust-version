use super::{ConfigError, HermesConfig};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

pub type HermesHome = PathBuf;

/// Resolves an existing Hermes home. Explicit path > HERMES_HOME > ~/.hermes.
pub fn resolve_hermes_home(explicit: Option<&Path>) -> Result<HermesHome, ConfigError> {
    let candidate = explicit
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HERMES_HOME")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        })
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".hermes")))
        .unwrap_or_else(|| PathBuf::from(".hermes"));
    let candidate = candidate.expand_user();
    if candidate.is_dir() {
        Ok(candidate)
    } else {
        Err(ConfigError::HomeNotFound { path: candidate })
    }
}

/// Loads `config.yaml` without writing to Hermes home.
pub fn load_config(home: &Path) -> Result<HermesConfig, ConfigError> {
    let path = home.join("config.yaml");
    if !path.is_file() {
        return Err(ConfigError::ConfigFileMissing { path });
    }
    let text = fs::read_to_string(&path).map_err(|source| ConfigError::ReadFailed {
        path: path.clone(),
        source,
    })?;
    serde_yaml::from_str(&text).map_err(|source| ConfigError::ParseFailed { path, source })
}

/// Loads config and applies an explicit provider override without writing the file.
pub fn load_config_with_provider(
    home: &Path,
    provider: Option<&str>,
) -> Result<HermesConfig, ConfigError> {
    let mut config = load_config(home)?;
    if let Some(provider) = provider {
        let provider = provider.trim();
        if provider.is_empty() {
            return Err(ConfigError::InvalidOverride {
                field: "provider".into(),
                reason: "value cannot be empty".into(),
            });
        }
        config.model.provider = Some(provider.to_owned());
    }
    Ok(config)
}

trait ExpandUser {
    fn expand_user(self) -> PathBuf;
}
impl ExpandUser for PathBuf {
    fn expand_user(self) -> PathBuf {
        if self == Path::new("~") {
            return env::var_os("HOME").map(PathBuf::from).unwrap_or(self);
        }
        if let Ok(rest) = self.strip_prefix("~/") {
            if let Some(home) = env::var_os("HOME") {
                return PathBuf::from(home).join(rest);
            }
        }
        self
    }
}
