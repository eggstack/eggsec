# Credential Access Feature - Next Steps & Handoff Plan

**Status**: Active Handoff Plan  
**Based on**: `credential-access-implementation-plan.md`  
**Current Date**: 2026-06-11  
**Goal**: Complete the integration of the existing `auth/` module into Eggsec with proper safety, feature gating, CLI execution, profiles, and output support.

---

## 1. Current State Snapshot (as of latest inspection)

**Completed**:
- `crates/eggsec/src/cli/auth.rs` created with rich `AuthTestArgs` struct (flags for brute_force, credential_stuffing, mfa, lockout, rate_limit, session, timing, `--all`, wordlist support, etc.).
- `AuthTest(AuthTestArgs)` registered in `Commands` enum in `cli/mod.rs`.
- `ScanProfile::Auth` variant added and partially wired into profile methods.
- Core `crates/eggsec/src/auth/` module (AuthEngine + all testers) remains intact and powerful.

**Not Yet Implemented**:
- `credential-testing` feature flag.
- Actual execution logic that calls `AuthEngine` from the CLI.
- Safety model integration (`EnforcementContext`, scope rules for auth targets, capability allowlisting).
- Findings / output conversion for `AuthTestReport`.
- Dedicated regression profiles (`auth-validation`, `credential-regression`).
- Pipeline wiring for the new `Auth` profile.
- Documentation updates.

**Overall Progress**: ~25-35% (mostly scaffolding). Strong foundation exists; integration layer is the remaining work.

---

## 2. Recommended Implementation Order (Prioritized)

### Phase 1: Foundation & Safety (Highest Priority - Do This First)
1. Add `credential-testing` feature flag.
2. Gate the auth CLI module and heavy auth operations behind the flag.
3. Implement basic safety guardrails (scope checking + lab-only defaults).
4. Wire minimal execution path (even if limited to safer tests initially).

### Phase 2: Core Execution & Output
5. Fully connect `AuthTest` command to `AuthEngine`.
6. Implement findings conversion so auth results appear in reports/SARIF/JUnit.
7. Add basic wordlist loading with size limits and security.

### Phase 3: Profiles & Polish
8. Implement `auth-validation` and `credential-regression` profiles.
9. Wire `ScanProfile::Auth` into the main pipeline.
10. Enhance safety model (full EnforcementContext integration).
11. Documentation and examples.

---

## 3. Detailed Task Breakdown

### Task 1: Add Feature Flag (Start Here)

**Files to modify**:
- `crates/eggsec/Cargo.toml`

**Actions**:
- Add under `[features]`:
  ```toml
  credential-testing = []
  ```
- Optionally add it to the `full` meta-feature.
- In `cli/mod.rs`, gate the auth module with:
  ```rust
  #[cfg(feature = "credential-testing")]
  pub mod auth;
  #[cfg(feature = "credential-testing")]
  pub use auth::*;
  ```
- Gate the `AuthTest` variant in `Commands` enum similarly.

**Goal**: `cargo build --features credential-testing` should succeed cleanly.

### Task 2: Basic CLI Execution Wiring

**Files**:
- `crates/eggsec/src/cli/auth.rs` (extend)
- `crates/eggsec/src/cli/misc.rs` or new handler (recommended: keep logic in `auth.rs` or create `auth_runner.rs`)

**Actions**:
- Create a function `pub async fn run_auth_test(args: AuthTestArgs, config: &EggsecConfig) -> Result<()>`.
- Inside it:
  - Load wordlists (if provided) with size caps (e.g., max 10k entries).
  - Create `AuthEngine` with `max_attempts`, `concurrency`, `timeout_secs` from args.
  - Call appropriate tester(s) based on flags (`--all` runs full suite).
  - Print or write results (start with simple pretty print + JSON support).
- Respect `--dry-run` / plan mode if possible.
- Add strong confirmation prompt unless `--yes` is passed (for high-risk tests).

**Tip**: Start by supporting safer tests (rate_limit, mfa, password_policy, session) before enabling brute_force/credential_stuffing by default.

### Task 3: Safety Guardrails (Critical)

**Files**:
- `crates/eggsec/src/safety/` or wherever `EnforcementContext` lives
- `crates/eggsec/src/auth/mod.rs` (minor extensions)

**Actions**:
- Add `AuthOperation` variant or integrate auth tests into existing policy evaluation.
- For aggressive operations (`BruteForce`, `CredentialStuffing`):
  - Default to `lab_only = true`.
  - Require explicit scope match on target + test accounts.
  - Use `--allow-high-risk` + reason for manual override (already partially supported globally).
- Log every auth attempt with policy decision context.
- In strict/CI/agent modes: hard deny unless capability is explicitly allowed.

**Quick Win**: Even simple target allow-list checking in the runner is valuable at this stage.

### Task 4: Findings & Output Conversion

**Files**:
- `crates/eggsec/src/auth/convert.rs` (new file recommended)
- Or extend `crates/eggsec/src/auth/mod.rs`

**Actions**:
- Implement:
  ```rust
  impl From<AuthFinding> for FindingData { ... }
  pub fn auth_report_to_scan_report_data(report: &AuthTestReport) -> ScanReportData { ... }
  ```
- Map categories to `"authentication"` or `"credential-access"`.
- Populate severity, remediation, and (where possible) CVSS/CWE.
- Wire this into the auth runner so results flow into normal output paths.

### Task 5: Profile Implementation

**Files**:
- Pipeline / profile definition files (likely in `src/pipeline/` or `src/config/`)
- `cli/scan.rs` (for profile handling)

**Actions**:
- Define two new profiles:
  - `auth-validation`: Safer tests (MFA, rate limit, password policy, session, timing) + endpoint discovery + WAF analysis.
  - `credential-regression`: Controlled brute/stuffing against known lab test accounts + baseline comparison.
- Add them to `ScanProfile` enum if not already fully supported.
- Wire basic stage execution in the scan pipeline.

### Task 6: Documentation Handoff

**Files to create/update**:
- `docs/AUTH_LAB.md` (new - high priority)
- `README.md`
- `docs/CAPABILITIES.md`
- `docs/SAFETY.md`
- `CHANGELOG.md`

**Content for `docs/AUTH_LAB.md`** (recommended structure):
- Overview and defense-focused philosophy
- Safety model and scope requirements
- Example scope file entries for auth targets
- Recommended lab setup (test accounts, rate limiting expectations)
- Command examples
- Interpreting results for defense improvement
- Warnings and responsible use

---

## 4. Suggested File Changes Summary

| File | Change Type | Priority |
|------|-------------|----------|
| `crates/eggsec/Cargo.toml` | Add feature flag | High |
| `crates/eggsec/src/cli/mod.rs` | Gate auth module (conditional compilation) | High |
| `crates/eggsec/src/cli/auth.rs` | Add runner function + execution logic | High |
| `crates/eggsec/src/auth/mod.rs` or new `convert.rs` | Add findings conversion | Medium |
| `crates/eggsec/src/safety/...` | Policy integration for auth ops | High |
| Pipeline/profile files | Add auth-validation & credential-regression | Medium |
| `docs/AUTH_LAB.md` | Create new file | Medium |

---

## 5. Testing & Verification Checklist

After each phase, verify:
- [ ] `cargo check --features credential-testing` passes cleanly.
- [ ] `eggsec auth-test --help` shows the command and all flags.
- [ ] Running against a lab target with `--yes` produces structured output.
- [ ] High-risk tests are blocked or require explicit override without proper scope.
- [ ] Results appear in JSON and (later) SARIF/JUnit output.
- [ ] Existing tests in `auth/` still pass.
- [ ] No regression in other scan profiles.

**Recommended Test Targets** (lab only):
- DVWA or similar vulnerable login form
- Custom test auth service with known lockout/MFA behavior

---

## 6. Risks & Mitigations

- **Risk**: Accidentally enabling powerful auth testing without safety → **Mitigation**: Feature flag first + lab-only defaults + confirmation prompts.
- **Risk**: Large wordlists causing performance issues → **Mitigation**: Hard size limits + concurrency caps in runner.
- **Risk**: Policy integration complexity → **Mitigation**: Start simple (target + test account scope check) and iterate.

---

## 7. Handoff Notes for Implementer

- The existing `AuthEngine` and testers are mature — focus on **calling** them rather than modifying them heavily.
- Prioritize safety and auditability over feature completeness in early iterations.
- Use the global `--allow-high-risk` and `--yes` mechanisms where possible instead of inventing new flags.
- Keep the defense-validation framing consistent ("test your own controls" language in help text and docs).
- Once basic execution works, the findings conversion and profile work will feel much easier.

---

## 8. Success Criteria for This Handoff Plan

By the end of this work:
- `eggsec auth-test` command works end-to-end on lab targets.
- Feature is properly gated and safe by default.
- Auth results integrate with the rest of Eggsec's reporting and pipeline system.
- Clear documentation exists for safe, defense-oriented usage.
- The feature is ready for deeper profile and agent/MCP integration in a follow-up cycle.

---

**This plan is intentionally more tactical and file-specific than the original high-level plan, designed for direct handoff and incremental progress.**

Next action: Pick Task 1 (feature flag) and implement it. Then move to Task 2.