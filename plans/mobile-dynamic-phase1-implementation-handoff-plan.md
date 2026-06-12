# Mobile Dynamic Phase 1 Implementation Handoff Plan

**Date**: 2026-06-12  
**Status**: Draft — Ready for Team Review & Handoff  
**Parent**: `plans/dynamic-mobile-testing-loadout-design-plan.md` (Phase 0 complete)  
**Target Phase**: Phase 1 — Android ADB Core + Runtime Log Analysis  
**Related**: `docs/MOBILE.md`, `architecture/mobile.md`, `crates/eggsec/src/mobile/`, wireless active implementation plans (pattern), EnforcementContext, CLI handler patterns  
**Authoring Note**: Created as the immediate next handoff artifact after Phase 0 doc integration. Focuses exclusively on implementing the highest-value, lowest-risk portion of the dynamic loadout (Android-focused ADB + logcat analysis). Intended for the implementation team to execute without ambiguity.

---

## 1. Executive Summary

This plan provides a detailed, actionable handoff for **Phase 1 implementation** of the Dynamic Mobile Application Testing (DMAT) loadout.

**Goal for Phase 1**: Deliver a working, policy-gated `mobile-dynamic` surface that allows controlled Android device/emulator interaction for:
- Device discovery & lab manifest validation
- Safe APK install / launch / uninstall (with automatic cleanup)
- Runtime logcat capture and high-signal security finding generation
- Full dry-run support + structured JSON output (bridge-ready)

**Scope**: Android-only. Pure-Rust preferred for ADB layer (or gated minimal subprocess fallback). No Frida, no proxy/MITM automation, no permission grant/revoke testing, no TUI in this phase. These are explicitly deferred to Phase 2+ per the parent design plan.

**Key Deliverables**:
- Feature flag `mobile-dynamic` in `Cargo.toml`
- New files: `crates/eggsec/src/mobile/dynamic.rs`, `adb.rs`, and minimal `runtime.rs`
- Extended CLI (`eggsec mobile dynamic ...` or dedicated subcommand)
- Handler + `EnforcementContext` integration with lab-manifest stub
- `DynamicMobileReport` / `DynamicMobileFinding` types + basic `to_scan_report_data` bridge
- Dry-run path that produces valid output without touching devices
- Unit tests + emulator smoke test
- Documentation updates (expanded dynamic section in `docs/MOBILE.md` + examples)

**Success Criteria (Phase 1)**:
- `cargo build --features mobile-dynamic` succeeds cleanly
- `eggsec mobile dynamic --help` shows new subcommands/flags
- Dry-run on any APK produces complete, schema-valid JSON report
- Real emulator run (install → launch → log capture → uninstall) works with policy confirmation and produces findings
- All dynamic actions are audited in the report (`actions_performed`)
- No regressions in existing static `mobile` functionality

**Timeline Target**: 3–4 weeks for core implementation + testing (aggressive but achievable given wireless precedent and existing patterns).

---

## 2. Background & Current State (Post Phase 0)

- Phase 0 (design + doc integration) is complete. The parent plan `plans/dynamic-mobile-testing-loadout-design-plan.md` is now the authoritative reference and is cross-linked from `docs/MOBILE.md`, architecture docs, and AGENTS files.
- Static mobile (`eggsec mobile <apk/ipa>`) remains fully functional and unchanged.
- No code for dynamic capabilities exists yet (no feature flag, no new modules, no ADB layer).
- The team has a clean slate and full design guidance.

**Parent Plan Key Decisions Already Made**:
- Standalone defense-lab surface (no MCP/agent tool registration in Phase 1)
- Heavily gated (`mobile-dynamic` feature + lab manifest + `--allow-dynamic-mobile` + provenance prompt)
- Android-first (iOS dynamic deferred)
- Pragmatic ADB approach (pure-Rust preferred)
- Phase 1 focuses on device control + log analysis only

---

## 3. Phase 1 Scope & Deliverables

### 3.1 In Scope for Phase 1
- Feature flag plumbing
- Basic device listing and validation against a simple lab manifest (TOML stub)
- Controlled APK lifecycle: `install` → `launch` (optional activity) → optional log capture → `uninstall`
- Logcat capture with bounded duration + basic high-signal parser (permission events, crashes with interesting frames, cleartext/network hints, obvious secret patterns in logs)
- Full dry-run mode (no device actions, complete report structure)
- Structured output (`DynamicMobileReport` JSON) + basic human formatting
- Optional `to_scan_report_data` bridge stub (so `report convert` path is ready even if not fully populated)
- Policy gate using existing `EnforcementContext` patterns (SafeActive + new capability)
- Prominent lab-use warnings and provenance confirmation prompt
- Unit tests + one end-to-end emulator smoke test

### 3.2 Explicitly Out of Scope for Phase 1
- Proxy / MITM setup automation
- Runtime permission grant/revoke testing
- Frida or any hooking
- Traffic correlation or observed endpoint findings
- TUI tab or actions
- iOS dynamic support
- Full `mobile-frida` sub-feature
- Pipeline `ScanProfile` integration
- Advanced redaction engine (basic secret patterns only)
- Production device safety beyond warnings + manifest

### 3.3 Deliverables Table

| # | Deliverable | Description | Priority | Dependencies | Owner Suggestion | Done When |
|---|-------------|-------------|----------|--------------|------------------|-----------|
| 1 | Feature flag | Add `mobile-dynamic = ["mobile"]` (and update `full`) in `crates/eggsec/Cargo.toml` | P0 | None | Core | Builds with flag |
| 2 | Types | `DynamicMobileReport`, `DynamicMobileFinding`, `DeviceInfo`, `LabManifest` (simple struct) in `mobile/dynamic.rs` or `types.rs` | P0 | None | Core | Serializable, documented |
| 3 | ADB layer (core) | `adb.rs` with `list_devices()`, `connect()`, `install()`, `launch()`, `logcat_capture()`, `uninstall()` — pure Rust or minimal gated wrapper | P0 | tokio/async if needed | Impl | Works on emulator TCP (5555) |
| 4 | Runtime log parser | Basic parser in `runtime.rs` that extracts high-signal events from logcat output | P1 | ADB logcat | Impl | Finds permission/crash/network hints on test APK |
| 5 | CLI surface | Extend `cli/mobile.rs` or new `cli/mobile_dynamic.rs` with `DynamicMobileArgs` + subcommand registration | P0 | Types + handler | CLI | `--help` shows dynamic commands |
| 6 | Handler + Policy | `commands/handlers/mobile_dynamic.rs` (or extend existing) with `EnforcementContext` call, lab-manifest check, provenance prompt, dry-run path | P0 | CLI + types | Policy/Impl | Policy denies without flag/manifest; dry-run works |
| 7 | Dispatcher | Update `mobile/mod.rs` to expose dynamic entry point and re-export types | P0 | Handler | Core | `run_dynamic_cli` callable |
| 8 | Report formatting | Human pretty-printer + JSON for `DynamicMobileReport` (mirrors static) | P1 | Types | Output | Matches style of static mobile output |
| 9 | Bridge stub | Basic `to_scan_report_data_dynamic()` that produces valid (even if minimal) `ScanReportData` | P2 | Types | Output | `report convert` accepts dynamic JSON without crash |
| 10 | Tests | Unit tests for ADB messages (mock), log parser on fixtures, policy stubs, serde roundtrips, dry-run | P0 | All core | Test | `cargo test --features mobile-dynamic` green |
| 11 | Smoke test | One documented emulator test (Android Studio AVD) that exercises full happy path | P1 | Emulator available | QA/Impl | Passes with known test APK |
| 12 | Docs update | Expand `docs/MOBILE.md` “Dynamic Testing” section with Phase 1 examples, warnings, lab setup | P1 | Working CLI | Docs | Users can follow examples |

---

## 4. Technical Design Details for Phase 1

### 4.1 Recommended Module Layout
```
crates/eggsec/src/mobile/
├── mod.rs                 # Add pub mod dynamic; pub use dynamic::{run_dynamic_cli, DynamicMobileReport, ...}
├── apk.rs                 # unchanged
├── ipa.rs                 # unchanged
├── dynamic.rs             # NEW: Public API, report types, run_dynamic_cli, formatting, bridge stub
├── adb.rs                 # NEW: ADB client primitives (list, connect, install, shell, logcat, uninstall)
├── runtime.rs             # NEW: Log parser, finding generators (permission, crash, network hints)
└── AGENTS.override.md     # Update with new files and Phase 1 notes
```

### 4.2 Key Types (Proposed Skeleton)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabManifest {
    pub allowed_device_serials: Vec<String>,
    pub allowed_packages: Vec<String>,
    // future: max_actions, allowed_permissions, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicMobileFinding {
    pub category: String,           // "runtime-permission", "crash-log", "cleartext-observed", etc.
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub recommendation: String,
    pub evidence: Option<String>,
    pub static_correlation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicMobileReport {
    pub target: String,
    pub scan_type: String,          // "mobile-dynamic"
    pub platform: MobilePlatform,   // Android only in Phase 1
    pub device_serial: Option<String>,
    pub app_id: Option<String>,
    pub version: Option<String>,
    pub timestamp: String,
    pub findings: Vec<DynamicMobileFinding>,
    pub recommendations: Vec<String>,
    pub duration_ms: u64,
    pub actions_performed: Vec<String>,   // audit trail
    pub dry_run: bool,
}
```

### 4.3 ADB Layer Decision Point (Team Must Choose Early)
**Option A (Recommended for Phase 1)**: Minimal pure-Rust ADB-over-TCP client (focus on emulator 5555 first). Implement only the messages needed (CNXN, AUTH, OPEN, WRITE, OKAY, etc.). Lightweight, no new heavy deps.

**Option B**: Add an optional crate (e.g. a mature `adb` or `rust-adb` crate) under the feature flag.

**Option C**: Gated subprocess to system `adb` binary (fastest to implement, but less pure).

**Recommendation**: Start with **Option A** for emulator TCP. Add Option C as fallback behind an internal `external-adb` sub-feature if real USB devices are needed quickly. Document the decision in the file header.

### 4.4 CLI & Handler Patterns
Follow the exact patterns used by:
- Static `mobile` (simple args + run_cli)
- `wireless deauth` subcommand style
- `auth-test` high-risk standalone command

Example desired UX:
```bash
# Dry run (safe, no device touch)
eggsec mobile dynamic app.apk --device emulator-5554 --dry-run --json

# Real controlled run
eggsec mobile dynamic /path/to/test.apk \
  --device emulator-5554 \
  --install --launch .MainActivity \
  --capture-logs --duration 90 \
  --uninstall-after \
  --allow-dynamic-mobile \
  --lab-manifest examples/lab-mobile.toml
```

### 4.5 Policy & Safety in Phase 1
- Require `mobile-dynamic` feature at compile time.
- Runtime: `EnforcementContext` check with `OperationRisk::SafeActive` (or new `MobileDynamic` tier).
- Lab manifest validation (even if simple TOML allowlist for devices + packages).
- Explicit provenance confirmation prompt (unless `--yes` or in strict mode).
- All actions recorded in `actions_performed`.
- Automatic best-effort uninstall on error/Ctrl-C.

---

## 5. Recommended Implementation Order

1. **Day 1–2**: Feature flag + types + `dynamic.rs` skeleton (report structs, dry-run path, formatting).
2. **Day 3–5**: `adb.rs` core (TCP emulator path first). Get `list_devices` + `install` + `uninstall` working.
3. **Day 6–8**: CLI args + basic handler dispatch + dry-run end-to-end.
4. **Day 9–11**: Logcat capture + basic `runtime.rs` parser + finding generation.
5. **Day 12–14**: Policy integration, lab-manifest stub, provenance prompt, error handling & cleanup.
6. **Day 15–17**: Tests (unit + integration with mocks) + bridge stub.
7. **Day 18–20**: Emulator smoke test + docs examples.
8. **Day 21+**: Polish, clippy, full test suite under feature, PR review.

Parallel track: One person can own docs + examples while another owns the ADB layer.

---

## 6. Dependencies & Open Decisions (Resolve Before Coding)

| Decision | Options | Recommendation | Impact if Delayed |
|----------|---------|----------------|-------------------|
| ADB implementation strategy | Pure-Rust TCP vs crate vs subprocess | Pure-Rust TCP for emulator first | High — blocks all device work |
| Lab manifest format & enforcement strictness | Simple TOML allowlist vs full policy object | Start simple (Vec<String> for devices/packages) | Medium — affects handler |
| CLI command structure | `eggsec mobile dynamic ...` vs new top-level `mobile-dynamic` binary | `eggsec mobile dynamic` (consistent with static) | Low | 
| How much log parsing in Phase 1 | Permission + crash only vs broader network hints | Start narrow (permission events + crashes with stack) | Medium | 
| Bridge completeness | Minimal stub vs full category mapping | Minimal valid stub is enough for Phase 1 | Low | 

Resolve these in the first standup after this plan is approved.

---

## 7. Testing Strategy

- **Unit tests**: ADB protocol message construction/parsing (use `Cursor` + mock TCP), log parser on synthetic logcat fixtures, report serde roundtrips, dry-run logic.
- **Integration tests**: Policy enforcement with mocked `EnforcementContext`, manifest validation.
- **Emulator smoke test** (documented in `docs/MOBILE.md` or a `tests/` script):
  1. Start clean Android emulator (API 34 recommended).
  2. Build a small test APK with known issues (debuggable + cleartext + exported component).
  3. Run full dynamic flow with `--allow-dynamic-mobile`.
  4. Verify install/launch/log findings/uninstall + report contents.
- Run full suite with `cargo test --features mobile-dynamic` and `cargo clippy --features mobile-dynamic`.
- No hardware USB device testing required in Phase 1 (emulator is sufficient).

---

## 8. Documentation & Communication

- Update `docs/MOBILE.md`:
  - Expand “Dynamic Testing Phases” section with Phase 1 examples and warnings.
  - Add “Phase 1 Lab Setup” subsection (emulator + USB debugging + lab manifest example).
  - Update Future section to mark Phase 1 complete once done.
- Update `architecture/mobile.md` with new files and Phase 1 status.
- Update `AGENTS.override.md` in mobile/ with new module responsibilities.
- Add quick example to README.md lab defense commands table (once stable).
- Consider a short note in the parent design plan’s resolution section.

---

## 9. Risks & Mitigations Specific to Phase 1 Implementation

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Choosing wrong ADB approach early | Medium | High (delays everything) | Decide Option A (pure TCP) on day 1; have subprocess fallback ready as Plan B |
| ADB implementation complexity underestimated | Medium | Medium | Scope strictly to emulator TCP + 4–5 primitives only. Defer USB/auth complexity |
| Log parser produces too much noise | Medium | Low | Keep extremely high-signal only in Phase 1; improve in Phase 2 |
| Cleanup fails (app left on device) | Low | Medium | Use `finally` / scope guard pattern; always attempt uninstall; document manual cleanup |
| Policy gate too permissive in early code | Low | High | Copy-paste from existing high-risk handlers (`auth-test`, wireless deauth) and review early |
| Emulator environment differences across machines | Medium | Low | Document exact AVD setup + provide minimal reproducible test APK in repo |

---

## 10. Handoff Checklist

- [ ] Team reviews and approves this Phase 1 plan (and confirms ADB strategy decision).
- [ ] Create feature branch `feature/mobile-dynamic-phase1`.
- [ ] Assign owners to the 12 deliverables above.
- [ ] Set up emulator test environment for the smoke test owner.
- [ ] Create GitHub issues or task list from the deliverables table.
- [ ] Schedule daily standup for first week (many small decisions expected).
- [ ] After core working: run full `cargo test --features mobile-dynamic && cargo clippy`.
- [ ] Merge to main only after smoke test passes and docs are updated.
- [ ] Update parent design plan status to “Phase 1 in progress” once branch is active.

**Immediate Next Action**: Team decides on ADB implementation strategy (pure-Rust TCP vs fallback) in the next planning meeting, then starts coding on the feature branch.

---

## 11. References

- Parent design: `plans/dynamic-mobile-testing-loadout-design-plan.md`
- Current static code: `crates/eggsec/src/mobile/{mod,apk,ipa}.rs`, `cli/mobile.rs`
- Policy patterns: `commands/handlers/mobile.rs`, `auth-test` handler, wireless deauth handler
- Wireless implementation plans (for tone and granularity): `plans/wireless-active-loadout-cli-integration-plan.md` etc.
- Docs to update: `docs/MOBILE.md`, `architecture/mobile.md`, `AGENTS.override.md`

---

**End of Phase 1 Implementation Handoff Plan**

This document is designed to be the single artifact the implementation team needs to begin productive work on the dynamic mobile loadout with minimal context switching.