# Integration Guide

## With rust-vmm / Firecracker

The generated initramfs is compatible with rust-vmm based hypervisors.

### Basic setup

1. **Build the initramfs**

```bash
initramfs-builder build python:3.12-alpine \
  --inject ./agent:/usr/bin/agent \
  --init ./init.sh \
  --platform-arch amd64 \
  -o python.cpio.gz
```

2. **Get a compatible kernel**

For Firecracker, use their provided kernel or build a minimal one:

```bash
# Firecracker kernel (x86_64)
curl -L -o vmlinux \
  https://s3.amazonaws.com/spec.ccfc.min/firecracker-ci/v1.10/x86_64/vmlinux-6.1.102

# Alpine kernel (arm64)
curl -L -o vmlinux \
  https://dl-cdn.alpinelinux.org/alpine/v3.21/releases/aarch64/netboot/vmlinuz-lts
```

3. **Boot the VM**

```rust
// Simplified rust-vmm example
let vm = VmBuilder::new()
    .kernel_path("vmlinux")
    .initramfs_path("python.cpio.gz")
    .kernel_cmdline("console=ttyS0 rdinit=/init")
    .memory_mb(256)
    .build()?;

vm.start()?;
```

### Communication with the VM

Typically done via **vsock** (virtio socket):

```
┌─────────────────┐         vsock          ┌─────────────────┐
│   Hypervisor    │◄──────────────────────►│   VM (agent)    │
│                 │    CID:3, Port:5000    │                 │
└─────────────────┘                        └─────────────────┘
```

Example init script for vsock agent:

```bash
#!/bin/sh
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev

# Agent listens on vsock port 5000
exec /usr/bin/agent --vsock-port 5000
```

## Testing with QEMU

Quick test without setting up rust-vmm:

```bash
# Build for arm64 (Apple Silicon)
initramfs-builder build alpine:latest \
  --platform-arch arm64 \
  -o test.cpio.gz

# Download kernel
curl -L -o vmlinuz \
  https://dl-cdn.alpinelinux.org/alpine/v3.21/releases/aarch64/netboot/vmlinuz-lts

# Boot with QEMU
qemu-system-aarch64 \
  -M virt \
  -cpu cortex-a72 \
  -m 256M \
  -kernel vmlinuz \
  -initrd test.cpio.gz \
  -append "console=ttyAMA0 rdinit=/init" \
  -nographic
```

For x86_64:

```bash
initramfs-builder build alpine:latest \
  --platform-arch amd64 \
  -o test.cpio.gz

qemu-system-x86_64 \
  -m 256M \
  -kernel vmlinuz \
  -initrd test.cpio.gz \
  -append "console=ttyS0 rdinit=/init" \
  -nographic
```

## Building for different platforms

```bash
# x86_64 (Intel/AMD, AWS EC2)
initramfs-builder build python:alpine --platform-arch amd64

# arm64 (Apple Silicon, AWS Graviton)
initramfs-builder build python:alpine --platform-arch arm64
```

## Optimizing image size

```bash
# Exclude unnecessary files
initramfs-builder build python:3.12-alpine \
  --exclude "/usr/share/doc/*" \
  --exclude "/usr/share/man/*" \
  --exclude "/var/cache/*" \
  --exclude "*.pyc" \
  --exclude "__pycache__" \
  -o python-slim.cpio.gz
```

## Using as a library

```rust
use rusty_initramfs_builder::{InitramfsBuilder, Compression};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    InitramfsBuilder::new()
        .image("python:3.12-alpine")
        .inject("./agent", "/usr/bin/agent")
        .init_script("./init.sh")
        .exclude(&["/usr/share/doc/*", "*.pyc"])
        .compression(Compression::Gzip)
        .platform("linux", "amd64")
        .build("output.cpio.gz")
        .await?;
    
    Ok(())
}
```

## Troubleshooting

### VM doesn't boot

1. Check kernel and initramfs architectures match
2. Verify `/init` exists and is executable
3. Use `rdinit=/init` in kernel cmdline (not `init=/init`)

### "No init found"

The kernel can't find `/init`. Make sure:
- You used `--init` to provide an init script, OR
- The base image has `/init` or `/sbin/init`

### Agent can't communicate

Check vsock is enabled in hypervisor config and the agent is listening on the correct CID/port.
