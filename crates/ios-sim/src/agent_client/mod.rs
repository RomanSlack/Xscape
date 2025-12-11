use anyhow::{Context, Result};
use ios_sim_common::{
    ApiError, BootSimulatorRequest, BootSimulatorResponse, BuildRequest, BuildResponse,
    BuildStatusResponse, HealthResponse, ListSimulatorsResponse, RunAppRequest, RunAppResponse,
    ShutdownSimulatorRequest, SyncProjectResponse,
};
use reqwest::multipart::{Form, Part};
use std::time::Duration;
use tracing::debug;
use uuid::Uuid;

/// HTTP client for communicating with xcode-agent
pub struct AgentClient {
    client: reqwest::Client,
    base_url: String,
}

impl AgentClient {
    pub fn new(base_url: &str, timeout_secs: u64) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Get agent base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Health check
    pub async fn health(&self) -> Result<HealthResponse> {
        let url = format!("{}/health", self.base_url);
        debug!("GET {}", url);

        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to connect to agent")?;

        if !response.status().is_success() {
            let error: ApiError = response.json().await
                .unwrap_or_else(|_| ApiError::new("UNKNOWN", "Unknown error"));
            anyhow::bail!("Agent error: {}", error);
        }

        response.json().await.context("Failed to parse health response")
    }

    /// Sync project to agent
    pub async fn sync_project(
        &self,
        project_name: &str,
        checksum: &str,
        tarball_data: Vec<u8>,
    ) -> Result<SyncProjectResponse> {
        let url = format!("{}/sync-project", self.base_url);
        debug!("POST {} ({} bytes)", url, tarball_data.len());

        let form = Form::new()
            .text("project_name", project_name.to_string())
            .text("checksum", checksum.to_string())
            .part("tarball", Part::bytes(tarball_data).file_name("project.tar.gz"));

        let response = self.client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .context("Failed to sync project")?;

        if !response.status().is_success() {
            let error: ApiError = response.json().await
                .unwrap_or_else(|_| ApiError::new("UNKNOWN", "Unknown error"));
            anyhow::bail!("Sync failed: {}", error);
        }

        response.json().await.context("Failed to parse sync response")
    }

    /// Start a build
    pub async fn build(&self, request: &BuildRequest) -> Result<BuildResponse> {
        let url = format!("{}/build", self.base_url);
        debug!("POST {}", url);

        let response = self.client
            .post(&url)
            .json(request)
            .send()
            .await
            .context("Failed to start build")?;

        if !response.status().is_success() {
            let error: ApiError = response.json().await
                .unwrap_or_else(|_| ApiError::new("UNKNOWN", "Unknown error"));
            anyhow::bail!("Build failed: {}", error);
        }

        response.json().await.context("Failed to parse build response")
    }

    /// Get build status
    pub async fn get_build_status(&self, build_id: Uuid) -> Result<BuildStatusResponse> {
        let url = format!("{}/build/{}", self.base_url, build_id);
        debug!("GET {}", url);

        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to get build status")?;

        if !response.status().is_success() {
            let error: ApiError = response.json().await
                .unwrap_or_else(|_| ApiError::new("UNKNOWN", "Unknown error"));
            anyhow::bail!("Failed to get build status: {}", error);
        }

        response.json().await.context("Failed to parse build status")
    }

    /// List simulators
    pub async fn list_simulators(&self) -> Result<ListSimulatorsResponse> {
        let url = format!("{}/simulator/list", self.base_url);
        debug!("GET {}", url);

        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to list simulators")?;

        if !response.status().is_success() {
            let error: ApiError = response.json().await
                .unwrap_or_else(|_| ApiError::new("UNKNOWN", "Unknown error"));
            anyhow::bail!("Failed to list simulators: {}", error);
        }

        response.json().await.context("Failed to parse simulators response")
    }

    /// Boot simulator
    pub async fn boot_simulator(&self, device_udid: &str) -> Result<BootSimulatorResponse> {
        let url = format!("{}/simulator/boot", self.base_url);
        debug!("POST {}", url);

        let request = BootSimulatorRequest {
            device_udid: device_udid.to_string(),
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to boot simulator")?;

        if !response.status().is_success() {
            let error: ApiError = response.json().await
                .unwrap_or_else(|_| ApiError::new("UNKNOWN", "Unknown error"));
            anyhow::bail!("Failed to boot simulator: {}", error);
        }

        response.json().await.context("Failed to parse boot response")
    }

    /// Run app in simulator
    pub async fn run_app(&self, request: &RunAppRequest) -> Result<RunAppResponse> {
        let url = format!("{}/simulator/run", self.base_url);
        debug!("POST {}", url);

        let response = self.client
            .post(&url)
            .json(request)
            .send()
            .await
            .context("Failed to run app")?;

        if !response.status().is_success() {
            let error: ApiError = response.json().await
                .unwrap_or_else(|_| ApiError::new("UNKNOWN", "Unknown error"));
            anyhow::bail!("Failed to run app: {}", error);
        }

        response.json().await.context("Failed to parse run response")
    }

    /// Shutdown simulator
    pub async fn shutdown_simulator(&self, device_udid: &str) -> Result<BootSimulatorResponse> {
        let url = format!("{}/simulator/shutdown", self.base_url);
        debug!("POST {}", url);

        let request = ShutdownSimulatorRequest {
            device_udid: device_udid.to_string(),
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to shutdown simulator")?;

        if !response.status().is_success() {
            let error: ApiError = response.json().await
                .unwrap_or_else(|_| ApiError::new("UNKNOWN", "Unknown error"));
            anyhow::bail!("Failed to shutdown simulator: {}", error);
        }

        response.json().await.context("Failed to parse shutdown response")
    }

    /// Check if agent is reachable
    pub async fn is_reachable(&self) -> bool {
        self.health().await.is_ok()
    }
}
