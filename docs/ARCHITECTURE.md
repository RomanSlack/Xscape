# ios-sim-launcher Architecture

## Overview

ios-sim-launcher enables iOS app development from Linux by orchestrating builds and simulator runs on macOS (either a VM or remote Mac).

```
┌─────────────────────────────────────────────────────────────────┐
│                        Ubuntu 24.04                              │
│                                                                  │
│  ┌──────────────┐     ┌──────────────────────────────────────┐ │
│  │   ios-sim    │     │         macOS (VM or Remote)          │ │
│  │     CLI      │────▶│                                        │ │
│  └──────────────┘     │  ┌────────────────────────────────┐   │ │
│         │             │  │        xcode-agent              │   │ │
│         │             │  │                                  │   │ │
│    ┌────┴────┐        │  │  ┌──────────┐  ┌────────────┐  │   │ │
│    │  QEMU   │        │  │  │xcodebuild│  │   simctl   │  │   │ │
│    │   VM    │◀──VNC──│  │  └──────────┘  └────────────┘  │   │ │
│    └─────────┘        │  │                                  │   │ │
│                       │  │         iOS Simulator            │   │ │
│                       │  └────────────────────────────────┘   │ │
│                       └──────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Components

### 1. ios-sim CLI (Ubuntu)

The main user interface. Written in Rust.

**Responsibilities:**
- Parse user commands (build, run, vm, devices, logs, config)
- Create project tarballs for upload
- Communicate with xcode-agent via HTTP
- Manage local QEMU VM lifecycle
- Stream logs via WebSocket
- Provide VNC access to simulator GUI

**Key modules:**
- `cli/` - Clap-based command parsing
- `agent_client/` - HTTP client for agent API
- `project/` - Tarball creation with .gitignore support
- `vm/` - QEMU and noVNC management

### 2. xcode-agent (macOS)

HTTP server running on macOS. Written in Rust.

**Responsibilities:**
- Receive and extract project uploads
- Run xcodebuild for iOS Simulator
- Control simulators via xcrun simctl
- Stream build and app logs
- Manage build artifacts

**Key modules:**
- `server/` - Axum HTTP server
- `handlers/` - API endpoint implementations
- `xcode/` - xcodebuild wrapper
- `simctl/` - simctl wrapper
- `storage/` - Project and artifact storage

### 3. ios-sim-common (Shared)

Shared library for types and protocols.

**Contents:**
- API request/response types
- Error types
- Configuration structures

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check, Xcode status |
| `/sync-project` | POST | Upload project tarball |
| `/build` | POST | Start async build |
| `/build/{id}` | GET | Get build status |
| `/simulator/list` | GET | List devices and runtimes |
| `/simulator/boot` | POST | Boot a simulator |
| `/simulator/run` | POST | Install and launch app |
| `/simulator/shutdown` | POST | Shutdown simulator |
| `/logs/{build_id}` | WS | Stream build/app logs |

## Data Flow

### Build + Run Flow

```
1. User runs: ios-sim run ./MyApp --scheme MyApp

2. CLI creates tarball:
   - Walks project directory
   - Respects .gitignore
   - Applies exclude patterns
   - Computes SHA256 checksum

3. CLI uploads to agent:
   POST /sync-project (multipart: project_name, checksum, tarball)

4. Agent extracts project:
   - Checks cache by checksum
   - Extracts to /var/xcode-agent/projects/{uuid}/

5. CLI starts build:
   POST /build { project_id, scheme, destination }

6. Agent runs xcodebuild:
   - Spawns xcodebuild process
   - Captures stdout/stderr
   - Streams via WebSocket
   - Finds .app in DerivedData

7. CLI polls for completion:
   GET /build/{id} until status = succeeded

8. CLI requests app launch:
   POST /simulator/run { build_id, device_udid }

9. Agent installs and launches:
   - xcrun simctl boot {udid}
   - xcrun simctl install {udid} {app_path}
   - xcrun simctl launch {udid} {bundle_id}

10. User views via VNC:
    - QEMU exposes VNC on port 5900
    - noVNC provides browser access
```

## VM Architecture

When using local-vm mode:

```
QEMU/KVM with macOS
├── CPU: Penryn (macOS compatible)
├── Memory: 8GB (configurable)
├── Disk: qcow2 image
├── Network: User-mode with port forwarding
│   ├── 2222 -> 22 (SSH)
│   ├── 8080 -> 8080 (agent)
│   └── 5900 -> 5900 (VNC)
└── Display: VNC
```

## Security Considerations

1. **Network**: Agent runs on 0.0.0.0 inside VM, only exposed via port forwarding
2. **Auth**: No authentication in MVP (designed for local development)
3. **Paths**: Tarball extraction prevents path traversal attacks
4. **Secrets**: Don't sync .env files (excluded by default)

## Configuration

CLI config: `~/.config/ios-sim/config.toml`
Agent config: `/opt/xcode-agent/config.toml`

See `config/example-config.toml` for all options.

## Error Handling

Errors flow as:
1. Agent returns `ApiError` JSON with code and message
2. CLI maps to user-friendly messages
3. Build failures include xcodebuild output

Common error codes:
- `XCODE_NOT_FOUND` - Xcode not installed
- `BUILD_FAILED` - xcodebuild returned non-zero
- `SIMULATOR_NOT_FOUND` - Device not available
- `PROJECT_NOT_FOUND` - Invalid project ID
