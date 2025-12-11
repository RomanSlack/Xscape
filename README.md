# ios-sim-launcher

Build and run iOS apps from Linux using a macOS VM or remote Mac.

> *"I hate MacOS so much I am going to automate the testing and emulation of iOS apps for Linux."*

## Overview

`ios-sim-launcher` lets you develop iOS apps on Ubuntu without needing a Mac as your primary development machine. It orchestrates builds via `xcodebuild` and runs them in the iOS Simulator via `simctl`, all from the comfort of your Linux terminal.

```
Ubuntu                          macOS (VM or Remote)
┌─────────────┐                ┌──────────────────────┐
│  ios-sim    │───HTTP/WS────▶│    xcode-agent       │
│    CLI      │                │  ┌────────────────┐  │
└─────────────┘                │  │  xcodebuild    │  │
      │                        │  │  simctl        │  │
      │                        │  │  iOS Simulator │  │
      └────VNC────────────────▶│  └────────────────┘  │
                               └──────────────────────┘
```

## Features

- **Interactive TUI** - Beautiful terminal interface for project selection, simulator management, and more
- **Build iOS apps** from Linux using Xcode on a macOS VM or remote Mac
- **Run in iOS Simulator** and view via VNC in your browser
- **Stream build logs** in real-time via WebSocket
- **Manage local macOS VM** with QEMU/KVM
- **Setup Wizard** - Verify and configure your installation with guided prompts
- **Support both modes**: local VM or remote Mac over network

## Quick Start

### 1. Install dependencies (Ubuntu)

```bash
./scripts/setup-ubuntu.sh
# Log out and back in
```

### 2. Set up macOS VM (or configure remote Mac)

```bash
./scripts/setup-macos-vm.sh
# Follow prompts to install macOS
```

### 3. Install the agent on macOS

Inside the macOS VM:
```bash
./scripts/setup-agent.sh
```

### 4. Build and run

```bash
# Initialize config
ios-sim config init
ios-sim config set vm.disk_image ~/macOS-VMs/macos.qcow2

# Start VM
ios-sim vm start

# Build and run your app
ios-sim run ./MyApp.xcodeproj --scheme MyApp
```

## CLI Commands

```
ios-sim interactive   Launch interactive TUI mode
ios-sim status        Quick status check
ios-sim setup         Run setup wizard

ios-sim build         Build an iOS project
ios-sim run           Build and run in simulator
ios-sim vm            Manage local macOS VM
  start               Start the VM
  stop                Stop the VM
  status              Show VM status
  vnc                 Open simulator in browser
ios-sim devices       List available simulators
ios-sim logs          Stream build/app logs
ios-sim config        Manage configuration
  init                Create config file
  show                Show current config
  set                 Set a config value
```

## Interactive Mode

Launch the interactive TUI for a guided experience:

```bash
ios-sim interactive
```

```
  ios-sim
  iOS Development from Linux

? Select action
> Run Project         Build and run in simulator
  Build Project       Build without running
  Manage Simulators   List, boot, shutdown devices
  VM Control          Start, stop, VNC access
  Settings            Configure ios-sim
  Setup Wizard        Verify installation
  Exit
```

The interactive mode provides:
- **Project Browser** - Navigate and select your Xcode projects
- **Scheme Selection** - Fuzzy search through available schemes
- **Simulator Picker** - Choose devices with runtime info and status
- **Progress Indicators** - Spinners and progress bars for all operations
- **Colorful Output** - Status indicators and formatted logs

## Quick Status

Check your setup at a glance:

```bash
ios-sim status
```

```
   +  Status:      Connected
   +  Xcode:       16.0
   +  Simulators:  12
```

## Setup Wizard

Verify and fix your installation:

```bash
ios-sim setup
```

The wizard will:
1. Check your configuration file
2. Test agent connectivity
3. Verify Xcode installation
4. List available simulators
5. Offer to start the VM if needed

## Configuration

Configuration file: `~/.config/ios-sim/config.toml`

```toml
[agent]
mode = "local-vm"  # or "remote"
remote_host = "192.168.1.100"
remote_port = 8080

[vm]
disk_image = "~/macOS-VMs/macos.qcow2"
memory = "8G"
cpus = 4

[simulator]
preferred_device = "iPhone 15 Pro"
```

See [config/example-config.toml](config/example-config.toml) for all options.

## Architecture

The system consists of three components:

1. **ios-sim CLI** (Ubuntu) - User-facing command-line tool
2. **xcode-agent** (macOS) - HTTP server wrapping xcodebuild and simctl
3. **ios-sim-common** - Shared types and protocols

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for details.

## Requirements

### Ubuntu Host
- Ubuntu 24.04 (or similar)
- CPU with virtualization support (Intel VT-x / AMD-V)
- 16GB+ RAM
- 150GB+ free disk space
- KVM enabled

### macOS VM/Remote
- macOS Sonoma or Sequoia
- Xcode 15+
- iOS Simulator runtimes

## Building from Source

```bash
# Build everything
cargo build --release

# Binaries will be in:
# target/release/ios-sim      (for Ubuntu)
# target/release/xcode-agent  (for macOS)
```

Cross-compile the agent for macOS:
```bash
# On a Mac or using cross-compilation
cargo build --release -p xcode-agent --target x86_64-apple-darwin
```

## Documentation

- [Architecture](docs/ARCHITECTURE.md) - System design and data flow
- [VM Setup](docs/VM-SETUP.md) - Detailed VM setup guide
- [Future Roadmap](docs/FUTURE.md) - Planned features

## Legal Note

Running macOS in a VM is only permitted on Apple-branded hardware according to Apple's EULA. When using the local VM mode, ensure you're in compliance with applicable licensing terms.

The "remote Mac" mode is fully compliant as it uses a physical Mac.

## License

MIT License - See [LICENSE](LICENSE) for details.

## Contributing

Contributions welcome! Please see the issues for planned features or open a new one to discuss your idea.
