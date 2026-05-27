# Slapper Hunt Skill

Intelligent vulnerability hunting module.

## Module Location
`crates/slapper/src/hunt/`

## Tab
Hunt tab is one of the 29 TUI tabs - see `slapper-tui/SKILL.md` for TUI patterns.

## Key Types

- `HuntEngine` - Main vulnerability hunting engine
- `VulnPattern` - Vulnerability pattern definitions
- `Severity` - Impact severity levels

## Patterns

### Running a Hunt
```rust
let engine = HuntEngine::new();
engine.set_target_url("https://example.com");
engine.add_patterns(vec!["sql_injection", "xss", "csrf"]);
let results = engine.run().await?;
```

### Focus Areas
- `HuntFocusArea::Inputs` - Target input fields
- `HuntFocusArea::Options` - Hunt configuration options

## Key Files
- `mod.rs` - Main engine implementation
- `patterns.rs` - Vulnerability patterns

## AGENTS.md Override
See `crates/slapper/src/hunt/AGENTS.override.md`