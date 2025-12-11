use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Request to build a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildRequest {
    /// Project ID from sync
    pub project_id: Uuid,
    /// Relative path to .xcodeproj or .xcworkspace within project
    #[serde(default)]
    pub project_file: Option<String>,
    /// Xcode scheme to build
    pub scheme: String,
    /// Build configuration
    #[serde(default)]
    pub configuration: BuildConfiguration,
    /// Target device for build destination
    pub destination: BuildDestination,
    /// Additional xcodebuild arguments
    #[serde(default)]
    pub extra_args: Vec<String>,
    /// Clean build directory first
    #[serde(default)]
    pub clean: bool,
}

/// Build configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BuildConfiguration {
    #[default]
    Debug,
    Release,
}

impl std::fmt::Display for BuildConfiguration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildConfiguration::Debug => write!(f, "Debug"),
            BuildConfiguration::Release => write!(f, "Release"),
        }
    }
}

/// Build destination specifying target platform and device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildDestination {
    /// Platform (e.g., "iOS Simulator")
    #[serde(default = "default_platform")]
    pub platform: String,
    /// Device name (e.g., "iPhone 15 Pro")
    pub device_name: String,
    /// OS version (e.g., "17.0"), optional
    pub os_version: Option<String>,
}

fn default_platform() -> String {
    "iOS Simulator".to_string()
}

impl BuildDestination {
    /// Create a new iOS Simulator destination
    pub fn ios_simulator(device_name: impl Into<String>) -> Self {
        Self {
            platform: "iOS Simulator".to_string(),
            device_name: device_name.into(),
            os_version: None,
        }
    }

    /// Convert to xcodebuild -destination string
    pub fn to_xcodebuild_arg(&self) -> String {
        let mut dest = format!("platform={},name={}", self.platform, self.device_name);
        if let Some(ref os) = self.os_version {
            dest.push_str(&format!(",OS={}", os));
        }
        dest
    }
}

/// Response when build is started
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildResponse {
    /// Unique build identifier
    pub build_id: Uuid,
    /// Initial status
    pub status: BuildStatus,
    /// When build was queued
    pub started_at: DateTime<Utc>,
}

/// Build status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BuildStatus {
    /// Waiting to start
    Queued,
    /// Currently building
    Building,
    /// Build completed successfully
    Succeeded,
    /// Build failed
    Failed,
    /// Build was cancelled
    Cancelled,
}

/// Detailed build status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildStatusResponse {
    pub build_id: Uuid,
    pub project_id: Uuid,
    pub scheme: String,
    pub status: BuildStatus,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    /// Path to built .app bundle (if succeeded)
    pub app_path: Option<String>,
    /// Bundle identifier of the app
    pub bundle_id: Option<String>,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// Build warnings
    #[serde(default)]
    pub warnings: Vec<String>,
    /// Build duration in seconds
    pub duration_secs: Option<f64>,
}
