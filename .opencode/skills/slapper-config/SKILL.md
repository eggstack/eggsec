# Slapper Config Skill

Configuration module workflows and patterns for managing Slapper settings.

## Key Types and Patterns

### SlapperConfig
`config::load_config()` returns the main configuration.

### PathsConfig
Directory paths are flattened into `SlapperConfig`.

## Testing

### Running Config Tests
```bash
cargo test --lib -p slapper config::
```

### Writing Tests
Follow existing test patterns in `config/` modules, testing configuration loading and validation.

## Common Tasks

### Adding a New Configuration Option
1. Add field to `SlapperConfig` or relevant sub-config struct
2. Update `config::load_config()` to load new option
3. Flatten directory paths into `SlapperConfig` if applicable
4. Add tests for new configuration option

## Resources
- `crates/slapper/src/config/AGENTS.override.md` - Detailed config patterns
- `AGENTS.md` - General project guidelines
- `ARCHITECTURE.md` - Overall design
