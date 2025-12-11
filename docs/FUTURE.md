# Future Development Roadmap

This document outlines planned features beyond the MVP.

## Phase 2: Testing Support

### New Endpoints

```
POST /test
{
  "project_id": "uuid",
  "scheme": "MyAppTests",
  "destination": {...},
  "test_plan": "MyApp.xctestplan",  // optional
  "only_testing": ["MyAppTests/TestClass/testMethod"],  // optional
  "skip_testing": []
}

Response: { "test_run_id": "uuid", "status": "running" }

GET /test/{id}
Response: { "status": "completed", "passed": 42, "failed": 2, "skipped": 1 }

GET /test/{id}/results
Response: JUnit XML format for CI integration
```

### CLI Commands

```bash
ios-sim test -p ./MyApp --scheme MyAppTests
ios-sim test --only "MyAppTests/LoginTests/*"
ios-sim test --coverage  # Generate code coverage
```

## Phase 2: Screenshots and Video

### New Endpoints

```
GET /simulator/{udid}/screenshot
Response: PNG image data

POST /simulator/{udid}/record
{ "format": "mp4", "codec": "h264" }
Response: { "recording_id": "uuid" }

DELETE /simulator/{udid}/record
Response: { "recording_id": "uuid", "video_url": "/recordings/{id}.mp4" }

GET /recordings/{id}
Response: Video file download
```

### CLI Commands

```bash
ios-sim screenshot --device "iPhone 15 Pro" -o screenshot.png
ios-sim record start --device "iPhone 15 Pro"
ios-sim record stop -o recording.mp4
```

## Phase 3: Device Management

### New Endpoints

```
POST /simulator/create
{
  "name": "Test iPhone",
  "device_type": "com.apple.CoreSimulator.SimDeviceType.iPhone-15-Pro",
  "runtime": "com.apple.CoreSimulator.SimRuntime.iOS-17-0"
}

DELETE /simulator/{udid}
Response: { "deleted": true }

POST /runtime/install
{ "runtime": "iOS 18.0" }
Response: { "status": "downloading", "progress": 0 }

GET /runtime/install/{id}
Response: { "status": "installed" }
```

### CLI Commands

```bash
ios-sim devices create "CI iPhone" --type "iPhone 15" --runtime "iOS 17.0"
ios-sim devices delete {udid}
ios-sim runtime install "iOS 18.0"
ios-sim runtime list
```

## Phase 3: Watch Mode

Automatic rebuild on file changes.

```bash
ios-sim run ./MyApp --scheme MyApp --watch
```

Implementation:
- Use `notify` crate for file system watching
- Debounce rapid changes
- Incremental sync (only changed files)

## Phase 4: Incremental Sync

Rsync-like delta transfers for faster syncs.

Current: Full tarball every time
Future:
- Compute file checksums locally
- Send manifest to agent
- Only transfer changed files
- Use binary diff for large files

## Phase 4: Multiple Projects

Support concurrent builds from different projects.

- Project isolation in separate directories
- Build queue with priorities
- Resource limits per project

## Phase 5: Device Farm

Support multiple macOS hosts for parallel builds.

### Architecture

```
┌─────────────────┐
│   ios-sim CLI   │
└────────┬────────┘
         │
    ┌────┴────┐
    │  Load   │
    │Balancer │
    └────┬────┘
         │
    ┌────┴────┬────────┬────────┐
    │         │        │        │
┌───┴───┐ ┌───┴───┐ ┌──┴────┐ ┌─┴─────┐
│ Mac 1 │ │ Mac 2 │ │ Mac 3 │ │ VM 1  │
└───────┘ └───────┘ └───────┘ └───────┘
```

### Configuration

```toml
[farm]
enabled = true
hosts = [
  { name = "mac1", url = "http://192.168.1.100:8080" },
  { name = "mac2", url = "http://192.168.1.101:8080" },
  { name = "vm1", mode = "local-vm" },
]
strategy = "round-robin"  # or "least-loaded"
```

## Phase 5: Web Dashboard

Browser-based UI for monitoring.

Features:
- Real-time build status
- Log viewer
- Simulator screenshots
- Device management
- Build history

Technology:
- Axum with embedded SPA
- React or Svelte frontend
- WebSocket for real-time updates

## Quality of Life Improvements

### Short Term
- [ ] Build caching (reuse previous builds)
- [ ] Scheme auto-detection from project
- [ ] Better error messages with suggestions
- [ ] Progress bars for long operations
- [ ] Tab completion for zsh/bash

### Medium Term
- [ ] Project templates (`ios-sim init`)
- [ ] Custom build scripts (pre/post hooks)
- [ ] Notifications (desktop, Slack)
- [ ] Build artifact export

### Long Term
- [ ] Swift Package Manager support
- [ ] Carthage/CocoaPods integration
- [ ] Xcode project generation
- [ ] App Store upload support

## Contributing

See CONTRIBUTING.md for how to contribute to these features.

Priority is determined by:
1. User demand (GitHub issues)
2. Implementation complexity
3. Maintenance burden
