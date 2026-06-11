# Eggsec Proxy Skill

Intercepting proxy module workflows and patterns for traffic inspection.

## Key Types and Patterns

### Intercepting Proxy
`proxy/intercept/` - Intercepting proxy with dynamic SSL certificates.

### Safe Logging
`proxy` module uses `to_log_key()` for safe logging of sensitive data.

## Testing

### Running Proxy Tests
```bash
cargo test --lib -p eggsec proxy::
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

## Bug Fixes (2026-05-30)

- **health.rs:158-170**: Changed `filter_map(|r| r.ok())` to explicit `match` with `is_panic()` detection and `tracing::warn!` for JoinErrors. Previously, panics in health check tasks were silently dropped.

## Resources
- `crates/eggsec/src/proxy/AGENTS.override.md` - Detailed proxy patterns
- `AGENTS.md` - General project guidelines
- `architecture/overview.md` - Overall design
