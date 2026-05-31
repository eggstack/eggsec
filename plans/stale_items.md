# Stale Items Detection

**Date:** 2026-05-31
**Documents Scanned:** 17

## Outdated References

- [pipeline.md:90] Claims "Five defense-lab profiles are planned but not yet implemented. They will add to the `ScanProfile` enum and `Stage::from_profile()` mapping." This is stale — all five defense-lab profiles (`DefenseLab`, `SynvoidLocal`, `WafRegression`, `ProtocolEdge`, `NseSafe`) are fully implemented in `cli/mod.rs:262-266` and wired into `pipeline/stage.rs:92-107`. The contradicting doc `defense_lab.md:102` correctly states they are implemented.

## Orphaned Documents

- None. All 17 architecture documents (excluding `review_plan.md`) have corresponding implementations in `crates/slapper/src/` or `slapper-nse/`.

## Statistical Drift

- [overview.md:7] Claims "526 source files" — actual count is **522** `.rs` files under `crates/slapper/src/`
- [tui.md:32] Claims "Security fuzzing with 31 payload types" — actual `PayloadType` enum has **30** variants (`fuzzer/payloads/mod.rs:39-70`)
- [tui.md:1082,1151,1509,1588,1663] Multiple session fix headers reference "29 Tabs" — the `Tab` enum (`tui/tabs/mod.rs:80-109`) has exactly **28** variants. The tab table at tui.md:23-54 correctly lists 28 tabs.
- [pipeline.md:23-35] Lists only 11 profiles in the main table — actual `ScanProfile` enum has **16** variants. The 5 defense-lab profiles are listed separately as "planned" but are implemented.

## Missing Coverage

These modules under `crates/slapper/src/` have no dedicated architecture document (only brief mentions in `overview.md`):

- `auth/` — Authentication security testing (brute force, credential stuffing, lockout detection, MFA bypass)
- `browser/` — Headless Chrome integration (DOM XSS, SPA crawling)
- `compliance/` — Compliance scanning (OWASP, PCI-DSS, HIPAA, SOC2)
- `container/` — Container security (Docker, Kubernetes, CIS benchmarks)
- `diff/` — Finding comparison engine
- `error/` — Unified error types (`SlapperError` with 20+ variants)
- `findings/` — Canonical `Finding` schema with confidence levels and evidence kinds
- `hunt/` — Advanced threat hunting (attack chains, business logic, race conditions)
- `integrations/` — Issue tracker connectors (Jira, GitHub, GitLab)
- `notify/` — Notification system (webhook, email, Slack, PagerDuty)
- `proxy/` — Proxy pool management (SOCKS4/5, HTTP, HTTPS, Tor)
- `storage/` — SQLx-based persistence (PostgreSQL)
- `supply_chain/` — SBOM generation (CycloneDX, SPDX) and vulnerability scanning
- `vuln/` — Vulnerability management (CVSS 3.1, triage, remediation)
- `websocket/` — WebSocket security testing
- `wireless/` — Wireless security testing
- `workflow/` — Finding lifecycle management (status, assignment, SLA)

## Duplicate Content

- [ai_agents.md] and [overview.md:136-144] Both cover `ai/`, `agent/`, and `tool/` modules. The overview provides a summary table while ai_agents.md provides deep dive. This is expected for an overview document, but the MCP profile/policy content is duplicated almost verbatim between the two docs (compare ai_agents.md:96-158 with overview.md:333-334).
- [defense_lab.md] and [pipeline.md:88-100] Both describe defense-lab profiles. `defense_lab.md` correctly documents them as implemented; `pipeline.md` incorrectly says they are "planned but not yet implemented."
- [overview.md:458-475] and individual module docs repeat the same codebase health statistics (source files, modules, tests, etc.). These are only maintained in `overview.md` and should be the single source of truth.

## Summary

- Total outdated references: 1
- Total orphaned documents: 0
- Total statistical drift: 4
- Total missing coverage: 17
- Total duplicates: 3
