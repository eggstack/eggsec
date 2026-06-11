# Credential Access Feature - Completion Plan

**Status**: Final Integration & Polish Phase  
**Date**: 2026-06-11  
**Goal**: Bring the `auth-test` capability to a production-ready, well-integrated state.

---

## 1. Executive Summary & Current State

Significant progress has been made:
- A fully functional `handle_auth_test()` exists in `crates/eggsec/src/commands/handlers/auth_test.rs`.
- Safety integration via `evaluate_and_enforce_operation()` with `OperationRisk::CredentialTesting` is in place.
- Findings are being collected internally.
- `docs/AUTH_LAB.md` provides solid guidance.

**Remaining Work** (in priority order):
1. **Findings & Output Conversion** – Make auth results flow into the standard reporting system (SARIF, JUnit, HTML, etc.).
2. **Safety Hardening & Verification** – Ensure scope, policy, and risk enforcement are robust.
3. **Profile Pipeline Integration** – Fully enable `auth-validation` and `credential-regression` profiles.
4. **Polish & Testing** – Robustness, error handling, and end-to-end validation.
5. **Documentation** – Update main docs to reflect the completed feature.

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

1. **Task 1** (Findings conversion) – Delivers the most immediate value.
2. **Task 2** (Safety hardening) – Ensures the feature is trustworthy.
3. **Task 3** (Profile integration) – Completes the pipeline story.
4. **Task 4 + 5** (Polish + Docs) – Final cleanup.

---

## 4. Testing Recommendations

- Use a controlled lab environment (DVWA, custom test auth service, or Juice Shop).
- Test matrix:
  - Normal run with mixed test types
  - Scope rejection cases
  - High-risk operations without proper flags
  - JSON + file output
  - Integration with `eggsec report` and SARIF output
- Add automated integration tests if possible.

---

## 5. Success Criteria

By the end of this plan:
- `auth-test` results are fully integrated into Eggsec’s reporting and findings system.
- Safety policy is consistently and visibly enforced.
- The two dedicated auth profiles work via the main `scan` command.
- The feature is well-documented and ready for broader use.
- No major regressions in existing functionality.

---

**This plan is designed to be executed incrementally. Start with Task 1.**