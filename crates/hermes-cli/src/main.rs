use clap::Parser;
use hermes_core::config::load_config;
use hermes_core::config::resolve_hermes_home;
use hermes_core::provider::{FAKE_PROVIDER, Provider, ProviderRegistry};

mod output;
mod render;
mod repl;
mod session_menu;
mod streaming;
mod tui;

#[derive(Debug, Parser)]
#[command(name = "hermes-rs", version, about = "Hermes Agent Rust rewrite")]
struct Args {
    /// Hermes home directory; defaults to HERMES_HOME or ~/.hermes.
    #[arg(long)]
    hermes_home: Option<std::path::PathBuf>,
    /// Provider name from config.yaml; overrides model.provider.
    /// Defaults to model.provider, then to "fake" when neither is set.
    #[arg(long)]
    provider: Option<String>,
    /// Resume the most recently updated session.
    #[arg(long)]
    resume: bool,
    /// Override the OpenAI-compatible API base URL.
    #[arg(long)]
    api_url: Option<String>,
    /// Launch the Ratatui TUI dashboard instead of the readline REPL
    /// (Spec 012). Requires an interactive terminal.
    #[arg(long)]
    tui: bool,
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
