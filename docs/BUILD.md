# Build and Install

Eggsec requires Rust 1.89 or newer. The workspace root is a virtual Cargo
manifest, so build or install the CLI by naming `eggsec-cli`.

## Build from source

```bash
git clone https://github.com/eggstack/eggsec.git
cd eggsec
cargo build --release -p eggsec-cli
```

The CLI binary is `target/release/eggsec`. The default build includes the
terminal UI and the core scanning, fuzzing, reconnaissance, WAF, pipeline, and
reporting commands.

To install the CLI in `~/.cargo/bin`:

```bash
cargo install --path crates/eggsec-cli
eggsec --version
```

## Optional features

Add features only when you need them. For example:

```bash
# Headless CLI, without the terminal UI
cargo build --release -p eggsec-cli --no-default-features

# Daemon client commands
cargo build --release -p eggsec-cli --features daemon-client

# REST, MCP, and agent serving commands
cargo build --release -p eggsec-cli --features rest-api

# NSE script support
cargo build --release -p eggsec-cli --features nse

# Mobile analysis, wireless analysis, or database assessment
cargo build --release -p eggsec-cli --features mobile,wireless,db-pentest
```

The `full` feature is a curated developer and lab build, not an exhaustive
"enable everything" option:

```bash
cargo build --release -p eggsec-cli --features full
```

## Feature prerequisites

Some optional features need system libraries, platform tools, privileges, or
compatible hardware:

| Feature | Additional requirement |
|---------|------------------------|
| `packet-inspection` | libpcap development package to build; Linux and `CAP_NET_RAW` or root for live capture |
| `wireless` | Linux, `wireless-tools` (`iwlist`), and a managed lab interface; live scans need `CAP_NET_ADMIN` or root |
| `nse` | `libssl-dev` to build |
| `grpc-api` | `protobuf-compiler` when regenerating the reflection descriptor |
| `mobile-dynamic` | `adb` and an owned lab device or emulator for live testing |

This is a short list of common requirements, not a complete matrix. See the
[feature matrix](FEATURE_MATRIX.md) and [platform guide](PLATFORM.md) for details.
`eggsec doctor` checks local platform prerequisites.

To build the daemon with its HTTP/SSE transport, use the daemon package:

```bash
cargo build --release -p eggsec-daemon --features http-api
```
