# ADR-006: Multi-Loadout Correlation Engine

**Date**: 2026-06-13
**Status**: Accepted
**Deciders**: Eggsec core team

## Context

Eggsec has multiple loadouts (proxy, db-pentest, auth-test, mobile-dynamic, wireless) that produce independent findings. Security analysts need to correlate findings across loadouts to identify attack chains and multi-vector vulnerabilities.

## Decision

Implement a `CorrelationEngine` with two correlation strategies:

1. **Temporal correlation**: Links findings from different sources that occur within a configurable time window. Uses RFC3339 timestamps and linear confidence decay based on time distance.
2. **Behavioral correlation**: Matches predefined patterns across loadouts (e.g., "SQLi in db-pentest + auth bypass in auth-test on the same host"). Pattern matching uses metadata fields (host, path) with configurable minimum source counts.

### Key Design Choices

1. **Engine pattern**: `CorrelationEngine` orchestrates all correlation strategies. Configurable via builder pattern (temporal window, patterns).
2. **Pattern registry**: `BehavioralPattern` structs define what cross-loadout correlations to look for. Patterns declare required sources, host/path matchers, and minimum match counts.
3. **Confidence scoring**: Temporal confidence decays linearly with time distance. Behavioral confidence scales with source diversity.
4. **Non-breaking extension**: New correlation strategies can be added as new methods on `CorrelationEngine` without changing existing types.

### Trade-offs

- **Chosen over ML heuristics**: Rule-based patterns are deterministic, auditable, and don't require training data. ML deferred to future phase.
- **Chosen over streaming correlation**: Batch correlation on completed sessions is simpler and sufficient for defense-lab use cases.
- **Temporal window configurable**: Default 60 seconds, adjustable per deployment.

## Consequences

- `CorrelationContext.summary` now tracks temporal and behavioral correlation counts.
- `BehavioralPattern` requires explicit source declarations (no implicit matching).
- Correlation runs after all loadouts complete, not during execution.
- Results integrate with existing `CorrelationContext` and bridge to `ScanReportData`.
