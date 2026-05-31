# Slapper Browser Skill

Headless browser security testing module.

## Module Location
`crates/slapper/src/browser/`

## Tab
Browser tab is one of the 29 TUI tabs - see `slapper-tui/SKILL.md` for TUI patterns.

## Key Types

- `BrowserEngine` - Headless browser automation
- `BrowserConfig` - Browser configuration
- `SecurityTest` - Security test definitions

## Patterns

### Browser Scan
```rust
let engine = BrowserEngine::new(config);
engine.navigate("https://example.com");
engine.inject_payloads(vec!["xss", "dom_xss"]);
let findings = engine.analyze().await?;
```

### Focus Areas
- `BrowserFocusArea::Inputs` - Form input fields
- `BrowserFocusArea::Options` - Scan configuration

## Key Files
- `mod.rs` - Main browser engine

## Module Notes
See `architecture/browser.md` for architecture documentation.