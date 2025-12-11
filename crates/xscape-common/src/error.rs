use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur in ios-sim operations
#[derive(Debug, Error)]
pub enum IosSimError {
    #[error("Xcode not found or not configured at expected path")]
    XcodeNotFound,

    #[error("Xcode command line tools not installed")]
    XcodeToolsNotInstalled,

    #[error("Simulator not found: {0}")]
    SimulatorNotFound(String),

    #[error("Simulator runtime not available: {0}")]
    RuntimeNotAvailable(String),

    #[error("Build failed: {0}")]
    BuildFailed(String),

    #[error("Project not found: {0}")]
    ProjectNotFound(Uuid),

    #[error("Invalid project structure: {0}")]
    InvalidProject(String),

    #[error("No .xcodeproj or .xcworkspace found in project")]
    NoXcodeProject,

    #[error("Scheme not found: {0}")]
    SchemeNotFound(String),

    #[error("Agent communication error: {0}")]
    AgentError(String),

    #[error("Agent not reachable at {0}")]
    AgentUnreachable(String),

    #[error("VM error: {0}")]
    VmError(String),

    #[error("VM not running")]
    VmNotRunning,

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Timeout waiting for {0}")]
    Timeout(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// API error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    /// Machine-readable error code
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Additional error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ApiError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    // Common error constructors
    pub fn not_found(resource: &str, id: &str) -> Self {
        Self::new(
            "NOT_FOUND",
            format!("{} not found: {}", resource, id),
        )
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new("BAD_REQUEST", message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("INTERNAL_ERROR", message)
    }
}

impl From<IosSimError> for ApiError {
    fn from(err: IosSimError) -> Self {
        let code = match &err {
            IosSimError::XcodeNotFound => "XCODE_NOT_FOUND",
            IosSimError::XcodeToolsNotInstalled => "XCODE_TOOLS_NOT_INSTALLED",
            IosSimError::SimulatorNotFound(_) => "SIMULATOR_NOT_FOUND",
            IosSimError::RuntimeNotAvailable(_) => "RUNTIME_NOT_AVAILABLE",
            IosSimError::BuildFailed(_) => "BUILD_FAILED",
            IosSimError::ProjectNotFound(_) => "PROJECT_NOT_FOUND",
            IosSimError::InvalidProject(_) => "INVALID_PROJECT",
            IosSimError::NoXcodeProject => "NO_XCODE_PROJECT",
            IosSimError::SchemeNotFound(_) => "SCHEME_NOT_FOUND",
            IosSimError::AgentError(_) => "AGENT_ERROR",
            IosSimError::AgentUnreachable(_) => "AGENT_UNREACHABLE",
            IosSimError::VmError(_) => "VM_ERROR",
            IosSimError::VmNotRunning => "VM_NOT_RUNNING",
            IosSimError::ConfigError(_) => "CONFIG_ERROR",
            IosSimError::Timeout(_) => "TIMEOUT",
            IosSimError::Io(_) => "IO_ERROR",
            IosSimError::Json(_) => "JSON_ERROR",
        };

        ApiError {
            code: code.to_string(),
            message: err.to_string(),
            details: None,
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}
