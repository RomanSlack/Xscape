use anyhow::Result;
use ios_sim_common::{BuildConfiguration, BuildDestination, BuildRequest, BuildStatus, CliConfig};
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

use crate::agent_client::AgentClient;
use crate::cli::BuildArgs;
use crate::project;

/// Run the build command
pub async fn run(args: BuildArgs, client: &AgentClient, config: &CliConfig) -> Result<()> {
    let project_path = args.project.canonicalize()?;
    let project_name = project::get_project_name(&project_path);

    println!("Building project: {}", project_name);
    println!("  Scheme: {}", args.scheme);
    println!("  Configuration: {}", args.configuration);

    // Check agent health first
    let health = client.health().await?;
    if health.xcode_version.is_none() {
        anyhow::bail!("Agent reports Xcode is not available");
    }
    println!("  Agent: {} (Xcode {})", client.base_url(), health.xcode_version.unwrap_or_default());

    // Get target device
    let device_name = args
        .device
        .clone()
        .unwrap_or_else(|| config.simulator.preferred_device.clone());
    println!("  Device: {}", device_name);

    // Create project tarball
    println!("\nSyncing project...");
    let (tarball, checksum) = project::create_tarball(&project_path, &config.project.exclude_patterns)?;

    // Upload to agent
    let sync_result = client
        .sync_project(&project_name, &checksum, tarball)
        .await?;

    if sync_result.was_cached {
        println!("  Project already synced (cached)");
    } else {
        println!("  Synced {} files", sync_result.files_extracted);
    }

    // Parse configuration
    let configuration = match args.configuration.to_lowercase().as_str() {
        "release" => BuildConfiguration::Release,
        _ => BuildConfiguration::Debug,
    };

    // Start build
    println!("\nStarting build...");
    let build_request = BuildRequest {
        project_id: sync_result.project_id,
        project_file: None,
        scheme: args.scheme.clone(),
        configuration,
        destination: BuildDestination::ios_simulator(&device_name),
        extra_args: vec![],
        clean: args.clean,
    };

    let build_response = client.build(&build_request).await?;
    println!("  Build ID: {}", build_response.build_id);

    // Poll for completion
    println!("\nBuilding...");
    loop {
        sleep(Duration::from_secs(2)).await;

        let status = client.get_build_status(build_response.build_id).await?;

        match status.status {
            BuildStatus::Succeeded => {
                println!("\nBuild succeeded!");
                if let Some(ref app_path) = status.app_path {
                    println!("  App: {}", app_path);
                }
                if let Some(ref bundle_id) = status.bundle_id {
                    println!("  Bundle ID: {}", bundle_id);
                }
                if let Some(duration) = status.duration_secs {
                    println!("  Duration: {:.1}s", duration);
                }
                if !status.warnings.is_empty() {
                    println!("  Warnings: {}", status.warnings.len());
                }
                return Ok(());
            }
            BuildStatus::Failed => {
                println!("\nBuild failed!");
                if let Some(ref error) = status.error_message {
                    println!("  Error: {}", error);
                }
                anyhow::bail!("Build failed");
            }
            BuildStatus::Cancelled => {
                println!("\nBuild cancelled");
                anyhow::bail!("Build was cancelled");
            }
            _ => {
                // Still building, continue polling
                print!(".");
                std::io::Write::flush(&mut std::io::stdout())?;
            }
        }
    }
}
