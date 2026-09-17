# Load-test authorization and transport corrective pass

Status: Executed

Date: 2026-09-16

Eggsec baseline: `a497009bb29a18dca3432f0b59bb83882ef2039f`

Depends on: completed crate-boundary consolidation Phases A-D

Sibling research snapshot:

- `eggstack/eggfetch` latest published release at research time: `v0.1.4`
  (2026-09-13; release target `475bd50f6f9b9f66b95eea814f06b4adb22ede93`).
- Eggfetch `main` at research time: `b199b96e0f1cac1e8d7ec24c7e310a2b265d11d2`.
- Eggfetch proxy-route pinning implementation: `e8260cd472c70edd87663191c6d984e0c7fabab8`.
- Eggfetch proxy-route qualification/closure is complete; the later reusable
  route-cache hardening includes `1f52d846c186b061481ebb14a7be414f5c78ec7e`.
- The proxy-route pinning APIs described below are present on Eggfetch `main`
  but are **not** in the latest published `v0.1.4`. Do not add a floating Git
  dependency to consume them.

## Purpose

Close the residual authorization and physical-route gaps exposed by the Phase D
load-test refactor without reopening the crate-boundary roadmap or creating a
new load-test/resilience crate.

Phase D successfully separated load-test planning/execution/metrics/progress
from its concrete HTTP backend, but the compatibility facade still synthesizes
a wildcard scope when no scope is supplied. Several programmatic execution
paths therefore lose the scope that was used to approve the operation before
constructing the transport authority. The new Reqwest transport also contains
fallbacks that can silently change redirect/proxy/TLS intent when client
construction fails, and Reqwest cannot bind its internal DNS/socket selection
to the exact address set Eggsec authorized.

The completed `eggsec-transport-eggfetch` adapter already provides an
approved-IP-pinned direct route and should become the first production backend
for load testing once protocol/performance parity is proven. Eggfetch has also
implemented and qualified pinned proxy peers and pinned CONNECT/SOCKS5 ultimate
destinations on `main`; full proxy migration is therefore a release/integration
problem, not a reason to reproduce that routing machinery in Eggsec.

This pass is primarily a security-correctness pass. Dependency reduction is a
secondary benefit.

## Confirmed current state

### 1. CLI load testing already carries an explicit scope

`crates/eggsec/src/commands/handlers/load.rs` evaluates/enforces the operation
and calls `loadtest::run_cli_with_scope(..., ctx.scope.clone())`. Preserve this
shape.

### 2. The load-test facade defaults to wildcard authorization

`crates/eggsec/src/loadtest/runner.rs` currently creates:

```rust
fn default_facade_scope() -> Scope {
    let mut scope = Scope::new();
    scope.allowed_targets.push(ScopeRule::new("*".to_string()));
    scope
}
```

`LoadTestRunner::new*()` stores that scope, and ordinary `run()` builds an
`OwnedScopeAuthority` from it. This preserves pre-Phase-D behavior but defeats
the transport seam's fail-closed purpose whenever a caller forgets to attach
the real execution scope.

### 3. Confirmed strict/programmatic paths currently lose scope

At the baseline:

- `tool/implementations/loadtest.rs` constructs a runner and calls
  `runner.run()` without attaching the scope that approved the tool request.
- `dispatch/network.rs::run_load_test()` constructs a runner, reads
  `runner.scope()`, and therefore receives the wildcard facade scope.
- `dispatch/canonical_execution.rs::execute_approved()` verifies the
  `ApprovedOperation` target/tool binding, then calls canonical execution
  without carrying the `EnforcementContext`'s scope snapshot to the domain
  executor.
- `ApprovedOperation` intentionally proves operation/target binding but does
  not currently carry an execution-scope snapshot.
- `SecurityTool::execute()` has no execution-context parameter, so the strict
  `EnforcedDispatcher` can validate an approval token and still lose the
  corresponding network scope when it invokes the tool implementation.

Outer approval of the initial target is not sufficient. Redirects,
re-resolution, socket selection, and proxy routing must use the same scope
snapshot that authorized the operation.

### 4. Reqwest load-test backend contains semantic fallbacks

`crates/eggsec/src/loadtest/backend.rs` currently:

- falls back from a failed verified-client build to `reqwest::Client::new()`;
- falls back from a failed insecure-client build to the verified client;
- substitutes a placeholder proxy for an invalid proxy builder path;
- falls back from failed proxied-client construction to a **direct** base
  client;
- caches proxied clients by `(proxy endpoint, TLS verification)` only, even
  though proxy credentials are baked into the Reqwest `Proxy`/client and are
  therefore a connection/client-identity dimension.

The direct fallback for a requested proxy is a route-policy violation. The
verified-client fallback can also re-enable concrete-client defaults that the
manual redirect/authorization loop intentionally disabled. These paths must
return errors, not alternate transports.

### 5. Reqwest cannot close the origin DNS/socket binding invariant here

The backend resolves and authorizes an address set through
`TransportResolver`, but Reqwest subsequently resolves the logical hostname
inside its connector. The transport can call `authorize_socket()` for an
approved address, but it cannot prove that Reqwest actually dialed that exact
address.

This is already documented in `loadtest/backend.rs`; this pass removes the
known direct-route gap rather than treating the checkpoint calls as equivalent
to physical binding.

### 6. Proxy physical binding is incomplete in the shared contract

`NetworkAuthority::authorize_proxy(proxy_endpoint, ultimate)` authorizes the
logical proxy endpoint and ultimate destination as separate names, but the
contract exposes no proxy-peer DNS-result or selected-socket checkpoint.
`ScopeAuthority::authorize_proxy()` therefore performs hostname/port prechecks
and comments that full address checks occur at DNS/socket time, while the
Reqwest load-test transport never resolves/binds the proxy endpoint through the
Eggsec resolver/authority path. Reqwest resolves the proxy internally.

A strict proxied route needs two independent physical bindings:

```text
logical proxy endpoint -> approved physical proxy peer
logical ultimate origin -> approved physical ultimate peer (where enforceable)
```

Authorizing only the two logical URLs is not enough to close DNS rebinding.

### 7. Eggfetch now has the needed generic route-pinning model

Published Eggfetch `v0.1.4` includes caller-supplied direct static destinations.
The existing Eggsec adapter independently pins direct wire URLs to an approved
IP while preserving logical Host/SNI and manually authorizing redirects.

Eggfetch `main` additionally contains completed, qualified generic APIs for:

- `Proxy::resolved_addresses(...)`: pin physical peers for the proxy endpoint
  while retaining the logical proxy hostname/TLS identity;
- `RequestBuilder::proxy_target_addresses(...)`: pin the physical ultimate
  target for enforceable proxied routes;
- HTTPS via HTTP/HTTPS CONNECT with a pinned ultimate target;
- SOCKS5 local-resolution routes with a pinned ultimate target;
- explicit fail-closed rejection of pinned SOCKS5H and plaintext HTTP
  forward-proxy target pinning;
- route/cache keys that include physical pinning and TLS/connection identity.

The two Eggfetch pin sets are deliberately independent: a security-sensitive
caller must pin both proxy peer and ultimate target when both are under local
control.

### 8. Eggsec's declared MSRV is stale relative to the adapter dependency

Eggsec workspace `rust-version` is currently `1.88`. Eggfetch workspace/core
`v0.1.4` declares Rust `1.89`. Because `eggsec-transport-eggfetch` is a normal
workspace member depending on `eggfetch-core`, Eggsec's workspace-wide MSRV
claim is not truthful for the full workspace. Reconcile this pass to Rust 1.89
and add a mechanical MSRV check instead of leaving CI to succeed only because
it runs a newer compiler.

## Security invariants

Every workstream must preserve these invariants:

1. No production load-test execution may synthesize `allowed_targets = ["*"]`
   because the caller omitted an execution context.
2. The scope snapshot used for per-hop `NetworkAuthority` checks must be the
   same policy snapshot that authorized the operation, not a newly loaded
   config or a caller-selected wider scope.
3. `ApprovedOperation` target/tool binding remains mandatory; carrying scope
   to execution supplements approval binding and never replaces it.
4. Missing execution scope fails closed before network I/O.
5. Client/transport initialization failure may never broaden redirect, proxy,
   TLS, DNS, or socket policy.
6. A requested proxy may never fall back to direct routing.
7. A hostname route counted as physically scoped must dial only an address
   that was in the resolver snapshot and approved by `NetworkAuthority`.
8. Proxy endpoint and ultimate destination remain distinct authorization and
   physical-routing decisions.
9. Redirects re-run logical authorization and physical binding for each new
   origin; an old address snapshot is never reused for a cross-origin hop.
10. SOCKS5H/remote-DNS and plaintext HTTP forward-proxy routes must not be
    described as physically pinned when Eggsec cannot constrain the ultimate
    peer.
11. `eggsec-policy` remains free of transport/runtime/network implementation
    dependencies. Execution-context composition belongs in the engine.
12. `eggsec-transport` remains implementation-neutral. Eggfetch stays below
    the adapter; Eggsec policy never moves into Eggfetch.
13. No unreleased/floating sibling Git dependency is introduced to bypass the
    manual release process.

## Non-goals

- No new workspace crate.
- No reopening `eggsec-loadtest`, `eggsec-resilience`, or `eggsec-net`.
- No universal redesign of all tool execution APIs beyond the minimum context
  seam required to preserve strict-surface scope.
- No new offensive/testing capability.
- No attempt to make SOCKS5H or arbitrary HTTP forward proxies falsely satisfy
  local ultimate-IP binding.
- No Eggfetch-specific policy language in `eggsec-transport`.
- No package publication from this implementation plan; sibling/library
  releases remain manual maintainer actions.

## Workstream 0 — Freeze the baseline and add failing regressions first

Before changing behavior, inventory every production call of:

```text
LoadTestRunner::new*
LoadTestRunner::from_*
LoadTestRunner::run*
loadtest::run_cli*
dispatch::network::run_load_test
LoadTestTool::execute
execute_canonical / execute_approved
EnforcedDispatcher::dispatch_checked
```

Classify each caller as:

- manual CLI;
- TUI/manual runtime;
- strict REST/MCP/gRPC/agent/tool surface;
- daemon/runtime execution;
- Python/library call;
- test/benchmark only.

Record where its authoritative `LoadedScope` / `Scope` originates. Do not infer
scope from target strings or reload a config as a substitute for the approval
context.

Add regression tests that fail on the baseline and pass after Workstream 1:

1. approved load-test target A redirects to out-of-scope B -> B listener sees
   no request;
2. DNS for approved hostname returns an out-of-scope address after approval ->
   transport denies before dispatch;
3. canonical `execute_approved` load-test path uses the approval context's
   scope rather than wildcard scope;
4. strict tool-dispatch load-test path uses the same scope;
5. a load-test facade constructed without scope cannot call ordinary `run()`;
6. explicit `run_with(transport, authority, ...)` remains usable for tests and
   callers that intentionally supply their own authority.

Use local deterministic fixtures and recording fakes. No public DNS is needed.

Also add backend regressions before Workstream 2 for:

- requested proxy cannot become a direct request after builder failure;
- invalid/unsupported proxy construction produces an error before origin I/O;
- two credential sets for one proxy endpoint cannot reuse a client carrying
  the other credential;
- verified/insecure client construction has no policy-changing fallback.

## Workstream 1 — Carry approval scope into execution

### Preferred engine boundary: `ApprovedExecution`

Introduce an engine-owned execution bundle (name may vary) conceptually:

```text
ApprovedExecution
  approved: ApprovedOperation
  scope: Scope              # snapshot from the same EnforcementContext
```

Construction must occur through the engine `EnforcementContext` so a strict
caller cannot pair a valid approval token with an unrelated broader scope.
Preferred API shapes are:

```text
EnforcementContext::approve_execution(...)
EnforcementContext::approve_manual_execution(...)
```

The pure `eggsec-policy::ApprovedOperation` remains the operation/target token.
Do not add `HttpTransport`, `NetworkAuthority`, DNS resolvers, or other engine
objects to the policy crate.

If implementation finds an existing engine execution-context type that can
carry the same private pair without duplication, extend that instead. The
required property is one construction point tying token + scope snapshot to the
same enforcement context.

### Canonical execution path

Thread the execution bundle/context through `execute_approved()` and the
load-test branch of canonical execution.

A useful shape is:

```text
execute_approved(ApprovedExecution, request, sink)
    -> verify approval binding
    -> build engine ExecutionContext
    -> load-test branch receives explicit Scope/NetworkAuthority
```

Keep domain executors policy-free where possible. The load-test executor only
needs a network authority/scope input; it does not need the entire policy
engine.

If `execute_canonical()` remains as an unapproved/manual primitive, it must not
silently run load testing without an execution scope. Either:

- add a context-aware canonical variant and make the uncontextual load-test arm
  return a clear error; or
- require all load-test canonical calls to use an explicit scoped helper.

Do not restore a wildcard for legacy compatibility.

### Tool-dispatch path

`SecurityTool::execute(ToolRequest)` currently cannot receive execution scope.
Use the smallest compatibility-preserving context seam. Preferred pattern:

```text
ToolExecutionContext { scope: Scope, ...narrow future-safe fields only }

SecurityTool::execute_with_context(request, context)
    default -> execute(request)       # unaffected tools
```

Then:

- `EnforcedDispatcher` invokes the context-aware path from the
  `ApprovedExecution` bundle;
- `LoadTestTool` overrides the context-aware method and passes the scope to the
  runner/executor;
- raw `LoadTestTool::execute()` without context fails closed rather than using
  wildcard scope;
- other tools need no mechanical rewrite merely to add this seam.

Routing the load-test tool through an existing canonical execution service is
also acceptable if it removes rather than duplicates code. Do not add scope as
an arbitrary JSON field on `ToolRequest`, do not use a process-global
"current scope", and do not trust a frontend to pair token + scope manually.

### Load-test facade

Replace stored `Scope` defaulting with one of:

```text
scope: Option<Scope>
```

or a constructor that requires scope for ordinary production execution.

Required behavior:

- `run()` / `run_with_cancellation()` without scope -> deterministic error
  before transport creation or I/O;
- `.with_scope()` / `.set_scope()` remains available if source compatibility
  matters;
- `run_with(transport, authority, ...)` stays explicit and does not consult a
  stored wildcard;
- `default_facade_scope()` is deleted;
- no production `ScopeRule::new("*")` replacement appears elsewhere.

Update deprecation/source-compatibility documentation rather than preserving an
unsafe default.

## Workstream 2 — Make the temporary Reqwest backend fail closed

Even if Reqwest becomes a fallback/transition backend, it must not change route
semantics on construction errors.

### Client construction

Change constructors to return a result:

```text
ReqwestTransport::new(...) -> Result<Self, TransportError>
ReqwestTransport::with_system_resolver() -> Result<Self, TransportError>
```

Required changes:

- verified client build failure -> error;
- insecure client build failure -> error (do not silently turn an explicitly
  insecure request into another mode; callers may retry/configure explicitly);
- preserve manual redirect disabling, retry assumptions, pool settings, and
  TLS intent exactly or fail;
- remove `reqwest::Client::new()` semantic fallback.

### Proxy construction

Make `client_for()` return `Result<reqwest::Client, TransportError>`.

- invalid proxy endpoint -> error; no `127.0.0.1:1` placeholder;
- proxy-client build failure -> error; never `base_client()` direct fallback;
- proxy credentials remain redacted from logs/errors;
- no direct network attempt may occur after a proxied route failed to build.

### Proxy cache identity

The current `(endpoint, verified)` cache key is insufficient because credentials
are embedded in the concrete Reqwest proxy/client configuration.

Either remove the cross-request proxied-client cache in the transition path or
make its key include every connection/client-affecting dimension, at minimum:

```text
proxy endpoint
proxy routing mode/scheme
TLS verification policy
proxy credential identity (without Debug/log exposure)
```

Do not key by a redacted constant. Do not emit credential material into error,
Debug, tracing, or metrics labels. A plan-scoped proxy client constructed once
per immutable load-test request template is preferable to a growing generic
secret-bearing cache if it is simpler and keeps connection reuse.

Add a local authenticated-proxy regression proving request B cannot inherit
request A's proxy credential.

## Workstream 3 — Extend the transport contract for physical proxy-peer facts

Before claiming any proxy path is scope-bound, extend `eggsec-transport` so the
transport—not the authority—resolves the proxy endpoint and supplies those
facts for policy.

Prefer additive checkpoint methods to changing existing ultimate-destination
methods, for example:

```text
NetworkAuthority::authorize_proxy_resolved(proxy_host, candidates)
NetworkAuthority::authorize_proxy_socket(proxy_host, selected, port)
```

Exact names may vary. Preserve source compatibility for third-party/custom
authorities with safe default implementations that delegate to the existing
resolved/socket policy where appropriate. `ScopeAuthority` should override or
map errors so proxy-route denials retain useful proxy provenance.

Required sequence for a proxy endpoint:

```text
logical proxy URL authorization
  -> resolve proxy host through TransportResolver
  -> authorize proxy candidate set
  -> validate_binding(proxy candidates, approved subset)
  -> authorize selected proxy socket
  -> give only approved physical peer(s) to concrete backend
```

This is independent from ultimate-origin resolution. Both bindings must be
recorded/tested separately.

Update the recording fake and parity tests so a transport that calls only
`authorize_proxy()` but never supplies proxy DNS/socket facts cannot satisfy the
strict proxy fixture.

Do not make `NetworkAuthority` perform DNS I/O.

## Workstream 4 — Make Eggfetch the production direct load-test backend

The existing `eggsec-transport-eggfetch::EggfetchTransport` already performs
per-hop scope checks and binds the backend to an approved IP by using a pinned
wire destination while preserving logical Host/SNI. Phase C parity tests cover
that contract, but the adapter currently has no production consumers.

Use it for `ProxyIntent::Direct` load-test requests.

### Protocol parity gate

The adapter currently builds Eggfetch with HTTP/1 only, while the Reqwest
backend is compiled with HTTP/2 support. Do not regress load-test protocol
behavior silently.

Before switching production direct traffic:

1. enable the minimal Eggfetch HTTP/2 feature set needed for H1/H2 ALPN;
2. use `HttpVersionPolicy::Auto { allow_http3: false }` (or equivalent current
   API) so HTTP/2-capable targets negotiate H2 and H1-only targets retain H1;
3. keep HTTP/3 disabled until `eggsec-transport` has an explicitly authorized
   QUIC-resolution/binding story;
4. add local H1 and H2 fixture tests proving redirects, Host/SNI, approved IP,
   response mapping, and connection reuse remain correct.

### Static-routing implementation choice

Published Eggfetch `v0.1.4` also exposes direct `resolved_addresses()` routing,
but that API intentionally builds an isolated resolved-route client so a stale
ordinary-DNS connection cannot be reused. For load-test throughput, do not swap
the adapter's existing pinning strategy merely for API aesthetics without
measurement.

Compare:

- current Eggsec adapter's pinned-wire/SNI path;
- Eggfetch `resolved_addresses()` direct path;
- Phase D Reqwest baseline fixture.

Measure at representative concurrency (at least 1, 10, 50, and a higher local
level such as 100 where stable):

```text
requests/sec
p50/p95/p99 client-observed latency
connection accepts / reuse count from fixture
CPU signal if practical
```

Choose the Eggfetch route that simultaneously preserves exact approved-address
binding, logical TLS/Host identity, and reusable-connection behavior. If
Eggfetch needs an upstream reusable resolved-route cache keyed by physical
route identity to meet load-test semantics, record that as a sibling
prerequisite rather than weakening pinning or rebuilding a second pool in
Eggsec.

No automatic fallback to Reqwest is permitted after an Eggfetch direct-route
error. Backend choice is configuration/composition, not an error recovery path.

## Workstream 5 — Gate full proxy migration on a published Eggfetch route-pinning release

At implementation time, query the latest published `eggfetch-core` release.
Proceed with full proxy migration only if the published crate contains the
qualified APIs/semantics now present on Eggfetch `main`:

- `Proxy::resolved_addresses(...)` (physical proxy peer pinning);
- per-request `proxy_target_addresses(...)` (physical ultimate target pinning);
- route/cache identity including proxy-peer/target pin state;
- the post-implementation proxy-route qualification fixes.

Record the exact Eggfetch version, crate checksum/lock resolution, release SHA,
and the upstream executable/qualification lineage in this plan's completion
record.

Do **not** use `git = ... branch = "main"` or an unpinned Git dependency to
unblock the phase. If no qualifying release exists, complete the Eggsec
security fixes and leave proxied load testing fail-closed with a clear
unsupported/prerequisite error. That is an acceptable closure state; manual
package release can enable the follow-up without leaving an unsafe fallback.

### Supported physical-route matrix once the release exists

Implement/test at least:

| Route | Proxy peer pin | Ultimate pin | Disposition |
|---|---|---|---|
| direct HTTP/HTTPS | n/a | required | supported via Eggfetch |
| HTTP/HTTPS proxy -> HTTPS origin (CONNECT) | required | required | supported |
| SOCKS5 local-resolution -> HTTP/HTTPS | required | required | supported |
| SOCKS5H remote-resolution | required | cannot be locally enforced | fail closed for strict scoped transport |
| HTTP forward proxy -> plaintext HTTP | required | standard proxy cannot enforce requested IP | fail closed for strict scoped transport |

For proxy-supported cases the adapter must:

1. resolve/authorize/bind the proxy endpoint through the new proxy-peer
   checkpoints;
2. configure Eggfetch `Proxy` with only approved peer addresses;
3. independently resolve/authorize the ultimate origin;
4. configure only approved proxied-target addresses;
5. preserve logical proxy TLS identity and logical origin Host/SNI;
6. re-run route authorization on redirects; cross-origin pinned-route reuse
   fails closed unless a new authorized snapshot is constructed.

A proxy endpoint pin does not authorize the ultimate origin. An ultimate pin
does not authorize the proxy endpoint.

### Transition behavior

Do not retain Reqwest as an automatic "proxy fallback" after Eggfetch is
selected. If a route shape is not physically enforceable, return an explicit
unsupported/policy error. This makes capability truthfulness visible to CLI,
TUI, daemon, Python, and agent surfaces instead of silently weakening scope.

## Workstream 6 — Reconcile MSRV and dependency/version policy

Because `eggfetch-core v0.1.4` already declares Rust 1.89, raise Eggsec's
workspace `rust-version` from `1.88` to `1.89` unless a fresh Cargo/MSRV check
proves the adapter dependency can build on 1.88 (expected result: it cannot).

Update:

- root `Cargo.toml`;
- CI/toolchain/MSRV jobs or scripts;
- package/release documentation;
- architecture/dependency docs that state the baseline;
- any binary installer/build docs that mention an older compiler.

Add a mechanical check that compiles at least the dependency-light leaves,
`eggsec-transport-eggfetch`, and the default engine profile on Rust 1.89. The
workspace may of course continue testing stable/newer compilers as well.

When upgrading Eggfetch for proxy pinning, prefer an explicit compatible
minimum version and commit the resulting lockfile. Do not use a broad version
change without recording which newly required APIs justify it.

## Workstream 7 — Surface/API parity and documentation cleanup

After the security path is stable:

- expose `LoadTestResults.error_kinds` in the Python load-test result model as
  an additive field, preserving older serialized inputs/defaults;
- update load-test architecture docs with explicit execution-scope ownership
  and backend route matrix;
- update transport docs with proxy-peer DNS/socket checkpoints;
- update `architecture/overview.md` so the 20-crate table includes
  `eggsec-transport-eggfetch` explicitly;
- update the Eggfetch adapter description from "no production consumers yet"
  once direct load testing uses it;
- update relevant `.opencode/skills` / `AGENTS.md` guidance so new load-test
  entry points require execution context rather than facade defaults.

Do not combine unrelated scanner/fuzzer/feature work into this pass.

## Workstream 8 — Add durable architecture/security guards

Extend the existing architecture guard suite with rules that encode semantics,
not incidental line numbers.

At minimum guard:

1. no production `default_facade_scope()` / wildcard load-test scope helper;
2. strict/canonical load-test execution must carry an explicit execution scope
   or approved execution context;
3. raw load-test tool execution without context fails closed;
4. Reqwest proxy path contains no direct-client fallback and no placeholder
   proxy substitution;
5. proxied client cache (if retained) cannot be keyed only by endpoint + TLS
   verification while credentials live on the client;
6. direct production load testing is wired through
   `eggsec-transport-eggfetch` after Workstream 4;
7. unsupported remote-DNS/forward-proxy pinning combinations remain explicit
   failures rather than weaker backend fallbacks;
8. workspace Rust version remains compatible with the selected
   `eggfetch-core` version.

Prefer Rust tests/cargo metadata for semantic behavior and use grep guards only
for simple forbidden fallbacks/wildcard constructors.

## Required tests

### Scope propagation

- CLI explicit scope remains green.
- canonical approved load test cannot reach a redirect target outside the
  approval scope.
- strict tool dispatcher cannot reach an out-of-scope redirect/re-resolution.
- missing execution scope fails before DNS/network I/O.
- explicit caller-supplied `NetworkAuthority` path remains supported.
- approval target mismatch still fails before execution.

### Direct route binding

- authorized DNS candidate -> exactly that socket is observed by fixture;
- mixed allowed/disallowed DNS answer denies;
- re-resolution to new unauthorized address denies;
- same-host redirect re-resolves and rebinds;
- cross-host redirect requires separate authorization;
- H1 and H2 logical Host/SNI remain correct under a pinned physical IP;
- connection reuse never crosses incompatible physical-route identity.

### Reqwest transition backend

- client build/configuration errors return errors, never altered defaults;
- invalid proxy never results in direct origin connection;
- proxy build failure never results in direct origin connection;
- proxy credential A cannot be reused for credential B;
- insecure/verified mode does not silently switch on construction failure.

### Proxy physical binding

With a qualifying Eggfetch release:

- unresolvable logical proxy + pinned local proxy peer succeeds without proxy
  DNS fallback;
- proxy listener observes only an approved peer connection;
- HTTPS CONNECT observes approved ultimate IP while origin TLS SNI/Host remain
  logical;
- SOCKS5 target command uses approved ultimate IP;
- SOCKS5H + target pin fails before target dispatch;
- plaintext HTTP forward proxy + ultimate pin fails before origin dispatch;
- retries/redirects preserve only compatible route snapshots;
- two different physical pin sets cannot cross-reuse a cached tunnel/client.

## Performance/regression gate

Load testing measures transport behavior, so backend migration itself must be
measured rather than assumed neutral.

Use deterministic loopback fixtures for H1 and, where practical, H2. Record:

```text
backend/version
request count
concurrency
RPS
p50/p95/p99
accepted TCP connections
redirect/proxy mode
CPU/memory signal if collected
```

Do not accept a performance win that bypasses authorization or physical
pinning. If safe Eggfetch direct routing has a material regression caused by
route/client reuse, fix the generic upstream reuse boundary or record a sibling
blocker; do not fall back to unbound Reqwest DNS.

## Required verification

Run focused suites first, then the repository gates.

```text
cargo fmt --all -- --check
cargo check -p eggsec-transport
cargo test -p eggsec-transport
cargo check -p eggsec-transport-eggfetch
cargo test -p eggsec-transport-eggfetch
cargo check -p eggsec --no-default-features
cargo test -p eggsec --lib loadtest
cargo test -p eggsec --test network_policy_invariants
cargo test -p eggsec --test enforced_dispatch_regression
cargo check -p eggsec-cli
cargo check -p eggsec-tui
cargo check -p eggsec-daemon
cargo check -p eggsec-python
cargo check --workspace --no-default-features
make check-feature-profiles
make check-features-individual
make test-architecture-guards
make check
make check-python
```

After the MSRV update, run the repository's supported equivalent of:

```text
cargo +1.89 check -p eggsec-transport
cargo +1.89 check -p eggsec-transport-eggfetch
cargo +1.89 check -p eggsec --no-default-features
```

If a qualifying Eggfetch proxy release is consumed, run its relevant direct,
proxy-peer, proxied-target, retry, redirect, TLS, and route-cache tests at the
recorded release SHA/version as sibling evidence; Eggsec still needs its own
adapter/integration fixtures.

## Acceptance criteria

1. No production load-test path manufactures wildcard scope when scope is
   absent.
2. Strict approval and per-hop network authorization use one enforcement-scope
   snapshot carried through an engine-owned execution context.
3. `ApprovedOperation` binding remains mandatory and cannot be paired with a
   caller-selected wider scope through the normal strict API.
4. Missing load-test execution scope fails before network I/O.
5. Reqwest client/proxy construction has no semantic fallback to default or
   direct routing.
6. Proxy credential/client cache identity cannot reuse another request's proxy
   credentials.
7. Direct load-test hostname traffic is physically pinned to approved
   addresses through the Eggfetch transport adapter.
8. Direct H1/H2 behavior is retained or any intentional protocol difference is
   explicitly documented and accepted; HTTP/3 remains out of scope.
9. Proxy endpoint DNS/socket facts have explicit transport checkpoints and are
   separately bound from the ultimate destination.
10. Full proxied route support uses a published, qualified Eggfetch route-
    pinning release; otherwise proxied load tests fail closed with the sibling
    prerequisite recorded.
11. SOCKS5H and plaintext forward-proxy routes are not falsely reported as
    ultimate-IP-pinned.
12. Eggsec's declared MSRV is truthful for `eggsec-transport-eggfetch` and the
    default engine profile (expected baseline: Rust 1.89).
13. Python/load-test result parity includes `error_kinds` or the completion
    record documents a concrete binding blocker.
14. Architecture guards encode the no-wildcard/no-route-fallback invariants.
15. Full feature/frontend/daemon/Python and hosted CI gates remain green.
16. No new workspace crate is added.

## Expected files touched

Likely Eggsec surface:

```text
Cargo.toml
Cargo.lock
crates/eggsec/Cargo.toml
crates/eggsec/src/config/policy_decision.rs
crates/eggsec/src/dispatch/canonical_execution.rs
crates/eggsec/src/dispatch/network.rs
crates/eggsec/src/loadtest/{runner,backend,mod}.rs
crates/eggsec/src/tool/{traits,dispatcher}.rs
crates/eggsec/src/tool/implementations/loadtest.rs
crates/eggsec-transport/src/{policy,request,...}.rs
crates/eggsec-transport-eggfetch/Cargo.toml
crates/eggsec-transport-eggfetch/src/{adapter,mapping,...}.rs
crates/eggsec-python/                    # result parity as needed
crates/eggsec/tests/                     # scope/dispatch/network regressions
architecture/{loadtest,overview,...}.md
docs/VERIFICATION.md
docs/CI_ARCHITECTURE_GUARDS.md
scripts/check-architecture-guards.sh
AGENTS.md / relevant .opencode skills
this plan completion record
```

Do not mechanically touch every listed file; use the smallest set that closes
the invariants.

## Handoff order

Implement in this order:

1. Workstream 0 regressions/inventory.
2. Workstream 1 execution-scope propagation and wildcard removal.
3. Workstream 2 Reqwest fail-closed corrections.
4. Workstream 3 proxy-peer transport contract.
5. Workstream 4 Eggfetch direct production migration + protocol/performance
   qualification.
6. Workstream 5 proxy release gate and supported-route migration or explicit
   fail-closed blocker.
7. Workstream 6 MSRV/dependency policy reconciliation (may be prepared earlier,
   but final selected Eggfetch version must be known before closure).
8. Workstreams 7-8 parity/docs/guards.
9. Full local + hosted verification and completion record.

Workstreams 1-4 do not require an unreleased Eggfetch proxy API and should not
be delayed waiting for a sibling release.

## Completion record template

Append after execution:

- baseline SHA / final SHA;
- all production load-test entry points and their execution-scope source;
- exact `ApprovedExecution` / tool-context API chosen and why;
- wildcard/default-scope removal evidence;
- Reqwest fallback/cache changes and adversarial test results;
- `NetworkAuthority` proxy-peer contract additions;
- direct Eggfetch adapter version/features + H1/H2 parity evidence;
- direct backend before/after performance measurements;
- Eggfetch proxy release version/SHA if consumed, or explicit release blocker
  and fail-closed route behavior if not;
- supported proxy route matrix and DNS non-fallback evidence;
- Eggsec MSRV before/after and exact 1.89 verification commands;
- Python result/API parity changes;
- architecture guard additions;
- exact local commands/results;
- hosted CI run IDs/results;
- residual debt and deliberately unsupported route shapes.
