# Pipeline Module

Orchestrates multi-stage security assessments by chaining scanner, fuzzer, WAF, recon, load-test, and vulnerability stages into a single pipeline with context sharing, session persistence, and risk-budget enforcement.

## Role & Responsibilities

The pipeline module provides a deterministic, repeatable execution harness that:

1. Resolves a `ScanProfile` into an ordered list of `Stage` values.
2. Executes stages sequentially (or concurrently by dependency wave), passing inter-stage results through a shared `PipelineContext`.
3. Enforces per-stage risk budgets against the profile's maximum, skipping stages that exceed the budget.
4. Persists session checkpoints after each stage so interrupted runs can be resumed.
5. Aggregates all stage outputs into a `PipelineReport` and writes it in the requested format.

## Location & Feature Gating

| Item | Path | Feature gate |
|------|------|--------------|
| Module root | `crates/eggsec/src/pipeline/mod.rs` | None (always compiled) |
| Stage definitions | `crates/eggsec/src/pipeline/stage.rs` | None |
| Executor | `crates/eggsec/src/pipeline/executor.rs` | None |
| Context | `crates/eggsec/src/pipeline/context.rs` | None |
| Session persistence | `crates/eggsec/src/pipeline/session.rs` | None |
| Report generation | `crates/eggsec/src/pipeline/report.rs` | None |
| PipelineTool (tool registry) | `crates/eggsec/src/tool/implementations/pipeline.rs` | `tool-api` |
| CLI entry points (`run_cli`, `resume_cli`) | `crates/eggsec/src/pipeline/mod.rs:248-312` | `cli` |
| Callback entry points (`run_with_callback`, `run_with_callback_for_profile`) | `crates/eggsec/src/pipeline/mod.rs:137-184` | `tool-api` |
| CLI args (`ScanArgs`, `ResumeArgs`) | `crates/eggsec/src/cli/` | `cli` |
| `DbPentest` stage variant | `crates/eggsec/src/pipeline/stage.rs:17` | `db-pentest` |
| `WebProxy` stage variant | `crates/eggsec/src/pipeline/stage.rs:19` | `web-proxy` |

**No module-level feature gate**: the pipeline module compiles unconditionally. The `cli` feature gates only the `ScanArgs`/`ResumeArgs` constructors (`from_args`, `from_args_with_config`, `from_args_with_tui_mode`); the canonical constructors `Pipeline::new()` and `Pipeline::from_profile()` are always available.

## Architecture

### Stage Enum (`pipeline/stage.rs:6-20`)

10 variants, 8 unconditional + 2 feature-gated:

| Variant | Display | ProbeIntent | ProbeRisk | Feature gate |
|---------|---------|-------------|-----------|--------------|
| `PortScan` | "Port Scan" | Discovery | SafeActive | — |
| `Fingerprint` | "Fingerprint" | Fingerprint | Passive | — |
| `EndpointScan` | "Endpoint Scan" | ServiceValidation | SafeActive | — |
| `Fuzz` | "Fuzzing" | EvasionResistance | Intrusive | — |
| `LoadTest` | "Load Test" | LoadBearing | Stress | — |
| `Waf` | "WAF Test" | WafEvaluation | Intrusive | — |
| `Recon` | "Recon" | Discovery | Passive | — |
| `Vuln` | "Vulnerability Assessment" | ServiceValidation | SafeActive | — |
| `DbPentest` | "DB Pentest" | ServiceValidation | Intrusive | `db-pentest` |
| `WebProxy` | "Web Proxy Intercept" | WafEvaluation | Intrusive | `web-proxy` |

### Stage Aliases (`stage.rs:145-180`)

`Stage::from_string()` normalizes user-facing aliases (case-insensitive):

| Alias group | Maps to |
|-------------|---------|
| `port`, `portscan`, `port-scan` | `PortScan` |
| `fingerprint`, `fp` | `Fingerprint` |
| `endpoint`, `endpoints`, `endpoint-scan` | `EndpointScan` |
| `fuzz`, `fuzzer`, `fuzzing`, `graphql`, `oauth`, `jwt` | `Fuzz` |
| `load`, `loadtest`, `load-test` | `LoadTest` |
| `waf` | `Waf` |
| `recon` | `Recon` |
| `vuln`, `vulnerability`, `vuln-assess` | `Vuln` |
| `db`, `dbpentest`, `db-pentest` | `DbPentest` (returns `None` without `db-pentest`) |
| `proxy`, `webproxy`, `web-proxy`, `intercept` | `WebProxy` (returns `None` without `web-proxy`) |

### ScanProfile Enum (`types.rs:121-142`)

Exactly 18 variants:

```
Quick, Endpoint, Web, Waf, Full, Api, Recon, Stealth, Deep, Vuln,
Auth, DefenseLab, SynvoidLocal, WafRegression, ProtocolEdge, NseSafe,
DbRegression, WebProxy
```

### Profile → Stage Mapping (`stage.rs:42-143`)

| # | Profile | Stages | Stage count |
|---|---------|--------|:-----------:|
| 1 | `Quick` | PortScan → Fingerprint | 2 |
| 2 | `Endpoint` | PortScan → Fingerprint → EndpointScan | 3 |
| 3 | `Web` | PortScan → Fingerprint → EndpointScan → Fuzz | 4 |
| 4 | `Waf` | PortScan → Fingerprint → EndpointScan → Waf | 4 |
| 5 | `Full` | PortScan → Fingerprint → EndpointScan → Fuzz → LoadTest | 5 |
| 6 | `Api` | PortScan → Fingerprint → EndpointScan → Fuzz | 4 |
| 7 | `Recon` | PortScan → Fingerprint → EndpointScan → Recon → Fuzz | 5 |
| 8 | `Stealth` | PortScan → Fingerprint → EndpointScan → Fuzz | 4 |
| 9 | `Deep` | PortScan → Fingerprint → EndpointScan → Fuzz | 4 |
| 10 | `Vuln` | PortScan → Fingerprint → EndpointScan → Recon → Vuln → Fuzz | 6 |
| 11 | `Auth` | PortScan → Fingerprint → EndpointScan → Fuzz | 4 |
| 12 | `DefenseLab` | PortScan → Fingerprint → EndpointScan → Waf → Fuzz | 5 |
| 13 | `SynvoidLocal` | PortScan → Fingerprint → EndpointScan → Waf | 4 |
| 14 | `WafRegression` | PortScan → Fingerprint → Waf | 3 |
| 15 | `ProtocolEdge` | PortScan → Fingerprint | 2 |
| 16 | `NseSafe` | PortScan → Fingerprint → EndpointScan | 3 |
| 17 | `DbRegression` | `DbPentest` (with `db-pentest`); else PortScan → Fingerprint → EndpointScan → Waf → Fuzz | 1 or 5 |
| 18 | `WebProxy` | `WebProxy` (with `web-proxy`); else PortScan → Fingerprint → EndpointScan → Waf → Fuzz | 1 or 5 |

### Profile Metadata (`types.rs:260-385`)

Each profile carries additional policy metadata:

| Method | Purpose | Values used in pipeline |
|--------|---------|------------------------|
| `max_risk_budget()` | Maximum `ProbeRisk` allowed | Quick/ProtocolEdge/NseSafe → SafeActive; Stealth → Passive; DefenseLab/SynvoidLocal/WafRegression/DbRegression/WebProxy/Endpoint/Web/Waf/Recon/Vuln/Auth → Intrusive; Full/Api/Deep → Stress |
| `requires_private_scope()` | Defense-lab gating | DefenseLab, SynvoidLocal, DbRegression, WafRegression, ProtocolEdge, NseSafe, WebProxy → true |
| `requires_packet_inspection()` | Feature gate | ProtocolEdge → true |
| `requires_nse()` | Feature gate | NseSafe → true |
| `operation_mode()` | StandardAssessment vs DefenseLab | Standard: Quick…Auth; DefenseLab: DefenseLab…WebProxy |
| `intended_uses()` | Policy categories | Maps to `WebAssessment`, `ApiAssessment`, `WafRegression`, `SynvoidRegression`, `ProtocolEdgeValidation`, `CodingAgentVerification` |

### Dependency Waves for Concurrent Execution (`executor.rs:233-268`)

When `concurrent_stages = true`, `Pipeline::run_concurrent()` partitions stages into waves:

| Wave | Stages | Dependency |
|:----:|--------|------------|
| 0 | PortScan, Recon | Independent |
| 1 | Fingerprint | Needs open_ports from PortScan |
| 2 | EndpointScan | Needs http_ports from Fingerprint |
| 3 | Fuzz, Waf, LoadTest, Vuln, DbPentest, WebProxy | Needs base_url from EndpointScan |

Stages within a wave execute via `futures::future::join_all()`. Waves execute sequentially.

### Constants

| Constant | Value | Location |
|----------|-------|----------|
| `DEFAULT_SCAN_PORTS` | `"80,443"` | `stage.rs:229` |
| `EXTENDED_SCAN_PORTS` | 37 ports (21–9090) | `stage.rs:231` |
| Stage timeout | 300 seconds | `executor.rs:387` |
| Default concurrency | 10 | `executor.rs:64,92,126` |
| Load test requests | 100 | `executor.rs:1110` |
| Load test timeout | 10 seconds | `executor.rs:1112` |
| Endpoint scan timeout | 10 seconds | `executor.rs:1028` |
| Fingerprint timeout | 5 seconds | `executor.rs:986` |
| Port scan timeout | 2 seconds | `executor.rs:959` |
| WAF timeout | 15 seconds | `executor.rs:1151` |

## Behavior / Flow

### Pipeline Execution Lifecycle

```
Pipeline::run()
  ├─ validate_defense_lab_scope()       // reject public targets for defense-lab profiles
  ├─ validate_feature_gates()           // check packet-inspection/nse requirements
  ├─ if concurrent_stages → run_concurrent()
  │     └─ dependency_waves() → join_all per wave
  └─ else sequential loop:
       for stage in self.stages:
         ├─ validate_stage_risk()       // skip if exceeds budget
         ├─ tokio::time::timeout(300s, execute_stage(stage))
         ├─ record StageResult { stage, duration_ms, success, error }
         └─ if session_path → save PipelineSession checkpoint
       └─ build PipelineReport + RunManifest
```

### Stage Dispatch (`executor.rs:595-610`)

`execute_stage()` routes to type-specific runners:

| Stage | Engine call | Config type |
|-------|-------------|-------------|
| PortScan | `scanner::ports::scan_ports()` | `PortScanConfig` |
| Fingerprint | `scanner::fingerprint::fingerprint_services()` | Positional args |
| EndpointScan | `scanner::endpoints::scan_endpoints()` | `EndpointScanConfig` |
| Fuzz | `fuzzer::engine::FuzzEngine::new_with_tui_mode().run()` | `FuzzConfig` |
| LoadTest | `loadtest::LoadTestRunner::from_config_with_engine().run()` | `LoadTestRunConfig` |
| Waf | `waf::WafEngine::new().run()` | `WafConfig` |
| Recon | `recon::runner::run_full_recon_from_request()` | `ReconRequest` |
| Vuln | Internal `run_vuln()` | Collects findings from context |
| DbPentest | `db_pentest::run_db_pentest_cli()` | `DbPentestRunArgs` |
| WebProxy | Internal `run_web_proxy_stage()` | Dry-run proxy analysis |

### Context Data Flow (`context.rs:27-85`)

```
PipelineContext::new(target)
  → PortScan::run_port_scan()      → update_ports()      → open_ports, port_results
  → Fingerprint::run_fingerprint() → update_services()   → services (FxHashMap), http_ports
  → EndpointScan::run_endpoint_scan() → update_endpoints() → endpoints
  → Fuzz/Waf/LoadTest use context.get_base_url() → "https://{target}" or "http://{target}:{port}"
  → Vuln::run_vuln()               → update_vuln_assessment()
  → LoadTest::run_load_test()      → update_load_test_results()
  → WebProxy::run_web_proxy_stage() → update_web_proxy_report()
```

### Session Persistence (`session.rs`)

- **Trigger**: `session_path` is set when `--output` ends with `.session` or `.session.json`.
- **Format**: Single JSON file, written atomically via `OpenOptions` with `truncate(true)` + `write(true)`.
- **Unix permissions**: `0o600` (owner read/write only) on Unix systems (`session.rs:31`).
- **Checkpoint timing**: After each stage completes (sequential), or once after all waves complete (concurrent).
- **Resume**: `Pipeline::from_session(session)` restores remaining stages, context, spoof config, concurrency, and config.

### PipelineSession Fields (`session.rs:9-23`)

```rust
pub struct PipelineSession {
    pub target: String,
    pub profile: ScanProfile,
    pub completed_stages: Vec<Stage>,
    pub remaining_stages: Vec<Stage>,
    pub context: PipelineContext,
    pub spoof_config: SpoofConfig,
    pub concurrency: Option<usize>,
    pub concurrent_stages: Option<bool>,
    pub config: Option<EggsecConfig>,
}
```

### Report Generation (`report.rs`)

`PipelineReport` aggregates all stage results plus collected data. Output formats:

| Format | Function | Location |
|--------|----------|----------|
| Display (console) | `impl Display` | `report.rs:34-147` |
| HTML | `report::generate_html()` (free fn) | `report.rs:172` |
| CSV | `report::generate_csv()` (free fn) | `report.rs:304` |
| Markdown | `report::generate_markdown()` (free fn) | `report.rs:373` |
| JSON | `serde_json::to_string_pretty()` | `mod.rs:78-92` |
| SARIF | `SarifBuilder::with_report()` | `mod.rs:98-101` |
| JUnit | `JUnitBuilder::with_report()` | `mod.rs:104-107` |

A `RunManifest` is populated after execution for regression workflows (`executor.rs:462-465`).

## Security Model

The pipeline itself is a **policy-free executor**. All authorization and scope enforcement happens upstream:

- **CLI**: `handle_scan()` (`commands/handlers/scan.rs:176`) calls `pipeline::run_cli()` after scope validation.
- **Tool API**: `PipelineTool::execute()` (`tool/implementations/pipeline.rs:48`) runs through the `SecurityTool` trait, which is invoked by `EnforcedDispatcher::dispatch_checked()`.
- **Dispatch**: The runtime bridge converts `TaskKind::Pipeline` → `OperationDescriptor` and issues an `ApprovedOperation` before invoking the pipeline.

Within the pipeline, the only security-relevant checks are:
1. **Defense-lab scope validation** (`executor.rs:271-312`): rejects public targets for defense-lab profiles.
2. **Feature-gate validation** (`executor.rs:314-333`): rejects profiles requiring `packet-inspection` or `nse` when those features are absent.
3. **Risk-budget enforcement** (`executor.rs:339-351`): skips stages whose `ProbeRisk` exceeds the profile's budget.

## Public API

### Pipeline Struct (`executor.rs:41-54`)

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `new(target: &str) -> Self` | Empty manual builder; profile defaults to Quick |
| `from_profile` | `from_profile(target: &str, profile: ScanProfile) -> Self` | Canonical profile constructor |
| `from_args` | `from_args(args: ScanArgs) -> Self` | CLI constructor (`cli` feature) |
| `from_args_with_config` | `from_args_with_config(args, config) -> Self` | CLI + config (`cli` feature) |
| `from_args_with_tui_mode` | `from_args_with_tui_mode(args, config, tui_mode) -> Self` | CLI + TUI mode (`cli` feature) |
| `from_session` | `from_session(session: PipelineSession) -> Self` | Resume from checkpoint |
| `with_spoof_config` | `with_spoof_config(self, spoof_config) -> Self` | Set IP spoofing config |
| `with_config` | `with_config(self, config) -> Self` | Set EggsecConfig |
| `add_stage` | `add_stage(self, stage) -> Self` | Append a stage |
| `with_concurrency` | `with_concurrency(self, concurrency) -> Self` | Set request concurrency |
| `with_concurrent_stages` | `with_concurrent_stages(self, enabled) -> Self` | Enable concurrent execution |
| `has_stages` | `has_stages(&self) -> bool` | Check if stages are present |
| `get_stages` | `get_stages(&self) -> &[Stage]` | Get stage list |
| `run` | `run(&self) -> Result<PipelineReport>` | Execute pipeline |

### CLI Entry Points (`mod.rs`)

| Function | Feature gate | Description |
|----------|-------------|-------------|
| `run_cli(args, config)` | `cli` | Standard CLI execution |
| `run_cli_with_callback(args, config, callback)` | `cli` + `tool-api` | CLI + finding callback |
| `run_with_callback(target, config, callback)` | `tool-api` | Parser-independent; defaults to Quick |
| `run_with_callback_for_profile(target, profile, config, callback)` | `tool-api` | Profile-aware callback entry |
| `resume_cli(args, config)` | `cli` | Resume from session file |

### PipelineTool (`tool/implementations/pipeline.rs`)

Implements `SecurityTool` trait:
- `id()` → `"scan"`
- `name()` → `"Security Assessment Pipeline"`
- `category()` → `ToolCategory::Pipeline`
- Accepts `profile` parameter (defaults to `"quick"`)
- Wraps `run_with_callback_for_profile()` with a `tokio::time::timeout` of `stage_count * 120` seconds (clamped 60–600s)

## Integration Points

| Surface | Entry point | Flow |
|---------|-------------|------|
| CLI `scan` command | `handle_scan()` → `pipeline::run_cli()` | `ScanArgs` → `Pipeline::from_args_with_config()` → `run()` |
| CLI `resume` command | `handle_resume()` → `pipeline::resume_cli()` | `ResumeArgs` → `session::load()` → `Pipeline::from_session()` → `run()` |
| CLI `plan` / `ci` commands | Use pipeline indirectly via `TaskKind::Pipeline` | Dispatch → `dispatch_inner()` → pipeline |
| Tool registry | `PipelineTool::execute()` | `ToolRequest` → `run_with_callback_for_profile()` |
| Dispatch (TaskKind::Pipeline) | `dispatch::executors` | `TaskKind::Pipeline` → pipeline entry |

## Testing

| Test suite | Path | What it covers |
|------------|------|----------------|
| Unit tests | `crates/eggsec/src/pipeline/stage.rs:259-458` | Stage parsing, aliases, profile mapping, probe intent/risk, defense-lab profile stage counts |
| Unit tests | `crates/eggsec/src/pipeline/executor.rs:1275-1513` | Profile constructor state, risk budget regression, defense-lab scope validation, concurrent wave assignment |
| Integration tests | `crates/eggsec/tests/pipeline_stage_tests.rs` | Display, from_string aliases, case insensitivity, profile stage invariants |
| Integration tests | `crates/eggsec/tests/pipeline_tests.rs` | Context, profile mapping, builder, report failure helpers |
| Integration tests | `crates/eggsec/tests/pipeline_e2e_tests.rs` | Port parsing, config defaults, scope rules |

```bash
cargo test --lib -p eggsec pipeline::
cargo test -p eggsec --test pipeline_stage_tests
cargo test -p eggsec --test pipeline_tests
cargo test -p eggsec --test pipeline_e2e_tests
```

## Invariants & Gotchas

1. **Stage timeout is 300s per stage** (`executor.rs:387`), not per-pipeline. A pipeline with 6 stages may run up to 30 minutes.
2. **Session checkpointing is not atomic across stages**: if the process crashes mid-stage, the stage's results are lost but earlier checkpoints survive.
3. **Concurrent mode does not checkpoint between waves**: the session is saved only once after all waves complete (`executor.rs:550-570`).
4. **`run_concurrent()` does not check feature gates or defense-lab scope**: those are checked in `run()` before dispatching to `run_concurrent()`.
5. **`StageResult.duration_ms` is `#[serde(skip)]`**: it is not serialized to JSON output.
6. **`generate_html()` and `generate_csv()` are free functions**, not methods on `PipelineReport`. Call as `report::generate_html(&report)`.
7. **The `DbRegression` and `WebProxy` profiles have conditional stage lists**: they map to a single feature-gated stage when the feature is enabled, or fall back to a defense-lab-like stage sequence when disabled.
8. **Fuzz stage adapts payload types by profile**: Api → `"graphql,jwt,oauth"`, Stealth → `"all"` (no mutation), Deep → `"all"` with mutation, Auth → `"jwt,oauth,idor"`, others → `"all"` (`executor.rs:1055-1062`).
9. **Defense-lab scope validation** (`executor.rs:271-312`) only checks the target string — it does not resolve DNS, so domain names that resolve to public IPs may bypass this check.

## Links

- [overview.md](overview.md)
- [cli_commands.md](cli_commands.md)
- [dispatch.md](dispatch.md)
- [config.md](config.md)

---

*Last verified against source: 2026-08-25*
