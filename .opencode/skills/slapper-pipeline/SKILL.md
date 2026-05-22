# Slapper Pipeline Skill

Pipeline module workflows and patterns for orchestrating security assessments.

## Key Files

| File | Purpose |
|------|---------|
| `crates/slapper/src/pipeline/mod.rs` | Module entry, CLI entry points (`run_cli`, `resume_cli`) |
| `crates/slapper/src/pipeline/stage.rs` | `Stage` enum, profiles, aliases, parsing |
| `crates/slapper/src/pipeline/executor.rs` | `Pipeline` struct, sequential execution, stage dispatch |
| `crates/slapper/src/pipeline/context.rs` | `PipelineContext` for inter-stage data sharing |
| `crates/slapper/src/pipeline/session.rs` | `PipelineSession` for pause/resume via JSON snapshots |
| `crates/slapper/src/pipeline/report.rs` | `PipelineReport`, HTML/CSV output |
| `crates/slapper/src/tool/implementations/pipeline.rs` | `PipelineTool` implementing `SecurityTool` |

## Core Concepts

### Stage Enum (`stage.rs:5-14`)

```rust
pub enum Stage {
    PortScan,
    Fingerprint,
    EndpointScan,
    Fuzz,
    LoadTest,
    Waf,
    Recon,
}
```

### Profiles

`Stage::from_profile(ScanProfile)` maps CLI profiles to stage sequences:
- `Quick`: PortScan + Fingerprint
- `Endpoint`: PortScan + Fingerprint + EndpointScan
- `Web`: PortScan + Fingerprint + EndpointScan + Fuzz
- `Full`: PortScan + Fingerprint + EndpointScan + Fuzz + LoadTest
- `Waf`: PortScan + Fingerprint + EndpointScan + Waf
- `Api`: PortScan + Fingerprint + EndpointScan + Fuzz
- `Recon`: PortScan + Fingerprint + EndpointScan + Recon + Fuzz

### Stage Aliases

Supported aliases in `Stage::from_string()`:
- `port`, `portscan`, `port-scan` → PortScan
- `fingerprint`, `fp` → Fingerprint
- `endpoint`, `endpoints`, `endpoint-scan` → EndpointScan
- `fuzz`, `fuzzer`, `fuzzing`, `graphql`, `oauth`, `jwt` → Fuzz
- `load`, `loadtest`, `load-test` → LoadTest
- `waf` → Waf
- `recon` → Recon

### PipelineContext (`context.rs`)

Persists inter-stage state:
```rust
pub struct PipelineContext {
    pub target: String,
    pub open_ports: Vec<u16>,
    pub services: FxHashMap<u16, ServiceFingerprint>,  // Line 12
    pub endpoints: Vec<EndpointResult>,
    pub port_results: Vec<PortResult>,
    pub http_ports: Vec<u16>,
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
execute_stage() → match Stage:
  Stage::PortScan → scanner::ports::scan_ports()
  Stage::Fingerprint → scanner::fingerprint::fingerprint_services()
  Stage::EndpointScan → scanner::endpoints::scan_endpoints()
  Stage::Fuzz → FuzzEngine::new_with_tui_mode().run()
  Stage::LoadTest → LoadTestRunner::from_args_with_config().run()
  Stage::Waf → waf::run_cli()
  Stage::Recon → recon::run_cli()
              ↓
         PipelineReport → Display / JSON / HTML / CSV / SARIF / JUnit
```

## Key Patterns

1. **Sequential execution** via simple `match` in `execute_stage()` - no trait abstraction
2. **Context sharing** via `Arc<Mutex<PipelineContext>>`
3. **Session persistence** only when output path is session-like
4. **No verify_tls in FuzzArgs** - use `common.insecure` flag instead
5. **Hash Collections**: Always use `FxHashMap` from `rustc_hash` instead of `std::collections::HashMap`
6. **Output writing**: Extracted to `write_output()` helper in `mod.rs:63-95` to avoid code duplication

## Bug Fixes (2026-05-27)

| Issue | Fix |
|-------|-----|
| Duplicate output writing code in `run_cli()` and `run_cli_with_callback()` | Extracted to `write_output()` helper |
| `StageResult.duration_ms` serialized to JSON unnecessarily | Added `#[serde(skip)]` attribute |
| `StageResult` lacked constructor | Added `StageResult::new()` builder |
| Progress bar created for empty stage list | Changed condition to `self.tui_mode \|\| self.stages.is_empty()` |

## Override File

For specialized guidance, see:
- `crates/slapper/src/pipeline/AGENTS.override.md` - Performance patterns, bug fixes

## Testing

```bash
cargo test --lib -p slapper pipeline::
cargo check --lib -p slapper
cargo clippy --lib -p slapper
```

## Resources
- `crates/slapper/src/pipeline/AGENTS.override.md` - (if exists)
- `architecture/pipeline.md` - Architecture documentation
- `AGENTS.md` - General project guidelines