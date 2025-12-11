use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request to sync a project to the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProjectRequest {
    /// Human-readable project name
    pub project_name: String,
    /// SHA256 checksum of the tarball for deduplication
    pub checksum: String,
}

/// Response after project sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProjectResponse {
    /// Unique identifier for this project on the agent
    pub project_id: Uuid,
    /// Path where project was extracted on the agent
    pub path: String,
    /// Number of files extracted
    pub files_extracted: u32,
    /// Whether the project was already cached (same checksum)
    pub was_cached: bool,
}

/// Project info stored on the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub project_id: Uuid,
    pub project_name: String,
    pub checksum: String,
    pub path: String,
    pub synced_at: chrono::DateTime<chrono::Utc>,
}
