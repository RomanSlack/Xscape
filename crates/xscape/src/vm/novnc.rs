use anyhow::{Context, Result};
use xscape_common::VncConfig;
use std::process::{Child, Command, Stdio};
use tracing::{debug, info};

/// noVNC proxy manager
pub struct NoVncProxy {
    config: VncConfig,
    vnc_port: u16,
    process: Option<Child>,
}

impl NoVncProxy {
    pub fn new(config: VncConfig, vnc_port: u16) -> Self {
        Self {
            config,
            vnc_port,
            process: None,
        }
    }

    /// Start websockify for noVNC
    pub fn start(&mut self) -> Result<()> {
        let websockify_path = self.config.novnc_path.join("utils/websockify/run");

        if !websockify_path.exists() {
            // Try alternative location
            let alt_path = self.config.novnc_path.join("utils/novnc_proxy");
            if alt_path.exists() {
                return self.start_with_path(&alt_path);
            }

            // Try system websockify
            return self.start_system_websockify();
        }

        self.start_with_path(&websockify_path)
    }

    fn start_with_path(&mut self, websockify_path: &std::path::Path) -> Result<()> {
        info!(
            "Starting noVNC proxy on port {} -> VNC port {}",
            self.config.websockify_port, self.vnc_port
        );

        let mut cmd = Command::new(websockify_path);
        cmd.args([
            "--web",
            self.config.novnc_path.to_str().unwrap(),
            &self.config.websockify_port.to_string(),
            &format!("localhost:{}", self.vnc_port),
        ]);

        debug!("websockify command: {:?}", cmd);

        cmd.stdout(Stdio::null()).stderr(Stdio::null());

        let child = cmd.spawn().context("Failed to start websockify")?;
        self.process = Some(child);

        Ok(())
    }

    fn start_system_websockify(&mut self) -> Result<()> {
        info!(
            "Starting system websockify on port {} -> VNC port {}",
            self.config.websockify_port, self.vnc_port
        );

        let mut cmd = Command::new("websockify");
        cmd.args([
            &self.config.websockify_port.to_string(),
            &format!("localhost:{}", self.vnc_port),
        ]);

        cmd.stdout(Stdio::null()).stderr(Stdio::null());

        let child = cmd.spawn().context(
            "Failed to start websockify. Install with: apt install websockify or pip install websockify",
        )?;
        self.process = Some(child);

        Ok(())
    }

    /// Stop the proxy
    pub fn stop(&mut self) {
        if let Some(ref mut child) = self.process {
            let _ = child.kill();
            self.process = None;
        }
    }

    /// Get the noVNC URL
    pub fn url(&self) -> String {
        format!(
            "http://localhost:{}/vnc.html?host=localhost&port={}&autoconnect=true",
            self.config.websockify_port, self.config.websockify_port
        )
    }

    /// Get just the base URL (for when noVNC isn't installed but websockify is)
    pub fn websockify_url(&self) -> String {
        format!("ws://localhost:{}", self.config.websockify_port)
    }

    /// Check if proxy is running
    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.process {
            child.try_wait().ok().flatten().is_none()
        } else {
            false
        }
    }
}

impl Drop for NoVncProxy {
    fn drop(&mut self) {
        self.stop();
    }
}
