# Stale Items Report

**Generated:** 2026-06-02
**Source:** All 43 review files from `plans/review_*.md`
**Purpose:** Document orphaned/missing documents, deprecated content, and cross-reference inconsistencies.

---

## 1. Orphaned Documents

Documents referencing non-existent modules or unimplemented features:

### 1.1 Storage Module - Stub Implementation
- **Document:** `architecture/storage.md`
- **Issue:** Documents a "SQLx-based persistence layer" but `Database` at `storage/postgres.rs:6-7` is explicitly a stub implementation with no actual database connection
- **Reference:** `plans/review_storage.md:21`
- **Recommended Action:** Update documentation to clearly state this is a stub/not yet implemented

### 1.2 Supply Chain - No CVE Lookup
- **Document:** `architecture/supply_chain.md`
- **Issue:** `SbomReport` has `vulnerabilities: Vec<SbomVulnerability>` field but all SBOM generators return empty vectors - no actual CVE lookup implemented
- **Reference:** `plans/review_supply_chain.md:24`
- **Recommended Action:** Document this limitation explicitly or implement CVE lookup

### 1.3 Vuln Module - Stub Assessment
- **Document:** `architecture/vuln.md`
- **Issue:** `VulnAssessment` struct at `vuln/mod.rs:37-40` only has `mode`, `results`, `assessed_at` - cannot hold structured findings
- **Reference:** `plans/review_vuln.md:26`
- **Recommended Action:** Either implement proper VulnAssessment or update docs to reflect placeholder status

---

## 2. Missing Documents

Modules that exist but have no architecture doc:

### 2.1 Agent Module
- **Module:** `crates/slapper/src/agent/`
- **Status:** Module exists with submodules (mod.rs, memory.rs, portfolio.rs, skills.rs, constraints/)
- **Reference:** `plans/review_ai_agents.md` partially covers it in "Agent Module" section
- **Recommended Action:** Consider if a dedicated `architecture/agent.md` is needed or if current coverage in `ai_agents.md` is sufficient

### 2.2 Tool Module
- **Module:** `crates/slapper/src/tool/`
- **Status:** Large module (6 subdirectories) with protocol, registry, traits
- **Recommended Action:** Document coverage scattered across `ai_agents.md` (MCP integration) - may need dedicated doc

---

## 3. Deprecated Content Across Docs

### 3.1 Historical Bug Fix Tables (Stale)
| Document | Lines | Issue |
|----------|-------|-------|
| `tui.md` | 886-1849 | 800+ lines of historical bug fixes from sessions 2026-05-30 to 2026-06-10 - many already fixed |
| `scanner.md` | 60-79 | Bug fixes from 2026-05-22 and 2026-05-27 - verified but historical |
| `networking.md` | 30-42 | Bug fixes from 2024 - verified fixed |

**Recommended Action:** Move historical fix logs to separate `ARCHITECTURE_<MODULE>_HISTORY.md` files to keep main docs focused on current state

### 3.2 k8s-openapi Issue (Resolved)
- **Document:** `architecture/feature_matrix.md`
- **Issue:** Lines 58-63 describe `full` feature failing to compile due to k8s-openapi, but `Cargo.toml:189` now includes `features = ["v8_30"]` directly
- **Reference:** `plans/review_feature_matrix.md:52-53`
- **Recommended Action:** Update documentation to reflect this is now resolved

### 3.3 Bug Patterns Section
- **Document:** `architecture/tui.md:327-884`
- **Issue:** Documents patterns that have been fixed - may be outdated now that they're caught by lints
- **Reference:** `plans/review_tui.md:110-112`
- **Recommended Action:** Verify which patterns are still relevant vs. now-enforced via lints

---

## 4. Cross-Reference Inconsistencies

### 4.1 Command Count Discrepancy
| Document | Claim | Actual |
|----------|-------|--------|
| `overview.md:156` | "37+" | ~29 without features, ~40 with all |
| `cli_commands.md:9` | "35+" | ~29 without features, ~40 with all |

### 4.2 Module Count Discrepancy
- **Document:** `architecture/recon.md`
- **Issue:** Text says 17 modules but actual `FULL_RECON_PIPELINE_MODULES` at `recon/mod.rs:350-368` has 18 entries
- **Reference:** `plans/review_recon.md:53-59`

### 4.3 Defense-Lab Profile Stage Counts
- **Document:** `architecture/pipeline.md:136-142`
- **Issue:** Stage counts are incorrect for all 5 defense-lab profiles:
  - `defense-lab`: Doc says 5 stages, actual is 4
  - `waf-regression`: Doc says 4 stages, actual is 3
  - `protocol-edge`: Doc says 4 stages, actual is 2
  - `nse-safe`: Doc says 4 stages, actual is 3
- **Reference:** `plans/review_pipeline.md:31-36`

### 4.4 Utils Module Count
- **Document:** `architecture/utils.md`
- **Issue:** Subtitle says "23 submodules" but table lists only 21; `serialization` module missing from table
- **Reference:** `plans/review_utils.md:12-13`

### 4.5 Error Module Line Reference
- **Document:** `architecture/error.md`
- **Issue:** From table says `Io` variant at `mod.rs:56` but actual is `mod.rs:82`
- **Reference:** `plans/review_error.md:20`

### 4.6 AI Agents File Paths
- **Document:** `architecture/ai_agents.md`
- **Issue:** Says `alerts/routing.rs` but actual path is `agent/alerts/routing.rs`
- **Reference:** `plans/review_ai_agents.md:61`

### 4.7 StressConfig Field Names
- **Document:** `architecture/stress.md`
- **Issue:** Documents `rate_limit` but actual field is `rate_pps`; documents `threads` but actual is `concurrency`
- **Reference:** `plans/review_stress.md:16-17`

---

## 5. Incomplete Documentation

### 5.1 Missing Fields from Docs
| Document | Missing Fields | Reference |
|----------|----------------|-----------|
| `stress.md` | `spoof_range`, `random_source_port`, `payload_size`, `use_proxies`, `proxy_pool` | `plans/review_stress.md:18` |
| `pipeline.md` | 5 defense-lab profiles not in "Available Stages" table | `plans/review_pipeline.md:36` |
| `utils.md` | `serialization` module not in table | `plans/review_utils.md:13` |
| `logging.md` | 4 macros not documented (`log_request!`, `log_scan_progress!`, `log_finding!`, `log_error_context!`) | `plans/review_logging.md:17` |
| `constants.md` | `BLOCKED_PATTERNS`, `WEAK_BLOCK_INDICATOR_PATTERNS`, `UNKNOWN_WAF_WEAK_PATTERN_THRESHOLD` | `plans/review_constants.md:37` |

---

## 6. Stale Statistics

### 6.1 Test Count
- **Document:** `architecture/overview.md:581`
- **Claim:** "1324 base, 1469+ with full features"
- **Issue:** Test counts may have changed since documentation
- **Recommended Action:** Verify with actual test run

### 6.2 CSV Formula Injection Test Coverage
- **Document:** `architecture/output.md`
- **Claim:** Test at `output/escape.rs:42-49` verifies formula injection
- **Issue:** Only tests fullwidth character bypass, not primary formula injection vectors (=, +, -, @)
- **Reference:** `plans/review_output.md:51`

---

## 7. Silent Error Suppression Patterns

These patterns use `let _ =` which silently ignores errors:

| File | Line | Issue | Priority |
|------|------|-------|----------|
| `notify/mod.rs` | 114, 140-143, 219-222, 293-296 | `let _ = notifier.notify()` | Medium |
| `loadtest/runner.rs` | 315 | `sem.acquire().await.unwrap()` | Medium |
| `networking/capture.rs` | 209 | `let _ = writer.write_packet()` | Low |
| `fuzzer/api_schema/mod.rs` | 291-306 | Silent continue on request failure | Medium |
| `scanner/endpoints.rs` | 768 | Silent error suppression | Low |

---

## 8. Recommended Actions Summary

| Priority | Item | Module/Doc | Action |
|----------|------|------------|--------|
| HIGH | Defense-Lab stage counts wrong | pipeline.md | Fix stage counts in table |
| HIGH | Storage is stub | storage.md | Document as stub/not implemented |
| HIGH | SBOM no CVE lookup | supply_chain.md | Document limitation or implement |
| HIGH | VulnAssessment is stub | vuln.md | Document as stub or implement |
| HIGH | Docker shell injection risk | container/docker.rs | Add input validation |
| MEDIUM | k8s-openapi issue resolved | feature_matrix.md | Remove stale warning |
| MEDIUM | Move historical bug tables | tui.md, scanner.md | Archive to separate docs |
| MEDIUM | Recon module count 17 vs 18 | recon.md | Correct count |
| MEDIUM | StressConfig field names wrong | stress.md | Update documentation |
| MEDIUM | Error line reference | error.md | Fix `Io` location |
| MEDIUM | Missing utils table entry | utils.md | Add serialization |
| MEDIUM | Missing logging macros | logging.md | Document 4 macros |
| LOW | AI agents file paths | ai_agents.md | Add `agent/` prefix |
| LOW | Command count precision | overview.md, cli_commands.md | Clarify base vs feature-gated |
| LOW | utils module count | utils.md | Fix 23 vs 21 discrepancy |
| LOW | CSV formula test coverage | output.md | Add =, +, -, @ tests |
| LOW | Bug patterns may be stale | tui.md | Verify vs current lints |