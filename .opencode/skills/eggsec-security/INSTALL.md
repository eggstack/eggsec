# Eggsec Installation Guide

## Prerequisites

- Rust 1.80 or later (MSRV: 1.80)
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

Example `~/.config/eggsec/config.toml`:

```toml
[target]
hosts = ["example.com"]

[scan]
timeout = 30
concurrency = 100

[fuzz]
rate_limit = 100
payload_count = 1000

[output]
format = "json"
path = "./reports"

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
eggsec recon --target example.com --dns
```

## Running Tests

```bash
# Library tests
cargo test --lib -p eggsec-cli

# Integration tests
cargo test --test scanner_tests -p eggsec-cli
cargo test --test negative_tests -p eggsec-cli

# All tests
cargo test -p eggsec-cli
```

## Linting

```bash
cargo clippy --lib -p eggsec-cli
```

## Troubleshooting

### Build fails with Rust version error

Ensure you have Rust 1.80 or later:
```bash
rustup update stable
```

### Raw socket features require root

The `stress-testing` feature uses raw sockets which require elevated privileges:
```bash
sudo ./target/release/eggsec scan ports --target 192.168.1.1 --spoof
```
