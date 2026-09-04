use serde::{de::Deserializer, Deserialize, Serialize};
use std::{collections::HashMap, fmt};
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct SecretString(String);
impl SecretString {
    pub fn expose(&self) -> &str {
        &self.0
    }
}
impl From<String> for SecretString {
    fn from(v: String) -> Self {
        Self(v)
    }
}
impl From<&str> for SecretString {
    fn from(v: &str) -> Self {
        Self(v.into())
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
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    #[serde(default)]
    pub ephemeral_system_prompt: Option<String>,
    #[serde(default)]
    pub output_dir: Option<String>,
    #[serde(default)]
    pub compression: Option<CompressionConfig>,
    #[serde(default)]
    pub eval_every: Option<u64>,
    #[serde(default)]
    pub eval_size: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompressionConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub target_max_tokens: Option<u64>,
}
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct ProviderConfig {
    pub api: Option<String>,
    pub name: Option<String>,
    pub api_mode: Option<ApiMode>,
    pub key_env: Option<String>,
    #[serde(default)]
    pub models: HashMap<String, serde_yaml::Value>,
    pub context_length: Option<u64>,
}

/// Wire shape a provider speaks. Absent `api_mode` means [`ApiMode::ChatCompletions`],
/// which keeps configs written before this field existed working unchanged.
///
/// Deserialization is strict: because the wire modes are unit variants with
/// `rename_all = "snake_case"`, any value other than `chat_completions` or
/// `completions` fails config parsing (`load_config`) with a serde error that
/// lists both valid values. This is deliberate: an unknown `api_mode` is a
/// schema error and must surface when the file is loaded, not when the first
/// HTTP request is attempted. It is distinct from *construction* failures
/// (missing `key_env`, bad base URL), which stay lazy per the registry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiMode {
    #[default]
    ChatCompletions,
    Completions,
}
impl ApiMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ChatCompletions => "chat_completions",
            Self::Completions => "completions",
        }
    }
}
impl fmt::Debug for ProviderConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProviderConfig")
            .field("api", &self.api)
            .field("api_mode", &self.api_mode)
            .field("key_env", &self.key_env)
            .field("models", &self.models.keys().collect::<Vec<_>>())
            .finish()
    }
}
#[derive(Clone, Serialize, Default)]
pub struct ModelConfig {
    pub default: Option<String>,
    pub provider: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<SecretString>,
    pub context_length: Option<u64>,
    pub name: Option<String>,
    pub dtype: Option<String>,
    pub quantization: Option<String>,
    pub device: Option<String>,
}
impl<'de> Deserialize<'de> for ModelConfig {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Input {
            Text(String),
            Map(Box<ModelMap>),
        }
        #[derive(Deserialize, Default)]
        struct ModelMap {
            default: Option<String>,
            provider: Option<String>,
            base_url: Option<String>,
            api_key: Option<SecretString>,
            context_length: Option<u64>,
            name: Option<String>,
            dtype: Option<String>,
            quantization: Option<String>,
            device: Option<String>,
        }
        match Input::deserialize(d)? {
            Input::Text(default) => Ok(Self {
                default: Some(default),
                ..Default::default()
            }),
            Input::Map(m) => Ok(Self {
                default: m.default,
                provider: m.provider,
                base_url: m.base_url,
                api_key: m.api_key,
                context_length: m.context_length,
                name: m.name,
                dtype: m.dtype,
                quantization: m.quantization,
                device: m.device,
            }),
        }
    }
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
