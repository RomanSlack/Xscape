use anyhow::Result;
use xscape_common::{AgentServerConfig, BuildStatusResponse, ProjectInfo};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::storage::BuildArtifacts;

/// Shared application state
pub struct AppState {
    pub config: AgentServerConfig,
    /// Cached project information
    pub projects: RwLock<HashMap<Uuid, ProjectInfo>>,
    /// Active builds
    pub builds: RwLock<HashMap<Uuid, BuildStatusResponse>>,
    /// Build artifacts (app paths, etc.)
    pub artifacts: RwLock<HashMap<Uuid, BuildArtifacts>>,
    /// Log subscribers per build (for streaming)
    pub log_subscribers: RwLock<HashMap<Uuid, Vec<tokio::sync::broadcast::Sender<String>>>>,
}

impl AppState {
    pub async fn new(config: AgentServerConfig) -> Result<Self> {
        Ok(Self {
            config,
            projects: RwLock::new(HashMap::new()),
            builds: RwLock::new(HashMap::new()),
            artifacts: RwLock::new(HashMap::new()),
            log_subscribers: RwLock::new(HashMap::new()),
        })
    }

    /// Get project by ID
    pub async fn get_project(&self, id: &Uuid) -> Option<ProjectInfo> {
        self.projects.read().await.get(id).cloned()
    }

    /// Store project info
    pub async fn store_project(&self, project: ProjectInfo) {
        self.projects.write().await.insert(project.project_id, project);
    }

    /// Get build status
    pub async fn get_build(&self, id: &Uuid) -> Option<BuildStatusResponse> {
        self.builds.read().await.get(id).cloned()
    }

    /// Store build status
    pub async fn store_build(&self, build: BuildStatusResponse) {
        self.builds.write().await.insert(build.build_id, build);
    }

    /// Get build artifacts
    pub async fn get_artifacts(&self, build_id: &Uuid) -> Option<BuildArtifacts> {
        self.artifacts.read().await.get(build_id).cloned()
    }

    /// Store build artifacts
    pub async fn store_artifacts(&self, build_id: Uuid, artifacts: BuildArtifacts) {
        self.artifacts.write().await.insert(build_id, artifacts);
    }

    /// Create a log broadcast channel for a build
    pub async fn create_log_channel(&self, build_id: Uuid) -> tokio::sync::broadcast::Sender<String> {
        let (tx, _) = tokio::sync::broadcast::channel(1000);
        self.log_subscribers
            .write()
            .await
            .entry(build_id)
            .or_insert_with(Vec::new)
            .push(tx.clone());
        tx
    }

    /// Get log sender for a build
    pub async fn get_log_sender(&self, build_id: &Uuid) -> Option<tokio::sync::broadcast::Sender<String>> {
        self.log_subscribers
            .read()
            .await
            .get(build_id)
            .and_then(|senders| senders.first().cloned())
    }
}
