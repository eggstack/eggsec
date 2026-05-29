# Slapper Installation Guide

## Prerequisites

- Rust 1.80 or later (MSRV: 1.80)
- Cargo package manager
- Linux, macOS, or Windows

## Quick Install

```bash
# Clone the repository
git clone https://github.com/anomalyco/slapper.git
cd slapper

# Build with default features
cargo build --release -p slapper

# The binary will be at:
# target/release/slapper
```

## Feature-Specific Builds

### Default Build (Recommended)

```bash
cargo build --release -p slapper
```

### Full Feature Set

```bash
cargo build --release -p slapper --features full
```

### REST API Server

```bash
cargo build --release -p slapper --features rest-api
```

### AI Integration

```bash
cargo build --release -p slapper --features ai-integration
```

### Nmap NSE Support

```bash
cargo build --release -p slapper --features nse
```

### Stress Testing (Raw Sockets)

```bash
cargo build --release -p slapper --features stress-testing
```

### All Features Combined

```bash
cargo build --release -p slapper --features full
```

Note: `grpc-api` and `nse-sandbox` are intentionally excluded from `full` and must be enabled separately.

## Installing from Source

```bash
# Install to ~/.cargo/bin
cargo install --path crates/slapper --features full

# Or with specific features
cargo install --path crates/slapper --features rest-api,ai-integration
```

## Configuration

After installation, create a configuration file:

```bash
# Default config location
mkdir -p ~/.config/slapper
```

Example `~/.config/slapper/config.toml`:

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
slapper --version

# Run help
slapper --help

# Run a basic recon scan
slapper recon --target example.com --dns
```

## Running Tests

```bash
# Library tests
cargo test --lib -p slapper

# Integration tests
cargo test --test scanner_tests -p slapper
cargo test --test negative_tests -p slapper

# All tests
cargo test -p slapper
```

## Linting

```bash
cargo clippy --lib -p slapper
```

## Troubleshooting

### Build fails with Rust version error

Ensure you have Rust 1.80 or later:
```bash
rustup update stable
```

### Python plugin build fails

Ensure Python 3.14 development headers are installed:
```bash
# Ubuntu/Debian
sudo apt install python3.14-dev

# macOS
brew install python@3.14
```

### Ruby plugin build fails

Ensure Ruby development headers are installed:
```bash
# Ubuntu/Debian
sudo apt install ruby-dev

# macOS
brew install ruby
```

### Raw socket features require root

The `stress-testing` feature uses raw sockets which require elevated privileges:
```bash
sudo ./target/release/slapper scan ports --target 192.168.1.1 --spoof
```
