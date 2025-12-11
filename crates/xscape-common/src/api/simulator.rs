use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

/// A simulator device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatorDevice {
    /// Device UDID
    pub udid: String,
    /// Device name (e.g., "iPhone 15 Pro")
    pub name: String,
    /// Device type identifier
    pub device_type_identifier: String,
    /// Runtime identifier
    pub runtime_identifier: String,
    /// Human-readable runtime (e.g., "iOS 17.0")
    pub runtime: String,
    /// Current state
    pub state: SimulatorState,
    /// Whether device is available
    pub is_available: bool,
}

/// Simulator device state
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SimulatorState {
    Shutdown,
    Booted,
    Booting,
    ShuttingDown,
}

impl Default for SimulatorState {
    fn default() -> Self {
        Self::Shutdown
    }
}

/// A simulator runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatorRuntime {
    /// Runtime identifier (e.g., "com.apple.CoreSimulator.SimRuntime.iOS-17-0")
    pub identifier: String,
    /// Human-readable name (e.g., "iOS 17.0")
    pub name: String,
    /// Version string
    pub version: String,
    /// Build version
    pub build_version: String,
    /// Whether runtime is available
    pub is_available: bool,
}

/// Response listing available simulators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListSimulatorsResponse {
    pub devices: Vec<SimulatorDevice>,
    pub runtimes: Vec<SimulatorRuntime>,
}

/// Request to boot a simulator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootSimulatorRequest {
    /// Device UDID to boot
    pub device_udid: String,
}

/// Response after booting simulator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootSimulatorResponse {
    pub device_udid: String,
    pub state: SimulatorState,
}

/// Request to run an app in simulator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunAppRequest {
    /// Build ID to get the app from
    pub build_id: Uuid,
    /// Device UDID to run on
    pub device_udid: String,
    /// Arguments to pass to the app
    #[serde(default)]
    pub launch_args: Vec<String>,
    /// Environment variables for the app
    #[serde(default)]
    pub environment: HashMap<String, String>,
    /// Wait for app to exit (vs launch and return immediately)
    #[serde(default)]
    pub wait_for_exit: bool,
}

/// Response after launching app
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunAppResponse {
    /// Session ID for this run
    pub session_id: Uuid,
    /// Bundle ID of launched app
    pub bundle_id: String,
    /// Process ID (if available)
    pub pid: Option<u32>,
    /// Device UDID where app is running
    pub device_udid: String,
}

/// Request to shutdown a simulator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShutdownSimulatorRequest {
    /// Device UDID to shutdown
    pub device_udid: String,
}
