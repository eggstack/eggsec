# Phase 1 Handoff Plan: Architecture Inventory and Invariant Map

## Objective

Create a precise architecture inventory for Eggsec that maps workspace ownership, module responsibilities, frontend entrypoints, side-effecting execution paths, enforcement chokepoints, and known transitional APIs. This phase is intentionally observational plus light validation. It should not attempt broad refactors. Its purpose is to establish a reliable baseline for later enforcement hardening and domain modularization.

## Context

Eggsec has evolved into a multi-surface security assessment engine with manual CLI/TUI usage, strict MCP/agent/CI/API usage, multiple high-risk lab domains, and several extracted crates. The policy model is now much stronger than the older local-check style: `ExecutionSurface`, `ExecutionProfile`, `OperationDescriptor`, `EnforcementContext`, `EnforcementOutcome`, `ManualOverride`, and `ApprovedOperation` exist as central concepts.

The next risk is architectural scale and drift. New capabilities can still require edits across command dispatch, CLI args, TUI tabs, MCP/tool registration, report conversion, docs, feature flags, and enforcement metadata. Before refactoring, document where those seams currently are.

## Deliverables

1. Add or update `docs/ARCHITECTURE.md`.

2. Add an enforcement-flow section covering CLI, TUI, MCP, agent, CI, REST, and gRPC surfaces.

3. Add a workspace ownership table covering each crate and whether it owns primitives, policy, frontend, domain execution, output, or orchestration.

4. Add a side-effecting execution path inventory covering all major command families and tool dispatch paths.

5. Add a transitional API inventory covering helpers or paths that should eventually be removed, quarantined, or converted.

6. Add a short `docs/ARCHITECTURE_INVARIANTS.md` or a section inside `docs/ARCHITECTURE.md` listing invariants that future work must preserve.

7. Add lightweight tests or comments only where a currently undocumented behavior is already easy to validate. Avoid large behavior changes in this phase.

## Files and areas to inspect

Start with these files and modules:

- `Cargo.toml`
- `crates/eggsec/Cargo.toml`
- `crates/eggsec/src/lib.rs`
- `crates/eggsec-cli/src/main.rs`
- `crates/eggsec/src/config/mod.rs`
- `crates/eggsec/src/config/policy.rs`
- `crates/eggsec/src/config/policy_decision.rs`
- `crates/eggsec/src/config/scope.rs`
- `crates/eggsec/src/commands/handlers/mod.rs`
- `crates/eggsec/src/tool/mod.rs`
- `crates/eggsec/src/tool/dispatcher.rs`
- `crates/eggsec-tui/src/app/mod.rs`
- `crates/eggsec-tui/src/app/enforcement.rs`
- `crates/eggsec-db-lab/src/lib.rs`
- `crates/eggsec-web-proxy/src/lib.rs`
- README and existing architecture/safety docs

Also inspect command-specific handlers for high-risk domains: stress, packet, db, web-proxy, wireless, mobile, evasion, postex, C2, MCP, agent, REST/gRPC, and pipeline.

## Architecture document structure

Use this outline for `docs/ARCHITECTURE.md` unless an equivalent file already exists and should be extended.

### 1. System overview

Describe Eggsec as a policy-mediated assessment engine with multiple frontends and multiple domain capabilities. Explicitly distinguish manual operator workflows from agent-controlled or non-interactive workflows.

### 2. Workspace crate ownership

Create a table with columns:

- crate
- role
- owns policy decisions? yes/no
- owns execution? yes/no/domain-specific
- frontend? yes/no
- should remain dependency-light? yes/no
- notes

Expected direction:

- `eggsec-core`: shared primitives, no frontend, no policy decisions.
- `eggsec-tool-core`: protocol-neutral DTOs, no engine dependency.
- `eggsec-output`: output/report adapters.
- `eggsec-agent`: coordination primitives, not central policy.
- `eggsec-cli`: binary entrypoint.
- `eggsec-tui`: TUI frontend.
- `eggsec-nse`: optional compatibility domain.
- `eggsec-db-lab`: domain execution/reporting for database lab checks; does not authorize.
- `eggsec-web-proxy`: domain execution/reporting for web proxy/interception; does not authorize.
- `eggsec`: composition root and central policy/orchestration.

### 3. Enforcement model

Document these concepts:

- `ExecutionSurface`
- `ExecutionProfile`
- `OperationRisk`
- `OperationMode`
- `Capability`
- `OperationDescriptor`
- `ExecutionPolicy`
- `LoadedScope` and scope provenance
- `EnforcementContext`
- `EnforcementOutcome`
- `ManualOverride`
- `ApprovedOperation`

State the key invariant: authorization is centralized; domain crates may declare and execute but must not authorize.

### 4. Frontend execution flows

For each frontend, document the current flow:

- CLI manual: parse CLI, resolve surface, load config/scope, build `CommandContext`, attach manual override, call command handler.
- CLI strict: same, but `--strict-scope` resolves to strict manual surface.
- TUI manual/guarded: TUI state owns `TuiEnforcementState`, preflight runs before direct-launch side effects.
- MCP: strict profile, explicit scope manifest required for networked ops, no manual override.
- Agent: strict profile, explicit scope manifest required, re-evaluation before dispatch where applicable.
- CI: strict deterministic behavior, no overrides.
- REST/gRPC: currently strict by surface mapping; document any pending ambiguity.

### 5. Side-effecting execution path inventory

Create a table with columns:

- operation family
- entrypoint(s)
- feature gate
- operation descriptor source
- policy/enforcement call site
- dispatch/execution call site
- report/output path
- strict-surface status
- notes/gaps

Include at least:

- port scan
- endpoint scan
- fingerprint
- recon
- fuzz
- WAF detect/bypass/stress
- load test
- auth-test
- packet/traceroute/ICMP
- stress
- proxy pool/proxy intercept
- db-pentest
- mobile
- wireless
- evasion
- postex
- C2
- browser/headless
- pipeline profiles
- MCP tools
- agent workflows
- REST/gRPC handlers

### 6. Transitional APIs and risk register

List APIs or design points that are acceptable now but should be addressed later:

- direct `CommandContext::ensure_scope` / `ensure_scope_url` style helpers if they can substitute for descriptor enforcement;
- `CommandContext::with_execution_profile` if it bypasses surface-derived profile selection;
- raw `ToolDispatcher::dispatch` access;
- duplicated CLI flag suggestion logic between CLI and TUI;
- feature metadata duplicated between Cargo, README, policy metadata, and tool docs;
- central command match growth;
- any domain logic still heavily embedded in the main crate.

Each item should have a recommended disposition: keep, deprecate, restrict visibility, wrap, migrate, or test.

### 7. Architecture invariants

List invariants in short normative language. Suggested examples:

- All side-effecting operations must have an `OperationDescriptor` before execution.
- Automated surfaces must never honor `ManualOverride`.
- Manual override state must not appear in MCP/agent request schemas.
- Strict surfaces must fail closed on `Warn`, `RequireConfirmation`, or `Deny`.
- Explicit manifest provenance must be checked for automated networked operations that require explicit scope.
- Domain crates must not decide authorization.
- Feature gates are not sufficient authorization; runtime policy must still apply.
- Dry-run must be side-effect free.
- Approval tokens must not be reusable for a different tool or target.

## Implementation steps

1. Inspect workspace and crate-level docs.

2. Inspect policy/enforcement modules and summarize the current enforcement path.

3. Inspect CLI and TUI entrypoints and document manual/strict posture selection.

4. Inspect MCP/agent/API paths and document strict posture selection.

5. Inspect tool dispatch and identify all raw/enforced dispatch uses.

6. Inspect command handlers for high-risk domains and map their descriptor/enforcement calls.

7. Write `docs/ARCHITECTURE.md` using the structure above.

8. Add or update a concise README pointer to the architecture document if appropriate.

9. Add lightweight tests only if an obvious existing invariant has no coverage and can be covered without refactoring.

10. Run formatting and tests appropriate to the scope of changes.

## Validation commands

Run at minimum:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo test -p eggsec --lib
cargo test -p eggsec-tui --lib
```

If feature combinations are already known to be expensive or platform-sensitive, record skipped commands and why. Do not hide failures.

## Non-goals

Do not extract domains in this phase.

Do not rewrite command dispatch in this phase.

Do not redesign the policy model in this phase.

Do not change manual CLI/TUI semantics except to fix obvious documentation mismatches.

Do not expose new MCP/agent tools.

## Acceptance criteria

- `docs/ARCHITECTURE.md` accurately reflects the current workspace and execution surfaces.
- The side-effecting execution path inventory exists and covers the major operation families.
- Transitional APIs are explicitly listed with recommended disposition.
- Architecture invariants are stated clearly enough to drive later tests and refactors.
- Existing tests/checks still pass or failures are documented.

## Handoff notes for the next phase

Phase 2 should use the inventory from this phase as its worklist. The most important outputs for Phase 2 are the list of strict-surface dispatch paths, raw dispatch use sites, legacy direct scope helpers, and any command families that lack descriptor-based pre-dispatch enforcement.
