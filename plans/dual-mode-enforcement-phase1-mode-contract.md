# Phase 1 Handoff Plan: Mode Contract Documentation

## Goal

Create the canonical architectural contract for Eggsec's dual-mode enforcement model. This phase is documentation-first, but it is not cosmetic. The document should become the source of truth for all later implementation phases and should prevent future hardening work from accidentally making manual CLI/TUI operation behave like MCP or autonomous agent operation.

Eggsec should have one shared enforcement vocabulary and two distinct operating postures:

- Manual operator posture: CLI/TUI, operator-directed, warning/confirmation/override capable, auditable, practical.
- Automated agent posture: MCP/security-agent/CI/programmatic noninteractive execution, strict, explicitly scoped, non-overridable, revalidated before dispatch.

## Rationale

The current code already has strong primitives: `ExecutionProfile`, `OperationDescriptor`, `EnforcementContext`, `LoadedScope`, `ManualOverride`, and `EnforcementOutcome`. What is missing is a concise written contract that explains how those pieces should behave per surface. Without that contract, future changes can drift in two bad directions:

1. Over-hardening manual use until Eggsec becomes too gated to compete with established legitimate security tools.
2. Under-hardening agent use by letting MCP or the security agent inherit human discretion.

This phase defines the boundary before more code is added.

## Files to create or update

Create:

- `docs/ENFORCEMENT_MODES.md`

Optionally update:

- `README.md` safety section, only to link to the new document.
- `docs/SAFETY.md`, only to cross-link or remove duplicate/conflicting language.

Do not refactor code in this phase unless a small doc-link compile/doc test issue requires it.

## Required content for `docs/ENFORCEMENT_MODES.md`

### 1. Opening summary

Explain that Eggsec intentionally supports two usage families:

- Human/manual security assessment through CLI and TUI.
- Agent/programmatic execution through MCP, security agent, CI, and similar noninteractive surfaces.

State explicitly that manual mode is not meant to be agent-strict by default. Manual operators may proceed through warnings and explicit confirmations where appropriate. Agents must not.

### 2. Terminology

Define these terms in repo language:

- Execution surface: where the request originates, such as CLI, TUI, MCP, security agent, CI, REST.
- Execution profile: enforcement behavior, currently represented by `ExecutionProfile`.
- Manual permissive: human-directed default mode.
- Manual guarded: strict human mode, equivalent to CLI `--strict-scope` and future TUI guarded toggle.
- Agent strict: noninteractive/model-controlled strict posture.
- Scope provenance: whether scope came from an explicit manifest versus default empty/config fallback.
- Manual override: explicit operator acceptance of specific confirmation classes, only valid in manual permissive.
- Confirmation class: machine-readable class requiring explicit operator action.

### 3. Surface behavior matrix

Include a table with at least these rows:

- CLI default/manual.
- CLI `--strict-scope`.
- TUI default/manual.
- TUI guarded.
- MCP server.
- Security agent.
- CI.
- REST API.

Columns should include:

- Intended posture.
- Expected `ExecutionProfile`.
- Explicit scope manifest requirement.
- Whether `Warn` may dispatch.
- Whether `RequireConfirmation` may dispatch after override.
- Whether manual override flags are honored.
- Whether policy is re-evaluated immediately before dispatch.

Expected values:

- Manual default/TUI default: `ManualPermissive`, warnings may dispatch, confirmation may dispatch only with matching explicit manual override.
- Manual guarded/TUI guarded: `ManualGuarded`, no confirmation dispatch, no overrides.
- MCP: `McpStrict`, explicit manifest required for networked operations, no overrides.
- Security agent: `AgentStrict`, explicit manifest required for networked operations, no overrides.
- CI: `CiStrict`, explicit manifest required where target/networked, no overrides.
- REST: strict by default unless a future explicitly named local/manual API mode is implemented.

### 4. Outcome semantics

Document `EnforcementOutcome` semantics:

- `Allow`: dispatch is permitted.
- `Warn`: dispatch is permitted only in manual-permissive contexts; warnings must be visible/audited.
- `RequireConfirmation`: dispatch is permitted only in manual-permissive contexts and only after matching manual override classes are present.
- `Deny`: dispatch is never permitted.

State the invariant: automated surfaces must treat `Warn` conservatively and must treat `RequireConfirmation` as denial.

### 5. Manual discretion classes

Document the manual confirmation classes and expected override behavior:

- `OutOfScope`: may be operator-confirmed in manual permissive.
- `TargetExpansion`: may be operator-confirmed in manual permissive.
- `HighRisk`: requires dedicated high-risk flag/reason.
- `NonBaselineCapability`: requires dedicated nonbaseline capability flag.
- `PrivateResolution`: requires dedicated private-resolution flag.
- `CrossHostRedirect`: requires dedicated cross-host redirect flag.
- `TrafficInterception`: requires dedicated web-proxy/interception flag.
- `ExplicitExclusion`: decide and document current intended behavior. If permitted manually, it must require a dedicated explicit-exclusion flag and audit reason. If not permitted, document it as hard deny.

Document that `--yes` must remain narrow. It may suppress low-risk manual prompts for classes such as `OutOfScope`/`TargetExpansion`, but must not authorize high-risk, private-resolution, cross-host redirect, nonbaseline capability, traffic interception, or explicit exclusion.

### 6. Hard-deny classes

Document classes that should not be converted to manual confirmation:

- Missing compile-time feature.
- Invalid target.
- Scope parse/check error.
- Capability explicitly denied by policy.
- Risk not allowed by execution policy.
- Missing explicit scope manifest in automated mode.
- Agent/model-supplied override attempt.

### 7. Policy invariants

Include these invariants verbatim or near-verbatim:

- Manual permissive behavior must not bleed into MCP, security agent, CI, or strict REST.
- Agent strict behavior must not become the default for normal CLI/TUI manual use.
- Manual override flags are only honored in manual permissive contexts.
- Scope provenance for automated networked execution must come from `LoadedScope`, not raw `Scope`.
- Every dispatch path must eventually flow through a shared enforcement evaluation.
- Agent/MCP dispatch must re-evaluate enforcement immediately before dispatch.
- Programmatic constructors for agent-facing servers should require explicit enforcement context or be clearly test-only.

### 8. Examples

Include short examples of expected behavior:

- CLI manual scan with missing scope: warning, not hard denial, if safe.
- CLI manual positive allowlist miss: confirmation required.
- CLI strict positive allowlist miss: denial.
- MCP missing explicit manifest: denial.
- Security agent with explicit manifest but high-risk nonbaseline capability not allowlisted: denial.
- TUI manual high-risk action: preflight shows confirmation required and needed flag/action.

## Acceptance criteria

- `docs/ENFORCEMENT_MODES.md` exists and can be read without code context.
- The doc explicitly distinguishes manual operator posture from automated agent posture.
- The doc states that manual CLI/TUI should remain productive and should not inherit agent-grade strictness by default.
- The doc states that MCP and security agent must never honor manual overrides.
- The doc defines outcome semantics and confirmation classes.
- README or `docs/SAFETY.md` links to the new document without duplicating large sections.
- No implementation behavior changes are introduced in this phase except documentation links.

## Suggested validation

Run:

```bash
cargo fmt --all --check
cargo test -p eggsec --lib config::policy_decision
```

The cargo commands are mostly sanity checks; this phase should not require code changes.

## Non-goals

- Do not introduce `ExecutionSurface` yet.
- Do not change CLI, TUI, MCP, REST, or agent behavior yet.
- Do not rewrite policy evaluator logic.
- Do not add a large new test matrix yet.

## Follow-on phases

Phase 2 should implement `ExecutionSurface` and derive enforcement contexts from it.

Phase 3 should correct security-agent strictness and add defense-in-depth checks so `eggsec agent` never inherits manual permissive enforcement.
