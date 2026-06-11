# Wireless Feature - First Handoff Plan (Standalone State)

**Status**: Initial Handoff Plan  
**Focus**: Bring wireless capabilities to a usable standalone state  
**Philosophy**: Defense validation and reconnaissance first (passive + controlled active where safe)

---

## 1. Current State Summary

**Existing Foundation** (`crates/eggsec/src/wireless/`):
- Passive scanning using `iwlist` (Linux).
- Parses SSID, BSSID, channel, signal strength, and basic security type (Open / WEP / WPA / WPA2 / WPA3 / Enterprise).
- Basic vulnerability analysis (`analyze_networks()`) that flags obvious issues.
- Generates security recommendations.
- Has `to_scan_report_data()` for output integration.
- CLI command exists (`eggsec wireless <interface>`) with JSON/file output support.
- Feature-gated behind `wireless`.

**Strengths**:
- Already has decent passive scanning and reporting hooks.
- Clean data models (`WirelessNetwork`, `WirelessScanResult`, `WirelessVulnerability`).

**Limitations / Gaps**:
- No rogue AP / Evil Twin detection.
- No controlled handshake capture or analysis (noted as aspirational).
- Limited active testing capabilities (deauth, etc.).
- Basic analysis only (no deeper WPS, PMKID, or configuration checks).
- Documentation is minimal.
- No dedicated defense-lab profiles or regression workflows yet.

---

## 2. Goal: Usable Standalone State

By the end of this phase, `eggsec wireless` should be a reliable tool for:
- Passive wireless reconnaissance in authorized environments.
- Basic security posture assessment of WiFi networks.
- Detection of obviously weak or misconfigured networks (Open, WEP, legacy WPA).
- Clean, structured output suitable for reports and further processing.
- Safe, documented usage with clear scope and permission requirements.

**Not in Scope for this phase**:
- Full active attack capabilities (e.g., handshake cracking, Evil Twin attacks in production).
- Deep WPS / PMKID / KRACK-style testing.
- Bluetooth / BLE (can be added later).

---

## 3. Prioritized Tasks

### Task 1: Enhance Passive Scanning & Analysis (High Priority)

**Goal**: Improve detection quality and add more useful insights from passive scans.

**Actions**:
- Improve `parse_scan_output()` to better detect:
  - WPS enabled networks
  - Transition mode (WPA2/WPA3 mixed)
  - Hidden SSIDs
- Enhance `analyze_networks()` with more findings:
  - Weak signal strength warnings
  - Duplicate SSID / possible rogue AP detection (basic version)
  - Enterprise networks with weak EAP methods (if detectable)
- Add more structured recommendations.

**Files**:
- `crates/eggsec/src/wireless/mod.rs`

### Task 2: Rogue AP / Suspicious Network Detection

**Goal**: Add basic rogue AP and Evil Twin style detection suitable for defense validation.

**Actions**:
- Add logic to detect:
  - Same SSID with different BSSID/security
  - Known good networks appearing on unexpected channels
  - Sudden appearance of new networks during repeated scans
- Make this opt-in or clearly labeled as "suspicious network detection".

**Safety Note**: This is still passive and relatively safe.

### Task 3: CLI & Output Polish

**Goal**: Make the command pleasant and useful as a standalone tool.

**Actions**:
- Improve `WirelessArgs` if needed (e.g., `--repeat` for continuous monitoring, `--json` already exists).
- Enhance `run_cli()` with better formatting and progress indication.
- Ensure `to_scan_report_data()` is consistently used when integrated with scan pipelines.
- Add clear warnings about root requirements and interface permissions.

**Files**:
- `crates/eggsec/src/cli/wireless.rs`
- `crates/eggsec/src/wireless/mod.rs`

### Task 4: Documentation

**Goal**: Make the feature usable without deep code diving.

**Actions**:
- Create or significantly expand `docs/WIRELESS.md`
- Update `README.md`, `CAPABILITIES.md`, and `SAFETY.md`
- Document:
  - Requirements (root, wireless tools, permissions)
  - Recommended lab vs production usage
  - Example commands and output interpretation
  - Safety warnings

### Task 5: Basic Controlled Active Capabilities (Stretch / Later)

Only if time permits in this phase:
- Add very basic deauth/disassoc testing (lab-only, heavily gated).
- This should be behind additional safety checks and clearly marked as advanced.

**Recommendation**: Defer active capabilities to a follow-up plan unless the team specifically wants them now.

---

## 4. Safety & Scope Considerations

Wireless testing has unique risks:
- Requires root for packet injection / certain scans.
- Active testing can disrupt legitimate networks.
- Legal implications of monitoring or transmitting on certain frequencies.

**Recommended Guardrails**:
- Strong emphasis on "authorized lab / defensive validation" use in all messaging.
- Clear documentation of required privileges.
- Consider adding a `wireless-advanced` sub-feature for any active testing later.
- Integrate with existing `EnforcementContext` where possible.

---

## 5. Suggested Implementation Order

1. **Task 1** (Enhanced passive analysis) — Quick wins with high value.
2. **Task 3** (CLI polish) — Improves day-to-day usability.
3. **Task 2** (Rogue detection) — Adds defensive value.
4. **Task 4** (Documentation) — Makes the feature adoptable.
5. Task 5 only if specifically requested.

---

## 6. Success Criteria for This Phase

- `eggsec wireless <interface>` produces useful, structured output with security findings and recommendations.
- Basic rogue/suspicious network detection is available.
- The command is well-documented and safe to use in lab environments.
- Output integrates reasonably with existing reporting paths.
- The feature feels like a solid, standalone wireless reconnaissance and basic assessment tool.

---

**This is the first handoff plan for wireless. The goal is to reach a usable standalone state before investing heavily in deep pipeline integration.**