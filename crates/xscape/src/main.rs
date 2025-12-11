use anyhow::Result;
use clap::Parser;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

mod agent_client;
mod cli;
mod config;
mod project;
mod tui;
mod vm;

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let log_level = if cli.verbose { Level::DEBUG } else { Level::INFO };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Load config
    let config = config::load_config(&cli.config)?;

    // Get agent URL (from CLI override or config)
    let agent_url = cli.agent_url.clone().unwrap_or_else(|| {
        match config.agent.mode {
            xscape_common::AgentMode::Remote => {
                format!("http://{}:{}", config.agent.remote_host, config.agent.remote_port)
            }
            xscape_common::AgentMode::LocalVm => {
                format!("http://127.0.0.1:{}", config.vm.agent_port)
            }
        }
    });

    // Create agent client
    let client = agent_client::AgentClient::new(&agent_url, config.agent.timeout_secs);

    match cli.command {
        Commands::Interactive => {
            cli::commands::interactive::run().await?;
        }
        Commands::Status => {
            tui::quick_status().await?;
        }
        Commands::Setup => {
            tui::SetupWizard::run().await?;
        }
        Commands::Build(args) => {
            cli::commands::build::run(args, &client, &config).await?;
        }
        Commands::Run(args) => {
            cli::commands::run::run(args, &client, &config).await?;
        }
        Commands::Vm { command } => {
            cli::commands::vm::run(command, &config).await?;
        }
        Commands::Devices { refresh } => {
            cli::commands::devices::run(&client, refresh).await?;
        }
        Commands::Logs(args) => {
            cli::commands::logs::run(args, &agent_url).await?;
        }
        Commands::Config { command } => {
            cli::commands::config::run(command).await?;
        }
    }

    Ok(())
}
