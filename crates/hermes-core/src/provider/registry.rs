//! Config-driven provider selection.
//!
//! Replaces the literal `match` on provider names that `main.rs` used to
//! perform. Providers are registered as factories and only constructed when
//! actually selected, so one misconfigured provider in `config.yaml` cannot
//! prevent startup or take down the provider currently in use.

use crate::config::{ApiMode, HermesConfig, ProviderConfig, SecretString};
use crate::provider::{FakeProvider, FallbackProvider, HttpProvider, Provider};
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
            // The model-level key is the global fallback when a provider does
            // not pin its own key_env. It is captured here (not read from the
            // config later) so construction stays free of any registry state.
            let fallback_key = config.model.api_key.clone();
            factories.insert(
                name.clone(),
                Box::new(move || build_configured(&owned_name, &owned_provider, fallback_key.clone())),
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

    /// Startup selection: resolves the active provider (same precedence as
    /// [`Self::select`]) and wraps it in a [`FallbackProvider`] whose remaining
    /// hops are `config.model.fallback_chain`, when that active provider is a
    /// registered `providers:` entry.
    ///
    /// Fallback only makes sense between declared providers, so it is ignored
    /// when the active provider comes from the model-level fallback path. Each
    /// fallback name must be registered (the built-in `fake` counts); an
    /// unknown name is a strict error listing what is available. If only the
    /// active provider survives, the plain provider is returned unwrapped so a
    /// single-provider session has no fallback indirection.
    pub fn select_with_fallback(
        &self,
        cli_provider: Option<&str>,
        config_provider: Option<&str>,
        base_url_override: Option<&str>,
        config: Option<&HermesConfig>,
    ) -> Result<Box<dyn Provider>, RegistryError> {
        let active = cli_provider.or(config_provider).unwrap_or(FAKE_PROVIDER);
        // Fallback is a per-`providers:` strategy: only engage when the active
        // provider is a registered name. Otherwise behave exactly like `select`.
        if !self.contains(active) {
            return self.select(
                cli_provider,
                config_provider,
                base_url_override,
                config,
            );
        }
        let fallback_chain: Vec<String> = config
            .map(|c| c.model.fallback_chain.clone())
            .unwrap_or_default();
        // Strict reject: every named fallback hop must exist up front, so a
        // typo in the chain is caught at startup rather than silently skipped.
        for name in &fallback_chain {
            if !self.contains(name) {
                return Err(RegistryError::UnknownProvider {
                    name: name.clone(),
                    available: self.available().join(", "),
                });
            }
        }
        // Build the primary (its construction failure propagates, matching
        // `select`), then each usable fallback hop in declared order. A fallback
        // hop whose construction fails (e.g. a missing key_env) is dropped so a
        // misconfigured backup never takes down a working primary; the strict
        // name check above already guarantees the name was declared.
        let mut hops: Vec<(String, Box<dyn Provider>)> = Vec::new();
        hops.push((active.to_owned(), self.build(active)?));
        for name in fallback_chain {
            if name == active {
                continue;
            }
            if let Ok(provider) = self.build(&name) {
                hops.push((name, provider));
            }
        }
        if hops.len() == 1 {
            return Ok(hops.pop().expect("one hop").1);
        }
        Ok(Box::new(FallbackProvider::new(hops)))
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
    fallback_key: Option<SecretString>,
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
        HttpProvider::new(
            base_url,
            resolve_api_key(name, provider, fallback_key)?,
            model,
        )
        .with_api_mode(api_mode),
    ))
}

/// Resolves a configured provider's credential. Names variables, never values.
///
/// Fallback chain (documented in `docs/SECURITY.md` under Spec 005 — STRIDE):
/// 1. If `key_env` is pinned (declared, non-empty) and the environment variable
///    it names holds a non-empty value, use it.
/// 2. If `key_env` is pinned but the variable is unset/empty, **error** naming
///    the variable — it must NOT silently fall back to `model.api_key`. The
///    operator explicitly chose that variable, so using a different key would
///    risk sending one provider's credential to another provider's endpoint
///    (cross-provider credential leakage / Spoofing).
/// 3. If `key_env` is absent, fall back to the global `model.api_key`, then
///    error if neither is available.
fn resolve_api_key(
    name: &str,
    provider: &ProviderConfig,
    fallback_key: Option<SecretString>,
) -> Result<SecretString, RegistryError> {
    // (1)/(2) Pinned key_env.
    if let Some(var) = provider.key_env.as_deref().filter(|v| !v.trim().is_empty()) {
        let set = std::env::var(var)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(SecretString::from);
        return set.ok_or_else(|| {
            fail(
                name,
                &format!("environment variable '{var}' is not set or empty"),
            )
        });
    }
    // (3) No pin: fall back to the global model key, then error.
    if let Some(key) = fallback_key {
        if !key.expose().is_empty() {
            return Ok(key);
        }
    }
    Err(fail(
        name,
        "no 'key_env' declared and no 'model.api_key' fallback configured",
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
    fn absent_key_env_falls_back_to_model_api_key() {
        // A configured provider that does not pin key_env uses the global
        // model.api_key (chain: key_env -> model.api_key -> error).
        std::env::remove_var("HERMES_TEST_ABSENT_FALLBACK_J");
        let registry = ProviderRegistry::from_config(&config_with(
            "model:\n  api_key: sk-global-dummy\nproviders:\n  p:\n    api: http://localhost:9/\n    models:\n      m: {}\n",
        ));
        assert!(
            registry.build("p").is_ok(),
            "absent key_env should fall back to model.api_key"
        );
    }

    #[test]
    fn pinned_but_empty_key_env_does_not_fall_back_to_model_key() {
        // Credential-confusion guard: an explicitly pinned key_env whose
        // variable is unset/empty must ERROR, even when model.api_key is
        // present. Falling back would send a different provider's key to this
        // endpoint (cross-provider credential leakage).
        std::env::remove_var("HERMES_TEST_PINNED_EMPTY_K");
        let registry = ProviderRegistry::from_config(&config_with(
            "model:\n  api_key: sk-openai-should-never-leak\nproviders:\n  p:\n    api: http://localhost:9/\n    key_env: HERMES_TEST_PINNED_EMPTY_K\n    models:\n      m: {}\n",
        ));
        let err = err_message(registry.build("p"));
        assert!(
            err.contains("HERMES_TEST_PINNED_EMPTY_K"),
            "must name the pinned var: {err}"
        );
        assert!(
            !err.contains("sk-openai-should-never-leak"),
            "must never mention a fallback value: {err}"
        );
        assert!(
            !err.contains("model.api_key"),
            "pinned-but-empty must not silently fall back: {err}"
        );
    }

    #[test]
    fn no_key_env_and_no_model_key_errors_with_guidance() {
        // Neither a pin nor a global key => error that names both options,
        // never a value.
        let registry = ProviderRegistry::from_config(&config_with(
            "providers:\n  p:\n    api: http://localhost:9/\n    models:\n      m: {}\n",
        ));
        let err = err_message(registry.build("p"));
        assert!(err.contains("key_env"), "must mention key_env: {err}");
        assert!(
            err.contains("model.api_key"),
            "must mention model.api_key option: {err}"
        );
        assert!(!err.contains("***REDACTED***"), "nothing to redact: {err}");
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

    #[test]
    fn select_with_fallback_resolves_offline_fake_single() {
        let registry = ProviderRegistry::offline();
        assert!(registry.select_with_fallback(None, None, None, None).is_ok());
    }

    #[test]
    fn select_with_fallback_rejects_an_unknown_fallback_name() {
        // A fallback name that is not a declared provider must be rejected
        // strictly (fail fast at startup), naming the bad name and available
        // providers.
        std::env::set_var("HERMES_TEST_FB_B", "b-key");
        let config = config_with(
            "model:\n  provider: b\n  fallback_chain: [nope]\nproviders:\n  b:\n    api: http://localhost:9/\n    key_env: HERMES_TEST_FB_B\n    models:\n      m: {}\n",
        );
        let registry = ProviderRegistry::from_config(&config);
        let err = err_message(registry.select_with_fallback(None, Some("b"), None, Some(&config)));
        assert!(err.contains("nope"), "must name the bad hop: {err}");
        assert!(err.contains("b"), "must list available providers: {err}");
        assert!(err.contains("fake"), "must list fake as available: {err}");
        std::env::remove_var("HERMES_TEST_FB_B");
    }

    #[test]
    fn select_with_fallback_builds_a_chain_when_more_than_one_hop_survives() {
        // Both hops are usable (keys present) -> a multi-hop chain is returned.
        std::env::set_var("HERMES_TEST_FB_PRIMARY", "primary-key");
        std::env::set_var("HERMES_TEST_FB_BACKUP", "backup-key");
        let config = config_with(
            "model:\n  provider: primary\n  fallback_chain: [backup]\nproviders:\n  primary:\n    api: http://localhost:9/\n    key_env: HERMES_TEST_FB_PRIMARY\n    models:\n      m: {}\n  backup:\n    api: http://localhost:10/\n    key_env: HERMES_TEST_FB_BACKUP\n    models:\n      m: {}\n",
        );
        let registry = ProviderRegistry::from_config(&config);
        let provider = registry
            .select_with_fallback(None, Some("primary"), None, Some(&config))
            .expect("both hops usable -> chain must build");
        let _ = provider;
        std::env::remove_var("HERMES_TEST_FB_PRIMARY");
        std::env::remove_var("HERMES_TEST_FB_BACKUP");
    }

    #[test]
    fn select_with_fallback_keeps_single_provider_when_only_primary_builds() {
        // The fallback hop has no usable credential, so it is dropped and a
        // single (unwrapped) provider is returned rather than failing startup.
        std::env::set_var("HERMES_TEST_FB_ONLY", "primary-key");
        let config = config_with(
            "model:\n  provider: primary\n  fallback_chain: [broken]\nproviders:\n  primary:\n    api: http://localhost:9/\n    key_env: HERMES_TEST_FB_ONLY\n    models:\n      m: {}\n  broken:\n    api: http://localhost:10/\n    key_env: HERMES_TEST_FB_UNSET_VAR\n    models:\n      m: {}\n",
        );
        let registry = ProviderRegistry::from_config(&config);
        let provider = registry
            .select_with_fallback(None, Some("primary"), None, Some(&config))
            .expect("primary usable -> misconfigured backup must not fail startup");
        let _ = provider;
        std::env::remove_var("HERMES_TEST_FB_ONLY");
        std::env::remove_var("HERMES_TEST_FB_UNSET_VAR");
    }

    #[test]
    fn select_with_fallback_honours_cli_precedence_over_config_provider() {
        // `--provider fake` wins over a stale `model.provider: auto`; the chain
        // is empty offline so a single fake provider is returned.
        let registry = ProviderRegistry::offline();
        let provider = registry
            .select_with_fallback(Some(FAKE_PROVIDER), Some("auto"), None, None)
            .expect("cli-named fake must resolve regardless of config provider");
        let _ = provider;
    }
}
