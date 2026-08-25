# Evasion Detection Module

## Role & Responsibilities

Defense-lab-only module for validating that security controls detect common evasion techniques. This is **defense validation (detection)**, not offense tooling. It produces structured reports with MITRE ATT&CK mappings and confidence scores.

**Non-responsibilities**: Does not perform active exploitation or evasion. Does not generate attack payloads. Does not integrate with MCP/agent/TUI/pipeline surfaces (standalone defense-lab CLI module). Does not perform network traffic analysis (traffic obfuscation techniques are placeholder-only requiring proxy interception).

## Location & Feature Gating

| Item | Location | Gate |
|------|----------|------|
| Module declaration | `crates/eggsec/src/lib.rs:185-186` | `#[cfg(feature = "evasion")]` |
| No stub module | — | When disabled, the module does not exist |
| Feature flag | `crates/eggsec/Cargo.toml:381` | `evasion = []` (marker, no deps) |
| C2 dependency | `crates/eggsec/Cargo.toml:389` | `c2 = ["postex", "evasion"]` |
| CLI handler | `crates/eggsec/src/commands/handlers/evasion.rs:5` | `#[cfg(feature = "cli")]` |
| `run_cli()` | `evasion/mod.rs:824` | `#[cfg(feature = "cli")]` |

## Architecture

### Files (3 total)

| File | Lines | Description |
|------|-------|-------------|
| `evasion/mod.rs` | 1208 | Core: `EvasionScanner`, `EvasionReport`, `EvasionDetection`, `EvasionTechnique`, 16 default techniques, detection checks per category, `to_scan_report_data()`, `run_cli()`, tests |
| `cli/evasion.rs` | — | `EvasionArgs` + `EVASION_ABOUT` (target, type, pid, dry-run, json, output, quiet) |
| `commands/handlers/evasion.rs` | 33 | `handle_evasion` with `EnforcementContext` (EvasionTesting risk + DefenseLab mode) |

### Key Types

| Type | Location | Description |
|------|----------|-------------|
| `EvasionScanner` | `mod.rs:122` | `dry_run`, `techniques` (Vec) — main scanner engine |
| `EvasionReport` | `mod.rs:106` | `target`, `detections` (Vec), `summary`, `timestamp`, `dry_run` |
| `EvasionDetection` | `mod.rs:97` | `technique`, `detected`, `confidence`, `evidence`, `recommendations` |
| `EvasionTechnique` | `mod.rs:47` | `id`, `name`, `mitre_id` (Option), `category`, `risk_level`, `description` |
| `EvasionSummary` | `mod.rs:114` | `total_techniques`, `detected`, `not_detected`, `detection_rate` |
| `EvasionTarget` | `mod.rs:18` | `target_type`, `path`, `pid` |
| `EvasionTargetType` | `mod.rs:26` | 5 variants: `Process`, `File`, `Network`, `Registry`, `Memory` |
| `EvasionCategory` | `mod.rs:58` | 6 variants: `Syscall`, `HookBypass`, `Obfuscation`, `Injection`, `AntiAnalysis`, `TrafficObfuscation` |
| `EvasionRisk` | `mod.rs:69` | 4 variants: `Low`, `Medium`, `High`, `Critical` — with `to_severity()` mapping |

### Technique Inventory (16 total, 6 categories)

All techniques defined in `default_techniques()` at `mod.rs:135-271`. Every technique has a unique ID and a MITRE ATT&CK ID (enforced by test at `mod.rs:959-968`).

#### Syscall (2 techniques)

| ID | Name | MITRE | Risk | Source |
|----|------|-------|------|--------|
| `evasion-syscall-001` | Direct Syscall Detection | T1106 | High | `mod.rs:137-145` |
| `evasion-syscall-002` | Indirect Syscall Detection | T1106 | High | `mod.rs:146-154` |

#### Hook Bypass (3 techniques)

| ID | Name | MITRE | Risk | Source |
|----|------|-------|------|--------|
| `evasion-hook-001` | ETW Patching Detection | T1562.006 | Critical | `mod.rs:155-162` |
| `evasion-hook-002` | AMSI Bypass Detection | T1562.001 | Critical | `mod.rs:163-170` |
| `evasion-hook-003` | Userland Hook Unhooking | T1014 | High | `mod.rs:171-179` |

#### Obfuscation (2 techniques)

| ID | Name | MITRE | Risk | Source |
|----|------|-------|------|--------|
| `evasion-obf-001` | String Obfuscation Detection | T1027 | Medium | `mod.rs:180-188` |
| `evasion-obf-002` | Code Segment Obfuscation | T1027.005 | Medium | `mod.rs:189-196` |

#### Injection (3 techniques)

| ID | Name | MITRE | Risk | Source |
|----|------|-------|------|--------|
| `evasion-inj-001` | Process Hollowing Detection | T1055.012 | Critical | `mod.rs:197-204` |
| `evasion-inj-002` | DLL Side-Loading Detection | T1574.002 | High | `mod.rs:205-212` |
| `evasion-inj-003` | Reflective DLL Loading | T1620 | Critical | `mod.rs:213-220` |

#### Anti-Analysis (3 techniques)

| ID | Name | MITRE | Risk | Source |
|----|------|-------|------|--------|
| `evasion-anti-001` | VM Detection | T1497.001 | Medium | `mod.rs:221-228` |
| `evasion-anti-002` | Debugger Detection | T1622 | Medium | `mod.rs:229-236` |
| `evasion-anti-003` | Timing-Based Evasion | T1497 | Low | `mod.rs:237-245` |

#### Traffic Obfuscation (3 techniques)

| ID | Name | MITRE | Risk | Source |
|----|------|-------|------|--------|
| `evasion-traffic-001` | Domain Fronting Detection | T1090.004 | High | `mod.rs:246-253` |
| `evasion-traffic-002` | DNS-over-HTTPS Tunneling | T1071.004 | High | `mod.rs:254-261` |
| `evasion-traffic-003` | Jittered Beacon Detection | T1071 | Medium | `mod.rs:262-270` |

### Unique MITRE IDs

16 unique MITRE ATT&CK IDs: T1106, T1562.006, T1562.001, T1014, T1027, T1027.005, T1055.012, T1574.002, T1620, T1497.001, T1622, T1497, T1090.004, T1071.004, T1071. Note: T1106 is shared by 2 techniques (direct + indirect syscall).

## Behavior / Flow

### `EvasionScanner::scan(target)` — `mod.rs:273-308`

1. Build target label from type + path/pid (`:275-279`)
2. Dispatch to `dry_run_detections()` or `real_detections()` based on `self.dry_run` (`:281-285`)
3. Compute summary: detected count, detection rate (`:287-299`)
4. Return `EvasionReport` with timestamp (`:301-308`)

### Dry-Run Mode — `mod.rs:310-335`

All 16 techniques reported as "detected" with deterministic confidence by risk level: Critical=0.85, High=0.75, Medium=0.65, Low=0.55. Each gets synthetic evidence and two generic recommendations.

### Real Detection — `mod.rs:337-353`

Dispatches per-category:

- **Syscall** (`check_syscall_evasion`, `:355-405`): Reads target binary, searches for 4 byte patterns (`syscall`, `NtCreateFile`, `NtWriteVirtualMemory`, `ZwCreateSection`). Confidence = 0.3 + (matches × 0.15), capped at 0.8.
- **Hook Bypass** (`check_hook_bypass`, `:407-488`): Searches binary for technique-specific symbols. ETW: `EtwpEventWrite`/`EventWrite`/`ntdll!Etwp` (confidence 0.4). AMSI: `AmsiScanBuffer`/`AmsiScanString`/`amsiInitFailed`/`amsi.dll` (0.45). Unhooking: `VirtualProtect`/`NtProtectVirtualMemory` (0.25).
- **Obfuscation** (`check_obfuscation`, `:490-550`): String obfuscation: XOR pattern density (>24/32-byte window = suspicious, >5 occurrences). Code obfuscation: NOP ratio (>30% of binary).
- **Injection** (`check_injection`, `:552-642`): Process target on Linux: reads `/proc/<pid>/maps` for anonymous RWX regions (hollowing) or temp-path libraries (side-loading). File target: searches for reflective loading APIs (`LoadLibraryA`/`GetProcAddress`/`VirtualAlloc`, ≥2 matches).
- **Anti-Analysis** (`check_anti_analysis`, `:644-739`): VM detection: 8 VM strings (VMware, VirtualBox, VBOX, QEMU, Xen, Hyper-V, svm, kvm). Debugger: 4 APIs (IsDebuggerPresent, CheckRemoteDebuggerPresent, NtQueryInformationProcess, OutputDebugString). Timing: 4 APIs (SleepEx, QueryPerformanceCounter, rdtsc, rdtscp), requires >1 match.
- **Traffic Obfuscation** (`check_traffic_obfuscation`, `:741-770`): **Placeholder only** — always returns `detected: false` with evidence noting that TLS SNI inspection / DNS capture / network flow monitoring is required.

### Safety Model

- Handler forces `dry_run: true` even when real mode is requested (`commands/handlers/evasion.rs:25-28`)
- Policy gate: `OperationRisk::EvasionTesting` + `OperationMode::DefenseLab` (`commands/handlers/evasion.rs:8-10`)
- Real mode prints warning (`commands/handlers/evasion.rs:19-22`)
- All operations are passive (file read, `/proc/<pid>/maps` read, byte-pattern matching)

### Reporting Bridge — `to_scan_report_data()` (`mod.rs:788-822`)

Converts `EvasionReport` to `ScanReportData` for SARIF/JUnit/HTML consumers. Only detected findings are bridged (filtered at `:794`). Category mapping uses `evasion-*` prefix (e.g., `evasion-syscall`, `evasion-hook-bypass`).

## Public API

| Function | Signature | Description |
|----------|-----------|-------------|
| `EvasionScanner::new` | `pub fn new(dry_run: bool) -> Self` | Create scanner with 16 default techniques |
| `EvasionScanner::scan` | `pub async fn scan(&self, target: &EvasionTarget) -> Result<EvasionReport>` | Run detection against target |
| `EvasionScanner::techniques` | `pub fn techniques(&self) -> &[EvasionTechnique]` | Access technique list |
| `to_scan_report_data` | `pub fn to_scan_report_data(report: &EvasionReport) -> ScanReportData` | Bridge to unified report format |
| `run_cli` | `pub async fn run_cli(args: EvasionArgs, config: &EggsecConfig) -> Result<()>` | CLI entry point (gated `cli`) |

## Integration Points

### CLI

`handle_evasion()` in `commands/handlers/evasion.rs:5` routes through `EnforcementContext` with:
- `OperationMode::DefenseLab` (`:8`)
- `OperationRisk::EvasionTesting` (`:9`)
- `IntendedUse::WafRegression` (`:10`)

Handler forces `dry_run: true` (`:25`) for safety, regardless of user input.

### C2 Dependency

`c2 = ["postex", "evasion"]` (`Cargo.toml:389`). The C2 module depends on evasion for integrated campaign orchestration. See [c2.md](c2.md) for details.

### Defense Lab Framing

The module is explicitly framed as defense-lab-only in `mod.rs:1-11` doc comments and the CLI output messages (`mod.rs:838-847`). See [defense_lab.md](defense_lab.md) for the broader defense-lab architecture.

### No Dispatch/MCP/TUI Integration

The evasion module is standalone. It does not register as an MCP tool, is not wired into `dispatch/` workers, and has no TUI tab.

## Testing

16 tests in `evasion/mod.rs`:

- `test_evasion_risk_as_str` (`:917`) — risk level string conversion
- `test_evasion_risk_to_severity` (`:925`) — risk → severity mapping
- `test_evasion_scanner_creation` (`:933`) — scanner constructor, technique count = 16
- `test_default_techniques_categories` (`:942`) — all 6 categories present
- `test_techniques_have_mitre_ids` (`:959`) — every technique has `mitre_id: Some(...)`
- `test_techniques_have_unique_ids` (`:971`) — no duplicate technique IDs
- `test_dry_run_scan_produces_all_detected` (`:984`) — all 16 detected in dry-run
- `test_dry_run_confidence_by_risk_level` (`:1002`) — deterministic confidence values
- `test_real_scan_nonexistent_target` (`:1028`) — graceful handling of missing files
- `test_scan_with_target_path` (`:1046`) — target label formatting
- `test_scan_with_pid` (`:1057`) — pid label formatting
- `test_evasion_category_for_mapping` (`:1069`) — category → string mapping
- `test_to_scan_report_data_bridge` (`:1097`) — bridge only includes detected findings
- `test_to_scan_report_data_empty` (`:1149`) — empty report produces empty findings
- `test_serialization_roundtrip` (`:1167`) — JSON round-trip
- `test_evasion_target_type_serialization` (`:1187`) — snake_case serialization
- `test_evasion_category_serialization` (`:1195`) — snake_case serialization
- `test_evasion_risk_serialization` (`:1203`) — snake_case serialization

## Invariants & Gotchas

1. **Defense-lab only**: Every scan is defense validation, not offense. The module doc says "no active exploitation" (`mod.rs:9`).
2. **Handler forces dry-run**: Even if user passes real mode, `handle_evasion()` overrides to `dry_run: true` (`commands/handlers/evasion.rs:25`).
3. **Traffic obfuscation is placeholder**: `check_traffic_obfuscation()` (`:741-770`) always returns `detected: false` with a message that network interception is needed. This is documented in the code.
4. **Injection detection is Linux-only**: `check_injection()` for process targets reads `/proc/<pid>/maps` under `#[cfg(target_os = "linux")]` (`:565-595`). On non-Linux, process injection checks always return undetected.
5. **MITRE ID uniqueness not enforced**: `test_techniques_have_unique_ids` checks technique IDs are unique, not MITRE IDs. T1106 is shared by 2 techniques.
6. **Binary read paths**: `check_syscall_evasion`, `check_hook_bypass`, `check_obfuscation`, `check_anti_analysis` all read the target file via `tokio::fs::read()`. Non-existent paths silently produce undetected results (the `if let Ok(bytes)` pattern).

## Bugs / Observations

| Location | Issue | Severity |
|----------|-------|----------|
| `mod.rs:564` | `let _ = &pid;` — unused variable suppression in the `#[cfg(target_os = "linux")]` block. The `pid` is used inside the cfg block, but this line exists to suppress a warning on non-Linux. Followed by proper use | Informational |
| `mod.rs:366` | `tokio::fs::read(path)` in `check_syscall_evasion` — reads entire binary into memory. Large binaries could cause OOM | Medium |
| `mod.rs:417` | Same pattern in `check_hook_bypass` — reads entire binary | Medium |
| `mod.rs:500` | Same pattern in `check_obfuscation` — reads entire binary | Medium |
| `mod.rs:654` | Same pattern in `check_anti_analysis` — reads entire binary | Medium |
| `commands/handlers/evasion.rs:25-28` | Handler silently overrides `dry_run` to `true` without returning the override to the caller. User sees no indication their `--dry-run false` was ignored unless `!args.quiet` | Low |

*Last verified against source: 2026-08-25*
