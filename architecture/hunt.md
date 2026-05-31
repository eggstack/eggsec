# Hunt Module

## Purpose

Advanced threat hunting module for detecting attack chains, business logic flaws, race conditions, authorization bypasses, and session management vulnerabilities.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `HuntReport` | `hunt/mod.rs` | Aggregated hunt results across all hunt categories |
| `HuntConfig` | `hunt/mod.rs` | Configuration controlling which hunt categories to run |
| `AttackChain` | `hunt/chain.rs` | Multi-step attack chain (privilege escalation, data exfiltration, RCE) |
| `BusinessLogicFlaw` | `hunt/business.rs` | Business logic vulnerability |
| `RaceCondition` | `hunt/race.rs` | Race condition / concurrency vulnerability |
| `AuthzBypass` | `hunt/authz.rs` | Authorization bypass vulnerability |
| `SessionIssue` | `hunt/session.rs` | Session management vulnerability |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `HuntReport`, `HuntConfig`, `run_hunt()` orchestrator |
| `chain.rs` | Attack chain detection (privilege escalation, RCE, data exfiltration) |
| `business.rs` | Business logic flaw detection (price manipulation, workflow bypass) |
| `race.rs` | Race condition and TOCTOU testing |
| `authz.rs` | Authorization bypass testing (IDOR, privilege escalation) |
| `session.rs` | Session management security (fixation, token leakage) |

## Implementation Status

Fully implemented. `run_hunt()` orchestrates all sub-modules based on `HuntConfig` flags. Each sub-module provides detection functions that return structured results.
