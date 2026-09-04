use hermes_core::AgentConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AgentConfig::default();
    println!("Hermes-RS bootstrap");
    println!("hermes_home={}", config.hermes_home);
    println!("status=architecture-bootstrap");
    Ok(())
}
