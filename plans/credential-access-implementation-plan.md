# Credential Access Feature Set Implementation Plan

**Status**: Draft / Ready for Implementation  
**Target Branch**: `main`  
**Related Issues**: (to be created)  
**Priority**: High (leverages existing `auth/` foundation)  
**Owner**: (to be assigned)  
**Estimated Effort**: 3–6 weeks for core integration + safety hardening (Phases 1–2)

---

## 1. Executive Summary

This plan details the implementation of a first-class **Credential Access / Authentication Testing** feature set for Eggsec. The goal is to expose, harden, and deeply integrate the already substantial `crates/eggsec/src/auth/` module into Eggsec’s CLI, profiles, safety model, and agent/MCP workflows.

**Primary Use Case (Defense Perspective)**: Controlled, repeatable validation of authentication defenses in lab and authorized environments. This includes testing account lockout policies, rate limiting effectiveness, MFA enforcement, WAF rules around auth endpoints, session management, and password policy strength — with full auditability and regression capabilities.

**Key Principles**:
- **Defense-validation first**: Focus on testing *your own defenses*, not unrestricted offensive attacks.
- **Strict safety model**: Feature-gated, scope-enforced, budget-capped, lab-only for aggressive tests.
- **Leverage existing code**: The `auth/` module (with `AuthEngine`, `BruteForceTester`, `CredentialStuffer`, `LockoutDetector`, `MfaTester`, etc.) already provides a strong foundation.
- **Native integration**: Reuse Eggsec’s pipeline profiles, findings system, output formats (SARIF/JUnit/JSON), scope enforcement, and dry-run planning.
- **Agent & CI friendly**: Full support for MCP/agent execution and CI regression gates.

This feature will significantly round out Eggsec’s network defense assessment capabilities alongside existing recon, web fuzzing, WAF, and wireless modules.

---

## 2. Current State Analysis

### Existing Foundation (`crates/eggsec/src/auth/`)

The module is already well-structured:

- **Core types**: `AuthEngine`, `AuthTestReport`, `AuthFinding`, `AuthTestType` enum.
- **Submodules**:
  - `brute_force.rs` — `BruteForceTester`
  - `credential_stuffing.rs` — `CredentialStuffer`, `CredentialPair`
  - `lockout.rs` — `LockoutDetector`
  - `mfa.rs` — `MfaTester`
  - `password_policy.rs` — `PasswordPolicyTester`, `PasswordPolicyResult`
  - `rate_limit.rs` — `RateLimitTester`
  - `session.rs` — `SessionTester`
  - `timing.rs` — `TimingTester`
  - `multi_protocol.rs` (behind `nse-ssh2` feature)
- **Safety mechanisms**: `max_attempts`, `stop_on_lockout`, `concurrency`, `timeout_secs`, `AtomicBool` stop flag, `AtomicUsize` attempt counter.
- **Orchestration**: `AuthEngine::run_full_test()` and individual testers.
- **Banner**: Strong `AUTH_BANNER` warning.

### Gaps

1. **Not exposed in main CLI** — No `eggsec auth ...` or `eggsec credential ...` commands.
2. **No feature flag** — Currently always compiled (or behind unrelated features).
3. **Limited safety integration** — Does not fully participate in `EnforcementContext`, scope provenance checks, or profile-based capability allowlisting.
4. **No dedicated profiles** — Missing `auth-validation`, `credential-regression`, etc.
5. **No auto-discovery integration** — Does not leverage recon/JS analysis or endpoint discovery to find login forms/APIs.
6. **Incomplete output integration** — `AuthTestReport` exists but is not wired into the main `output::convert` system or findings lifecycle.
7. **Documentation & UX** — Not mentioned in README, CAPABILITIES.md, or SAFETY.md at the same level as other modules.

**Conclusion**: ~60-70% of the core logic exists. The majority of work is **integration, safety hardening, exposure, and defense-oriented framing**.

---

## 3. Design Principles & Constraints

- **Defense-lab default**: Aggressive tests (brute force, stuffing) only available under `defense-lab` profiles or explicit manual override with audit.
- **Explicit scope required**: Targets + test accounts must be declared in scope files for networked auth tests.
- **Budget & rate limiting**: Hard caps on attempts, concurrency, and duration. Stop flags must be respected.
- **Audit everything**: Every attempt logged with policy decision context.
- **Feature gated**: New `credential-testing` feature flag (split from core if needed).
- **No internet-wide attacks**: Scope enforcement + lab-only defaults prevent this.
- **Composable**: Can be called from pipelines, agents, or standalone.
- **Output parity**: Full support for JSON, SARIF, JUnit, HTML, Markdown via existing converters.

---

## 4. Proposed Architecture

### 4.1 Feature Flag
- Add `credential-testing` to `Cargo.toml` (under `[features]` in workspace and `eggsec` crate).
- Gate the entire `auth` module usage behind this flag where appropriate (keep basic types available if needed for other modules).

### 4.2 Safety & Enforcement Integration
- Extend `EnforcementContext` (or create `AuthOperation` variant) to evaluate auth-related operations.
- Add capability: `AuthTesting` (with sub-capabilities: `BruteForce`, `CredentialStuffing`, `MfaTesting`, etc.).
- In strict/CI/agent modes: Only allow if explicit scope manifest + `allowed_capabilities` includes auth testing.
- In `ManualPermissive`: Warn + require confirmation for high-risk auth tests.
- Add `defense-lab` profile special casing that relaxes some limits while still enforcing scope and budgets.

### 4.3 Module Structure
Keep and enhance `crates/eggsec/src/auth/`:
- Minor additions for better integration (e.g., `AuthTestConfig`, structured findings conversion).
- New `auth_integration.rs` or extend existing for pipeline/profile use.

### 4.4 CLI Layer
New commands under `eggsec auth` (or `eggsec credential` as alias):
- `eggsec auth test <target> --type <brute|stuffing|mfa|full>`
- `eggsec auth validate-policy <target>`
- `eggsec auth regression <target> --baseline <file>`

### 4.5 Profile Integration
Add to pipeline profiles (in `src/pipeline/` or config):
- `auth-validation`
- `credential-regression`

These profiles will chain recon (endpoint discovery) → auth testing → WAF analysis → reporting.

### 4.6 Findings & Output
- Convert `AuthFinding` and `AuthTestReport` into the canonical `FindingData` / `ScanReportData` structures.
- Map to appropriate categories ("authentication", "credential-access").
- Support CVSS scoring and compliance mappings (e.g., NIST AC, CIS).

### 4.7 Agent / MCP Support
- Expose auth testers as MCP tools (with strict safety profile).
- Add agent skills for continuous auth posture monitoring.

---

## 5. Detailed Implementation Tasks

### Phase 1: Foundation & Safety Hardening (1–2 weeks)

**Task 1.1: Feature Flag**
- In root `Cargo.toml` and `crates/eggsec/Cargo.toml`, add:
  ```toml
  [features]
  credential-testing = []
  ```
- Gate relevant `auth` usage and CLI registration behind `#[cfg(feature = "credential-testing")]`.

**Task 1.2: Enhance `AuthEngine` & Safety**
- Add constructor that accepts `EnforcementContext` or policy decision reference.
- Strengthen `increment_attempts()` and stop logic to integrate with global budget system.
- Add `lab_only: bool` flag (default true for aggressive tests).
- Expose `AuthTestConfig` struct for profile-driven configuration.

**Task 1.3: Scope & Policy Integration**
- In `src/safety/` or `EnforcementContext`:
  - Add `AuthOperation` enum variant.
  - Implement evaluation that checks scope for target + test accounts.
  - Require explicit `allowed_capabilities` containing `AuthTesting` for strict modes.
- Update `policy-explain` and `scope-explain` to cover auth operations.

**Task 1.4: Findings Conversion**
- Create or extend `src/auth/convert.rs` (or add methods):
  - `impl From<AuthFinding> for FindingData`
  - `fn to_scan_report_data(report: &AuthTestReport) -> ScanReportData`
- Ensure CWE/CVSS/compliance fields are populated.

### Phase 2: CLI, Profiles & Basic Integration (1–2 weeks)

**Task 2.1: CLI Commands**
- In `src/cli/` and `src/commands/`:
  - Add `AuthArgs`, `AuthSubcommands` (test, validate-policy, regression).
  - Implement `run_auth_command()` that:
    - Loads wordlists (with size limits and secure handling).
    - Creates `AuthEngine` with profile-derived budgets.
    - Calls appropriate tester(s).
    - Converts results to output format.
    - Respects `--dry-run` / plan mode.
- Add strong interactive confirmation for high-risk operations in manual mode.

**Task 2.2: New Profiles**
- Define in pipeline/profile system:
  - `auth-validation`: Endpoint discovery + safe auth tests (MFA, rate limit, password policy, session) + WAF analysis.
  - `credential-regression`: Full controlled brute/stuffing against lab test accounts + baseline comparison.
- Wire into `eggsec scan --profile auth-validation` etc.

**Task 2.3: Auto-Discovery Integration**
- In recon or endpoint discovery modules, detect login forms/APIs (common paths + form detection).
- Pass discovered endpoints to auth testing when profile requests it.

**Task 2.4: Wordlist & Configuration Handling**
- Add secure wordlist loading (size caps, validation).
- Support profile-embedded small test wordlists + external file loading (with scope checks).

### Phase 3: Advanced Features & Polish (1 week)

- Full WAF evasion testing during auth attempts (integrate with `waf` module).
- Timing attack enhancements and better side-channel detection.
- Multi-protocol auth testing (expand `multi_protocol` behind feature).
- Baseline diffing for regression profiles (compare `AuthTestReport` over time).
- TUI views for live auth testing progress.
- MCP tool definitions for auth operations (with strict safety profile).

### Phase 4: Documentation & Release (0.5 week)
- Update `README.md`, `docs/CAPABILITIES.md`, `docs/SAFETY.md`.
- Add new `docs/AUTH_LAB.md` with examples, safety guidance, and lab setup.
- Add examples in `examples/` (e.g., `scope-lab-auth.toml`, sample profiles).
- Update `CHANGELOG.md`.

---

## 6. Safety & Scope Model (Critical Section)

### 6.1 Scope File Extensions
Recommend adding optional sections to scope TOML:
```toml
[[allowed_auth_targets]]
target = "https://lab.example.com/login"
test_accounts = ["testuser1", "testuser2"]
description = "Lab authentication endpoint"

[[allowed_auth_targets]]
target = "https://lab-api.example.com/auth"
methods = ["POST"]
```

### 6.2 Capability Model
- New capability: `AuthTesting`
- Sub-capabilities for granular control: `BruteForce`, `CredentialStuffing`, `MfaBypassTesting`, etc.
- In `ManualPermissive`: High-risk auth tests require explicit `--allow-high-risk` + reason (audited).
- In strict/CI/agent/MCP: Hard deny unless explicit scope manifest + capability allowlist.

### 6.3 Budgets & Guardrails
- `max_attempts` enforced globally per run.
- Concurrency limits (default low, e.g., 5–10).
- Automatic stop on detected lockout (configurable).
- Rate limiting between attempts (jitter + minimum delay).
- All attempts logged with full context (target, username attempted, response code/time, policy decision).

### 6.4 Lab-Only Defaults
- Aggressive tests (`BruteForce`, `CredentialStuffing`) default to `lab_only = true`.
- Non-lab targets rejected unless explicit manual override with justification.

---

## 7. CLI Surface (Proposed)

```bash
# Basic validation (safer tests)
eggsec auth validate-policy https://lab.example.com/login

# Full auth test suite (defense-lab profile recommended)
eggsec auth test https://lab.example.com/login --profile auth-validation --json

# Controlled credential regression against known test accounts
eggsec auth regression https://lab.example.com/login --baseline baseline-auth.json --wordlist lab-passwords.txt

# Dry-run planning
eggsec auth plan https://lab.example.com/login --type full

# With explicit scope
eggsec auth test https://lab.example.com/login --scope scope-lab.toml --yes
```

**Key Flags**:
- `--type brute|stuffing|mfa|full|policy`
- `--max-attempts`
- `--concurrency`
- `--wordlist-users`, `--wordlist-passwords`
- `--lab-only` / `--allow-production-auth` (dangerous, audited)
- `--baseline` for regression

---

## 8. New / Updated Profiles

Add to the 16 existing profiles:

| Profile                | Stages                                      | Risk Level | Notes |
|------------------------|---------------------------------------------|------------|-------|
| `auth-validation`     | Endpoint discovery + safe auth tests (MFA, rate limit, policy, session) + WAF | Low–Medium | Recommended default |
| `credential-regression` | Recon + controlled brute/stuffing + lockout validation + baseline diff | Medium (lab only) | Requires explicit test accounts in scope |

These profiles should be usable via:
```bash
eggsec scan <target> --profile auth-validation --scope scope.toml
```

---

## 9. Testing Strategy

- **Unit tests**: Expand existing tests in `auth/` submodules. Add tests for new integration points and safety logic.
- **Integration tests**: Lab-based tests using Docker Compose vulnerable apps (DVWA, Juice Shop, or custom test auth service). Run under `defense-lab` profile.
- **Safety tests**: Verify scope rejection, budget enforcement, stop flags, and policy decisions.
- **Regression tests**: Ensure existing pipeline profiles and output formats continue to work.
- **Fuzzing/Auth-specific**: Test wordlist loading edge cases and response parsing.

Add CI jobs that run auth tests in isolated lab containers.

---

## 10. Documentation Updates

- `README.md`: Add section under "Core Capabilities" and "Lab Defense Commands".
- `docs/CAPABILITIES.md`: Add detailed table for auth module (testers, supported test types).
- `docs/SAFETY.md`: New subsection on `AuthTesting` operations and required scope/capability rules.
- **New file**: `docs/AUTH_LAB.md` — Comprehensive guide with:
  - Recommended lab setup
  - Example scope files
  - How to run regression suites
  - Interpreting results for defense improvement
  - Warnings and responsible use
- Update `CONTRIBUTING.md` if new patterns are introduced.

---

## 11. Phased Implementation Roadmap

**Phase 1 (Foundation)**: Feature flag + safety/policy integration + findings conversion. (Core safety model complete)
**Phase 2 (Exposure)**: CLI commands + profile definitions + basic auto-discovery. (Usable end-to-end)
**Phase 3 (Polish)**: WAF integration, baseline diffing, TUI, MCP tools, advanced testers.
**Phase 4 (Docs & Release)**: Full documentation, examples, changelog, announcement.

**Milestones**:
- End of Phase 1: `cargo build --features credential-testing` succeeds and basic safety checks pass.
- End of Phase 2: `eggsec auth test ...` works in lab with scope enforcement.
- End of Phase 3: Full regression profile works with baseline comparison.

---

## 12. Risks, Mitigations & Open Questions

**Risks**:
- Misuse for unauthorized attacks → Strong scope enforcement + prominent warnings + lab-only defaults + audit logging.
- Performance impact on large wordlists → Enforce size limits + concurrency caps.
- Complexity of policy integration → Start conservative; iterate on `EnforcementContext`.

**Open Questions** (to resolve during implementation):
1. Should `credential-testing` be part of the `full` meta-feature or kept separate?
2. Exact granularity of sub-capabilities for fine-grained agent/MCP control.
3. Preferred command namespace: `auth` vs `credential` (recommend `auth` for consistency with module name).
4. How deeply to integrate wordlist management (bundled small lists vs external files only).

---

## 13. Success Criteria

- Existing `auth/` tests continue to pass.
- New feature can be built with `cargo build --features credential-testing`.
- `eggsec auth test` and `eggsec scan --profile auth-validation` work end-to-end in lab with proper scope.
- All outputs (JSON/SARIF/JUnit) contain auth findings.
- Safety model prevents execution against out-of-scope targets.
- Documentation clearly explains defense-focused usage and risks.

---

**Next Steps After Approval**:
1. Create GitHub issue(s) from this plan.
2. Assign implementation tasks.
3. Begin Phase 1 development.

---

*This plan is designed to be detailed enough for a smaller model or junior developer to implement cleanly while maintaining Eggsec’s high standards for safety, structure, and defense-validation focus.*