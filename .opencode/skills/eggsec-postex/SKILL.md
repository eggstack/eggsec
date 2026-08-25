---
name: eggsec-postex
description: "Post-exploitation simulation for defense validation - use when working with LOTL technique detection, persistence/lateral-movement/credential-access simulations, or purple-team detection validation."
---

# Eggsec Postex Skill

Post-exploitation and living-off-the-land (LOTL) simulation for defense validation (purple teaming). Dry-run safe; simulation only.

## Module Location

`crates/eggsec/src/postex/`

## Feature Gate

`postex` - marker-only, no dependencies. `c2` depends on this feature.

## Key Types

- `PostexCategory` - `Lotl`, `Persistence`, `LateralMovement`, `CredentialAccess`
- `PostexRisk` - Risk classification
- `PostexTechnique` - Technique definition (`id` + MITRE ATT&CK ID, 16 techniques across the 4 categories)
- `PostexDetection` - Detection outcome for a technique
- `PostexReport` / `PostexSummary` - Aggregated results
- `PostexProfile` - Simulation preset
- `PostexScanner::scan(target)` - Main entry point returning `Result<PostexReport>`

## CLI Integration

- Command: `eggsec postex <args>` (feature-gated: `postex`)
- Handler: `commands/handlers/postex.rs`
- Entry: `run_cli()` in `postex/mod.rs`

## Safety Model

- Always dry-run first; real execution is simulation-oriented but still policy-gated via the central `EnforcementContext`
- Defense-lab only; never against production systems

## Testing

```bash
cargo check -p eggsec --features postex
cargo test --lib -p eggsec --features postex postex::
```

## Resources

- `crates/eggsec/src/postex/AGENTS.override.md` - Module guidance
- `architecture/postex.md` - Architecture deep dive
