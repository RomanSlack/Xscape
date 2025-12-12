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

    // Check if using boot script (will block with interactive QEMU)
    let using_boot_script = !vm_config.boot_script.as_os_str().is_empty();

    let mut qemu = QemuVm::new(vm_config.clone());

    if using_boot_script {
        // Boot script mode - blocks until QEMU exits, user gets QEMU monitor
        println!("Starting VM with boot script...\n");
        qemu.start(headless)?;
        println!("\nVM exited.");
    } else {
        // Manual QEMU mode - starts in background
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
    }

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
        println!("VM is not running. Start it with 'xscape vm start'");
        return Ok(());
    }

    // Check if VNC is actually accessible
    let vnc_port = config.vm.vnc_port;
    if !check_vnc_port(vnc_port).await {
        println!("VNC not accessible on port {}", vnc_port);
        println!();

        // Try to detect VNC from running QEMU process
        if let Some(detected_port) = detect_vnc_port() {
            println!("Detected VNC on port {}. Updating...", detected_port);
            return open_vnc_with_port(config, port, detected_port).await;
        }

        println!("The macOS VM may not have VNC enabled.");
        println!();
        println!("If you started the VM with OSX-KVM directly, you need to add VNC:");
        println!("  1. Stop the VM");
        println!("  2. Edit your boot script and add: -vnc :0");
        println!("  3. Start the VM again");
        println!();
        println!("Or start the VM using xscape (VNC is auto-configured):");
        println!("  xscape vm stop");
        println!("  xscape vm start");
        return Ok(());
    }

    open_vnc_with_port(config, port, vnc_port).await
}

async fn check_vnc_port(port: u16) -> bool {
    use std::net::TcpStream;
    use std::time::Duration;

    std::net::TcpStream::connect_timeout(
        &format!("127.0.0.1:{}", port).parse().unwrap(),
        Duration::from_secs(2)
    ).is_ok()
}

fn detect_vnc_port() -> Option<u16> {
    use std::process::Command;

    // Get QEMU command line to find VNC port
    let output = Command::new("ps")
        .args(["aux"])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        if line.contains("qemu-system") {
            // Look for -vnc :N pattern
            if let Some(vnc_idx) = line.find("-vnc :") {
                let after_vnc = &line[vnc_idx + 6..];
                let display_str: String = after_vnc.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(display) = display_str.parse::<u16>() {
                    return Some(5900 + display);
                }
            }
            // Look for -vnc 0.0.0.0:N pattern
            if let Some(vnc_idx) = line.find("-vnc 0.0.0.0:") {
                let after_vnc = &line[vnc_idx + 13..];
                let port_str: String = after_vnc.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(p) = port_str.parse::<u16>() {
                    return Some(p);
                }
            }
        }
    }

    None
}

async fn open_vnc_with_port(config: &CliConfig, websockify_port: u16, vnc_port: u16) -> Result<()> {
    // Start noVNC proxy
    let mut vnc_config = config.vnc.clone();
    vnc_config.websockify_port = websockify_port;

    let mut proxy = NoVncProxy::new(vnc_config.clone(), vnc_port);

    match proxy.start() {
        Ok(_) => {
            let url = proxy.url();
            println!("noVNC proxy started on port {}", websockify_port);
            println!("  VNC backend: localhost:{}", vnc_port);
            println!();
            println!("Opening: {}", url);

            // Open browser
            if let Err(e) = open::that(&url) {
                println!("Failed to open browser: {}", e);
                println!("Please open the URL manually.");
            }

            println!();
            println!("Press Ctrl+C to stop the proxy...");

            // Keep running until interrupted
            tokio::signal::ctrl_c().await?;
            proxy.stop();
        }
        Err(e) => {
            println!("Failed to start noVNC proxy: {}", e);
            println!();
            println!("Try installing websockify:");
            println!("  sudo apt install websockify");
            println!();
            println!("Or connect directly with a VNC client:");
            println!("  vncviewer localhost:{}", vnc_port);
        }
    }

    Ok(())
}
