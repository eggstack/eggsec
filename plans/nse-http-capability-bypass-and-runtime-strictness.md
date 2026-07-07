# NSE HTTP Capability Bypass and Runtime Strictness Plan

## Purpose

Close the next narrow NSE correctness gap after Milestone 5: HTTP helper operations still have an actual network-execution mismatch between the policy model and the underlying `reqwest` path, and runtime library assertions remain slightly lenient.

Milestone 5 materially improved runtime corpus stability, local protocol coverage, `unpwdb` filesystem wrapping, report formatting, and bridge/evidence tests. The next pass should avoid broad expansion and instead make the existing policy/report story tighter.

## Current State

Confirmed state after Milestone 5:

- Runtime corpus tests execute real fixtures through `NseExecutor::with_profile()` and collect observed reports.
- Runtime corpus isolation improved with per-invocation temp directories using an atomic counter.
- Manifest parsing is cached with `LazyLock`.
- Local TCP/HTTP/UDP tests run against localhost services.
- `http.rs` now receives `NseCapabilityContext` and checks `wrappers::check_network_tcp()` before many HTTP operations.
- `unpwdb.rs` now routes wordlist reads through `wrappers::nse_fs_read_to_string()`.
- Human report formatting moved into a testable `format_human_report()` function.
- Closure docs record the remaining HTTP limitation.

Remaining gaps:

1. HTTP uses `reqwest::blocking::Client` and async `reqwest::Client` for the real network operation. The preflight `check_network_tcp()` records a policy decision, but the actual network send still occurs through `reqwest`, not through the NSE network wrappers. This means the helper is only partially wrapped.
2. Local protocol tests currently document that HTTP under AgentSafe may either succeed or fail because the `reqwest` path bypasses the capability context. That is too loose for eventual safety closure.
3. Runtime library assertions still allow expected library misses when `report.libraries` is empty. This should be narrowed to explicit manifest metadata, not a broad fallback.
4. Compatibility docs should clearly distinguish advisory/preflight HTTP policy checks from fully wrapper-owned socket operations.

## Non-Goals

Do not rewrite the entire HTTP library.

Do not implement full Nmap HTTP library parity.

Do not require public internet.

Do not remove manual HTTP functionality.

Do not reopen loader/profile semantics, library-report truthfulness, or the capability-context model.

Do not broaden this pass into SMB/SSH/database protocol migration.

## Workstream 1: Make HTTP Policy Enforcement Blocking Before Reqwest

### Problem

`http.rs` performs `wrappers::check_network_tcp()` before `reqwest` calls, but local tests still treat AgentSafe behavior as ambiguous. If `check_network_tcp()` denies access, HTTP must return a denied response and must not call `reqwest` at all.

### Required Outcome

For every HTTP function that can perform network I/O, denial by `NseCapabilityContext` must prevent the underlying `reqwest` call.

### Steps

1. Audit every HTTP function in `crates/eggsec-nse/src/libraries/http.rs`:
   - `get`
   - `post`
   - `put`
   - `delete`
   - `head`
   - `options`
   - `request`
   - `post_host`
   - `put_data`
   - async variants such as `async_get`, `async_post`, `async_request`
   - any helper that constructs a URL and calls `.send()` or `.await` on a request.
2. Ensure the capability decision is checked before the `Client` is selected and before the URL is sent.
3. Ensure denied decisions return `denied_response(lua, reason)` or an equivalent structured Lua table and do not invoke the underlying client.
4. Add one small internal helper to reduce drift:

```rust
fn http_policy_denied(
    lua: &Lua,
    ctx: &NseCapabilityContext,
    host: &str,
    operation: &'static str,
) -> LuaResult<Option<Table>>
```

or use an equivalent pattern.
5. Avoid duplicating denial logic across every closure if possible.

### Acceptance Criteria

- AgentSafe/DenyAll HTTP GET against a local server records a denial and does not hit the server.
- CiSafe/DenyAll HTTP GET records a denial and does not hit the server.
- ManualPermissive HTTP GET/POST still succeeds against local server.

## Workstream 2: Add Server Hit Counters to Local HTTP Fixtures

### Problem

A denied HTTP helper might still hit the local server if policy checks are advisory only. Current tests cannot prove the server was not contacted.

### Required Outcome

Local HTTP fixture server should expose hit counts per method/path so tests can assert denied requests produce zero server hits.

### Steps

1. Extend `tests/local_fixtures.rs` HTTP server with atomic counters:
   - total request count;
   - method counts if simple;
   - optional last path/method capture.
2. Expose methods such as:

```rust
server.request_count()
server.method_count("GET")
server.last_path()
```

3. Ensure counters are safe under test parallelism.
4. Update HTTP local tests to assert:
   - ManualPermissive GET increments server hit count.
   - AgentSafe/DenyAll GET does not increment server hit count.
   - CiSafe/DenyAll GET does not increment server hit count.

### Acceptance Criteria

- Tests fail if denied HTTP paths still call `reqwest` and reach the local server.

## Workstream 3: Make HTTP AgentSafe Tests Hard Assertions

### Problem

`local_protocol_tests.rs` currently accepts either HTTP success or failure under AgentSafe and documents a bypass. That is appropriate for documenting Milestone 5 state, but the next pass should make it a hard policy assertion.

### Required Outcome

AgentSafe and CiSafe HTTP tests must assert denial, no server hit, report `Partial` or equivalent, and capability denial evidence.

### Steps

1. Replace the permissive AgentSafe HTTP test branch with strict assertions:
   - at least one `network_tcp` denial event;
   - output contains denied/failure marker and not normal successful response;
   - server hit count is zero;
   - compatibility status is `Partial` or `Unsupported` according to existing report semantics;
   - evidence contains `CapabilityDenial` when denied events are observed.
2. Add matching CiSafe HTTP denial test if not already present.
3. Keep ManualPermissive success tests unchanged.

### Acceptance Criteria

- AgentSafe/CiSafe HTTP tests fail if the request reaches the local server.
- ManualPermissive HTTP tests continue to pass.

## Workstream 4: Clarify HTTP Wrapper Status in Registry and Docs

### Problem

HTTP may currently be described as partially wrapped/advisory. Once denied requests are proven not to execute, the status can improve, but it should not be overstated as full HTTP parity.

### Steps

1. Update `resolver/registry.rs` notes for `http`:
   - distinguish policy enforcement status from protocol parity;
   - state that reqwest calls are preflight-gated by capability context;
   - retain known HTTP/2/cookie jar/redirect limitations if applicable.
2. Update `docs/NSE_COMPATIBILITY.md`:
   - `http` status should be `Wrapped` or `PartiallyWrapped` based on actual outcome;
   - if all network-sending HTTP functions are hard-gated, mark wrapper status stronger but keep protocol fidelity partial.
3. Update `architecture/nse_integration.md` Milestone 5/future-work sections to remove or narrow the HTTP bypass caveat after fixed.
4. Update `.opencode/skills/eggsec-nse/SKILL.md` guidance: new HTTP helper functions must gate before send.

### Acceptance Criteria

- Docs no longer say AgentSafe HTTP may succeed because of reqwest bypass once tests prove denial.
- Docs still avoid claiming full Nmap HTTP parity.

## Workstream 5: Tighten Runtime Library Assertions

### Problem

`runtime_corpus_tests.rs` still allows an expected library miss if `report.libraries` is empty. That can hide missing runtime require tracking.

### Required Outcome

Missing expected libraries should fail unless a fixture explicitly opts into soft behavior.

### Steps

1. Replace broad condition:

```rust
assert!(report.libraries.is_empty() || found, ...)
```

with hard failure for required expected libraries.
2. Use explicit harness metadata for soft cases:
   - `allow_missing_runtime_libraries = true`; or
   - `allow_static_require_fallback = true`; or
   - new field `expected_libraries_optional = [...]` if cleaner.
3. For fixtures expected to short-circuit before `require()`, mark them explicitly in manifest.
4. Add a regression test or guard that a fixture requiring `stdnse` fails if runtime libraries are empty.

### Acceptance Criteria

- Required expected libraries are hard assertions.
- Optional missing-library behavior is manifest-explicit and visible.
- All-registry-loaded behavior remains impossible.

## Workstream 6: Architecture Guards

Add or tighten checks for:

1. HTTP functions that call `.send()` must have a nearby `check_network_tcp()` or centralized helper call before the send.
2. `local_protocol_tests.rs` must not contain permissive text such as “may fail or succeed” for AgentSafe HTTP once the bypass is fixed.
3. Runtime corpus library assertion must not contain `report.libraries.is_empty() || found`.
4. Public-network local protocol fixtures remain forbidden.

### Acceptance Criteria

- Guard fails if a new HTTP network operation bypasses preflight gating.
- Guard fails if runtime library strictness regresses.

## Workstream 7: Verification Record

Record results in `architecture/nse_integration.md` or a dedicated closure note.

Required commands:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse --test local_protocol_tests
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=1
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=4
cargo test -p eggsec-nse --features nse --test runtime_smoke_tests
cargo test -p eggsec-nse --features nse --test format_tests
cargo test -p eggsec-nse --features nse
bash scripts/check-architecture-guards.sh
cargo fmt --all --check
cargo clippy --lib -p eggsec-nse --features nse
```

### Acceptance Criteria

- Verification records whether HTTP denied paths produce zero server hits.
- Verification records whether library assertions are now strict.

## Final Acceptance Criteria

This next pass is complete when:

- AgentSafe/CiSafe HTTP requests are denied before any `reqwest` network call.
- Local HTTP server hit counters prove denied requests do not reach the server.
- ManualPermissive HTTP GET/POST still succeeds against local fixtures.
- HTTP docs/registry accurately distinguish wrapper enforcement from protocol fidelity.
- Runtime expected-library assertions are hard unless the manifest opts into soft behavior.
- Architecture guards protect HTTP gating and runtime library strictness.
- Verification is recorded.

## Handoff Notes

Keep this pass narrow. Do not migrate unrelated protocols while closing the HTTP bypass. The correct outcome is boring: denied automated HTTP calls never hit the local server; manual calls still work; docs stop carrying the broad reqwest-bypass caveat; runtime library assertions become less permissive.