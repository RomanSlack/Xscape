use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// CLI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    /// Agent configuration
    pub agent: AgentConfig,
    /// VM configuration (for local-vm mode)
    #[serde(default)]
    pub vm: VmConfig,
    /// VNC/noVNC configuration
    #[serde(default)]
    pub vnc: VncConfig,
    /// Project sync settings
    #[serde(default)]
    pub project: ProjectConfig,
    /// Default simulator settings
    #[serde(default)]
    pub simulator: SimulatorConfig,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            agent: AgentConfig::default(),
            vm: VmConfig::default(),
            vnc: VncConfig::default(),
            project: ProjectConfig::default(),
            simulator: SimulatorConfig::default(),
        }
    }
}

/// Agent connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Connection mode
    #[serde(default)]
    pub mode: AgentMode,
    /// Remote Mac host (when mode = remote)
    #[serde(default = "default_remote_host")]
    pub remote_host: String,
    /// Remote Mac port (when mode = remote)
    #[serde(default = "default_agent_port")]
    pub remote_port: u16,
    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            mode: AgentMode::default(),
            remote_host: default_remote_host(),
            remote_port: default_agent_port(),
            timeout_secs: default_timeout(),
        }
    }
}

/// How to connect to the agent
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AgentMode {
    /// Connect to a remote Mac over the network
    Remote,
    /// Use a local macOS VM managed by QEMU/KVM
    #[default]
    LocalVm,
}

/// QEMU VM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    /// Path to QEMU binary
    #[serde(default = "default_qemu_path")]
    pub qemu_path: PathBuf,
    /// Path to macOS disk image
    #[serde(default)]
    pub disk_image: PathBuf,
    /// Path to OVMF UEFI firmware
    #[serde(default = "default_ovmf_path")]
    pub ovmf_code: PathBuf,
    /// Memory allocation (e.g., "8G")
    #[serde(default = "default_memory")]
    pub memory: String,
    /// Number of CPU cores
    #[serde(default = "default_cpus")]
    pub cpus: u32,
    /// VNC display port
    #[serde(default = "default_vnc_port")]
    pub vnc_port: u16,
    /// Host SSH port forwarding
    #[serde(default = "default_ssh_port")]
    pub ssh_port: u16,
    /// Host agent port forwarding
    #[serde(default = "default_agent_port")]
    pub agent_port: u16,
    /// Wait timeout for VM to boot (seconds)
    #[serde(default = "default_boot_timeout")]
    pub boot_timeout_secs: u64,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            qemu_path: default_qemu_path(),
            disk_image: PathBuf::new(),
            ovmf_code: default_ovmf_path(),
            memory: default_memory(),
            cpus: default_cpus(),
            vnc_port: default_vnc_port(),
            ssh_port: default_ssh_port(),
            agent_port: default_agent_port(),
            boot_timeout_secs: default_boot_timeout(),
        }
    }
}

/// VNC/noVNC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VncConfig {
    /// Path to noVNC installation
    #[serde(default = "default_novnc_path")]
    pub novnc_path: PathBuf,
    /// websockify port for noVNC
    #[serde(default = "default_websockify_port")]
    pub websockify_port: u16,
    /// Auto-open browser when starting VNC
    #[serde(default = "default_true")]
    pub auto_open_browser: bool,
}

impl Default for VncConfig {
    fn default() -> Self {
        Self {
            novnc_path: default_novnc_path(),
            websockify_port: default_websockify_port(),
            auto_open_browser: true,
        }
    }
}

/// Project sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Default scheme to build (if not specified)
    #[serde(default)]
    pub default_scheme: Option<String>,
    /// Patterns to exclude from sync (in addition to .gitignore)
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            default_scheme: None,
            exclude_patterns: default_exclude_patterns(),
        }
    }
}

/// Default simulator settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatorConfig {
    /// Preferred device name
    #[serde(default = "default_device")]
    pub preferred_device: String,
    /// Preferred iOS runtime version
    #[serde(default)]
    pub preferred_runtime: Option<String>,
}

impl Default for SimulatorConfig {
    fn default() -> Self {
        Self {
            preferred_device: default_device(),
            preferred_runtime: None,
        }
    }
}

// Default value functions
fn default_remote_host() -> String {
    "localhost".to_string()
}

fn default_agent_port() -> u16 {
    8080
}

fn default_timeout() -> u64 {
    30
}

fn default_qemu_path() -> PathBuf {
    PathBuf::from("/usr/bin/qemu-system-x86_64")
}

fn default_ovmf_path() -> PathBuf {
    PathBuf::from("/usr/share/OVMF/OVMF_CODE.fd")
}

fn default_memory() -> String {
    "8G".to_string()
}

fn default_cpus() -> u32 {
    4
}

fn default_vnc_port() -> u16 {
    5900
}

fn default_ssh_port() -> u16 {
    2222
}

fn default_boot_timeout() -> u64 {
    180
}

fn default_novnc_path() -> PathBuf {
    PathBuf::from("/opt/novnc")
}

fn default_websockify_port() -> u16 {
    6080
}

fn default_true() -> bool {
    true
}

fn default_exclude_patterns() -> Vec<String> {
    vec![
        ".git".to_string(),
        "build".to_string(),
        "DerivedData".to_string(),
        "*.xcuserstate".to_string(),
        "Pods".to_string(),
        ".build".to_string(),
        "*.o".to_string(),
        "*.a".to_string(),
    ]
}

fn default_device() -> String {
    "iPhone 15 Pro".to_string()
}

/// Agent server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentServerConfig {
    /// Server host to bind to
    #[serde(default = "default_bind_host")]
    pub host: String,
    /// Server port
    #[serde(default = "default_agent_port")]
    pub port: u16,
    /// Storage configuration
    #[serde(default)]
    pub storage: StorageConfig,
    /// Xcode configuration
    #[serde(default)]
    pub xcode: XcodeConfig,
    /// Simulator configuration
    #[serde(default)]
    pub simulator: AgentSimulatorConfig,
}

impl Default for AgentServerConfig {
    fn default() -> Self {
        Self {
            host: default_bind_host(),
            port: default_agent_port(),
            storage: StorageConfig::default(),
            xcode: XcodeConfig::default(),
            simulator: AgentSimulatorConfig::default(),
        }
    }
}

fn default_bind_host() -> String {
    "0.0.0.0".to_string()
}

/// Agent storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Directory to store synced projects
    #[serde(default = "default_projects_dir")]
    pub projects_dir: PathBuf,
    /// Directory for logs
    #[serde(default = "default_logs_dir")]
    pub logs_dir: PathBuf,
    /// Maximum number of projects to cache
    #[serde(default = "default_max_projects")]
    pub max_projects: usize,
    /// Clean up projects older than this (hours)
    #[serde(default = "default_cleanup_hours")]
    pub cleanup_after_hours: u32,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            projects_dir: default_projects_dir(),
            logs_dir: default_logs_dir(),
            max_projects: default_max_projects(),
            cleanup_after_hours: default_cleanup_hours(),
        }
    }
}

fn default_projects_dir() -> PathBuf {
    PathBuf::from("/var/xcode-agent/projects")
}

fn default_logs_dir() -> PathBuf {
    PathBuf::from("/var/xcode-agent/logs")
}

fn default_max_projects() -> usize {
    10
}

fn default_cleanup_hours() -> u32 {
    24
}

/// Xcode configuration for agent
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct XcodeConfig {
    /// Path to Xcode.app (auto-detected if not specified)
    #[serde(default)]
    pub path: Option<PathBuf>,
    /// Custom DerivedData path
    #[serde(default)]
    pub derived_data_path: Option<PathBuf>,
}

/// Agent simulator management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSimulatorConfig {
    /// Auto-boot simulator when needed
    #[serde(default = "default_true")]
    pub auto_boot: bool,
    /// Shutdown simulator after idle for this many minutes
    #[serde(default = "default_idle_shutdown")]
    pub shutdown_idle_after_minutes: u32,
}

impl Default for AgentSimulatorConfig {
    fn default() -> Self {
        Self {
            auto_boot: true,
            shutdown_idle_after_minutes: default_idle_shutdown(),
        }
    }
}

fn default_idle_shutdown() -> u32 {
    30
}
