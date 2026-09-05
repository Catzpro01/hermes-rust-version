use clap::{Parser, Subcommand};
use hermes_core::config::load_config;
use hermes_core::config::resolve_hermes_home;
use hermes_core::provider::{Provider, ProviderRegistry, FAKE_PROVIDER};

mod output;
mod render;
mod repl;
mod session_menu;
mod status_bar;
mod streaming;
mod tui;

#[derive(Debug, Parser)]
#[command(name = "hermes-rs", version, about = "Hermes Agent Rust rewrite")]
struct Args {
    /// Hermes home directory; defaults to HERMES_HOME or ~/.hermes.
    #[arg(long, global = true)]
    hermes_home: Option<std::path::PathBuf>,
    /// Provider name from config.yaml; overrides model.provider.
    /// Defaults to model.provider, then to "fake" when neither is set.
    #[arg(long, global = true)]
    provider: Option<String>,
    /// Resume the most recently updated session.
    #[arg(long)]
    resume: bool,
    /// Override the OpenAI-compatible API base URL.
    #[arg(long, global = true)]
    api_url: Option<String>,
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
/// tickets 02-07 wire them up. T01 keeps every variant a static placeholder.
#[derive(Debug, Subcommand)]
enum Commands {
    /// List available models for the active provider
    Model,
    /// List all chat sessions
    Sessions,
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

/// Nested actions for `hermes mcp` (parity with the REPL's `/mcp`).
#[derive(Debug, Subcommand)]
enum McpAction {
    /// List MCP servers and their status
    List,
    /// Restart one MCP server
    Restart { name: String },
}

/// Shell-verbatim name of a subcommand (matches clap's kebab-case rendering).
fn subcommand_name(cmd: &Commands) -> &'static str {
    match cmd {
        Commands::Model => "model",
        Commands::Sessions => "sessions",
        Commands::Inspect { .. } => "inspect",
        Commands::Messages { .. } => "messages",
        Commands::ToolCalls { .. } => "tool-calls",
        Commands::Search { .. } => "search",
        Commands::Info => "info",
        Commands::Mcp { .. } => "mcp",
    }
}

/// T01 placeholder for the Spec 014 subcommands (implemented in T02-T07).
/// Static output only - no state, provider or network access - so the
/// CLI-boundary sanitization/redaction contract is trivially satisfied.
fn subcommand_placeholder(cmd: &Commands) -> String {
    format!("coming soon: {} (Spec 014)", subcommand_name(cmd))
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
    // Spec 014 (T01): a subcommand runs to completion and exits before any
    // home/config/provider/TTY work. No subcommand -> REPL/TUI path exactly
    // as before (zero regression).
    if let Some(cmd) = &args.command {
        println!("{}", subcommand_placeholder(cmd));
        return Ok(());
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
    let home = resolve_hermes_home(args.hermes_home.as_deref())?;
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
        tui::run_tui(&home, provider, provider_name, config).await
    } else {
        repl::run_repl(
            &home,
            provider,
            provider_name,
            registry,
            config,
            args.api_url,
            args.resume,
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

    #[test]
    fn every_subcommand_parses() {
        match parse(&["hermes-rs", "model"]).command {
            Some(Commands::Model) => {}
            other => panic!("expected Model, got {other:?}"),
        }
        match parse(&["hermes-rs", "sessions"]).command {
            Some(Commands::Sessions) => {}
            other => panic!("expected Sessions, got {other:?}"),
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
        for (cmd, name) in cases {
            assert_eq!(subcommand_name(&cmd), name);
            assert_eq!(
                subcommand_placeholder(&cmd),
                format!("coming soon: {name} (Spec 014)")
            );
        }
    }
}
