pub mod commands;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "ios-sim")]
#[command(about = "Build and run iOS apps from Linux using a macOS VM or remote Mac")]
#[command(version)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Configuration file path
    #[arg(short, long, global = true, env = "IOS_SIM_CONFIG")]
    pub config: Option<PathBuf>,

    /// Agent URL (overrides config)
    #[arg(long, global = true, env = "IOS_SIM_AGENT_URL")]
    pub agent_url: Option<String>,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Interactive mode - TUI for building and running iOS apps
    Interactive,

    /// Quick status check
    Status,

    /// Setup wizard - verify and configure your installation
    Setup,

    /// Build an iOS project
    Build(BuildArgs),

    /// Build and run an iOS app in the simulator
    Run(RunArgs),

    /// Manage the local macOS VM
    Vm {
        #[command(subcommand)]
        command: VmCommands,
    },

    /// List available simulator devices
    Devices {
        /// Refresh device list from agent
        #[arg(long)]
        refresh: bool,
    },

    /// Stream build or app logs
    Logs(LogsArgs),

    /// Manage configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(clap::Args)]
pub struct BuildArgs {
    /// Path to project directory
    #[arg(short, long, default_value = ".")]
    pub project: PathBuf,

    /// Xcode scheme to build
    #[arg(short, long)]
    pub scheme: String,

    /// Build configuration (debug/release)
    #[arg(short = 'C', long, default_value = "debug")]
    pub configuration: String,

    /// Target simulator device name
    #[arg(short, long)]
    pub device: Option<String>,

    /// Clean build before building
    #[arg(long)]
    pub clean: bool,

    /// Don't stream build logs
    #[arg(long)]
    pub no_logs: bool,
}

#[derive(clap::Args)]
pub struct RunArgs {
    /// Path to project directory
    #[arg(short, long, default_value = ".")]
    pub project: PathBuf,

    /// Xcode scheme to build and run
    #[arg(short, long)]
    pub scheme: String,

    /// Target simulator device name
    #[arg(short, long)]
    pub device: Option<String>,

    /// Arguments to pass to the app
    #[arg(long)]
    pub args: Vec<String>,

    /// Environment variables (KEY=VALUE)
    #[arg(long)]
    pub env: Vec<String>,

    /// Don't stream logs
    #[arg(long)]
    pub no_logs: bool,
}

#[derive(Subcommand)]
pub enum VmCommands {
    /// Start the macOS VM
    Start {
        /// Run without GUI (VNC only)
        #[arg(long)]
        headless: bool,

        /// VNC port
        #[arg(long, default_value = "5900")]
        vnc_port: u16,

        /// Don't wait for agent to be ready
        #[arg(long)]
        no_wait: bool,
    },

    /// Stop the macOS VM
    Stop,

    /// Show VM status
    Status,

    /// Open noVNC in browser
    Vnc {
        /// noVNC port
        #[arg(long, default_value = "6080")]
        port: u16,
    },
}

#[derive(clap::Args)]
pub struct LogsArgs {
    /// Build ID to stream logs for
    #[arg(long)]
    pub build_id: Option<Uuid>,

    /// Follow log output
    #[arg(short, long)]
    pub follow: bool,
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Initialize configuration file
    Init {
        /// Force overwrite existing config
        #[arg(long)]
        force: bool,
    },

    /// Show current configuration
    Show,

    /// Set a configuration value
    Set {
        /// Configuration key (e.g., agent.mode, vm.memory)
        key: String,
        /// Configuration value
        value: String,
    },
}
