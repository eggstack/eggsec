---
name: eggsec-hunt
description: "Intelligent vulnerability hunting - use when working with business logic flaws, authorization bypass, attack chain detection, race conditions, or session issue detection."
---

# Eggsec Hunt Skill

Intelligent vulnerability hunting module.

## Module Location
`crates/eggsec/src/hunt/`

## Tab
Hunt tab is one of the 33 TUI tabs - see `eggsec-tui/SKILL.md` for TUI patterns.

## Key Types

- `HuntClient` - Main vulnerability hunting client
- `HuntReport` - Hunt results and findings
- `HuntConfig` - Hunt configuration (target, options)

## Patterns

### Running a Hunt
```rust
let config = HuntConfig { check_session: true, ..Default::default() };
let report = run_hunt("https://example.com", config).await?;
```

## Key Files
- `mod.rs` - Main client (`HuntClient`, `HuntReport`, `HuntConfig`)
- `business.rs` - Business logic flaw detection (`BusinessLogicFlaw`, `FlawType`)
- `authz.rs` - Authorization bypass detection (`AuthzBypass`, `BypassType`)
- `chain.rs` - Attack chain detection (`AttackChain`, `ChainType`, `ChainStep`)
- `race.rs` - Race condition detection (`RaceCondition`, `RaceType`)
- `session.rs` - Session issue detection (`SessionIssue`, `SessionIssueType`)

## Module Notes
See `architecture/hunt.md` for architecture documentation.