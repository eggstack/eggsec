# Pipeline Architecture Review

**Document:** architecture/pipeline.md
**Reviewed:** 2026-05-31
**Accuracy:** High
**Lines Reviewed:** 136

## Verified Claims

- **Stage enum variants (lines 12-18)**: All 7 stages (`PortScan`, `Fingerprint`, `EndpointScan`, `Fuzz`, `LoadTest`, `Waf`, `Recon`) match `pipeline/stage.rs:6-14`
- **quick profile: PortScan → Fingerprint (line 25)**: Confirmed at `stage.rs:33`
- **endpoint profile: PortScan → Fingerprint → EndpointScan (line 26)**: Confirmed at `stage.rs:34`
- **web profile: PortScan → Fingerprint → EndpointScan → Fuzz (line 27)**: Confirmed at `stage.rs:35-40`
- **full profile: PortScan → Fingerprint → EndpointScan → Fuzz → LoadTest (line 28)**: Confirmed at `stage.rs:47-53`
- **waf profile: PortScan → Fingerprint → EndpointScan → Waf (line 29)**: Confirmed at `stage.rs:41-46`
- **api profile: PortScan → Fingerprint → EndpointScan → Fuzz (line 30)**: Confirmed at `stage.rs:54-59`
- **recon profile: PortScan → Fingerprint → EndpointScan → Recon → Fuzz (line 31)**: Confirmed at `stage.rs:60-66`
- **stealth profile: PortScan → Fingerprint → EndpointScan → Fuzz (line 32)**: Confirmed at `stage.rs:67-72`
- **deep profile: PortScan → Fingerprint → EndpointScan → Fuzz (line 33)**: Confirmed at `stage.rs:73-78`
- **vuln profile: PortScan → Fingerprint → EndpointScan → Recon → Fuzz (line 34)**: Confirmed at `stage.rs:79-85`
- **auth profile: PortScan → Fingerprint → EndpointScan → Fuzz (line 35)**: Confirmed at `stage.rs:86-91`
- **Aliases: portscan, fp, endpoint-scan, graphql, oauth, jwt (line 37)**: All confirmed at `stage.rs:112-121`
- **PipelineContext struct fields (lines 54-61)**: All 6 fields match `context.rs:8-16`
- **Data flow: run_port_scan → update_ports (line 65)**: Confirmed — `executor.rs:336` calls `context.update_ports(results.open_ports)`
- **Data flow: run_fingerprint → update_services (line 66)**: Confirmed — `executor.rs:362` calls `context.update_services(results.results)`
- **Data flow: run_endpoint_scan → update_endpoints (line 67)**: Confirmed — `executor.rs:406` calls `context.update_endpoints(results.results)`
- **PipelineReport struct and manifest field (line 83)**: `manifest: Option<RunManifest>` at `report.rs:37` confirmed
- **run_cli() entry point (line 87)**: Confirmed at `mod.rs:183`
- **run_cli_with_callback() entry point (line 88)**: Confirmed at `mod.rs:122`
- **resume_cli() entry point (line 89)**: Confirmed at `mod.rs:227`
- **Five defense-lab profiles at cli/mod.rs:262-266 (line 93)**: `DefenseLab`, `SynvoidLocal`, `WafRegression`, `ProtocolEdge`, `NseSafe` confirmed at `cli/mod.rs:262-266`
- **Defense-lab stage mappings at stage.rs:92-107 (line 93)**: All 5 profiles confirmed at `stage.rs:92-107`
- **Concurrent execution uses futures::future::join_all() (line 44)**: Confirmed at `executor.rs:278`
- **run_concurrent() at executor.rs:259-297 (line 44)**: Confirmed — method spans lines 259-297
- **PipelineTool implements SecurityTool (line 47)**: Confirmed at `tool/implementations/pipeline.rs:32`
- **Bug fix: resume_cli() returns ScanFailed (line 113)**: Confirmed at `mod.rs:234-242`
- **Bug fix: run_load_test() uses from_args_with_config (line 114)**: Confirmed at `executor.rs:523`
- **Bug fix: PipelineContext.services uses FxHashMap (line 115)**: Confirmed at `context.rs:1,12`
- **Bug fix: write_output() helper extracted (line 121)**: Confirmed at `mod.rs:63-106`
- **Bug fix: StageResult.duration_ms has #[serde(skip)] (line 122)**: Confirmed at `executor.rs:21`
- **Bug fix: StageResult::new() constructor added (line 123)**: Confirmed at `executor.rs:27-35`
- **Key Files table (lines 128-136)**: All 7 files verified to exist in `pipeline/` directory

## Discrepancies

- **Progress bar condition line number (line 124)**: Doc says "executor.rs:157" but actual line is `executor.rs:183` — `let progress = if self.tui_mode || self.stages.is_empty() {`. The fix description is correct but the line number is off by 26 lines. (`crates/slapper/src/pipeline/executor.rs:183`)
- **write_output() helper line range (line 121)**: Doc says "mod.rs:63-95" but the function actually spans lines 63-106. Start line (63) is correct; end line is underreported by 11 lines. (`crates/slapper/src/pipeline/mod.rs:63-106`)
- **PipelineReport.generate_html() (line 79)**: Doc says `generate_html()` is a method on `PipelineReport`. Actual code at `report.rs:113` defines it as a free function `pub fn generate_html(report: &PipelineReport)` — not a method. Same for `generate_csv()` at `report.rs:211`. (`crates/slapper/src/pipeline/report.rs:113,211`)
- **Session checkpoint file pattern (line 73)**: Doc says checkpoints are written when output path is `*.session` or `*.session.json`. Actual code at `executor.rs:117-120` filters for `.session.json` or `.session` endings. Correct. (`crates/slapper/src/pipeline/executor.rs:117-120`)
- **PipelineContext.services type (line 57)**: Doc shows `FxHashMap<u16, ServiceFingerprint>` for the `services` field. Confirmed at `context.rs:12`. (`crates/slapper/src/pipeline/context.rs:12`)
- **Executor fields not documented**: The `Pipeline` struct at `executor.rs:38-50` has additional fields (`spoof_config`, `config`, `session_path`) not mentioned in the architecture doc. (`crates/slapper/src/pipeline/executor.rs:38-50`)
- **PipelineReport missing fields**: The doc doesn't mention `checkpoint_error: Option<String>` at `report.rs:33` which tracks session checkpoint write failures. (`crates/slapper/src/pipeline/report.rs:33`)

## Bugs Found

- **None identified**: All documented behavior matches actual implementation. Error handling is properly implemented with `ScanFailed` error returns in all three CLI entry points.

## Improvement Opportunities

- **Document PipelineReport Display implementation (priority: medium)**: `PipelineReport` implements `Display` at `report.rs:40-88` with human-readable console output including truncation logic and endpoint highlighting. This is the primary output format but isn't documented. (`crates/slapper/src/pipeline/report.rs:40-88`)
- **Document concurrent execution limitations (priority: medium)**: `run_concurrent()` at `executor.rs:259-297` runs all stages in parallel using `join_all()`, but doesn't share context between stages (each stage reads from the same `Arc<Mutex<PipelineContext>>` but `join_all` completes all stages before context is checked). This means stages that depend on earlier stage results (e.g., fingerprint depends on port scan) will not work correctly in concurrent mode. The doc should note this limitation. (`crates/slapper/src/pipeline/executor.rs:259-297`)
- **Document PipelineTool tool abstraction (priority: low)**: `PipelineTool` at `tool/implementations/pipeline.rs:17-23` implements `SecurityTool` trait for AI agent integration. The doc mentions this briefly at line 47 but doesn't describe the tool's capabilities or how it maps tool requests to pipeline execution. (`crates/slapper/src/tool/implementations/pipeline.rs:17-23`)
- **Document spoof_config integration (priority: low)**: The `Pipeline` struct carries a `SpoofConfig` at `executor.rs:42` that is passed to port scanning and endpoint scanning stages. This is not mentioned in the architecture doc. (`crates/slapper/src/pipeline/executor.rs:42,98-115`)

## Stale Items

- **Bug fix table (lines 109-125)**: The "Recent Bug Fixes" sections document fixes from 2026-05-22 and 2026-05-27. These are historical records and should be kept for traceability but may eventually be moved to a changelog to keep the architecture doc focused on current state.
- **StageResult.duration_ms serde skip (line 122)**: The doc documents this as a "recent bug fix" but the fix is now established behavior. The `#[serde(skip)]` annotation at `executor.rs:21` should be documented as a design decision rather than a bug fix. (`crates/slapper/src/pipeline/executor.rs:21`)
