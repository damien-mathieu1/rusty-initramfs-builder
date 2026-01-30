# Architecture

## What is an initramfs?

An **initramfs** (initial RAM filesystem) is a compressed filesystem image loaded by the Linux kernel at boot time. It runs entirely in RAM and contains everything needed to start the system.

```
Boot sequence:
┌──────────────────────────────────────────────────────┐
│  1. Bootloader loads kernel + initramfs into memory  │
│  2. Kernel decompresses initramfs to RAM             │
│  3. Kernel executes /init as PID 1                   │
│  4. /init sets up the environment and runs services  │
└──────────────────────────────────────────────────────┘
```

## Why this tool?

For serverless/lambda workloads on microVMs (rust-vmm, Firecracker), we need:
- Fast boot times (milliseconds)
- Minimal footprint
- Language toolchains (Python, Rust, Node...)
- A custom agent to receive and execute code

This tool takes a Docker image and produces a ready-to-boot initramfs.

## Pipeline

```
┌─────────────────────────────────────────────────────────────┐
│                   initramfs-builder                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────┐    ┌───────────┐    ┌──────────┐    ┌───────┐  │
│  │  PULL   │───►│  EXTRACT  │───►│  INJECT  │───►│ PACK  │  │
│  └─────────┘    └───────────┘    └──────────┘    └───────┘  │
│       │              │                │              │      │
│       ▼              ▼                ▼              ▼      │
│   Download       Decompress       Add custom      Create    │
│   OCI layers     tar.gz layers    binaries +      CPIO      │
│   from           Handle           init script     archive   │
│   registry       whiteouts                        + gzip    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Module structure

```
src/
├── main.rs              # CLI entry point (clap)
├── lib.rs               # Public API: InitramfsBuilder
├── error.rs             # Error types
├── registry/
│   ├── mod.rs
│   └── client.rs        # OCI registry client (pulls without Docker)
├── image/
│   ├── mod.rs
│   ├── layer.rs         # Layer extraction, whiteout handling
│   └── rootfs.rs        # Rootfs assembly
└── initramfs/
    ├── mod.rs
    ├── cpio.rs          # CPIO newc format generation
    └── compress.rs      # gzip/zstd compression
```

## Key components

### Registry Client

Uses `oci-distribution` crate to pull images directly from registries (Docker Hub, ghcr.io, etc.) without requiring Docker to be installed.

Handles:
- Anonymous and authenticated pulls
- Multi-arch images (selects correct platform)
- Layer downloading

### Layer Extractor

Processes OCI image layers (tar.gz archives) and handles:
- Sequential extraction (layers must be applied in order)
- Whiteout files (`.wh.<name>` marks deleted files)
- Opaque whiteouts (`.wh..wh..opq` replaces entire directory)
- Hard links and symlinks

### CPIO Generator

Creates archives in **newc** (SVR4) format, which is the standard format expected by the Linux kernel for initramfs.

Format: `070701` magic + ASCII hex headers + file data + padding

### Compression

Supports:
- **gzip** - Default, universal compatibility
- **zstd** - Better ratio, faster decompression
- **none** - Uncompressed

## File injection

The `--inject` option copies files into the rootfs before packing:

```bash
--inject /host/path/binary:/initramfs/path/binary
```

Files are automatically made executable (mode 0755).

## Init script

The `--init` option replaces `/init` in the initramfs. This script runs as PID 1 when the kernel boots.

Minimal init script requirements:
1. Mount pseudo-filesystems (proc, sys, dev)
2. Start the main service/agent

```bash
#!/bin/sh
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev
exec /usr/bin/my-agent
```

## Output format

The output is a gzip-compressed CPIO archive:

```
initramfs.cpio.gz
    │
    └── CPIO newc archive
            │
            ├── /init          (custom init script)
            ├── /bin/          (busybox, etc.)
            ├── /usr/bin/      (python, node, etc.)
            ├── /usr/bin/agent (injected binary)
            ├── /lib/          (shared libraries)
            └── ...
```

## Typical sizes

| Image | Compressed size |
|-------|-----------------|
| alpine:latest | ~4 MB |
| python:3.12-alpine | ~17 MB |
| node:alpine | ~40 MB |
| rust:alpine | ~250 MB |
