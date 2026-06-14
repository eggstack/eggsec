# Evasion Detection Module

## Overview

Defense-lab-only module for validating that security controls detect common evasion techniques. Maps detections to MITRE ATT&CK IDs and produces structured reports with confidence scores.

## Architecture

- Feature-gated under `evasion`
- Standalone defense-lab surface (no MCP/agent/TUI/pipeline integration)
- 16 techniques across 6 categories (Syscall, HookBypass, Obfuscation, Injection, AntiAnalysis, TrafficObfuscation)
- Dry-run always safe (synthetic results)
- Real mode requires `--allow-evasion-testing`
- Policy: `OperationRisk::EvasionTesting` (high-risk tier)
- Mode: `OperationMode::DefenseLab`

## CLI Behavior

- Build with `--features evasion` (or `--features full`).
- `eggsec evasion --target <path> --dry-run --json` — plan mode with synthetic results.
- `eggsec evasion --target <path> --type file` — file analysis mode.
- `eggsec evasion --target <path> --type process --pid 1234` — process analysis.
- `eggsec evasion --type network --dry-run --json` — network target without path.
- Real mode forced back to dry-run by handler for safety (`commands/handlers/evasion.rs` forces `dry_run: true`).

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `EvasionScanner` | `evasion/mod.rs` | Main scanner engine (new, scan, techniques) |
| `EvasionReport` | `evasion/mod.rs` | Full report (target, detections, summary, timestamp, dry_run) |
| `EvasionDetection` | `evasion/mod.rs` | Per-technique result (technique, detected, confidence, evidence, recommendations) |
| `EvasionTechnique` | `evasion/mod.rs` | Technique definition (id, name, mitre_id, category, risk_level, description) |
| `EvasionSummary` | `evasion/mod.rs` | Aggregate stats (total, detected, not_detected, detection_rate) |
| `EvasionTarget` | `evasion/mod.rs` | Scan target (target_type, path, pid) |
| `EvasionCategory` | `evasion/mod.rs` | Category enum (Syscall, HookBypass, Obfuscation, Injection, AntiAnalysis, TrafficObfuscation) |
| `EvasionRisk` | `evasion/mod.rs` | Risk enum (Low, Medium, High, Critical) with `to_severity()` |
| `EvasionTargetType` | `evasion/mod.rs` | Target type enum (Process, File, Network, Registry, Memory) |
| `to_scan_report_data()` | `evasion/mod.rs` | Bridge to unified `ScanReportData` |

## Files

| File | Description |
|------|-------------|
| `evasion/mod.rs` | Core: scanner, models, 16 techniques, detection checks, `to_scan_report_data`, `run_cli` |
| `cli/evasion.rs` | `EvasionArgs` + `EVASION_ABOUT` (target, type, pid, dry-run, json, output, quiet) |
| `commands/handlers/evasion.rs` | `handle_evasion` with `EnforcementContext` (EvasionTesting risk + DefenseLab mode) |

## Techniques (16 total)

### Syscall (2)
| ID | Name | MITRE | Risk |
|----|------|-------|------|
| `evasion-syscall-001` | Direct Syscall Detection | T1106 | High |
| `evasion-syscall-002` | Indirect Syscall Detection | T1106 | High |

### Hook Bypass (3)
| ID | Name | MITRE | Risk |
|----|------|-------|------|
| `evasion-hook-001` | ETW Patching Detection | T1562.006 | Critical |
| `evasion-hook-002` | AMSI Bypass Detection | T1562.001 | Critical |
| `evasion-hook-003` | Userland Hook Unhooking | T1014 | High |

### Obfuscation (2)
| ID | Name | MITRE | Risk |
|----|------|-------|------|
| `evasion-obf-001` | String Obfuscation Detection | T1027 | Medium |
| `evasion-obf-002` | Code Segment Obfuscation | T1027.005 | Medium |

### Injection (3)
| ID | Name | MITRE | Risk |
|----|------|-------|------|
| `evasion-inj-001` | Process Hollowing Detection | T1055.012 | Critical |
| `evasion-inj-002` | DLL Side-Loading Detection | T1574.002 | High |
| `evasion-inj-003` | Reflective DLL Loading | T1620 | Critical |

### Anti-Analysis (3)
| ID | Name | MITRE | Risk |
|----|------|-------|------|
| `evasion-anti-001` | VM Detection | T1497.001 | Medium |
| `evasion-anti-002` | Debugger Detection | T1622 | Medium |
| `evasion-anti-003` | Timing-Based Evasion | T1497 | Low |

### Traffic Obfuscation (3)
| ID | Name | MITRE | Risk |
|----|------|-------|------|
| `evasion-traffic-001` | Domain Fronting Detection | T1090.004 | High |
| `evasion-traffic-002` | DNS-over-HTTPS Tunneling | T1071.004 | High |
| `evasion-traffic-003` | Jittered Beacon Detection | T1071 | Medium |

## Detection Methods

Each category dispatches to a dedicated check method:

- **Syscall**: Binary pattern matching for syscall-related strings (`syscall`, `NtCreateFile`, `NtWriteVirtualMemory`, `ZwCreateSection`)
- **Hook Bypass**: Symbol detection for ETW (`EtwpEventWrite`), AMSI (`AmsiScanBuffer`), and memory protection APIs (`VirtualProtect`)
- **Obfuscation**: XOR pattern density (encoded strings) and NOP ratio (code obfuscation)
- **Injection**: Linux `/proc/<pid>/maps` analysis for RWX regions and temp-path libraries; reflective loading API detection in binaries
- **Anti-Analysis**: VM string detection (`VMware`, `VirtualBox`), debugger API detection (`IsDebuggerPresent`), timing API detection (`SleepEx`, `rdtsc`)
- **Traffic Obfuscation**: Placeholder — requires proxy interception or network flow monitoring (not implemented in static analysis)

## Confidence Scoring

Dry-run confidence is deterministic by risk level: Critical=0.85, High=0.75, Medium=0.65, Low=0.55. Real-mode confidence is dynamic based on pattern match count and technique-specific thresholds.

## Safety

- Dry-run always produces complete reports with synthetic data
- Real mode requires explicit `--allow-evasion-testing` flag
- Policy gate: `OperationRisk::EvasionTesting` + `DefenseLab` mode
- Handler forces `dry_run: true` even when real mode is requested (safety override)
- All operations are passive (file inspection, process enumeration, network patterns)
- No active exploitation

## Integration with Reporting Pipeline

Produces local `EvasionReport` + findings directly (human/JSON via CLI). Optional `to_scan_report_data()` bridge converts to canonical `ScanReportData` for SARIF/JUnit/HTML/etc. consumers. Bridge uses `evasion-*` categories (e.g., `evasion-syscall`, `evasion-hook-bypass`, `evasion-obfuscation`, `evasion-injection`, `evasion-anti-analysis`, `evasion-traffic-obfuscation`). Only detected findings are bridged (non-detected filtered out).

## Commands

```bash
eggsec evasion --target /path/to/binary --dry-run          # Plan mode
eggsec evasion --target /path/to/binary --json             # JSON output
eggsec evasion --target /path/to/binary --type file        # File analysis
eggsec evasion --target /path/to/binary --type process --pid 1234  # Process analysis
eggsec evasion --type network --dry-run --json             # Network target
```
