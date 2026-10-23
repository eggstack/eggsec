# Daemon Feature Profile — `service`

The GUI-serving daemon is built from one **profile**: the cargo feature
`service` on `crates/eggsec-daemon`, which enables an explicit, documented
list of engine features.

- The **list** is owned by the engine —
  [`eggsec::config::daemon_profile::DAEMON_SERVICE_PROFILE`](../crates/eggsec/src/config/daemon_profile.rs)
  — and is checked against the daemon's manifest, against this document, and
  against the compiled binary by the conformance tests.
- The **build recipe** is [`scripts/build-daemon-profile.sh`](../scripts/build-daemon-profile.sh).
  It produces the binary plus a metadata sidecar; it performs no signing, no
  bundling and no packaging.
- The profile is a **build-side** contract. The *published* enabled-feature
  set on the wire belongs to the desktop-GUI frontend's capability
  negotiation, not to this document.

## Why this profile exists

The engine's `full` aggregate is a **developer/lab** profile: it enables the
REST serving surface, the mobile analysis stack, the AI integration and the
container scanner. None of those are what a daemon that serves the desktop
GUI needs, and two of them would be actively wrong: `rest-api` would give a
daemon that already speaks the Unix socket a second, unintended HTTP surface,
and `insecure-tls` would bypass certificate verification.

The inclusion rule is mechanical:

> An engine feature is **included** iff a code path reachable from one of the
> runtime's task kinds is `cfg`-gated on it, **or** it is the `cli` host gate,
> **or** it is a transitive dependency of an included feature and is declared
> explicitly so the effective set equals the declared set.

`cli` is a **host gate**, not a CLI surface: it gates the engine's
`commands` / `dispatch` / `runtime_bridge` modules, which is where
`EggsecRuntimeExecutor` lives. Without it the daemon cannot host the executor
it is built around.

## Included features

| Feature | Rationale |
|---|---|
| `cli` | Host gate: gates `commands` / `dispatch` / `runtime_bridge` (`crates/eggsec/src/lib.rs:88-91`, `:104-105`, `:136-137`), where `EggsecRuntimeExecutor` lives. The daemon cannot host its executor without it. |
| `advanced-hunting` | Required by the policy catalog (`crates/eggsec-policy/src/catalog.rs:642`) for the `Hunt` task kind. |
| `c2` | Required by the catalog (`:574`) for the `C2` task kind. |
| `compliance` | Required by the catalog (`:676`) for the `Compliance` task kind. |
| `database` | Required by the catalog (`:693`) for the `Storage` task kind. |
| `db-pentest` | Required by the catalog (`:557`) for the `DbPentest` task kind. The **base** domain only — the drivers stay opt-in. |
| `evasion` | Transitively implied by `c2` (`crates/eggsec/Cargo.toml:423`), declared explicitly so the effective set equals the declared set. |
| `external-integrations` | Required by the catalog (`:718`) for the `Integrations` task kind. |
| `finding-workflow` | Required by the catalog (`:743`) for the `Workflow` task kind. |
| `headless-browser` | Required by the catalog (`:659`) for the `Browser` task kind. Runtime prerequisite: a Chrome/Chromium binary. |
| `nse` | Required by the catalog (`:536`) for the `Nse` task kind. Build prerequisite: a C toolchain for vendored OpenSSL. |
| `packet-inspection` | Required by the catalog (`:468`) for the `PacketCapture`, `PacketTraceroute` and `PacketSend` task kinds. |
| `postex` | Transitively implied by `c2` (`crates/eggsec/Cargo.toml:423`), declared explicitly. |
| `stress-testing` | Required by the catalog (`:451`) for the `StressTest` task kind. |
| `tool-api` | Gates `pub mod tool` (`crates/eggsec/src/lib.rs:167-168`); transitively implied by `nse` (`crates/eggsec/Cargo.toml:329`), declared explicitly. |
| `vuln-management` | Required by the catalog (`:768`) for the `Vuln` task kind. |
| `web-proxy` | Required by the catalog (`:591`) for the `Intercept` task kind. |
| `websocket` | Changes the **behaviour** of the already-required `Fuzz` task kind: with it the fuzzer opens a real WebSocket and runs live connection/origin/injection tests (`crates/eggsec/src/fuzzer/advanced.rs:428-456`); without it it only emits static payloads (`:420-426`). |
| `wireless` | Required by the catalog (`:608`) for the `Wireless` and `WirelessActive` task kinds. |
| `wireless-advanced` | The actual compile gate for `WirelessActive` (`crates/eggsec/src/dispatch/canonical_execution.rs:1824-1855`); the catalog's separate `wireless-deauth` entry carries it (`:625`). Without it `WirelessActive` degrades to passive `wireless` only. |

## Excluded features

"Engine rule" is the engine's own `FULL_EXCLUDED_WITH_REASON` classification
(`crates/eggsec/src/config/feature_registry.rs`); "Daemon rule" is the reason
specific to a GUI-serving daemon.

| Feature | Engine rule | Daemon-specific reason |
|---|---|---|
| `ai-integration` | — | Measured dead path: the only ways to install an AI component are `FuzzEngine::set_ai_generator` (`crates/eggsec/src/fuzzer/engine/core.rs:264`) and `set_ai_bypass` (`crates/eggsec/src/waf/mod.rs:138`), and neither has a caller in the engine. The only other consumer, `crate::ai::AiClient` (`crates/eggsec/src/commands/handlers/ai_analyze.rs:41`), lives in the `rest-api`-gated `agent` module (`crates/eggsec/src/lib.rs:173-174`). |
| `api-schema` | `marker-only` | The daemon publishes the operation schema through `eggsec-daemon-protocol`'s `DescribeOperations`, not through the engine's `api_schema` module (`crates/eggsec/src/lib.rs:81-82`). |
| `c2-mcp` | `exposure-marker` | MCP exposure is a different protocol surface; the daemon speaks the Unix socket. |
| `cloud` | `marker-only` | No catalog entry, no task kind. |
| `config-watch` | `implicit-base` | Its only consumer is `crates/eggsec/src/agent/config_watcher.rs`, inside the `rest-api`-gated `agent` module (`crates/eggsec/src/lib.rs:173-174`). Excluding `rest-api` excludes it. |
| `container` | — | Measured unreachable: the `container` module has public APIs (`crates/eggsec/src/container/docker.rs:44`, `:86`; `crates/eggsec/src/container/cis.rs:43`) and `crates/eggsec/src/recon/containers.rs` is declared at `crates/eggsec/src/recon/mod.rs:81`, but no handler and no catalog entry references it. Enabling it would add `kube` + `k8s-openapi` for an unreachable capability. |
| `daemon-client` | `process-host` | The daemon **is** the host. It has no daemon to be a client of. |
| `db-pentest-mcp` | `exposure-marker` | Same as `c2-mcp`. |
| `db-pentest-mongodb` | `backend-driver` | The base `db-pentest` covers Postgres + MySQL through `eggsec-db-lab/db-drivers` (`crates/eggsec/Cargo.toml:376`). The MongoDB driver is an operator opt-in. |
| `db-pentest-mssql-tiberius` | `backend-driver` | Same, and this one additionally keeps `tiberius 0.12.3` → `tokio-rustls 0.24.1` → `rustls 0.21.12` out of the shipped daemon closure. |
| `db-pentest-redis` | `backend-driver` | Same as `db-pentest-mongodb`. |
| `dynamic-plugins` | `output-mode` | A web-proxy extension mode with its own profile. |
| `email-notifications` | `full` member | Its only consumer is `crates/eggsec/src/agent/alerts/routing.rs` (`lettre`), inside the `rest-api`-gated `agent` module. |
| `full` | the aggregate itself | A profile must not enable an aggregate; it would create a second, larger, undocumented feature set and a circular definition. |
| `git-secrets` | `marker-only` | No catalog entry, no task kind. |
| `grpc-api` | `serving-surface` | The daemon is the serving surface. It also requires `protoc` at build time (`crates/eggsec/build.rs:5-29`). |
| `insecure-tls` | `security-risk` | Never. Bypasses certificate verification. Absolute exclusion — it is in the profile's forbidden set. |
| `logging-subscriber` | `full` member | The daemon installs its own `tracing-subscriber` (`crates/eggsec-daemon/Cargo.toml:65`). The engine's file-appender mode (`crates/eggsec/src/lib.rs:122-123`) is a CLI/TUI concern. |
| `mobile` | `full` member | No mobile task kind in the runtime's request surface. Excluding it keeps `zip 2.4.2` — which carries RUSTSEC-2026-0367 / -0368 / -0369 — out of the shipped closure. |
| `mobile-dynamic` | `full` member | Same, and it is a `platform-mode` domain whose live path needs `adb` and a lab device. |
| `nse-sandbox` | `platform-mode` | An NSE hardening mode with its own profile; it is not what the `Nse` task kind needs. |
| `nse-ssh2` | `platform-mode` | Requires the system `libssh2` library (`libssh2-sys` → system link). Excluding it keeps a system-library prerequisite out of the daemon build. |
| `pdf` | `output-mode` | The daemon produces no reports — `TaskResultEnvelope.payload` is always `json!({})` (`crates/eggsec/src/dispatch/canonical_execution.rs:1111-1116`). Report rendering is a separate read path. |
| `rest-api` | `serving-surface` | The daemon already serves the Unix socket; a second HTTP surface would also pull `email-notifications` and `config-watch` in through `rest-api` (`crates/eggsec/Cargo.toml:311`). |
| `sbom` | `full` member | SBOM is a CLI-only path (`crates/eggsec/src/commands/handlers/sbom.rs`); there is no SBOM task kind and no catalog entry. |
| `test-helpers` | `test-only` | Never in a shipped aggregate. Absolute exclusion. |
| `transparent-proxy` | `output-mode` | A web-proxy extension mode with its own profile. |
| `udp-scan` | `platform-mode` | UDP verdicts depend on an ICMP receiver whose availability is OS- and privilege-dependent; no catalog entry requires it. |
| `web-proxy-mcp` | `exposure-marker` | Same as `c2-mcp`. |
| `ws-api` | `serving-surface` | The daemon is the serving surface. |

## System prerequisites

The three categories below are **different kinds of requirement** and must not
be conflated: a build-time requirement is needed to *compile* the profile, a
runtime-privilege requirement is needed to *run* a domain live, and a
runtime-binary requirement is a program that must be present on the host at
run time.

The domain ids are the engine's own — `crates/eggsec/src/platform/prereqs.rs`
(`all_domain_ids()`, `report_for()`, `skip_reason_for()`), which is also what
`docs/PLATFORM.md` tabulates.

| Domain | Build-time | Runtime-privilege | Runtime-binary |
|---|---|---|---|
| `mobile-dynamic` | none (excluded from the profile) | none for the fixture path; a lab device you own for live | `adb` (optional), `frida` CLI optional, emulator/AVD |
| `packet-inspection` | none — pure-Rust `pnet` (`crates/eggsec/Cargo.toml:326`), no `libpcap-dev` | Linux + `CAP_NET_RAW` (or root) for live capture; fixture paths run anywhere | `lo` for loopback checks, `ip` for netns |
| `wireless` | none | Linux + `CAP_NET_ADMIN` (or root) for live discovery; passive parsing runs anywhere | `iwlist` (wireless-tools) |
| `wireless-advanced` | none | Linux + `CAP_NET_ADMIN` (or root) + monitor mode + `--allow-active-wireless` | monitor interface (e.g. `wlan0mon`) |
| `stress-testing` | none — pure-Rust `pnet` (`crates/eggsec/Cargo.toml:323`), no `libpcap-dev` | Linux + `CAP_NET_RAW` (or root) for live floods; live floods are manual, scoped, lab-only | none beyond scope + budgets |
| `nse` | **C toolchain** (`cc`, `perl`, `make`) for **vendored** OpenSSL (`eggsec-nse 0.2.0` uses `openssl` with `features = ["vendored"]`) — **not** a system `libssl-dev` | none (local script execution) | none |

The `nse` row is the profile's **only** build-time system requirement class.
Two claims that were previously wrong are corrected here and in
`docs/PLATFORM.md` / `docs/FEATURE_MATRIX.md`: `packet-inspection` and
`stress-testing` do **not** need `libpcap-dev`, and `nse` does **not** need
`libssl-dev`.

### Runtime binaries outside the domain matrix

`headless-browser` is in the profile but is not one of the engine's six
platform-sensitive domains, so it has no row above. Its requirement is a
**runtime binary**, never a build-time one:

| Feature | Runtime-binary | Build-time |
|---|---|---|
| `headless-browser` | a Chrome/Chromium binary on the host | none |

The **build recipe**'s prerequisite probe reports, for the host it runs on,
which of the profile's runtime domains are live-capable. It calls the engine's
own `skip_reason_for` / `report_for` rather than reimplementing the check, and
it is **advisory**: it reports, it never blocks the build.

## Build recipe

```bash
bash scripts/build-daemon-profile.sh --profile service --out /tmp/eggsec-daemon-dist
```

The script refuses a dirty tree unless `--allow-dirty` is passed (and records
the dirty flag either way), pins the build to

```
cargo build --release --locked -p eggsec-daemon --features service --target <triple>
```

and writes a metadata sidecar next to the binary recording: the commit SHA,
the dirty flag, `rustc -Vv`, the resolved toolchain version, the MSRV floor
(**1.89**), the target triple, the **resolved engine feature set**, the
`Cargo.lock` hash, the binary size and its SHA-256.

The script performs **no** signing, **no** bundling, **no** packaging and no
npm step.

### Running the daemon

The cargo feature and the runtime flag are two different mechanisms:

| | Mechanism | Meaning |
|---|---|---|
| `cargo build --features service` | build time | which engine features the binary contains |
| `eggsec-daemon --full-executor` | run time | whether the process hosts the engine executor at all |

The runtime flag's **name and meaning are unchanged**. After building, run:

```bash
eggsec-daemon --full-executor --socket-path /tmp/eggsec-daemon.sock
```

## Conformance

| Check | Where |
|---|---|
| The manifest's `service` list equals the profile | `crates/eggsec/tests/feature_matrix.rs` |
| The documented table equals the profile | `crates/eggsec/tests/feature_matrix.rs` |
| The compiled feature set equals the profile | `crates/eggsec-daemon/tests/feature_profile.rs` |
| The profile enables nothing forbidden | both |
| The required set is derived from the policy catalog | `crates/eggsec/src/config/daemon_profile.rs` |

`full-executor` remains a **deprecated alias** for `service`
(`full-executor = ["service"]`), so every existing invocation keeps working
and now produces the profile daemon. The runtime flag `--full-executor` is
unaffected by that rename.
