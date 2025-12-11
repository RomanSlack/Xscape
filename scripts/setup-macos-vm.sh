#!/bin/bash
# Setup macOS VM using OSX-KVM
# This script helps you create a macOS VM for running Xcode builds

set -e

echo "=== ios-sim-launcher: macOS VM Setup ==="
echo ""

# Default VM directory
VM_DIR="${VM_DIR:-$HOME/macOS-VMs}"

echo "VM directory: $VM_DIR"
echo ""

# Create directory
mkdir -p "$VM_DIR"
cd "$VM_DIR"

# Check if OSX-KVM already exists
if [ -d "OSX-KVM" ]; then
    echo "OSX-KVM already cloned. Updating..."
    cd OSX-KVM
    git pull
else
    echo "Cloning OSX-KVM..."
    git clone --depth 1 https://github.com/kholia/OSX-KVM.git
    cd OSX-KVM
fi

echo ""
echo "=== macOS Version Selection ==="
echo ""
echo "You can now download a macOS installer."
echo "Run the fetch script to see available versions:"
echo ""
echo "  cd $VM_DIR/OSX-KVM"
echo "  ./fetch-macOS-v2.py"
echo ""
echo "Recommended: macOS Sonoma (14.x) or Sequoia (15.x) for latest Xcode support"
echo ""

# Create disk image if it doesn't exist
DISK_IMAGE="$VM_DIR/macos.qcow2"
if [ ! -f "$DISK_IMAGE" ]; then
    echo "Creating VM disk image (128GB)..."
    qemu-img create -f qcow2 "$DISK_IMAGE" 128G
    echo "Created: $DISK_IMAGE"
else
    echo "Disk image already exists: $DISK_IMAGE"
fi

echo ""
echo "=== Next Steps ==="
echo ""
echo "1. Download macOS installer:"
echo "   cd $VM_DIR/OSX-KVM"
echo "   ./fetch-macOS-v2.py"
echo ""
echo "2. Convert the installer (follow OSX-KVM instructions):"
echo "   ./OpenCore-Boot.sh"
echo ""
echo "3. Install macOS in the VM"
echo "   - The installer will boot"
echo "   - Use Disk Utility to format the virtual disk"
echo "   - Install macOS"
echo ""
echo "4. After installation, inside macOS:"
echo "   - Install Xcode from App Store"
echo "   - Install Xcode Command Line Tools: xcode-select --install"
echo "   - Accept Xcode license: sudo xcodebuild -license accept"
echo "   - Install iOS Simulator runtimes in Xcode preferences"
echo ""
echo "5. Install the xcode-agent on macOS:"
echo "   - Copy the xcode-agent binary to the VM"
echo "   - Run: ./scripts/setup-agent.sh"
echo ""
echo "6. Configure ios-sim to use the VM:"
echo "   ios-sim config set vm.disk_image $DISK_IMAGE"
echo ""
echo "=== Important Notes ==="
echo ""
echo "LEGAL: Running macOS in a VM is only permitted on Apple hardware"
echo "according to Apple's EULA. Use at your own discretion."
echo ""
echo "VM Files location: $VM_DIR"
echo ""
