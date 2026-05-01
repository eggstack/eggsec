# AI Module Override

Specialized guidance for the AI/LLM integration module.

## Circuit Breaker

`utils/circuit_breaker.rs` provides `CircuitBreaker`:
- Individual breaker with state (Closed/Open/HalfOpen)
- Tracks failure/success counts, total calls, failure rate
- Exposes `total_calls()`, `total_failures()`, `failure_rate()` methods

Each AI client creates its own breaker directly via `CircuitBreaker::new()`.