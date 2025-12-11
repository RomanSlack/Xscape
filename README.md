# Xscape

**Escape from Xcode.** Build and run iOS apps entirely from Linux.

> *Xscape* — A play on "escape" and "Xcode." Because you shouldn't need a Mac on your desk just to build an iOS app.

---

## What is Xscape?

Xscape lets you develop iOS apps on Linux by orchestrating a macOS VM (or remote Mac) that handles the actual Xcode builds and simulator runs. You get a clean terminal UI on Linux, and the iOS Simulator streams to your browser via VNC.

**No Mac on your desk required.**

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Linux Host                              │
│  ┌─────────────┐                                                │
│  │   xscape    │  <── You are here                              │
│  │    (CLI)    │                                                │
│  └──────┬──────┘                                                │
│         │ HTTP/WebSocket                                        │
│         ▼                                                       │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              QEMU/KVM macOS VM                           │   │
│  │  ┌─────────────────┐    ┌─────────────────────────────┐ │   │
│  │  │  xscape-agent   │───>│     iOS Simulator           │ │   │
│  │  │  (HTTP Server)  │    │  (via simctl + xcodebuild)  │ │   │
│  │  └─────────────────┘    └─────────────────────────────┘ │   │
│  └─────────────────────────────────────────────────────────┘   │
│         │ VNC                                                   │
│         ▼                                                       │
│  ┌─────────────┐                                                │
│  │   noVNC     │  <── View simulator in browser                 │
│  │  (Browser)  │                                                │
│  └─────────────┘                                                │
└─────────────────────────────────────────────────────────────────┘
```

## Features

- **Interactive TUI** — Clean terminal interface for project selection, simulator management, and builds
- **Build iOS apps** from Linux using Xcode on a macOS VM or remote Mac
- **Run in iOS Simulator** and view via VNC in your browser
- **Stream build logs** in real-time via WebSocket
- **Manage local macOS VM** with QEMU/KVM
- **Setup Wizard** — Verify and configure your installation with guided prompts
- **Two modes**: local VM or remote Mac over network

## Quick Start

### 1. Install Dependencies (Ubuntu/Debian)

```bash
sudo apt install qemu-system-x86 qemu-utils ovmf
```

### 2. Set Up macOS VM

Follow [OSX-KVM](https://github.com/kholia/OSX-KVM) to create a macOS VM with:
- Xcode installed
- `xscape-agent` binary running

### 3. Install Xscape

```bash
cargo install --path crates/xscape
```

### 4. Configure

```bash
xscape config init
# Edit ~/.config/xscape/config.toml with your VM disk image path
```

### 5. Run

```bash
xscape interactive
```

## CLI Commands

```
xscape interactive    Launch interactive TUI mode
xscape status         Quick status check
xscape setup          Run setup wizard

xscape build          Build an iOS project
xscape run            Build and run in simulator
xscape vm             Manage local macOS VM
  start               Start the VM
  stop                Stop the VM
  status              Show VM status
  vnc                 Open simulator in browser
xscape devices        List available simulators
xscape logs           Stream build/app logs
xscape config         Manage configuration
  init                Create config file
  show                Show current config
  set                 Set a config value
```

## Interactive Mode

Launch the interactive TUI for a guided experience:

```bash
xscape interactive
```

```
  xscape
  ──────────────────────────────────────

  agent: connected  |  xcode: 15.4  |  simulators: 12

? Select
> Run Project
  Build Project
  Simulators
  VM Control
  Settings
  Setup Wizard
  Exit
```

Features:
- **Step-by-step flows** — Clear progression through project, scheme, and device selection
- **Breadcrumb navigation** — Always know where you are
- **Back options everywhere** — Easy to navigate out at any point
- **iOS version picker** — Select runtime then device
- **Progress indicators** — Spinners and status for all operations

## Quick Status

```bash
xscape status
```

```
   +  Status:      Connected
   +  Xcode:       15.4
   +  Simulators:  12
```

## Setup Wizard

```bash
xscape setup
```

The wizard will:
1. Check your configuration file
2. Test agent connectivity
3. Verify Xcode installation
4. List available simulators
5. Offer to start the VM if needed

## Configuration

Config file: `~/.config/xscape/config.toml`

```toml
[agent]
mode = "local-vm"  # or "remote"
remote_host = "192.168.1.100"
remote_port = 8080
timeout_secs = 30

[vm]
disk_image = "/path/to/macos.qcow2"
memory = "8G"
cpus = 4
vnc_port = 5900
agent_port = 8080

[simulator]
preferred_device = "iPhone 15 Pro"
```

## Project Structure

```
xscape/
├── crates/
│   ├── xscape/           # Linux CLI binary
│   ├── xscape-agent/     # macOS agent (runs in VM)
│   └── xscape-common/    # Shared types
├── scripts/
│   ├── setup-ubuntu.sh   # Linux setup script
│   └── setup-agent.sh    # macOS agent setup
└── docs/
    └── VM-SETUP.md       # VM creation guide
```

## Requirements

**Linux Host:**
- Ubuntu 22.04+ or similar
- QEMU/KVM with macOS support
- 16GB+ RAM recommended
- Rust 1.75+

**macOS VM:**
- macOS 13+ (Ventura or later)
- Xcode 15+
- xscape-agent running

## Building from Source

```bash
cargo build --release

# Binaries:
# target/release/xscape        (Linux CLI)
# target/release/xscape-agent  (macOS agent)
```

## Legal Note

Running macOS in a VM is only permitted on Apple-branded hardware according to Apple's EULA. The "remote Mac" mode is fully compliant as it connects to a physical Mac.

## License

MIT

---

*Xscape — because iOS development shouldn't require a $2000 Mac on your desk.*
