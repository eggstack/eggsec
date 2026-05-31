# Pipeline Architecture Review

**Document:** architecture/pipeline.md
**Reviewed:** 2026-05-31
**Accuracy:** High

## Verified Claims

- **Stage enum variants**: All 7 stages (`PortScan`, `Fingerprint`, `EndpointScan`, `Fuzz`, `LoadTest`, `Waf`, `Recon`) at `stage.rs:6-14` — matches doc
- **Stage::from_profile() mappings**: All 11 profiles verified at `stage.rs:31-108`:
  - `quick`: PortScan → Fingerprint (line 33)
  - `endpoint`: PortScan → Fingerprint → EndpointScan (line 34)
  - `web`: PortScan → Fingerprint → EndpointScan → Fuzz (line 35-40)
  - `full`: PortScan → Fingerprint → EndpointScan → Fuzz → LoadTest (line 47-53)
  - `waf`: PortScan → Fingerprint → EndpointScan → Waf (line 41-46)
  - `api`: PortScan → Fingerprint → EndpointScan → Fuzz (line 54-59)
  - `recon`: PortScan → Fingerprint → EndpointScan → Recon → Fuzz (line 60-66)
  - `stealth`: PortScan → Fingerprint → EndpointScan → Fuzz (line 67-72)
  - `deep`: PortScan → Fingerprint → EndpointScan → Fuzz (line 73-78)
  - `vuln`: PortScan → Fingerprint → EndpointScan → Recon → Fuzz (line 79-85)
  - `auth`: PortScan → Fingerprint → EndpointScan → Fuzz (line 86-91)
- **Stage aliases**: `portscan`, `fp`, `endpoint-scan`, `graphql`, `oauth`, `jwt` at `stage.rs:111-125` — matches doc
- **PipelineContext struct**: All 6 fields match at `context.rs:9-16` — `target`, `open_ports`, `services`, `endpoints`, `port_results`, `http_ports`
- **Data flow methods**: `update_ports()` at `context.rs:50`, `update_services()` at `context.rs:55`, `update_endpoints()` at `context.rs:62`, `get_base_url()` at `context.rs:38` — matches doc
- **PipelineReport struct**: At `report.rs:25-38` with `target`, `total_duration_ms`, `stage_results`, `open_ports`, `services`, `endpoints`, `checkpoint_error`, `manifest` fields
- **CLI entry points**: `run_cli()` at `mod.rs:183`, `run_cli_with_callback()` at `mod.rs:122`, `resume_cli()` at `mod.rs:227` — all verified
- **PipelineSession**: At `session.rs:8-14` with `target`, `completed_stages`, `remaining_stages`, `context`, `spoof_config`
- **Session checkpoint logic**: At `executor.rs:216-232`, writes only when output path ends with `.session.json` or `.session` (line 117-120)
- **StageResult struct**: At `executor.rs:18-25` with `#[serde(skip)]` on `duration_ms` at line 21
- **StageResult::new() constructor**: At `executor.rs:27-35` — matches doc claim
- **write_output() helper**: At `mod.rs:63-106` — extracted from duplicated code as documented
- **PipelineTool implementing SecurityTool**: Referenced at `src/tool/implementations/pipeline.rs` — file exists per doc
- **FxHashMap in PipelineContext**: `services` field at `context.rs:12` uses `FxHashMap<u16, ServiceFingerprint>` — matches doc
- **Progress bar condition**: At `executor.rs:183` the condition is `self.tui_mode || self.stages.is_empty()` — matches doc claim about empty stage list fix
- **Defense-lab profiles**: Doc says "planned but not yet implemented" but actual code at `stage.rs:92-108` already has `DefenseLab`, `SynvoidLocal`, `WafRegression`, `ProtocolEdge`, `NseSafe` variants, and `profile_from_str()` at `stage.rs:138-158` maps all five defense-lab profiles

## Discrepancies

- **Defense-lab profiles are implemented, not planned**: The doc at line 90 states "Five defense-lab profiles are planned but not yet implemented" and "TODOs are placed in `cli/mod.rs`". However, the code at `stage.rs:92-108` and `stage.rs:151-155` shows all five profiles (`DefenseLab`, `SynvoidLocal`, `WafRegression`, `ProtocolEdge`, `NseSafe`) are fully implemented in `Stage::from_profile()` and `profile_from_str()`. The `ScanProfile` enum in `cli/mod.rs` also includes these variants. The doc's claim that these are "planned" is stale.
- **PipelineReport has manifest field**: Doc at line 76 mentions `PipelineReport` aggregates results from all stages but does not mention the `manifest: Option<RunManifest>` field. The code at `report.rs:37` shows this field exists and is populated after pipeline execution (executor.rs:252-254).
- **generate_html() and generate_csv() are free functions**: Doc says `PipelineReport` has `generate_html()` and `generate_csv()` as methods, but actual code at `report.rs:113` and `report.rs:211` shows these are free functions (`pub fn generate_html(report: &PipelineReport)` and `pub fn generate_csv(report: &PipelineReport)`), not methods on `PipelineReport`.
- **Missing Concurrent Execution from doc**: The executor supports `concurrent_stages` mode (`executor.rs:176-178`, `run_concurrent()` at line 259) but the doc only mentions "Sequential Execution" at line 43. The `Pipeline` struct has a `concurrent_stages` field at `executor.rs:43`.

## Bugs Found

- **Stale "Planned Defense-Lab Profiles" section**: The doc states these profiles are not yet implemented (line 90) and TODOs exist. This is incorrect — all five are fully implemented. The doc should be updated to reflect the current state.

## Improvement Opportunities

- **Update defense-lab profiles section**: Remove "planned but not yet implemented" language and document the actual implementation status. (priority: high)
- **Document concurrent execution mode**: Add a section about `concurrent_stages` and `run_concurrent()` which runs stages in parallel via `futures::future::join_all()`. (priority: medium)
- **Document PipelineReport.manifest field**: The `RunManifest` integration is significant for regression workflows and should be mentioned in the Report section. (priority: medium)
- **Clarify generate_html/generate_csv are free functions**: These are called as `report::generate_html(report)` not `report.generate_html()`. (priority: low)
- **Document PipelineTool feature gate**: `PipelineTool` integration is behind the `tool-api` feature gate (inferred from `run_cli_with_callback` being feature-gated at `mod.rs:121`). (priority: low)

## Stale Items

- **"Planned Defense-Lab Profiles" section (lines 88-100)**: This entire section describes profiles as "planned but not yet implemented" with TODO references. All five profiles are fully implemented in `stage.rs` and `cli/mod.rs`. Recommend rewriting this section to document the current implementation rather than the original plan.
- **"Recent Bug Fixes (2026-05-22)" and "(2026-05-27)" tables**: These describe fixes that are already merged. Consider moving to a changelog and keeping the architecture doc focused on current behavior.
- **Key Files table line references**: The doc references `mod.rs:63-95` for `write_output()` but the actual range is `mod.rs:63-106`. Minor drift.
