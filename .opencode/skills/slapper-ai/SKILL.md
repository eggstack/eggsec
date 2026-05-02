# Slapper AI Skill

AI/LLM integration module workflows and patterns for autonomous security testing.

## Key Types and Patterns

### Circuit Breaker
`utils/circuit_breaker.rs` provides `CircuitBreaker`:
- Individual breaker with state (Closed/Open/HalfOpen)
- Tracks failure/success counts, total calls, failure rate
- Exposes `total_calls()`, `total_failures()`, `failure_rate()` methods

Each AI client creates its own breaker directly via `CircuitBreaker::new()`.

## Testing

### Running AI Tests
```bash
cargo test --lib -p slapper ai::
```

### Writing Tests
Follow existing test patterns in `ai/` modules, testing circuit breaker integration and LLM client logic.

## Common Tasks

### Adding a New AI Client
1. Implement client logic in `ai/`
2. Create dedicated `CircuitBreaker` via `CircuitBreaker::new()`
3. Track failure/success counts for circuit breaker state
4. Add tests for new AI client

## Resources
- `crates/slapper/src/ai/AGENTS.override.md` - Detailed AI patterns
- `AGENTS.md` - General project guidelines
- `ARCHITECTURE.md` - Overall design
