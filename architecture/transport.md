# Scoped Transport Contract Module (Phase B)

## Role & Responsibilities

One EggSec-owned outbound HTTP capability contract, independent of any
concrete client. Authorization enforcement is part of dispatch: every
execution takes a `&dyn NetworkAuthority`, so a caller cannot obtain an
unrestricted dispatch by forgetting a scope helper.

**Non-responsibilities:**
- The contract does not migrate concrete backends (Phase D). Reqwest
  call sites keep working through documented compatibility wrappers.
- The contract does not invent a second target-policy language. Policy
  lives in the canonical `Scope`/`TargetScope` model; this crate owns only
  the checkpoint *shape* (neutral destination descriptors).
- The contract does no I/O itself. DNS facts come from a
  `TransportResolver`; connections are made by backends (Phase D) or
  simulated by the recording fake.

## Location & Feature Gating

| Item | Path | Feature Gate |
|------|------|:------------:|
| Transport crate | `crates/eggsec-transport/` | None (always compiled) |
| Fake/recording transport | `crates/eggsec-transport/src/fake.rs` | `cfg(test)` or `test-util` (never in production builds) |
| Engine authority binding | `crates/eggsec/src/config/scope_transport.rs` | None (always compiled) |
| Contract closure tests | `crates/eggsec/tests/transport_contract.rs` | Integration tests (run via `rest-api` suite) |
| Canonical header helpers | `crates/eggsec-transport/src/headers.rs` + `auth_context` transport fns | None |

Workspace membership: `eggsec-transport` is the 17th workspace crate
(dependency-light leaf). The engine depends on it; it depends on nothing
in the workspace.

Dependency envelope (`cargo tree -p eggsec-transport`):

- `bytes`, `http` (Method/Status/HeaderMap types), `url`, `thiserror`
- No `reqwest`/`hyper`/`rustls`/`tokio-rustls`/`hickory-resolver`/`eggfetch`/`eggress`
- No `tokio`, no `serde`, no `async-trait` (native `async fn` in traits, MSRV 1.88)

## Architecture

### Request DTOs (`request.rs`)

| Type | Purpose |
|------|---------|
| `ScopedHttpRequest` | Dispatch-boundary DTO: `method` (`http::Method`), `url` (`Url`, userinfo rejected), `headers` (`HeaderMap`), `body` (`RequestBody`), `timeout` (`TimeoutPolicy`), `redirect` (`RedirectPolicy`), `proxy` (`ProxyIntent`), `tls` (`TlsPolicy`), `hints` (`TransportHints`) |
| `RequestBody` | `Empty` / `Bytes(Bytes)` only — all variants replayable by construction (retries/redirects require re-emission; streaming is out of scope for Phase B per the parity matrix) |
| `TimeoutPolicy` | Per-request value (`request_timeout` always set, `connect_timeout` optional); pool/nodelay stay backend-owned |
| `RedirectPolicy` | `None` / `SameHostOnly{max}` (scoped-tooling gate, verbatim) / `AuthorityChecked{max}` (follow any, but each hop needs separate authority approval) |
| `ProxyIntent` | `Direct` / `Http{endpoint, credential?}` / `All{...}` — endpoint separate from ultimate destination (distinct auth decisions) |
| `TlsPolicy` | `verified()` / `insecure()` + optional SNI/Host overrides (mismatched overrides deny at `tls-consistency`); no custom roots/identity/SNI parity needed (unused per Phase A) |
| `TransportHints` | Narrow: `user_agent?`, `tcp_nodelay` (default true) |

Secret-bearing fields (`Authorization`, `Cookie`, `Proxy-Authorization`,
proxy credentials, URL userinfo) have redacted `Debug`: `redacted_headers_debug()`,
`redact_url_for_debug()`, `ProxyCredential`/`ProxyIntent`/`ScopedHttpRequest`/`ScopedHttpResponse`
impls. Tests assert no secret survives `format!("{:?}")`.

### Authorization checkpoints (`policy.rs` + `transport.rs`)

```rust
trait HttpTransport {
    async fn execute(
        &self,
        authority: &dyn NetworkAuthority,  // mandatory — no unchecked path
        request: ScopedHttpRequest,
    ) -> Result<ScopedHttpResponse, TransportError>;
}
```

Checkpoint order (all fail-closed via `TransportError::PolicyDenied{checkpoint, reason}`):

1. **initial-url** — canonicalization, host present, userinfo rejected
2. **host** — pattern + port pre-check (direct-IP literals get full CIDR check here)
3. **dns** — `authorize_resolved(host, candidates) -> approved subset`
4. **socket** — selected address re-verified immediately before connect
5. **redirect** — each hop (`authorize_redirect(from, to)` + re-run of host/dns/socket)
6. **reresolution** — each retry/re-dial (defaults to `authorize_resolved`)
7. **proxy** — endpoint **and** ultimate as separate decisions (authorizing one never authorizes the other)
8. **tls-consistency** — SNI/Host override must equal the request host

### Resolver/connect binding (`resolver.rs`)

Safe model (TOCTOU-closed):

```text
resolve host -> candidate addresses
             -> authority filters/approves candidates
             -> transport receives approved set/binding
             -> connector uses only approved addresses
```

- `TransportResolver::resolve(host) -> ResolvedCandidates` (facts only, deterministic sorted order, no class filtering).
- `SystemTransportResolver` (std `ToSocketAddrs`), `InMemoryResolver` (fixtures).
- `validate_binding(host, candidates, approved)` fails closed when `approved` is empty or contains un-resolved addresses.
- IPv4/IPv6: both reported, policy decides. No TTL/cache in Phase B: every evaluation resolves fresh; revalidation on each connect/retry/redirect hop.

### Engine binding (`config/scope_transport.rs`)

`ScopeAuthority<'a>` implements `NetworkAuthority` over `&'a Scope`:

- Mixed authorized/unauthorized DNS answers deny (all-must-match).
- Direct IP literals skip DNS but keep CIDR + port checks.
- Empty `allowed_targets` + `require_explicit_scope` denies; otherwise non-public addresses blocked (loopback exempt), mirroring `Scope::is_target_allowed_with_resolver`.
- Proxy endpoint and ultimate destination checked independently.
- Insecure TLS never changes the verdict (orthogonal).

### Canonical header/auth helpers (WS4)

| Canonical (new) | Compat wrapper (temporary) |
|-----------------|----------------------------|
| `eggsec_transport::apply_auth_headers(HeaderMap, headers, cookies)` — true cookie merge | `auth_context::apply_auth_context_to_request(RequestBuilder, ...)` |
| `auth_context::apply_auth_context_to_transport(HeaderMap, entry)` | — |
| `auth_context::apply_auth_context_to_map(HashMap, existing_cookie?, entry) -> Option<String>` | — |
| `ai::AiClient::auth_headers() -> Vec<(String,String)>` + `apply_auth_to_transport(HeaderMap)` | `ai::AiClient::apply_auth(RequestBuilder)` |
| `integrations::common::should_retry_status(u16)` + `backoff_for_attempt(attempt, retry_after)` | `integrations::common::send_with_retry(RequestBuilder, ...)` |

Pure transformation functions own the semantics; compat wrappers delegate
to them. No new `RequestBuilder` boundary-crossing APIs.

### Fake transport (`fake.rs`, `test-util`)

`RecordingFakeTransport` performs the full checkpoint sequence against the
caller authority + injected resolver, records each `RecordedHop` (method,
redacted URL, host/port, selected + approved addresses, timeout/redirect/
proxy/TLS propagation, auth-presence bits, checkpoint order), and returns
canned responses (including redirect chains). Hostname hops after the first
authorize via `authorize_reresolution` (checkpoint `reresolution`) rather
than `authorize_resolved`, mirroring real backends; IP literals keep
`authorize_resolved` on every hop (the literal is its own fact). No I/O,
no tasks, no clock: timeouts are recorded, not elapsed. Dropping the
future cancels (no background work).

Tests assert: exact destination + binding, redirect order (same-host follows,
cross-host stops without dispatch), header/cookie presence without values,
policy propagation, and fail-closed denial (no hop recorded).

## Behavior / Flow

```text
ScopedHttpRequest
  → authorize_initial_url → authorize_host
  → resolver.resolve → authorize_resolved (first hop) /
    authorize_reresolution (later hops) → validate_binding
  → authorize_socket → check_tls_consistency
  → authorize_proxy (+ proxy DNS/socket binding when proxied)
  → dispatch (fake: canned; Phase D: backend dials approved addr only)
  → 3xx? authorize_redirect → re-run host/dns-or-reresolution/socket per hop
```

## Public API

Re-exported from `eggsec-transport/src/lib.rs`:
- `ScopedHttpRequest`, `RequestBody`, `TimeoutPolicy`, `RedirectPolicy`, `ProxyIntent`, `ProxyCredential`, `TlsPolicy`, `TransportHints`
- `ScopedHttpResponse`, `ConnectionInfo`
- `NetworkAuthority`, `PolicyCheckpoint`, `TransportError`, `require_policy`
- `TransportResolver`, `SystemTransportResolver`, `InMemoryResolver`, `ResolvedCandidates`, `ApprovedBinding`, `validate_binding`
- `HttpTransport`
- Header helpers: `apply_auth_headers`, `merge_cookie_header`, `cookies_to_header_value`, `header_map_from_pairs`, `is_sensitive_header`, `redacted_headers_debug`, `REDACTED`
- `http` re-exports: `HeaderMap`, `HeaderName`, `HeaderValue`, `Method`, `StatusCode` (name transport-neutral types without a direct `http` dep)
- Test-only: `RecordingFakeTransport`, `CannedResponse`, `RecordedHop`

Engine: `eggsec::config::ScopeAuthority`.

## Integration Points

| Consumer | How It Uses the Contract |
|----------|--------------------------|
| Fuzzer/auth-context call sites | Keep using the compat wrapper today; new code uses `apply_auth_context_to_transport` / `_to_map` |
| AI tracker clients | `auth_headers()` / `apply_auth_to_transport()` own semantics; `apply_auth()` delegates |
| Integrations retry | `should_retry_status()` / `backoff_for_attempt()` own thresholds; `send_with_retry()` consumes them |
| Future Phase D backends | Implement `HttpTransport` with the checkpoint order above; dial only `ApprovedBinding` addresses |

## Testing

- `cargo test -p eggsec-transport` (18 unit tests: redaction, cookie merge, redirect policy, TLS consistency, resolver ordering, binding, fake destination/redirect/denial, later-hop re-resolution + binding-checkpoint mapping).
- `cargo test -p eggsec --features rest-api --test transport_contract` (12 closure tests running Phase A behaviors through `ScopeAuthority` + fake: binding, out-of-scope DNS, mixed answers, same/cross-host redirects, later-hop re-resolution order, userinfo, secret redaction, direct IP, proxy distinctness, TLS orthogonality, invented-address rejection).
- Phase A `network_policy_invariants.rs` (12 behaviors) remains the measurement baseline; the contract suite proves the same behaviors through the new layer.

## Phase C adapter (no production migration)

The contract is implemented against a real backend in
`crates/eggsec-transport-eggfetch/` ([transport_eggfetch.md](transport_eggfetch.md)):
`EggfetchTransport` enforces this checkpoint order over published
`eggfetch-core` via approved-IP pinning (the connector only ever sees the
authorized literal) and a manual redirect loop (auto-follow doubly
disabled; each hop authorized before dispatch). HTTP/3 is off, proxied
execution fails closed, and no production consumer is wired to it yet
(guard Check 102). Parity evidence:
`crates/eggsec-transport-eggfetch/tests/parity.rs` (33 tests over local
fixtures) + `crates/eggsec/tests/transport_eggfetch_parity.rs` (5 engine
interop tests through `ScopeAuthority`).

## Phase D first increment (interfaces + boundaries; backend wiring pending)

`plans/network-dependency-phase-d-outbound-client-migration.md` (increment 1,
2026-09-12; guard Check 103). Consumer backends still dispatch on their
pre-migration stacks; what migrated is the interface boundary so no new
concrete-client leakage is possible:

- **Agent (WS1):** `eggsec-agent::LifecycleManager<T: HttpTransport>` injects
  transport + `NetworkAuthority` at composition (no `reqwest`/`rustls` in
  manifest or sources; `cargo tree -p eggsec-agent` 257 → 153 lines).
  Callback probes are GET/5s/`SameHostOnly{5}`/verified-TLS through the
  mandatory checkpoints; tests use the recording fake (24 agent tests green).
- **Shared helpers (WS2):** `auth_context::apply_auth_context_to_request`
  and `AiClient::apply_auth` compat wrappers removed — shared APIs carry no
  `RequestBuilder`. Fuzzer translates via canonical
  `apply_auth_context_to_map` locally; AI loops `auth_headers()` directly.
  Remaining `pub` concrete surfaces (`fuzzer::advanced::fuzz`,
  `utils::client_pool::ClientPool`, crate-internal `send_with_retry`) are
  documented pending full backend migration; no new boundary allowed.
- **NSE capability (WS3):** `eggsec-nse/src/http_capability.rs` is the narrow
  script capability (pure 8-method DTO builder + profile-gated TLS, no I/O,
  no concrete clients). Lua dispatch still runs on `blocking` reqwest behind
  `check_network_tcp` preflight (async-Lua story + `ScopeAuthority` binding
  pending); `openssl`/`native-tls` stay feature-gated for protocol compat.
- **Proxy boundary (WS4):** `eggsec-web-proxy/src/outbound.rs` owns the
  A/B split (intercept/server TLS never touches the client contract;
  proxy-routing probes stay on minimal reqwest until the adapter supports
  authorized proxy routing). Direct-probe builder added for non-proxied paths.
- **Pruning (WS5):** web-proxy reqwest cut to `rustls-no-provider` + `socks`
  (unused json/http2/form/query/blocking/cookies dropped). Adapter features
  unchanged (still minimal). Engine keeps its full reqwest set — every
  feature has call sites (corrects the Phase A "no `.form()`/`.query()`"
  note).
- **Remaining owners (WS6):** see
  [network_dependency_baseline.md](network_dependency_baseline.md) §7.2 for
  the per-owner disposition table (engine per-subsystem pending, NSE
  blocking/compat, proxy split, python pending, adapter intentional).

## Invariants & Gotchas

1. **No unchecked dispatch** — `execute` without an authority does not exist.
2. **No second scope language** — policy is `Scope`/`TargetScope` only.
3. **Binding closed** — approved ⊆ candidates, non-empty, connector dials approved only.
4. **Redirects stop, not follow, cross-host** under `SameHostOnly` (response surfaced).
5. **Proxy ≠ target** — two decisions, two DNS/socket bindings.
6. **Secrets never in `Debug`** — presence bits only in recordings.
7. **Bodies replayable** — no streaming variant until replayability is proven.
8. **`http` types are stable STD, not concrete clients** — `Method`/`StatusCode`/`HeaderMap` are allowed; `reqwest::Client`/`RequestBuilder` are not.

---

See also: [network_dependency_baseline.md](network_dependency_baseline.md) (Phase A measurement + Phase D increment-1 addendum §7), [auth_context.md](auth_context.md) (canonical vs removed compat), [overview.md](overview.md), [config.md](config.md)

*Last verified against source: 2026-09-12*
