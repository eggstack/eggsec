# C2 Module - Agent Override

## Module Overview

The C2 module provides a lightweight Rust-native Command & Control framework for defense-lab purple teaming and red team simulation.

## Key Files

- `mod.rs` - Core types, C2Scanner, to_scan_report_data, run_cli
- `beacon.rs` - Beacon protocol simulation (HTTP/S, DNS, TCP)
- `tasking.rs` - Task queue and execution simulation
- `campaign.rs` - Campaign profile catalog
- `opsec.rs` - OPSEC scoring and anti-forensics assessment

## Safety Constraints

- **Always dry-run by default**: The handler forces `dry_run: true` regardless of user input
- **Policy gated**: `OperationRisk::C2Operation` requires `allow_c2_operations` in `ExecutionPolicy`
- **Lab-only**: All operations are simulated; no real C2 infrastructure is deployed
- **Depends on postex + evasion**: Feature `c2 = ["postex", "evasion"]`

## Patterns

- Follows the standalone defense-lab surface pattern (same as evasion, postex, wireless)
- `to_scan_report_data()` bridge converts `C2Report` to unified `ScanReportData`
- Auto-bridged in `report convert` handler
- No MCP/agent/TUI/pipeline integration (standalone only)

## Verification Commands

```bash
cargo check -p eggsec --features c2
cargo test --lib -p eggsec --features c2
cargo clippy --lib -p eggsec --features c2
```
