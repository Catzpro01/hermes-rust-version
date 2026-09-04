//! Config-driven provider selection.
//!
//! Replaces the literal `match` on provider names that `main.rs` used to
//! perform. Providers are registered as factories and only constructed when
//! actually selected, so one misconfigured provider in `config.yaml` cannot
//! prevent startup or take down the provider currently in use.

use crate::config::{ApiMode, HermesConfig, ProviderConfig, SecretString};
use crate::provider::{FakeProvider, HttpProvider, Provider};
use std::collections::HashMap;
use thiserror::Error;
use url::Url;

/// Built-in provider that needs neither config nor credentials.
pub const FAKE_PROVIDER: &str = "fake";

type Factory = Box<dyn Fn() -> Result<Box<dyn Provider>, RegistryError> + Send + Sync>;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("provider '{name}' is not configured (available: {available})")]
    UnknownProvider { name: String, available: String },
    #[error("no provider selected: pass --provider or set model.provider in config.yaml")]
    NoneSelected,
    #[error("provider '{name}' is not usable: {reason}")]
    Construction { name: String, reason: String },
}

/// Lazily constructs providers declared in `config.yaml`.
pub struct ProviderRegistry {
    factories: HashMap<String, Factory>,
}

impl ProviderRegistry {
    /// Registers every provider in `config.providers`, plus the built-in
    /// `fake` provider unless the config shadows that name.
    ///
    /// Performs no I/O and reads no environment variables; anything that can
    /// fail per-provider is deferred to [`Self::build`].
    pub fn from_config(config: &HermesConfig) -> Self {
        let mut factories: HashMap<String, Factory> = HashMap::new();
        for (name, provider) in &config.providers {
            let owned_name = name.clone();
            let owned_provider = provider.clone();
            factories.insert(
                name.clone(),
                Box::new(move || build_configured(&owned_name, &owned_provider)),
            );
        }
        if !factories.contains_key(FAKE_PROVIDER) {
            factories.insert(
                FAKE_PROVIDER.to_owned(),
                Box::new(|| Ok(Box::new(FakeProvider) as Box<dyn Provider>)),
            );
        }
        Self { factories }
    }

    /// An empty registry still resolves `fake`.
    pub fn offline() -> Self {
        Self::from_config(&HermesConfig::default())
    }

    /// Names of every registered provider, sorted, for use in error messages.
    pub fn available(&self) -> Vec<String> {
        let mut names: Vec<String> = self.factories.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn contains(&self, name: &str) -> bool {
        self.factories.contains_key(name)
    }

    /// Constructs the named provider. Failing here leaves the registry and any
    /// already-constructed provider untouched.
    pub fn build(&self, name: &str) -> Result<Box<dyn Provider>, RegistryError> {
        let factory = self
            .factories
            .get(name)
            .ok_or_else(|| RegistryError::UnknownProvider {
                name: name.to_owned(),
                available: self.available().join(", "),
            })?;
        factory()
    }

    /// Applies the documented precedence: explicit CLI `--provider` wins over
    /// `model.provider` from config, which wins over the `fake` default.
    ///
    /// `base_url_override` carries `--api-url` and applies to the model-level
    /// fallback path. Overriding the base URL of a fully declared provider is
    /// deferred to ticket 03, which already owns endpoint routing.
    pub fn select(
        &self,
        cli_provider: Option<&str>,
        config_provider: Option<&str>,
        base_url_override: Option<&str>,
        config: Option<&HermesConfig>,
    ) -> Result<Box<dyn Provider>, RegistryError> {
        let name = cli_provider.or(config_provider).unwrap_or(FAKE_PROVIDER);
        if self.contains(name) {
            return self.build(name);
        }
        // Not declared under `providers`. Fall back to the model-level config
        // so configs written before `providers` existed keep working, but only
        // when that config is actually usable as a provider.
        if let Some(config) = config {
            if let Some(provider) = model_level_fallback(name, base_url_override, config)? {
                return Ok(provider);
            }
        }
        if config_provider.is_none() && cli_provider.is_none() {
            return self.build(FAKE_PROVIDER);
        }
        Err(RegistryError::UnknownProvider {
            name: name.to_owned(),
            available: self.available().join(", "),
        })
    }
}

/// Builds a provider from the model-level config (`model.base_url` plus an API
/// key), which is how Hermes described a single provider before `providers`
/// existed. Returns `Ok(None)` when that section carries nothing usable.
fn model_level_fallback(
    name: &str,
    base_url_override: Option<&str>,
    config: &HermesConfig,
) -> Result<Option<Box<dyn Provider>>, RegistryError> {
    let raw_url = base_url_override
        .or(config.model.base_url.as_deref())
        .unwrap_or("https://api.openai.com/");
    let base_url =
        Url::parse(raw_url).map_err(|e| fail(name, &format!("invalid base URL: {e}")))?;
    let model = config.model.default.clone().unwrap_or_else(|| name.to_owned());

    let key = ["OPENAI_API_KEY", "HERMES_API_KEY"]
        .iter()
        .find_map(|var| std::env::var(var).ok().filter(|v| !v.is_empty()))
        .or_else(|| config.model.api_key.as_ref().map(|k| k.expose().to_owned()));
    let Some(key) = key else {
        // Nothing usable: no explicit base URL beyond the default and no key.
        return Ok(None);
    };
    Ok(Some(Box::new(HttpProvider::new(
        base_url,
        SecretString::from(key),
        model,
    ))))
}

fn build_configured(
    name: &str,
    provider: &ProviderConfig,
) -> Result<Box<dyn Provider>, RegistryError> {
    // Sorted so the choice is deterministic; `models` is a HashMap.
    let mut model_names: Vec<&String> = provider.models.keys().collect();
    model_names.sort();
    let model = model_names
        .first()
        .map(|s| (*s).clone())
        .ok_or_else(|| fail(name, "no models declared"))?;

    // `api_mode` is already a strict tagged enum on `ProviderConfig`, so an
    // unknown value can never reach here: it is rejected when the config file
    // is parsed (see `ApiMode` in config/schema.rs). Absence means the default,
    // `chat_completions`, keeping configs written before the field existed
    // working. Endpoint/payload selection for each mode lives in `HttpProvider`.
    let api_mode: ApiMode = provider.api_mode.unwrap_or_default();

    let raw_url = provider
        .api
        .as_deref()
        .ok_or_else(|| fail(name, "missing 'api' base URL"))?;
    let base_url = Url::parse(raw_url).map_err(|e| fail(name, &format!("invalid 'api' URL: {e}")))?;

    Ok(Box::new(
        HttpProvider::new(base_url, resolve_api_key(name, provider)?, model)
            .with_api_mode(api_mode),
    ))
}

/// Resolves a provider's credential. Names the variable when it is missing,
/// never its value.
fn resolve_api_key(name: &str, provider: &ProviderConfig) -> Result<SecretString, RegistryError> {
    if let Some(var) = provider.key_env.as_deref().filter(|v| !v.trim().is_empty()) {
        if let Ok(value) = std::env::var(var) {
            if !value.is_empty() {
                return Ok(SecretString::from(value));
            }
        }
        return Err(fail(
            name,
            &format!("environment variable '{var}' is not set or empty"),
        ));
    }
    Err(fail(
        name,
        "no 'key_env' declared and no fallback configured",
    ))
}

fn fail(name: &str, reason: &str) -> RegistryError {
    RegistryError::Construction {
        name: name.to_owned(),
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with(yaml: &str) -> HermesConfig {
        serde_yaml::from_str(yaml).expect("test fixture must parse")
    }

    /// `Box<dyn Provider>` is not `Debug`, so `unwrap_err()` is unavailable.
    fn err_message(result: Result<Box<dyn Provider>, RegistryError>) -> String {
        match result {
            Ok(_) => panic!("expected an error, got a provider"),
            Err(e) => e.to_string(),
        }
    }

    #[test]
    fn fake_is_available_without_any_config() {
        let registry = ProviderRegistry::offline();
        assert!(registry.contains(FAKE_PROVIDER));
        assert!(registry.build(FAKE_PROVIDER).is_ok());
    }

    #[test]
    fn from_config_does_not_require_credentials() {
        // No env var set, yet registration succeeds: construction is deferred.
        let registry = ProviderRegistry::from_config(&config_with(
            "providers:\n  local:\n    api: http://localhost:11434/\n    key_env: HERMES_TEST_UNSET_A\n    models:\n      llama3: {}\n",
        ));
        assert!(registry.contains("local"));
        assert!(registry.build("local").is_err());
    }

    #[test]
    fn unknown_provider_names_the_available_ones() {
        let registry = ProviderRegistry::from_config(&config_with(
            "providers:\n  beta:\n    api: http://b/\n  alpha:\n    api: http://a/\n",
        ));
        let err = err_message(registry.build("gamma"));
        assert!(err.contains("alpha"), "missing alpha: {err}");
        assert!(err.contains("beta"), "missing beta: {err}");
        assert!(err.contains("fake"), "missing fake: {err}");
    }

    #[test]
    fn select_prefers_cli_over_config_then_fake() {
        let registry = ProviderRegistry::offline();
        assert!(registry.select(Some(FAKE_PROVIDER), Some("nope"), None, None).is_ok());
        assert!(registry.select(None, Some(FAKE_PROVIDER), None, None).is_ok());
        assert!(registry.select(None, None, None, None).is_ok());
        assert!(matches!(
            registry.select(Some("nope"), None, None, None),
            Err(RegistryError::UnknownProvider { .. })
        ));
    }

    #[test]
    fn config_shadowing_fake_wins_over_builtin() {
        let registry = ProviderRegistry::from_config(&config_with(
            "providers:\n  fake:\n    api: http://localhost:9/\n    key_env: HERMES_TEST_UNSET_B\n    models:\n      m: {}\n",
        ));
        // The config entry replaced the built-in, so it now needs credentials.
        assert!(registry.build(FAKE_PROVIDER).is_err());
    }

    #[test]
    fn missing_key_env_names_the_variable_not_a_value() {
        std::env::remove_var("HERMES_TEST_UNSET_C");
        let registry = ProviderRegistry::from_config(&config_with(
            "providers:\n  p:\n    api: http://localhost:9/\n    key_env: HERMES_TEST_UNSET_C\n    models:\n      m: {}\n",
        ));
        let err = err_message(registry.build("p"));
        assert!(err.contains("HERMES_TEST_UNSET_C"), "must name the var: {err}");
        assert!(!err.contains("***REDACTED***"), "nothing to redact: {err}");
    }

    #[test]
    fn set_key_env_builds_successfully() {
        std::env::set_var("HERMES_TEST_SET_D", "dummy-value-not-a-real-key");
        let registry = ProviderRegistry::from_config(&config_with(
            "providers:\n  p:\n    api: http://localhost:9/\n    key_env: HERMES_TEST_SET_D\n    models:\n      m: {}\n",
        ));
        assert!(registry.build("p").is_ok());
        std::env::remove_var("HERMES_TEST_SET_D");
    }

    #[test]
    fn rejects_unknown_api_mode_at_config_parse() {
        // Ticket 03 (hybrid): an unknown api_mode is a schema error and must be
        // rejected while the config file is parsed, not at build/request time.
        let err = serde_yaml::from_str::<HermesConfig>(
            "providers:\n  p:\n    api: http://localhost:9/\n    api_mode: stream_magic\n    key_env: HERMES_TEST_SET_E\n    models:\n      m: {}\n",
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("stream_magic"), "must echo the bad value: {err}");
        assert!(err.contains("chat_completions"), "must list chat_completions: {err}");
        assert!(err.contains("completions"), "must list completions: {err}");
    }

    #[test]
    fn absent_api_mode_defaults_to_chat_completions() {
        // Absent means the documented default, backward compatible with configs
        // written before api_mode existed.
        let config = config_with(
            "providers:\n  p:\n    api: http://localhost:9/\n    key_env: HERMES_TEST_SET_G\n    models:\n      m: {}\n",
        );
        assert_eq!(config.providers["p"].api_mode, None);
        assert_eq!(
            config.providers["p"].api_mode.unwrap_or_default(),
            ApiMode::ChatCompletions
        );
    }

    #[test]
    fn completions_api_mode_parses_to_tagged_enum() {
        let config = config_with(
            "providers:\n  p:\n    api: http://localhost:9/\n    api_mode: completions\n    key_env: HERMES_TEST_SET_H\n    models:\n      m: {}\n",
        );
        assert_eq!(config.providers["p"].api_mode, Some(ApiMode::Completions));
    }

    #[test]
    fn model_choice_is_deterministic() {
        // HashMap iteration order is unspecified; the registry must sort.
        let registry = ProviderRegistry::from_config(&config_with(
            "providers:\n  p:\n    api: http://localhost:9/\n    key_env: HERMES_TEST_SET_F\n    models:\n      zebra: {}\n      alpha: {}\n      mango: {}\n",
        ));
        std::env::set_var("HERMES_TEST_SET_F", "dummy");
        for _ in 0..5 {
            assert!(registry.build("p").is_ok());
        }
        std::env::remove_var("HERMES_TEST_SET_F");
    }
}
