# Phase B Gap Closure — Performance Benchmarks, Architecture Guard, Budget Verification

> **Status: Executed** — 2026-07-16

## Objective

Close the three remaining gaps from Phase B:
1. Add Phase B-specific performance benchmarks for registry and dispatch operations
2. Add architecture guard Check 66 for legacy dispatch pattern removal
3. Establish and verify dispatch performance budgets against pre-refactor baseline

## Context

Phase B completed all 9 workstreams (B1–B9) with passing tests, but deferred:
- **Performance benchmarks**: Plan required benchmarks for registry construction, descriptor lookup, operation listing, request normalization, no-op/denied dispatch, and sync/async dispatch overhead. Existing performance tests (Release 2) cover general dispatch overhead but not Phase B registry-specific operations.
- **Architecture guard**: Plan item B9 required a CI check for "no legacy twenty-two-arm dispatch remains." Check 64 (operation count) and Check 65 (daemon mapping) were added; Check 66 was not.
- **Budget verification**: Acceptance criteria requires dispatch performance within budget. No Phase B-specific budgets exist in `performance_budgets.json`.

## Workstream 1 — Phase B Performance Benchmarks

Add a new test file `crates/eggsec-python/tests/test_phase_b_performance.py` with benchmarks for Phase B registry and dispatch operations.

### Benchmarks to add

| Benchmark | What it measures | Budget | Iterations |
|-----------|-----------------|--------|------------|
| `registry_construction` | `OperationExecutorDescriptor::all_descriptors()` | 5ms | 1000 |
| `descriptor_lookup` | `OperationExecutorDescriptor::from_operation()` per operation | 1ms each | 1000 per op |
| `operation_listing` | `list_operations()` / registry iteration | 5ms | 1000 |
| `request_normalization` | `pre_dispatch_lifecycle()` for a minimal OperationRequest | 2ms | 500 |
| `no_op_denied_dispatch` | Full dispatch for out-of-scope target (scope denial path) | 10ms | 200 |
| `dispatch_overhead_sync` | `Engine.dispatch()` for scope-denied operation (measures overhead, not I/O) | 15ms | 200 |
| `dispatch_overhead_async` | `AsyncEngine.dispatch_async()` for scope-denied operation | 15ms | 100 |

### Implementation details

- Reuse the `_bench()` helper pattern from `test_performance_gates.py`
- Use a minimal `Scope` that denies the test target to avoid real network I/O
- Assert each benchmark is within 3x budget (consistent with existing test conventions)
- Print timing info on PASS, fail only when threshold exceeded
- Skip gracefully if API surface changes break imports

### Files to create/modify

- **Create**: `crates/eggsec-python/tests/test_phase_b_performance.py`
- **Modify**: `crates/eggsec-python/tests/performance_budgets.json` — add Phase B budget entries

## Workstream 2 — Architecture Guard Check 66

Add Check 66 to `scripts/check-architecture-guards.sh` to verify no legacy twenty-two-arm dispatch remains in the old pattern.

### Guard logic

Check that `engine.rs` and `async_engine.rs` do NOT contain the old hardcoded dispatch pattern. The old pattern was characterized by:
- Direct string matching on operation IDs (e.g., `"scan_ports"`, `"scan-ports"`) in the daemon task mapping
- Inline request construction without using `NormalizedRequest` or `OperationRequest`

Since the daemon mapping was already checked in Check 65, Check 66 should verify:
- `engine.rs` delegates to `dispatch_helpers::pre_dispatch_lifecycle` (not inline validation)
- `engine.rs` delegates to `execute_operation` (not inline per-op dispatch in the main dispatch method)
- The main `dispatch()` method in engine.rs does NOT contain per-operation match arms (it should call `pre_dispatch_lifecycle` → `execute_operation` → `post_dispatch_hooks`)

### Files to modify

- **Modify**: `scripts/check-architecture-guards.sh` — add Check 66 after Check 65

## Workstream 3 — Budget Verification

Run the existing performance tests plus the new Phase B benchmarks to establish baseline numbers and verify they pass.

### Steps

1. Run `pytest crates/eggsec-python/tests/test_performance_gates.py -v` to verify existing budgets pass
2. Run `pytest crates/eggsec-python/tests/test_phase_b_performance.py -v` to verify new Phase B benchmarks pass
3. Run `pytest crates/eggsec-python/tests/test_performance_report.py -v` to generate the performance report
4. Record baseline numbers in the plan for future regression detection

### Files to modify

- **Modify**: `crates/eggsec-python/tests/performance_budgets.json` — add Phase B entries if not already done in Workstream 1

## Acceptance Criteria

- [ ] `test_phase_b_performance.py` exists with 7 benchmarks covering all Phase B operations
- [ ] All benchmarks pass within 3x budget threshold
- [ ] `performance_budgets.json` includes Phase B budget entries
- [ ] Check 66 added to `check-architecture-guards.sh` and passes
- [ ] `bash scripts/check-architecture-guards.sh` exits clean (no new failures)
- [ ] All existing tests continue to pass

## Verification Commands

```bash
# Phase B benchmarks
pytest crates/eggsec-python/tests/test_phase_b_performance.py -v

# Existing performance gates
pytest crates/eggsec-python/tests/test_performance_gates.py -v

# Full test suite
pytest crates/eggsec-python/tests/ crates/eggsec-python/python/tests/ -x -q

# Architecture guards
bash scripts/check-architecture-guards.sh

# Rust core tests (unchanged)
cargo test -p eggsec --lib
```
