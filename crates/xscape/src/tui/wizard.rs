use anyhow::Result;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use std::path::PathBuf;

use super::{progress, Styles, StatusColor};
use crate::agent_client::AgentClient;
use crate::config::{default_config_path, load_config, save_config};
use crate::vm;

/// Setup wizard to verify and configure the system
pub struct SetupWizard;

impl SetupWizard {
    pub async fn run() -> Result<()> {
        Styles::print_banner();
        Styles::header("Setup Wizard");

        println!(
            "{}",
            "Verifying your xscape installation.\n".dimmed()
        );

        // Step 1: Check configuration
        Self::check_config().await?;

        // Step 2: Check VM or remote connection
        Self::check_connection().await?;

        // Step 3: Verify Xcode
        Self::check_xcode().await?;

        // Step 4: Check simulators
        Self::check_simulators().await?;

        Styles::header("Setup Complete");
        Styles::success("Your xscape installation is ready to use!");
        println!();
        println!("   Run: {} {}", "xscape".bright_white(), "interactive");
        println!();

        Ok(())
    }

    async fn check_config() -> Result<()> {
        let pb = progress::spinner("Checking configuration...");

        let config_path = default_config_path();
        let config_exists = config_path.exists();

        if config_exists {
            progress::spinner_success(&pb, "Configuration file found");
            Styles::dimmed(&config_path.display().to_string());
        } else {
            progress::spinner_error(&pb, "Configuration file not found");

            let create = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Create default configuration?")
                .default(true)
                .interact()?;

            if create {
                let config = xscape_common::CliConfig::default();
                save_config(&config, &Some(config_path.clone()))?;
                Styles::success(&format!("Created {}", config_path.display()));
            }
        }

        // Load and display config
        let config = load_config(&Some(config_path))?;
        println!();
        Styles::kv("Mode", &format!("{:?}", config.agent.mode));

        match config.agent.mode {
            xscape_common::AgentMode::LocalVm => {
                if config.vm.disk_image.as_os_str().is_empty() {
                    Styles::warning("VM disk image not configured");

                    let configure = Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt("Configure VM disk image path?")
                        .default(true)
                        .interact()?;

                    if configure {
                        let path: String = Input::with_theme(&ColorfulTheme::default())
                            .with_prompt("Path to macOS disk image (.qcow2)")
                            .interact_text()?;

                        let mut config = config.clone();
                        config.vm.disk_image = PathBuf::from(path);
                        save_config(&config, &None)?;
                        Styles::success("VM disk image path saved");
                    }
                } else {
                    Styles::kv("VM Image", &config.vm.disk_image.display().to_string());
                }
            }
            xscape_common::AgentMode::Remote => {
                Styles::kv(
                    "Remote Host",
                    &format!("{}:{}", config.agent.remote_host, config.agent.remote_port),
                );
            }
        }

        Ok(())
    }

    async fn check_connection() -> Result<()> {
        println!();
        let pb = progress::spinner("Checking agent connection...");

        let config = load_config(&None)?;
        let agent_url = match config.agent.mode {
            xscape_common::AgentMode::LocalVm => {
                format!("http://127.0.0.1:{}", config.vm.agent_port)
            }
            xscape_common::AgentMode::Remote => {
                format!("http://{}:{}", config.agent.remote_host, config.agent.remote_port)
            }
        };

        let client = AgentClient::new(&agent_url, 5);

        match client.health().await {
            Ok(health) => {
                progress::spinner_success(&pb, "Agent connected");
                Styles::kv("Agent Version", &health.agent_version);
            }
            Err(_) => {
                progress::spinner_error(&pb, "Agent not reachable");

                if config.agent.mode == xscape_common::AgentMode::LocalVm {
                    println!();
                    let start_vm = Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt("Start the macOS VM?")
                        .default(true)
                        .interact()?;

                    if start_vm {
                        Self::start_vm(&config).await?;
                    }
                } else {
                    Styles::warning("Make sure xscape-agent is running on your remote Mac");
                }
            }
        }

        Ok(())
    }

    async fn start_vm(config: &xscape_common::CliConfig) -> Result<()> {
        let pb = progress::spinner("Starting macOS VM...");

        let mut vm = vm::QemuVm::new(config.vm.clone());
        match vm.start(false) {
            Ok(_) => {
                progress::spinner_success(&pb, "VM started");

                let pb2 = progress::spinner("Waiting for agent...");
                let agent_url = format!("http://127.0.0.1:{}", config.vm.agent_port);

                match vm::wait_for_agent(&agent_url, config.vm.boot_timeout_secs).await {
                    Ok(_) => {
                        progress::spinner_success(&pb2, "Agent ready");
                    }
                    Err(e) => {
                        progress::spinner_error(&pb2, &format!("Agent timeout: {}", e));
                    }
                }

                // Don't drop the VM handle
                std::mem::forget(vm);
            }
            Err(e) => {
                progress::spinner_error(&pb, &format!("Failed to start VM: {}", e));
            }
        }

        Ok(())
    }

    async fn check_xcode() -> Result<()> {
        println!();
        let pb = progress::spinner("Checking Xcode...");

        let config = load_config(&None)?;
        let agent_url = match config.agent.mode {
            xscape_common::AgentMode::LocalVm => {
                format!("http://127.0.0.1:{}", config.vm.agent_port)
            }
            xscape_common::AgentMode::Remote => {
                format!("http://{}:{}", config.agent.remote_host, config.agent.remote_port)
            }
        };

        let client = AgentClient::new(&agent_url, 10);

        match client.health().await {
            Ok(health) => {
                if let Some(version) = health.xcode_version {
                    progress::spinner_success(&pb, &format!("Xcode {} found", version));
                    if let Some(path) = health.xcode_path {
                        Styles::dimmed(&path);
                    }
                } else {
                    progress::spinner_error(&pb, "Xcode not found");
                    Styles::warning("Install Xcode from the App Store on macOS");
                }
            }
            Err(_) => {
                progress::spinner_error(&pb, "Could not check Xcode (agent not connected)");
            }
        }

        Ok(())
    }

    async fn check_simulators() -> Result<()> {
        println!();
        let pb = progress::spinner("Checking simulators...");

        let config = load_config(&None)?;
        let agent_url = match config.agent.mode {
            xscape_common::AgentMode::LocalVm => {
                format!("http://127.0.0.1:{}", config.vm.agent_port)
            }
            xscape_common::AgentMode::Remote => {
                format!("http://{}:{}", config.agent.remote_host, config.agent.remote_port)
            }
        };

        let client = AgentClient::new(&agent_url, 10);

        match client.list_simulators().await {
            Ok(response) => {
                let available = response.devices.iter().filter(|d| d.is_available).count();
                let booted = response
                    .devices
                    .iter()
                    .filter(|d| d.state == xscape_common::SimulatorState::Booted)
                    .count();

                progress::spinner_success(
                    &pb,
                    &format!("{} simulators available, {} running", available, booted),
                );

                // Show runtimes
                if !response.runtimes.is_empty() {
                    println!();
                    Styles::dimmed("Available runtimes:");
                    for runtime in response.runtimes.iter().filter(|r| r.is_available).take(5) {
                        Styles::dimmed(&format!("  - {}", runtime.name));
                    }
                }
            }
            Err(_) => {
                progress::spinner_error(&pb, "Could not list simulators");
            }
        }

        Ok(())
    }
}

/// Quick status check
pub async fn quick_status() -> Result<()> {
    let config = load_config(&None)?;

    let agent_url = match config.agent.mode {
        xscape_common::AgentMode::LocalVm => {
            format!("http://127.0.0.1:{}", config.vm.agent_port)
        }
        xscape_common::AgentMode::Remote => {
            format!("http://{}:{}", config.agent.remote_host, config.agent.remote_port)
        }
    };

    let client = AgentClient::new(&agent_url, 5);

    println!();
    match client.health().await {
        Ok(health) => {
            Styles::status("+", "Status", "Connected", StatusColor::Green);
            if let Some(xcode) = health.xcode_version {
                Styles::status("+", "Xcode", &xcode, StatusColor::White);
            }
            Styles::status(
                "+",
                "Simulators",
                &health.available_simulators.to_string(),
                StatusColor::White,
            );
        }
        Err(_) => {
            Styles::status("-", "Status", "Disconnected", StatusColor::Red);
            Styles::dimmed("Run 'xscape vm start' or check your remote Mac");
        }
    }
    println!();

    Ok(())
}
