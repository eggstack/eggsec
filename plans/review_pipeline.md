# Pipeline Module Architecture Review

**Document:** architecture/pipeline.md
**Reviewed:** 2026-06-02
**Accuracy:** Low
**Lines Reviewed:** 177

## Verified Claims

- Stage enum (PortScan, Fingerprint, EndpointScan, Fuzz, LoadTest, Waf, Recon): Verified at `pipeline/stage.rs:6-14`
- Available stages list: Matches `stage.rs:6-14`
- Stage::from_profile() profiles exist: Verified at `pipeline/stage.rs:31-109`
- Stage::from_string() aliases (portscan, fp, endpoint-scan, graphql, oauth, jwt): Verified at `pipeline/stage.rs:111-125`
- Sequential execution linear order: Verified at `pipeline/executor.rs:196`
- run_concurrent() at lines 259-297: Verified at `pipeline/executor.rs:259-297`
- run_concurrent() uses futures::future::join_all(): Verified at `pipeline/executor.rs:278`
- Pipeline struct fields at lines 38-50: Verified at `pipeline/executor.rs:38-50`
- PipelineContext uses FxHashMap for services: Verified at `pipeline/context.rs:12`
- PipelineContext struct fields: Verified at `pipeline/context.rs:8-16`
- PipelineReport struct at lines 24-38: Verified at `pipeline/report.rs:24-38`
- checkpoint_error field with #[serde(skip)]: Verified at `pipeline/report.rs:32-33`
- manifest field with #[serde(skip_serializing_if)]: Verified at `pipeline/report.rs:36-37`
- generate_html() and generate_csv() are free functions: Verified at `pipeline/report.rs:113,211`
- CLI entry points (run_cli, run_cli_with_callback, resume_cli): Verified at `pipeline/mod.rs:183,122,227`
- StageResult duration_ms has #[serde(skip)]: Verified at `pipeline/executor.rs:21-22`
- StageResult::new() constructor: Verified at `pipeline/executor.rs:27-35`
- Progress bar condition (tui_mode || stages.is_empty()): Verified at `pipeline/executor.rs:183`

## Discrepancies

- **Defense-Lab Profiles Stage Counts**: Document at line 136-142 shows stage counts for all 5 defense-lab profiles, but these counts are incorrect:
  - `defense-lab`: Document says 5 stages, but actual is 4 (PortScan→Fingerprint→EndpointScan→Waf→Fuzz vs actual: PortScan→Fingerprint→EndpointScan→Waf) at `stage.rs:92-98`
  - `waf-regression`: Document says 4 stages, but actual is 3 (PortScan→Fingerprint→Waf) at `stage.rs:105`
  - `protocol-edge`: Document says 4 stages, but actual is 2 (PortScan→Fingerprint) at `stage.rs:106`
  - `nse-safe`: Document says 4 stages, but actual is 3 (PortScan→Fingerprint→EndpointScan) at `stage.rs:107`
- **Missing Profile Stages in Table**: The "Available Stages" table at lines 23-34 lists only 11 profiles (quick, endpoint, web, full, waf, api, recon, stealth, deep, vuln, auth), but the defense-lab profiles (defense-lab, synvoid-local, waf-regression, protocol-edge, nse-safe) are NOT listed. The table is incomplete.
- **Defense-Lab Profile Mappings**: Document at line 134 says profiles are "mapped to stages in pipeline/stage.rs:92-107". While mappings exist at those lines, the document's table at lines 136-142 shows incorrect stage counts.

## Bugs Found

- **Stage count mismatch for defense-lab profiles**: The "Recent Bug Fixes" table at lines 150-165 documents fixes from 2026-05-22, but the defense-lab profile stage counts documented at lines 136-142 do not match the actual implementation. This suggests the documentation was not updated after code changes.

## Improvement Opportunities

- **Profile documentation incomplete**: The "Available Stages" table (lines 23-34) is missing the 5 defense-lab profiles. These should be added for completeness.
- **Defense-lab profile stage counts need correction**: The table at lines 136-142 shows incorrect stage counts for defense-lab, waf-regression, protocol-edge, and nse-safe profiles.
- **Recent Bug Fixes table may be stale**: The table at lines 150-165 references 2026-05-22 fixes, but doesn't mention the defense-lab profile stage count issue that exists in the current code.

## Stale Items

- **Defense-Lab Profile Stage Counts**: The discrepancy between documented stage counts and actual implementation at `stage.rs:92-107` indicates either:
  1. The documentation was written before code changes and was not updated, OR
  2. The code was changed without updating documentation
  
  Recommended action: Verify current intended behavior and update documentation to match.

## Code Interrogation Findings

- **Session path extraction**: At `pipeline/executor.rs:117-120`, session path is extracted by checking if output ends with `.session.json` or `.session`. This is correct but the logic could be clearer with a helper method.
- **Concurrent stage execution does not share context**: At `pipeline/executor.rs:259-297`, `run_concurrent()` runs all stages in parallel but each stage execution reads from and writes to the shared `PipelineContext`. However, the context is wrapped in `Arc<Mutex<PipelineContext>>`, so concurrent access is serialized. This is correct but worth noting for performance considerations.
- **Missing stage failure in concurrent mode**: If any stage fails in `run_concurrent()`, the error is stored in `StageResult.error` but the pipeline continues to run all stages. This may not be the intended behavior - consider early termination on critical failures.