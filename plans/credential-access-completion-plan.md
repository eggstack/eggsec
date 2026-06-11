# Credential Access Feature - Completion Plan

**Status**: Final Integration & Polish Phase  
**Date**: 2026-06-11  
**Goal**: Bring the `auth-test` capability to a production-ready, well-integrated state.

**SUPERSEDED / COMPLETED UNDER ADOPTED MODEL (2026-06-11)**: The broader items listed below (Findings & Output Conversion of `AuthFinding`/`AuthTestReport` into `ScanReportData`/`eggsec-output` canonical types + SARIF/JUnit/etc.; dedicated `auth-validation`/`credential-regression` pipeline profiles; subcommand hierarchy; `credential-testing` Cargo feature; `AuthFinding` conversion helpers) were **intentionally not implemented**. The final adopted model uses:
- Runtime policy gate only: `OperationRisk::CredentialTesting` + `allow_credential_testing` flag in `ExecutionPolicy` (default false) + central `EnforcementContext::evaluate()` via `CommandContext::evaluate_and_enforce_operation` (post-2026-06-10 handler policy alignment). Strict profiles (MCP/agent/CI) and `--strict-scope` treat high-risk as hard Deny; ManualPermissive surfaces `RequireConfirmation(HighRisk)` (honored only with `--allow-high-risk` or equivalent audited override).
- Local types only: `AuthTestReport`/`AuthFinding` (defined in `auth/mod.rs`) emitted directly by the handler as pretty text or `--json` (no `FindingData`, `ScanReportData`, `StoredFinding`, or `eggsec-output` converters).
- Standalone CLI: `eggsec auth-test` (defense-lab / high-risk credential control validation only; distinct from pipeline `ScanProfile::Auth` which is JWT/OAuth/IDOR fuzzer-focused via PortScan+Fingerprint+EndpointScan+Fuzz stages). TUI `AuthTab` exists as standalone code but is excluded from the main `Tab` enum (CLI-only surface).
- No dedicated Cargo feature (auth always compiled; control is purely runtime policy + scope + explicit test accounts).

See resolution notes in `plans/credential-access-implementation-plan.md` (historical, superseded), `plans/credential-access-next-steps.md`, `architecture/auth.md:72-76`, `CHANGELOG.md` (Unreleased Security), `docs/AUTH_LAB.md`, and `.opencode/skills/eggsec-auth/SKILL.md`. All relevant tests (17 wiremock auth_tests + enforcement credential_testing + policy contracts + pipeline stages) pass green. No code changes or further integration required to "close" this plan under the adopted model. The file is retained for historical context only.

---

## 1. Executive Summary & Current State

Significant progress has been made:
- A fully functional `handle_auth_test()` exists in `crates/eggsec/src/commands/handlers/auth_test.rs`.
- Safety integration via `evaluate_and_enforce_operation()` with `OperationRisk::CredentialTesting` is in place.
- Findings are being collected internally (local `AuthFinding` only).
- `docs/AUTH_LAB.md` provides solid guidance.

**Remaining Work** (in priority order; see note above — these were superseded by the adopted runtime-policy + local-findings model and are documented as intentionally not implemented):
1. **Findings & Output Conversion** – (Not pursued) Make auth results flow into the standard reporting system (SARIF, JUnit, HTML, etc.).
2. **Safety Hardening & Verification** – Ensure scope, policy, and risk enforcement are robust. (Completed under central `EnforcementContext`.)
3. **Profile Pipeline Integration** – (Not pursued) Fully enable `auth-validation` and `credential-regression` profiles.
4. **Polish & Testing** – Robustness, error handling, and end-to-end validation. (Tests green.)
5. **Documentation** – Update main docs to reflect the completed feature. (Done; see cross-refs in architecture/auth.md, CHANGELOG, etc.)

---

## 2. Prioritized Task List

### Task 1: Findings & Output Conversion (Highest Priority)

**Objective**: Convert internal `AuthFinding` / `AuthTestReport` into the canonical findings system.

**Files to Modify**:
- `crates/eggsec/src/commands/handlers/auth_test.rs`
- `crates/eggsec/src/auth/mod.rs` (or create `crates/eggsec/src/auth/convert.rs`)

**Implementation Steps**:
1. Create conversion functions:
   ```rust
   pub fn auth_finding_to_finding_data(finding: &AuthFinding) -> FindingData {
       FindingData {
           title: finding.title.clone(),
           severity: finding.severity.as_str().to_string(),
           category: "authentication".to_string(),
           description: finding.description.clone(),
           remediation: Some(finding.recommendation.clone()),
           // ... other fields
       }
   }
   ```
2. Add a function to convert the full report:
   ```rust
   pub fn auth_test_report_to_scan_report_data(report: &AuthTestReport) -> ScanReportData { ... }
   ```
3. In `handle_auth_test()`, after building the report, convert findings and pass them to the output system (or store in `CommandContext`).
4. Ensure both pretty-printed and JSON output still work, while also feeding structured findings.

**Success Criteria**: `eggsec auth-test ... --json` output can be consumed by the existing report pipeline, and findings appear in SARIF/JUnit when using `--profile` or report commands.

### Task 2: Safety Enforcement Hardening & Verification

**Objective**: Make safety guarantees explicit and robust inside the handler.

**Files**:
- `crates/eggsec/src/commands/handlers/auth_test.rs`
- `crates/eggsec/src/config/policy.rs` (verify `OperationRisk` definitions)

**Implementation Steps**:
1. Review and enhance the `evaluate_and_enforce_operation` call:
   - Ensure `requires_explicit_scope: true` for aggressive tests.
   - Add validation that test usernames (if provided) are covered by scope.
2. For high-risk operations (`brute_force`, `credential_stuffing`):
   - Require `OperationRisk::CredentialTesting` + explicit `--allow-high-risk`.
   - Enforce hard caps on attempts.
3. Add better logging of policy decisions (what was allowed/denied and why).
4. Test edge cases:
   - Target not in scope
   - No `allow_credential_testing` in config
   - Strict profile modes (CI, agent, MCP)

**Success Criteria**: Unauthorized or out-of-scope auth tests are cleanly blocked with clear messages.

### Task 3: Profile Pipeline Integration

**Objective**: Make the dedicated auth profiles fully functional.

**Files**:
- Pipeline / profile definition code
- `crates/eggsec/src/commands/handlers/scan.rs`
- `cli/scan.rs` (profile handling)

**Implementation Steps**:
1. Define clear stage sequences for:
   - `auth-validation` (safer tests + recon + WAF analysis)
   - `credential-regression` (controlled high-risk tests + baseline comparison)
2. Wire these profiles into the main scan pipeline so `eggsec scan --profile auth-validation` works end-to-end.
3. Update `ScanProfile` methods if needed (`max_risk_budget`, `intended_uses`, etc.).

### Task 4: Polish & Robustness

**Actions**:
- Improve `load_passwords()` function (better error messages, support for larger secure wordlists).
- Add progress indicators for long-running tests.
- Improve error handling and user-friendly messages.
- Consider adding a `--dry-run` mode.
- Add basic TUI progress support (lower priority).

### Task 5: Documentation & Communication

**Files to Update**:
- `README.md`
- `docs/CAPABILITIES.md`
- `docs/SAFETY.md`
- `CHANGELOG.md`

**Content to Add**:
- Command reference for `auth-test`
- Examples in lab/defense sections
- Risk tier explanation for `CredentialTesting`
- Link to `docs/AUTH_LAB.md`

---

## 3. Recommended Execution Order

1. **Task 1** (Findings conversion) – (Superseded; not pursued under adopted model — local `Auth*` types + direct handler emission only.)
2. **Task 2** (Safety hardening) – (Completed: central `EnforcementContext` + `CredentialTesting` risk + scope + explicit overrides + tests.)
3. **Task 3** (Profile integration) – (Superseded; not pursued — no `auth-validation`/`credential-regression` profiles; `ScanProfile::Auth` is a distinct JWT/OAuth/IDOR fuzzer pipeline.)
4. **Task 4 + 5** (Polish + Docs) – (Completed: tests green; docs updated with adopted-model clarifications and cross-refs.)

---

## 4. Testing Recommendations

- Use a controlled lab environment (DVWA, custom test auth service, or Juice Shop).
- Test matrix:
  - Normal run with mixed test types
  - Scope rejection cases
  - High-risk operations without proper flags (expect clean deny or precise `--allow-high-risk` guidance)
  - JSON + file output (local `AuthTestReport` format)
  - (Integration with `eggsec report` and SARIF output intentionally not applicable — auth-test is standalone/local.)
- Add automated integration tests if possible. (17 wiremock tests + enforcement/policy contract tests added and passing.)

---

## 5. Success Criteria

By the end of this plan (evaluated under the final adopted runtime-policy + local-findings model):
- `auth-test` is a fully functional, policy-gated standalone CLI command for defense-lab credential control validation (local `AuthTestReport`/`AuthFinding` only; direct JSON/text output).
- Safety policy is consistently and visibly enforced via central `EnforcementContext::evaluate()` (CredentialTesting risk, `allow_credential_testing`, scope provenance, high-risk confirmation/override audit). Strict/automated profiles treat as hard Deny.
- No dedicated pipeline profiles (`auth-validation`/`credential-regression`) or `AuthFinding` → canonical conversion were implemented (intentionally; see architecture/auth.md and historical plans for rationale).
- The feature (under the adopted model) is well-documented (`docs/AUTH_LAB.md`, `architecture/auth.md`, CHANGELOG, cross-refs in AGENTS.md / skills / SAFETY / CAPABILITIES) and ready for broader use in authorized lab settings.
- No major regressions in existing functionality. All relevant tests (auth wiremock 17+, enforcement credential_testing, policy contracts, pipeline stages for Auth profile) pass green.

---

**This plan is retained for historical context. It is superseded by the adopted model described at the top of this file. No further implementation or "completion" work is required.** See `architecture/auth.md`, `CHANGELOG.md`, `docs/AUTH_LAB.md`, and `plans/credential-access-implementation-plan.md` (resolution note) for the canonical final state.