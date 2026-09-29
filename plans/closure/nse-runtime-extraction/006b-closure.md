# NSE Runtime Extraction Milestone 006B — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/nse-runtime-extraction/006-eggsec-0-2-0-adoption-safe-staging.md`

Source subsystem roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-006--020-release-and-safe-eggsec-adoption`

Repository baseline reviewed: Eggsec `d9f98ccf` (M006A closure commit).

Published 0.2.0 identity from 006A (`plans/closure/nse-runtime-extraction/006a-closure.md`):

- `eggsec-nse 0.2.0` on crates.io from standalone source `ff0d2c09feba1bcd7ba5f7312579690d905be09c` (tag `v0.2.0`, docs.rs built, archive VCS identity = candidate).

Implementation commits or pull requests:

- Eggsec main (this slice): dependency bump `0.1.0` → `0.2.0` + lockfile; replayed `crates/eggsec/src/nse_http_provider.rs` + `lib.rs` wiring; `OperationMetadata` NSE quarantine; fail-closed strict-execution guard in `execute_approved`/`execute_approved_execution`; quarantine regression tests (catalog, registration, canonical boundary); guard 144 version bump + guard 145 dormancy pin; doc/skill reconciliation.

## 1. Executive finding

Milestone 006B is complete. Eggsec consumes the crates.io `eggsec-nse 0.2.0` artifact with no Git/path/patch source; the staged `NseHttpTransportProvider` is replayed onto current main and tested against the real 0.2.0 contract with no production caller; automated NSE exposure is disabled at three independent layers (operation metadata, surface listings, strict execution boundary) while manual CLI/TUI NSE remains source-compatible. **GO for 006C** (cross-repo qualification). Automated NSE activation remains deferred to M007.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| Manifest + lockfile resolve crates.io 0.2.0 | `crates/eggsec/Cargo.toml` `version = "0.2.0"`; lockfile `0.2.0` + `registry+...crates.io-index` + checksum `bf8befe7…0ffe` (= published archive); `cargo tree -p eggsec --features nse,cli -i eggsec-nse` → `eggsec-nse v0.2.0` | pass | `cargo update -p eggsec-nse --precise 0.2.0` |
| No Git/path/patch runtime source | Guard 144 source regex green; `cargo tree` shows registry source only | pass | — |
| Guard 144 requires 0.2.0 registry package | `scripts/check-architecture-guards.sh` check 144 updated + green | pass | §4 |
| Adapter replayed onto current main, compiles against 0.2.0 | `crates/eggsec/src/nse_http_provider.rs` + `lib.rs` `pub mod nse_http_provider` (cfg `nse`); `cargo check -p eggsec --features nse,cli` ok; combo `nse-ssh2,nse-sandbox,cli` ok | pass | Stale branch NOT merged wholesale (§3) |
| Adapter authority/TLS/no-fallback tests pass | 8/8 `nse_http_provider` lib tests green against registry 0.2.0 | pass | In-scope/out-of-scope/deny-all/TLS non-escalation/invalid-URL/no-concrete-client |
| Adapter names no concrete HTTP client | In-module source-scan test green | pass | — |
| Adapter has no production caller | `rg NseHttpTransportProvider` → only the module itself; guard 145 dormancy pin green; zero `with_host_services`/`NseHostServices` use in engine production code | pass | §5 |
| Automated exposure disabled/fail-closed | Metadata flip (§5) + listing tests + `execute_approved*` guard + 2 boundary tests; `NseTool` remains unregistered; REST/gRPC explicit exposure checks + registry miss stand as further layers | pass | Enumeration truthful; synthesized bundles fail closed |
| Manual/TUI NSE functional | NSE suites 237 passed; manual CLI path (`evaluate` + `run_cli_with_profile`, bypasses strict entries) untouched; `run_nse` still native/default-provider | pass | §4 |
| Python behind facade, passing | 231 passed; no Python source change | pass | — |
| No authorization logic moves into standalone | Standalone untouched by this slice; adapter engine-owned (`crates/eggsec/src/`) | pass | Guard 145 ownership check green |
| Docs state dormant adapter + M007 prerequisite | Skill + `architecture/nse_integration.md` + `docs/NSE_COMPATIBILITY.md` updated; no activation/scope-threading claim | pass | §9 |
| Full relevant Eggsec checks pass | `make check` green; `make check-python` green; `make check-deps` green | pass | §4 |
| Closure unblocks 006C | This record + registry/roadmap updates | pass | §11 |

## 3. Production implementation evidence

- **Dependency adoption** (`crates/eggsec/Cargo.toml`, `Cargo.lock`): `eggsec-nse = { version = "0.2.0", optional = true }`; lockfile resolves the published 0.2.0 checksum. No `[patch.crates-io]`, Git, path, tag, branch, or rev in committed state.
- **Adapter replay** (2 files, matching the branch's logical change; stale branch history not merged):
  - `crates/eggsec/src/nse_http_provider.rs` (new, 440 lines): `NseHttpTransportProvider<T: HttpTransport>` over an injected `Arc<dyn NetworkAuthority>`; `req.insecure_tls` structurally ignored (construction-time profile decision only); error mapping (denied → `Denied`, timeout-message → `Timeout`); ambient/ephemeral runtime bridge; 8 tests. Diff vs `4ade61a` is documentation-only (provenance header + dormancy note); **zero logic changes** — the 0.2.0 DTO surface (`NseHttpProvider`, `NseHttpRequest` fields, `NseHttpError` variants, `body_text`) matched the staged code exactly.
  - `crates/eggsec/src/lib.rs`: `#[cfg(feature = "nse")] pub mod nse_http_provider;`.
- **Exposure quarantine** (`crates/eggsec-policy/src/catalog.rs`): NSE `mcp/rest/agent/grpc_exposable` → `false` (manual/tui stay `true`; `TargetPolicyKind::ExplicitScopeRequired` untouched). This follows the 5 existing manual-only precedents (wireless-deauth, mobile-*, evasion, postex).
- **Strict-execution guard** (`crates/eggsec/src/dispatch/canonical_execution.rs`): `execute_approved` and `execute_approved_execution` fail closed with `InvalidRequest` ("not exposed on automated surfaces … pending protocol-gating/scope-threading (M007 for NSE)") for operations where `!metadata.is_exposed_automated()`. Data-driven from `OperationMetadata` (no hardcoded op id). Manual CLI (`evaluate` + `run_cli_with_profile`) and TUI (`TuiTaskDispatcher → execute_canonical`) do not enter through these entries and are unaffected.
- **Guards** (`scripts/check-architecture-guards.sh`): check 144 now requires the 0.2.0 registry package; check 145 additionally pins adapter presence (`nse_http_provider.rs`) and dormancy (no `NseHttpTransportProvider` constructor outside the module).
- Zero production dispatch rewiring; no new runtime profiles; no protocol-gating work entered this slice.

## 4. Verification executed

### Commands run

```bash
cargo tree -p eggsec --features nse,cli -i eggsec-nse
make check-deps
make test-architecture-guards   # via make check
cargo check -p eggsec --features nse-ssh2,nse-sandbox,cli
cargo test -p eggsec --features nse,cli --lib
cargo test -p eggsec --features nse,cli --test nse_bridge_tests --test nse_integration_tests --test nse_real_scripts --test nse_tests
cargo test -p eggsec-tui --features nse
cargo test -p eggsec-python --features nse
make check
make check-python
```

### Results

- `cargo tree`: `eggsec-nse v0.2.0` under `eggsec` (registry source).
- `make check-deps`: ok (licenses/sources/bans/advisories).
- Architecture guards: ALL PASSED (checks 144/145/146 green).
- Combo check (`nse-ssh2,nse-sandbox,cli`): ok.
- Lib (`nse,cli`): **1721 passed, 0 failed** (= 1710 baseline + 8 adapter + 2 boundary + 1 listing).
- NSE suites: **237 passed, 0 failed** (unchanged behavior).
- TUI (`nse`): **903 passed, 12 ignored** (unchanged).
- Python (`nse`): **231 passed** (unchanged).
- `make check`: green (fmt, no-default checks, check-deps, clippy, tests, guards).
- `make check-python`: PASSED.
- Not run (006C scope): `make check-features-individual` full sweep, docs.rs reconfirmation, tag/archive re-verification.

## 5. Invariant review

- Only `eggsec` directly consumes the runtime (guard 145 green); TUI/Python forward through `eggsec::nse` (facade re-export intact).
- Registry-only source (guard 144 green).
- Adapter engine-owned; standalone untouched and Eggsec-independent.
- No automated/strict path executes NSE with `ManualPermissive`: manual CLI still constructs `ManualPermissive` itself, but automated surfaces cannot reach execution — enumeration omits NSE (metadata-driven listings), `NseTool` is unregistered ("defined but not registered… Manual-only"), REST/gRPC explicit exposure checks stand, and synthesized `ApprovedExecution` bundles fail closed at both strict entries.
- Adapter never manufactures authority: constructor signature unchanged (`Arc<dyn NetworkAuthority>` injected); out-of-scope fails closed inside transport (tests).
- Script TLS intent cannot escalate: `script_tls_intent_never_escalates` green.
- 0.2.0 send/write accounting accepted as-is; `register_vulns_library` break absorbed via dependency update only (no Eggsec call-site change required — engine does not call it).
- `TargetPolicyKind::ExplicitScopeRequired` preserved for NSE (catalog test pins it).

## 6. Failure and recovery review

- Registry adoption needed no source override (0.2.0 API matched staged code exactly) — the plan's primary failure branch did not trigger.
- No authority-from-URL construction was needed during replay — second failure branch did not trigger.
- Quarantine required no broad API churn: metadata flip + 30-line guard + tests — third failure branch did not trigger.
- One test-design correction during implementation: the initial listing test asserted `resolve_tool_registration("nse").is_none()`, but `all_tool_registrations()` is metadata-derived by design (all 34 ops resolve); `resolve_tool_registration` is diagnostic-only on the REST path (real dispatch goes through the ToolRegistry, where NSE is absent). Replaced with per-flag quarantine assertions on the derived registration — stronger and truthful.
- One approval-shape correction: NSE manual approval needs the `NonBaselineCapability` override (same pattern as the load-test regression test); the execution boundary — not approval — is what the tests prove rejects.
- Rollback: prior Eggsec commit consuming crates.io 0.1.0. No yank consideration (consumer-side only).

## 7. Migration and compatibility review

- Additive only: one new engine module + metadata flag changes + guard. No public API removal; no report-schema change; manual NSE request/response shapes unchanged.
- The metadata flip is intentionally visible: MCP/REST/gRPC/agent listings no longer offer NSE (previously advertised but undispatched — the tool was never registered, so this removes a false advertisement, not a working path).
- The general `!is_exposed_automated()` execution guard also hardens the 5 pre-existing manual-only ops on strict entries; no existing test strict-dispatches them (full suite green).
- The newly live 0.2.0 write-limit enforcement (`max_network_bytes_written`) flows to manual NSE runs via the runtime default; operator impact documented in the 006A changelog.

## 8. Security review

- Threat addressed: automated NSE execution before protocol-wide capability gating (M005E 72-file ungated residual). Closed at enumeration (metadata/listings), approval-adjacent (tool unregistered; REST/gRPC explicit checks), and execution (strict-entry guard) layers.
- No `ManualPermissive` reaches any automated path; the adapter cannot be constructed without an existing authority; no authority fabrication; TLS non-escalation tested.
- No secrets handled; loopback-only test fixtures; `tracing` used (no `let _ =` introduced; fmt/clippy green).
- Residual risk unchanged from M005E §10 (medium ungated set, still manual-only bounded) — now additionally quarantined at the Eggsec boundary.

## 9. Documentation and operations

- `.opencode/skills/eggsec-nse/SKILL.md`: 0.2.0 consumed; `nse_http_provider` dormant; quarantine + M007 ownership; no-enforcement-overclaim note.
- `architecture/nse_integration.md`: M006B adoption note (0.2.0, replay provenance, dormancy, quarantine, M007 sequencing).
- `docs/NSE_COMPATIBILITY.md`: http-row qualifier updated (replayed-dormant, quarantined pending M007).
- Guards 144/145 updated (§3). Historical M005 closures untouched.
- Operator impact: none (no config/CLI/TUI/Python surface change). `eggsec doctor`/platform scripts unaffected.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| medium (carried) | M005E 72-file ungated specialized residual | Bounds automated NSE risk; quarantine holds it manual-only | M007 protocol-library capability gating + scope threading + controlled activation |
| low | `approve()` (strict) can still issue an NSE token; only execution/listings block it | Token alone executes nothing; all dispatch layers fail closed | M007 may add surface-exposure to approval; explicitly out of M006 scope (no broad metadata redesign) |
| low | Trusted Publishing registry-side entry still pending (from 006A) | Manual-token path remains the release route | Crate-owner website step; no code change |

No high/critical findings. No new medium findings beyond the carried residual.

## 11. Roadmap disposition

**Milestone 006B closed; 006C may proceed (GO).** Next: cross-repository qualification per `006-cross-repo-qualification-closure.md` (release/tag/archive identity, registry-only consumption, dormant-adapter source truth, quarantine proof, full consumer checks incl. `make check-features-individual`, doc/guard reconciliation, M006 closure + M007 sequencing).

## 12. Registry updates

- `plans/registry.md`: 006B closed; 006C becomes dependency-ready (M006A+M006B closed).
- `plans/subsystems/nse-runtime-extraction-roadmap.md`: M006B closed; M006C ready.
