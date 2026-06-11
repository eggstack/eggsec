# Mobile Feature - Micro Close-Out Checklist

**Status**: Final Polish / Close-Out  
**Date**: 2026-06-11  
**Purpose**: Lightweight checklist to finish the remaining items and mark mobile static analysis as complete in standalone form.

---

## Current State

- Strong static analysis foundation exists for both Android and iOS.
- `docs/MOBILE.md` has been created and is comprehensive.
- Core `run_cli()`, reporting, and output conversion are in place.
- Remaining work is mostly documentation exposure, CLI/policy integration, and final polish.

---

## Micro Close-Out Checklist

### 1. Main Project Documentation Updates
- [ ] Add mobile to `README.md`:
  - Include in Quick Command Reference with 1–2 examples.
  - Add a short entry in the Lab Defense Commands table.
- [ ] Update `CAPABILITIES.md`:
  - Add `eggsec mobile <file>` to the Lab Defense Commands section.
  - Mention the `mobile` feature in the Build Features table.
- [ ] (Optional) Add a brief mobile note in `SAFETY.md` under the appropriate risk tier if not already present.

### 2. CLI Exposure & Handler Integration
- [ ] Verify `MobileArgs` is fully defined in `crates/eggsec/src/cli/`.
- [ ] Ensure the `mobile` command is wired into the main CLI dispatch.
- [ ] Create or complete `crates/eggsec/src/commands/handlers/mobile.rs` (following the wireless/auth-test pattern).
- [ ] Add clear, helpful `--help` text.

### 3. Policy & Safety Integration
- [ ] Wire the handler through `ctx.evaluate_and_enforce_operation()` with appropriate `OperationRisk` (likely `SafeActive`) and `required_features: ["mobile"]`.
- [ ] Confirm strict profiles (MCP, agent, CI) correctly gate on the feature.
- [ ] Ensure prominent lab-use warnings appear in both code and `MOBILE.md`.

### 4. Polish & Quality
- [ ] Review finding categories, severities, and recommendation quality in `apk.rs` and `ipa.rs`.
- [ ] Improve human-readable output formatting in `format_mobile_report()` if needed.
- [ ] Add or expand unit tests for edge cases (corrupt files, large files, unusual manifests).
- [ ] Verify `to_scan_report_data()` produces clean, usable output for SARIF/JUnit/etc.

### 5. Final Sanity Checks
- [ ] Run `cargo test --features mobile` and confirm all tests pass.
- [ ] Build with `--features mobile` and test basic CLI usage (`eggsec mobile --help` and a dry test on a sample APK/IPA).
- [ ] Review `docs/MOBILE.md` one more time for accuracy against current code behavior.

---

## Recommended Approach

This checklist is intentionally small and focused:

1. Start with **documentation updates** (highest visibility).
2. Complete **CLI + handler wiring** (makes the feature usable).
3. Finish **policy integration** (ensures safety model consistency).
4. Do **polish + testing** in parallel or last.
5. End with **sanity checks**.

Most items can be completed in one or two focused sessions.

---

## Success Criteria

- Mobile is documented in `README.md` and `CAPABILITIES.md`.
- `eggsec mobile <file>` works cleanly from the CLI with proper policy enforcement.
- Output is high quality and integrates with the reporting system.
- The feature feels complete, safe, and professional as a standalone static mobile analysis tool.

Once this checklist is done, Phase 1 of mobile can be considered closed out.

---

**This is a lightweight micro close-out checklist. Keep it pragmatic and focused on the last 10-15% of work.**