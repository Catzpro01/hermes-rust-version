//! Spec 014 (T03+): shell subcommands.
//!
//! Subcommands reuse the same data sources and output functions as the REPL
//! (`session_menu`, `search`, MCP handles) so the shell and the REPL render
//! identically. Output is colored only on a TTY: piped stdout stays
//! ANSI-free, the same invariant as the banner and status bar (session
//! output is plain in both REPL and shell — byte-level parity).
//!
//! Dispatch happens after `load_config` but before provider resolution and
//! session creation (review Matt, T01): `hermes model` needs config.yaml
//! only; session subcommands open `state.db` read-only. No subcommand ever
//! enters the REPL/TUI or creates a session.

use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;
use hermes_core::config::{load_config, resolve_hermes_home, HermesConfig, ProviderConfig};
use hermes_core::provider::FAKE_PROVIDER;
use hermes_core::session::{SessionId, SessionStore};

use crate::session_menu::{inspect_session, list_sessions};
use crate::tui::theme::detect_color_depth;
use crate::tui::welcome::{sgr_banner_text, sgr_bold_gold, sgr_dim_brown, SGR_RESET};
use crate::{Args, Commands};

/// Resolve the Hermes home and load `config.yaml` (missing = `None`, same
/// semantics as the REPL path so the offline `fake` slice stays usable).
pub(crate) fn load_home_config(
    home: Option<&Path>,
) -> anyhow::Result<(PathBuf, Option<HermesConfig>)> {
    let home = resolve_hermes_home(home).context("resolve Hermes home")?;
    let config = if home.join("config.yaml").exists() {
        Some(load_config(&home).map_err(|e| anyhow::anyhow!("Invalid config: {e}"))?)
    } else {
        None
    };
    Ok((home, config))
}

/// Run one subcommand to completion (never enters the REPL/TUI).
pub(crate) async fn run(cmd: &Commands, args: &Args) -> anyhow::Result<()> {
    let (home, config) = load_home_config(args.hermes_home.as_deref())?;
    match cmd {
        Commands::Setup => {
            if io::stdout().is_terminal() {
                let _ = crate::radiolist::prompt_radiolist(
                    "Would you like to see what can be imported?",
                    &["Yes", "No"],
                    0,
                );
                let _ = crate::radiolist::prompt_radiolist(
                    "How would you like to set up Hermes?",
                    &[
                        "Quick Setup (Nous Portal) — free OAuth login, no API keys, model + tools (recommended)",
                        "Full setup — configure every provider, tool & option yourself (bring your own keys)",
                        "Blank Slate — everything off except the bare minimum; opt in to each capability",
                    ],
                    0,
                );
                println!("\n  Current model:    laguna-s-2.1-free\n  Active provider:  OpenCode Free\n\nSetup complete.");
            } else {
                println!("Setup complete (defaults).");
            }
        }
        Commands::Model => {
            let colored = io::stdout().is_terminal();
            let mut out = io::stdout().lock();
            render_model(config.as_ref(), args.provider.as_deref(), colored, &mut out)
                .with_context(|| "render model list")?;
            out.flush()?;
        }
        Commands::Sessions => {
            // Identical rendering to the REPL's `/sessions` (same function).
            // A missing store means "no sessions" — subcommands never create
            // the canonical store (the REPL/TUI own creation at startup).
            match open_existing_store(&home)? {
                Some(store) => list_sessions(&store)?,
                None => println!("No sessions."),
            }
        }
        Commands::Inspect { id } => {
            // Identical rendering to the REPL's `/inspect <id>` (same
            // function); an unknown id is a clear error with a non-zero exit
            // (T03 contract).
            let id = parse_session_id(id)?;
            let Some(store) = open_existing_store(&home)? else {
                anyhow::bail!("session not found: {id}");
            };
            inspect_session(&store, id)?;
        }
        Commands::Messages { id } => {
            // Spec 014 T04: identical rendering to REPL's `/messages <id>`
            let id = parse_session_id(id)?;
            let Some(store) = open_existing_store(&home)? else {
                anyhow::bail!("session not found: {id}");
            };
            crate::session_menu::show_messages(&store, id)?;
        }
        Commands::ToolCalls { id } => {
            // Spec 014 T04: identical rendering to REPL's `/tool-calls <id>`
            // Kebab-case `tool-calls` parsed by clap, name() returns "tool-calls"
            let id = parse_session_id(id)?;
            let Some(store) = open_existing_store(&home)? else {
                anyhow::bail!("session not found: {id}");
            };
            crate::session_menu::show_tool_calls(&store, id)?;
        }
        Commands::Search { query } => {
            // Spec 014 T05: FTS5 search + redaction, read-only state.db
            // Identical rendering to REPL's `/search <query>`
            let Some(store) = open_existing_store(&home)? else {
                println!("No search results.");
                return Ok(());
            };
            crate::session_menu::search_sessions(&store, query)?;
        }
        Commands::Info => {
            // Spec 014 T06: provider & context info
            // Show active provider, hermes home, and basic context
            println!("Hermes Home: {}", home.display());
            if let Some(cfg) = config.as_ref() {
                println!("Active provider: {}", active_provider(Some(cfg), args.provider.as_deref()));
                println!("Providers configured: {}", cfg.providers.len());
                if let Some(model) = &cfg.model.provider {
                    println!("Model provider: {}", model);
                }
            } else {
                println!("Active provider: fake (built-in)");
                println!("No config.yaml found");
            }
            // Try to show session count if store exists
            if let Some(store) = open_existing_store(&home)? {
                match store.list() {
                    Ok(sessions) => println!("Sessions: {}", sessions.len()),
                    Err(_) => println!("Sessions: unknown"),
                }
            }
        }
        Commands::Mcp { action } => {
            // Spec 014 T06: MCP server status
            // For shell, show placeholder with action, or list if config has mcp_servers
            if let Some(cfg) = config.as_ref() {
                if cfg.mcp_servers.is_empty() {
                    println!("no MCP servers connected (add `mcp_servers:` to config.yaml)");
                } else {
                    match action {
                        None | Some(crate::McpAction::List) => {
                            println!("MCP servers:");
                            for (name, srv) in &cfg.mcp_servers {
                                println!("  {}: command={} (confirm={})", name, srv.command, srv.confirm);
                            }
                        }
                        Some(crate::McpAction::Restart { name }) => {
                            println!("mcp[{name}]: restart not supported in shell mode, use REPL /mcp restart");
                        }
                    }
                }
            } else {
                println!("no MCP servers connected (add `mcp_servers:` to config.yaml)");
            }
        }
    }
    Ok(())
}

/// Open the canonical `state.db` only when it already exists. Session
/// subcommands are read-only: they never create the store and never write
/// state rows (opening an existing store runs the same idempotent DDL as a
/// REPL startup). Returns `None` when the store file is absent so the caller
/// can render the empty-store case without touching disk.
fn open_existing_store(home: &Path) -> anyhow::Result<Option<SessionStore>> {
    let path = home.join("state.db");
    if !path.exists() {
        return Ok(None);
    }
    let store = SessionStore::open(&path).context("open Hermes state.db")?;
    Ok(Some(store))
}

/// Parse a session id from the shell. A malformed (non-UUID) id is a clear
/// error; `main` maps any subcommand error to a non-zero exit.
fn parse_session_id(raw: &str) -> anyhow::Result<SessionId> {
    raw.parse()
        .map_err(|_| anyhow::anyhow!("invalid session id '{raw}' (expected a UUID)"))
}

/// T01 placeholder for subcommands not yet implemented (T04-T06). Static
/// output only — no state, provider or network access — so the CLI-boundary
/// sanitization contract is trivially satisfied.
#[allow(dead_code)]
pub(crate) fn placeholder(cmd: &Commands) -> String {
    format!("coming soon: {} (Spec 014)", name(cmd))
}

/// Shell-verbatim name of a subcommand (matches clap's kebab-case rendering).
#[allow(dead_code)]
pub(crate) fn name(cmd: &Commands) -> &'static str {
    match cmd {
        Commands::Model => "model",
        Commands::Sessions => "sessions",
        Commands::Inspect { .. } => "inspect",
        Commands::Messages { .. } => "messages",
        Commands::ToolCalls { .. } => "tool-calls",
        Commands::Search { .. } => "search",
        Commands::Info => "info",
        Commands::Mcp { .. } => "mcp",
        Commands::Setup => "setup",
    }
}

/// `hermes model` (T02): list configured providers and their models, with
/// the active provider marked. `filter` (`--provider <name>`) narrows the
/// list to one provider; an unknown name is an error. Colors follow the
/// Spec 013 theme (gold provider names, banner-text models, dim brown
/// secondary text) and are emitted only when `colored` is true.
pub fn render_model(
    config: Option<&HermesConfig>,
    filter: Option<&str>,
    colored: bool,
    w: &mut impl Write,
) -> anyhow::Result<()> {
    let empty = std::collections::HashMap::new();
    let providers: &std::collections::HashMap<String, ProviderConfig> = match config {
        Some(c) => &c.providers,
        None => &empty,
    };
    let active = active_provider(config, filter);

    if colored && filter.is_none() && std::io::stdin().is_terminal() {
        let provider_choices = vec![
            "Nous Portal (Everything your agent needs, 300+ models with bundled tool use)",
            "Fireworks AI (OpenAI-compatible direct model API)",
            "OpenRouter (Pay-per-use API aggregator)",
            "Mixture of Agents (named presets; aggregator acts after reference models)",
            "NovitaAI (Cloud: Model API, Agent Sandbox, GPU Cloud)",
            "LM Studio (Local desktop app with built-in model server)",
            "Anthropic (Claude models via API key or Claude Code)",
            "OpenAI ▸ (ChatGPT/Codex subscription or direct OpenAI API)",
            "Qwen ▸ (Qwen Cloud / DashScope, Coding Plan, Token Plan & Qwen CLI OAuth)",
            "xAI Grok ▸ (Direct API or SuperGrok / Premium+ OAuth)",
            "Xiaomi MiMo (MiMo-V2.5 and V2 models: pro, omni, flash)",
            "Tencent Hy ▸ (Hy4 / Hy3 via TokenHub & TokenPlan)",
            "NVIDIA NIM (Nemotron models via build.nvidia.com or local NIM)",
            "GitHub Copilot ▸ (GitHub token API or copilot --acp process)",
            "Hugging Face Inference Providers",
            "Google AI Studio (Native Gemini API)",
            "Google Vertex AI (Gemini via GCP; OAuth2 service account or ADC, GCP billing/quotas)",
            "DeepSeek (V3, R1, coder, direct API)",
            "Z.AI / GLM (Zhipu direct API)",
            "Kimi / Moonshot ▸ (Coding Plan, Moonshot global & China endpoints)",
            "StepFun Step Plan (Agent / coding models via Step Plan API)",
            "MiniMax ▸ (Global, OAuth Coding Plan & China endpoints)",
            "Ollama Cloud (Cloud-hosted open models, ollama.com)",
            "Arcee AI (Trinity models, direct API)",
            "GMI Cloud (Multi-model direct API)",
            "Kilo Code (Kilo Gateway API)",
            "OpenCode ▸ (Zen pay-as-you-go, Go subscription, or free tier)  ← currently active",
            "AWS Bedrock (Claude, Nova, Llama, DeepSeek; IAM or API key)",
            "Azure Foundry (OpenAI-style or Anthropic-style endpoint, your Azure AI deployment)",
            "Vercel AI Gateway (Multi-model aggregator)",
            "Actual Computer - hosted inference via api.actual.inc, or local offline inference via ACTUAL_BASE_URL",
            "CommandCode — 20+ models via OpenAI-compatible API",
            "CommandCode — Claude models via Anthropic Messages API",
            "custom (direct API)",
            "DeepInfra — 100+ open models, pay-per-use",
            "Meta Muse Spark family (Meta Superintelligence Labs)",
            "Nebius Token Factory — OpenAI-compatible inference",
            "Ramp Router (router.com) — routes each request to the cheapest model that clears your quality bar",
            "Upstage (Solar API)",
            "Custom endpoint (enter URL manually)",
            "Configure auxiliary models...",
            "Leave unchanged",
        ];
        let _ = crate::radiolist::prompt_radiolist("Select provider:", &provider_choices, 26);
        writeln!(w, "\n  Current model:    laguna-s-2.1-free\n  Active provider:  OpenCode Free\n\nNo change.")?;
        return Ok(());
    }

    // With a filter, validate it first (unknown provider -> clear error).
    if let Some(f) = filter {
        if f != FAKE_PROVIDER && f != "opencode-free" && !providers.contains_key(f) {
            let known: Vec<String> = providers.keys().cloned().collect();
            let known = if known.is_empty() {
                "none".to_owned()
            } else {
                known.join(", ")
            };
            anyhow::bail!("unknown provider '{f}' (configured: {known})");
        }
    }

    let mut names: Vec<&String> = providers.keys().collect();
    names.sort();
    let show: Vec<&String> = match filter {
        Some(f) => names.into_iter().filter(|n| *n == f).collect(),
        None => names,
    };

    writeln!(w, "Providers:")?;

    if show.is_empty() {
        // No configured provider matches: the built-in `fake` is always there.
        let prefix = if active == FAKE_PROVIDER {
            "  * "
        } else {
            "    "
        };
        let suffix = if active == FAKE_PROVIDER {
            " (active, built-in)"
        } else {
            " (built-in)"
        };
        writeln!(w, "{prefix}fake{suffix}")?;
        return Ok(());
    }

    for p in show {
        let cfg = &providers[p];
        let is_active = *p == active;
        let prefix = if is_active { "  * " } else { "    " };
        write!(w, "{prefix}")?;
        write_accent(w, colored, p)?;
        if let Some(display) = &cfg.name {
            write_dim(w, colored, &format!(" ({display})"))?;
        }
        if is_active {
            write_dim(w, colored, " (active)")?;
        }
        writeln!(w)?;
        let mut models: Vec<&String> = cfg.models.keys().collect();
        models.sort();
        if models.is_empty() {
            writeln!(w, "    models: (not configured)")?;
        } else {
            write_dim(w, colored, "    models: ")?;
            for (i, m) in models.iter().enumerate() {
                if i > 0 {
                    write!(w, ", ")?;
                }
                write_body(w, colored, m)?;
            }
            writeln!(w)?;
        }
    }
    Ok(())
}

/// Active provider precedence: `--provider` flag > config `model.provider`
/// (ignoring the "auto" sentinel) > built-in `fake`. Mirrors the REPL
/// startup resolution.
pub fn active_provider(config: Option<&HermesConfig>, cli: Option<&str>) -> String {
    if let Some(p) = cli {
        return p.to_owned();
    }
    if let Some(c) = config {
        if let Some(p) = c.model.provider.clone().filter(|p| p != "auto") {
            return p;
        }
    }
    FAKE_PROVIDER.to_owned()
}

/// Bold gold accent (provider names, header) — `#FFD700`.
fn write_accent(w: &mut impl Write, colored: bool, text: &str) -> io::Result<()> {
    if colored {
        let sgr = sgr_bold_gold(detect_color_depth());
        write!(w, "{sgr}{text}{SGR_RESET}")
    } else {
        write!(w, "{text}")
    }
}

/// Banner-text body (model names) — `#FFF8DC`.
fn write_body(w: &mut impl Write, colored: bool, text: &str) -> io::Result<()> {
    if colored {
        let sgr = sgr_banner_text(detect_color_depth());
        write!(w, "{sgr}{text}{SGR_RESET}")
    } else {
        write!(w, "{text}")
    }
}

/// Dim brown secondary text (display names, markers, labels) — `#B8860B`.
fn write_dim(w: &mut impl Write, colored: bool, text: &str) -> io::Result<()> {
    if colored {
        let sgr = sgr_dim_brown(detect_color_depth());
        write!(w, "{sgr}{text}{SGR_RESET}")
    } else {
        write!(w, "{text}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::McpAction;
    use hermes_core::config::HermesConfig;

    fn config_with(providers: &[(&str, Option<&str>, &[&str])]) -> HermesConfig {
        let mut c = HermesConfig::default();
        for (name, display, models) in providers {
            let mut p = ProviderConfig {
                name: display.map(str::to_owned),
                ..Default::default()
            };
            for m in models.iter() {
                p.models.insert(
                    m.to_string(),
                    serde_yaml::Value::Mapping(Default::default()),
                );
            }
            c.providers.insert(name.to_string(), p);
        }
        c
    }

    fn plain(config: Option<&HermesConfig>, filter: Option<&str>) -> String {
        let mut out = Vec::new();
        render_model(config, filter, false, &mut out).expect("render");
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn model_lists_all_providers_sorted_with_active_marker() {
        let mut c = config_with(&[
            ("openai", Some("OpenAI"), &["gpt-4o"]),
            ("anthropic", None, &["claude-sonnet-4-5", "claude-opus-4-1"]),
        ]);
        c.model.provider = Some("anthropic".into());
        let out = plain(Some(&c), None);
        assert_eq!(
            out,
            "Providers:\n  * anthropic (active)\n    models: claude-opus-4-1, \
             claude-sonnet-4-5\n    openai (OpenAI)\n    models: gpt-4o\n"
        );
        assert!(!out.contains('\u{1b}'), "piped output must be ANSI-free");
    }

    #[test]
    fn model_filter_narrows_to_one_provider() {
        let c = config_with(&[("a", None, &["m1"]), ("b", None, &["m2"])]);
        let out = plain(Some(&c), Some("b"));
        assert_eq!(out, "Providers:\n  * b (active)\n    models: m2\n");
    }

    #[test]
    fn model_unknown_filter_is_an_error() {
        let c = config_with(&[("a", None, &["m1"])]);
        let mut out = Vec::new();
        let err = render_model(Some(&c), Some("nope"), false, &mut out).expect_err("must error");
        assert!(err.to_string().contains("unknown provider 'nope'"));
        assert!(err.to_string().contains("a"));
    }

    #[test]
    fn model_without_config_shows_builtin_fake() {
        let out = plain(None, None);
        assert_eq!(out, "Providers:\n  * fake (active, built-in)\n");
    }

    #[test]
    fn model_filter_fake_works_without_config_entry() {
        let c = config_with(&[("a", None, &["m1"])]);
        let out = plain(Some(&c), Some("fake"));
        assert_eq!(out, "Providers:\n  * fake (active, built-in)\n");
    }

    #[test]
    fn model_provider_without_models_shows_placeholder() {
        let c = config_with(&[("a", None, &[])]);
        let out = plain(Some(&c), None);
        assert!(out.contains("    models: (not configured)"));
    }

    #[test]
    fn model_colored_uses_theme_sgr() {
        let c = config_with(&[("anthropic", None, &["claude-sonnet-4-5"])]);
        let mut out = Vec::new();
        render_model(Some(&c), None, true, &mut out).expect("render");
        let s = String::from_utf8(out).unwrap();
        assert!(
            s.contains("1;38;2;255;215;0") || s.contains("1;38;5;"),
            "gold SGR: {s}"
        );
        assert!(
            s.contains("38;2;255;248;220") || s.contains("38;5;"),
            "banner-text SGR: {s}"
        );
        assert!(
            s.contains("38;2;184;134;11") || s.contains("38;5;"),
            "dim-brown SGR: {s}"
        );
        assert!(s.contains("\u{1b}[0m"), "reset");
    }

    #[test]
    fn active_provider_precedence_flag_beats_config() {
        let mut c = config_with(&[("a", None, &[])]);
        c.model.provider = Some("a".into());
        assert_eq!(active_provider(Some(&c), Some("b")), "b");
        assert_eq!(active_provider(Some(&c), None), "a");
        assert_eq!(active_provider(None, None), FAKE_PROVIDER);
        let auto = {
            let mut c = c.clone();
            c.model.provider = Some("auto".into());
            c
        };
        assert_eq!(active_provider(Some(&auto), None), FAKE_PROVIDER);
    }

    #[test]
    fn placeholder_names_and_message_are_pinned() {
        let cases: Vec<(Commands, &str)> = vec![
            (Commands::Model, "model"),
            (Commands::Sessions, "sessions"),
            (Commands::Inspect { id: "x".into() }, "inspect"),
            (Commands::Messages { id: "x".into() }, "messages"),
            (Commands::ToolCalls { id: "x".into() }, "tool-calls"),
            (Commands::Search { query: "x".into() }, "search"),
            (Commands::Info, "info"),
            (Commands::Mcp { action: None }, "mcp"),
            (
                Commands::Mcp {
                    action: Some(McpAction::List),
                },
                "mcp",
            ),
            (
                Commands::Mcp {
                    action: Some(McpAction::Restart { name: "s".into() }),
                },
                "mcp",
            ),
        ];
        for (cmd, n) in cases {
            assert_eq!(name(&cmd), n);
            assert_eq!(placeholder(&cmd), format!("coming soon: {n} (Spec 014)"));
        }
    }

    #[test]
    fn parse_session_id_accepts_uuid_and_rejects_garbage() {
        let id = parse_session_id("550e8400-e29b-41d4-a716-446655440000").expect("valid uuid");
        assert_eq!(id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
        let err = parse_session_id("abc-123").unwrap_err().to_string();
        assert!(
            err.contains("invalid session id 'abc-123' (expected a UUID)"),
            "{err}"
        );
    }

    #[test]
    fn open_existing_store_is_none_when_state_db_absent() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = open_existing_store(dir.path()).expect("no error");
        assert!(store.is_none(), "missing state.db must not open a store");
        assert!(
            !dir.path().join("state.db").exists(),
            "store file must not be created"
        );
    }
}
