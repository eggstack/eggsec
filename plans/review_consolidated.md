# Architecture Review - Consolidated High-Priority Findings

**Generated:** 2026-06-02
**Source:** 43 review files from `plans/review_*.md`
**Purpose:** Extract all HIGH priority items for immediate action

---

## Critical Bugs Needing Immediate Fix

### 1. Defense-Lab Profile Stage Counts Incorrect
- **Module/Document:** `architecture/pipeline.md` / `pipeline` module
- **Issue:** Stage counts documented at lines 136-142 are incorrect for all 5 defense-lab profiles:
  - `defense-lab`: Says 5 stages, actual is 4 (PortScan→Fingerprint→EndpointScan→Waf vs actual without Fuzz)
  - `waf-regression`: Says 4 stages, actual is 3 (PortScan→Fingerprint→Waf)
  - `protocol-edge`: Says 4 stages, actual is 2 (PortScan→Fingerprint)
  - `nse-safe`: Says 4 stages, actual is 3 (PortScan→Fingerprint→EndpointScan)
- **File Reference:** `pipeline/stage.rs:92-107`
- **Recommended Fix:** Update the stage count table at `architecture/pipeline.md:136-142` to match actual implementation

### 2. Storage Module - Stub Implementation Misleading
- **Module/Document:** `architecture/storage.md` / `storage` module
- **Issue:** Documents a "SQLx-based persistence layer" but `Database` at `storage/postgres.rs:6-7` is explicitly a stub returning empty results
- **File Reference:** `plans/review_storage.md:21`
- **Recommended Fix:** Update documentation to clearly state this is a stub/not yet implemented, or implement actual SQLx integration

### 3. SBOM Generation - No CVE Lookup
- **Module/Document:** `architecture/supply_chain.md` / `supply_chain` module
- **Issue:** `SbomReport.vulnerabilities` field exists but all SBOM generators (`generate_from_cargo()`, `generate_from_npm()`, etc.) return empty vectors - no actual CVE lookup implemented
- **File Reference:** `supply_chain/sbom.rs`
- **Reference:** `plans/review_supply_chain.md:24`
- **Recommended Fix:** Either implement actual CVE/NVD lookup integration or document this as a planned feature

### 4. VulnAssessment is a Non-Functional Stub
- **Module/Document:** `architecture/vuln.md` / `vuln` module
- **Issue:** `VulnAssessment` struct at `vuln/mod.rs:37-40` only has `mode: String`, `results: Vec<String>`, `assessed_at: DateTime` - cannot hold structured findings. Any pipeline integration expecting structured vulnerability data will fail
- **File Reference:** `vuln/mod.rs:37-40`
- **Reference:** `plans/review_vuln.md:26`
- **Recommended Fix:** Either implement proper VulnAssessment struct with structured findings or update documentation to clarify this is a placeholder

### 5. Docker Scanner Shell Injection Risk
- **Module/Document:** `container` module
- **Issue:** `docker.rs:208-209` uses `std::process::Command::new("docker")` with `args(["inspect", _image_name])`. If `_image_name` contains special characters, this could lead to command injection
- **File Reference:** `container/docker.rs:208-209`
- **Reference:** `plans/review_container.md:35`
- **Recommended Fix:** Validate image names to reject special characters before passing to shell

---

## Documentation Accuracy Issues

### 6. k8s-openapi Issue Now Resolved (Stale Warning)
- **Module/Document:** `architecture/feature_matrix.md`
- **Issue:** Lines 58-63 describe `full` feature failing to compile due to k8s-openapi feature requirement, but `Cargo.toml:189` now includes `features = ["v1_30"]` directly on the dependency
- **File Reference:** `Cargo.toml:186-189`
- **Reference:** `plans/review_feature_matrix.md:52-53`
- **Recommended Fix:** Update documentation to indicate this issue has been resolved and remove the stale warning

### 7. Recon Module Count Mismatch
- **Module/Document:** `architecture/recon.md`
- **Issue:** Text at lines 87-93 says `run_full_recon()` executes 17 modules, but actual `FULL_RECON_PIPELINE_MODULES` at `recon/mod.rs:350-368` has 18 entries (cloud module added)
- **File Reference:** `recon/mod.rs:350-368`
- **Reference:** `plans/review_recon.md:53-59`
- **Recommended Fix:** Update text to say 18 modules and ensure the cloud module is properly documented as feature-gated

### 8. Error Module Line Reference Wrong
- **Module/Document:** `architecture/error.md`
- **Issue:** From implementations table at line 56 says `Io` variant is at `mod.rs:56`, but actual location is `mod.rs:82`
- **File Reference:** `error/mod.rs:82`
- **Reference:** `plans/review_error.md:20`
- **Recommended Fix:** Update line reference from `mod.rs:56` to `mod.rs:82`

### 9. AI Agents File Path Missing Prefix
- **Module/Document:** `architecture/ai_agents.md`
- **Issue:** Documents fixes at `alerts/routing.rs` but actual path is `agent/alerts/routing.rs`
- **File Reference:** `agent/alerts/routing.rs`
- **Reference:** `plans/review_ai_agents.md:61`
- **Recommended Fix:** Add `agent/` prefix to all file paths in bug fix section

### 10. StressConfig Field Names Don't Match Code
- **Module/Document:** `architecture/stress.md`
- **Issue:** Documents `rate_limit` field but actual is `rate_pps`; documents `threads` but actual is `concurrency`
- **File Reference:** `stress/mod.rs:52,54`
- **Reference:** `plans/review_stress.md:16-17`
- **Recommended Fix:** Update field names in documentation to match actual implementation

---

## Performance Concerns

### 11. Semaphore Acquire Uses Unwrap (Panic Risk)
- **Module/Document:** `loadtest` module
- **Issue:** At `loadtest/runner.rs:315`, semaphore acquire uses `.unwrap()` which could panic if semaphore is closed
- **File Reference:** `loadtest/runner.rs:315`
- **Reference:** `plans/review_loadtest.md:30-36`
- **Recommended Fix:** Handle the error explicitly instead of unwrapping

### 12. PcapWriter Write Result Silently Dropped
- **Module/Document:** `packet` module / `networking.md`
- **Issue:** At `packet/capture.rs:209`, the PcapWriter `write_packet` result is silently dropped with `let _ =`
- **File Reference:** `packet/capture.rs:209`
- **Reference:** `plans/review_networking.md:50-56`
- **Recommended Fix:** Log warning on write failure instead of silent suppression

### 13. Silent Error Suppression in Notify Module
- **Module/Document:** `notify` module
- **Issue:** Multiple locations use `let _ = notifier.notify()` pattern that silently ignores notification failures: `notify/mod.rs:114, 140-143, 219-222, 293-296`
- **File Reference:** `notify/mod.rs:114` and multiple other locations
- **Reference:** `plans/review_notify.md:25`
- **Recommended Fix:** Use `tracing::warn!` or similar to log failures instead of silent suppression

### 14. DEFAULT_ENDPOINTS Static Array (Binary Size)
- **Module/Document:** `scanner` module
- **Issue:** At `scanner/endpoints.rs:34`, `DEFAULT_ENDPOINTS` is a static array meaning all 261 endpoints are always compiled into binary even if unused
- **File Reference:** `scanner/endpoints.rs:34`
- **Reference:** `plans/review_scanner.md:56`
- **Recommended Fix:** Consider making this lazy-loaded from a config file for binary size optimization

---

## Security Concerns

### 15. Finding Fingerprint Uses Non-Cryptographic Hash
- **Module/Document:** `findings` module
- **Issue:** `Finding::compute_fingerprint()` at `findings/mod.rs:300-326` uses `DefaultHasher` (SipHash) instead of a cryptographically secure hash. For security-sensitive deduplication, SHA-256 would be more appropriate
- **File Reference:** `findings/mod.rs:300-326`
- **Reference:** `plans/review_findings.md:63`
- **Recommended Fix:** Consider using SHA-256 or another secure hash for fingerprint computation

### 16. CSV Escape in Pipeline Module Lacks NFKC
- **Module/Document:** `output` module
- **Issue:** `pipeline/report.rs:10-22` has its own `escape_csv()` that does NOT use NFKC normalization, unlike `output/escape.rs:16-35` which does. CSV export from pipeline may be vulnerable to formula injection
- **File Reference:** `pipeline/report.rs:10-22`, `output/escape.rs:16-35`
- **Reference:** `plans/review_output.md:55`
- **Recommended Fix:** Use the NFKC-normalized `escape_csv()` from output module in pipeline

### 17. From<anyhow::Error> Lossy Conversion
- **Module/Document:** `error` module
- **Issue:** `From<anyhow::Error>` impl maps all anyhow errors to `RequestFailed` variant with method="UNKNOWN" and url="unknown", losing context that could aid debugging
- **File Reference:** `error/mod.rs:172-200`
- **Reference:** `plans/review_error.md:33`
- **Recommended Fix:** Consider logging or preserving more context from the anyhow error chain

---

## Code Quality Issues

### 18. Pipeline Available Stages Table Incomplete
- **Module/Document:** `architecture/pipeline.md`
- **Issue:** "Available Stages" table at lines 23-34 lists only 11 profiles (quick, endpoint, web, full, waf, api, recon, stealth, deep, vuln, auth) but is missing all 5 defense-lab profiles (defense-lab, synvoid-local, waf-regression, protocol-edge, nse-safe)
- **File Reference:** `pipeline/stage.rs:92-107`
- **Reference:** `plans/review_pipeline.md:36`
- **Recommended Fix:** Add all 5 defense-lab profiles to the Available Stages table

### 19. Utils Module Table Missing Serialization
- **Module/Document:** `architecture/utils.md`
- **Issue:** Subtitle says "23 submodules" and table should list 23 but only shows 21. `serialization` module exists in `utils/` but is not in the table
- **File Reference:** `utils/serialization.rs`
- **Reference:** `plans/review_utils.md:12-13`
- **Recommended Fix:** Update table to include all 23 modules including `serialization`

### 20. Logging Module Missing Macro Documentation
- **Module/Document:** `architecture/logging.md`
- **Issue:** `init.rs` defines 4 macros (`log_request!`, `log_scan_progress!`, `log_finding!`, `log_error_context!`) that are not documented
- **File Reference:** `logging/init.rs:83-131`
- **Reference:** `plans/review_logging.md:17`
- **Recommended Fix:** Document these 4 macros or note them as internal

### 21. Fuzzer Endpoint Silent Error Continue
- **Module/Document:** `fuzzer` module
- **Issue:** `fuzz_endpoint` at `fuzzer/api_schema/mod.rs:291-306` silently continues on request failure without proper error propagation - error details lost if tracing::debug not enabled
- **File Reference:** `fuzzer/api_schema/mod.rs:291-306`
- **Reference:** `plans/review_fuzzer.md:35`
- **Recommended Fix:** Log at warn level or add failed requests to a counter

### 22. Kubernetes Scanner Silent Failures
- **Module/Document:** `container` module
- **Issue:** API calls use `.ok()` on results at `kubernetes.rs:65, 104, 163, 195, 254`, silently ignoring network errors and returning empty results
- **File Reference:** `container/kubernetes.rs:65, 104, 163, 195, 254`
- **Reference:** `plans/review_container.md:36`
- **Recommended Fix:** Log network errors instead of silently ignoring them

### 23. Command Count Imprecision
- **Module/Document:** `architecture/overview.md`, `architecture/cli_commands.md`
- **Issue:** `overview.md:156` says "37+" and `cli_commands.md:9` says "35+" but actual count is ~29 without features, ~40 with all features
- **File Reference:** `cli/mod.rs:81-201`
- **Reference:** `plans/review_overview.md:40`, `plans/review_cli_commands.md:21-24`
- **Recommended Fix:** Clarify base command count vs. feature-gated count in documentation

---

## Summary

| Category | Count |
|----------|-------|
| Critical Bugs | 5 |
| Documentation Accuracy | 5 |
| Performance Concerns | 4 |
| Security Concerns | 3 |
| Code Quality Issues | 6 |
| **Total HIGH Priority Items** | **23** |

### Top 5 Immediate Actions

1. **Fix defense-lab stage counts** in `architecture/pipeline.md:136-142`
2. **Document storage as stub** or implement actual SQLx integration
3. **Implement SBOM CVE lookup** or document as planned feature
4. **Implement proper VulnAssessment** or document as placeholder
5. **Fix Docker shell injection** by validating image names