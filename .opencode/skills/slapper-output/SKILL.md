# Slapper Output Skill

Report generation module workflows and patterns for exporting scan results.

## Key Types and Patterns

### Report Formats
`output/` supports multiple output formats:
- JSON
- HTML
- SARIF
- JUnit

### Severity Re-export
`output/agent::Severity` and `output::trend::Severity` re-export from `crate::types::Severity`.

## Testing

### Running Output Tests
```bash
cargo test --lib -p slapper output::
```

### Writing Tests
Follow existing test patterns in `output/` modules, testing report generation for all supported formats.

## Common Tasks

### Adding a New Report Format
1. Implement format generation in `output/`
2. Re-export `Severity` from `crate::types::Severity` if needed
3. Add tests for new report format

## Resources
- `crates/slapper/src/output/AGENTS.override.md` - Detailed output patterns
- `AGENTS.md` - General project guidelines
- `ARCHITECTURE.md` - Overall design
