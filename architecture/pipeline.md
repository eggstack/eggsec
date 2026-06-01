# Pipeline Module

The Pipeline module allows for the orchestration of complex security assessment workflows by chaining multiple Slapper tasks together.

## Core Concepts (`src/pipeline/`)

### Stage (`stage.rs`)

A `Stage` represents a single discrete task in the pipeline, such as a port scan, a tech detection run, or a targeted fuzzer execution.

**Available Stages:**
- `PortScan` - TCP port scanning
- `Fingerprint` - Service identification (banner grabbing, HTTP fingerprinting)
- `EndpointScan` - HTTP endpoint discovery (admin panels, API paths, config files)
- `Fuzz` - Security payload fuzzing (SQLi, XSS, SSRF, etc.)
- `LoadTest` - HTTP load testing and benchmarking
- `Waf` - WAF detection and bypass testing
- `Recon` - Passive reconnaissance (DNS, WHOIS, SSL, subdomains)

**Selection**: Stages are selected from a profile (for example `quick`, `web`, `full`) or from an explicit comma-separated list via `--stages`.

**Profiles** (`Stage::from_profile()`):
| Profile | Stages |
|---------|--------|
| `quick` | PortScan → Fingerprint |
| `endpoint` | PortScan → Fingerprint → EndpointScan |
| `web` | PortScan → Fingerprint → EndpointScan → Fuzz |
| `full` | PortScan → Fingerprint → EndpointScan → Fuzz → LoadTest |
| `waf` | PortScan → Fingerprint → EndpointScan → Waf |
| `api` | PortScan → Fingerprint → EndpointScan → Fuzz |
| `recon` | PortScan → Fingerprint → EndpointScan → Recon → Fuzz |
| `stealth` | PortScan → Fingerprint → EndpointScan → Fuzz |
| `deep` | PortScan → Fingerprint → EndpointScan → Fuzz |
| `vuln` | PortScan → Fingerprint → EndpointScan → Recon → Fuzz |
| `auth` | PortScan → Fingerprint → EndpointScan → Fuzz |

**Aliases**: User-facing aliases such as `portscan`, `fp`, `endpoint-scan`, `graphql`, `oauth`, and `jwt` are normalized into canonical stages via `Stage::from_string()`.

### Executor (`executor.rs`)

The `executor.rs` file is responsible for running the pipeline from start to finish.

- **Sequential Execution**: Stages run in linear order (`for stage in &self.stages`).
- **Concurrent Execution**: `run_concurrent()` method at `executor.rs:259-297` runs stages in parallel using `futures::future::join_all()`.
- **Result Passing**: Output from one stage (for example open ports and detected HTTP services) is persisted into `PipelineContext` and consumed by later stages.
- **Failure Recording**: Stage errors are recorded per stage in `StageResult` and surfaced in the report. CLI entrypoints return `ScanFailed` if any stage failed.
- **Tool Integration**: `PipelineTool` implements `SecurityTool` for AI agent tool registry.

#### Pipeline Struct Fields (`executor.rs:38-50`)

```rust
pub struct Pipeline {
    target: String,
    stages: Vec<Stage>,
    profile: ScanProfile,
    concurrency: usize,
    concurrent_stages: bool,
    common: CommonHttpArgs,
    spoof_config: SpoofConfig,        // IP spoofing, decoy, fragment, scan type options
    context: Arc<Mutex<PipelineContext>>,
    session_path: Option<String>,     // Path for session checkpoint persistence (*.session/*.session.json)
    tui_mode: bool,
    config: Option<SlapperConfig>,    // Optional config for TLS, concurrency, default settings
}
```

- `spoof_config` (`SpoofConfig`): Configures source IP spoofing, decoy addresses, fragmentation, scan type, packet trace, max rate, and TTL. Built from CLI args via `SpoofConfig::from_args()`.
- `config` (`Option<SlapperConfig>`): Optional loaded config file. Used to read `http.verify_tls`, `http.timeout_secs`, `scan.default_concurrency`, and other settings. When `None`, defaults are used.
- `session_path` (`Option<String>`): Extracted from `--output` arg when the path ends with `.session` or `.session.json`. When set, a `PipelineSession` checkpoint is written after each stage completes.

### Pipeline Context (`context.rs`)

Maintains the state of a running pipeline, including intermediate results, shared variables, and the overall status.

```rust
pub struct PipelineContext {
    pub target: String,
    pub open_ports: Vec<u16>,
    pub services: FxHashMap<u16, ServiceFingerprint>,
    pub endpoints: Vec<EndpointResult>,
    pub port_results: Vec<PortResult>,
    pub http_ports: Vec<u16>,
}
```

Data flow:
1. `run_port_scan()` → `context.update_ports()` → populates `open_ports`, `port_results`
2. `run_fingerprint()` → `context.update_services()` → populates `services`, `http_ports`
3. `run_endpoint_scan()` → `context.update_endpoints()` → populates `endpoints`
4. Subsequent stages use `context.get_base_url()` to construct target URLs

### Session (`session.rs`)

Manages persistence for resumable pipeline runs via JSON snapshots (`PipelineSession`).
Session checkpoints are written only when output path is explicitly a session-like file name (`*.session` or `*.session.json`) to avoid colliding with report outputs.

### Report (`report.rs`)

`PipelineReport` aggregates results from all stages. Output formats:
- `Display` - Human-readable console output
- `generate_html()` - Styled HTML report (**free function**, not a method)
- `generate_csv()` - CSV report (**free function**, not a method)
- SARIF/JUnit via `output/` module

#### PipelineReport Struct (`report.rs:24-38`)

```rust
pub struct PipelineReport {
    pub target: String,
    pub total_duration_ms: u64,
    pub stage_results: Vec<StageResult>,
    pub open_ports: Vec<PortResult>,
    pub services: Vec<ServiceFingerprint>,
    pub endpoints: Vec<EndpointResult>,
    #[serde(skip)]
    pub checkpoint_error: Option<String>,   // Session save error, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<RunManifest>,      // Run manifest for regression workflows
}
```

**Note**: `generate_html(report: &PipelineReport)` and `generate_csv(report: &PipelineReport)` at `report.rs:113,211` are free functions that take `&PipelineReport` as a parameter, NOT methods on the struct. Call them as `report::generate_html(&report)`.

**Key Field:** `checkpoint_error: Option<String>` at `report.rs:33` - captures any error from session checkpoint saves during pipeline execution. Logged at warn level when set. Skipped during serialization.

## CLI Entry Points (`mod.rs`)

- `run_cli(args, config)` - Standard CLI pipeline execution
- `run_cli_with_callback(args, config, callback)` - Pipeline execution with finding callback (for tool abstraction)
- `resume_cli(args)` - Resume from session checkpoint

## Implemented Defense-Lab Profiles

Five defense-lab profiles are implemented in `cli/mod.rs:262-266` and mapped to stages in `pipeline/stage.rs:92-107`. See `architecture/defense_lab.md` for full semantics.

| Profile | Purpose | Key Constraint |
|---------|---------|----------------|
| `defense-lab` | Comprehensive local/private-scope probe suite | Explicit scope required, no stress/packet defaults |
| `synvoid-local` | Synvoid validation on localhost/container | Loopback or private CIDR only |
| `waf-regression` | WAF evasion-resistance regression testing | Payload classification focus |
| `protocol-edge` | Malformed protocol and edge behavior | Requires `packet-inspection` feature |
| `nse-safe` | Sandboxed NSE scripts (safe/default/version/discovery) | Requires `nse` + `nse-sandbox` features |

## Benefits

- **Automation**: Automate standard pentesting methodologies.
- **Repeatability**: Ensure that complex scans are performed consistently every time.
- **Efficiency**: Reduce manual intervention by automatically triggering the next logical step in a scan.

## Recent Bug Fixes (2026-05-22)

| Issue | Fix |
|-------|-----|
| `resume_cli()` didn't return error on failed stages | Now returns `ScanFailed` error like `run_cli()` |
| `run_load_test()` ignored config, used default TLS settings | Changed to `LoadTestRunner::from_args_with_config()` |
| `PipelineContext.services` used `HashMap` instead of `FxHashMap` | Changed to `FxHashMap` for performance |

## Recent Bug Fixes (2026-05-27)

| Issue | Fix |
|-------|-----|
| `run_cli()` and `run_cli_with_callback()` had duplicated output writing code | Extracted to `write_output()` helper function in `mod.rs:63-95` |
| `StageResult.duration_ms` was serialized to JSON (unnecessary, causes bloat) | Added `#[serde(skip)]` to `duration_ms` field in `executor.rs:21` |
| `StageResult` lacked constructor for cleaner object creation | Added `StageResult::new()` constructor in `executor.rs:27-35` |
| Progress bar created even for empty stage list | Changed condition to `self.tui_mode \|\| self.stages.is_empty()` in `executor.rs:157` |

## Key Files

| File | Purpose |
|------|---------|
| `src/pipeline/mod.rs` | Module entry, public re-exports, CLI entry points |
| `src/pipeline/stage.rs` | `Stage` enum, profiles, aliases, parsing |
| `src/pipeline/executor.rs` | `Pipeline` struct, sequential execution, stage dispatch |
| `src/pipeline/context.rs` | `PipelineContext` for inter-stage data sharing |
| `src/pipeline/session.rs` | `PipelineSession` for pause/resume |
| `src/pipeline/report.rs` | `PipelineReport`, HTML/CSV generation |
| `src/tool/implementations/pipeline.rs` | `PipelineTool` implementing `SecurityTool` |