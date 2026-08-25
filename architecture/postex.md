# Post-Exploitation Module

## Role & Responsibilities

Standalone defense-lab module for simulating post-exploitation techniques against authorized lab targets. Provides **16 default techniques across 4 categories** (4 each) for purple teaming and defense validation. This is **simulation and detection validation**, not offensive tooling. Techniques are dry-run safe: zero side effects, complete reports with synthetic detections and confidence scores.

**Non-responsibilities**: Does not perform real post-exploitation. Does not integrate with MCP, TUI pipeline, or agent surfaces (standalone defense-lab CLI). Does not interact with remote systems in dry-run mode.

## Location & Feature Gating

| Item | Location | Gate |
|------|----------|------|
| Module declaration | `crates/eggsec/src/postex/` | `#[cfg(feature = "postex")]` |
| Feature flag | `crates/eggsec/Cargo.toml` | `postex = []` (marker, no deps) |
| Included in `full` | `crates/eggsec/Cargo.toml` | `full = [..., "postex", ...]` |
| CLI handler | `crates/eggsec/src/commands/handlers/postex.rs:5` | `#[cfg(feature = "cli")]` |
| CLI args | `crates/eggsec/src/cli/postex.rs` | `#[cfg(feature = "cli")]` |
| `run_cli()` | `crates/eggsec/src/postex/mod.rs:449` | `#[cfg(feature = "cli")]` |

## Architecture

### Files (7 total)

| File | Lines | Description |
|------|-------|-------------|
| `postex/mod.rs` | 488 | Core types (`PostexCategory`, `PostexRisk`, `PostexTechnique`, `PostexDetection`, `PostexReport`, `PostexProfile`, `PostexScanner`), default technique registry (16 entries), `to_scan_report_data()` bridge, `run_cli()` entry point, tests |
| `postex/lotl.rs` | 87 | Living-Off-The-Land command wrappers: `LotpCommand` enum (10 variants), MITRE mappings, risk levels, `simulate_lotl()` |
| `postex/persistence.rs` | 113 | Persistence mechanism simulation: `PersistenceType` enum (6 variants), MITRE mappings, `simulate_persistence()`, `generate_cleanup_command()` |
| `postex/lateral.rs` | 92 | Lateral movement simulation: `LateralTechnique` enum (6 variants), MITRE mappings, `simulate_lateral()` |
| `postex/credential.rs` | 100 | Credential access simulation: `CredentialTechnique` enum (6 variants), MITRE mappings, `simulate_credential()` with rate-limiting note for password spray |
| `postex/report.rs` | 68 | Human/JSON report formatting: `format_human_report()`, `format_json_report()`, tests |

### Key Types

| Type | Location | Description |
|------|----------|-------------|
| `PostexCategory` | `mod.rs:13` | Enum: `Lotl`, `Persistence`, `LateralMovement`, `CredentialAccess` |
| `PostexRisk` | `mod.rs:34` | Enum: `Low`, `Medium`, `High`, `Critical` (ordered) |
| `PostexTechnique` | `mod.rs:54` | Technique definition: id, name, mitre_id, category, risk, description, reversible |
| `PostexDetection` | `mod.rs:66` | Simulation result: technique, simulated flag, confidence (0.0–1.0), evidence, recommendations |
| `PostexReport` | `mod.rs:76` | Full report: target, detections, summary, timestamp, dry_run, actions_performed |
| `PostexSummary` | `mod.rs:87` | Aggregate stats: total, simulated, not_simulated, category counts |
| `PostexProfile` | `mod.rs:98` | Enum: `Minimal`, `Standard` (default), `Aggressive` |
| `PostexScanner` | `mod.rs:115` | Scanner struct: dry_run flag, profile, filtered techniques |

### Default Technique Registry (16 techniques, 4×4)

All 16 default techniques are defined in `PostexScanner::default_techniques()` at `mod.rs:236-389`:

#### LOTL (Living-Off-The-Land) — `mod.rs:238-275`

| ID | Name | MITRE | Risk | Reversible | Description |
|----|------|-------|------|------------|-------------|
| `lotl-001` | PowerShell Execution | T1059.001 | High | Yes | Detection of PowerShell-based command execution for defense evasion |
| `lotl-002` | WMIC Process Creation | T1047 | Medium | Yes | Detection of WMIC-based process creation and query operations |
| `lotl-003` | Certutil Download | T1105 | High | Yes | Detection of certutil.exe used for file download/decode |
| `lotl-004` | Rundll32 Execution | T1218.011 | Medium | Yes | Detection of rundll32.exe loading malicious DLLs |

#### Persistence — `mod.rs:276-312`

| ID | Name | MITRE | Risk | Reversible | Description |
|----|------|-------|------|------------|-------------|
| `persist-001` | Registry Run Key | T1547.001 | High | Yes | Detection of registry-based persistence via Run/RunOnce keys |
| `persist-002` | Scheduled Task | T1053.005 | High | Yes | Detection of scheduled task creation for persistence |
| `persist-003` | Service Creation | T1543.003 | Critical | Yes | Detection of Windows service creation for persistence |
| `persist-004` | DLL Side-Loading | T1574.002 | Critical | Yes | Detection of DLL side-loading via search order hijacking |

#### Lateral Movement — `mod.rs:313-348`

| ID | Name | MITRE | Risk | Reversible | Description |
|----|------|-------|------|------------|-------------|
| `lateral-001` | SMB Lateral Movement | T1021.002 | High | Yes | Detection of SMB-based lateral movement techniques |
| `lateral-002` | RDP Lateral Movement | T1021.001 | High | Yes | Detection of RDP-based lateral movement |
| `lateral-003` | Port Forwarding | T1090 | Medium | Yes | Detection of network port forwarding for pivoting |
| `lateral-004` | SOCKS Proxy | T1090.002 | Medium | Yes | Detection of SOCKS proxy setup for traffic relay |

#### Credential Access — `mod.rs:349-389`

| ID | Name | MITRE | Risk | Reversible | Description |
|----|------|-------|------|------------|-------------|
| `cred-001` | LSASS Memory Dump | T1003.001 | Critical | **No** | Detection of LSASS process memory access for credential extraction |
| `cred-002` | Token Impersonation | T1134 | High | Yes | Detection of access token manipulation for privilege escalation |
| `cred-003` | Password Spraying | T1110.003 | High | Yes | Detection of password spraying against authentication endpoints |
| `cred-004` | Kerberoasting | T1558.003 | Critical | Yes | Detection of Kerberos service ticket extraction for offline cracking |

### Extended Technique Enums (domain modules)

The domain modules (`lotl.rs`, `persistence.rs`, `lateral.rs`, `credential.rs`) define richer enums used by the CLI and simulation logic:

- `LotpCommand` — 10 variants in `lotl.rs:3-16`: PowerShell, Wmic, Certutil, Rundll32, Msiexec, Mshta, Regsvr32, Bash, Curl, Wget
- `PersistenceType` — 6 variants in `persistence.rs:5-12`: RegistryRunKey, ScheduledTask, ServiceCreation, DllHijack, StartupFolder, WmiEventSubscription
- `LateralTechnique` — 6 variants in `lateral.rs:5-12`: SmbShare, RdpSession, PortForward, SocksProxy, WinRm, PsExec
- `CredentialTechnique` — 6 variants in `credential.rs:5-12`: LsassDump, TokenImpersonation, PasswordSpray, Kerberoasting, Dcsync, LdapQuery

These produce additional `PostexTechnique` instances via `to_technique()` with their own IDs (e.g., `lotl-T1059-001`, `persist-registry`, `lateral-smb`, `cred-lsass`) and MITRE mappings, but the **default technique registry** that defines the canonical 16 is the one at `mod.rs:236-389`.

## Behavior / Flow

### Scan Lifecycle

```
CLI args → handle_postex() → EnforcementContext → PostexScanner::new(dry_run, profile) → scan(target)
  ├─ dry_run  → dry_run_simulations() → synthetic detections with risk-based confidence
  └─ real     → real_simulations() → lower-confidence detections (defense-lab mode, no real execution)
```

1. **Construction**: `PostexScanner::new(dry_run, profile)` loads `default_techniques()` (16) and filters by profile:
   - `Minimal`: risk ≤ Medium (8 techniques: lotl-002/004, persist-001/002, lateral-003/004, cred-002/003)
   - `Standard`: all 16 (default)
   - `Aggressive`: all 16 (same as Standard, expansion point for future)
2. **Execution**: `scan(target)` dispatches to `dry_run_simulations()` or `real_simulations()`:
   - **Dry-run**: produces `PostexDetection` per technique with confidence = 0.85 (Critical) / 0.75 (High) / 0.65 (Medium) / 0.55 (Low), evidence prefixed "dry-run:", `simulated: true`
   - **Real**: produces detections with flat confidence 0.30, `simulated: false`, evidence marked "(defense-lab mode)"
3. **Summary**: `build_summary()` counts detections per category, total/simulated/not_simulated
4. **Output**: `run_cli()` formats as human-readable or JSON, writes to file or stdout

### Handler Policy (`commands/handlers/postex.rs:5-37`)

- **Mode**: `DefenseLab`
- **Risk**: `SafeActive` for dry-run, `ExploitAdjacent` for real
- **Override**: Handler **always forces `dry_run: true`** regardless of CLI args (line 29). Real mode is effectively unreachable from the CLI handler.

### Cleanup Commands

`persistence.rs:91-112` generates platform-specific cleanup commands for each persistence type:
- Registry: `reg delete HKLM\...\Run /v EggsecLab /f`
- Scheduled Task: `schtasks /delete /tn EggsecLab /f`
- Service: `sc delete EggsecLabService`
- DLL: `Remove-Item -Path "$env:TEMP\eggsec_lab.dll" -Force`
- Startup: `Remove-Item -Path "$env:APPDATA\...\Startup\eggsec_lab.lnk" -Force`
- WMI: `Get-WmiObject ... | Remove-WmiObject`

### Dry-Run Contract

- Zero side effects (no real technique execution)
- Complete report with synthetic detections for all techniques in profile
- Confidence scores based on risk level (Critical=0.85, High=0.75, Medium=0.65, Low=0.55)
- Cleanup commands generated for reversible techniques (only in dry-run evidence; not executed)

## Safety Model

- **Dry-run default**: Handler forces `dry_run: true` at `postex.rs:29`, overriding any CLI input
- **Scope**: `DefenseLab` mode only
- **Risk gating**: `SafeActive` (dry) / `ExploitAdjacent` (real, unreachable from CLI)
- **No real execution**: `real_simulations()` produces low-confidence detections but does NOT execute techniques — it is a simulation stub
- **Explicit allow**: No `--allow-postex` gate exists; the handler's forced dry-run is the safety boundary
- **LotL only**: LOTL techniques reference living-off-the-land binaries (PowerShell, WMIC, certutil, etc.) — no custom payloads or exploit code

## Public API

| Function | Location | Description |
|----------|----------|-------------|
| `PostexScanner::new(dry_run, profile)` | `mod.rs:122` | Construct scanner with profile-filtered techniques |
| `PostexScanner::scan(target)` | `mod.rs:143` | Execute scan (async), returns `PostexReport` |
| `PostexScanner::techniques()` | `mod.rs:139` | Borrow filtered technique list |
| `to_scan_report_data(report)` | `mod.rs:394` | Bridge to `ScanReportData` for unified reporting |
| `run_cli(args, config)` | `mod.rs:449` | CLI entry point (feature-gated: `cli`) |
| `simulate_lotl(command, target)` | `lotl.rs:71` | Single LOTL technique simulation |
| `simulate_persistence(type, target)` | `persistence.rs:72` | Single persistence technique simulation |
| `simulate_lateral(technique, src, tgt)` | `lateral.rs:72` | Single lateral movement simulation |
| `simulate_credential(technique, target)` | `credential.rs:78` | Single credential access simulation |
| `generate_cleanup_command(type)` | `persistence.rs:91` | Platform-specific cleanup command |

## Integration Points

- **CLI dispatch**: `commands/handlers/mod.rs:560` → `handle_postex()`
- **CLI args**: `cli/postex.rs` — `PostexArgs` (target, dry_run, profile, category, json, output, quiet)
- **Reporting bridge**: `to_scan_report_data()` produces `ScanReportData` with `postex-*` categories (e.g., `postex-living-off-the-land`, `postex-persistence`, `postex-lateral-movement`, `postex-credential-access`, `postex-summary`)
- **Auto-bridge**: `report convert` handler detects `postex` scan_type and converts automatically
- **C2 integration**: C2 tasking module (`c2/tasking.rs`) references postex technique IDs for enrichment — LOTL/lateral/credential/persistence MITRE techniques are mapped to C2 task types via `postex_enrichment()`
- **Python bindings**: Not exposed (standalone defense-lab surface)
- **Feature gate**: `postex` feature in Cargo.toml; included in `full`

## Testing

- Unit tests in `postex/mod.rs` (scanner construction, technique filtering, dry-run scan, report bridge)
- Unit tests in `postex/report.rs` (human/JSON formatting)
- Integration tests via `make test-ci` (postex features in full profile)
- Handler tests verify forced dry-run override

## Invariants & Gotchas

1. **Handler forces dry-run**: `postex.rs:29` always sets `dry_run: true` — real mode is unreachable from CLI
2. **16 default techniques**: Exactly 4 categories × 4 techniques each, defined at `mod.rs:236-389`
3. **Minimal profile filters**: Uses `risk <= Medium` ordering (`mod.rs:127`), not explicit list
4. **`Aggressive == Standard`**: Both return all 16 techniques; `Aggressive` is a future expansion point (`mod.rs:130`)
5. **Only one irreversible technique**: `cred-001` (LSASS Memory Dump, `reversible: false`) at `mod.rs:357`
6. **Extended enums produce different IDs**: Domain module enums (`LotpCommand`, etc.) generate IDs like `lotl-T1059-001` (via `lotl.rs:60`), distinct from the default registry IDs (`lotl-001`)
7. **No real execution**: Both `dry_run_simulations()` and `real_simulations()` produce synthetic results — neither executes actual post-exploitation techniques
8. **Bridge skips non-simulated**: `to_scan_report_data()` filters `d.simulated` (`mod.rs:400`), so real-mode detections (always `simulated: false`) produce no bridged findings

---

*Last verified against source: 2026-08-25*
