# Slapper Implementation Plan

**Created:** 2026-05-30
**Last Updated:** 2026-06-02
**Status:** Implementation complete - see Remaining Items below

---

## Summary

| Category | Count |
|----------|-------|
| Completed (2026-06-02 session) | 17 items |
| Remaining Items | ~20 |
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

---

## Remaining Items

> **Note:** Storage and VulnAssessment are now fully implemented. See Completed Work section above.

### High Priority

| Module | Item | Status |
|--------|------|--------|
| Pipeline | CSV escape lacks NFKC normalization | Pending |
| Pipeline | Defense-Lab stage counts incorrect | Pending |
| Pipeline | Defense-Lab profiles missing from table | Pending |
| Output | Format count mismatch (7 vs 8) | Pending |
| Stress | StressConfig field names wrong in docs | Pending |
| Stress | StressConfig missing fields in docs | Pending |

### Medium Priority

| Module | Item | Status |
|--------|------|--------|
| Auth | `run_full_test()` missing 4 test types | Partial (PasswordPolicy done) |
| Browser | SPA route discovery limited | Pending |
| Diff | DiffEngine/BaselineComparison locations unverified | Pending |
| Fuzzer | Silent error suppression in fuzz_endpoint | Pending |
| Hunt | No error handling in run_hunt() | Pending |
| Logging | 4 macros not documented | Pending |
| Recon | Pattern/file counts unverified | Pending |
| Scanner | UDP fingerprinting timeout handling | Pending |
| Storage | Sensitive passwords not encrypted at rest | Pending |

| WAF | No timeout on bypass attempts | Pending |
| WebSocket | Silent error suppression in close() | Pending |

### Low Priority

| Module | Item | Status |
|--------|------|--------|
| Browser | SPA route parameters limited | Pending |
| Compliance | Score thresholds hardcoded | Pending |
| Findings | JSONL format limitations | Pending |
| Fuzzer | AdaptiveRateLimiter not integrated | Pending |
| Generated | Regeneration process not documented | Pending |
| Hunt | No aggregation of concurrent results | Pending |
| Integrations | IssueTracker trait should be async | Pending |
| Notify | No retry logic for webhooks | Pending |
| Output | report_summary.rs uses HashMap | Pending |
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