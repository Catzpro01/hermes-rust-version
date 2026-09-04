use clap::Parser;
use hermes_core::config::load_config;
use hermes_core::config::resolve_hermes_home;
use hermes_core::provider::{Provider, ProviderRegistry};

mod output;
mod render;
mod repl;
mod session_menu;

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
    let provider: Box<dyn Provider> = registry
        .select(
            args.provider.as_deref(),
            config_provider.as_deref(),
            args.api_url.as_deref(),
            config.as_ref(),
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    repl::run_repl(&home, provider, args.resume).await
}
