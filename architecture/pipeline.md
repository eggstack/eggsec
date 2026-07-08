# Pipeline Module

The Pipeline module allows for the orchestration of complex security assessment workflows by chaining multiple Eggsec tasks together.

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
- `Vuln` - Vulnerability assessment with CVSS scoring and asset criticality
- `DbPentest` - Direct database security assessment (Postgres/MySQL/MSSQL/MongoDB/Redis; feature-gated behind `db-pentest`)
- `WebProxy` - Web proxy intercept and traffic analysis (feature-gated behind `web-proxy`)

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
| `vuln` | PortScan → Fingerprint → Vuln → EndpointScan → Recon → Fuzz |
| `auth` | PortScan → Fingerprint → EndpointScan → Fuzz | (JWT/OAuth/IDOR-focused fuzzing via fuzzer payloads; distinct from CLI `auth-test` which uses `auth/` module for credential/brute/MFA control validation) |
| `defense-lab` | PortScan → Fingerprint → EndpointScan → Waf → Fuzz |
| `synvoid-local` | PortScan → Fingerprint → EndpointScan → Waf |
| `waf-regression` | PortScan → Fingerprint → Waf |
| `protocol-edge` | PortScan → Fingerprint |
| `nse-safe` | PortScan → Fingerprint → EndpointScan |
| `web-proxy` | PortScan → Fingerprint → EndpointScan → Fuzz | (Web proxy interception via `Stage::WebProxy`; requires `web-proxy` feature) |
| `db-regression` | `Stage::DbPentest` (when `db-pentest` feature enabled); falls back to `PortScan → Fingerprint → EndpointScan → Waf → Fuzz` when feature absent |

**Aliases**: User-facing aliases such as `portscan`, `fp`, `endpoint-scan`, `graphql`, `oauth`, `jwt`, `fuzzing`, `fuzzer`, `loadtest`, `load-test`, `vulnerability`, `vuln-assess`, `proxy`, `webproxy`, `intercept` are normalized into canonical stages via `Stage::from_string()`.

**Methods**:
- `to_probe_intent()` — Maps stage to `ProbeIntent` category (Discovery, Fingerprint, ServiceValidation, EvasionResistance, LoadBearing, WafEvaluation)
- `to_probe_risk()` — Maps stage to minimum required `ProbeRisk` level (Passive, SafeActive, Intrusive, Stress)

**Constants**:
- `DEFAULT_SCAN_PORTS` — `"80,443"`
- `EXTENDED_SCAN_PORTS` — `"21,22,23,25,53,80,110,143,443,445,993,995,1433,1521,3306,3389,5432,5900,6379,8080,8443,27017,9092,9200,5672,2181,2375,2376,6443,10250,3000,5000,8000,9000,4200,5601,9090"`

**Functions**:
- `profile_from_str(s)` — Parse a profile name string into `ScanProfile` variant
- `parse_stages(s)` — Parse comma-separated stage names into `Vec<Stage>`

### Executor (`executor.rs`)

The `executor.rs` file is responsible for running the pipeline from start to finish.

- **Sequential Execution**: Stages run in linear order (`for stage in &self.stages`).
- **Concurrent Execution**: `run_concurrent()` method at `executor.rs:380-474` runs stages in dependency waves using `futures::future::join_all()` within each wave.
- **Result Passing**: Output from one stage (for example open ports and detected HTTP services) is persisted into `PipelineContext` and consumed by later stages.
- **Failure Recording**: Stage errors are recorded per stage in `StageResult` and surfaced in the report. CLI entrypoints return `ScanFailed` if any stage failed.
- **Tool Integration**: `PipelineTool` implements `SecurityTool` for AI agent tool registry.

#### Pipeline Struct Fields (`executor.rs:39-52`)

```rust
pub struct Pipeline {
    target: String,
    stages: Vec<Stage>,
    profile: ScanProfile,
    risk_budget: ProbeRisk,
    concurrency: usize,
    concurrent_stages: bool,
    common: CommonHttpArgs,
    spoof_config: SpoofConfig,        // IP spoofing, decoy, fragment, scan type options
    context: Arc<Mutex<PipelineContext>>,
    session_path: Option<String>,     // Path for session checkpoint persistence (*.session/*.session.json)
    tui_mode: bool,
    config: Option<EggsecConfig>,    // Optional config for TLS, concurrency, default settings
}
```

#### PipelineProxyFinding (`executor.rs:19-28`)

Feature-gated on `web-proxy`. Simple finding type for pipeline proxy stage results:
- `title: String`
- `description: String`
- `severity: String`
- `category: String`
- `location: String`

- `spoof_config` (`SpoofConfig`): Configures source IP spoofing, decoy addresses, fragmentation, scan type, packet trace, max rate, and TTL. Built from CLI args via `SpoofConfig::from_args()`.
- `config` (`Option<EggsecConfig>`): Optional loaded config file. Used to read `http.verify_tls`, `http.timeout_secs`, `scan.default_concurrency`, and other settings. When `None`, defaults are used.
- `session_path` (`Option<String>`): Extracted from `--output` arg when the path ends with `.session` or `.session.json`. When set, a `PipelineSession` checkpoint is written after each stage completes.

#### Pipeline Methods

- `from_session(session)` — Reconstruct pipeline from a `PipelineSession` checkpoint
- `with_spoof_config(spoof_config)` — Set IP spoofing/decoy/fragment configuration
- `with_config(config)` — Set `EggsecConfig` for TLS, concurrency, defaults
- `add_stage(stage)` — Append a stage to the pipeline
- `with_concurrency(concurrency)` — Set concurrent request count
- `with_concurrent_stages(enabled)` — Enable/disable concurrent stage execution
- `has_stages()` — Returns `true` if pipeline has stages
- `get_stages()` — Returns `&[Stage]`
- `dependency_waves()` — Partition stages into dependency waves for concurrent execution
- `validate_defense_lab_scope()` — Validate defense-lab profiles target private/loopback only
- `validate_feature_gates()` — Validate required compile-time features are enabled
- `validate_stage_risk(stage)` — Check if a stage's risk level fits within the profile's budget

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
    pub vuln_assessment: Option<VulnAssessment>,
    pub load_test_results: Option<LoadTestResults>,
    #[cfg(feature = "web-proxy")]
    pub web_proxy_report: Option<crate::proxy::intercept::types::WebProxySessionReport>,
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

#### PipelineSession Fields

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

### Report (`report.rs`)

`PipelineReport` aggregates results from all stages. Output formats:
- `Display` - Human-readable console output
- `generate_html()` - Styled HTML report (**free function**, not a method)
- `generate_csv()` - CSV report (**free function**, not a method)
- `generate_markdown()` - Markdown report (**free function**, not a method)
- SARIF/JUnit via `output/` module

#### PipelineReport Struct (`report.rs:13-31`)

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vuln_assessment: Option<VulnAssessment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_test_results: Option<LoadTestResults>,
    #[cfg(feature = "web-proxy")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_proxy_report: Option<crate::proxy::intercept::types::WebProxySessionReport>,
}
```

**Note**: `generate_html(report: &PipelineReport)` at `report.rs:127` and `generate_csv(report: &PipelineReport)` at `report.rs:259` are free functions that take `&PipelineReport` as a parameter, NOT methods on the struct. Call them as `report::generate_html(&report)`.

**Key Field:** `checkpoint_error: Option<String>` at `report.rs:22` - captures any error from session checkpoint saves during pipeline execution. Logged at warn level when set. Skipped during serialization.

## CLI Entry Points (`mod.rs`)

- `run_cli(args, config)` - Standard CLI pipeline execution
- `run_cli_with_callback(args, config, callback)` - Pipeline execution with finding callback (for tool abstraction); feature-gated on `tool-api`
- `resume_cli(args, config)` - Resume from session checkpoint

#### `write_output(report, output_path, format)` (`mod.rs:64-119`)

Writes pipeline report to the specified path in the given format (HTML, JSON, CSV, Markdown, SARIF, JUnit). Handles manifest file writing for regression workflows.

## Implemented Defense-Lab Profiles

Five defense-lab profiles are implemented in `cli/mod.rs:262-266` and mapped to stages in `pipeline/stage.rs:92-107`. See `architecture/defense_lab.md` for full semantics.

| Profile | Purpose | Key Constraint |
|---------|---------|----------------|
| `defense-lab` | Comprehensive local/private-scope probe suite | Explicit scope required, no stress/packet defaults |
| `synvoid-local` | Synvoid validation on localhost/container | Loopback or private CIDR only |
| `waf-regression` | WAF evasion-resistance regression testing | Payload classification focus |
| `protocol-edge` | Malformed protocol and edge behavior | Requires `packet-inspection` feature |
| `nse-safe` | Sandboxed NSE scripts (safe/default/version/discovery) | Requires `nse` + `nse-sandbox` features |
| `db-regression` | Defense-lab family for db-pentest regression; native `Stage::DbPentest` (Phase 4) when `db-pentest` feature enabled (falls back to defense-lab stages) | `db-pentest` feature |

## Benefits

- **Automation**: Automate standard pentesting methodologies.
- **Repeatability**: Ensure that complex scans are performed consistently every time.
- **Efficiency**: Reduce manual intervention by automatically triggering the next logical step in a scan.

## Recent Bug Fixes (2026-06-03)

| Issue | Fix |
|-------|-----|
| `Stage::Vuln` was a no-op (`Ok(())`) | Implemented `run_vuln()` with CVSS scoring, asset criticality, and finding prioritization |
| `run_concurrent()` skipped session checkpointing | Added session save after concurrent execution completion |
| `PipelineContext` lacked `vuln_assessment` field | Added `vuln_assessment: Option<VulnAssessment>` for inter-stage data sharing |

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