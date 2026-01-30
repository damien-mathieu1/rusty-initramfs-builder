# rusty-initramfs-builder

A Rust CLI tool to convert Docker/OCI images into bootable initramfs for microVMs.

## Installation

```bash
cargo install initramfs-builder
```

## Quick Start

```bash
# Build an initramfs from a Docker image
initramfs-builder build python:3.12-alpine -o python.cpio.gz

# Inject a custom binary and init script
initramfs-builder build python:3.12-alpine \
  --inject ./my-agent:/usr/bin/my-agent \
  --init ./init.sh \
  -o python-lambda.cpio.gz
```

## Usage

```bash
# Build initramfs
initramfs-builder build <IMAGE> [OPTIONS]

Options:
  -o, --output <FILE>       Output file [default: initramfs.cpio.gz]
  --inject <SRC:DEST>       Inject file into initramfs (can be repeated)
  --init <SCRIPT>           Custom init script (placed at /init)
  --exclude <PATTERN>       Exclude files matching pattern
  --platform-arch <ARCH>    Target architecture [default: amd64]
  -c, --compression <FMT>   gzip, zstd, or none [default: gzip]

# Inspect image
initramfs-builder inspect <IMAGE>

# List layers
initramfs-builder list-layers <IMAGE>
```

## Example init script

```bash
#!/bin/sh
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev
exec /usr/bin/my-agent
```

## Documentation

See [docs/](docs/) for detailed documentation:

- [Architecture](docs/architecture.md) - How it works internally
- [Integration](docs/integration.md) - Using with rust-vmm/Firecracker

## License

MIT
