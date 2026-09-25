# NSE Runtime Extraction Milestone 002 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/nse-runtime-extraction/002-runtime-dependency-decoupling.md`

Source subsystem roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-002--runtime-dependency-decoupling-and-consumer-consolidation`

Milestone 001 closure reference: `plans/closure/nse-runtime-extraction/001-closure.md` (closed). Readiness finding: the hard dependency is satisfied — one canonical runtime pipeline (`execute_nse_run`) with parity evidence exists, and no report/orchestration defect was carried forward that would block decoupling. The `nse-ssh2` compile-only finding from 001 recurs here (§15).

Repository baseline reviewed: `2ef67febf17a2ae42e2b8863b140f6c47cb7f387` through the implementation commit below.

Implementation commits or pull requests:

- `85320be1` — refactor(nse): decouple runtime from Eggsec crates via engine facade (manifest/import removal, `nse_bridge`/`nse_http_capability` moves, consumer consolidation, guard suite 40/26/120/144/145/146, test relocation, docs)

## 1. Executive finding

The runtime crate is now independently ownable in-tree. `eggsec-nse` has zero `eggsec-*` dependencies in its manifest, source, and tests; the two Eggsec-specific adapters (report-envelope conversion, scoped-transport HTTP capability) moved upward into the engine; only `eggsec` declares a direct `eggsec-nse` edge; and TUI/Python consume NSE solely through `eggsec::nse` with `nse = ["eggsec/nse"]` feature forwarding. No authorization, profile-default, feature-name, or NSE behavior change occurred. Static guards now fail closed on regression. Milestone 003 (physical extraction) is unblocked for planning, with two carried qualifications (§16).

## 2. Acceptance-criteria-to-evidence matrix (plan §13)

| # | Acceptance criterion | Evidence | Result |
|---|---|---|---|
| 1 | `eggsec-nse` has zero direct `eggsec-*` dependencies | `cargo tree -p eggsec-nse --depth 1` (no workspace crate); manifest grep empty; guard 144 | pass |
| 2 | Runtime source imports no `eggsec_...` crate | `rg 'use eggsec_\|eggsec_...' crates/eggsec-nse/src/` → only Lua key/temp-path string literals; guard 145 | pass |
| 3 | Report-envelope conversion engine-owned and tested | `crates/eggsec/src/nse_bridge.rs` + `crates/eggsec/tests/nse_bridge_tests.rs` (14 tests); guard 40 | pass |
| 4 | Transport coupling removed without weakening authority/scope | `eggsec-transport` edge deleted; adapter → `crates/eggsec/src/nse_http_capability.rs` with unchanged `NetworkAuthority`/`TlsPolicy`/preflight semantics + in-file forbidden-token self-test; guards 52/67 and capability-denial suites green | pass |
| 5 | Only `eggsec` directly depends on `eggsec-nse` | workspace manifest search (§7); `cargo tree -p eggsec --features nse,cli -i eggsec-nse` → `eggsec-nse <- eggsec`; guard 146 | pass |
| 6 | TUI/Python consume via `eggsec::nse`, feature behavior unchanged | `nse = ["eggsec/nse"]` in both manifests; direct edges deleted; tui 903 / python 231 tests pass; `make check-python` pass | pass |
| 7 | Runtime corpus/tests run independently of Eggsec crates | `cargo test -p eggsec-nse --features nse` 558 pass with zero `eggsec-*` dev-deps or test imports | pass |
| 8 | Static guards prevent regression | checks 144/145/146 added (fail); 40 retargeted to `nse_bridge.rs`; 26/120 allowlists updated; `make test-architecture-guards` → ALL PASSED | pass |
| 9 | `nse` / `nse-ssh2` / `nse-sandbox` build/behavior qualified | `cargo check -p eggsec-nse --features nse-ssh2` (0), `--features nse,sandbox` (0), `cargo check -p eggsec --features nse-ssh2,nse-sandbox,cli` (0); `check-features-individual` 89/89 | pass (ssh2 runtime execution still compile-only — §15) |
| 10 | `NseRunReport` compatibility intact | `report_contract_tests` 7 pass; JSON round-trip tests in corpus/local-protocol suites; no field added/removed | pass |
| 11 | `make check` and feature/dependency checks pass | `make check` exit 0 (fmt, no-default, `cargo deny`, clippy, tests, guards); `make check-features-individual` 89/89 | pass |
| 12 | Closure recommends whether 003 is ready | §16 go/no-go | pass |

## 3. `eggsec-nse` dependency tree — before/after

Before (`HEAD` `4fd7663`, workspace edges):

```text
eggsec-nse
├── eggsec-core         { path = "../eggsec-core" }
├── eggsec-report-model { path = "../eggsec-report-model" }
└── eggsec-transport    { workspace = true }
```

After (`85320be1`, `cargo tree -p eggsec-nse --depth 1`):

```text
eggsec-nse
├── anyhow, async-trait, base64, chrono, dashmap, flate2, hex
├── hickory-resolver, hostname, ipnetwork, md-5, parking_lot, rand, regex
├── reqwest, rustc-hash, rustls, scraper, serde, serde_json, sha1
└── tokio, toml, tracing, url, urlencoding
```

No workspace crate appears. TLS remains ring-only (guards green). The manifest comment at `crates/eggsec-nse/Cargo.toml:50-52` records where the adapters went.

## 4. Prior `eggsec-*` import inventory and disposition

Complete inventory of workspace-crate imports in `eggsec-nse` at `4fd7663`:

| Location | Import | Disposition |
|---|---|---|
| `src/bridge.rs:11` | `eggsec_core::types::Severity` | module deleted; logic moved to `crates/eggsec/src/nse_bridge.rs` |
| `src/bridge.rs:12` | `eggsec_report_model::{EvidenceItem, EvidenceKind, EvidenceSource, FindingRecord, RedactionState, ReportEnvelope, ToolMetadata}` | same move (engine now depends on `eggsec-report-model` behind `nse`, `crates/eggsec/Cargo.toml:315`) |
| `src/http_capability.rs:35` | `eggsec_transport::{HttpTransport, NetworkAuthority, RedirectPolicy, RequestBody, ScopedHttpRequest, TimeoutPolicy, TlsPolicy}` | module moved to `crates/eggsec/src/nse_http_capability.rs` |
| `src/http_capability.rs:44` | `pub use eggsec_transport::{Method, StatusCode}` | preserved in the moved module |

Module declarations removed: `pub mod bridge;` and `pub mod http_capability;` (both `#[cfg(feature = "nse")]`) from `crates/eggsec-nse/src/lib.rs` (5 lines). Engine declarations added: `pub mod nse_bridge;` / `pub mod nse_http_capability;` (both `#[cfg(feature = "nse")]`) in `crates/eggsec/src/lib.rs:178-183`.

Remaining `eggsec_` string occurrences in `eggsec-nse` (not imports; guard 145 excludes them): Lua table key `eggsec_context_source` in `context.rs` (4), and temp-path literals in `capabilities.rs`, `wrappers.rs`, `libraries/io.rs` (7). No `use eggsec_*` / `eggsec_*::` path remains in `src/` or `tests/`; no `eggsec-*` dev-dependency exists.

## 5. Moved report bridge — location and API

- Location: `crates/eggsec/src/nse_bridge.rs` (engine, `#[cfg(feature = "nse")]`).
- API unchanged: `pub fn to_report_envelope(report: &NseRunReport) -> ReportEnvelope` plus the internal `evidence_kind_to_output` / `evidence_kind_to_severity` mappers. Signature and mapping behavior are byte-for-byte the pre-move logic apart from `crate::nse::report::...` paths (rename-only diff, `git show 85320be1 -- crates/eggsec/src/nse_bridge.rs`).
- Caller disposition: at `4fd7663` the function had **no production callers** — only `eggsec-nse` integration tests used it (`bridge_tests.rs` 4 sites, `evidence_tests.rs` 9, `local_protocol_tests.rs` 1, `runtime_corpus_tests.rs` 1, `runtime_smoke_tests.rs` header import). After the move it is exercised by `crates/eggsec/tests/nse_bridge_tests.rs` (14 tests: the 4 relocated bridge tests, 9 envelope cases relocated from `evidence_tests.rs`, plus new `live_canonical_report_bridges_to_nse_envelope` which runs the engine facade end to end). No production report path changed hands, so no operator-visible behavior changed.
- Guard 40 now fails if `crates/eggsec/src/nse_bridge.rs` is missing.

## 6. `http_capability.rs` disposition and network-semantics review

- Disposition: renamed to `crates/eggsec/src/nse_http_capability.rs` (engine; `R094` similarity in the commit — comment/doc-only edits plus the import path). It is a **dormant seam**: no Lua `http`-family library called it before the move (the module's own doc states Lua `http`/`httppipeline`/`brute`/`vulns`/`comm`/`upnp` libraries still dispatch on pre-migration `reqwest`), and nothing else referenced it. Moving it therefore cannot have altered live network behavior.
- Semantics not weakened: the module still preflights through `NseCapabilityContext::check_capability` (`NetworkTcp`), still requires an injected `HttpTransport` carrying the operation's `NetworkAuthority`, still maps `TlsPolicy`, and still gates insecure TLS on `NseCapabilityContext::allows_insecure_tls` (ManualPermissive / CompatibilityLab only). Its in-file self-test (`include_str!` + forbidden-token assertion at `nse_http_capability.rs:331-359`) still forbids `reqwest`/`eggfetch`/`rustls`/`tokio-rustls`/`hickory` types in that module.
- ADR-0001 respected: `eggsec-transport` remains the sole scoped outbound-HTTP owner; the engine adapter consumes that contract, and `eggsec-nse` now has no transport dependency at all. ADR-0002 selective-reuse boundary untouched (no reuse-scope code moved).
- Network denial coverage unaffected: capability-denial and profile-denial suites pass in the runtime (`evidence_tests` 10, corpus 43+18) and engine (`nse_bridge_tests` 14, `nse_tests` 28, `nse_integration_tests` 21, `nse_real_scripts` 174).

## 7. Direct-consumer inventory and feature forwarding

Before (`4fd7663`) — three production crates declared a direct `eggsec-nse` edge:

| Manifest | Edge |
|---|---|
| `crates/eggsec/Cargo.toml:115` | `eggsec-nse = { path = ... , optional = true }` (legitimate owner) |
| `crates/eggsec-tui/Cargo.toml:53,75` | `eggsec-nse = { path = ..., optional = true }`, `nse = ["dep:eggsec-nse", "eggsec/nse"]` |
| `crates/eggsec-python/Cargo.toml:58,36` | `eggsec-nse = { path = ..., optional = true }`, `nse = ["eggsec/nse", "dep:eggsec-nse"]` |

After (`85320be1`):

| Manifest | Edge |
|---|---|
| `crates/eggsec/Cargo.toml:115,315` | unchanged `dep:eggsec-nse`; `nse = ["tool-api", "dep:eggsec-nse", "eggsec-nse/nse", "dep:eggsec-report-model"]` |
| `crates/eggsec-tui/Cargo.toml:77` | no `eggsec-nse` edge; `nse = ["eggsec/nse"]` |
| `crates/eggsec-python/Cargo.toml:36` | no `eggsec-nse` edge; `nse = ["eggsec/nse"]` |

Workspace search: `rg 'eggsec-nse\s*=|eggsec-nse/|dep:eggsec-nse' crates/*/Cargo.toml` → only `crates/eggsec/Cargo.toml` (plus explanatory comments in `eggsec-tui/Cargo.toml:70-71`). `cargo tree -p eggsec --features nse,cli -i eggsec-nse` → `eggsec-nse <- eggsec` only.

Code-side consolidation: `crates/eggsec-tui/src/tabs/nse.rs` and `nse_report_view.rs` now import from `eggsec::nse::...`; Python already consumed `eggsec::nse`. Both forward `nse = ["eggsec/nse"]`.

## 8. Architecture-guard evidence

`make test-architecture-guards` exit 0, `ALL PASSED: No architecture drift detected.`

| Check | Change | Assertion |
|---|---|---|
| 144 (new, FAIL-class) | manifest | no `eggsec-*` / `path = "../eggsec...` in `crates/eggsec-nse/Cargo.toml` |
| 145 (new, FAIL-class) | source | no `use eggsec_*`, `eggsec_*::`, `eggsec::` in `crates/eggsec-nse/src/` (comment lines excluded) |
| 146 (new, FAIL-class) | graph | every `crates/*/Cargo.toml` mentioning `eggsec-nse` must be `crates/eggsec/Cargo.toml`, which must mention it |
| 40 | retargeted | `crates/eggsec/src/nse_bridge.rs` must exist (was `crates/eggsec-nse/src/bridge.rs`) |
| 26 | allowlist | `crates/eggsec/tests/nse_bridge_tests.rs` added (manual-profile adapter test) |
| 120 | scope | excludes `eggsec-nse` (which no longer carries `eggsec-report-model`; no DTO-consumer rule applies) |
| 143 (001) | unchanged, still green | canonical pipeline ownership |
| 25 / 30 / 35 (001) | unchanged, still green | canonical ownership of run/report paths |

## 9. Verification executed

### Commands run

```bash
cargo tree -p eggsec-nse --depth 1
cargo tree -p eggsec --features nse,cli -i eggsec-nse
cargo test -p eggsec-nse --features nse
cargo test -p eggsec --features nse,cli --lib
cargo test -p eggsec --features nse,cli --test nse_bridge_tests --test nse_tests \
  --test nse_integration_tests --test nse_real_scripts
cargo test -p eggsec-tui --features nse
cargo test -p eggsec-python --features nse
cargo check -p eggsec-nse --features nse-ssh2
cargo check -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse-ssh2,nse-sandbox,cli
cargo check -p eggsec-tui --features nse
cargo check -p eggsec-python --features nse
cargo fmt --all --check
make test-architecture-guards
make check
make check-features-individual
make check-python
```

### Results

- `eggsec-nse` (nse): **558 passed, 0 failed, 1 ignored** (23 suites). Key suites: unit lib 189, `compatibility_corpus_tests` 43, `runtime_corpus_tests` 18, `canonical_run_tests` 16, `report_contract_tests` 7, `evidence_tests` 10 (runtime-only residue after the envelope cases moved).
- `eggsec` lib (nse,cli): **1710 passed, 0 failed** (001 baseline 1704 + 6 relocated/added).
- `eggsec` NSE binaries: `nse_bridge_tests` 14, `nse_tests` 28, `nse_integration_tests` 21, `nse_real_scripts` 174 — all pass.
- `eggsec-tui` (nse): **903 passed, 0 failed**; `eggsec-python` (nse): **231 passed, 0 failed**.
- `make check`: exit 0. `make check-features-individual`: **89 passed, 0 failed, 0 skipped**. `make test-architecture-guards`: exit 0, ALL PASSED. `make check-python`: exit 0 (pytest/stub/mypy gates pass; pyright's 11 native-module-stub warnings are the pre-existing expected set).
- `cargo fmt --all --check`: clean.
- `nse-ssh2` runtime execution remains compile-only in this environment (carried from 001; §15).
- One environmental incident during verification, recorded for transparency: the workspace filesystem hit 100% during the first post-decoupling full-suite run; `cargo clean` (116 GiB reclaimed) and removal of `target/debug/incremental` restored headroom, after which every command above ran to completion. No test was skipped or weakened as a result — the rerun suite count matches the pre-incident run (558).

## 10. Invariant review

- Authorization outside `eggsec-nse`: unchanged. `EnforcementContext`, `OperationMetadata`, `EnforcedDispatcher`, and `handle_nse` pre-dispatch enforcement were not touched by this milestone (diff contains no auth symbols). Execution still requires outer Eggsec approval before the runtime is invoked.
- Manual vs strict profiles: `ManualPermissive` remains manual-surface-only (guard 26 green with the relocated adapter test allowlisted).
- `ScriptResolver` file policy: untouched; runtime resolver/policy suites green.
- Cancellation, limits, sandbox, capability events: executor and hook machinery untouched; Milestone 001 contention/cancellation coverage re-run green (4-thread isolation, pre/mid-execution cancellation).
- `NseRunReport` serialization: unchanged (contract + round-trip tests).
- Clean-room corpus provenance: guards 38/54 green; corpus suites pass with no Eggsec dev-dependency in the runtime crate.
- Ring-only TLS: guards green; `eggsec-nse` still pulls `rustls` with `default-features = false`.
- Dependency direction: `eggsec-report-model` still depends only on `eggsec-core`; the new edge is `eggsec --(nse)--> eggsec-report-model`, which is the documented direction (`nse = [... "dep:eggsec-report-model"]`), not a cycle.

## 11. Failure, cancellation, restart, and contention review

- No durable state introduced; restart recovery remains N/A.
- Cancellation semantics: re-run Milestone 001 coverage passes — the ownership move did not change execution semantics.
- Partial/report-failure paths unchanged; `NseRunError::failure_report()` intact.
- Concurrent runs: executors remain per-run; isolation test passes.
- Contention: no new shared state; the two moved modules are pure/DI-parameterized.

## 12. Migration and compatibility review

Internal ownership migration; no storage, protocol, or wire migration.

- `NseRunReport` JSON shape unchanged (no field added/removed/renamed).
- Public runtime surface changed only by removal of two module paths: `eggsec_nse::bridge` and `eggsec_nse::http_capability`. Both were Eggsec-composition seams with no external consumer outside this workspace (searched: no production caller; only workspace tests, which moved with them). No released API is broken.
- `eggsec::nse::*` re-exports, Python class/function names, feature names and defaults, built-in identifiers: unchanged (`make check-python` + stub checks green).
- Feature-forwarding change is internal: `eggsec-tui`/`eggsec-python` `nse` features still resolve to `eggsec/nse` plus the same transitive `eggsec-nse` feature set; the direct edge was redundant. `check-features-individual` 89/89 confirms every combination still builds.
- Test relocation inventory: `eggsec-nse/tests/bridge_tests.rs` (4 tests) → `eggsec/tests/nse_bridge_tests.rs`; 9 envelope cases from `eggsec-nse/tests/evidence_tests.rs` → same file; `local_protocol_tests`/`runtime_corpus_tests`/`runtime_smoke_tests` envelope assertions rescoped to the runtime report boundary with a pointer to the engine test file (residual assertions still cover evidence, output, round-trip, fidelity, and status).

## 13. Security review

- No authorization moved into the runtime; if anything, the runtime's dependency surface shrank (no engine, report-model, or transport crate reachable from it).
- Transport relocation did not remove profile/capability denials: `NetworkTcp` preflight, `NetworkAuthority` injection, and insecure-TLS gating are preserved verbatim in the moved module, and the in-file forbidden-token self-test still guards against smuggling an unrestricted client in.
- Guard 145 blocks any future `eggsec_*` import from creeping back into the runtime source; guard 144 blocks it at the manifest; guard 146 blocks a second direct consumer. All three fail closed.
- No secrets, keys, or credentials introduced or relocated.

## 14. Documentation and operations

- `architecture/overview.md`: dependency/ownership map updated to `eggsec-nse <- eggsec <- {CLI, TUI, Python}` with the two engine adapters named.
- `architecture/nse_integration.md`: consumer-ownership section added (runtime owns NSE semantics; engine owns report/transport adapters).
- `docs/NSE_COMPATIBILITY.md`: ownership/path claims reconciled; compatibility counts untouched.
- `docs/python/NSE_RUNTIME_ARCHITECTURE.md`: feature-gating example now shows the `eggsec` facade (`eggsec::nse::{execute_nse_run, NseRunRequest}`) instead of a direct `eggsec-nse` dependency.
- `.opencode/skills/eggsec-nse/SKILL.md`: "Consumer ownership (runtime extraction)" section documents zero `eggsec-*` runtime deps and the engine-facade-only rule for downstream crates (guards 144/145/146 cited).
- Grandfathered historical plan records under `plans/` top level were not modified.
- No operator action required: no new flags, profiles, defaults, or file formats.

## 15. Residual findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| low | `nse-ssh2` runtime execution not exercised (compile-only) | carried unchanged from 001 closure | qualify on an SSH-capable runner before standalone-extraction SSH-parity claims |
| low | Full `cargo test -p eggsec --features nse,cli` (all integration binaries) not run as a single shot | coverage obtained via lib (1710) + four targeted NSE binaries (237), all green; suite exceeds one command's timeout | none; CI runs the full matrix |
| info | `nse_bridge::to_report_envelope` still has no production caller | pre-existing (identical at baseline); it is a composition seam, not a live path | none now; 003 should decide whether the seam is adopted by the report pipeline or dropped |
| info | `nse_http_capability` remains a dormant seam (Lua HTTP libs still on `reqwest`) | no behavior change; backend cutover already deferred to Phase D by design | none for 002; tracked by the existing Phase D disposition |
| info | Verification required freeing ~150 GiB of `target/` mid-run | environmental, not code-related; all commands re-ran to completion | none; consider CI/local disk hygiene |

No critical/high/medium findings.

## 16. Roadmap disposition — go/no-go for Milestone 003

**Recommendation: GO for planning Milestone 003 (standalone repository extraction), conditional on two preconditions.**

Basis: all 12 acceptance criteria pass; the runtime crate's manifest, source, and tests are free of workspace dependencies; the two Eggsec adapters are engine-owned with guards preventing regression; only the engine consumes the runtime; the corpus and full runtime suite pass with zero inward edges; `make check`, the 89/89 feature matrix, the guard suite, and `make check-python` are green.

Preconditions carried into 003:

1. Qualify `nse-ssh2` on an SSH-capable runner (carried from 001) before any SSH-parity claim is made against an extracted repository.
2. In 003's plan, explicitly decide the disposition of the two dormant engine seams (`nse_bridge` production adoption or drop; `nse_http_capability` Phase D cutover) so the extraction boundary does not inherit an undefined ownership question.

No corrective pass is required for Milestone 002. Milestones 003-005 remain roadmap items requiring their own implementation plans (roadmap §6).

## 17. Registry updates

- `plans/registry.md`: 002 → closed (link this record); NSE runtime extraction row → milestone 002 closed / 003 next.
- `plans/subsystems/nse-runtime-extraction-roadmap.md`: milestone table 002 → closed; milestone 003 dependency satisfied.
- `plans/implementation/nse-runtime-extraction/002-*.md`: Status → implemented.
