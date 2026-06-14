# Evasion Module Guidance

## Purpose
Defense-lab-only module for validating that security controls detect common evasion techniques.

## Key Patterns
- Feature-gated under `evasion`
- Dry-run always safe (synthetic results only)
- Real operations require `--allow-evasion-testing` + `EnforcementContext`
- Standalone defense-lab surface (no MCP/agent/TUI integration)
- MITRE ATT&CK mapping for each technique

## Commands
- `eggsec evasion <target>` - Run evasion detection against a target
- `eggsec evasion <target> --dry-run` - Plan mode (no real checks)

## Safety
- Dry-run produces complete report with synthetic data
- Real mode requires explicit `--allow-evasion-testing`
- Policy: `OperationRisk::Intrusive` (high-risk tier)
- Mode: `OperationMode::DefenseLab`

## Technique Categories
- **Syscall**: Direct/indirect syscall detection (T1106)
- **Hook Bypass**: ETW patching, AMSI bypass, unhooking (T1562.001, T1562.006, T1014)
- **Obfuscation**: String/code obfuscation (T1027, T1027.005)
- **Injection**: Process hollowing, DLL side-loading, reflective loading (T1055.012, T1574.002, T1620)
- **Anti-Analysis**: VM/debugger detection, timing evasion (T1497, T1497.001, T1622)
- **Traffic Obfuscation**: Domain fronting, DoH tunneling, jittered beacons (T1071, T1071.004, T1090.004)

## Verification Commands

```bash
cargo check -p eggsec --features evasion
cargo test --lib -p eggsec --features evasion
cargo clippy --lib -p eggsec --features evasion
```
