# Security Assurance Hardening Milestone 005 — Daemon HTTP Adapter Safety

Status: closed

Repository baseline: `77188755` (`feat/auto-20261117-7451c367`, 2026-11-17)

Source roadmap:

- `plans/subsystems/security-assurance-hardening-roadmap.md` §7 Milestone 5
- Finding A4 in §4 of the same roadmap

Long-term requirements:

- `plans/000-long-term-specification.md` (daemon/IPC ownership)
- `plans/001-terminology-and-domain-model.md` (`RuntimeSurface`, client identity)
- `AGENTS.md` — "`eggsec-daemon` default deps are `eggsec-runtime` +
  `eggsec-daemon-protocol` only"

Applicable ADRs: **none for this milestone**, because the adapter stays unwired.
An ADR IS required before any future milestone connects this adapter to a
listener; see §14.

Primary class: invariant (latent surface)

## 1. Objective

Make the dormant HTTP adapter impossible to configure into an unauthenticated,
non-loopback listener, and close its Host/Origin and anonymous-ownership gaps —
without wiring it, exposing it, or changing the Unix-socket daemon contract.

## 2. Why this milestone is ready

- No hard dependency. The module compiles under `--features http-api`, which is
  not a default feature.
- The Unix-socket daemon already has the correct ownership model; this milestone
  applies it to a second transport rather than inventing one.
- `HttpConfig` is constructed nowhere outside its own module, so its shape
  can change without callers.
- The daemon already knows how to validate bind intent
  (`crates/eggsec-daemon/src/http.rs:568-578`) and already enforces ownership
  when a client id is present (`host.rs:885-950`).

## 3. Current implementation evidence

```rust
// crates/eggsec-daemon/src/http.rs:20-37
pub struct HttpConfig {
    pub bind_addr: String,        // "127.0.0.1:9876"
    pub require_auth: bool,       // false
    pub allow_public_bind: bool,  // false
}
impl Default for HttpConfig { ... }
```

```rust
// crates/eggsec-daemon/src/http.rs:96-101
fn enforce_auth(state: &HttpState, auth: &AuthenticatedClientId) -> Result<(), Response> {
    if state.require_auth && auth.0.is_none() { Err(auth_required_response()) } else { Ok(()) }
}
```

Facts that determine severity:

- `pub mod http` is behind `#[cfg(feature = "http-api")]`
  (`crates/eggsec-daemon/src/lib.rs:10-11`); the feature is not in
  `default = []` (`crates/eggsec-daemon/Cargo.toml:13-15`).
- `run_http_server` (`http.rs:579`) has **no call sites** in the workspace. The
  adapter is dormant; the audit finding is latent, not exploitable today.
- "Auth" is header presence: `AuthenticatedClientId` parses any `Uuid`
  (`http.rs:60-75`); `ClientId` is a bare `Uuid`
  (`crates/eggsec-runtime/src/ids.rs:102-116`). It is an identity assertion, not
  a credential, so `require_auth = true` does not authenticate anyone.
- The snapshot route skips ownership checks entirely for anonymous callers:
  `if let Some(client_id) = client_id { ... }` (`host.rs:885`), so `None` falls
  through to the store read.
- The router (`http.rs:609-630`) installs no `Host`/`Origin` validation and no
  CORS layer; `axum::serve(listener, app)` uses no `ConnectInfo`.

Consequence if wired as-is: a malicious web page can DNS-rebind its own hostname
to `127.0.0.1`, become same-origin with the daemon, create a session and submit
tasks. Scope still applies, but the no-manifest default exempts loopback
(`crates/eggsec-policy/src/scope.rs:190-197`), so the attack can drive scans of
the user's local services.

## 4. Invariants that must not regress

- The adapter stays unwired; no CLI flag, no binary entry point, no default
  feature change.
- The Unix-socket path's ownership semantics are unchanged. Prefer enforcing
  anonymity policy **in the HTTP adapter** over editing `host.rs` shared logic;
  if `host.rs` must change, it must change identically for both transports and
  be covered by the existing daemon tests.
- `EnforcementContext::evaluate()` remains the pre-dispatch gate; the HTTP
  adapter adds admission and identity, not policy.
- No new default dependency in `eggsec-daemon` beyond the documented contract;
  any `http-api`-scoped dependency is optional and recorded.
- No secret in logs or `Debug` output.
- `make check-no-default` keeps passing.

## 5. Scope

### In scope

- `HttpConfig` shape and its `Default` removal.
- An explicit `AuthMode` with no "disabled" variant.
- Bind validation coupled to auth mode.
- Host and Origin validation middleware.
- `401` for anonymous session routes.
- Tests for the configuration matrix and the middleware.
- Documentation stating the adapter is dormant and unwired.

### Explicitly out of scope

- Wiring `run_http_server` to a CLI command or the daemon binary.
- Token issuance, rotation, persistence, or a token file.
- TLS termination for this listener.
- Changing `ClientId` into an authenticated principal.
- Changing the Unix-socket handshake.
- CORS support (absence is intentional; document it).

## 6. Required production changes

### Core/domain

Replace the boolean pair with an explicit mode:

```rust
pub enum AuthMode {
    /// Bearer token required on every non-liveness route.
    Token(AuthToken),
    /// No token; only constructible for a loopback bind.
    LoopbackTrust,
}
pub struct HttpConfig {
    pub bind_addr: String,
    pub auth: AuthMode,
    pub allow_public_bind: bool,
}
```

- Remove `impl Default for HttpConfig`; construct only through a
  validating constructor (`HttpConfig::new(...) -> Result<_, ConfigError>`).
- `AuthToken` is a local newtype with: no `Display`, redacted `Debug`, and a
  constant-time comparison. Preferred implementation: `subtle::ConstantTimeEq`
  as an optional `http-api` dependency (already present in the workspace via
  `eggsec-policy`). If dependency policy rejects it, put the type and compare in
  `eggsec-daemon-protocol` instead and record the choice.
- Validation rules:
  1. loopback + `Token` → Ok;
  2. loopback + `LoopbackTrust` → Ok;
  3. non-loopback + `LoopbackTrust` → Err (regardless of `allow_public_bind`);
  4. non-loopback + `Token` without `allow_public_bind` → Err;
  5. non-loopback + `Token` with `allow_public_bind` → Ok, warn.
  6. empty token → Err.

### Storage and migrations

None. No token is persisted.

### Protocol and DTOs

HTTP headers only: `Authorization: Bearer <token>` plus the existing client-id
header. No `ServerMessage`/`ClientCommand` change.

### Runtime and concurrency

Add two `axum` middleware layers, both synchronous and cheap:

- **Host check:** compare the `Host` header authority to the configured bind
  authority (accept `localhost` ⇄ `127.0.0.1` and `[::1]` ⇄ `localhost` only for
  loopback binds). Missing or mismatched `Host` → `403`.
- **Origin check:** if `Origin` is present, require it to match the same
  authority set; `Origin: null` → `403`. No CORS preflight handling is added.

Then authorization:

- `Token` mode: require `Authorization: Bearer` and a constant-time match on
  every route except `/health` and `/capabilities`.
- Identity: require a parseable client id on `/sessions/**` in both modes;
  absent → `401`.
- Do not edit `host.rs`'s anonymous branch unless the middleware cannot
  guarantee the invariant; if edited, treat `None` as denied for
  `GetPersistedSnapshot` and mutating commands on **both** transports and add
  daemon tests.

Keep `axum::serve` unchanged except for `.layer(...)` calls.

### Frontend or operator surface

None. The module remains internal and unwired.

### Security and authorization

This is the change. Refusals log the route and the deciding rule, never the
token or the raw header value.

### Documentation and static guards

- Module doc comment in `http.rs`: "dormant; not wired; requires an ADR before
  exposure".
- `architecture/daemon.md`: state that the HTTP adapter is present but not a
  supported surface, and what it would require.
- Guard: fail if `run_http_server` gains a caller without an accompanying ADR
  reference in the same commit. If that guard is impractical, document the
  requirement in `architecture/daemon.md` and skip the guard rather than writing
  a fragile one.

## 7. Ordered work packages

### Work package A — Config and auth mode

Intent: make an unsafe configuration unrepresentable.

Required changes: the types and rules in §6; delete `Default`.

Acceptance evidence: a unit matrix over the six rules; compile failure if a
caller tries `HttpConfig::default()`.

### Work package B — Host/Origin middleware

Intent: close DNS rebinding.

Required changes: the two layers; tests with raw `http::Request` values (no
network needed) asserting `403` for missing/mismatched `Host`, and for
cross-origin/`null` `Origin`.

Acceptance evidence: middleware unit tests; `/health` still `200` with a correct
`Host`.

### Work package C — Authorization and identity

Intent: no anonymous session access.

Required changes: bearer enforcement in `Token` mode; client-id requirement on
`/sessions/**`; `401` bodies carry the existing `ErrorCode::InvalidRequest`
shape used by `auth_required_response`.

Acceptance evidence: tests for missing/incorrect bearer, missing client id, and
the allowed `/health` path.

### Work package D — Documentation and dormant-surface statement

Intent: nobody wires it by accident.

Required changes: comments and `architecture/daemon.md` as in §6.

Acceptance evidence: docs diff; `make check`.

## 8. Failure, cancellation, restart, and contention semantics

- Configuration error: `run_http_server` returns the existing
  `DaemonError::Protocol` variant before binding; no socket is created.
- Middleware rejection: request terminates with `401`/`403`; no command reaches
  `handle_command`.
- Graceful shutdown: unchanged.
- Restart: no persisted auth state, so a restarted process requires a newly
  supplied token. This is acceptable for a dormant surface and must be stated in
  the ADR before wiring.

## 9. Compatibility and migration

- No user-facing surface exists, so there is no migration.
- The module's public Rust API changes (constructor, `AuthMode`). That is
  acceptable because the crate is pre-1.0 and the module is unwired; record it
  in the closure record.
- `make check-no-default` must still pass; the feature remains off by default.

## 10. Required tests

### Focused unit tests

- Config matrix (§6 rules 1–6), including `0.0.0.0`, `::`, and a private LAN
  address.
- Middleware: Host match, Host mismatch, missing Host, cross-origin, `null`
  origin, absent origin.
- Auth: missing bearer, wrong bearer, correct bearer, wrong length, empty token
  config.

### Integration tests

- Router-level test with `tower::ServiceExt::oneshot` (already available via
  axum) proving `/sessions` is `401` without a client id and `/health` is `200`
  without a bearer.

### Restart and recovery tests

Not applicable while unwired.

### Contention and cancellation tests

Not applicable.

### Security and negative tests

- Token is absent from `Debug` output (assert on the formatted config).
- Mismatched `Host` cannot reach a handler (assert the handler was not called,
  or assert the response body is the rejection body).
- Anonymous `GetPersistedSnapshot` over HTTP is denied.

### Migration and compatibility tests

- `cargo check -p eggsec-daemon --no-default-features` unchanged.
- Unix-socket daemon tests unchanged.

## 11. Required verification commands

```bash
make check
make check-no-default
cargo test -p eggsec-daemon --features http-api
```

Do not claim commands that were not actually run in the closure record.

## 12. Documentation updates

- `crates/eggsec-daemon/src/http.rs` module doc.
- `architecture/daemon.md` (dormant-surface statement and the ADR requirement).
- `docs/FEATURE_MATRIX.md` only if the `http-api` description changes.

## 13. Acceptance criteria

1. No configuration yields a non-loopback bind without an explicit opt-in and a
   non-empty token.
2. `HttpConfig` cannot be built with `Default` or with auth "off".
3. Requests with a missing or mismatched `Host` are rejected.
4. Cross-origin and `null`-origin requests are rejected.
5. Anonymous callers cannot reach `/sessions/**`.
6. `run_http_server` still has no caller and the feature is still off by
   default.
7. `make check` and the two daemon commands pass.

## 14. Stop conditions

Stop and report rather than improvise when:

- the fix requires changing `host.rs` semantics for the Unix-socket transport;
- adding a constant-time compare requires a dependency the policy forbids
  (report the constraint; do not hand-roll a comparison with a comment claiming
  constant time);
- the middleware cannot be tested without a real listener (report the seam
  instead of adding a network test to the unit suite);
- anything would wire the adapter or expose a CLI flag — that requires an ADR
  and a separate milestone.

## 15. Closure evidence required

- Commit/PR.
- The config matrix test output.
- Middleware test output including the Host/Origin cases.
- `make check`, `make check-no-default`, and
  `cargo test -p eggsec-daemon --features http-api` output.
- A statement that the adapter remains unwired.
- The chosen home of `AuthToken` and its dependency decision.

## 16. Handoff notes

- Read `crates/eggsec-daemon/src/http.rs` end to end before editing; the test
  module constructs a router directly and is the cheapest place to add
  middleware tests.
- Do not add `ConnectInfo` or peer-address checks as a substitute for Host
  validation; DNS rebinding arrives from `127.0.0.1` and is indistinguishable by
  peer address.
- Do not add a CORS layer. Absence of CORS is part of the current defense; the
  fix is Host/Origin validation.
- Preserve unrelated user changes in the worktree.
