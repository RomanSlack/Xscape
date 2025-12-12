use anyhow::Result;

use crate::cli::ConfigCommands;
use crate::config::{default_config_path, generate_default_config, load_config, save_config};

/// Run config management commands
pub async fn run(command: ConfigCommands) -> Result<()> {
    match command {
        ConfigCommands::Init { force } => init_config(force),
        ConfigCommands::Show => show_config(),
        ConfigCommands::Set { key, value } => set_config(&key, &value),
    }
}

fn init_config(force: bool) -> Result<()> {
    let config_path = default_config_path();

    if config_path.exists() && !force {
        println!("Config file already exists at: {:?}", config_path);
        println!("Use --force to overwrite.");
        return Ok(());
    }

    // Create directory if needed
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write default config
    let content = generate_default_config();
    std::fs::write(&config_path, content)?;

    println!("Created config file: {:?}", config_path);
    println!("\nEdit this file to configure your macOS VM or remote Mac connection.");

    Ok(())
}

fn show_config() -> Result<()> {
    let config_path = default_config_path();

    if !config_path.exists() {
        println!("No config file found at: {:?}", config_path);
        println!("Run 'xscape config init' to create one.");
        return Ok(());
    }

    println!("Config file: {:?}\n", config_path);

    // Load and display
    let config = load_config(&Some(config_path.clone()))?;

    println!("[agent]");
    println!("  mode = {:?}", config.agent.mode);
    println!("  remote_host = {}", config.agent.remote_host);
    println!("  remote_port = {}", config.agent.remote_port);
    println!("  timeout_secs = {}", config.agent.timeout_secs);

    println!("\n[vm]");
    println!("  qemu_path = {:?}", config.vm.qemu_path);
    println!("  disk_image = {:?}", config.vm.disk_image);
    println!("  memory = {}", config.vm.memory);
    println!("  cpus = {}", config.vm.cpus);
    println!("  vnc_port = {}", config.vm.vnc_port);
    println!("  ssh_port = {}", config.vm.ssh_port);
    println!("  agent_port = {}", config.vm.agent_port);

    println!("\n[vnc]");
    println!("  novnc_path = {:?}", config.vnc.novnc_path);
    println!("  websockify_port = {}", config.vnc.websockify_port);
    println!("  auto_open_browser = {}", config.vnc.auto_open_browser);

    println!("\n[project]");
    println!("  default_scheme = {:?}", config.project.default_scheme);
    println!("  exclude_patterns = {:?}", config.project.exclude_patterns);

    println!("\n[simulator]");
    println!("  preferred_device = {}", config.simulator.preferred_device);
    println!("  preferred_runtime = {:?}", config.simulator.preferred_runtime);

    Ok(())
}

fn set_config(key: &str, value: &str) -> Result<()> {
    let config_path = default_config_path();
    let mut config = load_config(&Some(config_path.clone()))?;

    // Parse key and set value
    let parts: Vec<&str> = key.split('.').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid key format. Use 'section.key' (e.g., 'vm.memory')");
    }

    match (parts[0], parts[1]) {
        ("agent", "mode") => {
            config.agent.mode = match value.to_lowercase().as_str() {
                "remote" => xscape_common::AgentMode::Remote,
                "local-vm" | "localvm" | "vm" => xscape_common::AgentMode::LocalVm,
                _ => anyhow::bail!("Invalid mode. Use 'remote' or 'local-vm'"),
            };
        }
        ("agent", "remote_host") => config.agent.remote_host = value.to_string(),
        ("agent", "remote_port") => config.agent.remote_port = value.parse()?,
        ("agent", "timeout_secs") => config.agent.timeout_secs = value.parse()?,

        ("vm", "qemu_path") => config.vm.qemu_path = value.into(),
        ("vm", "disk_image") => config.vm.disk_image = value.into(),
        ("vm", "ovmf_code") => config.vm.ovmf_code = value.into(),
        ("vm", "memory") => config.vm.memory = value.to_string(),
        ("vm", "cpus") => config.vm.cpus = value.parse()?,
        ("vm", "vnc_port") => config.vm.vnc_port = value.parse()?,
        ("vm", "ssh_port") => config.vm.ssh_port = value.parse()?,
        ("vm", "agent_port") => config.vm.agent_port = value.parse()?,
        ("vm", "boot_timeout_secs") => config.vm.boot_timeout_secs = value.parse()?,

        ("vnc", "novnc_path") => config.vnc.novnc_path = value.into(),
        ("vnc", "websockify_port") => config.vnc.websockify_port = value.parse()?,
        ("vnc", "auto_open_browser") => config.vnc.auto_open_browser = value.parse()?,

        ("simulator", "preferred_device") => config.simulator.preferred_device = value.to_string(),
        ("simulator", "preferred_runtime") => {
            config.simulator.preferred_runtime = if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            };
        }

        _ => anyhow::bail!("Unknown config key: {}", key),
    }

    save_config(&config, &Some(config_path))?;
    println!("Set {} = {}", key, value);

    Ok(())
}
