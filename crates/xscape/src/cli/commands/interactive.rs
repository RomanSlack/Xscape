use anyhow::Result;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm, FuzzySelect, Input, Select};
use std::path::PathBuf;

use crate::agent_client::AgentClient;
use crate::config::load_config;
use crate::tui::{progress, Screen};
use xscape_common::SimulatorState;

/// Cached agent status for status bar
struct AgentStatus {
    connected: bool,
    xcode_version: Option<String>,
    simulator_count: Option<usize>,
}

impl AgentStatus {
    async fn fetch(client: &AgentClient) -> Self {
        match client.health().await {
            Ok(health) => Self {
                connected: true,
                xcode_version: health.xcode_version,
                simulator_count: Some(health.available_simulators as usize),
            },
            Err(_) => Self {
                connected: false,
                xcode_version: None,
                simulator_count: None,
            },
        }
    }
}

/// Run the interactive TUI mode
pub async fn run() -> Result<()> {
    let config = load_config(&None)?;
    let agent_url = get_agent_url(&config);
    let client = AgentClient::new(&agent_url, config.agent.timeout_secs);

    loop {
        // Fetch status for status bar
        let status = AgentStatus::fetch(&client).await;

        Screen::clear();
        Screen::header(&[]);
        Screen::status_bar(
            status.connected,
            status.xcode_version.as_deref(),
            status.simulator_count,
        );

        let options = vec![
            "Run Project",
            "Build Project",
            "Simulators",
            "VM Control",
            "Settings",
            "Setup Wizard",
            "Exit",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select")
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => run_project_flow(&client, &config).await?,
            1 => build_project_flow(&client, &config).await?,
            2 => simulators_flow(&client).await?,
            3 => vm_flow(&config).await?,
            4 => settings_flow(&config).await?,
            5 => setup_flow().await?,
            _ => break,
        }
    }

    Screen::clear();
    println!("\n  Goodbye!\n");
    Ok(())
}

/// Run project flow with step-by-step screens
async fn run_project_flow(client: &AgentClient, config: &xscape_common::CliConfig) -> Result<()> {
    // Step 1: Select project
    Screen::clear();
    Screen::header(&["Run Project", "Select Project"]);

    let project_path = match select_project()? {
        Some(p) => p,
        None => return Ok(()), // User cancelled
    };

    // Step 2: Select scheme
    Screen::clear();
    Screen::header(&["Run Project", "Select Scheme"]);
    Screen::kv("Project", &project_path.display().to_string());
    println!();

    let scheme = match select_scheme(&project_path)? {
        Some(s) => s,
        None => return Ok(()),
    };

    // Step 3: Select simulator
    Screen::clear();
    Screen::header(&["Run Project", "Select Simulator"]);
    Screen::kv("Project", &project_path.display().to_string());
    Screen::kv("Scheme", &scheme);
    println!();

    let devices = get_devices(client).await?;
    if devices.is_empty() {
        Screen::error("No simulators available");
        Screen::pause();
        return Ok(());
    }

    let device = match select_simulator(&devices)? {
        Some(d) => d,
        None => return Ok(()),
    };

    // Step 4: Confirm and run
    Screen::clear();
    Screen::header(&["Run Project", "Confirm"]);

    println!();
    Screen::kv("Project", &project_path.display().to_string());
    Screen::kv("Scheme", &scheme);
    Screen::kv("Device", &format!("{} ({})", device.name, device.runtime));
    println!();

    let confirm = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Start build and run?")
        .default(true)
        .interact()?;

    if !confirm {
        return Ok(());
    }

    // Execute
    Screen::clear();
    Screen::header(&["Run Project", "Running"]);

    let args = crate::cli::RunArgs {
        project: project_path,
        scheme,
        device: Some(device.name.clone()),
        args: vec![],
        env: vec![],
        no_logs: false,
    };

    if let Err(e) = super::run::run(args, client, config).await {
        println!();
        Screen::error(&format!("{}", e));
    }

    Screen::pause();
    Ok(())
}

/// Build project flow
async fn build_project_flow(client: &AgentClient, config: &xscape_common::CliConfig) -> Result<()> {
    // Step 1: Select project
    Screen::clear();
    Screen::header(&["Build Project", "Select Project"]);

    let project_path = match select_project()? {
        Some(p) => p,
        None => return Ok(()),
    };

    // Step 2: Select scheme
    Screen::clear();
    Screen::header(&["Build Project", "Select Scheme"]);
    Screen::kv("Project", &project_path.display().to_string());
    println!();

    let scheme = match select_scheme(&project_path)? {
        Some(s) => s,
        None => return Ok(()),
    };

    // Step 3: Select simulator (optional, for destination)
    Screen::clear();
    Screen::header(&["Build Project", "Select Target"]);
    Screen::kv("Project", &project_path.display().to_string());
    Screen::kv("Scheme", &scheme);
    println!();

    let devices = get_devices(client).await?;
    let device_name = if !devices.is_empty() {
        select_simulator(&devices)?.map(|d| d.name.clone())
    } else {
        None
    };

    // Step 4: Confirm and build
    Screen::clear();
    Screen::header(&["Build Project", "Confirm"]);

    println!();
    Screen::kv("Project", &project_path.display().to_string());
    Screen::kv("Scheme", &scheme);
    if let Some(ref dev) = device_name {
        Screen::kv("Target", dev);
    }
    println!();

    let confirm = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Start build?")
        .default(true)
        .interact()?;

    if !confirm {
        return Ok(());
    }

    // Execute
    Screen::clear();
    Screen::header(&["Build Project", "Building"]);

    let args = crate::cli::BuildArgs {
        project: project_path,
        scheme,
        configuration: "debug".to_string(),
        device: device_name,
        clean: false,
        no_logs: false,
    };

    if let Err(e) = super::build::run(args, client, config).await {
        println!();
        Screen::error(&format!("{}", e));
    }

    Screen::pause();
    Ok(())
}

/// Simulators management flow
async fn simulators_flow(client: &AgentClient) -> Result<()> {
    loop {
        Screen::clear();
        Screen::header(&["Simulators"]);

        let options = vec![
            "List All",
            "Boot Simulator",
            "Shutdown Simulator",
            "Back",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select")
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => list_simulators(client).await?,
            1 => boot_simulator(client).await?,
            2 => shutdown_simulator(client).await?,
            _ => break,
        }
    }
    Ok(())
}

async fn list_simulators(client: &AgentClient) -> Result<()> {
    Screen::clear();
    Screen::header(&["Simulators", "List"]);

    let pb = progress::spinner("Loading simulators...");
    let devices = get_devices(client).await?;
    progress::spinner_success(&pb, &format!("{} simulators found", devices.len()));
    println!();

    if devices.is_empty() {
        Screen::warning("No simulators available");
    } else {
        // Group by runtime
        let mut runtimes: Vec<String> = devices.iter().map(|d| d.runtime.clone()).collect();
        runtimes.sort();
        runtimes.dedup();
        runtimes.reverse();

        for runtime in &runtimes {
            println!("  {}", runtime.bright_white());
            for device in devices.iter().filter(|d| &d.runtime == runtime) {
                let status = if device.is_booted {
                    "running".bright_green().to_string()
                } else {
                    "stopped".dimmed().to_string()
                };
                println!("    {:<30} {}", device.name, status);
            }
            println!();
        }
    }

    Screen::pause();
    Ok(())
}

async fn boot_simulator(client: &AgentClient) -> Result<()> {
    Screen::clear();
    Screen::header(&["Simulators", "Boot"]);

    let devices = get_devices(client).await?;
    let stopped: Vec<_> = devices.into_iter().filter(|d| !d.is_booted).collect();

    if stopped.is_empty() {
        Screen::warning("All simulators are already running");
        Screen::pause();
        return Ok(());
    }

    let device = match select_simulator(&stopped)? {
        Some(d) => d,
        None => return Ok(()),
    };

    println!();
    let pb = progress::spinner(&format!("Booting {}...", device.name));

    match client.boot_simulator(&device.udid).await {
        Ok(_) => progress::spinner_success(&pb, &format!("{} is now running", device.name)),
        Err(e) => progress::spinner_error(&pb, &format!("Failed: {}", e)),
    }

    Screen::pause();
    Ok(())
}

async fn shutdown_simulator(client: &AgentClient) -> Result<()> {
    Screen::clear();
    Screen::header(&["Simulators", "Shutdown"]);

    let devices = get_devices(client).await?;
    let running: Vec<_> = devices.into_iter().filter(|d| d.is_booted).collect();

    if running.is_empty() {
        Screen::warning("No running simulators");
        Screen::pause();
        return Ok(());
    }

    let device = match select_simulator(&running)? {
        Some(d) => d,
        None => return Ok(()),
    };

    println!();
    let pb = progress::spinner(&format!("Shutting down {}...", device.name));

    match client.shutdown_simulator(&device.udid).await {
        Ok(_) => progress::spinner_success(&pb, &format!("{} stopped", device.name)),
        Err(e) => progress::spinner_error(&pb, &format!("Failed: {}", e)),
    }

    Screen::pause();
    Ok(())
}

/// VM control flow
async fn vm_flow(config: &xscape_common::CliConfig) -> Result<()> {
    loop {
        Screen::clear();
        Screen::header(&["VM Control"]);

        let options = vec![
            "Start VM",
            "Stop VM",
            "VM Status",
            "Open VNC",
            "Back",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select")
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => {
                Screen::clear();
                Screen::header(&["VM Control", "Starting"]);
                super::vm::run(
                    crate::cli::VmCommands::Start {
                        headless: false,
                        vnc_port: 5900,
                        no_wait: false,
                    },
                    config,
                ).await?;
                Screen::pause();
            }
            1 => {
                Screen::clear();
                Screen::header(&["VM Control", "Stopping"]);
                super::vm::run(crate::cli::VmCommands::Stop, config).await?;
                Screen::pause();
            }
            2 => {
                Screen::clear();
                Screen::header(&["VM Control", "Status"]);
                super::vm::run(crate::cli::VmCommands::Status, config).await?;
                Screen::pause();
            }
            3 => {
                super::vm::run(crate::cli::VmCommands::Vnc { port: 6080 }, config).await?;
            }
            _ => break,
        }
    }
    Ok(())
}

/// Settings flow
async fn settings_flow(config: &xscape_common::CliConfig) -> Result<()> {
    Screen::clear();
    Screen::header(&["Settings"]);

    Screen::kv("Mode", &format!("{:?}", config.agent.mode));

    match config.agent.mode {
        xscape_common::AgentMode::LocalVm => {
            Screen::kv("VM Image", &config.vm.disk_image.display().to_string());
            Screen::kv("Memory", &config.vm.memory);
            Screen::kv("CPUs", &config.vm.cpus.to_string());
            Screen::kv("Agent Port", &config.vm.agent_port.to_string());
        }
        xscape_common::AgentMode::Remote => {
            Screen::kv(
                "Remote Host",
                &format!("{}:{}", config.agent.remote_host, config.agent.remote_port),
            );
        }
    }

    println!();
    Screen::info(&format!(
        "Config file: {}",
        "~/.config/ios-sim/config.toml".dimmed()
    ));

    Screen::pause();
    Ok(())
}

/// Setup wizard flow
async fn setup_flow() -> Result<()> {
    Screen::clear();
    if let Err(e) = crate::tui::SetupWizard::run().await {
        Screen::error(&format!("{}", e));
    }
    Screen::pause();
    Ok(())
}

// === Helper Functions ===

fn select_project() -> Result<Option<PathBuf>> {
    let options = vec!["Enter path", "Back"];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select")
        .items(&options)
        .default(0)
        .interact()?;

    if selection == 1 {
        return Ok(None);
    }

    let path: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Project path")
        .default(".".to_string())
        .validate_with(|input: &String| -> Result<(), &str> {
            let path = PathBuf::from(expand_tilde(input));
            if path.exists() {
                Ok(())
            } else {
                Err("Path does not exist")
            }
        })
        .interact_text()?;

    let project_path = PathBuf::from(expand_tilde(&path))
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("Invalid path: {}", e))?;

    Ok(Some(project_path))
}

fn select_scheme(project_path: &PathBuf) -> Result<Option<String>> {
    let schemes = find_schemes(project_path)?;

    if schemes.is_empty() {
        Screen::error("No schemes found in project");
        Screen::pause();
        return Ok(None);
    }

    if schemes.len() == 1 {
        Screen::info(&format!("Using scheme: {}", schemes[0]));
        return Ok(Some(schemes[0].clone()));
    }

    let mut options: Vec<&str> = schemes.iter().map(|s| s.as_str()).collect();
    options.push("Back");

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select scheme")
        .items(&options)
        .default(0)
        .interact()?;

    if selection == options.len() - 1 {
        return Ok(None);
    }

    Ok(Some(schemes[selection].clone()))
}

#[derive(Clone)]
struct DeviceInfo {
    udid: String,
    name: String,
    runtime: String,
    is_booted: bool,
}

fn select_simulator(devices: &[DeviceInfo]) -> Result<Option<DeviceInfo>> {
    // First, select iOS version
    let mut runtimes: Vec<String> = devices.iter().map(|d| d.runtime.clone()).collect();
    runtimes.sort();
    runtimes.dedup();
    runtimes.reverse();

    let mut runtime_options: Vec<String> = runtimes
        .iter()
        .map(|r| {
            let count = devices.iter().filter(|d| &d.runtime == r).count();
            format!("{} ({} devices)", r, count)
        })
        .collect();
    runtime_options.push("Back".to_string());

    let runtime_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("iOS Version")
        .items(&runtime_options)
        .default(0)
        .interact()?;

    if runtime_selection == runtime_options.len() - 1 {
        return Ok(None);
    }

    let selected_runtime = &runtimes[runtime_selection];

    // Then select device within that runtime
    let filtered: Vec<_> = devices
        .iter()
        .filter(|d| &d.runtime == selected_runtime)
        .collect();

    let mut device_options: Vec<String> = filtered
        .iter()
        .map(|d| {
            if d.is_booted {
                format!("{} [running]", d.name)
            } else {
                d.name.clone()
            }
        })
        .collect();
    device_options.push("Back".to_string());

    let device_selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Device")
        .items(&device_options)
        .default(0)
        .interact()?;

    if device_selection == device_options.len() - 1 {
        return Ok(None);
    }

    Ok(Some(filtered[device_selection].clone()))
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
            is_booted: d.state == SimulatorState::Booted,
        })
        .collect())
}

fn get_agent_url(config: &xscape_common::CliConfig) -> String {
    match config.agent.mode {
        xscape_common::AgentMode::LocalVm => {
            format!("http://127.0.0.1:{}", config.vm.agent_port)
        }
        xscape_common::AgentMode::Remote => {
            format!(
                "http://{}:{}",
                config.agent.remote_host, config.agent.remote_port
            )
        }
    }
}

fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return format!("{}{}", home.display(), &path[1..]);
        }
    }
    path.to_string()
}

fn find_schemes(project_path: &PathBuf) -> Result<Vec<String>> {
    let mut schemes = Vec::new();

    for entry in std::fs::read_dir(project_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map_or(false, |e| e == "xcodeproj" || e == "xcworkspace") {
            let schemes_dir = path.join("xcshareddata/xcschemes");
            if schemes_dir.exists() {
                for scheme_entry in std::fs::read_dir(schemes_dir)? {
                    let scheme_entry = scheme_entry?;
                    let scheme_path = scheme_entry.path();
                    if scheme_path.extension().map_or(false, |e| e == "xcscheme") {
                        if let Some(name) = scheme_path.file_stem() {
                            schemes.push(name.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }

    if schemes.is_empty() {
        for entry in std::fs::read_dir(project_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "xcodeproj") {
                if let Some(name) = path.file_stem() {
                    schemes.push(name.to_string_lossy().to_string());
                }
            }
        }
    }

    Ok(schemes)
}
