# NSE HTTP Method Coverage and Guard Hardening Plan

## Purpose

Close the remaining NSE hardening items after the HTTP capability-bypass pass.

The previous pass proved the main semantic boundary for HTTP GET: ManualPermissive HTTP reaches the local server, while AgentSafe/CiSafe HTTP GET produces `network_tcp` denial events and does not hit the server. It also tightened runtime expected-library assertions and added guards for broad HTTP gating regressions.

The remaining work is to extend that proof across all HTTP network methods and replace coarse/count-based guards with method/path-specific guardrails.

## Current State

Confirmed current state:

- `HttpServer` in `tests/local_fixtures.rs` tracks accepted connections through an atomic `hits()` counter.
- ManualPermissive HTTP GET/POST tests assert the local server receives hits.
- AgentSafe/CiSafe HTTP GET tests assert `network_tcp` denial events and zero local-server hits.
- Runtime corpus expected-library assertions now hard-fail when a required expected library is not observed unless the fixture opts into soft behavior.
- Architecture guards reject lenient AgentSafe HTTP test language and the old `report.libraries.is_empty() || found` library assertion pattern.
- `http.rs` has broad `check_network_tcp()` coverage before request construction/send paths.

Remaining gaps:

1. Zero-hit denial proof currently covers HTTP GET only. POST has ManualPermissive success coverage, but AgentSafe/CiSafe denial should also be asserted.
2. PUT, DELETE, HEAD, OPTIONS, and generic `request` do not yet have local zero-hit denial tests.
3. Async HTTP helper methods are harder to test but should at least have structural guard coverage, or targeted tests if practical.
4. The guard for HTTP gating is count-based (`check_network_tcp` call count) rather than path-specific and dominance-aware.
5. Local HTTP fixture server tracks total hits but not method/path counts, limiting test precision.
6. Verification should be recorded after these method-specific hardening checks.

## Non-Goals

Do not rewrite the HTTP library into a custom HTTP client.

Do not implement full Nmap HTTP library parity.

Do not add public-internet-dependent tests.

Do not reopen loader/profile semantics, library-report truthfulness, or the capability-context model.

Do not migrate unrelated protocols in this pass.

## Workstream 1: Extend Local HTTP Server Counters

### Problem

The current local HTTP server exposes only total accepted-connection count. Method/path-aware counters would make tests more precise and improve debugging.

### Required Outcome

`HttpServer` should be able to report total hits and basic method/path information.

### Implementation Steps

1. Extend `HttpServer` with thread-safe tracking for:
   - total hits;
   - last method;
   - last path;
   - optional per-method counters if simple.
2. Keep the implementation low-risk:
   - use `Arc<Mutex<Option<String>>>` for last method/path, or compact atomic counters for methods;
   - avoid complex shared state;
   - ensure no test hangs on lock poisoning.
3. Add helper methods:

```rust
server.hits()
server.last_method()
server.last_path()
server.method_hits("GET")
```

Use only the helpers that are simple to implement cleanly.

### Acceptance Criteria

- ManualPermissive GET asserts `hits() > 0` and `last_method() == Some("GET")` if implemented.
- ManualPermissive POST asserts `hits() > 0` and `last_method() == Some("POST")` if implemented.
- Denied AgentSafe/CiSafe methods assert `hits() == 0`.

## Workstream 2: Add Local Fixture Scripts for Missing HTTP Methods

### Required Scripts

Add local-only fixture scripts under:

```text
crates/eggsec-nse/tests/fixtures/nse_corpus/scripts/protocol/
```

Suggested files:

- `http_put_local.nse`
- `http_delete_local.nse`
- `http_head_local.nse`
- `http_options_local.nse`
- `http_request_local.nse`

Each script should:

- use the existing `http` NSE library;
- call the corresponding method against `host.ip` / `port.number`;
- return a simple deterministic output string;
- avoid public internet;
- be small enough to debug quickly.

### Acceptance Criteria

- Fixture scripts compile and execute through `NseExecutor::with_profile()`.
- Fixtures are listed in `manifest.toml` with `[local_service]` metadata.
- Runtime corpus skips them and `local_protocol_tests.rs` owns real listener execution.

## Workstream 3: Add ManualPermissive Success Tests for All HTTP Methods

### Required Tests

In `local_protocol_tests.rs`, add ManualPermissive tests for:

- GET already exists; keep it.
- POST already exists; keep it.
- PUT.
- DELETE.
- HEAD.
- OPTIONS.
- Generic `request`.

For each test:

1. Start local `HttpServer`.
2. Run the matching fixture under ManualPermissive.
3. Assert compatibility is `Compatible` or `CompatibleWithWarnings`.
4. Assert output includes the expected status/body marker.
5. Assert server hit count increments.
6. Assert method/path where server tracking supports it.

### Acceptance Criteria

- ManualPermissive still has usable HTTP behavior across common methods.
- Tests prove real local server contact for permitted manual mode.

## Workstream 4: Add AgentSafe/CiSafe Zero-Hit Denial Tests for All HTTP Methods

### Required Tests

For each method above, add AgentSafe and/or CiSafe denial tests. At minimum:

- AgentSafe POST zero-hit.
- CiSafe POST zero-hit.
- AgentSafe generic request zero-hit.
- AgentSafe HEAD/OPTIONS/DELETE/PUT zero-hit if scripts are added.

For each denied test:

1. Start local `HttpServer`.
2. Run method fixture under AgentSafe or CiSafe runtime profile.
3. Assert at least one `network_tcp` denial event.
4. Assert server hit count is exactly zero.
5. Assert output does not contain the normal success marker.
6. Assert evidence includes `CapabilityDenial` if the report path extracts it.

### Acceptance Criteria

- Every covered HTTP network method has at least one automated-profile zero-hit denial test.
- Denied HTTP methods do not reach the server.
- ManualPermissive success remains intact.

## Workstream 5: Consolidate HTTP Policy Check Logic

### Problem

Repeated preflight logic across closures can drift. A future method could call `.send()` without a policy check.

### Required Outcome

HTTP policy denial should be centralized enough to make audits and guards reliable.

### Implementation Options

Option A: Add a helper returning `Option<Table>`:

```rust
fn maybe_denied_response(
    lua: &Lua,
    ctx: &NseCapabilityContext,
    host: &str,
    operation: &'static str,
) -> LuaResult<Option<Table>>
```

Usage:

```rust
if let Some(resp) = maybe_denied_response(lua, &ctx, &host, "http.get")? {
    return Ok(resp);
}
```

Option B: Add a helper returning `LuaResult<()>` and leave response construction local.

Option C: Keep repeated checks but add stronger structural guards. Use this only if helper extraction introduces too much churn.

### Recommended Choice

Use Option A unless borrow/lifetime constraints make it awkward.

### Acceptance Criteria

- All sync HTTP methods use the same denial helper or an equivalent visibly identical pattern.
- Async methods are either covered by the same helper or documented with tests/guards.

## Workstream 6: Replace Count-Based Guard With Path-Specific Guarding

### Problem

The current guard only verifies that `http.rs` contains enough `check_network_tcp()` calls. It does not prove each `.send()` path has a preceding policy gate.

### Required Guard Improvements

Add shell guard checks that catch obvious bypasses:

1. Fail if `http.rs` contains `client.get(`, `client.post(`, `client.put(`, `client.delete(`, `client.head(`, `client.request(`, `req.send()`, or async send calls in a closure that does not also contain `check_network_tcp` or `maybe_denied_response` nearby.
2. If robust closure-level parsing is too hard in shell, require each operation string to appear:
   - `http.get`
   - `http.post`
   - `http.put`
   - `http.delete`
   - `http.head`
   - `http.options`
   - `http.request`
   - async operation names if present.
3. Fail if local protocol tests contain permissive language such as:
   - `may fail or succeed`;
   - `accept either outcome`;
   - `should complete without crash` for denied automated HTTP tests unless paired with zero-hit and denial assertions.
4. Fail if local HTTP denied tests do not assert `server.hits() == 0`.

### Acceptance Criteria

- A new HTTP method that sends without preflight is likely caught by guards.
- Tests cannot revert to permissive AgentSafe/CiSafe HTTP assertions unnoticed.

## Workstream 7: Tighten Documentation and Registry Status

### Required Updates

Update:

- `docs/NSE_COMPATIBILITY.md`
- `architecture/nse_integration.md`
- `crates/eggsec-nse/src/resolver/registry.rs`
- `.opencode/skills/eggsec-nse/SKILL.md`
- `AGENTS.md`
- `crates/eggsec-nse/AGENTS.override.md`

Required wording:

- HTTP network operations are preflight-gated by `NseCapabilityContext`.
- Automated-profile denials must occur before request send and are tested with zero-hit local fixtures.
- HTTP protocol fidelity remains partial if cookie jar, redirects, HTTP/2, TLS, or NSE-specific edge behavior is incomplete.
- New HTTP methods must add both ManualPermissive success and automated-profile zero-hit denial tests.

### Acceptance Criteria

- Docs distinguish capability enforcement from protocol parity.
- Registry status is accurate: do not mark full fidelity if only enforcement is complete.

## Workstream 8: Verification Record

Record verification in `architecture/nse_integration.md` or an adjacent closure note.

Run:

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

- Verification records exact HTTP method coverage.
- Verification records guard status.
- Any remaining unsupported async or TLS-specific HTTP behavior is explicitly listed.

## Final Acceptance Criteria

This hardening pass is complete when:

- Local HTTP server exposes precise enough hit/method/path tracking.
- GET/POST/PUT/DELETE/HEAD/OPTIONS/generic request have ManualPermissive success tests where implemented.
- Automated profiles have zero-hit denial tests for every covered HTTP method.
- HTTP network send paths have centralized or structurally guarded preflight checks.
- Architecture guards are method/path-specific enough to catch obvious future bypasses.
- Runtime expected-library assertions remain strict.
- Documentation accurately states capability enforcement status versus protocol fidelity.
- Verification is recorded.

## Handoff Notes

Keep this pass focused on HTTP method coverage and guard hardening. Do not expand into SSH/SMB/database libraries until the HTTP enforcement proof is comprehensive. The desired result is strong, boring evidence: allowed manual HTTP calls hit the local server; denied automated HTTP calls never hit it.