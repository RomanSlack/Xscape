# macOS VM Setup Guide

This guide walks you through setting up a macOS VM on Ubuntu for use with ios-sim-launcher.

## Prerequisites

- Ubuntu 24.04 (or similar Linux distribution)
- CPU with virtualization support (Intel VT-x or AMD-V)
- At least 16GB RAM (8GB for VM + 8GB for host)
- At least 150GB free disk space
- KVM enabled

## Step 1: Prepare Ubuntu Host

Run the setup script:

```bash
./scripts/setup-ubuntu.sh
```

This installs:
- QEMU/KVM
- libvirt
- OVMF (UEFI firmware)
- noVNC
- websockify

**Important**: Log out and log back in after running the script.

Verify KVM is working:

```bash
kvm-ok
# Should output: KVM acceleration can be used
```

## Step 2: Download macOS

Run the VM setup script:

```bash
./scripts/setup-macos-vm.sh
```

This clones OSX-KVM and creates a disk image.

Then download macOS:

```bash
cd ~/macOS-VMs/OSX-KVM
./fetch-macOS-v2.py
```

Select your preferred version (Sonoma or Sequoia recommended).

## Step 3: Install macOS

Start the installer:

```bash
cd ~/macOS-VMs/OSX-KVM
./OpenCore-Boot.sh
```

In the VM:
1. Wait for OpenCore boot menu
2. Select "Install macOS"
3. Open Disk Utility
4. Erase the virtual disk (APFS format)
5. Close Disk Utility
6. Select "Install macOS"
7. Wait for installation (takes 30-60 minutes)

The VM will reboot several times during installation.

## Step 4: Configure macOS

After installation completes:

### Initial Setup
1. Create a local user account
2. Skip Apple ID sign-in (or sign in if you want App Store)
3. Complete setup wizard

### Install Xcode
Option A: App Store
1. Open App Store
2. Sign in with Apple ID
3. Search for "Xcode"
4. Download and install (this takes a while)

Option B: Direct Download
1. Go to developer.apple.com/download
2. Sign in with Apple ID
3. Download Xcode .xip file
4. Extract and move to Applications

### Configure Xcode
```bash
# Install command line tools
xcode-select --install

# Accept license
sudo xcodebuild -license accept

# Run first launch tasks
sudo xcodebuild -runFirstLaunch
```

### Install Simulator Runtimes
1. Open Xcode
2. Go to Settings > Platforms
3. Download iOS Simulator runtimes you need

Or via command line:
```bash
xcodebuild -downloadPlatform iOS
```

## Step 5: Install xcode-agent

Build the agent on your Ubuntu host:

```bash
cargo build --release -p xcode-agent
```

Copy to the VM (via shared folder or scp):

```bash
# If using SSH (port 2222)
scp target/release/xcode-agent localhost:2222:~/

# In the VM
cd ~
chmod +x xcode-agent
./scripts/setup-agent.sh
```

Verify the agent is running:

```bash
curl http://localhost:8080/health
```

## Step 6: Configure ios-sim

On Ubuntu:

```bash
# Initialize config
ios-sim config init

# Set the disk image path
ios-sim config set vm.disk_image ~/macOS-VMs/macos.qcow2

# Test the connection
ios-sim vm start
ios-sim devices
```

## VM Management

### Starting the VM

```bash
ios-sim vm start           # Start with GUI
ios-sim vm start --headless  # Start without GUI
```

### Accessing the VM

```bash
ios-sim vm vnc             # Open simulator in browser
ssh -p 2222 localhost      # SSH to VM
```

### Stopping the VM

```bash
ios-sim vm stop
```

### Checking Status

```bash
ios-sim vm status
```

## Performance Tuning

### VM Resources

Edit `~/.config/ios-sim/config.toml`:

```toml
[vm]
memory = "12G"  # More RAM for faster builds
cpus = 6        # More cores for parallel compilation
```

### Disk Performance

Use SSD storage for the qcow2 image. If you have NVMe:

```bash
# Create image on fast storage
qemu-img create -f qcow2 /nvme/macos.qcow2 128G
ios-sim config set vm.disk_image /nvme/macos.qcow2
```

### CPU Pass-through

For better performance, enable all CPU features:

Edit the QEMU command (advanced):
```
-cpu host,kvm=on,vendor=GenuineIntel
```

## Troubleshooting

### VM won't boot

1. Check KVM is enabled: `ls -la /dev/kvm`
2. Check OVMF path exists: `ls /usr/share/OVMF/`
3. Try with verbose output: modify start command to show QEMU output

### Agent not reachable

1. Check VM is booted: `ios-sim vm status`
2. Check agent is running in VM: `ps aux | grep xcode-agent`
3. Check firewall isn't blocking: `sudo ufw status`
4. Check port forwarding: `ss -tlnp | grep 8080`

### Slow builds

1. Increase VM memory and CPUs
2. Use SSD storage
3. Disable antivirus scanning of VM files
4. Consider using a remote Mac instead

### Xcode issues

1. Ensure license is accepted: `sudo xcodebuild -license accept`
2. Reset Xcode: `rm -rf ~/Library/Developer/Xcode/DerivedData/*`
3. Re-run first launch: `sudo xcodebuild -runFirstLaunch`

## Snapshots

Create a snapshot after setup for quick recovery:

```bash
# Using qemu-img
qemu-img snapshot -c "clean-setup" ~/macOS-VMs/macos.qcow2

# Restore if needed
qemu-img snapshot -a "clean-setup" ~/macOS-VMs/macos.qcow2
```

## Remote Mac Alternative

If you have access to a Mac:

1. Install xcode-agent on the Mac
2. Configure ios-sim to use remote mode:

```bash
ios-sim config set agent.mode remote
ios-sim config set agent.remote_host 192.168.1.100
ios-sim config set agent.remote_port 8080
```

This is the recommended approach for:
- Better performance
- Legal compliance
- Access to real device testing
