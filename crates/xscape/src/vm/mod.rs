mod novnc;
pub mod qemu;

pub use novnc::NoVncProxy;
pub use qemu::QemuVm;

use anyhow::Result;
use xscape_common::CliConfig;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info};

/// Wait for agent to become reachable
pub async fn wait_for_agent(agent_url: &str, timeout_secs: u64) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let health_url = format!("{}/health", agent_url);
    let start = std::time::Instant::now();

    info!("Waiting for agent at {}...", agent_url);

    while start.elapsed().as_secs() < timeout_secs {
        match client.get(&health_url).send().await {
            Ok(response) if response.status().is_success() => {
                info!("Agent is ready!");
                return Ok(());
            }
            Ok(response) => {
                debug!("Agent returned {}, waiting...", response.status());
            }
            Err(e) => {
                debug!("Agent not ready: {}", e);
            }
        }
        sleep(Duration::from_secs(3)).await;
    }

    anyhow::bail!(
        "Agent did not become available within {} seconds",
        timeout_secs
    )
}

/// Check if VM is running (checks QEMU process first, then agent health)
pub async fn is_vm_running(config: &CliConfig) -> bool {
    // First check if there's a QEMU process running
    if !qemu::find_running_vms().is_empty() {
        return true;
    }

    // Fallback: check if agent is responding (for remote mode or external VM)
    let agent_url = format!("http://127.0.0.1:{}", config.vm.agent_port);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .ok();

    if let Some(client) = client {
        let health_url = format!("{}/health", agent_url);
        client.get(&health_url).send().await.is_ok()
    } else {
        false
    }
}
