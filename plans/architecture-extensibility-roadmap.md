# Eggsec Architecture and Extensibility Roadmap

## Purpose

This roadmap defines the next line of architecture work for Eggsec: preserving the current central enforcement model while making the repository easier to extend, audit, and maintain as more security-assessment domains are added.

The current architecture has a strong safety spine: execution surfaces derive execution profiles, operations are described by `OperationDescriptor`, policy and scope flow through `EnforcementContext`, policy outcomes are explicit, and strict programmatic surfaces can use `ApprovedOperation` tokens before dispatch. The next problem is architectural scale. Eggsec now contains many capability families: standard scanning, recon, WAF validation, load/stress testing, raw packets, proxy interception, database lab assessment, mobile analysis, wireless, evasion detection, post-exploitation simulation, C2 simulation, API/MCP/agent surfaces, reporting, and TUI workflows.

The goal is to prevent the main `eggsec` crate, the command dispatcher, the feature matrix, and the documentation from becoming long-term bottlenecks.

## Target architecture

Eggsec should evolve into a policy-mediated assessment platform with stable extension seams.

The desired ownership model is:

- `eggsec-core`: dependency-light shared primitives, constants, and portable types.
- `eggsec-tool-core`: protocol-neutral request/response/finding/history/rate-limit DTOs.
- `eggsec-output`: report/output adapters and common rendering formats.
- `eggsec-agent`: agent coordination primitives, not policy ownership.
- `eggsec-cli`: CLI binary entrypoint and command-line argument surface.
- `eggsec-tui`: terminal UI adapter, tab state, task orchestration, interactive preflight/confirmation UX.
- `eggsec-nse`: optional Nmap NSE compatibility domain.
- `eggsec-db-lab`: database defense-lab execution, reports, baselines, compliance mapping, and driver integration.
- `eggsec-web-proxy`: web proxy/interception execution and related report/evidence paths.
- future domain crates: mobile lab, wireless lab, evasion lab, post-exploitation lab, C2 lab, and other high-growth domains.
- `eggsec`: composition/orchestration crate that owns config loading, scope provenance, policy/enforcement, operation metadata, command routing, pipeline assembly, tool registration, report bridging, and compatibility facades.

The primary invariant is: domain crates may declare operations and execute already-approved work, but they must not decide whether work is authorized. Authorization belongs to central policy/scope enforcement.

## Architectural invariants

1. Caller identity must be represented by `ExecutionSurface`.

2. `ExecutionSurface` must derive `ExecutionProfile`; entrypoints should not hand-roll profile selection.

3. Networked or side-effecting work must be represented by `OperationDescriptor` before execution.

4. Strict programmatic surfaces must fail closed: MCP, agent, CI, REST, and gRPC must not honor manual override flags.

5. Automated networked operations that require explicit scope must require explicit manifest provenance, not only an in-memory `Scope` value.

6. Manual CLI/TUI operation may remain operator-directed, but discretionary continuation must be explicit, audited, and limited to permissive manual surfaces.

7. Hazardous and defense-lab domains must remain compile-time feature-gated and runtime policy-gated.

8. Tool/MCP dispatch for strict surfaces should require an approval token or equivalent proof of pre-dispatch enforcement.

9. Documentation, policy metadata, tool metadata, and TUI/CLI preflight should converge on one source of truth for operation risk/capability/scope semantics.

10. New domains should be integrated through a declarative contract rather than scattered manual edits across CLI, TUI, MCP, reports, docs, and policy.

## Phase overview

### Phase 1: Architecture inventory and invariant map

Create a precise map of current workspace boundaries, module responsibilities, side-effecting execution paths, and enforcement chokepoints. Document where the architecture is already strong and where transitional APIs still exist.

Primary output: an architecture document that future work can use as a baseline, plus targeted tests or checklists for high-risk paths.

### Phase 2: Enforcement invariant hardening

Make the current safety model mechanically harder to regress. Audit all strict surfaces, raw dispatch paths, manual override handling, scope provenance requirements, and direct scope helpers. Add tests for the manual/strict/automated behavior matrix.

Primary output: stricter dispatch/enforcement tests and removal or quarantine of legacy bypass-prone helpers.

### Phase 3: Domain module contract design

Define a static domain integration contract that can describe domain identity, operations, capabilities, feature gates, CLI/TUI/MCP exposure, report adapters, dry-run support, and test fixtures. Use an existing domain such as db-lab or web-proxy as the pilot.

Primary output: a domain descriptor trait/struct and one pilot implementation.

### Phase 4: Metadata unification for operations, tools, docs, and policy

Move toward a single source of truth for operation risk, capabilities, scope requirements, feature gates, frontend exposure, and report support. Feed policy explain, TUI preflight, MCP registration, CLI help annotations, and generated docs from the same metadata.

Primary output: generated or validated capability matrix and reduced README/doc drift.

### Phase 5: Main crate slimming pass

Use the domain contract and metadata model to extract additional large capability domains out of the main `eggsec` crate, following the `eggsec-db-lab` model: domain crates own execution and reports; main crate owns enforcement and composition.

Primary output: at least one additional domain extraction or deep extraction cleanup, with compatibility facades retained where needed.

### Phase 6: Command registry refactor

Replace scattered command metadata with a declarative registry. Keep the CLI match exhaustive if desired, but make command IDs, domain IDs, feature gates, descriptor builders, dry-run support, and frontend exposure discoverable.

Primary output: command metadata registry used by policy explain, docs, and TUI command palette.

### Phase 7: Tool and MCP registration modernization

Move tool/MCP registration toward domain metadata. Optional hazardous MCP tools must remain explicit feature-gated opt-ins. Programmatic surfaces should use enforced dispatch by default.

Primary output: metadata-driven tool registration and strict dispatch regression tests.

### Phase 8: TUI architecture tightening

Normalize TUI tabs around descriptor-driven preflight, CLI-equivalent preview, approval-token-aware launch, task runtime integration, and declarative tab metadata.

Primary output: side-effecting tabs consistently preflight before execution and become easier to add.

### Phase 9: Report and evidence unification

Define a common report/evidence envelope for domain outputs. Domain crates can own rich reports but must convert into a shared assessment/report shape for output adapters and agents.

Primary output: common report envelope used by multiple domains and evidence bundles with enforcement/audit provenance.

### Phase 10: Feature matrix and build profile cleanup

Reduce historical comments in `Cargo.toml`, document feature categories, define common build profiles, and add representative feature-combination CI coverage.

Primary output: cleaner feature definitions, generated feature matrix, and better cfg-rot detection.

### Phase 11: CI architecture guards

Add CI checks for dependency direction, forbidden imports, forbidden strict-surface dispatch paths, doc/metadata drift, and enforcement behavior regressions.

Primary output: architecture invariants enforced by tests and static checks.

### Phase 12: Extensibility handoff and contributor model

Write `docs/ADDING_A_DOMAIN.md` and provide a skeleton/mock domain that exercises the full integration path without real network side effects.

Primary output: a contributor-facing path for adding new domains without broad invasive edits.

## Recommended sequencing

The first three phases form the architecture stabilization block. Do not do broad extraction before the domain contract exists; otherwise complexity will move from modules into crates without reducing integration burden.

Phases 4 through 7 form the extensibility management block. They make command/tool/docs/policy integration more declarative and reduce metadata duplication.

Phases 8 through 10 form the frontend/report/build hygiene block. They keep the TUI, reports, evidence bundles, and feature profiles coherent as the domain surface grows.

Phases 11 and 12 form the sustainability block. They turn architecture decisions into enforceable checks and handoff documentation.

## Success criteria

This roadmap is complete when:

- strict programmatic surfaces cannot conveniently bypass central enforcement;
- manual CLI/TUI discretion remains available but is explicit, narrow, and audited;
- at least several large domains follow the db-lab style extraction model;
- adding a new domain does not require broad ad hoc edits across many subsystems;
- operation metadata drives docs, policy explain, preflight, and tool registration;
- CI catches common architecture regressions;
- user-facing documentation no longer drifts from workspace and feature reality.
