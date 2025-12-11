use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod handlers;
mod server;
mod simctl;
mod storage;
mod xcode;

use xscape_common::AgentServerConfig;

#[derive(Parser)]
#[command(name = "xcode-agent")]
#[command(about = "Xcode build and simulator agent for ios-sim-launcher")]
#[command(version)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, env = "XCODE_AGENT_CONFIG")]
    config: Option<PathBuf>,

    /// Host to bind to
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Port to listen on
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Setup logging
    let log_level = if args.verbose { Level::DEBUG } else { Level::INFO };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Load configuration
    let config = if let Some(config_path) = &args.config {
        let content = std::fs::read_to_string(config_path)?;
        toml::from_str(&content)?
    } else {
        AgentServerConfig {
            host: args.host.clone(),
            port: args.port,
            ..Default::default()
        }
    };

    info!(
        "Starting xcode-agent v{} on {}:{}",
        env!("CARGO_PKG_VERSION"),
        config.host,
        config.port
    );

    // Initialize storage
    storage::init(&config.storage).await?;

    // Start server
    server::run(config).await?;

    Ok(())
}
