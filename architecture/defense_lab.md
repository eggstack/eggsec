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

## Safety Model

Defense-lab mode enforces strict safety boundaries:

| Constraint | Default |
|------------|---------|
| **Target scope** | Localhost or private-lab ranges only |
| **Explicit scope** | Required for all defense-lab profiles |
| **Rate/concurrency budgets** | Required for load-bearing and stress probes |
| **Feature gates** | Stress and packet features require `--features stress-testing` / `--features packet-inspection` |
| **No unscoped internet** | Defense-lab profiles reject public targets by default |

Defense-lab profiles should not be run against targets you do not own or control.

### Mobile Static Analysis (Phase 1)

`mobile-static` is currently exposed as the standalone CLI command `eggsec mobile <apk-or-ipa>` (feature `mobile`). It performs pure-Rust static analysis of Android APKs and iOS IPAs on user-supplied lab binaries only (manifest, config, permissions, transport settings, hardcoded secrets, signing indicators, etc.). No dynamic instrumentation, Frida, or network activity. Policy gate uses `SafeActive` via `EnforcementContext`. Outputs local `MobileScanReport`/`MobileFinding` types directly (with optional `to_scan_report_data` bridge for unified report consumers). Not yet integrated with `ScanProfile` pipelines; `mobile-static`/`mobile-regression` profiles are aspirational per the handoff plan. See `architecture/mobile.md`, `architecture/cli_commands.md` (Special Cases), and `crates/eggsec/src/mobile/`.

Wireless passive recon (`eggsec wireless <iface>`) is similarly a standalone-complete defense-lab surface (CLI primary + TUI tab under `wireless` feature; see `architecture/wireless.md`). It produces local `WirelessScanResult` + findings directly, with an optional `to_scan_report_data` bridge (and CLI auto-bridge for `report convert`). Not integrated into `ScanProfile` pipelines or dedicated profiles. No stages were added on standalone completion.

Both are lightweight, opt-in for reporting unification only; they preserve their standalone nature.

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
- **Mobile static/regression profiles**: `mobile-static` and `mobile-regression` pipeline profiles (aspirational; Phase 1 is standalone CLI `eggsec mobile` under `SafeActive` only, suitable for defense-lab use on lab-provided APKs/IPAs). See `architecture/mobile.md`, `architecture/proposed-wireless-mobile-stages.md`, and plans/mobile-first-handoff-plan.md + integration-work-plan.md.
- **Wireless stages**: Similarly aspirational (`WirelessAnalysis` or `wireless-defense` profile). See the proposed stages design note and `architecture/wireless.md`. Decision from integration work: Defer.

## Integration with Policy System

Defense-lab profiles integrate with the unified operation taxonomy:

- Each profile declares an `OperationMode` (DefenseLab or HazardousLab)
- Each profile declares `IntendedUse` values (WafRegression, SynvoidRegression, etc.)
- Policy decisions are emitted for every operation with structured metadata
- Budgets enforce finite limits on all defense-lab runs

See `config/policy.rs` for `OperationMode`, `OperationRisk`, and `IntendedUse`.
See `config/policy_decision.rs` for `PolicyDecision`.
See `config/budget.rs` for `ExecutionBudget`.
See `config/presets.rs` for built-in defense-lab presets.
