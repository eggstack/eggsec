# Handoff Plan: Full Integration of Expanded Auth Tab (Credential Cracking & Password Attacks Loadout)

**Date**: 2026-06-11
**Author**: Grok (xAI) — based on deep inspection via GitHub connector + code expansion
**Context**: Expansion of the newly added `crates/eggsec-tui/src/tabs/auth.rs` skeleton (part of the 2026-06-11 TUI usability pass) into a rich, functional loadout for defense-lab authentication control validation.
**Status**: Core UI/UX + backend stub expanded and committed. Full system integration pending.

---

## 1. Executive Summary & Current State

### What Was Expanded (Already Done)
- `crates/eggsec-tui/src/tabs/auth.rs` significantly upgraded from a minimal 3-field skeleton to a comprehensive loadout:
  - 7 input fields (Target, Username/Userlist, Password/Wordlist, Credential file, Max Attempts, Concurrency, Timeout).
  - Test selection system (`AuthTestSelection` enum supporting All + granular: BruteForce, CredentialStuffing, Lockout, RateLimit, Mfa, Timing, Session, PasswordPolicy).
  - Rich rendering: Safety banners, progress %, severity-colored findings, structured results with recommendations.
  - Execution stub (`run_tests` async method) that uses real `eggsec::auth::*` testers (`RateLimitTester`, `TimingTester`, `BruteForceTester`, etc.) and populates `AuthTestReport` + `AuthFinding`.
  - Helpers for `primary_target()` and `build_cli_equivalent()`.
  - Full navigation, focus management, error handling, and trait implementations extended for new fields.
  - Strong safety messaging aligned with the project's defense-lab framing.
- Added `mod auth;` + `pub use auth::AuthTab;` in `crates/eggsec-tui/src/tabs/mod.rs`.

### What Is Missing (Integration Debt)
The tab is **not yet live** in the TUI:
- Not present in `Tab` enum or tab bar.
- No entry in `TabStore`.
- No `TabSpec` (title, stable_id, risk_group, operation, etc.).
- Dozens of match arms across `tabs/mod.rs` and `app/mod.rs` are not updated.
- No dedicated worker task (async execution, progress, cancellation, result handling).
- Policy enforcement (`EnforcementContext`, `CredentialTesting` risk, confirmation overlays) is only stubbed.
- Session restore, bookmarks, quick-switch, copy-CLI, etc., will not work until registered.

**Goal of this plan**: Provide a clear, phased, low-risk handoff so the next developer (or team) can complete integration quickly while preserving the project's strong safety model, code patterns, and TUI architecture (UiAction layer, overlays, enforcement posture, etc.).

---

## 2. Goals of Full Integration

1. Make the Auth tab appear in the tab bar (logical position: near OAuth / after intrusive assessment tabs).
2. Full keyboard/mouse navigation, focus, insert mode, and state management work identically to peers.
3. Execution goes through the **central policy gate** (`EnforcementContext::evaluate`, `RequireConfirmation` overlay for high-risk `CredentialTesting`, narrow manual override support).
4. Async task management with progress, cancellation, and result delivery (matching `workers/security.rs`, `runner.rs`, etc.).
5. `copy_cli_equivalent()`, session persistence, bookmarks, and quick-switch work out of the box.
6. Maintain "defense-lab only" framing with prominent warnings; never allow production credential use.
7. Keep the tab's local-findings-only design (no automatic bridge to canonical `ScanReportData`).
8. Zero regression on existing tabs or the safety model.

**Non-goals** (for v1):
- Full toggle UI for individual test selection (can remain text-parsed or simple multi-select for now).
- Export of Auth findings to SARIF/JUnit (by design — keep local-only).
- Mobile/wireless-style standalone completeness unless explicitly requested.

---

## 3. Recommended Phased Implementation Plan

**Phase 0 (Done)**: Core expansion of `auth.rs` + basic mod declaration.

**Phase 1: Registration & Metadata (Low risk, high visibility — 1-2 hours)**
- Add `Auth` variant to `Tab` enum (assign next discriminant, e.g. `Auth = 29`).
- Update `from_discriminant`, `from_stable_id`, `from_index`, `visible_index`, `next/prev`, etc.
- Add `Tab::Auth` to the base `vec![]` in `Tab::all()` (and update any conditional pushes if needed).
- Create `TabSpec` entry in `spec.rs` (see example below).
- Add to `visible_tab_specs()` construction.
- Update `TabStore` struct + `new()` initializer.
- Add `pub use auth::AuthTab;` (already done).

**Phase 2: Trait Dispatch Wiring (Mechanical but critical — 2-4 hours)**
- Update **all** match arms in `tabs/mod.rs`:
  - `as_tab_state`
  - `as_tab_state_mut`
  - `as_tab_render`
  - `as_tab_input`
- Add `Auth` arm in `Tab::default_breadcrumb()` if custom needed (or rely on spec).

**Phase 3: App-Level Integration (Core behavior — 3-5 hours)**
- `app/mod.rs`:
  - `current_tab_target()` match arm (use `self.tabs.auth.primary_target()`).
  - `build_current_task()` — decide on `TaskConfig` variant or reuse.
  - `copy_cli_equivalent()` match arm (use the helper already in `AuthTab`).
  - `is_direct_launch_tab()` / post-enter policy gate handling (Auth should be treated as direct-launch high-risk like Stress/Packet/OAuth).
  - `build_current_operation_descriptor()` — will work automatically once spec has `operation: Some("auth-test")`.
- Ensure `handle_enter` path triggers policy confirmation overlay for `CredentialTesting`.

**Phase 4: Async Worker & Task System (Most important for UX — 4-6 hours)**
- Create `crates/eggsec-tui/src/workers/auth.rs` (modeled on `security.rs` or `fuzzer.rs`).
- Define `TaskConfig::Auth { target, config: AuthTestConfig }` (or similar) in `workers/mod.rs` or `runner.rs`.
- Implement `run_auth_task(...)` that:
  - Sends progress updates.
  - Calls `AuthEngine::new(...)` or individual testers based on selection.
  - Handles stop/cancellation via shared flags.
  - Sends `TaskResult::Auth(report)` variant.
- Wire into `task_runtime.rs` or `runner.rs` dispatch.
- Update `AuthTab` to participate in the shared task system (set running state, receive results via channels, update findings/progress).

**Phase 5: Polish, Safety, & Edge Cases (2-3 hours)**
- Prominent policy warnings in render + on-run confirmation text.
- Handle `allow_credential_testing` policy flag gracefully (surface clear error if disabled).
- Test scope enforcement interaction.
- Add `Auth` to any tab-category or risk-group UI indicators (if exists in dashboard/settings).
- Verify session save/restore for auth tab state (inputs + last results).
- Accessibility / small-terminal degraded layout (follow recent TUI patterns).
- Update `AGENTS.md`, `CONTRIBUTING.md`, or `docs/TUI.md` if new patterns introduced.
- Add unit tests in `auth.rs` for the new helpers.

**Phase 6: Documentation & Handoff Artifacts**
- Update `README.md` (TUI section) and `docs/CAPABILITIES.md` to mention the new tab.
- Add example in `examples/` or `plans/`.
- This handoff plan itself (committed).

**Total estimated effort**: 12-20 hours for one experienced contributor familiar with the TUI patterns.

---

## 4. Detailed File-by-File Changes

### `crates/eggsec-tui/src/tabs/mod.rs`
- Add `mod auth;` near top (already partially done).
- Add `Auth` to `Tab` enum (with discriminant).
- Update `Tab::title()`, `cli_command()`, `description()`, `stable_id()` via spec (preferred) or hardcoded.
- Update **every** match in `as_tab_*` functions (copy-paste pattern from nearby intrusive tab like `oauth` or `stress`).
- Add to `Tab::all()` base vec.
- Update `from_discriminant` and `from_stable_id`.

### `crates/eggsec-tui/src/tabs/spec.rs`
Add near the end of `TAB_SPECS` (before Wireless or in logical Assessment/Intrusive group):

```rust
TabSpec {
    tab: Tab::Auth,
    stable_id: "auth",
    title: "Auth Test",
    cli_command: "eggsec auth-test",
    description: "Authentication control validation (brute-force, lockout, MFA, rate-limit, timing, credential stuffing — defense-lab only)",
    category: TabCategory::Assessment,
    risk_group: TabRiskGroup::Intrusive,  // Critical: triggers high-risk policy path
    feature: None,                       // Or Some("credential-testing") if we add a feature flag later
    breadcrumb_label: "Auth / Credential Validation",
    operation: Some("auth-test"),
    direct_launch: true,
},
```

Update `visible_tab_specs()` construction list.

### `crates/eggsec-tui/src/app/tab_store.rs`
```rust
pub struct TabStore {
    ...
    pub auth: tabs::AuthTab,
    ...
}

impl TabStore {
    pub fn new() -> Self {
        Self {
            ...
            auth: tabs::AuthTab::new(),
            ...
        }
    }
}
```

### `crates/eggsec-tui/src/app/mod.rs` (Critical)
- `current_tab_target()`: Add arm `Tab::Auth => self.tabs.auth.primary_target(),`
- `build_current_task()`: Add arm or route to new auth task builder.
- `copy_cli_equivalent()`: Add arm using `self.tabs.auth.build_cli_equivalent()`.
- In `handle_enter` / policy sections: Ensure `CredentialTesting` risk is handled (it will be via the descriptor from spec).
- `is_direct_launch_tab()` will pick it up automatically if `direct_launch: true` in spec.

### New File: `crates/eggsec-tui/src/workers/auth.rs`
Model after `workers/security.rs` or `fuzzer.rs`:

```rust
pub async fn run_auth_task(
    target: String,
    // config: AuthTestConfig { max_attempts, concurrency, timeout, selected_tests, username, wordlist_path, ... }
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    // progress updates
    // instantiate AuthEngine or individual testers
    // run selected tests
    // send TaskResult::Auth(AuthTestReport)
}
```

Add `TaskResult::Auth(eggsec::auth::AuthTestReport)` variant (or a TUI-specific wrapper).

### `crates/eggsec-tui/src/tabs/auth.rs` (Further Polish)
- Replace the current execution stub with proper channel-based communication to the new worker.
- Implement `build_task_config(&self) -> Option<workers::TaskConfig>`.
- Add `fn stop(&mut self)` that also signals any running worker.
- Enhance test selection UI (simple text field for now is acceptable; future: checkbox list component).

---

## 5. Key Technical Patterns to Follow

- **Direct-launch intrusive tabs** (Stress, Packet, OAuth, WafStress): Use `direct_launch: true` in spec + post-`handle_enter` policy check in `app/mod.rs`.
- **High-risk policy path**: `OperationRisk::Intrusive` (or dedicated `CredentialTesting` if exposed) + `RequireConfirmation` overlay. The `build_current_operation_descriptor()` already does most of the work once the spec is present.
- **Task system**: All long-running work should go through `TaskState` / `task_runtime` for unified progress bar, pause, cancel, and background notification.
- **Safety first**: Never bypass `allow_credential_testing`. Surface clear messages. Use the existing `AUTH_BANNER` and lab-account warnings.
- **Findings handling**: Keep local-only (`AuthTestReport` / `AuthFinding`). Do **not** implement `to_scan_report_data` bridge unless explicitly requested (per current design intent in `docs/AUTH_LAB.md`).
- **Recent TUI architecture (2026-06-11 pass)**: Prefer `UiAction` where possible, use semantic `tc!()` colors, respect `EnforcementContext` in the app, and support degraded small-terminal layouts.

---

## 6. Safety, Policy & Edge-Case Considerations

- **High-risk nature**: `CredentialTesting` must always hit the confirmation overlay on ManualPermissive and hard-deny on strict/MCP/agent/CI paths.
- **Scope interaction**: The tab should respect `LoadedScope` (explicit manifest required for automated paths).
- **Wordlist / credential file safety**: Keep the path-traversal and canonicalization checks from the CLI handler.
- **Account lockout risk**: Prominent warning + conservative default max-attempts.
- **No production data**: Enforce in UI help text and perhaps a runtime check if a non-lab-looking target is entered.
- **Cancellation**: Ensure running tests can be stopped cleanly without leaving accounts in bad state.
- **Error surfacing**: Use `TabError` + the new overlay system.

**Edge cases to test**:
- Policy disabled (`allow_credential_testing = false`).
- No explicit scope.
- Very small terminal width.
- Rapid tab switching while a test is running.
- Session restore with previous auth inputs/results.
- Copy-CLI button produces a safe, minimal command.

---

## 7. Testing & Validation Checklist

- [ ] Tab appears in bar and is keyboard-navigable.
- [ ] All input fields accept typing, focus, paste, word movement, home/end.
- [ ] Running a test shows live progress and populates findings with correct severities.
- [ ] Policy confirmation overlay appears for high-risk runs (and narrow override works).
- [ ] `eggsec auth-test` CLI equivalent is accurate and safe.
- [ ] No compile errors or broken matches in any tab dispatch.
- [ ] Existing tabs (especially OAuth, Stress, Fuzz) still work perfectly.
- [ ] Session save/restore preserves auth tab state.
- [ ] Help text / breadcrumb / description are accurate and safety-focused.
- [ ] Unit tests pass (`cargo test -p eggsec-tui`).
- [ ] Manual end-to-end against a lab target (e.g., DVWA or custom auth test app) with dedicated accounts.

---

## 8. Open Decisions & Questions for the Team

1. **Exact tab title & position**: "Auth Test", "Auth", "Credential Validation", or "Cred Cracking"? Recommended position: after OAuth or in the Assessment group.
2. **Feature flag?** Keep as always-on base tab (with runtime policy) or gate behind a new `credential-testing` feature (like `wireless` or `mobile`)?
3. **Test selection UI**: Keep simple text field / "All" default for v1, or invest in a proper multi-select / checkbox list component now?
4. **Result persistence**: Should last `AuthTestReport` be saved to history/session like normal scans, or remain tab-local only?
5. **Worker granularity**: One `run_auth_task` that runs everything, or separate tasks per test type for finer progress?
6. **Naming consistency**: Use "Auth Control Validation" everywhere in UI strings to match docs, or shorter "Auth Test" for the tab title?

---

## 9. Risks & Mitigations

- **Risk**: Large number of match arms → easy to miss one and cause runtime panic or missing functionality.
  **Mitigation**: Use a script or careful diff; add a compile-time test that all tabs are covered (many projects do this).
- **Risk**: Policy integration bugs could weaken safety model.
  **Mitigation**: Reuse existing `build_current_operation_descriptor` + `EnforcementContext` paths exactly; do not invent new bypass logic.
- **Risk**: Async worker complexity.
  **Mitigation**: Start with a simple synchronous execution in the stub, then migrate to channels. Many other tabs already have working patterns.
- **Risk**: Scope creep into offensive use.
  **Mitigation**: Keep all language, banners, and docs explicitly "defense-lab / control validation only". Do not add production credential support.

---

## 10. Suggested Commit / PR Strategy

- Create feature branch `feat/tui-auth-full-integration`.
- Phase 1 + 2 in one PR (registration + wiring) — easy to review.
- Phase 3 + 4 in second PR (app + worker) — core functionality.
- Phase 5 polish + tests in final PR.
- Link this plan in the PR description.
- Tag reviewers familiar with TUI architecture (UiAction, enforcement, task system).

---

## 11. Handoff Artifacts & References

- This plan: `plans/auth-tui-full-integration-handoff-plan.md`
- Expanded tab: `crates/eggsec-tui/src/tabs/auth.rs` (current main)
- Key references:
  - `docs/AUTH_LAB.md` and `architecture/auth.md` (safety model & design intent)
  - `crates/eggsec/src/commands/handlers/auth_test.rs` (CLI reference implementation)
  - `crates/eggsec-tui/src/tabs/oauth.rs` or `stress.rs` (good patterns for direct-launch intrusive tabs)
  - `crates/eggsec-tui/src/workers/security.rs` (worker pattern)
  - `app/mod.rs` sections on `EnforcementContext` and policy confirmation overlays (2026-06-11 pass)

**Ready for handoff**. The expanded loadout is in a state where integration is straightforward mechanical work plus one focused worker implementation. The safety model and existing architecture make it low-risk if patterns are followed.

---

*End of plan. Questions or clarifications welcome before starting Phase 1.*
