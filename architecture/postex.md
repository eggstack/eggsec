# Post-Exploitation Module

Standalone defense-lab module for simulating post-exploitation techniques for purple teaming and defense validation.

## Architecture

```
postex/
  mod.rs          - Types, PostexScanner, to_scan_report_data bridge, run_cli entry
  lotl.rs         - Living-Off-The-Land command wrappers
  persistence.rs  - Persistence mechanism simulation
  lateral.rs      - Lateral movement simulation
  credential.rs   - Credential access simulation
  report.rs       - Human/JSON report formatting
```

## Feature Gating

- `postex` - Marker feature, no dependencies
- Included in `full` feature set

## Policy Model

- Dry-run: `OperationRisk::SafeActive` (no confirmation needed)
- Real: `OperationRisk::PostExploitation` (high-risk, requires `--allow-postex`)
- Mode: `OperationMode::DefenseLab`

## Technique Categories

| Category | Techniques | MITRE ATT&CK |
|----------|-----------|---------------|
| LOTL | PowerShell, WMIC, certutil, rundll32, msiexec, mshta, regsvr32, bash, curl, wget | T1059.001, T1047, T1105, T1218.* |
| Persistence | Registry run key, scheduled task, service creation, DLL hijack, startup folder, WMI subscription | T1547.001, T1053.005, T1543.003, T1574.002 |
| Lateral Movement | SMB, RDP, port forwarding, SOCKS proxy, WinRM, PsExec | T1021.*, T1090.* |
| Credential Access | LSASS dump, token impersonation, password spray, Kerberoasting, DCSync, LDAP query | T1003.*, T1134, T1110.003, T1558.003 |

## Dry-Run Contract

- Zero side effects (no real technique execution)
- Complete report with synthetic detections for all techniques in profile
- Confidence scores based on risk level (Critical=0.85, High=0.75, Medium=0.65, Low=0.55)
- Cleanup commands generated for reversible techniques

## Profiles

- **Minimal**: Medium and Low risk techniques only (8 techniques)
- **Standard**: All 16 techniques (default)
- **Aggressive**: All 16 techniques (same as Standard, future expansion point)

## CLI

```
eggsec postex --target <ip> --dry-run --json
eggsec postex --target <ip> --profile minimal --dry-run
eggsec postex --category lotl --dry-run --json
```

## Reporting

- `to_scan_report_data()` bridges `PostexReport` → `ScanReportData` with `postex-*` categories
- Auto-bridged in `report convert` handler
- SARIF, HTML, JSON, CSV, JUnit, Markdown output supported
