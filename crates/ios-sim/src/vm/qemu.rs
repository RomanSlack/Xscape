use anyhow::{Context, Result};
use ios_sim_common::VmConfig;
use std::process::{Child, Command, Stdio};
use tracing::{debug, info, warn};

/// QEMU VM manager
pub struct QemuVm {
    config: VmConfig,
    process: Option<Child>,
}

impl QemuVm {
    pub fn new(config: VmConfig) -> Self {
        Self {
            config,
            process: None,
        }
    }

    /// Start the QEMU VM
    pub fn start(&mut self, headless: bool) -> Result<()> {
        if self.config.disk_image.as_os_str().is_empty() {
            anyhow::bail!("VM disk image path not configured. Run 'ios-sim config set vm.disk_image /path/to/macos.qcow2'");
        }

        if !self.config.disk_image.exists() {
            anyhow::bail!(
                "VM disk image not found: {:?}",
                self.config.disk_image
            );
        }

        info!("Starting macOS VM...");

        let mut cmd = Command::new(&self.config.qemu_path);

        // Machine type for macOS
        cmd.args([
            "-machine",
            "q35,accel=kvm",
            "-cpu",
            "Penryn,kvm=on,vendor=GenuineIntel,+invtsc,vmware-cpuid-freq=on,+pcid,+ssse3,+sse4.2,+popcnt,+avx,+aes,+xsave,+xsaveopt,check",
        ]);

        // Memory and CPUs
        cmd.args(["-m", &self.config.memory]);
        cmd.args([
            "-smp",
            &format!("cpus={},cores={},threads=1,sockets=1", self.config.cpus, self.config.cpus),
        ]);

        // UEFI firmware
        if self.config.ovmf_code.exists() {
            cmd.args([
                "-drive",
                &format!(
                    "if=pflash,format=raw,readonly=on,file={}",
                    self.config.ovmf_code.display()
                ),
            ]);
        }

        // Disk image
        cmd.args([
            "-drive",
            &format!(
                "id=disk0,if=virtio,format=qcow2,file={}",
                self.config.disk_image.display()
            ),
        ]);

        // Networking with port forwarding
        cmd.args([
            "-netdev",
            &format!(
                "user,id=net0,hostfwd=tcp::{}-:22,hostfwd=tcp::{}-:8080",
                self.config.ssh_port, self.config.agent_port
            ),
            "-device",
            "virtio-net,netdev=net0",
        ]);

        // VNC display
        let vnc_display = self.config.vnc_port.saturating_sub(5900);
        cmd.args(["-vnc", &format!(":{}", vnc_display)]);

        if headless {
            cmd.args(["-display", "none"]);
        }

        // USB (macOS needs this)
        cmd.args(["-usb", "-device", "usb-tablet"]);

        // Audio (helps with macOS boot)
        cmd.args(["-device", "ich9-intel-hda", "-device", "hda-output"]);

        debug!("QEMU command: {:?}", cmd);

        cmd.stdout(Stdio::null()).stderr(Stdio::null());

        let child = cmd.spawn().context("Failed to start QEMU")?;

        info!(
            "VM started (VNC: {}, SSH: {}, Agent: {})",
            self.config.vnc_port, self.config.ssh_port, self.config.agent_port
        );

        self.process = Some(child);
        Ok(())
    }

    /// Stop the VM gracefully
    pub fn stop(&mut self) -> Result<()> {
        if let Some(ref mut child) = self.process {
            info!("Stopping VM...");

            // Try graceful shutdown first (SIGTERM)
            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                unsafe {
                    libc::kill(child.id() as i32, libc::SIGTERM);
                }
            }

            // Wait a bit
            std::thread::sleep(std::time::Duration::from_secs(5));

            // Force kill if still running
            match child.try_wait() {
                Ok(Some(_)) => {
                    info!("VM stopped gracefully");
                }
                Ok(None) => {
                    warn!("VM didn't stop gracefully, force killing...");
                    let _ = child.kill();
                }
                Err(e) => {
                    warn!("Error checking VM status: {}", e);
                }
            }

            self.process = None;
        }
        Ok(())
    }

    /// Check if VM process is running
    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.process {
            match child.try_wait() {
                Ok(None) => true, // Still running
                _ => false,
            }
        } else {
            false
        }
    }

    /// Get agent URL
    pub fn agent_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.config.agent_port)
    }
}

impl Drop for QemuVm {
    fn drop(&mut self) {
        // Don't stop the VM when the handle is dropped
        // Let it run in the background
    }
}

/// Find running QEMU processes (for status check when we didn't start it)
pub fn find_running_vms() -> Vec<u32> {
    let mut pids = Vec::new();

    #[cfg(unix)]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("pgrep").arg("-f").arg("qemu-system").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Ok(pid) = line.trim().parse::<u32>() {
                    pids.push(pid);
                }
            }
        }
    }

    pids
}

/// Kill a QEMU process by PID
#[cfg(unix)]
pub fn kill_vm(pid: u32) -> Result<()> {
    unsafe {
        if libc::kill(pid as i32, libc::SIGTERM) != 0 {
            anyhow::bail!("Failed to send SIGTERM to PID {}", pid);
        }
    }
    Ok(())
}

#[cfg(not(unix))]
pub fn kill_vm(_pid: u32) -> Result<()> {
    anyhow::bail!("VM management not supported on this platform")
}
