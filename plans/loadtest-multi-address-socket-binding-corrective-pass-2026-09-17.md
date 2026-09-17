# Multi-address socket-binding corrective pass

Status: Planned

Date: 2026-09-17

Eggsec baseline: `ea41641866b53a5c2ea7151482b32c71c1275706`

Depends on: `plans/loadtest-authorization-transport-corrective-pass-2026-09-16.md`

## Purpose

Close the remaining physical-route authorization defect in the production
`eggsec-transport-eggfetch` load-test backend without reopening the broader
transport migration or changing Eggsec's policy model.

The 2026-09-17 load-test authorization/transport pass correctly removed the
wildcard execution scope, bound strict execution to `ApprovedExecution`, made
Reqwest construction fail closed, added proxy-peer checkpoints, and moved
production load-test traffic to the pinned Eggfetch adapter. A follow-up audit
found one narrower contract violation in the new proxied multi-address route.

`NetworkAuthority` defines the socket checkpoint as authorization of the
address actually dialed. The Eggfetch adapter currently validates a set of DNS
candidates, calls `authorize_socket()` / `authorize_proxy_socket()` for only
`binding.primary()`, but then hands the complete approved candidate set to
Eggfetch through `Proxy::resolved_addresses(...)` and
`RequestBuilder::proxy_target_addresses(...)`. If the first candidate cannot be
used, the backend may attempt another approved-DNS candidate that never passed
the selected-socket checkpoint.

The canonical Eggsec `ScopeAuthority` currently applies the same scope rules to
the whole approved set, so this is not evidence of a present wildcard-scope
bypass. It is still a real transport-contract defect: custom/stricter
authorities may make a narrower decision at the socket checkpoint, and the
backend must not be allowed to dial an address Eggsec did not authorize as the
actual socket choice.

This pass makes physical binding exact again. The immediate correction is to
hand the backend one fully socket-authorized address per leg. Multi-address
retry/failover is deliberately not implemented here; if added later, Eggsec
must own or observe each candidate attempt and re-run the corresponding socket
checkpoint immediately before that attempt.

## Confirmed baseline defect

### Direct routes are already exact

For direct hostname traffic, `authorize_hop()` chooses `binding.primary()`,
calls `authority.authorize_socket(host, binding.primary(), port)`, and rewrites
the wire URL to that exact IP literal. The Eggfetch connector therefore sees a
single physical origin address. Preserve this behavior.

### Proxied routes broaden after the socket checkpoint

For a proxied hop, the current adapter performs the right semantic stages:

1. resolve and authorize the ultimate destination candidate set;
2. create a validated ultimate binding and call
   `authorize_socket(..., binding.primary(), ...)`;
3. resolve and authorize the proxy endpoint candidate set;
4. create a validated proxy binding and call
   `authorize_proxy_socket(..., proxy_binding.primary(), ...)`.

However, the resulting `AuthorizedProxyRoute` is populated from the full
approved vectors rather than the selected bindings:

```rust
let proxy_peers: Vec<SocketAddr> = proxy_approved
    .iter()
    .map(|ip| SocketAddr::new(*ip, proxy_port))
    .collect();

let ultimate_peers: Vec<SocketAddr> = approved
    .iter()
    .map(|ip| SocketAddr::new(*ip, port))
    .collect();
```

Those vectors are then supplied to Eggfetch as:

```rust
proxy.resolved_addresses(proxy_route.proxy_peers.clone())
builder.proxy_target_addresses(proxy_route.ultimate_peers.clone())
```

This is wider than the final selected-socket authorization. A backend retry to
candidate N is not equivalent to `authorize_socket(candidate N)`.

### Connection metadata can become untruthful

For proxied responses, `map_response()` currently reports the first proxy peer
as `ConnectionInfo.remote_addr`. If Eggfetch falls back internally to a later
proxy address, that field no longer describes the actual peer and can
misrepresent evidence/audit output.

The narrow single-address correction below removes both problems at once.

## Non-goals

Do **not** use this pass to:

- create a new workspace crate;
- add another scope/policy language;
- move authorization into Eggfetch;
- restore Reqwest as an automatic production fallback;
- add transparent multi-address retries without per-attempt authorization;
- broaden SOCKS5H or plaintext HTTP forward-proxy support;
- enable HTTP/3;
- redesign load-test scheduling, pacing, metrics, or UI behavior;
- change the `ApprovedExecution` / mandatory-scope work that already landed;
- make Eggfetch expose Eggsec-specific concepts.

## Required invariant

For every physical connection leg, every socket address that the concrete
backend may dial must have passed the matching selected-socket checkpoint.

In the current contract this means:

```text
direct origin:
  backend dial set == { address passed to authorize_socket }

proxy endpoint:
  backend proxy-peer dial set == { address passed to authorize_proxy_socket }

proxied ultimate target:
  backend ultimate-target dial set == { address passed to authorize_socket }
```

A DNS-approved set is an input to selection. It is **not** permission for the
backend to choose any member after Eggsec has performed a narrower socket
checkpoint.

## Workstream 0 — Lock the regression before changing routing

Add a deterministic adversarial test that fails on the baseline.

The fixture must model at least two addresses for one physical leg:

- candidate A is first/primary and passes the selected-socket checkpoint but
  cannot complete the connection;
- candidate B is reachable (or otherwise would allow observable progress) but
  must not be dialed unless it separately passes the selected-socket
  checkpoint;
- the authority records every `authorize_socket` /
  `authorize_proxy_socket` invocation.

Prefer testing the **proxy peer** leg because that is where the current adapter
passes a multi-address `resolved_addresses` set directly to Eggfetch. If a
fully deterministic proxy fixture is materially harder, an adapter-local test
hook may inspect the authorized route object under `#[cfg(test)]`, but the final
suite still needs at least one end-to-end fixture proving no unauthorized
fallback reaches a socket/server.

Add the analogous ultimate-target assertion where practical. The test should
make it impossible to pass by merely checking DNS authorization calls.

Required assertions:

- candidate B receives no network traffic;
- candidate B is never reported as an authorized socket unless the authority
  was actually called for B;
- a failure of candidate A fails the request rather than silently falling back
  to B;
- no direct-route fallback occurs;
- recorded connection metadata never claims an address different from the
  single authorized backend route.

Do not use public DNS or external network services. Use loopback/local fixtures
and deterministic resolvers.

## Workstream 1 — Narrow proxied backend route sets to the selected binding

In `crates/eggsec-transport-eggfetch/src/adapter.rs`, construct proxy and
ultimate backend pin sets from the exact selected bindings that immediately
passed their socket checkpoints.

Expected shape:

```rust
let proxy_peer = SocketAddr::new(proxy_binding.primary(), proxy_port);
authority.authorize_proxy_socket(proxy_host, proxy_binding.primary(), proxy_port)?;

let ultimate_peer = SocketAddr::new(binding.primary(), port);
// `binding.primary()` has already passed `authorize_socket` for this hop.

AuthorizedProxyRoute {
    ...
    proxy_peers: vec![proxy_peer],
    ultimate_peers: vec![ultimate_peer],
    ...
}
```

Exact spelling may differ, but preserve these semantics:

- `proxy_peers` contains only the address passed to
  `authorize_proxy_socket()`;
- `ultimate_peers` contains only the address passed to `authorize_socket()`;
- the selected address comes from the validated binding, not by re-indexing the
  unvalidated resolver result;
- no later helper expands the vectors from the DNS-approved set;
- redirects re-resolve/re-authorize and create a fresh single-address route for
  the new hop;
- direct routes remain pinned to their single selected IP literal.

If the route type becomes clearer as singular fields (`proxy_peer`,
`ultimate_peer`) rather than one-element vectors, that refactor is acceptable
provided the Eggfetch call boundary converts them to the API shape without
reintroducing widening. Prefer the representation that makes an accidental
multi-address expansion hardest.

## Workstream 2 — Keep connection evidence truthful

Review `AuthorizedHop` / `AuthorizedProxyRoute` and `map_response()` after the
single-address change.

For direct traffic, `ConnectionInfo.remote_addr` remains the selected ultimate
socket.

For proxied traffic, `ConnectionInfo.remote_addr` should describe the exact
proxy peer Eggsec authorized and supplied as the only backend proxy route. Do
not infer an unobserved alternate address from the original DNS candidate set.

If Eggfetch 0.1.5 exposes authoritative connected-peer metadata, it may be used
only if it is stable and does not add a broader dependency/API requirement.
That is optional for this pass: single-address backend pinning is sufficient to
make the existing metadata truthful because the connector has no authorized
alternate.

Add/adjust a test asserting the reported proxied peer equals the socket-bound
address.

## Workstream 3 — Preserve route semantics and fail-closed unsupported cases

Re-run and extend the existing proxy route matrix after narrowing the pin sets.
The correction must not weaken the qualified behavior:

| Route | Required result |
|---|---|
| direct HTTP/HTTPS | supported; one authorized ultimate address |
| HTTP/HTTPS proxy -> HTTPS origin (CONNECT) | supported; one authorized proxy peer + one authorized ultimate target |
| SOCKS5 local-resolution -> HTTP/HTTPS | supported; one authorized proxy peer + one authorized ultimate target |
| SOCKS5H remote-resolution | explicit `Proxy` denial before origin dispatch |
| HTTP forward proxy -> plaintext HTTP | explicit `Proxy` denial before origin dispatch |

Keep `without_proxy()` on direct requests and keep automatic Reqwest fallback
forbidden.

Credential handling, SNI/Host preservation, redirect credential stripping,
and H1/H2 behavior are unchanged by this pass and must remain covered by the
existing parity suite.

## Workstream 4 — Decide multi-address retry policy explicitly

Do not preserve backend-internal multi-address fallback merely for
availability. Record the current production rule:

> One authorization cycle selects one physical address per connection leg. If
> that address fails, the request fails. A retry may select another candidate
> only after a fresh authorization cycle and selected-socket checkpoint.

If future resilience work needs address failover, design it above the opaque
backend retry layer. Acceptable future patterns include:

1. Eggsec iterates authorized DNS candidates and invokes
   `authorize_socket`/`authorize_proxy_socket` immediately before each attempt;
2. a transport/backend callback allows Eggsec to authorize each concrete
   socket choice before dial;
3. a backend API returns control on connect failure so Eggsec can reselect and
   reauthorize.

Do not implement those patterns in this corrective pass unless required to
preserve an existing tested public guarantee. Correctness takes precedence over
silent address failover.

## Workstream 5 — Durable tests and guard

Update the local adapter/engine regression suite with the narrowest tests that
prove the invariant rather than implementation spelling.

Expected test surfaces:

- `crates/eggsec-transport-eggfetch/tests/parity.rs`
  - multi-address proxy peer cannot fall through to an un-socket-authorized
    secondary;
  - proxied ultimate route is narrowed to the selected socket-authorized
    address;
  - proxied `ConnectionInfo.remote_addr` is the selected proxy peer;
  - existing SOCKS5H/plaintext failures remain closed.
- `crates/eggsec/tests/loadtest_authorization_regression.rs`
  - add an engine-level regression only if needed to prove canonical
    `OwnedScopeAuthority` composition; do not duplicate adapter mechanics for
    coverage count.

Architecture guard: add one only if it can encode the security boundary
without brittle source matching. A useful guard may reject construction of
Eggfetch proxy pin vectors by iterating the full `approved` / `proxy_approved`
sets in the adapter. If the guard would merely grep a specific variable name,
prefer the executable adversarial fixture instead.

## Workstream 6 — Documentation and prior-pass closure

Update the documentation so it no longer implies that a DNS-approved
multi-address set is equivalent to a fully socket-pinned route.

At minimum review:

- `architecture/transport_eggfetch.md`;
- `architecture/transport.md`;
- `architecture/loadtest.md`;
- `.opencode/skills/eggsec-loadtest/SKILL.md`;
- `crates/eggsec/src/loadtest/AGENTS.override.md` if its production backend
  description needs the invariant called out;
- `docs/CI_ARCHITECTURE_GUARDS.md` if a durable guard is added.

Also close the documentation gap in
`plans/loadtest-authorization-transport-corrective-pass-2026-09-16.md`:
append the actual completion record requested by that plan rather than leaving
only `Status: Executed`. Include its final SHA, scope-propagation result,
Reqwest fail-closed result, Eggfetch 0.1.5 selection, route matrix, MSRV 1.89,
Python parity, local verification, hosted CI run IDs, and a note that this
follow-up corrected the multi-address selected-socket binding discovered during
post-landing audit.

Do not rewrite the historical intent of the prior plan; append factual closure
information.

## Verification

Run targeted tests first:

```bash
cargo test -p eggsec-transport-eggfetch --test parity
cargo test -p eggsec --no-default-features --test loadtest_authorization_regression
cargo test -p eggsec --no-default-features --test transport_contract
```

Then run the repository contract required by `AGENTS.md`:

```bash
make check
make check-deps
make check-msrv
```

If Python files are touched while closing the prior completion record or
parity, also run:

```bash
make check-python
```

Hosted CI and code-quality checks must be green on the final implementation
commit before this plan is marked executed. Record run IDs/results in the
completion record.

## Acceptance criteria

This pass is complete only when all of the following are true:

1. Direct origin traffic still exposes exactly one backend-dialable address,
   and it is the address passed to `authorize_socket()`.
2. A proxied route exposes exactly one backend-dialable proxy peer, and it is
   the address passed to `authorize_proxy_socket()`.
3. A proxied route exposes exactly one backend-dialable ultimate target, and it
   is the address passed to `authorize_socket()`.
4. The backend cannot silently fail over from a failed selected address to a
   second DNS-approved but socket-unchecked address.
5. Redirects construct a new per-hop binding and cannot reuse a prior hop's
   physical route without fresh authorization.
6. `ConnectionInfo.remote_addr` is truthful for direct and proxied routes under
   the single-address binding rule.
7. SOCKS5H and plaintext HTTP forward-proxy cases remain fail closed.
8. Proxy credentials, Host/SNI behavior, redirect credential stripping, and
   H1/H2 parity are not regressed.
9. No Reqwest/direct automatic fallback is reintroduced.
10. No new crate or Eggsec-specific Eggfetch API is introduced.
11. An adversarial multi-address regression fails on the baseline and passes
    after the correction.
12. The prior corrective pass has a factual completion record, including CI
    evidence and the follow-up defect note.
13. Targeted transport/load-test tests, `make check`, dependency policy, MSRV
    verification, and hosted CI are green.

## Expected files touched

Likely implementation surface:

```text
crates/eggsec-transport-eggfetch/src/adapter.rs
crates/eggsec-transport-eggfetch/tests/parity.rs
crates/eggsec/tests/loadtest_authorization_regression.rs      # only if engine-level proof adds value
architecture/transport_eggfetch.md
architecture/transport.md                                    # if contract wording needs clarification
architecture/loadtest.md
.opencode/skills/eggsec-loadtest/SKILL.md
crates/eggsec/src/loadtest/AGENTS.override.md                 # if guidance changes
scripts/check-architecture-guards.sh                          # only for a durable non-brittle guard
docs/CI_ARCHITECTURE_GUARDS.md                               # iff guard added
plans/loadtest-authorization-transport-corrective-pass-2026-09-16.md
this plan
```

Do not mechanically touch every listed file. Use the smallest set that proves
and documents the invariant.

## Handoff order

Implement in this order:

1. Add the adversarial multi-address regression and prove it fails on
   `ea41641866b53a5c2ea7151482b32c71c1275706`.
2. Narrow proxy-peer and ultimate-target backend pin sets to their selected,
   socket-authorized bindings.
3. Make connection metadata singular/truthful and add the assertion.
4. Re-run the existing direct/CONNECT/SOCKS5/SOCKS5H/plaintext route matrix.
5. Add a durable architecture guard only if it is semantic enough to survive
   harmless refactors.
6. Update transport/load-test documentation and agent skill guidance.
7. Append the missing factual completion record to the prior corrective plan.
8. Run targeted tests, full repository checks, MSRV/dependency gates, and
   hosted CI.
9. Append this plan's completion record and change `Status: Planned` to
   `Status: Executed` only after the final SHA and hosted checks are known.

## Completion record template

Append after execution:

- baseline SHA / final SHA;
- exact baseline reproduction showing unauthorized secondary fallback or
  backend route widening;
- selected-address representation chosen (`SocketAddr` singular vs one-element
  vectors) and rationale;
- proxy peer candidate set, selected binding, socket checkpoint, and final
  backend pin evidence;
- ultimate target candidate set, selected binding, socket checkpoint, and final
  backend pin evidence;
- adversarial multi-address fixture result;
- `ConnectionInfo.remote_addr` evidence for direct and proxied traffic;
- route-matrix regression results (direct / CONNECT / SOCKS5 / SOCKS5H /
  plaintext forward proxy);
- confirmation that no automatic Reqwest/direct fallback exists;
- exact targeted test commands/results;
- `make check`, `make check-deps`, and `make check-msrv` results;
- hosted CI/code-quality run IDs and conclusions;
- prior-plan completion-record update;
- residual debt, especially the explicit absence/policy of multi-address
  failover.
