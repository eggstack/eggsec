# Subsystem Roadmaps

Subsystem roadmaps translate the canonical Eggsec direction into coherent, dependency-aware workstreams. They are not direct coding-agent checklists.

Each roadmap should remain useful across several implementation milestones and repository revisions. Commit-specific mechanics belong in `plans/implementation/`.

## Naming

```text
<subsystem>-roadmap.md
```

Examples:

```text
transport-http-egress-roadmap.md
frontend-runtime-tui-roadmap.md
python-programmability-roadmap.md
```

Legacy roadmaps from the flat era (2026-07 through 2026-09) are grandfathered at the `plans/` top level under their original dated filenames (e.g. `network-dependency-hardening-roadmap-2026-09-11.md`). They are historical records and MUST NOT be renamed or rewritten; `plans/registry.md` maps each of them to its subsystem. All NEW subsystem roadmaps live here and follow the naming and structure below.

## Required roadmap structure

```markdown
# <Subsystem> Roadmap

Status: proposed | active | closing | closed | superseded

Long-term references:

- `plans/000-long-term-specification.md#...`
- `plans/001-terminology-and-domain-model.md#...`
- `plans/002-long-term-roadmap.md#...`

Related ADRs:

- `plans/adrs/ADR-NNNN-...md`

## 1. Purpose and ownership boundary

Define what the subsystem owns, what it consumes, and what it must not own.

## 2. Work classification

### Invariants

- ...

### Capabilities

- ...

### Infrastructure

- ...

### Polish

- ...

## 3. Non-goals

- ...

## 4. Current state

Summarize repository evidence, existing contracts, compatibility paths, and known gaps. Avoid fragile line-number references unless essential.

## 5. Target architecture

Describe the end-state module, storage, protocol, ownership, and lifecycle model for this subsystem.

## 6. Dependency graph

```text
Milestone A
    |
    +--> Milestone B
    |
    `--> Milestone C
             |
             `--> Milestone D
```

Classify each dependency as hard, interface, soft, or operational.

## 7. Milestones

### Milestone 1 — Title

Class: invariant | capability | infrastructure | polish

Objective:

Dependencies:

Deliverable boundary:

User or operator value:

Exit conditions:

Deferred work:

### Milestone 2 — Title

...

## 8. Cross-cutting requirements

### Storage and migration

### Protocol and compatibility

### Security and authorization

### Concurrency, cancellation, and recovery

### Observability and audit

### Performance and resource use

### Documentation and operations

## 9. Verification strategy

Define subsystem-level integration, property, contention, restart, migration, and end-to-end evidence. Name the `make` contract explicitly (`make check` is mandatory; deep checks are `make check-full`, `make check-features-individual`, `make clippy-domain`, `make test-tui-pty`).

## 10. Risks and decision points

List unresolved decisions and identify which require ADRs.

## 11. Completion definition

Describe what must be true before the subsystem roadmap is closed.

## 12. Milestone status

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| 1 | not started | — | — | — |
```

## Roadmap rules

A subsystem roadmap MUST:

- link to canonical long-term requirements rather than duplicating them wholesale;
- define ownership boundaries before milestones;
- distinguish infrastructure from completed capability;
- expose dependencies and decision points;
- preserve completed milestone history;
- link each active milestone to one implementation plan and later one closure record;
- state non-goals to prevent scope expansion;
- remain at the subsystem level rather than becoming a file-by-file implementation checklist.

A subsystem roadmap MAY be updated when implementation evidence changes sequencing or decomposition. Material changes must record why the roadmap changed.

## Initial subsystem decomposition

The long-term roadmap suggests, but does not mandate, the following workstreams:

1. enforcement, scope, and dispatch ownership;
2. crate boundaries and reusable-library ownership;
3. scoped HTTP transport, Eggfetch backend, and Eggress reuse;
4. frontend/runtime convergence (CLI, TUI, daemon, runtime bridge);
5. Python programmability and bindings;
6. CI, verification, and release engineering;
7. performance and resource efficiency;
8. protocol, agent, distributed, and domain execution;
9. supply-chain, TLS, and platform integration.

Create only roadmaps that are ready to be reasoned about. Do not generate all candidate files merely to populate the directory.
