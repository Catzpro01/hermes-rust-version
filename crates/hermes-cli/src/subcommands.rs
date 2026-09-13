//! Spec 014: shell subcommands.
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
use crate::tui::welcome::{sgr_banner_text, sgr_bold_gold, sgr_dim_brown, SGR_RESET, VERSION_LABEL};
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

/// `hermes tools` (piped): `  [x] key  label — tools` per catalog entry.
pub fn render_tools(config: Option<&HermesConfig>, w: &mut impl Write) -> anyhow::Result<()> {
    use crate::wizard::catalog::{DEFAULT_OFF_TOOLSETS, TOOLSETS};
    let enabled = config.and_then(|c| c.tools.as_ref()).map(|t| t.enabled_toolsets.clone());
    writeln!(w, "Toolsets:")?;
    for t in TOOLSETS {
        let on = match &enabled {
            Some(list) => list.iter().any(|k| k == t.key),
            None => !DEFAULT_OFF_TOOLSETS.contains(&t.key),
        };
        let mark = if on { "[x]" } else { "[ ]" };
        writeln!(w, "  {mark} {}  {} — {}", t.key, t.label, t.tools)?;
    }
    Ok(())
}

/// Hermes home for `setup`: explicit flag → `HERMES_HOME` → `~/.hermes`,
/// without requiring the directory to exist yet.
fn setup_home(explicit: Option<&Path>) -> PathBuf {
    if let Some(p) = explicit {
        return p.to_path_buf();
    }
    if let Some(v) = std::env::var_os("HERMES_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(v);
    }
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join(".hermes"))
        .unwrap_or_else(|| PathBuf::from(".hermes"))
}

/// Run one subcommand to completion (never enters the REPL/TUI).
pub(crate) async fn run(cmd: &Commands, args: &Args) -> anyhow::Result<()> {
    // `version` (T07) is static: it must work even when config.yaml is
    // invalid or the home does not exist, so it never loads anything.
    if matches!(cmd, Commands::Version) {
        let mut out = io::stdout().lock();
        render_version(args.hermes_home.as_deref(), &mut out)?;
        return out.flush().map_err(Into::into);
    }
    // `setup` (Spec 017 T05) may run before the home directory exists
    // (first-time setup): resolve the path leniently and let the wizard
    // create it on apply. Config validity is irrelevant here (the wizard
    // edits the raw YAML mapping and backs the old file up first).
    if let Commands::Setup { section } = cmd {
        let home = setup_home(args.hermes_home.as_deref());
        let section = match section.as_deref() {
            None => None,
            Some(tok) => match crate::wizard::setup::Section::parse(tok) {
                Some(s) => Some(s),
                None => anyhow::bail!(
                    "unknown setup section '{tok}' (expected model|terminal|gateway|tools)"
                ),
            },
        };
        return match crate::wizard::setup::run_setup(&home, section) {
            Ok(_) => Ok(()),
            Err(e) => anyhow::bail!("{e}"),
        };
    }
    let (home, config) = load_home_config(args.hermes_home.as_deref())?;
    match cmd {
        Commands::Version => unreachable!("handled above"),
        Commands::Setup { .. } => unreachable!("handled above"),
        Commands::Model if args.provider.is_none() && io::stdin().is_terminal() && io::stdout().is_terminal() => {
            // Spec 017 T06 — interactive `hermes model` IS the wizard's
            // `Model & Provider` section (setup.py delegates to `cmd_model`:
            // one code path, spec §C.3). 39-entry verbatim catalog picker,
            // then URL / key_env / model, atomic write + backup.
            match crate::wizard::setup::run_setup(&home, Some(crate::wizard::setup::Section::Model)) {
                Ok(_) => {}
                Err(e) => anyhow::bail!("{e}"),
            }
        }
        Commands::Tools if io::stdin().is_terminal() && io::stdout().is_terminal() => {
            // Spec 017 T07 — interactive `hermes tools` IS the wizard's
            // `Tools` section (26-toolset checklist, spec §C.6).
            match crate::wizard::setup::run_setup(&home, Some(crate::wizard::setup::Section::Tools)) {
                Ok(_) => {}
                Err(e) => anyhow::bail!("{e}"),
            }
        }
        Commands::Tools => {
            // Piped: plain, ANSI-free listing of the toolset catalog with the
            // effective on/off state (config `tools.enabled_toolsets`, else
            // catalog minus `_DEFAULT_OFF_TOOLSETS`).
            let mut out = io::stdout().lock();
            render_tools(config.as_ref(), &mut out)?;
            out.flush()?;
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
            // Spec 014 T06: provider & context info, read-only. The first
            // line mirrors the REPL's `/info` accounting for a fresh session
            // (no turns in flight from the shell), followed by shell-only
            // facts (home, config presence, session count).
            let active = active_provider(config.as_ref(), args.provider.as_deref());
            let ctx = crate::repl::resolve_context(config.as_ref(), &active);
            let sessions = match open_existing_store(&home)? {
                Some(store) => store.list().map(|s| s.len()).ok(),
                None => Some(0),
            };
            let mut out = io::stdout().lock();
            render_info(
                &home,
                config.as_ref(),
                &active,
                &ctx,
                sessions,
                &mut out,
            )?;
            out.flush()?;
        }
        Commands::Mcp { action } => {
            // Spec 014 T06: MCP server status from config.yaml. The shell
            // never spawns an MCP child (no new execution surface outside the
            // REPL), so the status column reads `configured` instead of the
            // REPL's live `connected`/`down`; the row layout is the REPL's.
            let mut out = io::stdout().lock();
            render_mcp(config.as_ref(), action.as_ref(), &mut out)?;
            out.flush()?;
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

/// Shell-verbatim name of a subcommand (matches clap's kebab-case rendering).
/// Pinned by unit tests as the contract between clap and the docs.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn name(cmd: &Commands) -> &'static str {
    match cmd {
        Commands::Model => "model",
        Commands::Tools => "tools",
        Commands::Sessions => "sessions",
        Commands::Inspect { .. } => "inspect",
        Commands::Messages { .. } => "messages",
        Commands::ToolCalls { .. } => "tool-calls",
        Commands::Search { .. } => "search",
        Commands::Info => "info",
        Commands::Mcp { .. } => "mcp",
        Commands::Setup { .. } => "setup",
        Commands::Version => "version",
    }
}

/// `hermes version` / `--version` (T07). Python prints
/// `Hermes Agent v0.21.0 (2026.8.31) · upstream 63279301` followed by install
/// facts; the Rust port prints the same label shape (`VERSION_LABEL`, shared
/// with the banner) plus the facts that exist for a cargo build. Static:
/// no config, state, provider or network access.
pub fn render_version(home_flag: Option<&Path>, w: &mut impl Write) -> anyhow::Result<()> {
    writeln!(w, "{VERSION_LABEL}")?;
    let exe = std::env::current_exe().ok();
    let install_dir = exe
        .as_deref()
        .and_then(Path::parent)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "unknown".to_owned());
    writeln!(w, "Install directory: {install_dir}")?;
    writeln!(w, "Install method: cargo")?;
    writeln!(w, "Crate version: {}", env!("CARGO_PKG_VERSION"))?;
    // Home is reported only if it resolves; never an error from `version`.
    if let Ok(home) = resolve_hermes_home(home_flag) {
        writeln!(w, "Hermes home: {}", home.display())?;
    }
    Ok(())
}

/// `hermes info` (T06) body. Line 1 has the exact shape of the REPL's
/// `/info` for a fresh session; the trailing lines are shell-only facts.
/// Pure over its inputs (unit-testable).
pub(crate) fn render_info(
    home: &Path,
    config: Option<&HermesConfig>,
    active: &str,
    ctx: &crate::repl::ResolvedContext,
    sessions: Option<usize>,
    w: &mut impl Write,
) -> anyhow::Result<()> {
    let limit = ctx
        .limit
        .map(|l| l.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let compression = if !ctx.compression_enabled {
        "off".to_owned()
    } else {
        match ctx.compression_target {
            Some(t) => format!("on (target ~{t} tokens)"),
            None => "on (no target)".to_owned(),
        }
    };
    writeln!(
        w,
        "provider: {active} | estimated context: ~0 tokens | limit: {limit} | window: 0/0 turns sent | pinned: 0 | compression: {compression}"
    )?;
    writeln!(w, "Hermes Home: {}", home.display())?;
    match config {
        Some(cfg) => {
            writeln!(w, "Active provider: {active}")?;
            writeln!(w, "Providers configured: {}", cfg.providers.len())?;
            if let Some(model) = &cfg.model.provider {
                writeln!(w, "Model provider: {model}")?;
            }
            writeln!(w, "MCP servers configured: {}", cfg.mcp_servers.len())?;
        }
        None => {
            writeln!(w, "Active provider: {active} (built-in)")?;
            writeln!(w, "No config.yaml found")?;
        }
    }
    // Spec 007: the boundary shell tools would run inside from this cwd
    // (same resolution as the REPL; names/numbers only, never env values).
    let root = std::env::current_dir().unwrap_or_default();
    let sandbox = hermes_core::tools::SandboxPolicy::from_config(
        config.and_then(|c| c.sandbox.as_ref()),
        &root,
    );
    writeln!(w, "{}", sandbox.summary())?;
    match sessions {
        Some(n) => writeln!(w, "Sessions: {n}")?,
        None => writeln!(w, "Sessions: unknown")?,
    }
    Ok(())
}

/// `hermes mcp [list|restart <name>]` (T06) body. Rows use the REPL's
/// `/mcp` layout (`{:<12} {:<10} {} tool(s) ({} mode)`); the shell has no
/// live child so status is `configured` and the tool count is unknown
/// (`?`). `restart` is REPL-only: the shell prints a clear pointer, and an
/// unknown name is an error (non-zero exit), as in the REPL.
pub(crate) fn render_mcp(
    config: Option<&HermesConfig>,
    action: Option<&crate::McpAction>,
    w: &mut impl Write,
) -> anyhow::Result<()> {
    let servers = config.map(|c| &c.mcp_servers);
    let none = servers.map(|s| s.is_empty()).unwrap_or(true);
    match action {
        None | Some(crate::McpAction::List) => {
            if none {
                writeln!(w, "no MCP servers connected (add `mcp_servers:` to config.yaml)")?;
                return Ok(());
            }
            let servers = servers.expect("non-empty");
            let mut names: Vec<&String> = servers.keys().collect();
            names.sort();
            writeln!(w, "MCP servers:")?;
            for name in names {
                let mode = if servers[name].confirm { "confirm" } else { "auto" };
                writeln!(w, "  {:<12} {:<10} ? tool(s) ({} mode)", name, "configured", mode)?;
            }
        }
        Some(crate::McpAction::Restart { name }) => {
            let known = servers.map(|s| s.contains_key(name)).unwrap_or(false);
            if !known {
                anyhow::bail!("no MCP server named '{name}'");
            }
            writeln!(
                w,
                "mcp[{name}]: restart is only available inside the REPL (`/mcp restart {name}`); the shell never spawns MCP servers"
            )?;
        }
    }
    Ok(())
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
    fn subcommand_names_are_pinned() {
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
            (Commands::Setup { section: None }, "setup"),
            (Commands::Version, "version"),
        ];
        for (cmd, n) in cases {
            assert_eq!(name(&cmd), n);
        }
    }

    #[test]
    fn tools_listing_follows_defaults_then_config() {
        let mut out = Vec::new();
        render_tools(None, &mut out).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(s.starts_with("Toolsets:\n"), "{s}");
        assert!(s.contains("  [x] web  🔍 Web Search & Scraping — web_search, web_extract\n"), "{s}");
        assert!(s.contains("  [ ] spotify  "), "{s}");
        assert_eq!(s.lines().count(), 27, "{s}");

        let c = HermesConfig {
            tools: Some(hermes_core::config::ToolsConfig {
                enabled_toolsets: vec!["spotify".into()],
            }),
            ..HermesConfig::default()
        };
        let mut out = Vec::new();
        render_tools(Some(&c), &mut out).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("  [x] spotify  ") && s.contains("  [ ] web  "), "{s}");
    }

    #[test]
    fn version_renders_label_and_never_touches_home() {
        let dir = tempfile::TempDir::new().unwrap();
        let home = dir.path().join("does-not-exist");
        let mut out = Vec::new();
        render_version(Some(&home), &mut out).expect("render");
        let s = String::from_utf8(out).unwrap();
        assert!(s.starts_with(VERSION_LABEL), "{s}");
        assert!(s.contains("Install method: cargo"), "{s}");
        assert!(s.contains("Crate version: "), "{s}");
        assert!(!s.contains('\u{1b}'), "plain text: {s}");
        assert!(!home.exists(), "version must not create the home");
    }

    #[test]
    fn info_first_line_matches_repl_shape() {
        let c = config_with(&[("a", None, &["m1"])]);
        let ctx = crate::repl::resolve_context(Some(&c), "a");
        let mut out = Vec::new();
        render_info(Path::new("/tmp/h"), Some(&c), "a", &ctx, Some(2), &mut out).expect("render");
        let s = String::from_utf8(out).unwrap();
        let first = s.lines().next().unwrap();
        assert_eq!(
            first,
            "provider: a | estimated context: ~0 tokens | limit: none | window: 0/0 turns sent | pinned: 0 | compression: off"
        );
        assert!(s.contains("Hermes Home: /tmp/h\n"), "{s}");
        assert!(s.contains("Providers configured: 1\n"), "{s}");
        assert!(s.contains("MCP servers configured: 0\n"), "{s}");
        assert!(s.contains("sandbox: off (inherit)\n"), "{s}");
        assert!(s.contains("Sessions: 2\n"), "{s}");
    }

    #[test]
    fn info_reports_sandbox_policy_without_env_values() {
        let mut c = config_with(&[]);
        c.sandbox = Some(hermes_core::config::SandboxConfig {
            enabled: true,
            network: Some("deny".into()),
            cpu_seconds: Some(5),
            ..Default::default()
        });
        let ctx = crate::repl::resolve_context(Some(&c), FAKE_PROVIDER);
        let mut out = Vec::new();
        render_info(Path::new("/tmp/h"), Some(&c), FAKE_PROVIDER, &ctx, Some(0), &mut out)
            .expect("render");
        let s = String::from_utf8(out).unwrap();
        let line = s.lines().find(|l| l.starts_with("sandbox: on")).expect("sandbox line");
        assert!(line.contains("cpu=5s") && line.contains("network=deny"), "{line}");
        assert!(line.contains("env=PATH,HOME"), "{line}");
        // Names only: the actual PATH value must not be printed.
        if let Ok(path) = std::env::var("PATH") {
            assert!(!s.contains(&path), "env value leaked: {s}");
        }
    }

    #[test]
    fn info_without_config_reports_builtin_fake() {
        let ctx = crate::repl::resolve_context(None, FAKE_PROVIDER);
        let mut out = Vec::new();
        render_info(Path::new("/tmp/h"), None, FAKE_PROVIDER, &ctx, Some(0), &mut out)
            .expect("render");
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("Active provider: fake (built-in)\n"), "{s}");
        assert!(s.contains("No config.yaml found\n"), "{s}");
    }

    fn config_with_mcp(names: &[(&str, bool)]) -> HermesConfig {
        let mut c = config_with(&[]);
        for (n, confirm) in names {
            c.mcp_servers.insert(
                (*n).to_owned(),
                hermes_core::config::McpServerConfig {
                    command: "true".to_owned(),
                    confirm: *confirm,
                    ..Default::default()
                },
            );
        }
        c
    }

    #[test]
    fn mcp_list_uses_repl_row_layout_sorted() {
        let c = config_with_mcp(&[("zeta", true), ("alpha", false)]);
        let mut out = Vec::new();
        render_mcp(Some(&c), None, &mut out).expect("render");
        let s = String::from_utf8(out).unwrap();
        assert_eq!(
            s,
            "MCP servers:\n  alpha        configured ? tool(s) (auto mode)\n  zeta         configured ? tool(s) (confirm mode)\n"
        );
        let mut out = Vec::new();
        render_mcp(Some(&c), Some(&McpAction::List), &mut out).expect("render");
        assert_eq!(String::from_utf8(out).unwrap(), s, "`mcp` and `mcp list` are identical");
    }

    #[test]
    fn mcp_without_servers_matches_repl_message() {
        for cfg in [None, Some(config_with(&[]))] {
            let mut out = Vec::new();
            render_mcp(cfg.as_ref(), None, &mut out).expect("render");
            assert_eq!(
                String::from_utf8(out).unwrap(),
                "no MCP servers connected (add `mcp_servers:` to config.yaml)\n"
            );
        }
    }

    #[test]
    fn mcp_restart_points_to_repl_and_rejects_unknown_name() {
        let c = config_with_mcp(&[("srv", true)]);
        let mut out = Vec::new();
        render_mcp(
            Some(&c),
            Some(&McpAction::Restart {
                name: "srv".into(),
            }),
            &mut out,
        )
        .expect("render");
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("mcp[srv]: restart is only available inside the REPL"), "{s}");
        let err = render_mcp(
            Some(&c),
            Some(&McpAction::Restart {
                name: "nope".into(),
            }),
            &mut Vec::new(),
        )
        .unwrap_err()
        .to_string();
        assert_eq!(err, "no MCP server named 'nope'");
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
