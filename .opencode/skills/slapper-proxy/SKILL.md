# Slapper Proxy Skill

Intercepting proxy module workflows and patterns for traffic inspection.

## Key Types and Patterns

### Intercepting Proxy
`proxy/intercept/` - Intercepting proxy with dynamic SSL certificates.

### Safe Logging
`proxy` module uses `to_log_key()` for safe logging of sensitive data.

## Testing

### Running Proxy Tests
```bash
cargo test --lib -p slapper proxy::
```

### Writing Tests
Follow existing test patterns in `proxy/` modules, testing interception and safe logging.

## Common Tasks

### Adding a New Proxy Feature
1. Implement logic in `proxy/` modules
2. Use `to_log_key()` for logging sensitive data
3. Add tests for new proxy feature

### Adding Dynamic SSL Certificate Support
1. Update `proxy/intercept/` with certificate generation logic
2. Test certificate handling

## Resources
- `crates/slapper/src/proxy/AGENTS.override.md` - Detailed proxy patterns
- `AGENTS.md` - General project guidelines
- `ARCHITECTURE.md` - Overall design
