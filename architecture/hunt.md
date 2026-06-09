# Hunt Module

## Purpose

Advanced threat hunting module for detecting attack chains, business logic flaws, race conditions, authorization bypasses, and session management vulnerabilities. Performs real HTTP-based probing of the target.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `HuntClient` | `hunt/mod.rs:27` | HTTP client wrapper with target URL, timeout, and convenience methods |
| `HuntReport` | `hunt/mod.rs:160` | Aggregated hunt results across all categories; tracks per-category findings and total count |
| `HuntConfig` | `hunt/mod.rs:247` | Configuration controlling which hunt categories to run |
| `AttackChain` | `hunt/chain.rs:7` | Multi-step attack chain (privilege escalation, data exfiltration, lateral movement) |
| `BusinessLogicFlaw` | `hunt/business.rs:7` | Business logic vulnerability (sensitive files, error handling, rate limiting) |
| `RaceCondition` | `hunt/race.rs:7` | Race condition / concurrency vulnerability |
| `AuthzBypass` | `hunt/authz.rs:7` | Authorization bypass vulnerability |
| `SessionIssue` | `hunt/session.rs:7` | Session management vulnerability |

### Feature Gate

`advanced-hunting` is a **marker-only** feature flag (`Cargo.toml:248`). It has no additional dependencies — the hunt module compiles unconditionally. The flag exists for feature-matrix completeness and is included in the `full` feature set.

### CLI Support

The `eggsec hunt` subcommand is available when the `advanced-hunting` feature is enabled:

```bash
eggsec hunt https://example.com
eggsec hunt https://example.com --skip-chains --skip-business
eggsec hunt https://example.com --concurrency 20 --timeout 60
eggsec hunt https://example.com --format json --output results.json
```

### TUI Integration

The Hunt tab provides a full GUI for configuring and running vulnerability hunts:
- Target URL input
- Category checkboxes (Attack Chains, Business Logic, Race Conditions, AuthZ Bypass, Session Security)
- Concurrency and timeout configuration
- Results display with severity-colored findings
- JSON export via `e` key

### `HuntConfig` Defaults

All sub-module checks are **enabled by default**:

| Field | Default | Description |
|-------|---------|-------------|
| `check_attack_chains` | `true` | Run `chain::detect_attack_chains()` |
| `check_business_logic` | `true` | Run `business::check_business_logic()` |
| `check_race_conditions` | `true` | Run `race::check_race_conditions()` |
| `check_authz_bypass` | `true` | Run `authz::check_authz_bypass()` |
| `check_session` | `true` | Run `session::check_session_security()` |
| `concurrency` | `10` | Max concurrent HTTP requests (used by semaphore) |
| `timeout_ms` | `30000` | Per-request timeout in milliseconds (sets reqwest client timeout) |

### Sub-module Check Details

`run_hunt()` orchestrates all sub-modules based on `HuntConfig` flags. Each sub-module performs real HTTP requests to the target and returns findings based on actual responses:

- **session**: Analyzes `Set-Cookie` headers for HttpOnly/Secure/SameSite flags, checks security headers (X-Frame-Options, CSP), tests session token entropy, checks for session fixation
- **authz**: Probes admin paths for unauthenticated access, tests IDOR-prone endpoints, checks for forced browsing, tests HTTP method restrictions (OPTIONS/TRACE)
- **race**: Sends concurrent POST requests to state-changing endpoints, measures timing anomalies, detects response inconsistencies under load
- **business**: Discovers API endpoints, probes for sensitive files (.env, .git, credentials), tests error handling for verbose messages, checks rate limiting
- **chain**: Correlates findings from other modules into multi-step attack chains (no additional HTTP requests)

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `HuntClient`, `HuntReport`, `HuntConfig`, `run_hunt()` orchestrator |
| `chain.rs` | Attack chain detection (correlates findings from other modules) |
| `business.rs` | Business logic flaw detection (sensitive files, API discovery, error handling, rate limiting) |
| `race.rs` | Race condition and concurrency testing (concurrent requests, timing analysis) |
| `authz.rs` | Authorization bypass testing (admin access, IDOR, forced browsing, HTTP methods) |
| `session.rs` | Session management security (cookies, headers, token entropy, fixation) |

## Implementation Status

Fully implemented with real HTTP-based detection. All sub-modules perform actual HTTP requests to the target using `reqwest` via `HuntClient`. Findings are based on real response analysis, not hardcoded templates. The `concurrency` and `timeout_ms` configuration fields are actively used.
