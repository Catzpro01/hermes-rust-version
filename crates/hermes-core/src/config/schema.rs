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
/// A configured MCP server: spawn `command` with `args`/`env` as a child
/// process speaking JSON-RPC over stdio (Spec 011). Off by default (an empty
/// `mcp_servers` map spawns nothing), so configs written before this field
/// existed behave unchanged. `env` values may hold secrets and are redacted on
/// every display/log path (see the manual `Debug`); the raw values remain
/// readable for spawning via the public field.
#[derive(Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Executable to launch (e.g. `npx`). Must be non-empty.
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// Extra environment for the child. Values may be secrets.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// When true, every tool from this server goes through the Spec 002
    /// confirmation gate (a denial surfaces `ToolError::Denied`, never
    /// bypassed). **Defaults to true (secure-by-default)**: a server must set
    /// `confirm: false` explicitly to run its tools without per-call approval.
    /// This prevents a configured MCP server from executing silently.
    #[serde(default = "default_true_confirm")]
    pub confirm: bool,
}
/// serde default helper: confirmation is ON unless a config sets `confirm: false`.
fn default_true_confirm() -> bool {
    true
}
impl Default for McpServerConfig {
    /// Programmatic default also confirms (consistent with the secure config
    /// default); callers that want auto-run set `confirm: false` explicitly.
    fn default() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            env: HashMap::new(),
            confirm: true,
        }
    }
}
impl fmt::Debug for McpServerConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut keys: Vec<&String> = self.env.keys().collect();
        keys.sort();
        let env = keys
            .into_iter()
            .map(|k| format!("{k}=***REDACTED***"))
            .collect::<Vec<_>>();
        f.debug_struct("McpServerConfig")
            .field("command", &self.command)
            .field("args", &self.args)
            .field("env", &env)
            .field("confirm", &self.confirm)
            .finish()
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
    /// Named MCP servers (Spec 011). Empty by default -> no MCP spawn, no MCP
    /// tools (zero regression).
    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerConfig>,
}
impl HermesConfig {
    /// Semantic validation of configured MCP servers. Returns `(server, reason)`
    /// pairs for each problem; empty means valid. A server with an empty
    /// `command` cannot be spawned and is reported here rather than failing at
    /// first use.
    pub fn validate_mcp_servers(&self) -> Vec<(String, String)> {
        let mut problems = Vec::new();
        let mut names: Vec<&String> = self.mcp_servers.keys().collect();
        names.sort();
        for name in names {
            let cfg = &self.mcp_servers[name];
            if cfg.command.trim().is_empty() {
                problems.push((name.clone(), "mcp server command must not be empty".into()));
            }
        }
        problems
    }
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
    /// Ordered names of `providers:` to try after the active provider when it
    /// fails before producing a stream, e.g. `model.fallback_chain:
    /// [anthropic, local]`. Empty (default) means no automatic fallback. Names
    /// must refer to configured `providers:` entries (the built-in `fake` is
    /// also allowed); an unknown name is rejected at startup. Only consulted
    /// when the active provider is itself a configured `providers:` entry.
    #[serde(default)]
    pub fallback_chain: Vec<String>,
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
            #[serde(default)]
            fallback_chain: Vec<String>,
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
                fallback_chain: m.fallback_chain,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_no_mcp_servers() {
        let c = HermesConfig::default();
        assert!(c.mcp_servers.is_empty());
        assert!(c.validate_mcp_servers().is_empty());
    }

    #[test]
    fn parses_mcp_servers_section() {
        let yaml = r#"
model:
  default: gpt
mcp_servers:
  github:
    command: npx
    args: ["-y", "@modelcontextprotocol/server-github"]
    env:
      GITHUB_PERSONAL_ACCESS_TOKEN: "tok-secret"
  local:
    command: "./my-server"
    confirm: false
  guarded:
    command: "./guarded"
"#;
        let c: HermesConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(c.mcp_servers.len(), 3);
        let gh = &c.mcp_servers["github"];
        assert_eq!(gh.command, "npx");
        assert_eq!(gh.args, vec!["-y", "@modelcontextprotocol/server-github"]);
        assert_eq!(gh.env["GITHUB_PERSONAL_ACCESS_TOKEN"], "tok-secret");
        // Secure-by-default: no explicit confirm -> true.
        assert!(gh.confirm);
        // Explicit opt-out is respected.
        assert!(!c.mcp_servers["local"].confirm);
        // A server with no confirm field defaults to true.
        assert!(c.mcp_servers["guarded"].confirm);
        assert!(c.validate_mcp_servers().is_empty());
    }

    #[test]
    fn mcp_server_confirm_defaults_to_true() {
        // Programmatic default confirms; explicit false opts out.
        assert!(McpServerConfig::default().confirm);
        assert!(!McpServerConfig { confirm: false, ..McpServerConfig::default() }.confirm);
    }

    #[test]
    fn debug_redacts_env_values_but_shows_keys_and_command() {
        let mut env = HashMap::new();
        env.insert("GITHUB_PERSONAL_ACCESS_TOKEN".to_owned(), "super-secret".to_owned());
        env.insert("B".to_owned(), "other".to_owned());
        let c = McpServerConfig {
            command: "npx".into(),
            args: vec!["-y".into(), "pkg".into()],
            env,
            confirm: false,
        };
        let dbg = format!("{c:?}");
        assert!(dbg.contains("npx"), "command must be visible: {dbg}");
        assert!(dbg.contains("GITHUB_PERSONAL_ACCESS_TOKEN"), "env key visible: {dbg}");
        assert!(dbg.contains("B"), "other env key visible: {dbg}");
        assert!(!dbg.contains("super-secret"), "value must be redacted: {dbg}");
        assert!(!dbg.contains("other"), "value must be redacted: {dbg}");
        assert!(dbg.contains("REDACTED"));
    }

    #[test]
    fn validation_reports_empty_command_per_server() {
        let mut c = HermesConfig::default();
        c.mcp_servers.insert("ok".into(), McpServerConfig { command: "npx".into(), ..Default::default() });
        c.mcp_servers.insert("bad".into(), McpServerConfig { command: "  ".into(), ..Default::default() });
        let problems = c.validate_mcp_servers();
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].0, "bad");
        assert!(problems[0].1.contains("command must not be empty"));
    }

    #[test]
    fn full_config_debug_redacts_server_env() {
        let mut c = HermesConfig::default();
        let mut env = HashMap::new();
        env.insert("K".to_owned(), "hidden".to_owned());
        c.mcp_servers.insert("srv".into(), McpServerConfig { command: "run".into(), env, ..Default::default() });
        let dbg = format!("{c:?}");
        assert!(!dbg.contains("hidden"), "server env value leaked in HermesConfig debug: {dbg}");
        assert!(dbg.contains("REDACTED"));
    }
}
