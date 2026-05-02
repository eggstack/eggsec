# Slapper WAF Skill

WAF detection and bypass module workflows and patterns.

## Key Types and Patterns

### Constants
`constants::waf` module has scoring and detection constants. Use these instead of magic numbers in WAF-related code.

## Testing

### Running WAF Tests
```bash
cargo test --lib -p slapper waf::
```

### Writing Tests
Follow existing test patterns in `waf/` modules, testing detection logic and bypass techniques.

## Common Tasks

### Adding a New WAF Detection Rule
1. Add scoring/detection constants to `constants::waf`
2. Implement detection logic in `waf/` modules
3. Avoid magic numbers by using defined constants
4. Add tests for new detection rule

## Resources
- `crates/slapper/src/waf/AGENTS.override.md` - Detailed WAF patterns
- `AGENTS.md` - General project guidelines
- `ARCHITECTURE.md` - Overall design
