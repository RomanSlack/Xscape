use anyhow::Result;
use ios_sim_common::{
    BuildConfiguration, BuildDestination, BuildRequest, BuildStatus, CliConfig, RunAppRequest,
    SimulatorState,
};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

use crate::agent_client::AgentClient;
use crate::cli::RunArgs;
use crate::project;

/// Run the run command (build + run in simulator)
pub async fn run(args: RunArgs, client: &AgentClient, config: &CliConfig) -> Result<()> {
    let project_path = args.project.canonicalize()?;
    let project_name = project::get_project_name(&project_path);

    println!("Building and running: {}", project_name);
    println!("  Scheme: {}", args.scheme);

    // Check agent health first
    let health = client.health().await?;
    if health.xcode_version.is_none() {
        anyhow::bail!("Agent reports Xcode is not available");
    }
    println!(
        "  Agent: {} (Xcode {})",
        client.base_url(),
        health.xcode_version.unwrap_or_default()
    );

    // Get target device
    let device_name = args
        .device
        .clone()
        .unwrap_or_else(|| config.simulator.preferred_device.clone());
    println!("  Device: {}", device_name);

    // Find the device UDID
    let simulators = client.list_simulators().await?;
    let device = simulators
        .devices
        .iter()
        .find(|d| {
            d.name.to_lowercase().contains(&device_name.to_lowercase()) && d.is_available
        })
        .ok_or_else(|| anyhow::anyhow!("Device '{}' not found", device_name))?;

    println!("  Device UDID: {}", device.udid);

    // Create project tarball
    println!("\nSyncing project...");
    let (tarball, checksum) =
        project::create_tarball(&project_path, &config.project.exclude_patterns)?;

    // Upload to agent
    let sync_result = client
        .sync_project(&project_name, &checksum, tarball)
        .await?;

    if sync_result.was_cached {
        println!("  Project already synced (cached)");
    } else {
        println!("  Synced {} files", sync_result.files_extracted);
    }

    // Start build
    println!("\nBuilding...");
    let build_request = BuildRequest {
        project_id: sync_result.project_id,
        project_file: None,
        scheme: args.scheme.clone(),
        configuration: BuildConfiguration::Debug,
        destination: BuildDestination::ios_simulator(&device_name),
        extra_args: vec![],
        clean: false,
    };

    let build_response = client.build(&build_request).await?;

    // Poll for completion
    let build_status = loop {
        sleep(Duration::from_secs(2)).await;

        let status = client.get_build_status(build_response.build_id).await?;

        match status.status {
            BuildStatus::Succeeded => {
                println!("  Build succeeded ({:.1}s)", status.duration_secs.unwrap_or(0.0));
                break status;
            }
            BuildStatus::Failed => {
                println!("\nBuild failed!");
                if let Some(ref error) = status.error_message {
                    println!("  Error: {}", error);
                }
                anyhow::bail!("Build failed");
            }
            BuildStatus::Cancelled => {
                anyhow::bail!("Build was cancelled");
            }
            _ => {
                print!(".");
                std::io::Write::flush(&mut std::io::stdout())?;
            }
        }
    };

    // Boot simulator if needed
    if device.state != SimulatorState::Booted {
        println!("\nBooting simulator...");
        client.boot_simulator(&device.udid).await?;
        println!("  Simulator booted");
    }

    // Parse environment variables
    let mut environment = HashMap::new();
    for env_str in &args.env {
        if let Some((key, value)) = env_str.split_once('=') {
            environment.insert(key.to_string(), value.to_string());
        }
    }

    // Run app
    println!("\nLaunching app...");
    let run_request = RunAppRequest {
        build_id: build_response.build_id,
        device_udid: device.udid.clone(),
        launch_args: args.args.clone(),
        environment,
        wait_for_exit: false,
    };

    let run_result = client.run_app(&run_request).await?;

    println!("  App launched!");
    println!("  Bundle ID: {}", run_result.bundle_id);
    if let Some(pid) = run_result.pid {
        println!("  PID: {}", pid);
    }
    println!("  Session: {}", run_result.session_id);

    println!("\nApp is running in the simulator.");
    println!("Use 'ios-sim vm vnc' to view the simulator GUI.");

    Ok(())
}
