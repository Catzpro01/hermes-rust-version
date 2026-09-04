use clap::Parser;
use hermes_core::config::resolve_hermes_home;

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
}

#[tokio::main]
async fn main() {
    std::process::exit(match run().await {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("error: {error}");
            1
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
    repl::run_repl(&home).await
}
