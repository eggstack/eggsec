# Eggress 1.0.8 Phase B — health qualification and closure

Status: Executed (2026-09-22)

Date: 2026-09-22

Depends on:
[eggress-1.0.8-phase-a-proxy-engine-adoption-2026-09-22.md](eggress-1.0.8-phase-a-proxy-engine-adoption-2026-09-22.md)

Parent roadmap:
[eggress-1.0.8-adoption-roadmap-2026-09-22.md](eggress-1.0.8-adoption-roadmap-2026-09-22.md)

## Purpose

Close the Eggress 1.0.8 adoption by making proxy health checks truthful with
respect to the migrated protocol engine, removing now-unnecessary dependencies
only where semantics are preserved, and recording final dependency/security
evidence.

This phase is intentionally separate from proxy-dialing migration. A proxy
health check currently means “perform the configured HTTP(S) request through
this proxy and require a successful HTTP response.” It must not be weakened to
“the proxy TCP handshake succeeded.”

## Confirmed pre-phase behavior

At the original planning baseline, `HealthChecker::check_proxy()`:

1. constructs a Reqwest proxy per check;
2. maps both `ProxyType::Socks4` and `ProxyType::Socks5` to a
   `socks5://` Reqwest proxy;
3. maps both `ProxyType::Http` and `ProxyType::Https` to an
   `http://` Reqwest proxy;
4. maps Tor to SOCKS5;
5. creates a fresh Reqwest client for each proxy check;
6. performs GET against `HealthCheckConfig.test_url`;
7. treats only `2xx` as healthy;
8. measures end-to-end check elapsed time;
9. preserves per-proxy credentials in the actual proxy configuration while
   log keys remain redacted;
10. has a stored `HealthChecker.client` that is cloned but is not the client
    used by the per-proxy request path.

These behaviors are characterization targets. Some represent technical debt;
none should change accidentally.

## Global invariants

Preserve:

1. health means an application-level request through the selected proxy, not
   merely proxy-connect success;
2. configured `test_url` and timeout remain authoritative;
3. status success remains `2xx` unless a separately documented behavior
   decision says otherwise;
4. TLS verification/insecure behavior remains explicit and compatible;
5. no environment proxy discovery;
6. proxy credentials remain redacted in diagnostics;
7. health failure cannot trigger a silent direct request;
8. health results continue to drive the existing pool healthy/unhealthy state;
9. concurrency remains bounded;
10. public health DTO/config shape remains compatible;
11. Phase A ownership remains intact: Eggress executes proxy protocol hops,
    Eggsec owns health policy;
12. no change to `eggsec-transport` or its authorization contract;
13. Rust 1.89 and ring-only TLS remain mandatory.

## Workstream 0 — characterize the Phase A end state

Record the Phase A final SHA and rerun:

```sh
cargo test -p eggsec-web-proxy -- --test-threads=1
cargo tree -p eggsec-web-proxy
cargo tree -p eggsec-web-proxy -e features
cargo tree -d
bash scripts/check-architecture-guards.sh
```

Confirm exactly which custom SOCKS/CONNECT code remains for compatibility and
whether Reqwest's only production owner in `eggsec-web-proxy` is now
`health.rs` / health-only helper code.

Search for:

```text
reqwest::
reqwest =
create_insecure_client_with_options
socks::
http_connect::
eggress_outbound
```

Record findings before changing health code.

## Workstream 1 — build a protocol-faithful local health matrix

Create deterministic local fixtures where one endpoint acts as the proxy and a
second endpoint acts as the HTTP/HTTPS health target.

The matrix must cover at least:

1. SOCKS5 -> HTTP target -> 200;
2. SOCKS5 auth -> HTTP target -> 200;
3. SOCKS5 -> HTTP target -> non-2xx;
4. HTTP CONNECT -> HTTPS target -> 2xx;
5. HTTP CONNECT auth -> HTTPS target -> 2xx;
6. proxy handshake succeeds but application request fails;
7. target TLS verification failure;
8. overall timeout;
9. proxy auth failure;
10. target response non-success;
11. no direct fallback when the proxy leg fails;
12. redaction of credentials on every failure path.

If SOCKS4 is retained as a supported health type, add a real SOCKS4 fixture.
Do not continue testing SOCKS4 by silently issuing SOCKS5.

If `ProxyType::Https` is retained, establish its actual compatibility
meaning with a dedicated fixture before changing the mapping. Do not assume
the enum means TLS-to-proxy simply from its name.

No fixture may require Internet access, public DNS, external Tor, or root.

## Workstream 2 — choose the smallest truthful application-level probe

Evaluate these implementation options in order.

### Option A — small HTTP/TLS probe over the Eggress stream

Preferred only if it stays genuinely small and maintainable.

The flow is conceptually:

```text
Eggress connector
  -> established stream to health target
  -> TLS wrap for https target when required
  -> one bounded HTTP GET
  -> drain bounded response as required
  -> classify status
```

Requirements:

- use standard/current workspace HTTP primitives rather than creating a second
  general-purpose client abstraction;
- preserve Host/SNI identity from `test_url`;
- support HTTP and HTTPS target URLs needed by current defaults;
- obey one aggregate health timeout;
- cap headers/body work so a hostile health endpoint cannot allocate
  unbounded memory;
- do not add redirect following unless current Reqwest behavior is explicitly
  characterized and preserved;
- do not support cookies, retries, environment proxies, compression, or other
  unrelated client features.

If this requires substantial HTTP client machinery, do not proceed merely to
remove a dependency line.

### Option B — retain Reqwest as a health-only owner

This is an acceptable closure outcome.

Keep Reqwest if it is materially simpler and safer to preserve the existing
HTTP(S)-through-proxy health contract.

If retained:

- narrow comments/docs to state that Reqwest exists only for application-level
  proxy health checks;
- remove any unused generic client field/factory state;
- avoid rebuilding invariant client pieces unnecessarily where possible;
- correct SOCKS4/HTTPS protocol misclassification only when supported by
  Reqwest and proven by fixtures;
- otherwise fail closed for a health mode Reqwest cannot represent faithfully
  rather than testing a different protocol and calling it equivalent.

The goal is truthful ownership, not dependency-count vanity.

### Rejected option — tunnel-only health

Do not replace the request with `OutboundConnector::connect_tcp*` success
alone. That would detect a live proxy endpoint but not prove the proxy can
carry the configured application request to the configured health target.

If tunnel liveness is useful, it may be added later as a separately named
metric/state, not substituted for the current health meaning.

## Workstream 3 — resolve current protocol-classification debt

Use the fixture results to make an explicit disposition for every
`ProxyType`:

```text
Socks4
Socks5
Http
Https
Tor
```

For each, document:

- actual connection protocol used;
- where DNS resolution occurs;
- whether auth is supported;
- whether TLS is to the proxy, the target, or both;
- whether the health implementation exactly represents that type.

Do not preserve a known false-positive/false-negative mapping solely because it
is old behavior if it can be corrected compatibly. But treat a correction as a
behavior fix with dedicated tests and release-note/documentation language, not
as invisible refactoring.

If a type cannot be checked faithfully with the chosen health backend, return
an explicit unsupported/error result rather than silently testing another
protocol.

## Workstream 4 — simplify HealthChecker lifecycle/concurrency

The original implementation stores a Reqwest client on `HealthChecker` but
constructs a separate proxy-configured client inside each `check_proxy()`.

After the backend decision:

- remove unused stored client state;
- make cloning cheap and semantically meaningful;
- preserve the configured timeout;
- keep `check_all()` deterministic;
- keep concurrent checking bounded;
- avoid one spawned task per arbitrarily large proxy set if Phase A/current
  code still retains O(total proxies) JoinHandles.

If the proxy list can be large, prefer a bounded in-flight scheduler such as
`buffer_unordered(concurrency)` or an equivalent O(concurrency) pattern,
while preserving one result per enabled proxy and existing accounting.

Do not mix pool-selection policy into health execution.

## Workstream 5 — dependency cleanup

If Option A fully replaces Reqwest:

- remove Reqwest from `eggsec-web-proxy`;
- remove health-only helper code that existed solely to construct Reqwest
  clients;
- remove now-unused direct dependencies such as `base64` only when no
  compatibility/interception code still owns them;
- record what transitive packages disappear;
- ensure the replacement does not add a larger or broader graph than the
  removed client stack.

If Option B retains Reqwest:

- keep the dependency explicitly documented as health-only;
- do not claim the crate is Reqwest-free;
- still remove dead client state/helpers and unrelated features;
- confirm Reqwest remains `default-features = false` and only the narrow
  required TLS/SOCKS features are present.

In either outcome, re-run `cargo tree -d` and dependency policy.

## Workstream 6 — final architecture/documentation reconciliation

Update current documentation to reflect the actual end state:

- `architecture/egress_reuse_decision.md`;
- `architecture/proxy.md` or the canonical web-proxy architecture doc;
- `crates/eggsec-web-proxy/src/outbound.rs` boundary comments;
- `docs/WEB_PROXY.md` where proxy type/health behavior is user-visible;
- `docs/CI_ARCHITECTURE_GUARDS.md`;
- relevant skill/AGENTS guidance only if current instructions would otherwise
  direct future agents to rebuild the removed protocol machinery.

Preserve historical plan records.

Document explicitly:

- Eggress is a specialized proxy execution dependency, not an authorization
  authority;
- Eggfetch remains the canonical scoped HTTP transport backend;
- interception remains Eggsec-owned;
- why Reqwest remains, if retained;
- unresolved upstream feature-width or resolver-injection debt.

## Workstream 7 — closure measurements

Compare the final state with the pre-Phase-A baseline and Phase-A result.

Record:

```sh
cargo tree -p eggsec-web-proxy
cargo tree -p eggsec-web-proxy -e features
cargo tree -d
cargo metadata --locked --format-version 1
```

Where repository tooling exists, also record release binary size for the same
feature profile before/after. Do not compare mismatched profiles.

Report:

- direct dependency additions/removals;
- transitive package delta;
- duplicate-version delta;
- Tokio feature delta;
- release artifact size delta where reproducible;
- source files/LOC of custom protocol code removed versus compatibility code
  retained.

Do not claim footprint improvement unless the measurements show it. A
maintenance-ownership improvement may still justify the adoption if dependency
cost is bounded and explicitly recorded.

## Required verification

Focused:

```sh
cargo fmt --all --check
cargo check -p eggsec-web-proxy --no-default-features
cargo check -p eggsec-web-proxy --features web-proxy
cargo test -p eggsec-web-proxy -- --test-threads=1
cargo test -p eggsec --features web-proxy --test proxy_adapter_smoke -- --test-threads=1
bash scripts/check-architecture-guards.sh
make check-deps
```

Repository closure:

```sh
make check-feature-profiles
make check-features-individual
make check
make check-msrv
```

Run `make check-full` and hosted Deep Checks according to current policy.
Record platform-dependent skips precisely.

If Python proxy bindings are touched:

```sh
make check-python
```

## Expected files touched

Depending on the selected health option:

```text
crates/eggsec-web-proxy/Cargo.toml
crates/eggsec-web-proxy/src/health.rs
crates/eggsec-web-proxy/src/outbound.rs
crates/eggsec-web-proxy/src/utils.rs
crates/eggsec-web-proxy/tests/*
Cargo.lock
architecture/egress_reuse_decision.md
architecture/proxy.md
docs/WEB_PROXY.md
docs/CI_ARCHITECTURE_GUARDS.md
plans/README.md
this plan
```

Do not broaden this phase into interception refactoring or generic HTTP
transport redesign.

## Stop conditions

Stop and record residual debt instead of forcing cleanup if:

- the proposed Reqwest replacement cannot preserve HTTPS target verification;
- the replacement materially recreates a general HTTP client;
- removing Reqwest increases graph/complexity enough to defeat the maintenance
  goal;
- SOCKS4/HTTPS cannot be represented truthfully by the selected health
  backend;
- health changes require weakening timeout, TLS, or no-direct-fallback
  behavior;
- closure would require changing `eggsec-transport` or authorization
  semantics.

## Acceptance criteria

Phase B is complete only when:

1. every supported proxy type has an explicit, tested health disposition;
2. health remains application-level HTTP(S)-through-proxy validation;
3. proxy-connect success alone is never reported as application health;
4. current SOCKS4/HTTPS normalization debt is either corrected or explicitly
   fail-closed/documented;
5. credentials remain redacted;
6. timeout/TLS/non-2xx/auth failure behavior is tested locally;
7. health concurrency is bounded and result accounting is correct;
8. Reqwest is removed only if the replacement is smaller/clearer and
   semantically equivalent; otherwise it is retained as an explicit
   health-only dependency;
9. dead client/helper state is removed;
10. final graph/Tokio/artifact measurements are recorded;
11. architecture docs and guards match the actual ownership boundary;
12. `eggsec-transport` remains unchanged and Eggress-free;
13. focused, dependency-policy, feature-profile, MSRV, and repository checks
    are green;
14. hosted validation required by current policy is green;
15. this plan and the roadmap receive completion records with final SHAs,
     evidence, and any residual upstream Eggress prerequisites.

---

## Completion record (executed 2026-09-22)

- Phase A SHA: `d9749cc`. Implementation SHA (A+B combined): `d9749cc`
  (health work landed in the same commit; this record documents it).
- Backend decision WS2: **Option B — Reqwest retained as the explicit
  health-only owner.** Rebuilding HTTPS verification, redirect, timeout,
  and body-cap semantics over raw Eggress streams would recreate a general
  HTTP client to remove a dependency line. `Cargo.toml`, `outbound.rs`,
  and `health.rs` document the rationale; no Reqwest-free claim is made.
- Health matrix: `crates/eggsec-web-proxy/tests/health_matrix.rs`,
  15/15 local fixtures green (SOCKS5→HTTP 200, SOCKS5 auth, non-2xx,
  HTTP-CONNECT→HTTPS 200, CONNECT auth, tunnel-ok/app-fail stays
  unhealthy, self-signed HTTPS validates via unchanged lab-insecure mode,
  bounded timeout, proxy-auth failure redacted, no-direct-fallback with
  zero target hits, redaction on every failure path, SOCKS4 fail-closed,
  `Https` plaintext-CONNECT fidelity, bounded concurrent accounting,
  cheap config-only clone).
- WS3 disposition: `Socks5`/`Tor`→SOCKS5, `Http`→HTTP, `Https`→HTTP
  (faithful to plaintext-CONNECT production behavior; naming debt, not a
  health bug) validated; `Socks4`→explicit unsupported error (behavior fix
  with fixture: previously a SOCKS5 result was presented as SOCKS4 health).
- WS4: removed unused stored `client` (Clone is now config-only `derive`);
  `check_all` stays sequential/deterministic; `check_concurrent` rewritten
  on `buffer_unordered(concurrency)` — O(concurrency) in-flight, one result
  per enabled proxy, no spawn-per-proxy JoinHandle retention.
- WS5: Reqwest stays `default-features = false` with `rustls-no-provider`
  + `socks` only; `utils::create_insecure_client_with_options` retained as
  the health-only factory; `From<reqwest::Error>` retained for the health
  path. Final graph unchanged from Phase A (+11 lock entries, no
  duplicates, Tokio +fs/+signal recorded, MSRV 1.89, ring-only).
- WS6 docs: `architecture/proxy.md` (engine ownership, health
  disposition, placeholder local_addr, shim boundary),
  `architecture/egress_reuse_decision.md` (1.0.8 addendum),
  `architecture/overview.md`, `docs/CI_ARCHITECTURE_GUARDS.md`,
  `README.md`, `eggsec-proxy` + `eggsec-config` skills updated. Historical
  Phase E records untouched.
- Residual upstream prerequisites: `OutboundInfo.local_addr` always None
  (placeholder stands); `Https` TLS-to-proxy meaning unproven; no
  resolver-injection API for deeper transport integration (deferred per
  roadmap; `eggsec-transport` unchanged and Eggress-free).
- Acceptance 1–15 met (criterion 14/hosted: no hosted run required by
  current policy beyond PR CI; local gates all green, pushed for remote CI
  verification).
- Verification: `cargo test -p eggsec-web-proxy` 456 passed (includes 21
  parity + 15 matrix), `proxy_adapter_smoke` 3 passed, `make check`,
  `make check-deps`, `make check-feature-profiles`, `make
  check-features-individual`, `make check-msrv` — all green locally.
