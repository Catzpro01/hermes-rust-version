use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use hermes_core::config::load_config;
use hermes_core::config::resolve_hermes_home;
use hermes_core::config::ConfigError;
use hermes_core::provider::{Provider, ProviderRegistry, FAKE_PROVIDER};

pub(crate) mod approval;
// Spec 017 T08 — /slash autocomplete + ghost text (REPL completer).
mod completion;
mod output;
mod render;
mod repl;
mod session_menu;
// Spec 017 T09 — session browse picker (spec §F verbatim strings).
mod session_picker;
mod status_bar;
mod streaming;
mod subcommands;
mod tui;
mod wizard;

#[derive(Debug, Parser)]
#[command(
    name = "hermes-rs",
    // Spec 014 T07: clap's built-in `--version` renders `hermes-rs <ver>`;
    // Python prints `Hermes Agent v0.21.0 (2026.8.31) · upstream …` plus
    // install facts, so the flag is handled manually (see `Args::version`).
    disable_version_flag = true,
    about = "Hermes Agent Rust rewrite - Rust implementation of Hermes Agent v0.21.0",
    long_about = "Hermes Agent Rust rewrite - Rust implementation of Hermes Agent

Usage: hermes-rs [OPTIONS] [COMMAND]

Commands: model, tools, sessions, inspect, messages, tool-calls, search, info, mcp, setup, help, version
Mirrors Python Hermes Agent where implemented."
)]
struct Args {
    /// Print version information and exit (same output as `hermes version`).
    #[arg(short = 'V', long, global = true)]
    version: bool,

    /// Hermes home directory; defaults to HERMES_HOME or ~/.hermes.
    #[arg(long, global = true)]
    hermes_home: Option<std::path::PathBuf>,

    /// Provider name from config.yaml; overrides model.provider.
    /// Defaults to model.provider, then to "fake" when neither is set.
    #[arg(long, global = true)]
    provider: Option<String>,

    /// Resume the most recently updated session.
    #[arg(short = 'r', long, aliases = ["continue", "c"])]
    resume: bool,

    /// Resume a specific session by id (see `hermes sessions browse`).
    #[arg(long)]
    resume_id: Option<String>,

    /// Override the model name for this invocation.
    #[arg(short = 'm', long)]
    model: Option<String>,

    /// One-shot mode: send a single prompt and print response.
    #[arg(short = 'z', long)]
    oneshot: Option<String>,

    /// Comma-separated toolsets to enable.
    #[arg(short = 't', long)]
    toolsets: Option<String>,

    /// Preload one or more skills.
    #[arg(short = 's', long)]
    skills: Option<String>,

    /// Run in an isolated git worktree.
    #[arg(short = 'w', long)]
    worktree: bool,

    /// Bypass all dangerous command approval prompts.
    #[arg(long)]
    yolo: bool,

    /// Troubleshooting mode: disable customizations.
    #[arg(long)]
    safe_mode: bool,

    /// Override the OpenAI-compatible API base URL.
    #[arg(long, global = true)]
    api_url: Option<String>,

    /// Run shell tools unconfined (Spec 007b: the process-level sandbox is
    /// on by default). Overrides `sandbox:` in config.yaml.
    #[arg(long, global = true)]
    no_sandbox: bool,

    /// Launch the Ratatui TUI dashboard instead of the readline REPL
    /// (Spec 012). Requires an interactive terminal.
    #[arg(long, global = true)]
    tui: bool,

    /// Shell subcommand (Spec 014). Omitted -> interactive REPL
    /// (zero regression: the pre-014 default behavior).
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Spec 014: shell-accessible subcommands, mirroring the Hermes Python
/// `hermes <subcommand>` surface. Data sources already exist (ProviderRegistry,
/// SessionStore, `search_messages`, theme/status-bar, McpServerRegistry);
/// every variant is wired in `subcommands::run` (T02-T07).
#[derive(Debug, Subcommand)]
enum Commands {
    /// Interactive setup wizard (optionally one section:
    /// model|terminal|gateway|tools)
    Setup {
        /// Jump straight to a section (setup.py: `hermes setup model|terminal|gateway|tools|agent`).
        section: Option<String>,
    },
    /// Show Hermes version and install information
    Version,

    /// Pick provider + model (interactive) or list configured providers (piped / --provider)
    Model,
    /// Enable/disable toolsets (interactive checklist) or list them (piped)
    Tools,
    /// List all chat sessions (or `browse` them interactively)
    Sessions {
        #[command(subcommand)]
        action: Option<SessionsAction>,
    },
    /// Inspect a session's metadata
    Inspect { id: String },
    /// Show messages in a session
    Messages { id: String },
    /// Show tool calls in a session
    ToolCalls { id: String },
    /// Search message history
    Search { query: String },
    /// Show provider & context info
    Info,
    /// Show MCP server status
    Mcp {
        #[command(subcommand)]
        action: Option<McpAction>,
    },
}

/// Nested actions for `hermes sessions` (Spec 017 T09: `browse` is the
/// interactive picker from spec §F; a bare `sessions` keeps listing).
#[derive(Debug, Subcommand)]
enum SessionsAction {
    /// Browse sessions interactively (↑↓ navigate, type to filter, `d` delete)
    Browse,
}

/// Nested actions for `hermes mcp` (parity with the REPL's `/mcp`).
#[derive(Debug, Subcommand)]
enum McpAction {
    /// List MCP servers and their status
    List,
    /// Restart one MCP server
    Restart { name: String },
}

/// What a bare `hermes-rs` run does when the resolved Hermes home is missing.
///
/// Python Hermes onboards on first run instead of failing; the Rust port used
/// to hard-error with `HomeNotFound`, so a fresh machine had to know about
/// `hermes-rs setup` before the binary would start at all.
#[derive(Debug, PartialEq, Eq)]
enum FirstRun {
    /// The home exists: nothing to do.
    Ready,
    /// Interactive terminal: create the home and run the first-time wizard.
    Onboard,
    /// Piped/scripted: actionable error, and never write anything.
    Reject,
}

/// Pure first-run decision (unit-tested below). Onboarding needs a terminal
/// because the wizard prompts, and a piped caller must neither block forever on
/// a prompt nor discover a directory it never asked for.
fn first_run_action(home_exists: bool, interactive: bool) -> FirstRun {
    if home_exists {
        FirstRun::Ready
    } else if interactive {
        FirstRun::Onboard
    } else {
        FirstRun::Reject
    }
}

/// Resolves the Hermes home, running the first-time setup wizard when the home
/// does not exist yet and this is an interactive session.
///
/// `resolve_hermes_home` in `hermes-core` deliberately stays strict (its
/// `HomeNotFound` contract is pinned by core tests and every read-only
/// inspection subcommand relies on it), so the onboarding decision lives here
/// at the CLI boundary and only on the REPL/TUI path: subcommands keep their
/// actionable error rather than writing during an inspection.
fn resolve_home_or_onboard(explicit: Option<&Path>) -> anyhow::Result<PathBuf> {
    let path = match resolve_hermes_home(explicit) {
        Ok(home) => return Ok(home),
        Err(ConfigError::HomeNotFound { path }) => path,
        Err(e) => anyhow::bail!("{e}"),
    };
    use std::io::IsTerminal;
    let interactive = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
    match first_run_action(path.is_dir(), interactive) {
        FirstRun::Ready => Ok(path),
        FirstRun::Reject => Err(subcommands::missing_home_error(&path)),
        FirstRun::Onboard => {
            // `run_setup` prints the verbatim Python first-time notice
            // (`FIRST_TIME`), asks the mode question and creates the home when
            // it applies. ESC -> `Setup cancelled.` with nothing written
            // (invariant 3); Ctrl+C surfaces "interrupted" so `main()` maps it
            // to exit 130 (invariant 2).
            if let Err(e) = wizard::setup::run_setup(&path, None) {
                anyhow::bail!("{e}");
            }
            // A cancelled run (or a mode that answers nothing) writes no
            // config, so the directory can still be missing; the REPL needs it
            // for `state.db`. Creating it here is the only write this path
            // makes on its own.
            if !path.is_dir() {
                std::fs::create_dir_all(&path).map_err(|e| {
                    anyhow::anyhow!("failed to create Hermes home {}: {e}", path.display())
                })?;
            }
            Ok(path)
        }
    }
}

#[tokio::main]
async fn main() {
    std::process::exit(match run().await {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("error: {error}");
            if error.chain().any(|cause| {
                cause.to_string().contains("interrupted") || cause.to_string().contains("SIGINT")
            }) {
                130
            } else {
                1
            }
        }
    });
}
async fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .without_time()
        .try_init()
        .ok();
    let args = Args::parse();
    // Spec 014 (T07): `--version` (any position) wins over everything else
    // and never touches config, state, provider or network.
    if args.version {
        return subcommands::run(&Commands::Version, &args).await;
    }
    // Spec 014 (T02): a subcommand runs to completion and exits before any
    // provider resolution or session creation (it loads home/config itself;
    // see subcommands::run). No subcommand -> REPL/TUI path exactly as before
    // (zero regression).
    if let Some(cmd) = &args.command {
        return subcommands::run(cmd, &args).await;
    }
    // Spec 012: TUI requires an interactive terminal. Rejecting a piped stdin
    // here prevents crossterm raw-mode from hanging/crashing smoke/E2E tests
    // that spawn the binary with piped input, and stops `echo x | hermes-rs
    // --tui` from silently doing the wrong thing.
    if args.tui {
        use std::io::IsTerminal;
        if !std::io::stdin().is_terminal() {
            anyhow::bail!("--tui requires an interactive terminal");
        }
    }
    let home = resolve_home_or_onboard(args.hermes_home.as_deref())?;
    // Load once. A missing config.yaml is allowed so the offline `fake` slice
    // stays usable with a disposable home containing only state.db.
    let config = if home.join("config.yaml").exists() {
        Some(load_config(&home).map_err(|e| anyhow::anyhow!("Invalid config: {e}"))?)
    } else {
        None
    };
    let registry = match &config {
        Some(config) => ProviderRegistry::from_config(config),
        None => ProviderRegistry::offline(),
    };
    // `model.provider: auto` means "not chosen yet", so it must not be treated
    // as a provider name.
    let config_provider = config
        .as_ref()
        .and_then(|c| c.model.provider.clone())
        .filter(|p| p != "auto");
    // Startup resolves with fallback (config `model.fallback_chain`), so a
    // single `Box<dyn Provider>` is handed to the REPL whether or not a chain
    // was configured. The mid-session `/provider <name>` command still uses the
    // single-provider `select`, letting a user-explicit choice bypass fallback.
    let provider: Box<dyn Provider> = registry
        .select_with_fallback(
            args.provider.as_deref(),
            config_provider.as_deref(),
            args.api_url.as_deref(),
            config.as_ref(),
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    // The active provider's name mirrors the select precedence above and feeds
    // the mid-session `/provider` command (available list + active marker).
    let provider_name = args
        .provider
        .clone()
        .or_else(|| config_provider.clone())
        .unwrap_or_else(|| FAKE_PROVIDER.to_owned());
    if args.tui {
        tui::run_tui(&home, provider, provider_name, config, args.no_sandbox).await
    } else {
        repl::run_repl(
            &home,
            provider,
            provider_name,
            registry,
            config,
            repl::ReplOptions {
                base_url_override: args.api_url,
                resume: args.resume,
                resume_id: args.resume_id,
                no_sandbox: args.no_sandbox,
            },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parse(args: &[&str]) -> Args {
        Args::try_parse_from(args).expect("args must parse")
    }

    #[test]
    fn no_subcommand_means_repl_default() {
        assert!(parse(&["hermes-rs"]).command.is_none());
        assert!(parse(&["hermes-rs", "--provider", "fake"])
            .command
            .is_none());
        assert!(parse(&["hermes-rs", "--tui"]).command.is_none());
    }

    /// First-run onboarding is interactive-only and only for a missing home:
    /// an existing home never re-prompts, and a piped caller never blocks on a
    /// prompt nor gets a directory created behind its back.
    #[test]
    fn first_run_action_matrix() {
        assert_eq!(first_run_action(true, true), FirstRun::Ready);
        assert_eq!(first_run_action(true, false), FirstRun::Ready);
        assert_eq!(first_run_action(false, true), FirstRun::Onboard);
        assert_eq!(first_run_action(false, false), FirstRun::Reject);
    }

    #[test]
    fn every_subcommand_parses() {
        match parse(&["hermes-rs", "model"]).command {
            Some(Commands::Model) => {}
            other => panic!("expected Model, got {other:?}"),
        }
        match parse(&["hermes-rs", "sessions"]).command {
            Some(Commands::Sessions { action: None }) => {}
            other => panic!("expected Sessions, got {other:?}"),
        }
        match parse(&["hermes-rs", "sessions", "browse"]).command {
            Some(Commands::Sessions {
                action: Some(SessionsAction::Browse),
            }) => {}
            other => panic!("expected Sessions browse, got {other:?}"),
        }
        match parse(&["hermes-rs", "inspect", "abc-123"]).command {
            Some(Commands::Inspect { id }) if id == "abc-123" => {}
            other => panic!("expected Inspect, got {other:?}"),
        }
        match parse(&["hermes-rs", "messages", "s1"]).command {
            Some(Commands::Messages { id }) if id == "s1" => {}
            other => panic!("expected Messages, got {other:?}"),
        }
        match parse(&["hermes-rs", "tool-calls", "s1"]).command {
            Some(Commands::ToolCalls { id }) if id == "s1" => {}
            other => panic!("expected ToolCalls, got {other:?}"),
        }
        match parse(&["hermes-rs", "search", "deploy"]).command {
            Some(Commands::Search { query }) if query == "deploy" => {}
            other => panic!("expected Search, got {other:?}"),
        }
        match parse(&["hermes-rs", "info"]).command {
            Some(Commands::Info) => {}
            other => panic!("expected Info, got {other:?}"),
        }
        match parse(&["hermes-rs", "mcp"]).command {
            Some(Commands::Mcp { action: None }) => {}
            other => panic!("expected bare Mcp, got {other:?}"),
        }
    }

    #[test]
    fn mcp_actions_parse() {
        match parse(&["hermes-rs", "mcp", "list"]).command {
            Some(Commands::Mcp {
                action: Some(McpAction::List),
            }) => {}
            other => panic!("expected Mcp List, got {other:?}"),
        }
        match parse(&["hermes-rs", "mcp", "restart", "srv-1"]).command {
            Some(Commands::Mcp {
                action: Some(McpAction::Restart { name }),
            }) if name == "srv-1" => {}
            other => panic!("expected Mcp Restart, got {other:?}"),
        }
    }

    #[test]
    fn global_flags_parse_before_and_after_subcommand() {
        let a = parse(&["hermes-rs", "--provider", "fake", "model"]);
        assert_eq!(a.provider.as_deref(), Some("fake"));
        assert!(matches!(a.command, Some(Commands::Model)));

        let b = parse(&["hermes-rs", "model", "--provider", "fake"]);
        assert_eq!(b.provider.as_deref(), Some("fake"));

        let c = parse(&["hermes-rs", "info", "--api-url", "http://x"]);
        assert_eq!(c.api_url.as_deref(), Some("http://x"));

        let d = parse(&["hermes-rs", "--hermes-home", "/tmp/h", "sessions"]);
        assert!(d.hermes_home.is_some());

        let e = parse(&["hermes-rs", "search", "q", "--tui"]);
        assert!(e.tui);
    }

    #[test]
    fn version_flag_and_subcommand_parse() {
        assert!(parse(&["hermes-rs", "--version"]).version);
        assert!(parse(&["hermes-rs", "-V"]).version);
        // Global: also accepted after a subcommand.
        let a = parse(&["hermes-rs", "info", "--version"]);
        assert!(a.version && matches!(a.command, Some(Commands::Info)));
        assert!(matches!(
            parse(&["hermes-rs", "version"]).command,
            Some(Commands::Version)
        ));
        assert!(!parse(&["hermes-rs"]).version);
    }
}
