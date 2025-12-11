use anyhow::Result;
use ios_sim_common::SimulatorState;

use crate::agent_client::AgentClient;

/// List available simulator devices
pub async fn run(client: &AgentClient, _refresh: bool) -> Result<()> {
    println!("Fetching available simulators...\n");

    let response = client.list_simulators().await?;

    // Group devices by runtime
    let mut devices_by_runtime: std::collections::HashMap<String, Vec<_>> =
        std::collections::HashMap::new();

    for device in &response.devices {
        devices_by_runtime
            .entry(device.runtime.clone())
            .or_default()
            .push(device);
    }

    // Print runtimes and devices
    let mut runtimes: Vec<_> = devices_by_runtime.keys().collect();
    runtimes.sort();

    for runtime in runtimes {
        println!("{}:", runtime);

        let mut devices = devices_by_runtime.get(runtime).unwrap().clone();
        devices.sort_by(|a, b| a.name.cmp(&b.name));

        for device in devices {
            let state_icon = match device.state {
                SimulatorState::Booted => "🟢",
                SimulatorState::Booting => "🟡",
                SimulatorState::ShuttingDown => "🟡",
                SimulatorState::Shutdown => "⚪",
            };

            let available = if device.is_available { "" } else { " (unavailable)" };

            println!(
                "  {} {} [{}]{}",
                state_icon, device.name, device.udid, available
            );
        }
        println!();
    }

    // Summary
    let booted_count = response
        .devices
        .iter()
        .filter(|d| d.state == SimulatorState::Booted)
        .count();
    let available_count = response.devices.iter().filter(|d| d.is_available).count();

    println!(
        "Total: {} devices ({} available, {} booted)",
        response.devices.len(),
        available_count,
        booted_count
    );

    // Print available runtimes
    println!("\nAvailable Runtimes:");
    for runtime in &response.runtimes {
        let status = if runtime.is_available { "✓" } else { "✗" };
        println!("  {} {} ({})", status, runtime.name, runtime.version);
    }

    Ok(())
}
