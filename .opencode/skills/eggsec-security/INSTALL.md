# Eggsec Installation Guide

## Prerequisites

- Rust 1.88 or later (MSRV: 1.88)
- Cargo package manager
- Linux, macOS, or Windows

## Quick Install

```bash
# Clone the repository
git clone https://github.com/eggstack/eggsec.git
cd eggsec

# Build with default features
cargo build --release -p eggsec-cli

# The binary will be at:
# target/release/eggsec
```

## Feature-Specific Builds

### Default Build (Recommended)

```bash
cargo build --release -p eggsec-cli
```

### Full Feature Set

```bash
cargo build --release -p eggsec-cli --features full
```

### REST API Server

```bash
cargo build --release -p eggsec-cli --features rest-api
```

### AI Integration

```bash
cargo build --release -p eggsec-cli --features ai-integration
```

### Nmap NSE Support

```bash
cargo build --release -p eggsec-cli --features nse
```

### Stress Testing (Raw Sockets)

```bash
cargo build --release -p eggsec-cli --features stress-testing
```

### All Features Combined

```bash
cargo build --release -p eggsec-cli --features full
```

Note: `grpc-api`, `ws-api`, `pdf`, and `nse-sandbox` are intentionally excluded from `full` and must be enabled separately.

## Installing from Source

```bash
# Install to ~/.cargo/bin
cargo install --path crates/eggsec-cli --features full

# Or with specific features
cargo install --path crates/eggsec-cli --features rest-api,ai-integration
```

## Configuration

After installation, create a configuration file:

```bash
# Default config location
mkdir -p ~/.config/eggsec
```

Config discovery order: explicit `-c` path, then `./eggsec.toml`,
`./.eggsec/eggsec.toml`, `./config/eggsec.toml`, `~/.config/eggsec/eggsec.toml`.

Example `~/.config/eggsec/eggsec.toml` (or generate a template with `eggsec --generate-config`):

```toml
[http]
timeout_secs = 30
verify_tls = true

[scan]
default_concurrency = 100
rate_limit_per_second = 100

[output]
format = "json"
color = true

# Optional: AI integration
[ai]
provider = "openai"
model = "gpt-4"
base_url = "https://api.openai.com/v1"
max_tokens = 4096
temperature = 0.7
```

## Verifying Installation

```bash
# Check version
eggsec --version

# Run help
eggsec --help

# Run a basic recon scan
eggsec recon example.com
```

## Running Tests

Integration tests live in the engine crate (`eggsec`), not the binary shell:

```bash
# Engine lib tests
make test   # cargo test --lib -p eggsec

# Full package suite (includes integration tests, needs rest-api)
make test-ci

# Architecture guards + formatting/lint
make check
```

## Linting

```bash
cargo clippy --lib -p eggsec-cli
```

## Troubleshooting

### Build fails with Rust version error

Ensure you have Rust 1.88 or later:
```bash
rustup update stable
```

### Raw socket features require root

The `stress-testing` feature uses raw sockets which require elevated privileges:
```bash
sudo ./target/release/eggsec scan-ports 192.168.1.1 --source-ip 10.0.0.1
```
