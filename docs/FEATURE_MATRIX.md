# Eggsec Feature Matrix

> **Maintenance model**: This matrix is the canonical reference for all feature flags across the
> eggsec workspace. It is manually maintained to be consistent with `[features]` in
> `crates/eggsec/Cargo.toml`, `OperationMetadata` in `config/policy.rs`, and `DomainDescriptor`
> in `domain/mod.rs`. When adding or modifying a feature, update the source tables first, then
> update this file to match.
>
> **Scope**: Covers the main `eggsec` crate (42 features) and domain crate features. Does not
> repeat domain crate internals (those are documented in their own Cargo.toml comments).

---

## 1. Feature Inventory and Classification

Categories:

| Category | Meaning |
|----------|---------|
| **Protocol/front-end adapter** | Exposes a serving surface (REST, gRPC, WebSocket, MCP) |
| **Domain capability** | Enables a domain's core functionality (scanning, assessment) |
| **Domain protocol exposure marker** | Opt-in MCP/agent exposure for a domain |
| **Report/output** | Adds report generation or export formats |
| **Storage/integration** | Adds persistence or external service integration |
| **Backend/driver dependency** | Pulls in a specific driver crate for a domain |
| **Platform-sensitive/lab-only** | Requires root, CAP_NET_ADMIN, or lab-only hardware |
| **Marker-only** | No dependencies; compile-time gate only |

### 1.1 Main Crate Features

| Feature | Category | Implied Features | Declaring Crate | In Defaults | Affects Programmatic Exposure | OperationMetadata IDs | DomainDescriptor IDs |
|---------|----------|-----------------|-----------------|-------------|------------------------------|----------------------|---------------------|
| `tool-api` | Marker-only | — | eggsec | No | Yes (required by programmatic surfaces) | all | all |
| `insecure-tls` | Marker-only | — | eggsec | No | No | — | — |
| `rest-api` | Protocol/front-end adapter | `tool-api` | eggsec | No | Yes (REST surface) | all (REST-exposed) | all (REST-exposed) |
| `ws-api` | Protocol/front-end adapter | — | eggsec | No | Yes (WebSocket surface) | all | all |
| `grpc-api` | Protocol/front-end adapter | `tool-api` | eggsec | No | Yes (gRPC surface) | all (gRPC-exposed) | all (gRPC-exposed) |
| `stress-testing` | Domain capability | `pnet`, `pnet_packet`, `socket2`, `nix`, `libc`, `surge-ping`, `eggsec-nse?/stress-testing` | eggsec | No | No | `stress-test` | — |
| `packet-inspection` | Domain capability | `pnet`, `pnet_packet`, `libc` | eggsec | No | No | `packet` | — |
| `nse` | Domain capability | `tool-api`, `eggsec-nse` | eggsec | No | Yes (MCP-exposed) | `nse` | — |
| `nse-ssh2` | Backend/driver dependency | `nse`, `ssh2`, `eggsec-nse/nse-ssh2` | eggsec | No | No | `nse` | — |
| `nse-sandbox` | Domain capability | `nse`, `eggsec-nse/sandbox` | eggsec | No | No | `nse` | — |
| `advanced-hunting` | Marker-only | — | eggsec | No | Yes (MCP-exposed) | `hunt` | — |
| `compliance` | Marker-only | — | eggsec | No | Yes (MCP-exposed) | `compliance` | — |
| `external-integrations` | Marker-only | — | eggsec | No | Yes (MCP-exposed) | `integrations` | — |
| `finding-workflow` | Marker-only | — | eggsec | No | Yes (MCP-exposed) | `workflow` | — |
| `vuln-management` | Marker-only | — | eggsec | No | Yes (MCP-exposed) | `vuln` | — |
| `ai-integration` | Domain capability | `tool-api`, `eventsource-stream`, `semver` | eggsec | No | No | — | — |
| `websocket` | Domain capability | `tokio-tungstenite` | eggsec | No | No | — | — |
| `headless-browser` | Domain capability | `headless_chrome` | eggsec | No | Yes (MCP-exposed) | `browser` | — |
| `database` | Storage/integration | `sqlx` | eggsec | No | Yes (MCP-exposed) | `storage` | — |
| `db-pentest` | Domain capability | `sqlx`, `eggsec-db-lab`, `eggsec-db-lab/db-drivers` | eggsec | No | Yes (MCP via marker) | `db-pentest` | `db-pentest` |
| `db-pentest-mssql-tiberius` | Backend/driver dependency | `tiberius`, `eggsec-db-lab/mssql` | eggsec | No | No | — | — |
| `db-pentest-mongodb` | Backend/driver dependency | `mongodb`, `bson`, `eggsec-db-lab/mongodb` | eggsec | No | No | — | — |
| `db-pentest-redis` | Backend/driver dependency | `redis`, `eggsec-db-lab/redis` | eggsec | No | No | — | — |
| `db-pentest-mcp` | Domain protocol exposure marker | `db-pentest`, `eggsec-db-lab/mcp` | eggsec | No | Yes (MCP surface) | `db-pentest` | `db-pentest` |
| `c2-mcp` | Domain protocol exposure marker | `c2` | eggsec | No | Yes (MCP surface) | `c2` | — |
| `container` | Domain capability | `kube`, `k8s-openapi` | eggsec | No | No | — | — |
| `cloud` | Marker-only | — | eggsec | No | No | — | — |
| `sbom` | Report/output | `cyclonedx-bom`, `spdx`, `walkdir` | eggsec | No | No | — | — |
| `git-secrets` | Marker-only | — | eggsec | No | No | — | — |
| `pdf` | Report/output | `printpdf` | eggsec | No | No | — | — |
| `wireless` | Marker-only | — | eggsec | No | Yes (MCP-exposed) | `wireless` | — |
| `wireless-advanced` | Platform-sensitive/lab-only | `wireless` | eggsec | No | No | — | — |
| `evasion` | Marker-only | — | eggsec | No | No | — | — |
| `postex` | Marker-only | — | eggsec | No | No | — | — |
| `c2` | Domain capability | `postex`, `evasion` | eggsec | No | Yes (MCP via marker) | `c2` | — |
| `mobile` | Domain capability | `eggsec-mobile-lab`, `zip`, `plist` | eggsec | No | Yes (MCP-exposed) | `mobile-static` | `mobile-static` |
| `mobile-dynamic` | Domain capability | `mobile`, `eggsec-mobile-lab/mobile-dynamic` | eggsec | No | No | `mobile-dynamic` | `mobile-dynamic` |
| `api-schema` | Marker-only | — | eggsec | No | No | — | — |
| `web-proxy` | Domain capability | `eggsec-web-proxy`, `tokio-tungstenite`, `h2`, `http`, `prost`, `prost-types` | eggsec | No | Yes (MCP via marker) | `proxy-intercept` | — |
| `web-proxy-mcp` | Domain protocol exposure marker | `web-proxy`, `eggsec-web-proxy/web-proxy-mcp` | eggsec | No | Yes (MCP surface) | `proxy-intercept` | — |
| `transparent-proxy` | Domain capability | `web-proxy`, `eggsec-web-proxy/transparent-proxy` | eggsec | No | No | — | — |
| `dynamic-plugins` | Domain capability | `web-proxy`, `eggsec-web-proxy/dynamic-plugins` | eggsec | No | No | — | — |
| `full` | Meta/aggregate (developer/lab) | see `Cargo.toml` | eggsec | No | No | — | — |

### 1.1a Daemon Crate Features

| Feature | Category | Implied Features | Declaring Crate | In Defaults | Notes |
|---------|----------|-----------------|-----------------|-------------|-------|
| `http-api` | Protocol/front-end adapter | `axum`, `async-stream`, `futures` | eggsec-daemon | No | HTTP/SSE transport for daemon (loopback-only default bind) |
| `full-executor` | Execution adapter | `dep:eggsec` | eggsec-daemon | No | Real task execution via `EggsecRuntimeExecutor` (enforcement + dispatch) |

### 1.1b CLI Crate Features

| Feature | Category | Implied Features | Declaring Crate | In Defaults | Notes |
|---------|----------|-----------------|-----------------|-------------|-------|
| `tui` | Protocol/front-end adapter | `dep:eggsec-tui` | eggsec-cli | Yes | Terminal UI adapter |
| `daemon-client` | Protocol/front-end adapter | `dep:eggsec-daemon`, `dep:tokio-util`, `eggsec/daemon-client` | eggsec-cli | No | Daemon client CLI commands |
| `headless` | Marker-only | — | eggsec-cli | No | Headless/CI builds (no TUI, no daemon client) |

### 1.2 Domain Crate Features

| Crate | Feature | Category | Implies | Notes |
|-------|---------|----------|---------|-------|
| `eggsec-nse` | `nse` | Domain capability | — | Lua VM for NSE script execution |
| `eggsec-nse` | `nse-ssh2` | Backend/driver dependency | `nse`, `ssh2` | SSH2/libssh2-backed NSE support |
| `eggsec-nse` | `sandbox` | Domain capability | — | Restrict dangerous Lua operations |
| `eggsec-nse` | `stress-testing` | Domain capability | — | NSE stress-testing primitives |
| `eggsec-db-lab` | `db-drivers` | Domain capability | — | Core DB pentest driver abstractions |
| `eggsec-db-lab` | `mssql` | Backend/driver dependency | — | Real MSSQL client (tiberius) |
| `eggsec-db-lab` | `mongodb` | Backend/driver dependency | — | Real MongoDB client |
| `eggsec-db-lab` | `redis` | Backend/driver dependency | — | Real Redis client |
| `eggsec-db-lab` | `mcp` | Domain protocol exposure marker | — | MCP tool registration for db-pentest |
| `eggsec-web-proxy` | `web-proxy` | Domain capability | — | Core HTTP/HTTPS/WebSocket proxy |
| `eggsec-web-proxy` | `web-proxy-mcp` | Domain protocol exposure marker | — | MCP tool registration for web-proxy |
| `eggsec-web-proxy` | `transparent-proxy` | Domain capability | — | iptables/nftables REDIRECT mode |
| `eggsec-web-proxy` | `dynamic-plugins` | Domain capability | — | Dynamic plugin loading from .so/.dylib |
| `eggsec-mobile-lab` | `mobile-dynamic` | Domain capability | — | ADB + logcat + Frida runtime testing |

### 1.3 Defaults

The default feature set is **empty** (`default = []`). The `full` meta-feature enables everything
but is not a default. Users build with explicit feature flags or the `full` meta-feature.

The `eggsec-cli` crate has `default = ["tui"]`. Build with `--no-default-features` for headless
usage, or `--no-default-features --features daemon-client` for daemon client mode.

---

## 2. Feature Naming Conventions

| Pattern | Convention | Examples |
|---------|-----------|----------|
| Base domain feature | `<domain>` | `db-pentest`, `mobile`, `wireless`, `web-proxy`, `nse` |
| Protocol exposure marker | `<domain>-mcp` | `db-pentest-mcp`, `web-proxy-mcp`, `c2-mcp` |
| Backend driver | `<domain>-<backend>` | `db-pentest-mongodb`, `db-pentest-redis` |
| Advanced extension | `<domain>-advanced` | `wireless-advanced` |
| Platform-sensitive driver | `<domain>-<driver>` | `db-pentest-mssql-tiberius` (pure-Rust TDS) |
| Meta/aggregate | `full` | Enables all non-default features |

**Naming consistency**: All features follow these patterns consistently. No deviations observed.

---

## 3. Build Profiles and Test Matrix

### 3.1 Profiles

#### minimal — No default features

```bash
cargo check -p eggsec --no-default-features
cargo test --lib -p eggsec --no-default-features
```

#### manual-standard — CLI/TUI plus standard manual workflows (default build)

```bash
cargo build --release -p eggsec-cli
cargo test --lib -p eggsec
cargo clippy --lib -p eggsec
```

#### protocol — Tool API plus REST/gRPC/WebSocket

```bash
cargo check -p eggsec --features rest-api
cargo check -p eggsec --features grpc-api
cargo check -p eggsec --features ws-api
cargo test --lib -p eggsec --features rest-api
cargo test --lib -p eggsec --features grpc-api
```

Requires system deps: `grpc-api` needs protobuf compiler for gRPC reflection.

#### database-lab — Database domain and selected driver features

```bash
cargo check -p eggsec --features db-pentest
cargo check -p eggsec --features db-pentest,db-pentest-mssql-tiberius
cargo check -p eggsec --features db-pentest,db-pentest-mongodb
cargo check -p eggsec --features db-pentest,db-pentest-redis
cargo check -p eggsec --features db-pentest,db-pentest-mcp
cargo test --lib -p eggsec --features db-pentest
cargo test -p eggsec-db-lab
```

#### mobile-lab — Mobile static and optional dynamic runtime workflows

```bash
cargo check -p eggsec --features mobile
cargo check -p eggsec --features mobile-dynamic
cargo test --lib -p eggsec --features mobile
cargo test --lib -p eggsec --features mobile-dynamic
cargo test -p eggsec-mobile-lab
```

#### proxy-lab — Web proxy domain and optional protocol exposure marker

```bash
cargo check -p eggsec --features web-proxy
cargo check -p eggsec --features web-proxy-mcp
cargo test --lib -p eggsec --features web-proxy
cargo test -p eggsec-web-proxy
```

#### advanced-lab — Explicitly opt-in advanced/lab-only features

```bash
cargo check -p eggsec --features wireless
cargo check -p eggsec --features wireless-advanced
cargo check -p eggsec --features evasion
cargo check -p eggsec --features postex
cargo check -p eggsec --features c2
cargo check -p eggsec --features stress-testing
cargo check -p eggsec --features nse
cargo test --lib -p eggsec --features wireless
cargo test --lib -p eggsec --features evasion
cargo test --lib -p eggsec --features postex
cargo test --lib -p eggsec --features c2
cargo test --lib -p eggsec --features stress-testing
cargo test -p eggsec-nse --features nse
```

#### docs-metadata — Broad metadata/doc validation without heavy optional dependencies

```bash
cargo check --workspace --no-default-features
cargo test -p eggsec --test metadata_consistency
```

#### full — Developer/lab aggregate (not a production profile)

The `full` meta-feature enables all non-default features including advanced/lab-only capabilities
(`wireless-advanced`, `evasion`, `postex`, `c2`, `mobile-dynamic`). It is intended for development,
integration testing, and explicit lab builds. **`full` is not a conservative user/default profile**
and should not be recommended for production or standard deployment.

```bash
cargo check -p eggsec --features full
cargo test --lib -p eggsec --features full
```

#### cli-headless — Headless CLI (no TUI, no daemon client)

```bash
cargo check -p eggsec-cli --no-default-features
cargo test -p eggsec-cli --no-default-features
```

#### cli-daemon-client — Daemon client CLI commands

```bash
cargo check -p eggsec-cli --no-default-features --features daemon-client
cargo test -p eggsec-cli --no-default-features --features daemon-client
```

#### daemon-http — Daemon with HTTP/SSE transport

```bash
cargo check -p eggsec-daemon --features http-api
cargo test -p eggsec-daemon --features http-api
```

#### daemon-full-executor — Daemon with real task execution

```bash
cargo check -p eggsec-daemon --features full-executor
cargo test -p eggsec-daemon --features full-executor
```

### 3.2 System Dependency Requirements

| Profile | Required System Dep | Install (Debian/Ubuntu) |
|---------|-------------------|------------------------|
| `packet-inspection` | libpcap-dev | `apt install libpcap-dev` |
| `nse` | libssl-dev | `apt install libssl-dev` |
| `wireless` / `wireless-advanced` | wireless-tools (iwlist) | `apt install wireless-tools` |
| `grpc-api` | protobuf-compiler | `apt install protobuf-compiler` |
| `mobile-dynamic` | ADB + Android device | Android SDK Platform Tools |
| `stress-testing` | Raw sockets | `CAP_NET_RAW` or root |

---

## 4. Feature-to-Metadata Cross-Reference

| Feature | OperationMetadata IDs | DomainDescriptor IDs | Programmatic Exposure |
|---------|----------------------|---------------------|-----------------------|
| `rest-api` | all REST-exposed ops | all REST-exposed domains | Yes (REST server) |
| `grpc-api` | all gRPC-exposed ops | all gRPC-exposed domains | Yes (gRPC server) |
| `ws-api` | all ops (WebSocket transport) | — | Yes (WebSocket server) |
| `nse` | `nse` | — | Yes (via `tool-api`) |
| `db-pentest` | `db-pentest` | `db-pentest` | Yes (via `db-pentest-mcp`) |
| `db-pentest-mcp` | `db-pentest` | `db-pentest` | Yes (MCP surface) |
| `c2` | `c2` | — | Yes (via `c2-mcp`) |
| `c2-mcp` | `c2` | — | Yes (MCP surface) |
| `web-proxy` | `proxy-intercept` | — | Yes (via `web-proxy-mcp`) |
| `web-proxy-mcp` | `proxy-intercept` | — | Yes (MCP surface) |
| `wireless` | `wireless` | — | Yes (MCP-exposed) |
| `mobile` | `mobile-static` | `mobile-static` | Yes (MCP-exposed) |
| `mobile-dynamic` | `mobile-dynamic` | `mobile-dynamic` | No (lab-only) |
| `advanced-hunting` | `hunt` | — | Yes (MCP-exposed) |
| `headless-browser` | `browser` | — | Yes (MCP-exposed) |
| `compliance` | `compliance` | — | Yes (MCP-exposed) |
| `database` | `storage` | — | Yes (MCP-exposed) |
| `external-integrations` | `integrations` | — | Yes (MCP-exposed) |
| `finding-workflow` | `workflow` | — | Yes (MCP-exposed) |
| `vuln-management` | `vuln` | — | Yes (MCP-exposed) |
| `stress-testing` | `stress-test` | — | No |
| `packet-inspection` | `packet` | — | No |
| `evasion` | — | — | No |
| `postex` | — | — | No |
| `container` | — | — | No |
| `sbom` | — | — | No |
| `pdf` | — | — | No |
| `cloud` | — | — | No |
| `git-secrets` | — | — | No |
| `api-schema` | — | — | No |
| `tool-api` | all | all | Required base |
| `insecure-tls` | — | — | No |
| `ai-integration` | — | — | No |
| `websocket` | — | — | No |
| `transparent-proxy` | — | — | No |
| `dynamic-plugins` | — | — | No |
| `db-pentest-mssql-tiberius` | — | — | No |
| `db-pentest-mongodb` | — | — | No |
| `db-pentest-redis` | — | — | No |
| `nse-ssh2` | — | — | No |
| `nse-sandbox` | — | — | No |

---

## 5. Safety Invariants

1. **Feature presence is not authorization.** Runtime policy (`EnforcementContext`) is always required
   before dispatch. Enabling `db-pentest` does not grant permission to run database pentests —
   `--allow-db-pentest` and scope rules are mandatory.

2. **The `full` meta-feature is a developer/lab aggregate, not a safe default.** It includes advanced
   features like `wireless-advanced`, `evasion`, `postex`, and `c2`. Standard deployment should use
   explicit feature flags. `full` is for development, integration testing, and lab builds.

3. **Protocol exposure markers are opt-in.** `db-pentest-mcp`, `web-proxy-mcp`, and `c2-mcp` are
   not defaults and are not included in any base profile. Domains are standalone defense-lab
   surfaces unless the marker is explicitly enabled.

4. **Domain base features are clearly distinguished from protocol exposure features.** A domain's
   core capability (e.g., `db-pentest`) and its MCP exposure marker (e.g., `db-pentest-mcp`) are
   separate features. The base feature enables the domain; the marker registers it with MCP/agent
   surfaces.

5. **Strict profiles never honor manual overrides.** MCP, REST, gRPC, and agent surfaces use
   `EnforcementContext::evaluate()` as a mandatory pre-dispatch gate. `Warn` and
   `RequireConfirmation` outcomes are denied in strict mode.

6. **Marker-only features have no dependencies.** Features like `advanced-hunting`, `compliance`,
   `wireless`, `evasion`, `postex`, and `cloud` are compile-time gates only. They do not pull
   optional crates and are safe for all build environments.

7. **Backend driver features require the base domain feature.** `db-pentest-mongodb` is a no-op
   without `db-pentest`. The main crate enforces this via `dep:` syntax and domain crate feature
   forwarding.

---

## Updating This Document

This document is manually maintained. When adding or modifying a feature:

1. Update `[features]` in `crates/eggsec/Cargo.toml`
2. Update `OperationMetadata` in `crates/eggsec/src/config/policy.rs` (if operation-related)
3. Update `DomainDescriptor` in `crates/eggsec/src/domain/mod.rs` (if domain-related)
4. Run `cargo test --lib -p eggsec` to verify metadata consistency
5. Update this file to reflect the new feature (category, dependencies, metadata IDs, etc.)
6. Run `cargo test -p eggsec --test metadata_consistency` to validate cross-references
