# NSE Expansion Roadmap: Closeout, TUI Reporting, and Selective Compatibility Growth

## Purpose

This roadmap defines the next NSE work after the HTTP enforcement and runtime-corpus hardening passes.

The current NSE line is no longer primarily corrective. The core loader/profile/report/capability/runtime-corpus foundations are in strong shape. The remaining corrective work is documentation and verification hygiene, followed by user-visible expansion through TUI report rendering and bounded compatibility growth.

## Current Baseline

The repo currently has:

- profile-aware NSE execution with manual and automated safety boundaries;
- runtime-observed `NseRunReport` fields for compatibility, rules, libraries, capability events, evidence, output, diagnostics, and stats;
- local-only runtime corpus execution through `NseExecutor::with_profile()`;
- strict runtime library assertions unless explicitly soft in manifest metadata;
- local TCP/HTTP/UDP protocol fixtures;
- HTTP method enforcement with zero-hit tests for denied automated profiles;
- report formatting and `ReportEnvelope` bridge coverage;
- architecture guards for major NSE drift patterns.

## Phase Set

This roadmap contains six phases:

1. `plans/nse-expansion-phase-00-corrective-closeout.md`
2. `plans/nse-expansion-phase-01-tui-report-rendering.md`
3. `plans/nse-expansion-phase-02-report-filtering-navigation.md`
4. `plans/nse-expansion-phase-03-tls-sslcert-local-fixtures.md`
5. `plans/nse-expansion-phase-04-upstream-style-corpus-growth.md`
6. `plans/nse-expansion-phase-05-selective-deferred-library-migration.md`

## Recommended Order

Implement Phase 00 first. It is small but important because it reconciles stale documentation and verification notes before new work starts.

Then implement the TUI phases. The report model is now mature enough that the highest-value expansion is user-visible rendering, not more backend churn.

After that, expand deterministic local protocol compatibility through TLS/sslcert fixtures and curated upstream-style scripts.

Only migrate deferred protocol libraries after local fixtures exist to verify them.

## Non-Goals

Do not reopen loader/profile enforcement semantics.

Do not reopen library-report truthfulness semantics.

Do not relax AgentSafe/CiSafe behavior to make compatibility tests pass.

Do not claim full Nmap NSE parity.

Do not add public-internet-dependent tests.

Do not migrate SSH, SMB, databases, LDAP, or SNMP without deterministic local fixtures and capability-wrapper tests.

## Roadmap Acceptance Criteria

This roadmap is complete when:

- stale architecture/compatibility docs are corrected and final verification is recorded;
- the TUI can render structured NSE reports without parsing prose;
- users can navigate denials, errors, evidence, raw output, and compatibility reasons in the TUI;
- TLS/sslcert has deterministic local fixture coverage or explicit deferral notes;
- any broader corpus growth is local-only and truthfully labeled;
- at least one deferred library family has either a wrapper-backed migration plan or an explicit deferral rationale tied to fixture readiness;
- the next roadmap boundary is documented.

## Suggested Next Roadmap After Completion

After this roadmap, the next meaningful track would be one of:

1. TUI-driven NSE authoring/debugging workflow.
2. Broader safe local fixture harnesses for deferred protocols.
3. Integration of NSE reports with main Eggsec scan result aggregation.
4. Release packaging/docs/examples for manual users.
