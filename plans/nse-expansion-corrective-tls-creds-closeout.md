# NSE Expansion Corrective Closeout: TLS/sslcert Network Gating and creds Truthfulness

## Purpose

Close the remaining issues found after the NSE expansion implementation pass.

The expansion work substantially advanced the repo: structured TUI report rendering landed, report filtering/search was added, TLS/sslcert local fixtures were introduced, upstream-style corpus fixtures were added, and `creds` registration was moved toward capability-context awareness. Two narrow correctness issues remain:

1. `sslcert` appears to gate crypto capability but still performs direct TCP connection attempts through `TcpStream::connect(...)` without a visible `network_tcp` capability gate.
2. `creds` now accepts `NseCapabilityContext`, but the current implementation appears to use it only nominally. Because the library is currently an in-memory credential store, this may be correct, but docs/registry/guards must not imply capability-routed filesystem/network side effects that do not exist.

This pass should be corrective and narrow. Do not broaden into new protocol migrations.

## Current State

Confirmed current state before this plan:

- TUI rendering consumes `NseRunReport` directly.
- TUI report sections include summary, compatibility, rules, libraries, capability denials, evidence, raw output, and diagnostics.
- TUI report filtering/search and section navigation exist.
- Local TLS fixture infrastructure exists via `TlsEchoServer` bound to `127.0.0.1` on a dynamic port.
- Local sslcert ManualPermissive tests cover `get_certificate`, `parse_cert`, `get_subject`, `get_chain_certs`, and `is_valid`.
- Guard script checks TLS fixture existence and manifest declarations.
- `creds` registration accepts `&NseCapabilityContext` and executor callsite passes it.

Remaining concerns:

- `sslcert.get_certificate` and `sslcert.get_chain_certs` perform direct TCP connections after only crypto checks.
- There are no observed AgentSafe/CiSafe sslcert denial tests proving the local TLS server receives zero connections.
- TLS fixture server does not currently expose a hit counter equivalent to the HTTP server.
- Guard coverage for direct TCP connects in `sslcert.rs` is not strong enough.
- `creds` capability semantics are ambiguous: in-memory store operations may not need external side-effect checks, but the registry/docs should describe that truthfully.

## Non-Goals

Do not redesign the TLS/sslcert library.

Do not implement full Nmap `sslcert` or TLS parity.

Do not migrate SSH, SMB, databases, LDAP, SNMP, or other deferred protocol families.

Do not add public-internet TLS fixtures.

Do not weaken AgentSafe/CiSafe semantics to make TLS tests pass.

Do not invent capability events for pure in-memory operations unless a clear policy decision is made.

## Workstream 1: Add Network TCP Gating to sslcert Connection Paths

### Problem

`sslcert` checks crypto capability before certificate operations, but certificate retrieval also opens a TCP connection. That network side effect must be gated by the same `NseCapabilityContext` model used by HTTP/socket/comm.

### Required Outcome

Every `sslcert` function that can open a TCP connection must check `network_tcp` before `TcpStream::connect(...)`. If denied, it must return a structured Lua table with denial information and must not attempt the connect.

### Implementation Steps

1. Audit `crates/eggsec-nse/src/libraries/sslcert.rs` for all network paths:
   - `TcpStream::connect(...)`;
   - native TLS connect/handshake paths;
   - any helper that accepts host/port and opens a socket.
2. Add a helper, similar to HTTP:

```rust
fn maybe_network_denied_response(
    lua: &Lua,
    ctx: &NseCapabilityContext,
    host: &str,
    operation: &'static str,
) -> LuaResult<Option<Table>>
```

3. Before every TCP connect, call the helper with operation names such as:
   - `sslcert.get_certificate`;
   - `sslcert.get_chain_certs`;
   - any future `sslcert` network function.
4. Preserve the existing crypto gate, but order checks deliberately:
   - either network first, then crypto;
   - or crypto first, then network;
   - document the order and test for the expected denied event kinds.
5. Ensure denied response has fields consistent with HTTP denial tables where practical:
   - `status = 0` if relevant;
   - `error` with denial reason;
   - `reason = "denied"`;
   - optional `capability = "network_tcp"`.

### Acceptance Criteria

- AgentSafe/CiSafe network denial prevents `TcpStream::connect(...)`.
- Denied sslcert network paths produce `network_tcp` capability events.
- ManualPermissive TLS/sslcert success tests still pass.

## Workstream 2: Add TLS Server Hit Counters

### Problem

Current TLS tests prove ManualPermissive success but do not prove denied automated-profile sslcert paths avoid connecting to the TLS server.

### Required Outcome

`TlsEchoServer` should expose a hit counter so tests can assert zero connections for denied sslcert calls.

### Implementation Steps

1. Add `hits: Arc<AtomicUsize>` to `TlsEchoServer`.
2. Increment `hits` only when a TCP connection is accepted by the TLS listener.
3. Expose:

```rust
pub fn hits(&self) -> usize
```

4. Optionally expose last peer or last operation only if useful; avoid unnecessary state.
5. Keep shutdown behavior unchanged.

### Acceptance Criteria

- ManualPermissive TLS tests assert `server.hits() > 0` where stable.
- AgentSafe/CiSafe denial tests assert `server.hits() == 0`.

## Workstream 3: Add AgentSafe/CiSafe sslcert Denial Tests

### Required Tests

In `crates/eggsec-nse/tests/local_protocol_tests.rs`, add denial tests for at least:

- `sslcert_get_certificate_local.nse` under AgentSafe;
- `sslcert_get_certificate_local.nse` under CiSafe;
- `sslcert_get_chain_certs_local.nse` under AgentSafe;
- `sslcert_get_chain_certs_local.nse` under CiSafe.

If `parse_cert`, `get_subject`, or `is_valid` scripts internally call `get_certificate`, decide whether to test them too. If they parse static PEM only, they may be pure crypto/parse tests and should not require network denial.

Each denied test must assert:

1. at least one denied `network_tcp` capability event;
2. TLS server hit count remains zero;
3. output does not contain the ManualPermissive success marker;
4. report compatibility is `Partial`, `Unsupported`, or otherwise consistent with existing denial semantics;
5. evidence includes capability-denial evidence where the report extraction path supports it.

### Acceptance Criteria

- sslcert network operations match HTTP-style zero-hit proof.
- Automated profiles cannot contact the local TLS server through sslcert.

## Workstream 4: Tighten Guards for sslcert Network Side Effects

### Required Guard Updates

In `scripts/check-architecture-guards.sh`, add checks that:

1. fail if `sslcert.rs` contains `TcpStream::connect` without a nearby `check_network_tcp` or centralized `maybe_network_denied_response` call;
2. fail if local TLS denial tests do not assert `server.hits() == 0`;
3. fail if TLS fixture scripts use public hosts or hardcoded non-loopback targets;
4. keep the existing TLS fixture presence/manifest checks.

### Suggested Guard Strategy

A pragmatic window-based guard is acceptable:

- for each `TcpStream::connect` line in `sslcert.rs`, require `check_network_tcp` or `maybe_network_denied_response` within the previous 25 lines;
- print offending line numbers on failure.

### Acceptance Criteria

- Future direct sslcert TCP connects without gating fail guards.
- Denial tests cannot regress to non-zero-hit or permissive assertions unnoticed.

## Workstream 5: Clarify creds Semantics

### Problem

`creds` now accepts a capability context, but the current implementation is an in-memory store using a process-local `LazyLock<Mutex<...>>`. That means it has no direct filesystem/network/process side effect to gate. The implementation may be safe, but the registry/docs/guards need to describe it accurately.

### Required Outcome

`creds` should be described as one of:

1. **Pure/in-memory**: no external side effects; capability context is accepted for future policy compatibility but currently unused; or
2. **Capability-observed state mutation**: add explicit policy checks/events for credential store mutation/access if the project wants in-memory credential storage treated as sensitive; or
3. **PartiallyWrapped**: if some behavior is implemented but not fully policy-observed.

Do not mark it as externally wrapped unless it actually routes side effects through wrappers.

### Recommended Decision

Prefer option 1 unless there is a concrete policy requirement to deny in-memory credential store operations in AgentSafe/CiSafe.

Rationale:

- The current `creds` implementation does not read files, write files, open sockets, or execute processes.
- Treating it as pure/in-memory is truthful and simple.
- File-backed credential/wordlist behavior belongs to `unpwdb` or future file-backed creds work and should be separately gated.

### Steps

1. Audit `creds.rs` for actual side effects:
   - filesystem;
   - network;
   - process/env;
   - persistent storage;
   - global mutable state.
2. Decide status:
   - `Pure` / no external side effects; or
   - `Wrapped` with explicit state-policy events.
3. Update registry metadata:
   - side effects should reflect actual behavior;
   - fallback behavior should be accurate;
   - notes should mention in-memory process-local store if applicable.
4. Update docs:
   - `docs/NSE_COMPATIBILITY.md`;
   - `architecture/nse_integration.md`;
   - agent guidance docs.
5. Update guards:
   - if pure/in-memory, guard direct file/network/process operations do not appear in `creds.rs`;
   - if policy-observed, guard capability checks are present.

### Acceptance Criteria

- `creds` status is truthful.
- No docs imply file/network side-effect wrapping if none exists.
- Guards match the chosen semantics.

## Workstream 6: Runtime Corpus and Manifest Updates

### TLS Manifest

Update TLS fixtures with expected denial metadata if AgentSafe/CiSafe variants are added to manifest-driven tests.

If TLS denial tests remain in `local_protocol_tests.rs` only, document why local-service tests own the network listener path and runtime corpus skips them.

### creds Manifest

For `creds_store.nse`, ensure manifest expectations match actual behavior:

- expected library `creds`;
- no external capability event unless policy-observed state mutation is added;
- compatibility/fidelity label should match implemented API shape;
- profile coverage should be explicit.

### Acceptance Criteria

- Manifest entries remain truthful and not metadata-only unless explicitly labeled.

## Workstream 7: Documentation Closeout

Update docs to reflect the correction:

- TLS/sslcert has both crypto and network gates for network certificate retrieval;
- local TLS denial tests prove automated profiles do not reach the TLS listener;
- sslcert parsing-only helpers may be crypto/pure depending implementation;
- creds is in-memory/pure or policy-observed according to final decision;
- no full TLS/Nmap parity is claimed.

## Workstream 8: Verification

Run and record:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse --test local_protocol_tests
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=1
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=4
cargo test -p eggsec-nse --features nse --test runtime_smoke_tests
cargo test -p eggsec-nse --features nse
bash scripts/check-architecture-guards.sh
cargo fmt --all --check
cargo clippy --lib -p eggsec-nse --features nse
```

If TUI files were touched, also run the relevant TUI/workspace checks.

## Final Acceptance Criteria

This corrective closeout is complete when:

- sslcert TCP connection paths are network-gated before connect;
- AgentSafe/CiSafe sslcert network denial tests prove zero TLS server hits;
- guards catch ungated sslcert `TcpStream::connect` regressions;
- creds status is documented truthfully as in-memory/pure, wrapped, or partial according to actual behavior;
- registry/docs/manifest match the implementation;
- verification is recorded.

## Handoff Notes

Keep this pass narrow. The TUI report implementation looks strong and should not be reopened unless a concrete regression is found. The main risk is side-effect truthfulness in TLS/sslcert and semantic overclaiming for creds. Close those, then return to optional expansion work.