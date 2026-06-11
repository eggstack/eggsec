# Mobile Feature - Phase 1 Completion Plan

**Status**: Next Steps / Completion Plan  
**Date**: 2026-06-11  
**Goal**: Bring mobile static analysis to a clean, usable standalone state.

---

## 1. Executive Summary

A strong technical foundation for mobile static analysis already exists (`crates/eggsec/src/mobile/` with substantial `apk.rs` and `ipa.rs` implementations). The module follows the original plan's guidance (pure static analysis, lab-focused, safety-conscious).

The remaining work is primarily around **documentation, CLI exposure, policy integration, polish, and testing** to make the feature feel complete and adoptable as a standalone tool.

---

## 2. Current State Snapshot

**Completed**:
- Module structure with `mod.rs`, `apk.rs`, and `ipa.rs`.
- Core static analysis for Android APKs and iOS IPAs.
- `MobileFinding` / `MobileScanReport` data models.
- `run_cli()` entry point with JSON/human output and file writing.
- `to_scan_report_data()` integration for unified reporting.
- Basic safety framing and input validation in code.

**Remaining Gaps**:
- No dedicated `docs/MOBILE.md`.
- Limited presence in main project documentation (`README.md`, `CAPABILITIES.md`).
- CLI command not yet prominently wired/exposed.
- Policy / `EnforcementContext` integration still needed.
- Polish on findings quality, recommendations, and error handling.
- Testing coverage could be expanded.

---

## 3. Prioritized Tasks

### Task 1: Documentation (Highest Priority)

**Goal**: Make the feature discoverable and usable.

**Actions**:
- Create `docs/MOBILE.md` covering:
  - Purpose and scope (static analysis only, lab/defense use)
  - Supported platforms and file types (.apk / .ipa)
  - CLI usage and examples
  - What kinds of findings are detected
  - Safety warnings and best practices
  - Limitations (static only, no dynamic analysis)
- Update `README.md`:
  - Add mobile to the command reference and lab/defense sections.
- Update `CAPABILITIES.md`:
  - Add mobile to the Lab Defense Commands table and Build Features.
- Update `SAFETY.md`:
  - Add mobile under appropriate risk tier (likely `SafeActive`).

### Task 2: CLI Exposure & Integration

**Goal**: Make `eggsec mobile` a first-class command.

**Actions**:
- Ensure `MobileArgs` is properly defined in `crates/eggsec/src/cli/` and wired into the main CLI.
- Add clear help text and examples.
- Integrate with `CommandContext` / handler pattern (similar to wireless and auth-test).
- Add policy enforcement call using `EnforcementContext` (risk tier: `SafeActive` or new `MobileStatic`).

### Task 3: Policy & Safety Wiring

**Goal**: Ensure consistent safety model.

**Actions**:
- Wire the mobile command through `evaluate_and_enforce_operation()`.
- Define appropriate risk tier and required features.
- Add clear warnings in both code and documentation.
- Consider size / provenance checks for app binaries.

### Task 4: Polish & Findings Quality

**Actions**:
- Review and refine finding categories and severity assignments in `apk.rs` and `ipa.rs`.
- Improve recommendation quality and actionability.
- Enhance error messages and edge-case handling (corrupt APKs/IPAs, unusual manifest structures).
- Add more structured evidence to findings where useful.
- Improve `format_mobile_report()` output readability.

### Task 5: Testing & Robustness

**Actions**:
- Expand unit tests in `mod.rs`, `apk.rs`, and `ipa.rs`.
- Add integration-style tests with sample (non-sensitive) test APKs/IPAs.
- Verify `to_scan_report_data()` produces valid output for SARIF/JUnit/etc.
- Test large file rejection and invalid input handling.

---

## 4. Recommended Implementation Order

1. **Task 1 (Documentation)** — Highest impact for adoption.
2. **Task 2 (CLI Exposure)** — Makes the feature usable from the command line.
3. **Task 3 (Policy Integration)** — Ensures safety model consistency.
4. **Task 4 + 5 (Polish & Testing)** — Improves quality and confidence.

---

## 5. Success Criteria for Phase 1

- `docs/MOBILE.md` exists and is high quality.
- `eggsec mobile <file>` works cleanly for both `.apk` and `.ipa` files.
- The command is documented in `README.md` and `CAPABILITIES.md`.
- Policy enforcement is wired in.
- Output is structured, useful, and integrates with the reporting system.
- The feature feels like a complete, professional standalone static mobile analysis tool.

---

## 6. Future Phases (Out of Scope for Now)

- Dynamic analysis / instrumentation (Frida-based, behind additional gates)
- Deeper dependency / supply chain analysis
- `mobile-static` and `mobile-regression` pipeline profiles
- Active testing capabilities (explicitly deferred)

---

**This plan focuses on finishing Phase 1 so mobile can be considered complete as a standalone static analysis capability.**