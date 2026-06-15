# Post-Exploitation Module Guidance

## Purpose
Defense-lab-only module for simulating post-exploitation techniques (LOTL, persistence, lateral movement, credential access) for purple teaming and defense validation.

## Key Patterns
- Feature-gated under `postex`
- Dry-run always safe (synthetic results only)
- Real operations require `--allow-postex` + `EnforcementContext`
- Standalone defense-lab surface (no MCP/agent/TUI integration)
- MITRE ATT&CK mapping for each technique
- Reversible actions in lab mode (cleanup commands generated)

## Module Structure
- `mod.rs` - Types, `PostexScanner`, `to_scan_report_data` bridge, `run_cli` entry
- `lotl.rs` - Living-Off-The-Land command wrappers (PowerShell, WMIC, certutil, etc.)
- `persistence.rs` - Persistence mechanism simulation (registry, scheduled task, service, DLL hijack)
- `lateral.rs` - Lateral movement simulation (SMB, RDP, port forwarding, SOCKS proxy)
- `credential.rs` - Credential access simulation (LSASS dump, token impersonation, password spray, Kerberoasting)
- `report.rs` - Human/JSON report formatting

## Commands
- `eggsec postex --target <ip> --dry-run` - Dry-run simulation
- `eggsec postex --target <ip> --profile minimal --dry-run` - Minimal profile
- `eggsec postex --target <ip> --profile aggressive --dry-run` - All techniques
- `eggsec postex --category lotl --dry-run --json` - LOTL only

## Safety
- Dry-run produces complete report with synthetic data
- Real mode requires explicit `--allow-postex` flag
- Policy: `OperationRisk::PostExploitation` (high-risk tier)
- Mode: `OperationMode::DefenseLab`
- All techniques marked reversible (except LSASS dump)

## Technique Categories
- **LOTL**: PowerShell (T1059.001), WMIC (T1047), certutil (T1105), rundll32 (T1218.011), msiexec, mshta, regsvr32, bash, curl, wget
- **Persistence**: Registry run key (T1547.001), scheduled task (T1053.005), service creation (T1543.003), DLL hijack (T1574.002), startup folder, WMI subscription
- **Lateral Movement**: SMB (T1021.002), RDP (T1021.001), port forwarding (T1090), SOCKS proxy (T1090.002), WinRM (T1021.006), PsExec
- **Credential Access**: LSASS dump (T1003.001), token impersonation (T1134), password spray (T1110.003), Kerberoasting (T1558.003), DCSync (T1003.006), LDAP query

## Verification Commands
cargo check -p eggsec --features postex
cargo test --lib -p eggsec --features postex
cargo clippy --lib -p eggsec --features postex
