# Hunt Module

## Purpose

Advanced threat hunting module for detecting attack chains, business logic flaws, race conditions, authorization bypasses, and session management vulnerabilities.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `HuntReport` | `hunt/mod.rs:24` | Aggregated hunt results across all categories; tracks per-category findings and total count |
| `HuntConfig` | `hunt/mod.rs:110` | Configuration controlling which hunt categories to run |
| `AttackChain` | `hunt/chain.rs` | Multi-step attack chain (privilege escalation, data exfiltration, RCE, lateral movement, persistence, DoS) |
| `BusinessLogicFlaw` | `hunt/business.rs` | Business logic vulnerability |
| `RaceCondition` | `hunt/race.rs` | Race condition / concurrency vulnerability (`remediation` field is private) |
| `AuthzBypass` | `hunt/authz.rs` | Authorization bypass vulnerability |
| `SessionIssue` | `hunt/session.rs` | Session management vulnerability |

### Feature Gate

`advanced-hunting` is a **marker-only** feature flag (`Cargo.toml:248`). It has no additional dependencies — the hunt module compiles unconditionally. The flag exists for feature-matrix completeness and is included in the `full` feature set.

### `HuntConfig` Defaults (`hunt/mod.rs:121-132`)

All sub-module checks are **enabled by default**:

| Field | Default | Description |
|-------|---------|-------------|
| `check_attack_chains` | `true` | Run `chain::detect_attack_chains()` |
| `check_business_logic` | `true` | Run `business::check_business_logic()` |
| `check_race_conditions` | `true` | Run `race::check_race_conditions()` |
| `check_authz_bypass` | `true` | Run `authz::check_authz_bypass()` |
| `check_session` | `true` | Run `session::check_session_security()` |
| `concurrency` | `10` | Max concurrent checks (unused — see note below) |
| `timeout_ms` | `30000` | Per-check timeout (30s) (unused — see note below) |

> **Note:** `concurrency` and `timeout_ms` are plumbed through `HuntConfig` but ignored by all sub-module detection functions (parameter name prefixed with `_config`).

### Sub-module Check Details

`run_hunt()` (`hunt/mod.rs:69-108`) iterates each `HuntConfig` flag and calls the corresponding sub-module's detection function only when enabled. Each sub-module returns a `Vec` of its finding type, which is appended to the report via the corresponding `add_*()` method. `total_findings` is incremented by 1 per finding, except for `AttackChain` which adds `chain.steps.len()`.

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `HuntReport`, `HuntConfig`, `run_hunt()` orchestrator |
| `chain.rs` | Attack chain detection (privilege escalation, RCE, data exfiltration) |
| `business.rs` | Business logic flaw detection (price manipulation, workflow bypass) |
| `race.rs` | Race condition and TOCTOU testing |
| `authz.rs` | Authorization bypass testing (IDOR, privilege escalation) |
| `session.rs` | Session management security (fixation, timeout, token prediction, cookies, CSRF) |

## Implementation Status

Implemented as template-based detection. `run_hunt()` orchestrates all sub-modules based on `HuntConfig` flags, but each detection function returns hardcoded findings regardless of target. No actual HTTP requests or analysis is performed.
