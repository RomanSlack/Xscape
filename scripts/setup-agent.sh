#!/bin/bash
# Setup xcode-agent on macOS
# Run this script INSIDE the macOS VM or on a remote Mac

set -e

echo "=== xcode-agent Setup ==="
echo ""

# Configuration
AGENT_DIR="${AGENT_DIR:-/opt/xcode-agent}"
AGENT_PORT="${AGENT_PORT:-8080}"

# Check if we're on macOS
if [ "$(uname)" != "Darwin" ]; then
    echo "Error: This script must be run on macOS"
    exit 1
fi

# Check for Xcode
if ! command -v xcodebuild &> /dev/null; then
    echo "Error: Xcode is not installed"
    echo "Please install Xcode from the App Store first."
    exit 1
fi

# Check Xcode license
if ! xcodebuild -checkFirstLaunchStatus &> /dev/null; then
    echo "Xcode first launch tasks not completed."
    echo "Running: sudo xcodebuild -runFirstLaunch"
    sudo xcodebuild -runFirstLaunch
fi

echo "Creating directories..."
sudo mkdir -p "$AGENT_DIR"
sudo mkdir -p "$AGENT_DIR/projects"
sudo mkdir -p "$AGENT_DIR/logs"
sudo chown -R "$(whoami)" "$AGENT_DIR"

# Check if agent binary exists in current directory
if [ -f "./xcode-agent" ]; then
    echo "Installing xcode-agent binary..."
    cp ./xcode-agent "$AGENT_DIR/"
    chmod +x "$AGENT_DIR/xcode-agent"
else
    echo "Note: xcode-agent binary not found in current directory."
    echo "You'll need to build it with: cargo build --release -p xcode-agent"
    echo "Then copy target/release/xcode-agent to this directory."
fi

# Create config file
echo "Creating configuration..."
cat > "$AGENT_DIR/config.toml" << EOF
[server]
host = "0.0.0.0"
port = $AGENT_PORT

[storage]
projects_dir = "$AGENT_DIR/projects"
logs_dir = "$AGENT_DIR/logs"
max_projects = 10
cleanup_after_hours = 24

[xcode]
# Path is auto-detected

[simulator]
auto_boot = true
shutdown_idle_after_minutes = 30
EOF

echo "Created: $AGENT_DIR/config.toml"

# Create LaunchAgent for auto-start (runs as user, more reliable than LaunchDaemon)
LAUNCH_AGENTS_DIR="$HOME/Library/LaunchAgents"
PLIST_PATH="$LAUNCH_AGENTS_DIR/com.iossim.agent.plist"

echo "Creating LaunchAgent for auto-start..."
mkdir -p "$LAUNCH_AGENTS_DIR"

cat > "$PLIST_PATH" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.iossim.agent</string>
    <key>ProgramArguments</key>
    <array>
        <string>$AGENT_DIR/xcode-agent</string>
        <string>--config</string>
        <string>$AGENT_DIR/config.toml</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>$AGENT_DIR/logs/stdout.log</string>
    <key>StandardErrorPath</key>
    <string>$AGENT_DIR/logs/stderr.log</string>
    <key>WorkingDirectory</key>
    <string>$AGENT_DIR</string>
</dict>
</plist>
EOF

echo "Created: $PLIST_PATH"

# Remove old LaunchDaemon if it exists (from previous installs)
OLD_DAEMON="/Library/LaunchDaemons/com.iossim.agent.plist"
if [ -f "$OLD_DAEMON" ]; then
    echo "Removing old LaunchDaemon..."
    sudo launchctl unload "$OLD_DAEMON" 2>/dev/null || true
    sudo rm -f "$OLD_DAEMON"
fi

# Load the service if binary exists
if [ -f "$AGENT_DIR/xcode-agent" ]; then
    echo "Loading LaunchAgent..."
    # Unload first in case it's already loaded
    launchctl unload "$PLIST_PATH" 2>/dev/null || true
    launchctl load "$PLIST_PATH"

    # Wait a moment for it to start
    sleep 2

    # Verify it's running
    if curl -s "http://localhost:$AGENT_PORT/health" > /dev/null 2>&1; then
        echo ""
        echo "=== Agent Started Successfully ==="
        echo "The agent is running on port $AGENT_PORT"
        echo ""
        echo "To check status: curl http://localhost:$AGENT_PORT/health"
        echo "To view logs: tail -f $AGENT_DIR/logs/stdout.log"
        echo ""
        echo "The agent will auto-start on login."
    else
        echo ""
        echo "=== Agent Installed (checking startup...) ==="
        echo "The LaunchAgent is installed but the agent may still be starting."
        echo ""
        echo "Check status in a few seconds: curl http://localhost:$AGENT_PORT/health"
        echo "View logs: tail -f $AGENT_DIR/logs/stderr.log"
    fi
else
    echo ""
    echo "=== Setup Complete (Agent Binary Missing) ==="
    echo ""
    echo "To finish setup:"
    echo "1. Build the agent: cargo build --release -p xcode-agent"
    echo "2. Copy the binary: scp target/release/xcode-agent mac:$AGENT_DIR/"
    echo "3. Load the agent: launchctl load $PLIST_PATH"
fi

echo ""
echo "Configuration file: $AGENT_DIR/config.toml"
echo "Log files: $AGENT_DIR/logs/"
echo ""
