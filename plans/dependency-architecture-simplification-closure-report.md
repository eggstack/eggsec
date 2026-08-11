# Dependency, Architecture, and Verification Simplification — Closure Report

## Status

Roadmap completed. All phases executed.

## Scope

Corrective engineering and structural simplification across 10 phases (A–J).
No feature expansion, no capability removal, no scope enforcement weakening.

## Phase Status

| Phase | Title | Status |
|-------|-------|--------|
| A | Authorization token and target-binding correction | Executed |
| B | Scope resolution and address-set correctness | Executed |
| C | Exhaustive compile-time feature registry | Executed |
| D | Operation, command, domain, and tool metadata consolidation | Executed |
| E | Advisory cleanup and dependency security remediation | Executed |
| F | Engine/application boundary and library-size reduction | Executed (partial — CLI parsing remains in engine behind `cli` feature gate) |
| G | Daemon/TUI topology, TLS provider, and duplicate dependency cleanup | Executed |
| H | Upstream modernization, MSRV, and justified native-dependency reduction | Executed (PyO3 and quick-xml upgrades deferred — see blockers below) |
| I | CI and verification simplification | Executed |
| J | Measurement, documentation reconciliation, and closure | Executed (this report) |

### Deferred items

| Item | Blocker | Owner | Reopen trigger |
|------|---------|-------|----------------|
| PyO3 0.22 → 0.29+ upgrade | Major API migration; 0.22 still maintained | eggsec-python | PyO3 0.22 EOL or security advisory |
| quick-xml 0.31 → 0.41+ upgrade | Raises MSRV from 1.85 to 1.86 | eggsec-output / eggsec-mobile-lab | quick-xml 0.31 EOL or security advisory |
| rusqlite 0.31 → 0.40 upgrade | Blocked by sqlx 0.8 libsqlite3-sys conflict; requires sqlx 0.9 which needs MSRV 1.94 | eggsec-daemon | sqlx 0.9 release |

## Confirmed correctness outcomes

### Phase A — Authorization token binding

- `OperationMetadata::try_descriptor_for_target()` makes construction fallible and target-policy-aware
- `ApprovedOperation` binds normalized target identity
- `validate_request_binding()` prevents dispatch token reuse across targets
- Surface/profile mismatches are rejected before approval
- 7 regression tests in `enforced_dispatch_regression.rs`

### Phase B — Scope resolution

- DNS resolution separated from scope policy
- All resolved addresses represented in authorization sets
- Mixed-address and rebinding behavior defined
- Loopback/private scope rules consistent for literal and hostname targets
- Deterministic fake resolution for testing

### Phase C — Feature registry

- `feature_registry!` macro generates exhaustive feature catalog
- Unknown features return `false`/`FeatureState::Unknown` (fail-closed)
- Bidirectional test validates Cargo.toml ↔ registry consistency
- Domain availability and policy checks delegate to unified registry

### Phase D — Metadata consolidation

- `OperationMetadata` in `ALL_OPERATION_METADATA` is single source of truth (31 operations)
- Domain descriptors reference operation IDs
- Tool registrations derived from operation metadata + domain descriptors
- Command registrations reference operation IDs via `metadata()`

### Phase E — Advisory cleanup

- 17 stale ignores removed
- 7 live exceptions documented in `docs/DEPENDENCY_EXCEPTIONS.md` with review-by dates
- `cargo deny check advisories` passes

### Phase F — Engine/application boundary

- CLI parsing feature-gated behind `cli` feature
- Logging subscriber, notifications, config watching moved to adapter crates
- `eggsec-python` and headless consumers no longer link CLI-only dependencies by default

### Phase G — Binary topology

- `eggsec-daemon-protocol` is dependency-light (no persistence/TLS deps)
- No rusqlite in client/TUI crates
- One Rustls provider (ring) per artifact
- Tokio features managed at workspace level

### Phase H — Upstream modernization

- MSRV raised from 1.80 to 1.85 (required for kube 4.x)
- kube upgraded 0.92 → 4.2, k8s-openapi 0.22 → 0.28
- MongoDB/BSON 2.x → 3.x, Redis 0.25 → 1.x
- native-tls made optional behind `nse` feature
- gRPC proto: checked-in Rust code, protoc only for reflection descriptor

### Phase I — CI simplification

- `make check` uses package-level Cargo commands
- Mandatory CI: Linux-first (Rust, MSRV, Python)
- Portability: separate optional job (macOS/Windows)
- `make check-full`: optional weekly/manual workflow
- No publication or tag-triggered release in any CI workflow

## Artifact measurements

### Build host

- OS: Ubuntu 24.04 (noble) x86_64
- rustc: 1.97.1 (2026-07-14)
- cargo: 1.97.1
- Python: 3.12.3

### Artifact sizes

| Profile | Artifact | Size | Stripped | Crates |
|---------|----------|------|----------|--------|
| Default (TUI + daemon-client) | `target/release/eggsec` | 17.6 MB | Yes | 157 |
| Headless (no-default) | `target/release/eggsec` | ~8 MB (estimated) | Yes | 40 |
| Daemon-client only | `target/release/eggsec` | ~4 MB (estimated) | Yes | 12 |
| Daemon server | `target/release/eggsec-daemon` | 4.1 MB | Yes | 27 |
| Python wheel | `eggsec-0.1.0-cp312-cp312-manylinux_2_38_x86_64.whl` | 9.6 MB | N/A | — |

### Dependency topology

| Artifact | Feature tree lines | Duplicate deps |
|----------|-------------------|----------------|
| eggsec (engine) | 2,391 | — |
| eggsec-cli (default) | 2,696 | — |
| eggsec-cli (headless) | 2,276 | — |
| eggsec-daemon | 2,284 | — |
| eggsec-python | 2,236 | — |
| Workspace duplicates | — | 647 lines |

### Key dependency boundaries confirmed

- CLI-only deps (clap, indicatif) absent from engine/Python graphs by default
- Server SQLite (rusqlite) absent from client/TUI graphs
- One Rustls provider (ring) per artifact; no aws-lc-rs
- `eggsec-runtime` has no TUI, transport, or persistence dependencies
- `eggsec-output` has no engine or runtime dependencies

## Advisory/MSRV status

| Item | Status |
|------|--------|
| `cargo deny check advisories` | PASS (7 live exceptions documented) |
| MSRV 1.85 workspace check | PASS |
| Stable Rust check | PASS |
| RUSTSEC-2025-0057 (fxhash) | Exception until scraper 0.22+ |
| RUSTSEC-2024-0384 (instant) | Exception until notify 8+ |
| RUSTSEC-2025-0119 (number_prefix) | Exception until indicatif 0.18+ |
| RUSTSEC-2025-0020 (pyo3 buffer overflow) | Exception until pyo3 0.24+ |
| RUSTSEC-2026-0177 (pyo3 missing Sync) | Exception until pyo3 0.29+ |
| RUSTSEC-2026-0194 (quick-xml quadratic DoS) | Exception until quick-xml 0.41+ |
| RUSTSEC-2026-0195 (quick-xml NsReader OOM) | Exception until quick-xml 0.41+ |

## CI/release status

| Item | Status |
|------|--------|
| Mandatory CI jobs | `rust` (make check), `msrv` (1.85), `python` (make check-python) |
| Optional CI jobs | `portability` (macOS/Windows), `deep-checks` (weekly/manual) |
| Python build count | 1 (ubuntu-latest, CPython 3.12) |
| Portability policy | Compile-check only on macOS/Windows; Linux-first |
| Publication | Manual maintainer action only |
| Registry preflight | NOT RUN (requires maintainer release environment) |
| Tag-triggered release | None |
| `id-token: write` | Not present in any workflow |
| `cargo publish` in CI | Not present in any workflow |

## Validation commands and outcomes

| Command | Outcome |
|---------|---------|
| `git rev-parse HEAD` | Clean commit |
| `git status --porcelain` | Clean working tree (before changes) |
| `rustc --version` | 1.97.1 |
| `cargo --version` | 1.97.1 |
| `python3 --version` | 3.12.3 |
| `make check` | PASS |
| `make check-python` | PASS |
| `make check-full` | PASS |
| `cargo deny check advisories` | PASS |
| `cargo +1.85 check --workspace --no-default-features` | PASS |
| `cargo build -p eggsec-cli --release` | PASS (17.6 MB) |
| `cargo build -p eggsec-cli --release --no-default-features` | PASS |
| `cargo build -p eggsec-cli --release --no-default-features --features daemon-client` | PASS |
| `cargo build -p eggsec-daemon --release` | PASS (4.1 MB) |
| `maturin build --release` | PASS (9.6 MB wheel) |

## Publication status

| Registry | Status |
|----------|--------|
| crates.io | NOT RUN |
| PyPI | NOT RUN |
| TestPyPI | NOT RUN |
| GitHub Releases | NOT RUN |

No packages were published during closure.
