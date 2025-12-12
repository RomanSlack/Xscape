use anyhow::{Context, Result};
use xscape_common::VmConfig;
use std::process::{Child, Command, Stdio};
use tracing::{debug, info, warn};

/// QEMU VM manager (OSX-KVM compatible)
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

    /// Start the QEMU VM using OSX-KVM compatible configuration
    pub fn start(&mut self, headless: bool) -> Result<()> {
        // If boot_script is configured, use that instead
        if !self.config.boot_script.as_os_str().is_empty() {
            return self.start_with_script(headless);
        }

        // Validate required paths for manual QEMU config
        self.validate_config()?;

        info!("Starting macOS VM (OSX-KVM compatible)...");

        let mut cmd = Command::new(&self.config.qemu_path);

        // Enable KVM acceleration
        cmd.arg("-enable-kvm");

        // Memory (in MiB)
        cmd.args(["-m", &self.config.memory]);

        // CPU - Haswell-noTSX for macOS Sonoma compatibility
        let cpu_opts = "+ssse3,+sse4.2,+popcnt,+avx,+aes,+xsave,+xsaveopt,check";
        cmd.args([
            "-cpu",
            &format!("Haswell-noTSX,kvm=on,vendor=GenuineIntel,+invtsc,vmware-cpuid-freq=on,{}", cpu_opts),
        ]);

        // Machine type
        cmd.args(["-machine", "q35"]);

        // SMP - cores and threads
        cmd.args([
            "-smp",
            &format!("{},cores={},sockets=1", self.config.cpus, self.config.cpus),
        ]);

        // USB controllers and devices (required for macOS)
        cmd.args(["-device", "qemu-xhci,id=xhci"]);
        cmd.args(["-device", "usb-kbd,bus=xhci.0"]);
        cmd.args(["-device", "usb-tablet,bus=xhci.0"]);
        cmd.args(["-device", "usb-ehci,id=ehci"]);

        // Apple SMC (required for macOS)
        cmd.args([
            "-device",
            "isa-applesmc,osk=ourhardworkbythesewordsguardedpleasedontsteal(c)AppleComputerInc",
        ]);

        // OVMF UEFI firmware
        if self.config.ovmf_code.exists() {
            cmd.args([
                "-drive",
                &format!(
                    "if=pflash,format=raw,readonly=on,file={}",
                    self.config.ovmf_code.display()
                ),
            ]);
        }

        // OVMF vars (for resolution and other settings)
        if self.config.ovmf_vars.exists() {
            cmd.args([
                "-drive",
                &format!(
                    "if=pflash,format=raw,file={}",
                    self.config.ovmf_vars.display()
                ),
            ]);
        }

        // SMBIOS
        cmd.args(["-smbios", "type=2"]);

        // Audio (helps with macOS boot)
        cmd.args(["-device", "ich9-intel-hda"]);
        cmd.args(["-device", "hda-duplex"]);

        // SATA controller
        cmd.args(["-device", "ich9-ahci,id=sata"]);

        // OpenCore bootloader (snapshot mode so original isn't modified)
        if self.config.opencore_image.exists() {
            cmd.args([
                "-drive",
                &format!(
                    "id=OpenCoreBoot,if=none,snapshot=on,format=qcow2,file={}",
                    self.config.opencore_image.display()
                ),
            ]);
            cmd.args(["-device", "ide-hd,bus=sata.2,drive=OpenCoreBoot"]);
        }

        // BaseSystem recovery image
        if self.config.base_system_image.exists() {
            cmd.args([
                "-drive",
                &format!(
                    "id=InstallMedia,if=none,file={},format=raw",
                    self.config.base_system_image.display()
                ),
            ]);
            cmd.args(["-device", "ide-hd,bus=sata.3,drive=InstallMedia"]);
        }

        // Main macOS disk
        let disk_format = if self.config.disk_image.extension().map(|e| e == "qcow2").unwrap_or(false) {
            "qcow2"
        } else {
            "raw"
        };
        cmd.args([
            "-drive",
            &format!(
                "id=MacHDD,if=none,file={},format={}",
                self.config.disk_image.display(),
                disk_format
            ),
        ]);
        cmd.args(["-device", "ide-hd,bus=sata.4,drive=MacHDD"]);

        // Network with port forwarding for SSH and agent
        cmd.args([
            "-netdev",
            &format!(
                "user,id=net0,hostfwd=tcp::{}-:22,hostfwd=tcp::{}-:8080",
                self.config.ssh_port, self.config.agent_port
            ),
        ]);
        cmd.args([
            "-device",
            "virtio-net-pci,netdev=net0,id=net0,mac=52:54:00:c9:18:27",
        ]);

        // Graphics
        cmd.args(["-device", "vmware-svga"]);

        // VNC display
        let vnc_display = self.config.vnc_port.saturating_sub(5900);
        cmd.args(["-vnc", &format!(":{}", vnc_display)]);

        if headless {
            cmd.args(["-display", "none"]);
        }

        debug!("QEMU command: {:?}", cmd);

        // Capture stderr to show errors, but let stdout go to null
        cmd.stdout(Stdio::null()).stderr(Stdio::piped());

        let mut child = cmd.spawn().context("Failed to start QEMU")?;

        // Wait a moment to see if QEMU starts successfully
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Check if process is still running
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process exited - get the error
                let stderr = child.stderr.take();
                let error_msg = if let Some(mut err) = stderr {
                    use std::io::Read;
                    let mut buf = String::new();
                    err.read_to_string(&mut buf).ok();
                    buf
                } else {
                    String::new()
                };
                anyhow::bail!("QEMU exited immediately ({})\n{}", status, error_msg);
            }
            Ok(None) => {
                // Process still running - good!
                info!(
                    "VM started (VNC: {}, SSH: {}, Agent: {})",
                    self.config.vnc_port, self.config.ssh_port, self.config.agent_port
                );
            }
            Err(e) => {
                warn!("Could not check QEMU status: {}", e);
            }
        }

        self.process = Some(child);
        Ok(())
    }

    /// Validate the VM configuration
    fn validate_config(&self) -> Result<()> {
        // Check if we have OSX-KVM path set - if so, derive other paths
        if !self.config.osx_kvm_path.as_os_str().is_empty() {
            // Paths will be derived from osx_kvm_path
            if !self.config.osx_kvm_path.exists() {
                anyhow::bail!(
                    "OSX-KVM path not found: {:?}",
                    self.config.osx_kvm_path
                );
            }
            return Ok(());
        }

        // Otherwise check individual paths
        if self.config.disk_image.as_os_str().is_empty() {
            anyhow::bail!(
                "VM disk image path not configured.\n\
                 Set vm.osx_kvm_path to your OSX-KVM directory, or set vm.disk_image directly.\n\
                 Example: xscape config set vm.osx_kvm_path /home/user/OSX-KVM"
            );
        }

        if !self.config.disk_image.exists() {
            anyhow::bail!(
                "VM disk image not found: {:?}",
                self.config.disk_image
            );
        }

        Ok(())
    }

    /// Start VM using the configured boot script (e.g., OpenCore-Boot.sh)
    fn start_with_script(&mut self, _headless: bool) -> Result<()> {
        let script = &self.config.boot_script;

        if !script.exists() {
            anyhow::bail!("Boot script not found: {:?}", script);
        }

        info!("Starting macOS VM using boot script: {:?}", script);

        // Get the script's directory to run from there
        let script_dir = script.parent().unwrap_or(std::path::Path::new("."));

        let mut cmd = Command::new(script);
        cmd.current_dir(script_dir);

        // Inherit all stdio so user gets the QEMU monitor prompt
        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        debug!("Running boot script: {:?} in {:?}", script, script_dir);

        // This will block until QEMU exits (since we inherit stdio)
        let status = cmd.status().context("Failed to run boot script")?;

        if !status.success() {
            anyhow::bail!("Boot script exited with: {}", status);
        }

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
