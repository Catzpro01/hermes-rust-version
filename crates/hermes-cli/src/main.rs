use clap::Parser;
use hermes_core::config::{load_config, resolve_hermes_home, SecretString};
use hermes_core::provider::{FakeProvider, HttpProvider, Provider};
use url::Url;

mod render;
mod repl;
mod session_menu;

#[derive(Debug, Parser)]
#[command(name = "hermes-rs", version, about = "Hermes Agent Rust rewrite")]
struct Args {
    /// Hermes home directory; defaults to HERMES_HOME or ~/.hermes.
    #[arg(long)]
    hermes_home: Option<std::path::PathBuf>,
    /// Provider for the first offline CLI slice.
    #[arg(long, default_value = "fake")]
    provider: String,
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
            if error.to_string().contains("SIGINT") {
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
    if args.provider != "fake" {
        anyhow::bail!(
            "provider '{}' is not available in the offline CLI slice; use --provider fake",
            args.provider
        );
    }
    let home = resolve_hermes_home(args.hermes_home.as_deref())?;
    // Validate an existing config even in fake mode, while keeping fake mode usable
    // with a disposable home that only contains state.db.
    if home.join("config.yaml").exists() {
        let _ = load_config(&home).map_err(|e| anyhow::anyhow!("Invalid config: {e}"))?;
    }
    let provider: Box<dyn Provider> = match args.provider.as_str() {
        "fake" => Box::new(FakeProvider),
        "openai" | "custom" => {
            let config = load_config(&home)?;
            let model = config
                .model
                .default
                .clone()
                .ok_or_else(|| anyhow::anyhow!("model.default is required"))?;
            let base_url = args
                .api_url
                .clone()
                .or_else(|| config.model.base_url.clone())
                .unwrap_or_else(|| "https://api.openai.com/".into());
            let key = std::env::var("OPENAI_API_KEY")
                .or_else(|_| std::env::var("HERMES_API_KEY"))
                .ok()
                .or_else(|| config.model.api_key.as_ref().map(|k| k.expose().to_owned()))
                .ok_or_else(|| {
                    anyhow::anyhow!("OPENAI_API_KEY is required for provider {}", args.provider)
                })?;
            Box::new(HttpProvider::new(
                Url::parse(&base_url)?,
                SecretString::from(key),
                model,
            ))
        }
        other => anyhow::bail!("unsupported provider '{other}' (use fake, openai, or custom)"),
    };
    repl::run_repl(&home, provider, args.resume).await
}
