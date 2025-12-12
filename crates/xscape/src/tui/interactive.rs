use anyhow::{Context, Result};
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm, FuzzySelect, Input, Select};
use std::path::PathBuf;

use super::Styles;

/// Interactive project selector
pub struct ProjectSelector;

impl ProjectSelector {
    /// Browse and select an Xcode project
    pub fn select() -> Result<(PathBuf, String)> {
        Styles::header("Select Xcode Project");

        // Get project path
        let path: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Project directory")
            .default(".".to_string())
            .validate_with(|input: &String| -> Result<(), &str> {
                let path = PathBuf::from(shellexpand::tilde(input).to_string());
                if path.exists() {
                    Ok(())
                } else {
                    Err("Directory does not exist")
                }
            })
            .interact_text()?;

        let project_path = PathBuf::from(shellexpand::tilde(&path).to_string())
            .canonicalize()
            .context("Failed to resolve project path")?;

        // Find schemes
        let schemes = find_schemes(&project_path)?;

        if schemes.is_empty() {
            anyhow::bail!("No schemes found in project. Make sure it's a valid Xcode project.");
        }

        let scheme = if schemes.len() == 1 {
            Styles::info(&format!("Using scheme: {}", schemes[0]));
            schemes[0].clone()
        } else {
            let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
                .with_prompt("Select scheme")
                .items(&schemes)
                .default(0)
                .interact()?;
            schemes[selection].clone()
        };

        Ok((project_path, scheme))
    }

    /// Quick select from recent projects
    pub fn select_recent(recent: &[PathBuf]) -> Result<Option<PathBuf>> {
        if recent.is_empty() {
            return Ok(None);
        }

        let items: Vec<String> = recent
            .iter()
            .map(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| p.to_string_lossy().to_string())
            })
            .collect();

        let mut options = items.clone();
        options.push("Browse for project...".to_string());

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select project")
            .items(&options)
            .default(0)
            .interact()?;

        if selection == options.len() - 1 {
            Ok(None) // User wants to browse
        } else {
            Ok(Some(recent[selection].clone()))
        }
    }
}

/// Interactive simulator selector
pub struct SimulatorSelector;

impl SimulatorSelector {
    /// Select a simulator device (shows all devices)
    pub fn select(devices: &[DeviceInfo]) -> Result<String> {
        if devices.is_empty() {
            anyhow::bail!("No simulators available");
        }

        Styles::header("Select Simulator");

        // Build display items
        let items: Vec<String> = devices
            .iter()
            .map(|d| {
                let status = if d.is_booted { "[running]" } else { "" };
                if status.is_empty() {
                    format!("{} ({})", d.name, d.runtime)
                } else {
                    format!("{} ({}) {}", d.name, d.runtime, status)
                }
            })
            .collect();

        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select device")
            .items(&items)
            .default(0)
            .interact()?;

        Ok(devices[selection].udid.clone())
    }

    /// Select by first choosing iOS version, then device
    pub fn select_with_runtime(devices: &[DeviceInfo]) -> Result<String> {
        if devices.is_empty() {
            anyhow::bail!("No simulators available");
        }

        Styles::header("Select iOS Version");

        // Get unique runtimes and sort (newest first)
        let mut runtimes: Vec<String> = devices.iter().map(|d| d.runtime.clone()).collect();
        runtimes.sort();
        runtimes.dedup();
        runtimes.reverse(); // Newest versions first

        // Build runtime display with device counts
        let runtime_items: Vec<String> = runtimes
            .iter()
            .map(|r| {
                let count = devices.iter().filter(|d| &d.runtime == r).count();
                let booted = devices
                    .iter()
                    .filter(|d| &d.runtime == r && d.is_booted)
                    .count();
                if booted > 0 {
                    format!("{} ({} devices, {} running)", r, count, booted)
                } else {
                    format!("{} ({} devices)", r, count)
                }
            })
            .collect();

        let runtime_selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select iOS version")
            .items(&runtime_items)
            .default(0)
            .interact()?;

        let selected_runtime = &runtimes[runtime_selection];

        // Filter devices by selected runtime
        let filtered_devices: Vec<&DeviceInfo> = devices
            .iter()
            .filter(|d| &d.runtime == selected_runtime)
            .collect();

        Styles::header("Select Device");

        // Build device display items
        let device_items: Vec<String> = filtered_devices
            .iter()
            .map(|d| {
                if d.is_booted {
                    format!("{} [running]", d.name)
                } else {
                    d.name.clone()
                }
            })
            .collect();

        let device_selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select device")
            .items(&device_items)
            .default(0)
            .interact()?;

        Ok(filtered_devices[device_selection].udid.clone())
    }

    /// Quick select preferred device or let user choose with runtime selection
    pub fn select_or_default(devices: &[DeviceInfo], preferred: &str) -> Result<String> {
        // Try to find preferred device
        if let Some(device) = devices.iter().find(|d| d.name.contains(preferred)) {
            let use_preferred = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(format!("Use {} ({})? ", device.name, device.runtime))
                .default(true)
                .interact()?;

            if use_preferred {
                return Ok(device.udid.clone());
            }
        }

        // Let user select with runtime picker
        Self::select_with_runtime(devices)
    }
}

#[derive(Clone)]
pub struct DeviceInfo {
    pub udid: String,
    pub name: String,
    pub runtime: String,
    pub is_booted: bool,
}

/// Interactive main menu
pub struct MainMenu;

impl MainMenu {
    pub fn show() -> Result<MenuAction> {
        Styles::print_banner();

        let options = vec![
            "Run Project         Build and run in simulator",
            "Build Project       Build without running",
            "Manage Simulators   List, boot, shutdown devices",
            "VM Control          Start, stop, VNC access",
            "Settings            Configure xscape",
            "Setup Wizard        Verify installation",
            "Exit",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select action")
            .items(&options)
            .default(0)
            .interact()?;

        Ok(match selection {
            0 => MenuAction::Run,
            1 => MenuAction::Build,
            2 => MenuAction::Simulators,
            3 => MenuAction::Vm,
            4 => MenuAction::Settings,
            5 => MenuAction::Setup,
            _ => MenuAction::Exit,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuAction {
    Run,
    Build,
    Simulators,
    Vm,
    Settings,
    Setup,
    Exit,
}

/// Simulator management menu
pub struct SimulatorMenu;

impl SimulatorMenu {
    pub fn show() -> Result<SimulatorAction> {
        let options = vec![
            "List Devices      Show all available simulators",
            "Boot Device       Start a simulator",
            "Shutdown Device   Stop a simulator",
            "Back",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Simulator Management")
            .items(&options)
            .default(0)
            .interact()?;

        Ok(match selection {
            0 => SimulatorAction::List,
            1 => SimulatorAction::Boot,
            2 => SimulatorAction::Shutdown,
            _ => SimulatorAction::Back,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SimulatorAction {
    List,
    Boot,
    Shutdown,
    Back,
}

/// VM control menu
pub struct VmMenu;

impl VmMenu {
    pub fn show() -> Result<VmAction> {
        let options = vec![
            "Start VM      Boot the macOS VM",
            "Stop VM       Shutdown the VM",
            "Status        Check VM status",
            "Open VNC      View in browser",
            "Back",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("VM Control")
            .items(&options)
            .default(0)
            .interact()?;

        Ok(match selection {
            0 => VmAction::Start,
            1 => VmAction::Stop,
            2 => VmAction::Status,
            3 => VmAction::Vnc,
            _ => VmAction::Back,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VmAction {
    Start,
    Stop,
    Status,
    Vnc,
    Back,
}

/// Find Xcode schemes in a project
fn find_schemes(project_path: &PathBuf) -> Result<Vec<String>> {
    let mut schemes = Vec::new();

    // Look for shared schemes
    for entry in std::fs::read_dir(project_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map_or(false, |e| e == "xcodeproj" || e == "xcworkspace") {
            let schemes_dir = path.join("xcshareddata/xcschemes");
            if schemes_dir.exists() {
                for scheme_entry in std::fs::read_dir(schemes_dir)? {
                    let scheme_entry = scheme_entry?;
                    let scheme_path = scheme_entry.path();
                    if scheme_path.extension().map_or(false, |e| e == "xcscheme") {
                        if let Some(name) = scheme_path.file_stem() {
                            schemes.push(name.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }

    // If no shared schemes, try to guess from project name
    if schemes.is_empty() {
        for entry in std::fs::read_dir(project_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "xcodeproj") {
                if let Some(name) = path.file_stem() {
                    schemes.push(name.to_string_lossy().to_string());
                }
            }
        }
    }

    Ok(schemes)
}

// Shell expansion helper
mod shellexpand {
    pub fn tilde(path: &str) -> std::borrow::Cow<str> {
        if path.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                return std::borrow::Cow::Owned(format!("{}{}", home.display(), &path[1..]));
            }
        }
        std::borrow::Cow::Borrowed(path)
    }
}
