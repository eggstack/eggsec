# Slapper Implementation Plan

**Created:** 2026-05-30
**Last Updated:** 2026-06-05
**Status:** Implementation complete - see Remaining Items below

---

## Summary

| Category | Count |
|----------|-------|
| Completed (2026-06-02 session) | 17 items |
| Completed (2026-06-05 session) | 13 items |
| Remaining Items | ~7 |
| Known Intentional Stubs | 0 |

---

## Non-Goals

- Do NOT add new offensive capability
- Do NOT reintroduce Python/Ruby plugin runtimes
- Do NOT publish crates or flip visibility unless instructed
- Do NOT invent domains/organizations/support contacts
- Do NOT claim production maturity for experimental features
- Do NOT remove NSE support
- Do NOT perform large architectural rewrites in single passes

---

## Completed Work (2026-06-02 Session)

### Security Fixes

| Item | Description | Reference |
|------|-------------|-----------|
| NSE TOCTOU vulnerability | `get_allowed_path()` method added to lfs/os libraries to prevent race conditions | `slapper-nse/src/sandbox/lfs.rs` |
| NSE DNS rebinding mitigation | `is_host_allowed()` limitation documented; `resolve_host()` returns bound IPs | `slapper-nse/src/sandbox/mod.rs` |
| NSE sandbox integration tests | 17 new tests for path/command/network restrictions | `slapper-nse/tests/` |
| Docker shell injection | FIXED - `inspect_image()` validates image names before shell | `container/docker.rs:208-209` |

### Auth Module

| Item | Description | Reference |
|------|-------------|-----------|
| PasswordPolicy execution | `PasswordPolicy` test now runs in `run_full_test()` | `auth/mod.rs` |
| stop_on_lockout parameterization | `AuthEngine::new()` now uses passed parameter | `auth/mod.rs` |
| Multi-protocol testers | SSH/FTP/SMTP testers now conditionally compiled | `auth/multi_protocol.rs` |

### Browser Module

| Item | Description | Reference |
|------|-------------|-----------|
| XSS payload configurable | XSS test now uses configurable payload | `browser/mod.rs` |
| ClientIssueType coverage | All 8 variants now implemented | `browser/mod.rs` |
| Corpus integrated | RequestCorpus/CorpusEntry integrated into browser scan | `browser/mod.rs` |

### Error & Findings Module

| Item | Description | Reference |
|------|-------------|-----------|
| Lossy anyhow::Error conversion | Fixed with proper error mapping | `error/mod.rs` |
| FindingStore deduplication | `store_finding()` now deduplicates by fingerprint | `findings/store.rs` |

### Storage

| Item | Description | Reference |
|------|-------------|-----------|
| Unified StoredFinding type | `StoredFinding` re-exported from `findings::lifecycle` for database persistence | `storage/models.rs` |
| SQLx implementation | Full CRUD operations with PostgreSQL via `PgPool` and parameterized queries | `storage/postgres.rs` |
| Migrations | Schema creation for scans, findings, and users tables | `crates/slapper/migrations/` |

### Vuln Assessment

| Item | Description | Reference |
|------|-------------|-----------|
| Structured VulnAssessment | Rich struct with `cvss_score`, `exploit_info`, `asset_criticality`, `prioritized_findings`, `triage_results`, `remediation_plans`, `summary` | `vuln/mod.rs:37` |
| TUI worker wiring | VulnAssessment integrated into TUI scan workflow | `tui/` |
| Stage::Vuln pipeline integration | Vulnerability assessment stage in security assessment pipeline | `pipeline/stage.rs` |

### Diff Module

| Item | Description | Reference |
|------|-------------|-----------|
| Evidence content tracking | Old/new evidence content stored in `FindingChange` | `output/diff.rs` |
| FxHashMap migration | Replaced `std::collections::HashMap` with `FxHashMap` | `output/diff.rs` |
| EvidenceKind display names | Human-readable names (e.g., "HTTP Request") | `findings/types.rs` |

### Proxy Module

| Item | Description | Reference |
|------|-------------|-----------|
| Rotation strategies docs | All 5 strategies documented (RoundRobin, Random, Weighted, LeastUsed, LowestLatency) | `architecture/proxy.md` |

### AI/Coding Agent

| Item | Description | Reference |
|------|-------------|-----------|
| Endpoints tool test | Test coverage added for `endpoints` tool | `tool/protocol/mcp/policy.rs` |

### Distributed Module

| Item | Description | Reference |
|------|-------------|-----------|
| CAPABILITIES documentation | Worker capabilities documented | `architecture/distributed.md` |

### Documentation Fixes (2026-06-05 Session)

| Item | Description | Reference |
|------|-------------|-----------|
| Output format count | Fixed overview.md:103 format list (was 7 wrong names, now 8 correct) | `architecture/overview.md` |
| Stress skill fields | Added 6 missing StressConfig fields to skill doc | `.opencode/skills/slapper-stress/SKILL.md` |
| Stale review entries | Removed false claims from review skill and review plan | Various |

---

## Remaining Items

> **Note:** Storage and VulnAssessment are now fully implemented. See Completed Work section above.

### High Priority

| Module | Item | Status |
|--------|------|--------|
| Pipeline | CSV escape lacks NFKC normalization | Done - `escape.rs` already has NFKC |
| Pipeline | Defense-Lab stage counts incorrect | Done - counts match docs |
| Pipeline | Defense-Lab profiles missing from table | Done - present in all tables |
| Output | Format count mismatch (7 vs 8) | Done - fixed overview.md:103 |
| Stress | StressConfig field names wrong in docs | Done - stress.md already correct |
| Stress | StressConfig missing fields in docs | Done - fixed skill file |

### Medium Priority

| Module | Item | Status |
|--------|------|--------|
| Auth | `run_full_test()` missing 4 test types | Done - all 8 present |
| Browser | SPA route discovery limited | Pending |
| Diff | DiffEngine/BaselineComparison locations unverified | Pending |
| Fuzzer | Silent error suppression in fuzz_endpoint | Done - proper error handling |
| Hunt | No error handling in run_hunt() | Done - uses `?` propagation |
| Logging | 4 macros not documented | Done - logging has no macros |
| Recon | Pattern/file counts unverified | Done - counts verified (30 secrets, 80 paths, 12 IAM) |
| Scanner | UDP fingerprinting timeout handling | Done - timeouts on send/recv |
| Storage | Sensitive passwords not encrypted at rest | Pending |

| WAF | No timeout on bypass attempts | Done - 15s client timeout |
| WebSocket | Silent error suppression in close() | Done - no close() method exists |

### Low Priority

| Module | Item | Status |
|--------|------|--------|
| Browser | SPA route parameters limited | Pending |
| Compliance | Score thresholds hardcoded | Pending |
| Findings | JSONL format limitations | Pending |
| Fuzzer | AdaptiveRateLimiter not integrated | Done - fully integrated |
| Generated | Regeneration process not documented | Pending |
| Hunt | No aggregation of concurrent results | Pending |
| Integrations | IssueTracker trait should be async | Pending |
| Notify | No retry logic for webhooks | Done - exponential backoff implemented |
| Output | report_summary.rs uses HashMap | Done - already uses FxHashMap |
| Pipeline | DEFAULT_ENDPOINTS static array | Pending |
| TUI | Tab::all() ordering inconsistent | Pending |
| Workflow | SLA calculation ignores resolved findings | Pending |
| Workflow | FalsePositive transitions missing | Pending |

---

## Key Module Locations

| Module | Location |
|--------|----------|
| Agent | `crates/slapper/src/agent/` |
| AI | `crates/slapper/src/ai/` |
| Auth | `crates/slapper/src/auth/` |
| Browser | `crates/slapper/src/browser/` |
| Config | `crates/slapper/src/config/` |
| Container | `crates/slapper/src/container/` |
| Distributed | `crates/slapper/src/distributed/` |
| Findings | `crates/slapper/src/findings/` |
| Fuzzer | `crates/slapper/src/fuzzer/` |
| Hunt | `crates/slapper/src/hunt/` |
| Loadtest | `crates/slapper/src/loadtest/` |
| Networking | `crates/slapper/src/networking/` |
| NSE | `slapper-nse/` |
| Output | `crates/slapper/src/output/` |
| Pipeline | `crates/slapper/src/pipeline/` |
| Proxy | `crates/slapper/src/proxy/` |
| Recon | `crates/slapper/src/recon/` |
| Scanner | `crates/slapper/src/scanner/` |
| Storage | `crates/slapper/src/storage/` |
| Stress | `crates/slapper/src/stress/` |
| Supply Chain | `crates/slapper/src/supply_chain/` |
| TUI | `crates/slapper/src/tui/` |
| Vuln | `crates/slapper/src/vuln/` |
| WAF | `crates/slapper/src/waf/` |
| Workflow | `crates/slapper/src/workflow/` |

---

## Verification Notes (2026-06-02 Session)

The following items were verified against actual codebase:

| Item | Finding |
|------|---------|
| NSE CveCache/FxHashMap | Already uses FxHashMap - no migration needed |
| Defense Lab RunManifest | File exists at `output/run_manifest.rs:25-56` |
| Recon module count | Verified as 17 entries |
| Kubernetes .ok() lines | Only line 65 uses silent ignore |
| Notify `let _` lines | Only line 114 uses silent ignore |
| AI Agents CodingAgent | `endpoints` documented but not tested |
| AiClient Clone | Uses `#[derive(Clone)]`, not manual impl |
| Diff JSONL vs JSON | Code correctly uses JSONL format |
| Overview command counts | Actual is 24 base, 37 with all features |

---

(End of file)