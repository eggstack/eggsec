# NSE HTTP Polish and Roadmap Closeout Plan

## Purpose

Close the remaining polish items after the NSE HTTP method-coverage and guard-hardening pass.

The current implementation is in strong shape: HTTP sync network methods are preflight-gated through `maybe_denied_response()`, local fixtures cover ManualPermissive success and automated-profile zero-hit denial, runtime corpus library assertions are strict, and architecture guards catch the main regression patterns. The remaining work is primarily documentation consistency, test symmetry, guard precision, and a final roadmap boundary.

## Current State

Confirmed current state:

- `http.rs` centralizes policy denial through `maybe_denied_response()`.
- Sync methods inspected are gated before sending: `get`, `post`, `put`, `delete`, `head`, `options`, `request`, `post_host`, and `put_data`.
- `HttpServer` tracks total hits plus last method/path.
- ManualPermissive local protocol tests cover GET, POST, PUT, DELETE, HEAD, OPTIONS, and generic request success.
- AgentSafe local protocol tests cover zero-hit denials for GET, POST, PUT, DELETE, HEAD, OPTIONS, and generic request.
- CiSafe local protocol tests cover zero-hit denials for GET and POST.
- The runtime corpus no longer accepts missing expected libraries unless the fixture explicitly opts into soft behavior.
- Guards check HTTP operation presence, strict server hit assertions, no permissive automated HTTP text, and no old lenient library assertion pattern.

Remaining polish items:

1. Documentation still has stale future-work wording implying the HTTP reqwest bypass remains unresolved.
2. Some architecture notes refer to `Milestone 6` inside a Milestone 5 section or otherwise mix milestone numbering.
3. CiSafe zero-hit denial tests are not symmetric with AgentSafe across PUT, DELETE, HEAD, OPTIONS, and generic request.
4. Guards are better but still mostly string/pattern based; they do not prove every `send()` path is dominated by `maybe_denied_response()`.
5. Async HTTP helper status needs a precise statement: either tested, structurally guarded, or explicitly deferred.
6. Final verification should be recorded after polish.

## Non-Goals

Do not reopen loader/profile semantics.

Do not redesign the HTTP library.

Do not implement full Nmap HTTP parity.

Do not migrate SSH, SMB, database, LDAP, or SNMP libraries in this polish pass.

Do not add public-internet-dependent fixtures.

Do not expand the roadmap unless the current closure items are complete.

## Workstream 1: Documentation Consistency Cleanup

### Files

- `architecture/nse_integration.md`
- `docs/NSE_COMPATIBILITY.md`
- `.opencode/skills/eggsec-nse/SKILL.md`
- `AGENTS.md`
- `crates/eggsec-nse/AGENTS.override.md`

### Required Updates

1. Remove stale references that say HTTP reqwest capability bypass is still unresolved if the current method tests prove it is fixed for covered sync methods.
2. Replace broad wording with precise language:
   - HTTP sync network methods are preflight-gated by `NseCapabilityContext`.
   - Denied automated-profile HTTP calls are tested with zero-hit local fixtures.
   - HTTP protocol fidelity remains partial: HTTP/2, full cookie jar semantics, redirect edge cases, TLS/NSE-specific behavior, and async helper parity may still be partial or deferred.
3. Fix milestone numbering inconsistencies such as `Milestone 6` references inside Milestone 5 sections unless intentionally introducing a new milestone.
4. Ensure compatibility matrix labels do not conflate capability enforcement with full protocol parity.

### Acceptance Criteria

- Docs no longer imply the covered sync HTTP reqwest bypass is unresolved.
- Docs still avoid claiming full Nmap HTTP parity.
- Milestone numbering is internally consistent.

## Workstream 2: CiSafe HTTP Denial Symmetry

### Problem

AgentSafe zero-hit denial coverage is broad. CiSafe zero-hit denial coverage currently covers GET and POST. CiSafe is the stricter automated/CI profile and should have symmetric coverage for HTTP methods where fixtures exist.

### Required Tests

Add CiSafe zero-hit denial tests for:

- PUT;
- DELETE;
- HEAD;
- OPTIONS;
- generic request.

Each test should:

1. Start local `HttpServer`.
2. Use `make_ci_safe_runtime_profile(...)`.
3. Run the corresponding fixture.
4. Assert at least one `network_tcp` denial event.
5. Assert `server.hits() == 0`.
6. Assert the normal success marker is absent if the fixture output makes this stable.

### Acceptance Criteria

- AgentSafe and CiSafe zero-hit coverage are symmetric for all covered sync HTTP methods.
- ManualPermissive success tests continue to pass.

## Workstream 3: Strengthen HTTP Send-Path Guarding

### Problem

The current guard confirms operation strings and strict hit assertions, but it does not robustly prove that every `.send()` call in `http.rs` is preflight-gated by `maybe_denied_response()`.

### Preferred Guard Strategy

Add a small script-level structural check using `python`, `perl`, or robust shell scanning inside `scripts/check-architecture-guards.sh`.

The guard should:

1. Scan `crates/eggsec-nse/src/libraries/http.rs`.
2. Locate every sync send expression:
   - `.send()`;
   - `req.send()`;
   - `client.get(...).send()`;
   - `client.post(...).send()`;
   - `client.put(...).send()`;
   - `client.delete(...).send()`;
   - `client.head(...).send()`;
   - `client.request(...).send()`.
3. For each send, verify the same closure/function block includes `maybe_denied_response(` or a direct `check_network_tcp(` before that send.
4. Fail with line numbers if any send path lacks a nearby preflight gate.

If full closure parsing is too brittle, use a bounded window check:

- for each `.send()` line, require `maybe_denied_response(` within the previous 40 lines;
- explicitly exempt non-network helper functions if any appear.

### Acceptance Criteria

- A new HTTP send path without preflight gate fails the architecture guard.
- Guard output identifies the offending line.

## Workstream 4: Async HTTP Helper Status

### Problem

The current hardening proof is strongest for sync HTTP methods. Async helper methods may exist or may be unused/deferred. The repo should explicitly state and test their status.

### Steps

1. Audit `http.rs` for async methods such as:
   - `async_get`;
   - `async_post`;
   - `async_request`;
   - any `AsyncClient` send path.
2. If async methods are registered and callable from Lua:
   - ensure they use `maybe_denied_response()` or equivalent before send;
   - add at least one zero-hit AgentSafe or CiSafe async method test if practical.
3. If async methods are internal/deferred/not exposed:
   - document status clearly in `docs/NSE_COMPATIBILITY.md` and architecture notes;
   - add a guard that flags future async send paths without preflight gating.

### Acceptance Criteria

- Async HTTP helper status is not ambiguous.
- Any exposed async send path is preflight-gated or explicitly deferred.

## Workstream 5: Method/Path Assertions in Success Tests

### Problem

`HttpServer` now tracks `last_method()` and `last_path()`, but success tests may still only assert `hits() > 0`.

### Steps

For ManualPermissive success tests, assert method/path where stable:

- GET `/`;
- POST `/api/test`;
- PUT `/api/test`;
- DELETE `/api/test`;
- HEAD `/`;
- OPTIONS `/`;
- generic request `/`.

### Acceptance Criteria

- Success tests prove the expected method/path reached the server, not just that some connection occurred.

## Workstream 6: Final Verification Record

Record the final polish verification in `architecture/nse_integration.md` or an adjacent closure note.

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

- Verification records method coverage and guard status.
- Any remaining async/TLS/HTTP parity gap is explicitly listed.

## Final Acceptance Criteria

This polish pass is complete when:

- Docs accurately describe covered sync HTTP enforcement as closed while preserving protocol-fidelity caveats.
- Milestone numbering/future-work references are consistent.
- CiSafe zero-hit tests match AgentSafe coverage for all implemented sync HTTP fixtures.
- Success tests assert method/path where stable.
- Architecture guards catch un-gated HTTP send paths more directly than the current count/string checks.
- Async HTTP status is explicit.
- Final verification is recorded.

## Further Roadmap Items

After this polish pass, the NSE track does not need another immediate corrective milestone unless CI or runtime tests reveal failures. Remaining roadmap items are optional expansion tracks:

1. **TUI NSE report rendering** — consume `NseRunReport` / `ReportEnvelope` in the TUI with panels for compatibility, rules, libraries, capability denials, evidence, raw output, and diagnostics.
2. **Additional upstream-style fixtures** — broaden representative compatibility coverage, still local-only and truthfully labeled.
3. **Deferred protocol-library migration** — prioritize SSH, SMB, databases, LDAP, SNMP only when local deterministic fixtures are available.
4. **TLS/sslcert local fixture coverage** — add deterministic local certificate/TLS tests where practical.
5. **Async HTTP parity** — only if async helper methods are actually user-facing or needed by scripts.
6. **Runtime corpus performance** — add caching or fixture selection if corpus runtime becomes materially slow.

Recommended next roadmap item after polish: TUI NSE report rendering, because the report/data model is now mature enough to surface usefully to manual users without reopening enforcement semantics.