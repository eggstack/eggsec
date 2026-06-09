# Eggsec Hunt Skill

Intelligent vulnerability hunting module.

## Module Location
`crates/eggsec/src/hunt/`

## Tab
Hunt tab is one of the 29 TUI tabs - see `eggsec-tui/SKILL.md` for TUI patterns.

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

## Module Notes
See `architecture/hunt.md` for architecture documentation.