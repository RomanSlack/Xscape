#!/bin/bash
# Setup QEMU/KVM and dependencies on Ubuntu 24.04
# This script prepares your Ubuntu system to run a macOS VM

set -e

echo "=== ios-sim-launcher: Ubuntu Setup ==="
echo ""

# Check if running as root (we need sudo for some commands)
if [ "$EUID" -eq 0 ]; then
    echo "Please run this script as a regular user (not root)."
    echo "The script will use sudo when needed."
    exit 1
fi

# Check Ubuntu version
if [ -f /etc/os-release ]; then
    . /etc/os-release
    echo "Detected: $PRETTY_NAME"
else
    echo "Warning: Could not detect OS version"
fi

echo ""
echo "Installing required packages..."

# Update package list
sudo apt-get update

# Install QEMU and KVM
sudo apt-get install -y \
    qemu-system-x86 \
    qemu-utils \
    libvirt-daemon-system \
    libvirt-clients \
    bridge-utils \
    ovmf \
    virt-manager

# Install websockify for noVNC
sudo apt-get install -y python3-websockify

# Install git (needed for cloning noVNC and OSX-KVM)
sudo apt-get install -y git

echo ""
echo "Enabling KVM modules..."

# Load KVM modules
sudo modprobe kvm

# Detect CPU vendor and load appropriate module
if grep -q Intel /proc/cpuinfo; then
    echo "Intel CPU detected, loading kvm_intel..."
    sudo modprobe kvm_intel
elif grep -q AMD /proc/cpuinfo; then
    echo "AMD CPU detected, loading kvm_amd..."
    sudo modprobe kvm_amd
fi

echo ""
echo "Adding user to required groups..."

# Add user to KVM and libvirt groups
sudo usermod -aG kvm "$USER"
sudo usermod -aG libvirt "$USER"

echo ""
echo "Installing noVNC..."

# Clone noVNC if not present
if [ ! -d /opt/novnc ]; then
    sudo git clone https://github.com/novnc/noVNC.git /opt/novnc
    sudo git clone https://github.com/novnc/websockify.git /opt/novnc/utils/websockify
    sudo chown -R "$USER:$USER" /opt/novnc
else
    echo "noVNC already installed at /opt/novnc"
fi

echo ""
echo "Checking KVM support..."

# Check KVM support
if [ -e /dev/kvm ]; then
    echo "✓ KVM is available"
    if [ -r /dev/kvm ] && [ -w /dev/kvm ]; then
        echo "✓ KVM is accessible"
    else
        echo "⚠ KVM exists but may not be accessible yet (logout and login required)"
    fi
else
    echo "✗ KVM is not available"
    echo "  Make sure virtualization is enabled in BIOS/UEFI"
fi

echo ""
echo "=== Setup Complete ==="
echo ""
echo "IMPORTANT: Please log out and log back in for group changes to take effect."
echo ""
echo "Next steps:"
echo "1. Log out and log back in"
echo "2. Run ./scripts/setup-macos-vm.sh to set up the macOS VM"
echo "3. Configure ios-sim with: ios-sim config init"
echo ""
