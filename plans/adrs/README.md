# Architecture Decision Records

This directory contains durable decisions that affect Eggsec architecture across milestones or subsystems.

Use an ADR when a question cannot be answered safely inside one implementation plan without establishing a reusable architectural contract.

## Naming

```text
ADR-NNNN-short-title.md
```

Numbers are monotonically increasing and never reused.

## Status lifecycle

```text
proposed -> accepted -> deprecated or superseded
          `-> rejected
```

Accepted ADRs are historical records. Do not rewrite an accepted ADR to make a later decision appear original. Create a new ADR and mark the old one superseded.

## ADR template

```markdown
# ADR-NNNN: Title

Status: proposed

Date: YYYY-MM-DD

Decision owners: project maintainers

Related specification sections:

- `plans/000-long-term-specification.md#...`
- `plans/001-terminology-and-domain-model.md#...`

Affected subsystem roadmaps:

- `plans/subsystems/...`

## Context

Describe the architectural problem, existing implementation, constraints, and why the decision is required now.

## Decision drivers

- ...

## Considered options

### Option A — Name

Description, benefits, costs, and failure modes.

### Option B — Name

Description, benefits, costs, and failure modes.

## Decision

State the selected option precisely, including ownership and interface boundaries.

## Consequences

### Positive

- ...

### Negative

- ...

### Neutral or deferred

- ...

## Compatibility and migration

Describe storage, protocol, configuration, API, and operational migration requirements.

## Security and reliability implications

Describe authorization, secret handling, scope enforcement, transport checkpoints, contention, cancellation, restart, recovery, and denial-of-service effects.

## Verification

Describe the evidence required to prove implementations conform to this decision.

## Supersession

None.
```

## ADR threshold

An ADR is normally required when a decision:

- changes an enforcement or authorization semantic (`EnforcementContext`, `eggsec-policy`, approval tokens);
- introduces a new scope, transport-checkpoint, or `NetworkAuthority` contract;
- selects a durable external dependency for HTTP, proxy, TLS, or database behavior;
- changes dispatch ownership (`EnforcedDispatcher`, canonical execution, runtime bridge);
- changes daemon IPC, runtime wire DTOs, or `TaskKind` operation identity;
- establishes a public compatibility contract (report envelope, Python API, CLI surface);
- changes the Tokio/capability baseline or the ring-only TLS policy;
- materially changes a long-term non-goal.

An ADR is usually unnecessary for local refactors, internal naming cleanup, implementation-specific data structures, or reversible optimizations that preserve established contracts.
