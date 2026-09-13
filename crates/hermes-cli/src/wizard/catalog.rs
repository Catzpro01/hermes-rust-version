//! Spec 017 T05 — static catalogs for the setup wizard (v0.21.0 verbatim).
//!
//! Generated from `docs/hermes-ui-spec/017/verbatim/` (spec §J.3: static data,
//! **no** extra `api_url`/`key_env`/`has_submenu` fields — those are resolved
//! from config like Python does). The `catalogs_match_verbatim_files` test
//! re-parses the source files and compares every string.
//!
//! - [`PROVIDERS`]  — `hermes_cli/models.py` `CANONICAL_PROVIDERS` (39 entries)
//! - [`TOOLSETS`]   — `hermes_cli/tools_config.py` `CONFIGURABLE_TOOLSETS` (26)
//! - [`DEFAULT_OFF_TOOLSETS`] — `tools_config.py` `_DEFAULT_OFF_TOOLSETS` (8)
//! - [`PLATFORMS`]  — `hermes_cli/gateway.py` `_PLATFORMS` (6, with the
//!   per-platform env `vars` prompts where the source defines them)

// Static catalog: some fields/helpers are only consumed by later tickets
// (T06/T07 pickers) and by the verbatim tests.
#![allow(dead_code)]

/// `ProviderEntry(id, label, description)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderEntry {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
}

/// `(key, "emoji Title", "tools…")`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolsetEntry {
    pub key: &'static str,
    pub label: &'static str,
    pub tools: &'static str,
}

/// One env var a platform asks for (`_PLATFORMS[*]['vars'][*]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformVar {
    pub name: &'static str,
    pub prompt: &'static str,
    pub password: bool,
    pub help: &'static str,
}

/// One `_PLATFORMS` entry. Platforms without `vars` in the source only
/// define `token_var`; the wizard prompts for that single variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformEntry {
    pub key: &'static str,
    pub label: &'static str,
    pub emoji: &'static str,
    pub token_var: &'static str,
    pub setup_instructions: &'static [&'static str],
    pub vars: &'static [PlatformVar],
}

pub const PROVIDERS: [ProviderEntry; 39] = [
    ProviderEntry { id: "nous", label: "Nous Portal", description: "Nous Portal (Everything your agent needs, 300+ models with bundled tool use)" },
    ProviderEntry { id: "fireworks", label: "Fireworks AI", description: "Fireworks AI (OpenAI-compatible direct model API)" },
    ProviderEntry { id: "openrouter", label: "OpenRouter", description: "OpenRouter (Pay-per-use API aggregator)" },
    ProviderEntry { id: "moa", label: "Mixture of Agents", description: "Mixture of Agents (named presets; aggregator acts after reference models)" },
    ProviderEntry { id: "novita", label: "NovitaAI", description: "NovitaAI (Cloud: Model API, Agent Sandbox, GPU Cloud)" },
    ProviderEntry { id: "lmstudio", label: "LM Studio", description: "LM Studio (Local desktop app with built-in model server)" },
    ProviderEntry { id: "anthropic", label: "Anthropic", description: "Anthropic (Claude models via API key or Claude Code)" },
    ProviderEntry { id: "openai-codex", label: "ChatGPT or Codex Subscription", description: "ChatGPT or Codex Subscription (Sign in with your ChatGPT account, uses Codex models)" },
    ProviderEntry { id: "openai-api", label: "OpenAI API", description: "OpenAI API (api.openai.com, API key)" },
    ProviderEntry { id: "alibaba", label: "Qwen Cloud", description: "Qwen Cloud / DashScope (Qwen + multi-provider)" },
    ProviderEntry { id: "xai-oauth", label: "xAI Grok OAuth (SuperGrok / Premium+)", description: "xAI Grok OAuth (SuperGrok / Premium+ subscription)" },
    ProviderEntry { id: "xiaomi", label: "Xiaomi MiMo", description: "Xiaomi MiMo (MiMo-V2.5 and V2 models: pro, omni, flash)" },
    ProviderEntry { id: "tencent-tokenhub", label: "Tencent TokenHub", description: "Tencent TokenHub (Hy4 preview via tokenhub.tencentmaas.com)" },
    ProviderEntry { id: "tencent-tokenplan", label: "Tencent TokenPlan", description: "Tencent TokenPlan (Hy4 preview via api.lkeap.cloud.tencent.com, Anthropic Messages)" },
    ProviderEntry { id: "nvidia", label: "NVIDIA NIM", description: "NVIDIA NIM (Nemotron models via build.nvidia.com or local NIM)" },
    ProviderEntry { id: "copilot", label: "GitHub Copilot", description: "GitHub Copilot (Uses GITHUB_TOKEN or gh auth token)" },
    ProviderEntry { id: "copilot-acp", label: "GitHub Copilot ACP", description: "GitHub Copilot ACP (Spawns copilot --acp --stdio)" },
    ProviderEntry { id: "huggingface", label: "Hugging Face", description: "Hugging Face Inference Providers" },
    ProviderEntry { id: "gemini", label: "Google AI Studio", description: "Google AI Studio (Native Gemini API)" },
    ProviderEntry { id: "vertex", label: "Google Vertex AI", description: "Google Vertex AI (Gemini via GCP; OAuth2 service account or ADC, GCP billing/quotas)" },
    ProviderEntry { id: "deepseek", label: "DeepSeek", description: "DeepSeek (V3, R1, coder, direct API)" },
    ProviderEntry { id: "xai", label: "xAI", description: "xAI Grok (Direct API)" },
    ProviderEntry { id: "zai", label: "Z.AI / GLM", description: "Z.AI / GLM (Zhipu direct API)" },
    ProviderEntry { id: "kimi-coding", label: "Kimi / Kimi Coding Plan", description: "Kimi Coding Plan (api.kimi.com & Moonshot API)" },
    ProviderEntry { id: "kimi-coding-cn", label: "Kimi / Moonshot (China)", description: "Kimi / Moonshot China (Domestic direct API)" },
    ProviderEntry { id: "stepfun", label: "StepFun Step Plan", description: "StepFun Step Plan (Agent / coding models via Step Plan API)" },
    ProviderEntry { id: "minimax", label: "MiniMax", description: "MiniMax (Global direct API)" },
    ProviderEntry { id: "minimax-oauth", label: "MiniMax (OAuth)", description: "MiniMax via OAuth browser login (Coding Plan, minimax.io)" },
    ProviderEntry { id: "minimax-cn", label: "MiniMax (China)", description: "MiniMax China (Domestic direct API)" },
    ProviderEntry { id: "ollama-cloud", label: "Ollama Cloud", description: "Ollama Cloud (Cloud-hosted open models, ollama.com)" },
    ProviderEntry { id: "arcee", label: "Arcee AI", description: "Arcee AI (Trinity models, direct API)" },
    ProviderEntry { id: "gmi", label: "GMI Cloud", description: "GMI Cloud (Multi-model direct API)" },
    ProviderEntry { id: "kilocode", label: "Kilo Code", description: "Kilo Code (Kilo Gateway API)" },
    ProviderEntry { id: "opencode-zen", label: "OpenCode Zen", description: "OpenCode Zen (Curated models, pay-as-you-go)" },
    ProviderEntry { id: "opencode-go", label: "OpenCode Go", description: "OpenCode Go (Open models subscription)" },
    ProviderEntry { id: "bedrock", label: "AWS Bedrock", description: "AWS Bedrock (Claude, Nova, Llama, DeepSeek; IAM or API key)" },
    ProviderEntry { id: "azure-foundry", label: "Azure Foundry", description: "Azure Foundry (OpenAI-style or Anthropic-style endpoint, your Azure AI deployment)" },
    ProviderEntry { id: "ai-gateway", label: "Vercel AI Gateway", description: "Vercel AI Gateway (Multi-model aggregator)" },
    ProviderEntry { id: "qwen-oauth", label: "Qwen OAuth (Portal)", description: "Qwen OAuth (Reuses local Qwen CLI login)" },
];

pub const TOOLSETS: [ToolsetEntry; 26] = [
    ToolsetEntry { key: "web", label: "🔍 Web Search & Scraping", tools: "web_search, web_extract" },
    ToolsetEntry { key: "browser", label: "🌐 Browser Automation", tools: "navigate, click, type, scroll" },
    ToolsetEntry { key: "terminal", label: "💻 Terminal & Processes", tools: "terminal, process" },
    ToolsetEntry { key: "file", label: "📁 File Operations", tools: "read, write, patch, search" },
    ToolsetEntry { key: "code_execution", label: "⚡ Code Execution", tools: "execute_code" },
    ToolsetEntry { key: "vision", label: "👁️  Vision / Image Analysis", tools: "vision_analyze" },
    ToolsetEntry { key: "video", label: "🎬 Video Analysis", tools: "video_analyze (requires video-capable model)" },
    ToolsetEntry { key: "image_gen", label: "🎨 Image Generation", tools: "image_generate" },
    ToolsetEntry { key: "video_gen", label: "🎬 Video Generation", tools: "video_generate (text/image/reference)" },
    ToolsetEntry { key: "x_search", label: "🐦 X (Twitter) Search", tools: "x_search (requires xAI OAuth or XAI_API_KEY)" },
    ToolsetEntry { key: "tts", label: "🔊 Text-to-Speech", tools: "text_to_speech" },
    ToolsetEntry { key: "stt", label: "🎙️ Speech-to-Text", tools: "voice transcription (gateway voice messages + voice mode)" },
    ToolsetEntry { key: "skills", label: "📚 Skills", tools: "list, view, manage" },
    ToolsetEntry { key: "todo", label: "📋 Task Planning", tools: "todo_list" },
    ToolsetEntry { key: "memory", label: "💾 Memory", tools: "persistent memory across sessions" },
    ToolsetEntry { key: "context_engine", label: "🧩 Context Engine", tools: "runtime tools from the active context engine" },
    ToolsetEntry { key: "session_search", label: "🔎 Session Search", tools: "search past conversations" },
    ToolsetEntry { key: "clarify", label: "❓ Clarifying Questions", tools: "clarify" },
    ToolsetEntry { key: "delegation", label: "👥 Task Delegation", tools: "delegate_task" },
    ToolsetEntry { key: "cronjob", label: "⏰ Cron Jobs", tools: "create/list/update/pause/resume/run, with optional attached skills" },
    ToolsetEntry { key: "homeassistant", label: "🏠 Home Assistant", tools: "smart home device control" },
    ToolsetEntry { key: "spotify", label: "🎵 Spotify", tools: "playback, search, playlists, library" },
    ToolsetEntry { key: "discord", label: "💬 Discord (read/participate)", tools: "fetch messages, search members, create thread" },
    ToolsetEntry { key: "discord_admin", label: "🛡️  Discord Server Admin", tools: "list channels/roles, pin, assign roles" },
    ToolsetEntry { key: "yuanbao", label: "🤖 Yuanbao", tools: "group info, member queries, DM" },
    ToolsetEntry { key: "computer_use", label: "🖱️  Computer Use (macOS/Windows/Linux)", tools: "background desktop control via cua-driver" },
];

/// Sorted for deterministic display/tests.
pub const DEFAULT_OFF_TOOLSETS: [&str; 8] = [
    "a2a",
    "discord",
    "discord_admin",
    "homeassistant",
    "spotify",
    "video",
    "video_gen",
    "x_search",
];

pub const PLATFORMS: [PlatformEntry; 6] = [
    PlatformEntry {
        key: "mattermost",
        label: "Mattermost",
        emoji: "💬",
        token_var: "MATTERMOST_TOKEN",
        setup_instructions: &[
            "1. In Mattermost: Integrations → Bot Accounts → Add Bot Account",
            "   (System Console → Integrations → Bot Accounts must be enabled)",
            "2. Give it a username (e.g. hermes) and copy the bot token",
            "3. Works with any self-hosted Mattermost instance — enter your server URL",
            "4. To find your user ID: click your avatar (top-left) → Profile",
            "   Your user ID is displayed there — click it to copy.",
            "   ⚠ This is NOT your username — it's a 26-character alphanumeric ID.",
            "5. To get a channel ID: click the channel name → View Info → copy the ID",
        ],
        vars: &[
            PlatformVar { name: "MATTERMOST_URL", prompt: "Server URL (e.g. https://mm.example.com)", password: false, help: "Your Mattermost server URL. Works with any self-hosted instance." },
            PlatformVar { name: "MATTERMOST_TOKEN", prompt: "Bot token", password: true, help: "Paste the bot token from step 2 above." },
            PlatformVar { name: "MATTERMOST_ALLOWED_USERS", prompt: "Allowed user IDs (comma-separated)", password: false, help: "Your Mattermost user ID from step 4 above." },
            PlatformVar { name: "MATTERMOST_HOME_CHANNEL", prompt: "Home channel ID (for cron/notification delivery, or empty to set later with /set-home)", password: false, help: "Channel ID where Hermes delivers cron results and notifications." },
            PlatformVar { name: "MATTERMOST_REPLY_MODE", prompt: "Reply mode — 'off' for flat messages, 'thread' for threaded replies (default: off)", password: false, help: "off = flat channel messages, thread = replies nest under your message." },
        ],
    },
    PlatformEntry {
        key: "signal",
        label: "Signal",
        emoji: "📡",
        token_var: "SIGNAL_HTTP_URL",
        setup_instructions: &[
        ],
        vars: &[
        ],
    },
    PlatformEntry {
        key: "weixin",
        label: "Weixin / WeChat",
        emoji: "💬",
        token_var: "WEIXIN_ACCOUNT_ID",
        setup_instructions: &[
        ],
        vars: &[
        ],
    },
    PlatformEntry {
        key: "bluebubbles",
        label: "BlueBubbles (iMessage)",
        emoji: "💬",
        token_var: "BLUEBUBBLES_SERVER_URL",
        setup_instructions: &[
            "1. Install BlueBubbles on a Mac that will act as your iMessage server:",
            "   https://bluebubbles.app/",
            "2. Complete the BlueBubbles setup wizard — sign in with your Apple ID",
            "3. In BlueBubbles Settings → API, note the Server URL and password",
            "4. The server URL is typically http://<your-mac-ip>:1234",
            "5. Hermes connects via the BlueBubbles REST API and receives",
            "   incoming messages via a local webhook",
            "6. To authorize users, use DM pairing: hermes pairing generate bluebubbles",
            "   Share the code — the user sends it via iMessage to get approved",
        ],
        vars: &[
            PlatformVar { name: "BLUEBUBBLES_SERVER_URL", prompt: "BlueBubbles server URL (e.g. http://192.168.1.10:1234)", password: false, help: "The URL shown in BlueBubbles Settings → API." },
            PlatformVar { name: "BLUEBUBBLES_PASSWORD", prompt: "BlueBubbles server password", password: true, help: "The password shown in BlueBubbles Settings → API." },
            PlatformVar { name: "BLUEBUBBLES_ALLOWED_USERS", prompt: "Pre-authorized phone numbers or iMessage IDs (comma-separated, or leave empty for DM pairing)", password: false, help: "Optional — pre-authorize specific users. Leave empty to use DM pairing instead (recommended)." },
            PlatformVar { name: "BLUEBUBBLES_HOME_CHANNEL", prompt: "Home channel (phone number or iMessage ID for cron/notifications, or empty)", password: false, help: "Phone number or Apple ID to deliver cron results and notifications to." },
        ],
    },
    PlatformEntry {
        key: "qqbot",
        label: "QQ Bot",
        emoji: "🐧",
        token_var: "QQ_APP_ID",
        setup_instructions: &[
            "1. Register a QQ Bot application at q.qq.com",
            "2. Note your App ID and App Secret from the application page",
            "3. Enable the required intents (C2C, Group, Guild messages)",
            "4. Configure sandbox or publish the bot",
        ],
        vars: &[
            PlatformVar { name: "QQ_APP_ID", prompt: "QQ Bot App ID", password: false, help: "Your QQ Bot App ID from q.qq.com." },
            PlatformVar { name: "QQ_CLIENT_SECRET", prompt: "QQ Bot App Secret", password: true, help: "Your QQ Bot App Secret from q.qq.com." },
            PlatformVar { name: "QQ_ALLOWED_USERS", prompt: "Allowed user OpenIDs (comma-separated, leave empty for open access)", password: false, help: "Optional — restrict DM access to specific user OpenIDs." },
            PlatformVar { name: "QQBOT_HOME_CHANNEL", prompt: "Home channel (user/group OpenID for cron delivery, or empty)", password: false, help: "OpenID to deliver cron results and notifications to." },
        ],
    },
    PlatformEntry {
        key: "yuanbao",
        label: "Yuanbao",
        emoji: "💎",
        token_var: "YUANBAO_APP_ID",
        setup_instructions: &[
            "1. Download the Yuanbao app from https://yuanbao.tencent.com/",
            "2. In the app, go to PAI → My Bot and create a new bot",
            "3. After the bot is created, copy the App ID and App Secret",
            "4. Enter them below and Hermes will connect automatically over WebSocket",
        ],
        vars: &[
            PlatformVar { name: "YUANBAO_APP_ID", prompt: "App ID", password: false, help: "The App ID from your Yuanbao IM Bot credentials." },
            PlatformVar { name: "YUANBAO_APP_SECRET", prompt: "App Secret", password: true, help: "The App Secret (used for HMAC signing) from your Yuanbao IM Bot." },
        ],
    },
];

/// Toolsets that are ON by default (everything not in
/// [`DEFAULT_OFF_TOOLSETS`]), in catalog order.
pub fn default_on_toolsets() -> Vec<&'static str> {
    TOOLSETS
        .iter()
        .map(|t| t.key)
        .filter(|k| !DEFAULT_OFF_TOOLSETS.contains(k))
        .collect()
}

pub fn provider_by_id(id: &str) -> Option<&'static ProviderEntry> {
    PROVIDERS.iter().find(|p| p.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Extracts every Python string literal from an `ast.unparse` dump, in
    /// order (handles `'…'`/`"…"` and the `\'`, `\"`, `\\`, `\n` escapes
    /// that occur in the catalogs).
    fn python_strings(src: &str) -> Vec<String> {
        let body: String = src
            .lines()
            .filter(|l| !l.starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n");
        let mut out = Vec::new();
        let mut chars = body.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '\'' && c != '"' {
                continue;
            }
            let quote = c;
            let mut s = String::new();
            loop {
                match chars.next() {
                    Some('\\') => match chars.next() {
                        Some('n') => s.push('\n'),
                        Some(other) => s.push(other),
                        None => break,
                    },
                    Some(ch) if ch == quote => break,
                    Some(ch) => s.push(ch),
                    None => break,
                }
            }
            out.push(s);
        }
        out
    }

    fn verbatim(name: &str) -> String {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/hermes-ui-spec/017/verbatim/");
        std::fs::read_to_string(format!("{root}{name}")).expect(name)
    }

    #[test]
    fn catalogs_match_verbatim_files() {
        let want = python_strings(&verbatim("providers_canonical.txt"));
        let got: Vec<String> = PROVIDERS
            .iter()
            .flat_map(|p| [p.id, p.label, p.description])
            .map(str::to_owned)
            .collect();
        assert_eq!(PROVIDERS.len(), 39, "spec §G: 39 providers");
        assert_eq!(got, want);

        let want = python_strings(&verbatim("toolsets_configurable.txt"));
        let got: Vec<String> = TOOLSETS
            .iter()
            .flat_map(|t| [t.key, t.label, t.tools])
            .map(str::to_owned)
            .collect();
        assert_eq!(TOOLSETS.len(), 26, "spec §C.6: 26 toolsets");
        assert_eq!(got, want);

        let mut want = python_strings(&verbatim("toolsets_default_off.txt"));
        want.sort();
        assert_eq!(DEFAULT_OFF_TOOLSETS.map(str::to_owned).to_vec(), want);
        // Every default-off key that is configurable must exist in the
        // catalog (`a2a` is a non-configurable toolset upstream).
        for k in DEFAULT_OFF_TOOLSETS {
            assert!(k == "a2a" || TOOLSETS.iter().any(|t| t.key == k), "{k}");
        }
    }

    #[test]
    fn platform_catalog_matches_verbatim_source() {
        let src = verbatim("platforms_gateway.txt");
        let strings = python_strings(&src);
        assert_eq!(PLATFORMS.len(), 6, "spec §C.5: 6 `_PLATFORMS` entries");
        for p in PLATFORMS {
            for s in [p.key, p.label, p.emoji, p.token_var] {
                assert!(strings.contains(&s.to_owned()), "{s:?} missing from source");
            }
            for line in p.setup_instructions {
                assert!(strings.contains(&(*line).to_owned()), "{line:?}");
            }
            for v in p.vars {
                for s in [v.name, v.prompt, v.help] {
                    assert!(strings.contains(&s.to_owned()), "{s:?}");
                }
            }
        }
        // Order + keys pinned.
        assert_eq!(
            PLATFORMS.map(|p| p.key),
            ["mattermost", "signal", "weixin", "bluebubbles", "qqbot", "yuanbao"]
        );
    }

    #[test]
    fn default_on_excludes_default_off() {
        let on = default_on_toolsets();
        assert_eq!(on.len(), 26 - 7); // 8 default-off, `a2a` not configurable
        assert!(on.contains(&"web") && !on.contains(&"spotify"));
        assert_eq!(provider_by_id("lmstudio").unwrap().label, "LM Studio");
    }
}
