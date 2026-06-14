---
name: eggsec-evasion
description: "Evasion technique detection for defense validation - MITRE ATT&CK mapped, dry-run safe"
triggers:
  - evasion
  - evasion detection
  - defense evasion
  - mitre att&ck
  - evasion techniques
  - ewr patching
  - amsi bypass
  - process hollowing
  - dll side-loading
  - reflective loading
  - vm detection
  - debugger detection
  - domain fronting
  - doh tunneling
  - jittered beacon
metadata:
  category: security
  tools: [evasion, scanner]
  scope: eggsec-evasion
---

## Overview

The evasion module (`crates/eggsec/src/evasion/`) validates that security controls detect common evasion techniques used by malware and advanced threats. It maps detections to MITRE ATT&CK IDs with confidence scores and produces structured reports.

## Key Components

| Component | File | Purpose |
|-----------|------|---------|
| `EvasionScanner` | `evasion/mod.rs:122` | Main scanner: `new(dry_run)`, `scan(&target)`, `techniques()` |
| `EvasionReport` | `evasion/mod.rs:105` | Full report: target, detections, summary, timestamp, dry_run |
| `EvasionDetection` | `evasion/mod.rs:96` | Per-technique: technique, detected, confidence, evidence, recommendations |
| `EvasionTechnique` | `evasion/mod.rs:46` | Definition: id, name, mitre_id, category, risk_level, description |
| `EvasionTarget` | `evasion/mod.rs:17` | Scan input: target_type, path, pid |
| `to_scan_report_data()` | `evasion/mod.rs:710` | Bridge to unified `ScanReportData` |

## Features

```
evasion = []  # Marker-only, no dependencies
```

## Workflow

1. Build with evasion feature: `cargo build --features evasion`
2. Run dry-run first: `eggsec evasion --target /path/to/binary --dry-run --json`
3. Review detection results and confidence scores
4. Run real checks if authorized: `eggsec evasion --target /path/to/binary --allow-evasion-testing`

## Safety

- Always use `--dry-run` for planning (synthetic results, no side effects)
- Real mode requires explicit `--allow-evasion-testing`
- Defense-lab only; never against production systems
- Policy: `OperationRisk::EvasionTesting` (high-risk tier) + `OperationMode::DefenseLab`
- Handler forces dry-run even when real mode is requested (safety override)

## Technique Categories

| Category | Techniques | Detection Method |
|----------|------------|------------------|
| Syscall | 2 (T1106) | Binary pattern matching for syscall strings |
| Hook Bypass | 3 (T1562.006, T1562.001, T1014) | Symbol detection for ETW, AMSI, memory protection APIs |
| Obfuscation | 2 (T1027, T1027.005) | XOR pattern density, NOP ratio analysis |
| Injection | 3 (T1055.012, T1574.002, T1620) | `/proc/<pid>/maps` analysis, reflective loading API detection |
| Anti-Analysis | 3 (T1497.001, T1622, T1497) | VM string detection, debugger API detection, timing API detection |
| Traffic Obfuscation | 3 (T1090.004, T1071.004, T1071) | Placeholder — requires proxy/network flow monitoring |

## Confidence Scoring

- **Dry-run**: Deterministic by risk level — Critical=0.85, High=0.75, Medium=0.65, Low=0.55
- **Real mode**: Dynamic based on pattern match count and technique-specific thresholds

## Key Commands

```bash
# Dry-run (always safe)
eggsec evasion --target /path/to/binary --dry-run --json

# File analysis
eggsec evasion --target /path/to/binary --type file --json

# Process analysis
eggsec evasion --target /path/to/binary --type process --pid 1234

# Network target
eggsec evasion --type network --dry-run --json

# Save report
eggsec evasion --target /path/to/binary --dry-run --json -o report.json
```

## Architecture

- Module: `crates/eggsec/src/evasion/mod.rs` (1022 lines, 16 techniques, 6 categories)
- CLI: `crates/eggsec/src/cli/evasion.rs` (EvasionArgs, EVASION_ABOUT)
- Handler: `crates/eggsec/src/commands/handlers/evasion.rs` (EnforcementContext, EvasionTesting risk)
- Feature: `evasion` (marker-only, no deps)
- Bridge: `to_scan_report_data()` for unified reporting (evasion-* categories)

## Verification Commands

```bash
cargo check -p eggsec --features evasion
cargo test --lib -p eggsec --features evasion
cargo clippy --lib -p eggsec --features evasion
```

## Common Patterns

### Running a Dry-Run Scan

```rust
use eggsec::evasion::{EvasionScanner, EvasionTarget, EvasionTargetType};

let scanner = EvasionScanner::new(true); // dry_run = true
let target = EvasionTarget {
    target_type: EvasionTargetType::File,
    path: Some("/path/to/binary".to_string()),
    pid: None,
};
let report = scanner.scan(&target).await?;
println!("Detection rate: {:.0}%", report.summary.detection_rate * 100.0);
```

### Bridging to Unified Reports

```rust
use eggsec::evasion::{EvasionScanner, to_scan_report_data};

let scanner = EvasionScanner::new(true);
let report = scanner.scan(&target).await?;
let scan_data = to_scan_report_data(&report);
// scan_data can be used with SARIF, JUnit, HTML, etc. formatters
```

## Error Handling

Use explicit error handling instead of `unwrap_or_default()`:
```rust
let report = match scanner.scan(&target).await {
    Ok(report) => report,
    Err(e) => {
        tracing::warn!("Evasion scan failed: {}", e);
        return Err(e);
    }
};
```

## Testing

```bash
cargo test --lib -p eggsec --features evasion
```

Tests cover: risk-to-severity mapping, technique uniqueness, dry-run confidence levels, detection coverage, serialization roundtrips, and the `to_scan_report_data` bridge.
