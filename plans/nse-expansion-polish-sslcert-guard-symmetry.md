# NSE Expansion Polish: sslcert CiSafe Symmetry and Per-Connect Guarding

> **Status: Executed** (2026-07-07)

## Purpose

Close the final small polish items after the TLS/sslcert and creds corrective pass.

The corrective pass fixed the main issue: visible `sslcert` network certificate paths now check `network_tcp` before `TcpStream::connect`, the TLS local fixture server tracks accepted connections, and AgentSafe/CiSafe denial coverage proves denied paths do not reach the local TLS server for key cases. The remaining items are test symmetry and guard precision.

## Current State

Confirmed current state:

- `sslcert.get_certificate` checks crypto capability and then `network_tcp` before `TcpStream::connect`.
- `sslcert.get_chain_certs` checks crypto capability and then `network_tcp` before `TcpStream::connect`.
- `TlsEchoServer` exposes `hits()` and increments it on accepted connections.
- ManualPermissive TLS/sslcert tests cover `get_certificate`, `parse_cert`, `get_subject`, `get_chain_certs`, and `is_valid`.
- AgentSafe zero-hit tests cover `get_certificate` and `get_chain_certs`.
- CiSafe zero-hit tests cover `get_certificate`.
- `creds` is documented as pure/in-memory with no external side effects.
- Architecture guards check that `sslcert.rs` contains `check_network_tcp` when `TcpStream::connect` exists.

Remaining polish items:

1. Add CiSafe zero-hit denial coverage for `sslcert.get_chain_certs` to match AgentSafe coverage.
2. Strengthen the `sslcert` guard so it checks each `TcpStream::connect` line has a nearby preceding network gate, rather than only checking that the file contains at least one `check_network_tcp` call.
3. Optionally add ManualPermissive `server.hits() > 0` assertions for TLS success tests where stable, matching the HTTP proof pattern.
4. Record the polish verification in architecture docs.

## Non-Goals

Do not redesign `sslcert`.

Do not implement full Nmap `sslcert` parity.

Do not migrate additional protocol libraries.

Do not change `creds` semantics unless a new issue is found.

Do not add public network TLS tests.

## Workstream 1: Add CiSafe get_chain_certs Zero-Hit Test

### Required Test

Add a test in `crates/eggsec-nse/tests/local_protocol_tests.rs`:

```rust
#[test]
fn local_sslcert_get_chain_certs_ci_safe_denied() { ... }
```

The test should:

1. Start `local_fixtures::TlsEchoServer`.
2. Use `make_ci_safe_runtime_profile(vec![])`.
3. Run `scripts/protocol/sslcert_get_chain_certs_local.nse` against `127.0.0.1` and the dynamic TLS port.
4. Assert at least one denied `network_tcp` capability event.
5. Assert `server.hits() == 0`.
6. Assert normal success marker such as `chain_count=` is absent if output is stable enough.

### Acceptance Criteria

- AgentSafe and CiSafe both cover zero-hit denial for `get_certificate` and `get_chain_certs`.
- Denied CiSafe `get_chain_certs` cannot reach the local TLS listener.

## Workstream 2: Add ManualPermissive TLS Hit Assertions

### Problem

ManualPermissive TLS success tests currently prove output success, but not always that the TLS listener accepted a connection. The server now exposes `hits()`, so tests should assert the expected permitted side effect occurred where stable.

### Steps

For success tests that call networked sslcert paths, assert:

```rust
assert!(server.hits() > 0, "ManualPermissive sslcert ... must reach the TLS server");
```

Apply to:

- `local_sslcert_get_certificate_success`;
- `local_sslcert_parse_cert_success` if it calls `get_certificate` first;
- `local_sslcert_get_subject_success` if it calls `get_certificate` first;
- `local_sslcert_get_chain_certs_success`.

Do not add hit assertions to pure parsing tests if they are changed later to use static PEM only.

### Acceptance Criteria

- ManualPermissive tests prove allowed TLS paths reach the listener.
- Denied automated-profile tests prove denied TLS paths do not reach the listener.

## Workstream 3: Strengthen sslcert Per-Connect Guard

### Problem

The current architecture guard only checks whether `sslcert.rs` has `TcpStream::connect` and at least one `check_network_tcp`. That can miss a future second connect path without a gate.

### Required Guard

Replace or supplement the current guard with a per-connect/window-based check.

Suggested implementation in `scripts/check-architecture-guards.sh`:

1. Search `crates/eggsec-nse/src/libraries/sslcert.rs` for `TcpStream::connect` lines.
2. For each connect line, inspect the preceding 30 lines.
3. Require `check_network_tcp` or a centralized helper such as `maybe_network_denied_response` in that window.
4. Print offending line number and source line on failure.

Example shape:

```bash
SSL_CERT_BAD_CONNECTS=$(awk '
/TcpStream::connect/ {
  line=NR; text=$0; found=0;
  for (i=line-30; i<line; i++) { if (checks[i]) found=1; }
  if (!found) print line ": " text;
}
/check_network_tcp|maybe_network_denied_response/ { checks[NR]=1 }
' crates/eggsec-nse/src/libraries/sslcert.rs)
```

Use a simpler robust approach if preferred.

### Acceptance Criteria

- A future ungated `TcpStream::connect` in `sslcert.rs` fails the guard.
- Guard output identifies the offending line.
- Existing gated connect paths pass.

## Workstream 4: Optional Helper Consolidation

The current `sslcert` implementation repeats crypto/network denial table construction. This is acceptable, but future drift risk can be reduced with helpers:

- `denied_table(lua, kind, reason)`;
- `maybe_crypto_denied_response(...)`;
- `maybe_network_denied_response(...)`.

This workstream is optional. Do it only if it reduces duplication without churn.

### Acceptance Criteria

- If added, helpers preserve current behavior.
- Tests still pass.
- No broad library rewrite.

## Workstream 5: Documentation and Verification Closeout

Update `architecture/nse_integration.md` or a dedicated closeout note with:

- CiSafe `get_chain_certs` zero-hit test coverage;
- per-connect sslcert guard status;
- ManualPermissive hit assertion coverage if added;
- final status of creds as pure/in-memory.

Required verification:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse --test local_protocol_tests
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=1
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=4
cargo test -p eggsec-nse --features nse
bash scripts/check-architecture-guards.sh
cargo fmt --all --check
cargo clippy --lib -p eggsec-nse --features nse
```

## Final Acceptance Criteria

This polish pass is complete when:

- CiSafe `sslcert.get_chain_certs` denial test exists and asserts zero TLS server hits;
- ManualPermissive TLS success tests assert positive server hits where stable;
- `sslcert` architecture guard checks each `TcpStream::connect` path individually;
- docs record final sslcert/creds closeout status;
- verification is recorded.

## Handoff Notes

Keep this pass small. The main functional issue is already fixed. This is closure polish to make the proof symmetric and make the guard harder to accidentally bypass.