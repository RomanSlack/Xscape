use anyhow::{Context, Result};
use xscape_common::CliConfig;
use std::path::PathBuf;
use tracing::debug;

/// Get the default config file path
pub fn default_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("xscape")
        .join("config.toml")
}

/// Load configuration from file or return defaults
pub fn load_config(path: &Option<PathBuf>) -> Result<CliConfig> {
    let config_path = path.clone().unwrap_or_else(default_config_path);

    if config_path.exists() {
        debug!("Loading config from {:?}", config_path);
        let content = std::fs::read_to_string(&config_path)
            .context("Failed to read config file")?;
        let config: CliConfig = toml::from_str(&content)
            .context("Failed to parse config file")?;
        Ok(config)
    } else {
        debug!("Config file not found, using defaults");
        Ok(CliConfig::default())
    }
}

/// Save configuration to file
pub fn save_config(config: &CliConfig, path: &Option<PathBuf>) -> Result<()> {
    let config_path = path.clone().unwrap_or_else(default_config_path);

    // Create parent directory
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .context("Failed to create config directory")?;
    }

    let content = toml::to_string_pretty(config)
        .context("Failed to serialize config")?;

    std::fs::write(&config_path, content)
        .context("Failed to write config file")?;

    Ok(())
}

/// Generate default config content for `config init`
pub fn generate_default_config() -> String {
    r#"# xscape configuration

[agent]
# Connection mode: "remote" or "local-vm"
mode = "local-vm"

# Remote Mac settings (when mode = "remote")
remote_host = "192.168.1.100"
remote_port = 8080

# Connection timeout in seconds
timeout_secs = 30

[vm]
# Path to QEMU binary
qemu_path = "/usr/bin/qemu-system-x86_64"

# Path to OSX-KVM directory (recommended - other paths derived automatically)
# osx_kvm_path = "/home/user/OSX-KVM"

# Individual paths (alternative to osx_kvm_path)
disk_image = ""              # Main macOS disk (mac_hdd_ng.img)
opencore_image = ""          # OpenCore bootloader (OpenCore/OpenCore.qcow2)
base_system_image = ""       # Recovery image (BaseSystem.img)
ovmf_code = ""               # UEFI firmware code (OVMF_CODE.fd)
ovmf_vars = ""               # UEFI firmware vars (OVMF_VARS-1920x1080.fd)

# VM resources
memory = "16384"             # Memory in MiB (16GB)
cpus = 8

# Ports
vnc_port = 5900
ssh_port = 2222
agent_port = 8080

# Boot timeout in seconds
boot_timeout_secs = 180

[vnc]
# Path to noVNC installation (optional)
novnc_path = "/usr/share/novnc"

# websockify port
websockify_port = 6080

# Auto-open browser
auto_open_browser = true

[project]
# Patterns to exclude from sync (in addition to .gitignore)
exclude_patterns = [
    ".git",
    "build",
    "DerivedData",
    "*.xcuserstate",
    "Pods",
    ".build",
]

[simulator]
# Preferred device name
preferred_device = "iPhone 15 Pro"

# Preferred iOS version (optional)
# preferred_runtime = "iOS 17.0"
"#.to_string()
}
