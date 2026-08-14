# Dependency, Architecture, and Verification Simplification — Closure Report

## Status

Roadmap phases A–J executed. Corrective closure pass, final polish pass,
post-polish corrective pass, and dispatch-profile parity corrective pass all
completed.

Final validated implementation SHA (dispatch-profile parity corrective pass):
`5366e77ead376a40713cf60f01cbce2817b5af07`.

Closure-record-only SHA for this document: `ca57755c` (documentation-only;
no Rust/Python/manifest/script/Makefile/workflow changes).

## Scope

Corrective engineering and structural simplification across 10 phases (A–J),
plus a corrective closure pass resolving remaining dependency security exceptions,
CI simplification, and documentation reconciliation.

## Phase Status

| Phase | Title | Status |
|-------|-------|--------|
| A | Authorization token and target-binding correction | Executed |
| B | Scope resolution and address-set correctness | Executed |
| C | Exhaustive compile-time feature registry | Executed |
| D | Operation, command, domain, and tool metadata consolidation | Executed |
| E | Advisory cleanup and dependency security remediation | Executed |
| F | Engine/application boundary and library-size reduction | Executed (CLI parsing remains gated behind `cli` feature for engine consumers; parser-independent `FuzzConfig`/`WafConfig`/`ReconRequest`/`LoadTestRunConfig`/`PortScanRequest`/`EndpointScanRequest`/`FingerprintRequest` introduced so non-CLI consumers can use the engine directly) |
| G | Daemon/TUI topology, TLS provider, and duplicate dependency cleanup | Executed |
| H | Upstream modernization, MSRV, and justified native-dependency reduction | Executed |
| I | CI and verification simplification | Executed |
| J | Measurement, documentation reconciliation, and closure | Executed |
| — | Corrective closure pass | Executed |
| — | Final polish pass (parser-independent types, Python `cli` removal) | Executed |
| — | Post-polish corrective pass (headless pipeline/tool-api parity) | Executed |
| — | Dispatch-profile parity corrective pass (profile state preservation) | Executed |

### Resolved in corrective closure pass

| Item | Resolution |
|------|-----------|
| PyO3 0.22 → 0.29.2 | Upgraded. RUSTSEC-2025-0020 and RUSTSEC-2026-0177 resolved. |
| quick-xml 0.31 → 0.41.0 | Upgraded. RUSTSEC-2026-0194 and RUSTSEC-2026-0195 resolved. |
| MSRV | Raised from 1.85 to 1.88 (required by quick-xml 0.41 + plist 1.10). |
| CI simplification | MSRV and portability checks moved to deep-checks.yml. Routine CI: Rust + Python only. |
| Legacy scope helpers | Removed (no production callers). |
| Advisory exceptions | PyO3 and quick-xml exceptions removed from deny.toml and DEPENDENCY_EXCEPTIONS.md. |

### Remaining residuals

| Item | Blocker | Owner |
|------|---------|-------|
| rusqlite 0.31 → 0.40 | Blocked by sqlx 0.8 libsqlite3-sys conflict; requires sqlx 0.9 (MSRV 1.94) | eggsec-daemon |
| fxhash unmaintained (RUSTSEC-2025-0057) | scraper upstream must drop it | eggsec-scanner |
| instant unmaintained (RUSTSEC-2024-0384) | notify upstream must drop it | eggsec-cli |
| number_prefix unmaintained (RUSTSEC-2025-0119) | indicatif upstream must drop it | eggsec-cli |

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
- 7 live exceptions documented in `docs/DEPENDENCY_EXCEPTIONS.md` with review-by dates; 4 resolved in corrective closure pass (PyO3, quick-xml), 3 retained (transitive unmaintained)
- `cargo deny check advisories` passes

### Phase F — Engine/application boundary

- CLI parsing feature-gated behind `cli` feature
- Logging subscriber, notifications, config watching moved to adapter crates
- `eggsec-python` no longer enables the `cli` feature; `clap`/`clap_complete` absent from Python/headless graphs

### Phase G — Binary topology

- `eggsec-daemon-protocol` is dependency-light (no persistence/TLS deps)
- No rusqlite in client/TUI crates
- One Rustls provider (ring) per artifact
- Tokio features managed at workspace level

### Phase H — Upstream modernization

- MSRV raised from 1.80 to 1.85 (required for kube 4.x); further raised to 1.88 in corrective closure pass (required by quick-xml 0.41 + plist 1.10)
- kube upgraded 0.92 → 4.2, k8s-openapi 0.22 → 0.28
- MongoDB/BSON 2.x → 3.x, Redis 0.25 → 1.x
- native-tls made optional behind `nse` feature
- gRPC proto: checked-in Rust code, protoc only for reflection descriptor

### Phase I — CI simplification

- `make check` uses package-level Cargo commands
- Mandatory CI: Linux-first (Rust, Python)
- MSRV and portability: scheduled/manual via deep-checks.yml
- `make check-full`: optional weekly/manual workflow
- No publication or tag-triggered release in any CI workflow

### Dispatch-profile parity corrective pass

- `Pipeline::from_profile(target, profile)` added as canonical parser-independent constructor
- Dispatch `run_pipeline()` now uses `Pipeline::from_profile()` instead of hand-mapping profiles to stages
- Tool-API `PipelineTool` forwards parsed profile via `run_with_callback_for_profile()`
- Dead `output_file`/`output_format` parameters removed from dispatch internal API
- 24 behavioral regression tests: profile construction, Quick non-empty, dispatch canonical stages, risk-budget consistency, defense-lab scope, feature-gate validation, `new()` emptiness
- `eggsec-python` remains independent of `cli`/Clap (verified via `cargo tree`)

## Artifact measurements

### Build host

- OS: Ubuntu 24.04 (noble) x86_64
- rustc: 1.97.1 (2026-07-14)
- cargo: 1.97.1
- Python: 3.12.3

### Artifact sizes

Measured at commit `7b878d79` on Ubuntu 24.04 x86_64, rustc 1.97.1.

| Profile | Artifact | Size | Stripped |
|---------|----------|------|----------|
| Default (TUI + daemon-client) | `target/release/eggsec` | 20.9 MB | Yes |
| Headless (no-default) | `target/release/eggsec` | 16.6 MB | Yes |
| Daemon-client only | `target/release/eggsec` | 17.6 MB | Yes |
| Daemon server | `target/release/eggsec-daemon` | 4.1 MB | Yes |
| Python wheel | `eggsec-0.1.0-cp312-cp312-manylinux_2_38_x86_64.whl` | 9.5 MB | N/A |

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

- CLI-only deps (`clap`, `clap_complete`) absent from `eggsec-python` and headless graphs
- `indicatif` retained in engine graph for progress reporting in scanners, fuzzer, loadtest, and pipeline modules
- Server SQLite (rusqlite) absent from client/TUI graphs
- One Rustls provider (ring) per artifact; no aws-lc-rs
- `eggsec-runtime` has no TUI, transport, or persistence dependencies
- `eggsec-output` has no engine or runtime dependencies

## Advisory/MSRV status

| Item | Status |
|------|--------|
| `cargo deny check advisories` | PASS (3 live exceptions documented) |
| MSRV 1.88 workspace check | PASS |
| Stable Rust check | PASS |
| RUSTSEC-2025-0057 (fxhash) | Exception until scraper 0.22+ |
| RUSTSEC-2024-0384 (instant) | Exception until notify 8+ |
| RUSTSEC-2025-0119 (number_prefix) | Exception until indicatif 0.18+ |
| RUSTSEC-2025-0020 (pyo3) | Resolved — upgraded to 0.29.2 |
| RUSTSEC-2026-0177 (pyo3) | Resolved — upgraded to 0.29.2 |
| RUSTSEC-2026-0194 (quick-xml) | Resolved — upgraded to 0.41.0 |
| RUSTSEC-2026-0195 (quick-xml) | Resolved — upgraded to 0.41.0 |

## CI/release status

| Item | Status |
|------|--------|
| Mandatory CI jobs | `rust` (make check), `python` (make check-python) |
| Scheduled/manual jobs | `deep-checks.yml` — weekly: make check-full, MSRV 1.88, macOS/Windows portability |
| Python build count | 1 (ubuntu-latest, CPython 3.12) |
| Portability policy | Compile-check only on macOS/Windows; scheduled/manual |
| Publication | Manual maintainer action only |
| Registry preflight | NOT RUN (requires maintainer release environment) |
| Tag-triggered release | None |
| `id-token: write` | Not present in any workflow |
| `cargo publish` in CI | Not present in any workflow |

## Validation commands and outcomes

Final validated implementation SHA (dispatch-profile parity corrective pass):
`5366e77ead376a40713cf60f01cbce2817b5af07`.
Earlier measurements retained below are tagged with their original measurement SHA.

| Command | Outcome | SHA |
|---------|---------|-----|
| `git rev-parse HEAD` | `5366e77ead376a40713cf60f01cbce2817b5af07` | `5366e77` |
| `git status --porcelain` | Clean working tree | `5366e77` |
| `rustc --version` | 1.97.1 | `5366e77` |
| `cargo --version` | 1.97.1 | `5366e77` |
| `python3 --version` | 3.12.3 | `5366e77` |
| `make check` | PASS | `5366e77` |
| `make check-python` | PASS | `5366e77` |
| `cargo deny check advisories` | PASS | `5366e77` |
| `cargo +1.88 check --workspace --no-default-features` | PASS | `5366e77` |
| `cargo tree -p eggsec-python -i clap` | PASS — not reachable | `5366e77` |
| `cargo tree -p eggsec-python -i clap_complete` | PASS — not reachable | `5366e77` |
| `cargo check -p eggsec --no-default-features` | PASS | `5366e77` |
| `cargo check -p eggsec --no-default-features --features tool-api` | PASS | `5366e77` |
| `cargo check -p eggsec --no-default-features --features tool-api,rest-api` | PASS | `5366e77` |
| `cargo check -p eggsec-python` | PASS | `5366e77` |
| `rg 'requires the .cli. feature\|requires the cli feature' crates/eggsec/src` | PASS — no regression | `5366e77` |
| `rg 'cargo publish\|maturin publish\|twine upload\|gh release\|id-token: write' .github/workflows` | PASS — no matches | `5366e77` |
| focused behavioral tests (24 in `pipeline::executor::tests`) | PASS | `5366e77` |
| hosted CI: `ci.yml` workflow 31724388231 | PASS | `5366e77` |
| hosted CI: `code-quality.yml` workflow 31724387469 | PASS | `5366e77` |
| `cargo build -p eggsec-cli --release` | PASS (20.9 MB) | `7b878d79` |
| `cargo build -p eggsec-cli --release --no-default-features` | PASS (16.6 MB) | `7b878d79` |
| `cargo build -p eggsec-cli --release --no-default-features --features daemon-client` | PASS (17.6 MB) | `7b878d79` |
| `cargo build -p eggsec-daemon --release` | PASS (4.1 MB) | `7b878d79` |
| `maturin build --release` | PASS (9.5 MB wheel) | `7b878d79` |

Final Python/CLI dependency result: `eggsec-python` does not enable the `cli`
feature; `clap` and `clap_complete` are not reachable from the default Python
artifact. The post-polish corrective pass restored headless pipeline/tool-API
parity: Fuzz/LoadTest/WAF/Recon pipeline stages execute through plain engine
types without requiring the `cli` feature. The dispatch-profile parity corrective
pass ensured `ScanProfile` is the single source of truth for pipeline profile
state, stage selection, risk budget, and profile-specific validation across
dispatch and tool-API construction paths.

## Publication status

| Registry | Status |
|----------|--------|
| crates.io | NOT RUN |
| PyPI | NOT RUN |
| TestPyPI | NOT RUN |
| GitHub Releases | NOT RUN |

No packages were published during closure.
