use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Log message streamed from agent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LogMessage {
    /// Output from xcodebuild during build
    BuildOutput {
        timestamp: DateTime<Utc>,
        level: LogLevel,
        message: String,
    },
    /// Log from running app
    AppLog {
        timestamp: DateTime<Utc>,
        process: String,
        subsystem: Option<String>,
        category: Option<String>,
        message: String,
    },
    /// System event (build started, completed, etc.)
    SystemEvent {
        timestamp: DateTime<Utc>,
        event: SystemEventType,
        message: String,
    },
    /// Build phase progress
    BuildProgress {
        timestamp: DateTime<Utc>,
        phase: String,
        target: Option<String>,
        progress_percent: Option<u8>,
    },
}

/// Log severity level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

/// System event types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SystemEventType {
    BuildQueued,
    BuildStarted,
    BuildSucceeded,
    BuildFailed,
    BuildCancelled,
    SimulatorBooting,
    SimulatorBooted,
    SimulatorShutdown,
    AppInstalling,
    AppInstalled,
    AppLaunching,
    AppLaunched,
    AppCrashed,
    AppExited,
}

impl LogMessage {
    pub fn build_output(level: LogLevel, message: impl Into<String>) -> Self {
        Self::BuildOutput {
            timestamp: Utc::now(),
            level,
            message: message.into(),
        }
    }

    pub fn system_event(event: SystemEventType, message: impl Into<String>) -> Self {
        Self::SystemEvent {
            timestamp: Utc::now(),
            event,
            message: message.into(),
        }
    }

    pub fn build_progress(phase: impl Into<String>, target: Option<String>, progress: Option<u8>) -> Self {
        Self::BuildProgress {
            timestamp: Utc::now(),
            phase: phase.into(),
            target,
            progress_percent: progress,
        }
    }
}
