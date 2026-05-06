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

### SARIF Error Handling
`convert_to_sarif()` returns `Result<String, String>` - properly propagate errors:

```rust
// Correct
ReportFormat::Sarif => convert::convert_to_sarif(&report).map_err(|e| anyhow::anyhow!(e))?

// Or with unwrap_or_else for fallback
OutputFormat::Sarif => convert::convert_to_sarif(&report).unwrap_or_else(|e| format!("Error: {}", e))
```

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
