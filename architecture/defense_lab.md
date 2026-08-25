# Defense-Lab and Regression Validation Architecture

## Purpose

Defense-lab mode provides local, controlled testing against Synvoid-like defensive systems. It enables:

- **Repeatable adversarial traffic generation** against a known target
- **WAF and protocol behavior regression validation** after configuration changes
- **Controlled defense validation**, not public-target stress or exploitation

Defense-lab mode is distinct from general assessment mode. It assumes a local or private-lab environment where you control both the target and the traffic.

## Core Workflow

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│ Build/Run    │────▶│ Run Eggsec  │────▶│ Collect      │
│ Synvoid      │     │ Profile      │     │ Observations │
└──────────────┘     └──────────────┘     └──────────────┘
                                                  │
                            ┌─────────────────────┘
                            ▼
                     ┌──────────────┐     ┌──────────────┐
                     │ Collect      │────▶│ Compare      │
                     │ Synvoid Logs │     │ Baseline     │
                     └──────────────┘     └──────────────┘
                                                  │
                            ┌─────────────────────┘
                            ▼
                     ┌──────────────┐
                     │ Convert      │
                     │ Regressions  │
                     │ to Tests     │
                     └──────────────┘
```

1. **Build and run Synvoid** (or similar) locally or in a controlled lab
2. **Run a Eggsec defense-lab profile** against the target
3. **Collect Eggsec observations** (responses, latencies, error classes)
4. **Optionally collect Synvoid logs/metrics** (WAF decisions, rule matches, resource usage)
5. **Compare against baseline** to identify changes or regressions
6. **Convert regressions into test cases** for CI or future runs

## Category Taxonomy

The `DomainCategory` enum (`crates/eggsec/src/domain/mod.rs:27-38`) classifies domains into five categories:

| Category | Label | Semantics |
|----------|-------|-----------|
| `StandardAssessment` | "standard assessment" | Scoped recon, scanning, fuzzing, API testing, WAF detection, and reporting |
| `DefenseLab` | "defense lab" | Local/private defense validation and regression testing |
| `HazardousLab` | "hazardous lab" | High-risk operations requiring explicit authorization |
| `FrontendAdapter` | "frontend adapter" | Bridges external protocols (REST, MCP, gRPC) |
| `OutputAdapter` | "output adapter" | Produces output formats (reports, exports) |

### DefenseLab vs. HazardousLab Semantics

`DefenseLab` (`OperationMode::DefenseLab`, `policy.rs:184`) provides a local/private/scope-constrained environment for WAF and distributed-system validation. Its `default_max_risk()` is `OperationRisk::Intrusive` (`policy.rs:205`), meaning Intrusive operations are allowed by default without explicit policy approval.

`HazardousLab` (`OperationMode::HazardousLab`, `policy.rs:188`) covers raw packet operations, flood-style stress tests, proxy rotation, low-level protocol edge cases, and other aggressive tests. Its `default_max_risk()` is `OperationRisk::AgentAutonomous` (`policy.rs:206`), the highest risk tier.

Both modes require explicit scope and are distinct from `StandardAssessment` (which defaults to `SafeActive`). The enforcement engine (`policy_decision.rs:1171`) treats them differently: DefenseLab operations can proceed with `Intrusive` risk under default policy, while HazardousLab operations require explicit `allow_*` flags in `ExecutionPolicy`.

## Lab-Gated Module Inventory

| Module | Gate Feature | Operation Mode | Risk Tier | Dry-Run? | Confirmation Required? |
|--------|-------------|----------------|-----------|----------|----------------------|
| db-pentest | `db-pentest` | DefenseLab | DbPentest (real) / SafeActive (dry) | Always safe | `--allow-db-pentest` for real |
| wireless-active | `wireless-advanced` | DefenseLab | Intrusive (live) / SafeActive (dry) | `-d` flag | `--allow-wireless-advanced` |
| mobile-dynamic | `mobile-dynamic` | DefenseLab | Intrusive (real) / SafeActive (dry) | Always safe | `--allow-frida` for Frida |
| auth-test | (always compiled) | StandardAssessment | CredentialTesting | N/A (no dry-run mode) | No |
| web-proxy | `web-proxy` | StandardAssessment | TrafficInterception (real) / SafeActive (dry) | Always safe | `--allow-web-proxy` for real |
| evasion | `evasion` | DefenseLab | EvasionTesting | Dry-run-only handler | `--allow-evasion-testing` |
| postex | `postex` | DefenseLab | PostExploitation | Dry-run-only handler | `--allow-postex` |
| c2 | `c2` | StandardAssessment | C2Operation | Dry-run-only handler | `--allow-c2` |
| stress | `stress-testing` | HazardousLab | StressTest | No | `--allow-stress-testing` |
| packet-inspection | `packet-inspection` | HazardousLab | RawPacket | No | `--allow-raw-packets` |

### Module Details

- **db-pentest**: Direct database security checks (Postgres/MySQL/MSSQL/MongoDB/Redis). Phase 1-6 delivered. TUI tab `Tab::DbPentest` + native `Stage::DbPentest` pipeline. Advanced gated checks behind `--allow-db-pentest-advanced`. Cross-DB correlation, compliance mapping, optional MCP via `db-pentest-mcp`. See `architecture/database_pentest.md`.
- **wireless-active**: Active deauth/disassoc frame injection (lab-only). Risk `Intrusive` (live) / `SafeActive` (dry-run via `-d`). No MCP/agent exposure (design decision).
- **mobile-dynamic**: Android runtime testing via ADB, logcat, proxy, and Frida. Phase 2a (proxy + permissions) + Phase 3 (Frida) delivered. Standalone defense-lab (no pipeline/TUI/MCP).
- **auth-test**: Credential control validation (brute, lockout, MFA, timing). Local `Auth*` types only (no bridge). Risk `CredentialTesting`. Distinct from pipeline `ScanProfile::Auth`.
- **web-proxy**: Interactive MITM proxy for HTTP/HTTPS traffic interception. Phase 1 delivers dry-run mode with complete `WebProxySessionReport`. Standalone defense-lab (no MCP/agent/TUI/pipeline).
- **evasion**: Evasion technique detection for defense validation. 16 techniques across 6 categories. Feature-gated: `evasion`. Dry-run-only handler behavior.
- **postex**: Post-exploitation and LOTL simulation. Feature-gated: `postex`. Dry-run-only handler behavior.
- **c2**: C2 simulation (beaconing, tasking, campaign orchestration). Feature-gated: `c2`. Depends on postex+evasion. Dry-run-only handler behavior.
- **stress**: Network stress testing (SYN/UDP/HTTP/TCP/ICMP floods, IP spoofing). Feature-gated: `stress-testing`. HazardousLab mode.
- **packet-inspection**: Packet capture, crafting, parsing. Feature-gated: `packet-inspection`. HazardousLab mode.

## Probe Categories

Defense-lab profiles target these categories:

| Category | Description |
|----------|-------------|
| **TCP/IP stack behavior** | SYN/ACK patterns, window sizes, TTL handling, RST behavior |
| **Malformed packets** | Oversized headers, invalid chunked encoding, broken HTTP framing |
| **TLS/client fingerprints** | JA3/JA4 variants, cipher suite ordering, SNI manipulation |
| **HTTP ambiguity** | Request smuggling, transfer-encoding variants, host header quirks |
| **WAF payload classification** | Evasion pattern detection, encoding bypass, case manipulation |
| **Bot-like request patterns** | User-agent spoofing, header ordering, timing analysis |
| **Rate-limit/tarpit behavior** | Rate detection, slowloris patterns, connection exhaustion |
| **Load-bearing validation** | Concurrency scaling, connection pool behavior, timeout thresholds |

## Preset System

`DefenseLabPreset` (`crates/eggsec/src/config/presets.rs:7`) defines constraints for specific lab workflows. 7 built-in presets:

| Preset | Mode | Max Risk | Intended Uses | Concurrency | Max Duration | Max Requests | Raw Sockets |
|--------|------|----------|---------------|-------------|-------------|-------------|-------------|
| `synvoid-local` | DefenseLab | Intrusive | SynvoidRegression, WafRegression | 10 | 300s | 10,000 | No |
| `synvoid-waf-regression` | DefenseLab | Intrusive | WafRegression | 20 | 600s | 50,000 | No |
| `synvoid-protocol-edge` | DefenseLab | SafeActive | ProtocolEdgeValidation | 5 | 120s | 1,000 | Yes |
| `distributed-system-smoke` | DefenseLab | LoadTest | DistributedSystemStress | 5 | 60s | 500 | No |
| `distributed-system-stress` | HazardousLab | StressTest | DistributedSystemStress | 50 | 300s | 100,000 | Yes |
| `waf-regression-safe` | DefenseLab | SafeActive | WafRegression | 5 | 120s | 5,000 | No |
| `waf-regression-intrusive` | DefenseLab | Intrusive | WafRegression | 20 | 600s | 50,000 | No |

All presets enforce `localhost_or_private_required: true` (verified by test at `presets.rs:216`). `distributed-system-stress` is the only HazardousLab preset; it enables `raw_sockets_allowed: true` and `dns_resolution_allowed: true`.

### GeneratedPreset ScopeSource

`ScopeSource::GeneratedPreset` (`config/scope.rs:209`) is one of four scope provenance values. When a scope is generated from a preset, it carries this source tag, enabling strict execution profiles to distinguish it from user-provided config or CLI scope files. `LoadedScope::is_explicit_manifest()` (`scope.rs:226`) returns `true` for `GeneratedPreset`, satisfying the explicit-manifest requirement for automated surfaces.

## Approval / Confirmation Flow

### ConfirmationClass

`ConfirmationClass` (`config/policy_decision.rs:402`) defines 8 categories of conditions that trigger `RequireConfirmation` under `ManualPermissive`:

| Class | CLI Flag | Semantics |
|-------|----------|-----------|
| `OutOfScope` | `--allow-out-of-scope` | Target not in declared scope (positive rules present) |
| `ExplicitExclusion` | `--allow-excluded-target` | Target explicitly excluded from scope |
| `HighRisk` | `--allow-high-risk` | Intrusive/LoadTest/StressTest/RawPacket/CredentialTesting/DbPentest/ExploitAdjacent/RemoteExecution |
| `NonBaselineCapability` | `--allow-nonbaseline-capability` | Required capability not in baseline allowlist |
| `PrivateResolution` | `--allow-private-resolution` | Public input resolved to private/loopback (DNS rebinding signal) |
| `CrossHostRedirect` | `--allow-cross-host-redirect` | Redirect to different host detected |
| `TargetExpansion` | `--allow-out-of-scope` | New targets discovered outside original input |
| `TrafficInterception` | `--allow-web-proxy` | MITM proxy / traffic interception |

### How Lab Ops Map to ConfirmationClass

- **db-pentest** (real, `OperationRisk::DbPentest`): triggers `HighRisk` confirmation (`policy_decision.rs:1422`). `ManualOverride.allow_db_pentest` permits `HighRisk` (`policy_decision.rs:460`).
- **wireless-active** (live, `OperationRisk::Intrusive`): triggers `HighRisk`.
- **mobile-dynamic** (real + Frida, `OperationRisk::Intrusive`): triggers `HighRisk`.
- **web-proxy** (real interception, `OperationRisk::TrafficInterception`): triggers `TrafficInterception` confirmation.
- **evasion/postex/c2** (real execution): triggers `HighRisk` for their respective risk tiers.
- **stress** (HazardousLab, `OperationRisk::StressTest`): triggers `HighRisk`.

### ManualOverride

`ManualOverride` (`policy_decision.rs:433`) is honored only for `ExecutionProfile::ManualPermissive`. Key behaviors:

- `assume_yes` only permits `OutOfScope` and `TargetExpansion` (low-risk scope confirmations). It does NOT authorize high-risk, explicit exclusions, non-baseline capabilities, private-resolution, or cross-host redirects (`policy_decision.rs:449-452`).
- `allow_db_pentest` specifically permits `HighRisk` confirmation class (`policy_decision.rs:460`).
- `allow_web_proxy` specifically permits `TrafficInterception` (`policy_decision.rs:461`).
- Strict profiles (McpStrict, AgentStrict, CiStrict) never honor manual overrides — `RequireConfirmation` becomes a hard `Deny`.

### EnforcementOutcome Flow

`evaluate_enforcement()` (`policy_decision.rs:1171`) produces one of four outcomes:

1. **Allow**: operation proceeds (no warnings, no confirmations needed).
2. **Warn**: operation proceeds with recorded warnings (ManualPermissive only, for safe ambiguity cases).
3. **RequireConfirmation**: operator must provide matching `--allow-*` flags (ManualPermissive only). Automated profiles treat as `Deny`.
4. **Deny**: operation must not proceed (hard denial).

For DefenseLab operations, the flow is:
- `evaluate_operation_policy()` checks features, scope, risk against `ExecutionPolicy`.
- `evaluate_enforcement()` maps the decision to an outcome based on the `ExecutionProfile`.
- `confirmation_classes_for()` (`policy_decision.rs:1365`) determines which `ConfirmationClass` values apply.
- `approve_manual()` or `approve()` produces an `ApprovedOperation` token if the outcome permits dispatch.

## Dry-Run Support Contract

Defense-lab modules implement dry-run with varying completeness:

| Module | Dry-Run Behavior | Zero Network? | Complete Report? |
|--------|-----------------|:------------:|:---------------:|
| db-pentest | Synthesizes representative findings per check type and db_type | Yes | Yes |
| web-proxy | Produces complete `WebProxySessionReport` | Yes | Yes |
| mobile-dynamic | Produces complete `DynamicMobileReport` | Yes | Yes |
| wireless-active | Produces synthetic findings via `-d` flag | Yes | Yes |
| evasion | Dry-run-only handler (real requires feature) | Yes | Yes |
| postex | Dry-run-only handler (real requires feature) | Yes | Yes |
| c2 | Dry-run-only handler (real requires feature) | Yes | Yes |

The db-pentest dry-run (`utils::populate_dry_run_findings` + `simulate_advanced_test_vector`) exercises 100% of report generation paths including correlation, compliance, and baseline operations — always producing a valid, serializable `DbPentestReport` with zero DB/network interaction.

## Scope Requirements

All DefenseLab operations require explicit scope for strict automated surfaces (MCP, Agent, CI). The `DomainDescriptor` for db-pentest (`domain/mod.rs:465`) sets `requires_explicit_scope: true`.

`DefenseLabPreset` enforces `localhost_or_private_required: true` for all built-in presets (`presets.rs:216-223`). The scope system (`scope.rs`) classifies addresses via `classify_address()` into `AddressClass` variants (Loopback, Private, Public, etc.) and evaluates targets against `Scope` rules (allowed/excluded CIDR patterns).

For ManualPermissive profiles, missing scope for safe low-risk operations may downgrade to `Warn` (`policy_decision.rs:1214-1262`). For DefenseLab operations (Intrusive+ risk), scope misses produce `RequireConfirmation` or `Deny`.

## Invariants

1. **DefenseLab is local/private**: All DefenseLab profiles reject public targets by default. `DefenseLabPreset.localhost_or_private_required` is always `true` (verified by test `presets.rs:216`).
2. **Dry-run is safe**: Every lab-gated module produces a complete report without network interaction when dry-run is active.
3. **Explicit scope required**: Strict surfaces (MCP/Agent/CI) always require explicit scope for DefenseLab operations (`requires_explicit_scope: true` in `DomainDescriptor`).
4. **Confirmation required for high-risk**: `HighRisk` confirmation class is triggered for Intrusive/DbPentest/StressTest/CredentialTesting/ExploitAdjacent/RemoteExecution risk tiers under ManualPermissive (`policy_decision.rs:1415-1432`).
5. **Manual overrides are narrow**: `--yes` only covers low-risk scope confirmations (OutOfScope, TargetExpansion). High-risk and capability confirmations require dedicated `--allow-*` flags.
6. **HazardousLab is strictly gated**: Only `distributed-system-stress` uses HazardousLab mode. It requires explicit feature flags (`stress-testing`), raw sockets, and policy approval.
7. **No dangerous defaults**: No profile enables raw sockets, IP spoofing, or SYN flood by default (except `distributed-system-stress` and `synvoid-protocol-edge`).
8. **Engine modules are policy-free**: Security modules (scanner, fuzzer, etc.) are policy-free executors. All authorization happens upstream via `EnforcementContext::evaluate()`.
9. **PolicyDecision is the single source of truth**: Operation policy checks use `OperationMetadata` and `evaluate_operation_policy()`, not inline checks.
10. **Dry-run-only handlers**: Evasion, postex, and c2 modules have dry-run-only handler behavior — real execution requires feature-gated code paths behind explicit `--allow-*` flags.

## Integration with Policy System

Defense-lab profiles integrate with the unified operation taxonomy:

- Each profile declares an `OperationMode` (DefenseLab or HazardousLab)
- Each profile declares `IntendedUse` values (WafRegression, SynvoidRegression, etc.)
- Policy decisions are emitted for every operation with structured metadata
- Budgets enforce finite limits on all defense-lab runs

See `config/policy.rs` for `OperationMode`, `OperationRisk`, and `IntendedUse`.
See `config/policy_decision.rs` for `PolicyDecision` and `ConfirmationClass`.
See `config/budget.rs` for `ExecutionBudget`.
See `config/presets.rs` for built-in defense-lab presets.

## Output Model

A defense-lab run produces structured output suitable for regression analysis. The canonical envelope for this is `RunManifest` defined in `crates/eggsec/src/output/run_manifest.rs` and documented in `architecture/output.md`.

| Field | Description |
|-------|-------------|
| `schema_version` | Manifest schema version for forward compatibility |
| `run_id` | Unique identifier for this run |
| `started_at` / `ended_at` | Timestamps |
| `eggsec_version` | Version used |
| `target_scope` | Target specification |
| `profile` | Defense-lab profile name |
| `probe_intents` | Categorized probe metadata (uses `ProbeIntent` enum from `probe.rs`) |
| `risk_budget` | Allowed risk tier (uses `ProbeRisk` enum from `probe.rs`) |
| `feature_flags` | Enabled features |
| `observations` | Raw probe results (response codes, latencies, payloads) |
| `findings` | Interpreted findings |
| `artifacts` | Paths to output files (JSON, HTML, CSV, etc.) |
| `baseline_id` | Reference to baseline run, if comparing |
| `diff_summary` | Summary of differences against baseline (uses `DiffSummary` from `output::diff`) |

The manifest wraps run-level provenance so that two manifests can be meaningfully compared. A baseline run produces a manifest with `baseline_id: None`. Subsequent runs reference the baseline and populate `diff_summary`. The `DiffSummary` type in `crates/eggsec-output/src/diff.rs` and `BaselineComparison` in `crates/eggsec-output/src/baseline.rs` provide the comparison logic.

Mobile dynamic (under `mobile-dynamic` feature) is a standalone defense-lab surface (CLI + local reports + optional `to_scan_report_data_dynamic` bridge; MCP/agent/TUI/pipeline absent). Phase 4c (2026-06-12) added partial supply-chain observation (native-load builtin + correlation), regression enrichment, bundle manifest, and a pure workflow helper; all dry-run safe. See architecture/mobile.md + docs/MOBILE.md.

## Shared Probe Vocabulary

Defense-lab profiles use the shared `ProbeIntent` and `ProbeRisk` enums defined in `crates/eggsec/src/probe.rs`. These enums are also used by scanner, NSE, WAF, and loadtest modules to tag probes with consistent intent and risk metadata. This enables guardrails and budget enforcement across all assessment modes.

## Defense-Lab Profiles

All profiles are fully implemented in the `ScanProfile` enum (`cli/mod.rs:334-352`) and wired into the stage runner (`pipeline/stage.rs:96-111`).

| Profile | Semantics | Stages | Feature Requirements |
|---------|-----------|--------|---------------------|
| `defense-lab` | Local/private-scope controlled probe suite. Comprehensive defense validation. | PortScan → Fingerprint → EndpointScan → Waf → Fuzz | Explicit scope required. No stress/packet features by default. |
| `synvoid-local` | Localhost/container/private lab defaults for Synvoid validation. | PortScan → Fingerprint → EndpointScan → Waf | Targets restricted to loopback or private CIDRs. |
| `waf-regression` | WAF payload and evasion-resistance regression profile. | PortScan → Fingerprint → Waf | Focused on payload classification, encoding bypass, case manipulation. |
| `protocol-edge` | Malformed protocol, TCP/TLS/HTTP edge behavior. | PortScan → Fingerprint | Requires `packet-inspection` feature. No stress features by default. |
| `nse-safe` | Sandboxed safe/default/version/discovery NSE scripts only. | PortScan → Fingerprint → EndpointScan | Requires `nse` + `nse-sandbox` features. No intrusive categories. |
| `db-regression` | Defense-lab family profile for db-pentest regression. Maps to native `Stage::DbPentest` (Phase 4) when `db-pentest` feature enabled. | `Stage::DbPentest` | `db-pentest` feature. Full native Stage::DbPentest (Phase 4). Phase 5 adds MongoDB/Redis engines, cross-DB correlation, compliance mapping, optional MCP exposure. |

### Guardrails for Defense-Lab Profiles

1. **Scope required**: All defense-lab profiles require explicit scope (localhost or private CIDR).
2. **Rate/concurrency budgets**: Required for any load-bearing probes.
3. **Feature gates**: Stress features (`stress-testing`) and packet features (`packet-inspection`) require explicit opt-in at both compile time and runtime.
4. **No dangerous defaults**: No profile enables raw sockets, IP spoofing, or SYN flood by default.
5. **NSE sandbox**: The `nse-safe` profile only runs sandboxed script categories (safe, default, version, discovery). Intrusive categories require explicit opt-in.

## Future Integration

- **Synvoid metrics import**: Pull WAF decision logs and rule-match counts directly from Synvoid
- **Agent loop integration**: Automated defense-lab runs triggered on schedule or CI events
- **Golden baseline fixtures**: Versioned baseline captures for regression testing
- **CI-compatible regression profiles**: Lightweight profiles that run in CI pipelines to detect defense regressions early
- **Mobile static/regression profiles**: `mobile-static`, `mobile-dynamic`, and `mobile-regression` pipeline profiles (aspirational; Phase 1 + Phase 2a + final polish + close-out polish are standalone CLI `eggsec mobile ...` under `SafeActive`/`DefenseLab` only, suitable for defense-lab use on lab-provided APKs/IPAs and controlled lab devices). See `architecture/mobile.md`. Dynamic loadout completed. Phase 2 closed (proxy + permissions + correlation; 2026-06-12) + final polish + close-out + Phase 3/4a (Frida + CorrelationEngine) delivered 2026-06-12 all remain standalone defense-lab (no pipeline/TUI/MCP).
- **Wireless stages**: Similarly aspirational (`WirelessAnalysis` or `wireless-defense` profile). See `architecture/wireless.md`. Decision from integration work: Defer.

## Integration with Reporting Pipeline

A defense-lab run produces structured output suitable for regression analysis. The canonical envelope for this is `RunManifest` defined in `crates/eggsec/src/output/run_manifest.rs` and documented in `architecture/output.md`.

| Field | Description |
|-------|-------------|
| `schema_version` | Manifest schema version for forward compatibility |
| `run_id` | Unique identifier for this run |
| `started_at` / `ended_at` | Timestamps |
| `eggsec_version` | Version used |
| `target_scope` | Target specification |
| `profile` | Defense-lab profile name |
| `probe_intents` | Categorized probe metadata (uses `ProbeIntent` enum from `probe.rs`) |
| `risk_budget` | Allowed risk tier (uses `ProbeRisk` enum from `probe.rs`) |
| `feature_flags` | Enabled features |
| `observations` | Raw probe results (response codes, latencies, payloads) |
| `findings` | Interpreted findings |
| `artifacts` | Paths to output files (JSON, HTML, CSV, etc.) |
| `baseline_id` | Reference to baseline run, if comparing |
| `diff_summary` | Summary of differences against baseline (uses `DiffSummary` from `output::diff`) |

The manifest wraps run-level provenance so that two manifests can be meaningfully compared. A baseline run produces a manifest with `baseline_id: None`. Subsequent runs reference the baseline and populate `diff_summary`. The `DiffSummary` type in `crates/eggsec-output/src/diff.rs` and `BaselineComparison` in `crates/eggsec-output/src/baseline.rs` provide the comparison logic.

Lightweight opt-in reporting unification only. Auto-bridge lives in `commands/handlers/report.rs`. See also the short shared "Output Models" block in `docs/USAGE.md` (Report Management → Convert Reports) as the canonical cross-reference for the three-surface distinction.

*Last verified against source: 2026-08-25*
