---
name: eggsec-pipeline
description: "Security assessment pipeline orchestration - use when working with Stage enum, ScanProfile, PipelineContext, session persistence, pipeline execution flow, or stage dispatch."
---

# Eggsec Pipeline Skill

Pipeline module workflows and patterns for orchestrating security assessments.

## Key Files

| File | Purpose |
|------|---------|
| `crates/eggsec/src/pipeline/mod.rs` | Module entry, CLI entry points (`run_cli`, `resume_cli`) |
| `crates/eggsec/src/pipeline/stage.rs` | `Stage` enum, profiles, aliases, parsing |
| `crates/eggsec/src/pipeline/executor.rs` | `Pipeline` struct, sequential execution, stage dispatch |
| `crates/eggsec/src/pipeline/context.rs` | `PipelineContext` for inter-stage data sharing |
| `crates/eggsec/src/pipeline/session.rs` | `PipelineSession` for pause/resume via JSON snapshots |
| `crates/eggsec/src/pipeline/report.rs` | `PipelineReport`, HTML/CSV output |
| `crates/eggsec/src/tool/implementations/pipeline.rs` | `PipelineTool` implementing `SecurityTool` |

## Core Concepts

### Stage Enum (`stage.rs:8-20`)

```rust
pub enum Stage {
    PortScan,
    Fingerprint,
    EndpointScan,
    Fuzz,
    LoadTest,
    Waf,
    Recon,
    Vuln,
    #[cfg(feature = "db-pentest")]
    DbPentest,
    #[cfg(feature = "web-proxy")]
    WebProxy,
}
```

### Profiles

`Stage::from_profile(ScanProfile)` maps CLI profiles to stage sequences. There are 18 `ScanProfile` variants:
- `Quick`: PortScan + Fingerprint
- `Endpoint`: PortScan + Fingerprint + EndpointScan
- `Web`, `Api`, `Stealth`, `Deep`, `Auth`: PortScan + Fingerprint + EndpointScan + Fuzz (all five are the same stage list)
- `Waf`: PortScan + Fingerprint + EndpointScan + Waf
- `Full`: PortScan + Fingerprint + EndpointScan + Fuzz + LoadTest (**no** Vuln stage — use `Vuln` for that)
- `Recon`: PortScan + Fingerprint + EndpointScan + Recon + Fuzz
- `Vuln`: PortScan + Fingerprint + EndpointScan + Recon + Vuln + Fuzz
- Defense-lab family: `DefenseLab` (+Waf, +Fuzz), `SynvoidLocal` (+Waf), `WafRegression` (PortScan + Fingerprint + Waf), `ProtocolEdge` (PortScan + Fingerprint), `NseSafe` (PortScan + Fingerprint + EndpointScan)
- `DbRegression`: DbPentest when `db-pentest` is enabled, else the Web-like set
- `WebProxy`: WebProxy stage when `web-proxy` is enabled, else the Web-like set

### Stage Aliases

Supported aliases in `Stage::from_string()`:
- `port`, `portscan`, `port-scan` → PortScan
- `fingerprint`, `fp` → Fingerprint
- `endpoint`, `endpoints`, `endpoint-scan` → EndpointScan
- `fuzz`, `fuzzer`, `fuzzing`, `graphql`, `oauth`, `jwt` → Fuzz
- `load`, `loadtest`, `load-test` → LoadTest
- `waf` → Waf
- `recon` → Recon
- `vuln`, `vulnerability`, `vuln-assess` → Vuln
- `db`, `dbpentest`, `db-pentest` → DbPentest (feature-gated; `None` without it)
- `proxy`, `webproxy`, `web-proxy`, `intercept` → WebProxy (feature-gated; `None` without it)

### PipelineContext (`context.rs`)

Persists inter-stage state (`context.rs:12-26`):
```rust
pub struct PipelineContext {
    pub target: String,
    pub open_ports: Vec<u16>,
    pub services: FxHashMap<u16, ServiceFingerprint>,  // context.rs:15
    pub endpoints: Vec<EndpointResult>,
    pub port_results: Vec<PortResult>,
    pub http_ports: Vec<u16>,
    pub vuln_assessment: Option<VulnAssessment>,        // skipped when None
    pub load_test_results: Option<LoadTestResults>,      // skipped when None
    #[cfg(feature = "web-proxy")]
    pub web_proxy_report: Option<WebProxySessionReport>, // skipped when None
}
```

Data flow: PortScan → `update_ports()` → Fingerprint → `update_services()` → EndpointScan → `update_endpoints()` → subsequent stages.

### Session Persistence (`session.rs`)

Saves JSON snapshots only when output path matches `*.session` or `*.session.json`. Checkpointing happens after each stage in `Pipeline::run()`.

## CLI Integration

### Handlers (`commands/handlers/scan.rs`)
- `handle_scan()` - Calls `pipeline::run_cli()`, validates scope
- `handle_resume()` - Calls `pipeline::resume_cli()`

### Tool Integration (`tool/implementations/pipeline.rs`)
- `PipelineTool` implements `SecurityTool` trait
- `id()` → `"scan"`, `name()` → `"Security Assessment Pipeline"`
- Wraps `run_cli_with_callback()` for finding propagation

## Execution Flow

```
ScanArgs → Pipeline::from_args_with_config()
              ↓
         Pipeline::run() → sequential stage iteration
              ↓
execute_stage() → match Stage (all stages available without `cli` feature):
  Stage::PortScan → scanner::ports::scan_ports()
  Stage::Fingerprint → scanner::fingerprint::fingerprint_services()
  Stage::EndpointScan → scanner::endpoints::scan_endpoints()
  Stage::Fuzz → fuzzer::engine::FuzzEngine::new_with_tui_mode(FuzzConfig).run()
  Stage::LoadTest → loadtest::LoadTestRunner::from_config_with_engine().run()
  Stage::Waf → waf::WafEngine::new(WafConfig).run()
  Stage::Recon → recon::runner::run_full_recon_from_request()
  Stage::Vuln → vuln::run_cli()
  Stage::DbPentest → db_pentest::run_cli() (feature-gated)
  Stage::WebProxy → proxy::intercept::run_cli() (feature-gated)
              ↓
         PipelineReport → Display / JSON / HTML / CSV / SARIF / JUnit
```

Parser-independent engine entry points are the canonical constructors for
non-CLI consumers. CLI args are converted into plain config types (`FuzzConfig`,
`WafConfig`, `LoadTestRunConfig`, `ReconRequest`) before reaching engine code,
so removing the `cli` feature no longer hides any pipeline stage.

## Key Patterns

1. **Sequential execution** via simple `match` in `execute_stage()` - no trait abstraction
2. **Context sharing** via `Arc<Mutex<PipelineContext>>`
3. **Session persistence** only when output path is session-like
4. **Plain config types** for engine entry points (FuzzConfig, WafConfig,
   LoadTestRunConfig, ReconRequest) — CLI args convert via `From` impls; non-CLI
   consumers construct the plain type directly
5. **Hash Collections**: Always use `FxHashMap` from `rustc_hash` instead of `std::collections::HashMap`
6. **Output writing**: Extracted to `write_output()` helper in `mod.rs:77` to avoid code duplication

## Bug Fixes (2026-05-27)

| Issue | Fix |
|-------|-----|
| Duplicate output writing code in `run_cli()` and `run_cli_with_callback()` | Extracted to `write_output()` helper |
| `StageResult.duration_ms` serialized to JSON unnecessarily | Added `#[serde(skip)]` attribute |
| `StageResult` lacked constructor | Added `StageResult::new()` builder |
| Progress bar created for empty stage list | Changed condition to `self.tui_mode \|\| self.stages.is_empty()` |

## Override File

For specialized guidance, see:
- `crates/eggsec/src/pipeline/AGENTS.override.md` - Performance patterns, bug fixes

## Testing

```bash
cargo test --lib -p eggsec pipeline::
cargo check --lib -p eggsec
cargo clippy --lib -p eggsec
```

## Resources
- `crates/eggsec/src/pipeline/AGENTS.override.md` - (if exists)
- `architecture/pipeline.md` - Architecture documentation
- `AGENTS.md` - General project guidelines