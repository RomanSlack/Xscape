use anyhow::Result;
use xscape_common::CliConfig;

use crate::cli::VmCommands;
use crate::vm::{self, NoVncProxy, QemuVm};

/// Run VM management commands
pub async fn run(command: VmCommands, config: &CliConfig) -> Result<()> {
    match command {
        VmCommands::Start {
            headless,
            vnc_port,
            no_wait,
        } => {
            start_vm(config, headless, vnc_port, no_wait).await
        }
        VmCommands::Stop => stop_vm(config).await,
        VmCommands::Status => show_status(config).await,
        VmCommands::Vnc { port } => open_vnc(config, port).await,
    }
}

async fn start_vm(config: &CliConfig, headless: bool, vnc_port: u16, no_wait: bool) -> Result<()> {
    // Check if already running
    if vm::is_vm_running(config).await {
        println!("VM is already running!");
        return Ok(());
    }

    // Create VM config with overridden VNC port
    let mut vm_config = config.vm.clone();
    vm_config.vnc_port = vnc_port;

    let mut qemu = QemuVm::new(vm_config.clone());
    qemu.start(headless)?;

    println!("VM started!");
    println!("  VNC port: {}", vnc_port);
    println!("  SSH port: {}", vm_config.ssh_port);
    println!("  Agent port: {}", vm_config.agent_port);

    if !no_wait {
        println!("\nWaiting for agent to be ready...");
        let agent_url = format!("http://127.0.0.1:{}", vm_config.agent_port);
        vm::wait_for_agent(&agent_url, vm_config.boot_timeout_secs).await?;
        println!("Agent is ready!");
    }

    // Don't drop the VM handle (it would try to stop it)
    std::mem::forget(qemu);

    Ok(())
}

async fn stop_vm(config: &CliConfig) -> Result<()> {
    let pids = vm::qemu::find_running_vms();

    if pids.is_empty() {
        println!("No running VMs found.");
        return Ok(());
    }

    for pid in pids {
        println!("Stopping VM (PID: {})...", pid);
        vm::qemu::kill_vm(pid)?;
    }

    println!("VM stopped.");
    Ok(())
}

async fn show_status(config: &CliConfig) -> Result<()> {
    let pids = vm::qemu::find_running_vms();

    if pids.is_empty() {
        println!("VM Status: Not running");
        return Ok(());
    }

    println!("VM Status: Running");
    println!("  PIDs: {:?}", pids);

    // Check agent health
    let agent_url = format!("http://127.0.0.1:{}", config.vm.agent_port);
    let client = crate::agent_client::AgentClient::new(&agent_url, 5);

    match client.health().await {
        Ok(health) => {
            println!("  Agent: Healthy");
            if let Some(version) = health.xcode_version {
                println!("  Xcode: {}", version);
            }
            println!("  Simulators: {}", health.available_simulators);
        }
        Err(e) => {
            println!("  Agent: Not reachable ({})", e);
        }
    }

    println!("\nPorts:");
    println!("  VNC: {}", config.vm.vnc_port);
    println!("  SSH: {}", config.vm.ssh_port);
    println!("  Agent: {}", config.vm.agent_port);

    Ok(())
}

async fn open_vnc(config: &CliConfig, port: u16) -> Result<()> {
    // Check if VM is running
    if !vm::is_vm_running(config).await {
        println!("VM is not running. Start it with 'ios-sim vm start'");
        return Ok(());
    }

    // Start noVNC proxy
    let mut vnc_config = config.vnc.clone();
    vnc_config.websockify_port = port;

    let mut proxy = NoVncProxy::new(vnc_config.clone(), config.vm.vnc_port);

    match proxy.start() {
        Ok(_) => {
            let url = proxy.url();
            println!("noVNC proxy started on port {}", port);
            println!("\nOpening: {}", url);

            // Open browser
            if let Err(e) = open::that(&url) {
                println!("Failed to open browser: {}", e);
                println!("Please open the URL manually.");
            }

            println!("\nPress Ctrl+C to stop the proxy...");

            // Keep running until interrupted
            tokio::signal::ctrl_c().await?;
            proxy.stop();
        }
        Err(e) => {
            println!("Failed to start noVNC proxy: {}", e);
            println!("\nYou can connect directly with a VNC client:");
            println!("  vnc://localhost:{}", config.vm.vnc_port);
        }
    }

    Ok(())
}
