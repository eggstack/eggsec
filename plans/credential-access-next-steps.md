# Credential Access - Next Steps Plan (Phase 2)

**Status**: Active Handoff  
**Date**: 2026-06-11  
**Context**: Significant progress has been made since the initial plan. A real execution handler now exists, and documentation has been added. This plan focuses on completing integration, safety hardening, and polish.

---

## 1. Current State Snapshot

**Completed**:
- `cli/auth.rs` — Rich `AuthTestArgs` with comprehensive flags.
- `commands/handlers/auth_test.rs` — New substantial handler implementing execution logic (calls into `auth/` module testers).
- `docs/AUTH_LAB.md` — Good defense-lab framing, safety guidance, and config examples created.
- Runtime policy control implemented (`allow_credential_testing` + `OperationRisk::CredentialTesting` in `EnforcementContext`) instead of a Cargo feature flag.
- `ScanProfile::Auth` variant exists and is partially wired.

**Remaining Gaps**:
- Findings / output conversion (auth results → standard `FindingData`, SARIF, JUnit).
- Verification and strengthening of safety enforcement inside the handler.
- Full pipeline integration for `auth-validation` and `credential-regression` profiles.
- Polish: wordlist robustness, TUI support, better error handling, and end-to-end testing.
- Documentation updates in `CAPABILITIES.md`, `SAFETY.md`, and `README.md`.

**Key Design Decision**: Control is via runtime policy/config rather than a compile-time feature flag. This is acceptable and safety-oriented.

---

## 2. Prioritized Next Steps

### High Priority (Do These First)

1. **Findings & Output Integration**
2. **Safety Enforcement Verification & Hardening** (inside the handler)
3. **Basic End-to-End Testing** with a lab target

### Medium Priority

4. Profile pipeline support (`auth-validation` / `credential-regression`)
5. Wordlist handling robustness
6. Documentation updates

### Lower Priority / Polish
7. TUI views
8. Agent/MCP tool exposure
9. Multi-protocol tester improvements

---

## 3. Detailed Task Breakdown

### Task 1: Findings & Output Conversion (Highest Priority)

**Goal**: Make `auth-test` results appear in normal reports, SARIF, JUnit, and the findings system.

**Files to modify**:
- `crates/eggsec/src/commands/handlers/auth_test.rs`
- `crates/eggsec/src/auth/` (add or extend conversion logic)
- Possibly `crates/eggsec/src/output/convert.rs`

**Actions**:
- Create or extend conversion functions:
  ```rust
  pub fn auth_finding_to_finding_data(finding: &AuthFinding) -> FindingData { ... }
  pub fn auth_test_report_to_scan_report_data(report: &AuthTestReport) -> ScanReportData { ... }
  ```
- Map `severity`, `category` (use `"authentication"` or `"credential-access"`), remediation, and any available CVSS/CWE data.
- Call the conversion at the end of the handler and feed results into the normal output path.
- Ensure JSON, pretty, and structured outputs all work.

**Success Criteria**: Running `eggsec auth-test ... --json` produces findings that can also be consumed by the report system.

### Task 2: Safety Enforcement Verification & Hardening

**Goal**: Ensure the handler properly respects scope, policy, and risk tiers.

**Files**:
- `crates/eggsec/src/commands/handlers/auth_test.rs`
- `crates/eggsec/src/config/policy.rs` (verify `OperationRisk::CredentialTesting`)
- `crates/eggsec/src/safety/enforcement.rs` or equivalent

**Actions**:
- In the handler, before running tests:
  - Call into `EnforcementContext::evaluate()` (or equivalent) with an `AuthOperation` or `CredentialTesting` risk.
  - Validate that the target + any test usernames are covered by the loaded scope.
  - Respect `allow_credential_testing` from config.
- For aggressive tests (`brute_force`, `credential_stuffing`):
  - Require `--allow-high-risk` + reason (or treat as `RequireConfirmation`).
  - Enforce hard attempt/concurrency budgets.
- Add clear logging of policy decisions.
- Test that strict profiles (CI, agent, `--strict-scope`) block high-risk auth tests.

**Quick Check**: Review the new handler to see how much of this is already implemented.

### Task 3: End-to-End Testing & Validation

**Recommended Lab Targets**:
- DVWA login form
- Juice Shop or a custom minimal auth service with known lockout/MFA behavior

**Test Cases**:
- Basic run with `--all --max-attempts 20 --yes`
- Scope rejection test (target not in scope)
- High-risk test without `--allow-high-risk` (should warn or block)
- JSON + file output verification
- Lockout detection and rate limit testing behavior

**Actions**:
- Add or update integration tests under `tests/` or in CI.
- Document a reproducible lab setup in `docs/AUTH_LAB.md` or `docs/lab/`

### Task 4: Profile Pipeline Integration (Medium)

**Goal**: Make `eggsec scan --profile auth-validation` and `credential-regression` actually work.

**Files**:
- Pipeline definition files
- `crates/eggsec/src/commands/handlers/scan.rs`
- Profile-related code in `cli/scan.rs` or config

**Actions**:
- Define the stages for the two new profiles (e.g., endpoint discovery → safe auth tests → WAF analysis).
- Wire them into the main scan pipeline.
- Ensure `ScanProfile::Auth` has full support.

### Task 5: Polish Items

- Robust wordlist loading (size limits, error handling, secure zeroization if sensitive).
- Better error messages and progress reporting in the handler.
- Consider adding a `--dry-run` / planning mode for auth tests.
- TUI progress view (lower priority).

### Task 6: Documentation Updates

**Files**:
- `README.md` (add `auth-test` to command reference and lab section)
- `docs/CAPABILITIES.md` (add auth testing capabilities table)
- `docs/SAFETY.md` (expand `CredentialTesting` risk tier section)
- `CHANGELOG.md`

---

## 4. Suggested Implementation Order

1. **Task 1** (Findings conversion) — Makes the feature immediately more useful.
2. **Task 2** (Safety verification) — Critical for trust and correctness.
3. **Task 3** (End-to-end testing) — Validate everything works as intended.
4. **Task 4** (Profile integration) — For full pipeline users.
5. Polish + documentation.

---

## 5. Risks & Mitigations

- **Risk**: Safety logic in the new handler is incomplete → **Mitigation**: Explicitly review and test `EnforcementContext` calls early in Task 2.
- **Risk**: Findings conversion is complex → **Mitigation**: Start simple (map core fields) and iterate. Reuse existing conversion patterns from other modules.
- **Risk**: Breaking existing behavior → **Mitigation**: Keep auth-test as a standalone command (it already is) and add profiles gradually.

---

## 6. Success Criteria for This Phase

- `eggsec auth-test` produces usable findings in JSON and structured report formats.
- Safety policy is consistently enforced (scope + risk tier).
- Basic end-to-end lab testing passes with documented setup.
- `docs/AUTH_LAB.md` is accurate and complete.
- The feature feels integrated with the rest of Eggsec rather than bolted on.

---

**This plan is designed as a direct handoff for the next developer or model to continue the work efficiently.**

Start with **Task 1 or Task 2** depending on whether you want quick visible value or stronger safety guarantees first.