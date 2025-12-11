use anyhow::Result;
use colored::Colorize;

use crate::agent_client::AgentClient;
use crate::config::load_config;
use crate::tui::{
    DeviceInfo, MainMenu, MenuAction, ProjectSelector, SimulatorAction, SimulatorMenu,
    SimulatorSelector, Styles, VmAction, VmMenu,
};

/// Run the interactive TUI mode
pub async fn run() -> Result<()> {
    loop {
        match MainMenu::show()? {
            MenuAction::Run => {
                if let Err(e) = run_project().await {
                    Styles::error(&format!("Run failed: {}", e));
                    println!();
                }
            }
            MenuAction::Build => {
                if let Err(e) = build_project().await {
                    Styles::error(&format!("Build failed: {}", e));
                    println!();
                }
            }
            MenuAction::Simulators => {
                if let Err(e) = manage_simulators().await {
                    Styles::error(&format!("Error: {}", e));
                    println!();
                }
            }
            MenuAction::Vm => {
                if let Err(e) = manage_vm().await {
                    Styles::error(&format!("Error: {}", e));
                    println!();
                }
            }
            MenuAction::Settings => {
                if let Err(e) = show_settings().await {
                    Styles::error(&format!("Error: {}", e));
                    println!();
                }
            }
            MenuAction::Setup => {
                if let Err(e) = crate::tui::SetupWizard::run().await {
                    Styles::error(&format!("Setup failed: {}", e));
                    println!();
                }
            }
            MenuAction::Exit => {
                println!();
                Styles::dimmed("Goodbye!");
                println!();
                break;
            }
        }
    }

    Ok(())
}

async fn run_project() -> Result<()> {
    let (project_path, scheme) = ProjectSelector::select()?;

    let config = load_config(&None)?;
    let agent_url = get_agent_url(&config);
    let client = AgentClient::new(&agent_url, config.agent.timeout_secs);

    // Get available simulators
    let devices = get_devices(&client).await?;

    if devices.is_empty() {
        anyhow::bail!("No simulators available. Make sure the agent is connected.");
    }

    // Select device
    let device_udid = SimulatorSelector::select_or_default(&devices, "iPhone")?;
    let device = devices.iter().find(|d| d.udid == device_udid).unwrap();

    Styles::header("Running Project");
    Styles::info(&format!("Project: {}", project_path.display()));
    Styles::info(&format!("Scheme: {}", scheme));
    Styles::info(&format!("Device: {} ({})", device.name, device.runtime));
    println!();

    // Create run args and execute
    let args = crate::cli::RunArgs {
        project: project_path,
        scheme,
        device: Some(device.name.clone()),
        args: vec![],
        env: vec![],
        no_logs: false,
    };

    super::run::run(args, &client, &config).await?;

    Ok(())
}

async fn build_project() -> Result<()> {
    let (project_path, scheme) = ProjectSelector::select()?;

    let config = load_config(&None)?;
    let agent_url = get_agent_url(&config);
    let client = AgentClient::new(&agent_url, config.agent.timeout_secs);

    // Get available simulators for destination
    let devices = get_devices(&client).await?;
    let device_name = if !devices.is_empty() {
        let device_udid = SimulatorSelector::select_or_default(&devices, "iPhone")?;
        devices
            .iter()
            .find(|d| d.udid == device_udid)
            .map(|d| d.name.clone())
    } else {
        None
    };

    Styles::header("Building Project");
    Styles::info(&format!("Project: {}", project_path.display()));
    Styles::info(&format!("Scheme: {}", scheme));
    if let Some(ref dev) = device_name {
        Styles::info(&format!("Device: {}", dev));
    }
    println!();

    let args = crate::cli::BuildArgs {
        project: project_path,
        scheme,
        configuration: "debug".to_string(),
        device: device_name,
        clean: false,
        no_logs: false,
    };

    super::build::run(args, &client, &config).await?;

    Ok(())
}

async fn manage_simulators() -> Result<()> {
    let config = load_config(&None)?;
    let agent_url = get_agent_url(&config);
    let client = AgentClient::new(&agent_url, config.agent.timeout_secs);

    loop {
        match SimulatorMenu::show()? {
            SimulatorAction::List => {
                Styles::header("Available Simulators");
                let devices = get_devices(&client).await?;

                if devices.is_empty() {
                    Styles::warning("No simulators found");
                } else {
                    for device in &devices {
                        let status = if device.is_booted {
                            "[running]".bright_green()
                        } else {
                            "".dimmed()
                        };
                        println!(
                            "   {:<28} {:<20} {}",
                            device.name.bright_white(),
                            device.runtime.dimmed(),
                            status
                        );
                    }
                }
                println!();
            }
            SimulatorAction::Boot => {
                let devices = get_devices(&client).await?;
                let shutdown: Vec<_> = devices.iter().filter(|d| !d.is_booted).cloned().collect();

                if shutdown.is_empty() {
                    Styles::warning("No shutdown simulators to boot");
                } else {
                    let udid = SimulatorSelector::select(&shutdown)?;
                    let device = shutdown.iter().find(|d| d.udid == udid).unwrap();

                    let pb = crate::tui::progress::spinner(&format!("Booting {}...", device.name));
                    match client.boot_simulator(&udid).await {
                        Ok(_) => {
                            crate::tui::progress::spinner_success(&pb, &format!("{} booted", device.name));
                        }
                        Err(e) => {
                            crate::tui::progress::spinner_error(&pb, &format!("Failed: {}", e));
                        }
                    }
                }
                println!();
            }
            SimulatorAction::Shutdown => {
                let devices = get_devices(&client).await?;
                let booted: Vec<_> = devices.iter().filter(|d| d.is_booted).cloned().collect();

                if booted.is_empty() {
                    Styles::warning("No booted simulators to shutdown");
                } else {
                    let udid = SimulatorSelector::select(&booted)?;
                    let device = booted.iter().find(|d| d.udid == udid).unwrap();

                    let pb = crate::tui::progress::spinner(&format!("Shutting down {}...", device.name));
                    match client.shutdown_simulator(&udid).await {
                        Ok(_) => {
                            crate::tui::progress::spinner_success(&pb, &format!("{} shutdown", device.name));
                        }
                        Err(e) => {
                            crate::tui::progress::spinner_error(&pb, &format!("Failed: {}", e));
                        }
                    }
                }
                println!();
            }
            SimulatorAction::Back => break,
        }
    }

    Ok(())
}

async fn manage_vm() -> Result<()> {
    let config = load_config(&None)?;

    loop {
        match VmMenu::show()? {
            VmAction::Start => {
                super::vm::run(
                    crate::cli::VmCommands::Start {
                        headless: false,
                        vnc_port: 5900,
                        no_wait: false,
                    },
                    &config,
                )
                .await?;
            }
            VmAction::Stop => {
                super::vm::run(crate::cli::VmCommands::Stop, &config).await?;
            }
            VmAction::Status => {
                super::vm::run(crate::cli::VmCommands::Status, &config).await?;
            }
            VmAction::Vnc => {
                super::vm::run(crate::cli::VmCommands::Vnc { port: 6080 }, &config).await?;
            }
            VmAction::Back => break,
        }
    }

    Ok(())
}

async fn show_settings() -> Result<()> {
    Styles::header("Current Settings");

    let config = load_config(&None)?;

    Styles::kv("Mode", &format!("{:?}", config.agent.mode));

    match config.agent.mode {
        ios_sim_common::AgentMode::LocalVm => {
            Styles::kv("VM Image", &config.vm.disk_image.display().to_string());
            Styles::kv("Memory", &format!("{} MB", config.vm.memory));
            Styles::kv("CPUs", &config.vm.cpus.to_string());
            Styles::kv("Agent Port", &config.vm.agent_port.to_string());
        }
        ios_sim_common::AgentMode::Remote => {
            Styles::kv(
                "Remote Host",
                &format!("{}:{}", config.agent.remote_host, config.agent.remote_port),
            );
        }
    }

    println!();
    Styles::dimmed("Edit ~/.config/ios-sim/config.toml to change settings");
    println!();

    Ok(())
}

fn get_agent_url(config: &ios_sim_common::CliConfig) -> String {
    match config.agent.mode {
        ios_sim_common::AgentMode::LocalVm => {
            format!("http://127.0.0.1:{}", config.vm.agent_port)
        }
        ios_sim_common::AgentMode::Remote => {
            format!(
                "http://{}:{}",
                config.agent.remote_host, config.agent.remote_port
            )
        }
    }
}

async fn get_devices(client: &AgentClient) -> Result<Vec<DeviceInfo>> {
    let response = client.list_simulators().await?;

    Ok(response
        .devices
        .into_iter()
        .filter(|d| d.is_available)
        .map(|d| DeviceInfo {
            udid: d.udid,
            name: d.name,
            runtime: d.runtime,
            is_booted: d.state == ios_sim_common::SimulatorState::Booted,
        })
        .collect())
}
