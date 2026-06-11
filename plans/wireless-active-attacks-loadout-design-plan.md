# Wireless Active Attacks Loadout Design & Implementation Plan

**Date**: 2026-06-11  
**Status**: Draft — Ready for Team Review & Handoff  
**Branch**: `feature/wireless-active-attacks-loadout-plan`  
**Related**: Passive wireless module (newly added 2026-06-11), `docs/WIRELESS.md`, `architecture/wireless.md`, `plans/wireless-*-plan.md` series, `docs/SAFETY.md`, EnforcementContext / OperationRisk model  
**Authoring Note**: Generated via detailed analysis of current codebase using GitHub connector tools. Intended as a complete handoff artifact for the eggstack team to implement the active expansion.

---

## 1. Executive Summary

This plan outlines the design and phased implementation for expanding Eggsec's newly added **passive wireless reconnaissance module** into a **gated Active Wireless / WiFi Attacks loadout**.

**Goal**: Provide controlled, lab-only active attack primitives (deauthentication, handshake forcing/capture, basic DoS flooding, Evil Twin simulation, etc.) strictly for **defense validation, regression testing, and WIDS/WIPS evaluation** in authorized environments. The loadout must preserve and extend Eggsec's rigorous safety model (scope/policy gating, feature flags, budgets, auditability, explicit warnings).

**Key Principles** (non-negotiable):
- **Defense-lab / regression focus only** — never a general exploitation or offensive framework.
- **Standalone-complete surface** like the current passive `eggsec wireless` command and TUI tab (MCP/agent tool exposure intentionally absent initially, consistent with current wireless design decision).
- **Phased & heavily gated**: New `wireless-advanced` feature flag. Additional runtime confirmations, packet budgets, `--allow-active-wireless` style overrides (audited), and lab-context manifests where appropriate.
- **Leverage existing patterns**: Extend `WirelessScanner` / analysis, `to_scan_report_data()` bridge, EnforcementContext (`SafeActive` or new `ActiveWireless` tier), CLI handler structure, TUI tab architecture (recent 10-phase updates), and reporting pipeline.
- **Pragmatic implementation**: Pure-Rust 802.11 frame crafting + pnet where feasible for core primitives; optional, gated subprocess wrappers (aireplay-ng, hostapd, etc.) for reliable injection on real hardware.
- **Documentation-first**: Every capability includes prominent legal/ethical warnings, hardware requirements, and lab-only usage guidance.

**Deliverables**:
- New `plans/wireless-active-attacks-loadout-design-plan.md` (this document).
- Feature flag `wireless-advanced` (depends on `wireless`).
- New module(s) under `crates/eggsec/src/wireless/` (e.g., `active.rs` or `attacks/` subdir).
- CLI extensions (`eggsec wireless deauth ...`, `capture-handshake`, etc.).
- TUI attack controls and monitoring.
- Extended findings schema + reporting bridge.
- Updated docs (`WIRELESS.md`, `architecture/wireless.md`, `SAFETY.md`, `CAPABILITIES.md`, README).
- Unit + lab hardware tests.

**Timeline Suggestion (Aggressive but Realistic)**: Phase 1 (deauth core) in 2–3 weeks post-review; full loadout (Phases 1–3) in 6–8 weeks with parallel TUI/policy work.

**Success Criteria**: All active operations require explicit feature + policy approval; dry-run produces valid structured output; lab regression workflows (e.g., "measure WIPS detection latency under deauth flood") are fully supported with before/after findings and temporal summaries.

---

## 2. Background & Current State

### 2.1 Passive Wireless Module (Newly Added)

The passive module (`crates/eggsec/src/wireless/mod.rs`, ~57 KB) provides:
- `iwlist <iface> scan` parsing into rich `WirelessNetwork` (SSID, BSSID, channel, `SecurityType` enum, signal, WPS, hidden, transition_mode).
- `analyze_networks()` producing `WirelessVulnerability` findings (Open/WEP/WPA legacy, WPS, hidden, transition, weak signal, **passive rogue/Evil-Twin heuristic** based on same-SSID multiple BSSID or security differences).
- `--repeat`, `--known-good` allowlist (suppresses rogue for lab baselines), `--dry-run`, `--detect-suspicious`, native JSON + `to_scan_report_data()` bridge to `ScanReportData` (with `wireless-*` finding categories).
- Temporal diff/summary logic for repeated scans.
- Standalone CLI (`eggsec wireless <iface>`) + TUI tab + reporting integration.
- **Explicitly passive-only** by design and documentation.

Key quotes from current artifacts (2026-06-11 state):
> "**This is passive reconnaissance only.** No packet injection, deauthentication, handshake capture, or active attacks are implemented in this standalone module."
> "Future phases may add a `wireless-advanced` sub-feature for gated active/lab-only capabilities."
> (See `docs/WIRELESS.md` "Not In Scope" and `architecture/wireless.md` MCP/Agentic section: standalone defense-lab surface, intentionally **not** registered as `SecurityTool` for MCP/agent dispatch.)

The module integrates cleanly with the central policy model (`EnforcementContext`, `OperationRisk::SafeActive` + feature requirement) and recent TUI architecture improvements (UiAction, preflight, policy indicators).

### 2.2 Related Existing Capabilities
- `stress-testing` feature: SYN/UDP/ICMP/HTTP floods, proxy pool — already demonstrates controlled aggressive traffic with budgets and lab gating.
- `packet-inspection`: pnet-based live capture, hexdump, traceroute.
- `auth-test`: Standalone defense-lab credential control validation (separate from pipeline `ScanProfile::Auth`).
- These provide proven patterns for feature gating, warnings, local findings, optional bridges, and strict enforcement in automated paths.

### 2.3 Why Expand Now?
The passive module is production-ready for reconnaissance and rogue hunting. Adding a complementary **active loadout** enables full defense-in-depth validation loops: baseline passive scan → simulate attacks → measure detection/response → regression over time. This mirrors mature wireless defense testing practices while staying inside Eggsec's safety philosophy.

---

## 3. Goals, Non-Goals, and Scope

### 3.1 Primary Goals
- Deliver a curated set of **active attack primitives** usable for:
  - Testing WIDS/WIPS, client roaming behavior, AP resilience under deauth/disassoc.
  - Forcing re-authentication to capture handshakes in controlled lab conditions.
  - Simulating Evil Twin / rogue scenarios to validate detection heuristics and physical response procedures.
  - Basic DoS / availability testing of wireless infrastructure.
- Maintain **zero accidental misuse surface**: every active path requires the new feature flag + explicit policy confirmation (or audited override).
- Produce structured, reportable findings (new `wireless-active-*` categories) that integrate with existing `ScanReportData`, SARIF, JUnit, HTML/Markdown pipelines, and temporal/repeat workflows.
- Provide excellent dry-run / planning support and human-readable + JSON output consistent with passive `wireless`.
- Extend (not duplicate) the existing `WirelessScanner` / analysis / recommendation engine where sensible.

### 3.2 Non-Goals (Explicitly Out of Scope for This Plan)
- General-purpose exploitation framework or "all-in-one" wireless pentest tool (no goal to match aircrack-ng/wifite feature parity).
- Production or internet-facing offensive use.
- Unfettered packet injection or monitor mode creation without user/hardware prerequisites and warnings.
- Bluetooth/BLE, Zigbee, or other non-802.11 wireless.
- Full automated WPS PIN brute-force or advanced KRACK-style exploitation (aspirational future phases only, after core loadout proven).
- Changes to MCP/agent tool registry exposure for wireless (remain absent / standalone defense-lab by default; future opt-in only after security review).
- Windows or macOS native active support in Phase 1 (Linux + pnet/iw/aireplay focus).

### 3.3 In-Scope Attack Primitives (Phased)

| Phase | Primitive                  | Description                                                                 | Risk Tier     | Example CLI Command                              | Safety / Gating Notes                          | Priority |
|-------|----------------------------|-----------------------------------------------------------------------------|---------------|--------------------------------------------------|------------------------------------------------|----------|
| 1     | Targeted Deauth / Disassoc | Send 802.11 deauthentication or disassociation frames to specific BSSID/client | High         | `eggsec wireless wlan0 deauth --bssid AA:BB:... --client MAC --count 20` | Packet budget, confirm, lab-only, audited override | P0      |
| 1     | Broadcast Deauth           | Deauth all clients on a BSSID (DoS / roaming test)                         | High         | `... deauth --bssid ... --broadcast`            | Same + explicit `--allow-broadcast-deauth`    | P0      |
| 2     | Handshake Capture + Force  | Passive capture (or with deauth trigger) of WPA/WPA2 4-way handshake       | Medium-High  | `eggsec wireless wlan0 capture-handshake --essid CorpNet --deauth` | Requires monitor mode; output .cap or JSON   | P1      |
| 3     | Beacon / Probe Flood       | Flood beacons or probe responses for availability / DoS resilience testing | High         | `eggsec wireless wlan0 beacon-flood --ssid FakeNet --duration 30` | Strict rate limiting + budget; lab channel only | P2      |
| 3     | Evil Twin / Rogue Sim      | Stand up minimal rogue AP (or simulate via frame injection) on lab channel to test detection | High         | `eggsec wireless wlan0 evil-twin --ssid CorpNet --channel 6 --duration 120` | Isolated lab channel + known-good suppression; hostapd wrapper or pure Rust | P2      |
| 4+    | WPS Advanced, PMKID, KRACK-style | Controlled simulation / detection testing (non-destructive)               | Very High    | Future subcommands                               | Additional sub-feature or explicit allowlist  | Future  |

**Phase 1 Focus**: Deauth/Disassoc core (highest immediate value for defense testing, relatively contained implementation).

---

## 4. Safety, Policy & Enforcement Model Extensions

### 4.1 Feature Flag
```toml
# crates/eggsec/Cargo.toml
wireless-advanced = ["wireless"]  # depends on base wireless
```
CLI/TUI features propagate it. Full build includes it optionally.

### 4.2 Risk / Operation Classification
- Extend `OperationRisk` enum or add capability flag `ActiveWireless`.
- CLI handler (`commands/handlers/wireless.rs`) and TUI `TabSpec` use `SafeActive` today for passive; active paths require new `ActiveWireless` (or `HighRisk` + explicit `allowed_capabilities`).
- `EnforcementContext::evaluate()` must treat active operations as non-downgradable in strict profiles.
- New denial/confirmation classes for "active wireless requires explicit lab authorization".

### 4.3 Runtime Gating & UX
- Prominent startup / pre-execution warning (even more visible than current passive warning):
  > "ACTIVE WIRELESS ATTACK MODE — Requires root/CAP_NET_ADMIN + monitor-mode capable interface + explicit authorization. For lab defense validation ONLY."
- `--dry-run` always supported and produces valid structured JSON (no injection).
- `--allow-active-wireless` (narrow override, audited, like other high-risk flags) + optional `--manual-override-reason`.
- Packet / frame count budgets (e.g., `--max-frames 1000` default low; configurable in profile or env).
- Optional lab manifest file (analogous to `--known-good` but for "authorized attack targets"): list of allowed BSSIDs/SSIDs + channels for active ops.
- In TUI: Pre-flight policy indicator, confirmation overlay, live packet counter, emergency stop.

### 4.4 Legal / Ethical / Documentation Requirements
Every new command, TUI screen, and generated finding must surface:
- "Use only on networks and hardware you own or have explicit written authorization to test."
- Reference to local spectrum regulations.
- Link to `docs/SAFETY.md` and new "Active Wireless" section in `WIRELESS.md`.

### 4.5 MCP / Agent Exposure
**Recommendation**: Keep wireless (including advanced) as a **standalone defense-lab surface**. Do not register active commands as `SecurityTool` in the tool registry. This preserves the current intentional design decision and avoids complex policy surface for autonomous agents. Reporting bridge remains available for any consumer that obtains native JSON output.

---

## 5. Technical Architecture

### 5.1 Module Structure
```
crates/eggsec/src/wireless/
├── mod.rs                 # Existing passive (keep mostly unchanged; re-export active types)
├── active.rs              # NEW: Active primitives, frame builders, attack runners
├── attacks/
│   ├── deauth.rs          # Deauth / Disassoc frame crafting + send
│   ├── handshake.rs       # Capture + optional deauth trigger
│   ├── flood.rs           # Beacon / probe flood
│   └── rogue.rs           # Evil Twin simulation logic
└── types.rs               # Extend or add AttackResult, ActiveFinding, etc.
```

`active.rs` re-exports and provides high-level `run_deauth(...)`, `run_capture_handshake(...)`, etc., called from updated CLI handler.

### 5.2 Data Models (Additions)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveWirelessAttackResult {
    pub interface: String,
    pub attack_type: String,           // "deauth", "handshake-capture", ...
    pub target_bssid: Option<String>,
    pub frames_sent: u64,
    pub duration_secs: u64,
    pub findings: Vec<ActiveWirelessFinding>,
    pub raw_output: Option<String>,    // .cap path or hexdump summary
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveWirelessFinding {
    pub attack_type: String,
    pub severity: Severity,
    pub description: String,
    pub evidence: String,              // e.g. "Sent 47 deauth frames to BSSID... observed X client disconnects"
    pub remediation: String,
}
```

Extend `to_scan_report_data()` or add `to_active_scan_report_data()` that populates `ScanReportData` with new `wireless-active-*` categories and enriches `wireless_networks` or adds attack metadata.

### 5.3 Frame Crafting & Injection Layer
**Preferred Approach (Pragmatic)**:
- Pure Rust for frame construction (radiotap header + 802.11 deauth/disassoc structure using `pnet_packet` or custom bitfields).
- Example skeleton in plan (to be implemented in `attacks/deauth.rs`):
```rust
#[repr(C, packed)]
struct DeauthFrame {
    radiotap: [u8; ...],
    ieee80211: [u8; 26], // FC (deauth), duration, addr1/2/3, seq, reason code
}

pub fn build_deauth(bssid: &[u8;6], client: Option<&[u8;6]>, reason: u16) -> Vec<u8> { ... }

pub async fn inject_frames(iface_mon: &str, frames: &[Vec<u8>], rate_limit: Duration) -> Result<u64> {
    // pnet or raw socket + AF_PACKET / SOCK_RAW with ETH_P_ALL or radiotap
}
```
- For reliable injection on diverse hardware: Optional, feature-gated subprocess fallback to `aireplay-ng` (or `mdk4` for flooding) when `external-tools` sub-feature enabled. Pure Rust path is primary for reproducibility and to avoid external binary dependencies.
- Monitor mode: Require user to provide `--monitor-iface wlan0mon` or auto-detect/create via `iw` / airmon-ng wrapper (gated).

**pnet Leverage**: Already present in `stress-testing` and `packet-inspection`. Extend usage or add conditional `pnet` re-export under `wireless-advanced`.

### 5.4 CLI Integration
Extend existing `WirelessArgs` or introduce subcommand structure (clap derive):
```bash
eggsec wireless <iface> deauth [OPTIONS]
eggsec wireless <iface> capture-handshake [OPTIONS]
# etc.
```
New flags: `--bssid`, `--client`, `--count` / `--duration`, `--reason-code`, `--broadcast`, `--monitor-iface`, `--max-frames`, `--dry-run`, `--json`, `--output`, `--allow-active-wireless`, etc.

Handler in `commands/handlers/wireless.rs` dispatches to `wireless::active::run_*` after policy check.

### 5.5 TUI Integration
- Add attack mode toggle / dedicated sub-view in WirelessTab.
- Use existing `UiAction`, `OverlayController`, preflight status, global task strip.
- Live metrics: frames sent/sec, observed client behavior (if passive listener running concurrently), policy state.
- Confirmation dialogs for high-risk actions.
- Consistent semantic styling (risk = high for active attacks).

### 5.6 Dependencies & Build
- Core: No new mandatory deps for Phase 1 (reuse pnet when feature enabled).
- Optional: `pcap` crate or direct libc for advanced socket control; subprocess for aireplay/hostapd under `wireless-advanced + external-tools`.
- System: Document `libpcap-dev`, `aircrack-ng` (optional), `hostapd` (for Evil Twin phase), `iw`.

### 5.7 Concurrency & Safety
- Active attacks should be able to run while a passive background scan continues (or pause it).
- Hard limits on concurrent active operations.
- Graceful shutdown / packet drain on Ctrl-C or TUI stop.

---

## 6. Detailed CLI Command Designs (Phase 1 Focus)

### 6.1 Deauth / Disassoc
```bash
# Dry run (valid JSON, no privileges or injection)
eggsec wireless wlan0 deauth --bssid 00:11:22:33:44:55 --count 50 --dry-run --json

# Targeted client deauth (lab only)
sudo eggsec wireless wlan0 deauth \
  --bssid 00:11:22:33:44:55 \
  --client aa:bb:cc:dd:ee:ff \
  --count 30 \
  --reason-code 7 \
  --allow-active-wireless \
  --manual-override-reason "Authorized WIPS regression test on lab AP"

# Broadcast (all clients) — requires extra confirmation
sudo eggsec wireless wlan0 deauth --bssid ... --broadcast --max-frames 200
```

**Output (human)**: Summary of frames sent, observed effects (if listener active), findings (e.g. "X clients observed disconnecting / reassociating"), recommendations ("Verify WIPS logged event within acceptable latency").

**JSON**: `ActiveWirelessAttackResult` (or wrapped for repeat).

### 6.2 Handshake Capture (Phase 2)
```bash
eggsec wireless wlan0 capture-handshake --essid "CorpNet" --channel 6 --deauth-trigger --deauth-count 5 --output lab-handshake.cap --json
```

Integrates with existing passive scan to identify target first.

### 6.3 Other Phase 3+ (Sketched)
- `beacon-flood`, `probe-flood`, `evil-twin` — similar structure with duration/rate controls and lab-channel enforcement.

---

## 7. Reporting, Findings & Output

New finding categories:
- `wireless-active-deauth`
- `wireless-active-handshake`
- `wireless-active-flood`
- `wireless-active-rogue-sim`

`to_scan_report_data()` (or dedicated active bridge) populates:
- `findings` with title, severity, description, evidence (rich: frames sent, observed impact, channel, etc.), remediation.
- Optional `attack_metadata` or extension of `wireless_networks`.
- Full support for repeat/temporal summaries ("Attack #3 showed improved WIPS detection latency of 1.2s vs baseline 4.7s").

Native JSON from active commands is accepted by `eggsec report convert` (auto-bridge when `wireless-advanced` feature present).

---

## 8. Phased Implementation Roadmap

**Phase 0 (This Plan — Complete)**: Handoff document created and pushed.

**Phase 1 (Core Deauth — P0, ~2–3 weeks)**:
- Feature flag + Cargo plumbing.
- `active.rs` + `attacks/deauth.rs` with pure-Rust frame builder + basic pnet/raw injection.
- CLI args + handler dispatch + policy gate.
- Dry-run path + JSON schema.
- Unit tests (frame construction, policy stubs).
- Minimal TUI action stub.
- Update `WIRELESS.md` (new "Active Attacks" section), architecture, SAFETY, CAPABILITIES, README quick-ref.
- Lab hardware smoke test on known AP + client.

**Phase 2 (Handshake + Polish — P1)**:
- Capture logic + optional deauth trigger.
- Richer findings + evidence.
- Full TUI attack controls + live monitoring.
- `--known-good` / lab-manifest integration for active targets.
- Repeat / temporal support for attack runs.
- Documentation examples and best-practice workflows ("defense regression loop").

**Phase 3 (Flood + Evil Twin Sim — P2)**:
- Flood primitives.
- Evil Twin simulation (pure Rust beacon/probe or hostapd wrapper).
- Advanced policy (channel lock to lab frequencies, extra confirmation for rogue sim).
- Performance / rate limiting hardening.

**Phase 4+ (Future)**: WPS, PMKID, advanced simulation, possible limited MCP opt-in after review.

**Cross-Cutting Work** (parallel):
- EnforcementContext / OperationRisk extensions.
- TUI preflight / confirmation patterns (reuse recent architecture).
- Output convert bridge tests.
- Hardware compatibility matrix doc.

**Testing Strategy**:
- Unit: Frame builders, parsers, analysis, dry-run paths, serde roundtrips (no hardware).
- Integration: Policy enforcement tests (mock EnforcementContext).
- Lab hardware: Dedicated test matrix (supported chipsets, monitor mode creation, injection success rate, observed client/AP behavior). Use `--dry-run` heavily in CI.
- Regression: Add wireless-active examples to defense-lab profiles / CI jobs once stable.

---

## 9. Risks, Edge Cases & Mitigations

| Risk / Edge Case                        | Impact                          | Mitigation                                                                 | Owner     |
|-----------------------------------------|---------------------------------|----------------------------------------------------------------------------|-----------|
| Hardware lacks reliable injection / monitor mode | Feature unusable on many laptops | Clear error + guidance; optional aireplay wrapper; document supported chipsets | Impl team |
| User runs active commands on production / unauthorized networks | Legal / operational incident   | Multi-layer warnings + policy gate + lab-manifest + prominent disclaimers in all output | Policy + Docs |
| Frame crafting bugs cause kernel panic or driver crash | Stability issue                | Careful testing, bounded buffers, graceful error paths; start with low rates | Impl      |
| Concurrent passive + active interference | Confusing results or dropped frames | Explicit mutex or cooperative pause; document best practice               | TUI/CLI   |
| Regulatory / spectrum rules vary by country | Compliance risk                | Strong disclaimers + "know your local laws" in every help text and finding | Docs      |
| Misuse of Evil Twin sim for real phishing | Severe reputational / legal    | Phase 3 gating behind extra allowlist + isolated channel enforcement; never auto-channel hop | Policy    |
| Performance under high frame rates      | System load or dropped packets | Configurable rate limits + budgets; background task isolation             | Impl      |
| MCP/agent accidental exposure           | Policy bypass risk             | Do not register; keep standalone; future opt-in only after security audit | Arch      |

**Monitoring for Abuse**: All active operations produce auditable policy decisions and findings even in dry-run / JSON paths.

---

## 10. Open Questions & Decisions Needed (for Team)

1. Exact feature flag name: `wireless-advanced` (preferred, matches existing comment) vs. `wireless-attacks` vs. `wireless-active`?
2. Primary injection strategy for Phase 1: Pure Rust pnet/raw socket first, or hybrid with aireplay-ng wrapper from day one for broader hardware support?
3. Should active attacks support a lightweight "lab manifest" file (allowed BSSIDs + channels) analogous to `--known-good`, or rely solely on runtime `--allow-active-wireless` + reason?
4. Confirm MCP/agent exposure remains **absent** for the entire wireless surface (including advanced) in this round?
5. Desired default packet budget / rate limit values for deauth (e.g., 100 frames / 10 pkt/s)?
6. Any preference for reason code handling (hardcode common values or expose `--reason-code`)?
7. Should Evil Twin phase include a minimal pure-Rust AP simulator, or start with hostapd subprocess wrapper?

---

## 11. Handoff Checklist

- [ ] Review & approve this plan (team + security).
- [ ] Merge `feature/wireless-active-attacks-loadout-plan` or cherry-pick the plan file to main.
- [ ] Create follow-up issues for Phase 1 tasks (feature flag, deauth module, CLI, policy gate, docs updates).
- [ ] Assign owners for cross-cutting work (EnforcementContext changes, TUI patterns).
- [ ] Update `AGENTS.md` or internal notes if needed for contributor context.
- [ ] After Phase 1 implementation: Run full test suite (`cargo test --features wireless-advanced`), lab smoke tests, and generate sample reports.
- [ ] Post-implementation: Update `docs/WIRELESS.md` "Active Attacks" section with real examples and workflows; refresh architecture diagram if changed.
- [ ] Consider adding a short ADR in `docs/adr/` for the active loadout safety model decision.

**Immediate Next Action After Handoff**: Team decides on feature flag name and injection strategy (pure Rust vs hybrid), then starts Phase 1 implementation on a new feature branch.

---

## 12. References & Further Reading

- Current passive implementation: `crates/eggsec/src/wireless/mod.rs`
- CLI: `crates/eggsec/src/cli/wireless.rs` + `commands/handlers/wireless.rs`
- TUI: `crates/eggsec-tui/src/tabs/wireless.rs` + workers
- Output bridge: `crates/eggsec-output/src/convert.rs`
- Policy core: `crates/eggsec/src/.../enforcement*` and `OperationRisk`
- pnet usage examples: `crates/eggsec/src/stress/` and `src/packet/`
- Full docs: `docs/WIRELESS.md`, `architecture/wireless.md`, `docs/SAFETY.md`, `docs/CAPABILITIES.md`
- Existing plans: `plans/auth-tui-full-integration-handoff-plan.md` and historical wireless plans in `docs/plans/` or repo history
- Related standalone surfaces: `auth-test`, `mobile`

---

**End of Plan Document**

*This document is intended as a complete, self-contained handoff artifact. It captures context, rationale, detailed design, risks, and actionable roadmap so the team can implement without ambiguity while preserving Eggsec's core safety and quality standards.*
