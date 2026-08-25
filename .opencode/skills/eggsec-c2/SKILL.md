---
name: eggsec-c2
description: "C2 (Command & Control) simulation for defense-lab purple teaming - use when working with campaign orchestration, beacon simulation, tasking, or OPSEC assessment."
---

# Eggsec C2 Skill

C2 framework simulation for defense-lab purple teaming and red-team rehearsal. Heavily gated: policy + `--allow-c2` + dry-run support.

## Module Location

`crates/eggsec/src/c2/`

## Feature Gate

`c2 = ["postex", "evasion"]` - depends on both. Standalone defense-lab surface; MCP exposure additionally requires the `c2-mcp` marker.

## Key Types (`mod.rs`)

- `C2Scanner` - Main scanner: `new()`, `scan()`, `campaign()`, `to_scan_report_data()`
- `C2Report` / `C2Summary` - Aggregated results
- `C2Campaign` / `CampaignPhase` - Multi-phase campaign orchestration
- `BeaconResult` / `BeaconProtocol` - Simulated beacon outcomes and protocols (`beacon.rs`)
- `TaskResult` / `TaskType` / `TaskStatus` - Simulated tasking model (`tasking.rs`)
- `OpsecAssessment` / `OpsecFinding` / `OpsecCategory` / `OpsecSeverity` - OPSEC evaluation (`opsec.rs`)
- `mitre_technique_ids()` - ATT&CK mapping helper

## Supporting Files

| File | Purpose |
|------|---------|
| `agent.rs` | Simulated implant/agent behavior |
| `campaign.rs` | Campaign phase orchestration |
| `beacon.rs` | Beacon protocol simulation |
| `tasking.rs` | Task dispatch simulation |
| `opsec.rs` | OPSEC assessment |

## CLI Integration

- Command: `eggsec c2 <args>` (feature-gated: `c2`)
- Handler: `commands/handlers/c2.rs`
- Entry: `run_cli()` in `c2/mod.rs`

## Safety Model

- Dry-run is always safe
- Real runs require explicit `--allow-c2` plus policy confirmation via the central `EnforcementContext`
- Lab-only: never point at systems you do not own

## Testing

```bash
cargo check -p eggsec --features c2
cargo test --lib -p eggsec --features c2 c2::
```

## Resources

- `crates/eggsec/src/c2/AGENTS.override.md` - Module guidance
- `architecture/c2.md` - Architecture deep dive
