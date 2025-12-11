use serde::{Deserialize, Serialize};

/// Health check response from the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub xcode_version: Option<String>,
    pub xcode_path: Option<String>,
    pub available_simulators: u32,
    pub agent_version: String,
}

/// Agent health status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// All systems operational
    Healthy,
    /// Partial functionality available
    Degraded,
    /// Critical systems unavailable
    Unhealthy,
}

impl Default for HealthResponse {
    fn default() -> Self {
        Self {
            status: HealthStatus::Unhealthy,
            xcode_version: None,
            xcode_path: None,
            available_simulators: 0,
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}
