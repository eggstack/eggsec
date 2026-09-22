# Eggress 1.0.8 Phase A — proxy protocol engine adoption

Status: Ready for handoff

Date: 2026-09-22

Depends on:
[eggress-1.0.8-adoption-roadmap-2026-09-22.md](eggress-1.0.8-adoption-roadmap-2026-09-22.md)

Eggsec implementation baseline:
`a81ae04219996d4db50b5d417b0e7e74a3666f8d`

Upstream release:

- `eggress v1.0.8`;
- release commit `f0affac49c0fdaf6bcb51dfe1cb47f1f6548ffed`;
- consume published crates only;
- candidate crates: `eggress-outbound = "=1.0.8"` and
  `eggress-uri = "=1.0.8"`, both with no unnecessary optional features.

## Purpose

Move production HTTP CONNECT, SOCKS4/SOCKS5, and proxy-chain execution in
`eggsec-web-proxy` behind the new listener-free `eggress-outbound` API,
without changing Eggsec's public configuration, proxy-selection policy,
authorization boundary, interception stack, or health-check semantics.

This is the implementation phase that changes the 2026-09-13 Eggress
disposition. The old decision was valid for Eggress 1.0.6; the new
`eggress-outbound` crate in 1.0.8 supplies the specific deletion/consolidation
case that did not previously exist.

## Hard scope boundary

In scope:

- `crates/eggsec-web-proxy`;
- root/member manifests and lockfile;
- focused proxy tests/fixtures;
- architecture guards and current architecture documentation;
- any engine tests required to prove the domain crate still integrates
  correctly.

Out of scope:

- `eggsec-transport` implementation changes;
- `eggsec-transport-eggfetch` changes;
- interception listener/TLS/certificate/HTTP2/WebSocket/gRPC behavior;
- proxy pool and rotation replacement;
- health-probe implementation changes beyond adaptations strictly required to
  compile after low-level production migration;
- advanced Eggress protocols/features;
- public CLI/Python/MCP schema changes.

## Mandatory behavioral invariants

1. `ProxyConfig`, `ProxyEntry`, `ProxyType`, `RotationStrategy`,
   `ProxyPool`, and `ProxyRotator` remain Eggsec-owned.
2. Existing selection order, weights, priority, health eligibility, and
   failure accounting remain unchanged.
3. A selected proxy failure must return an error; no Eggress direct fallback.
4. Credentials must not appear in Debug/Display/error text, panic text, or
   evidence.
5. Existing connection timeouts remain effective around the complete
   establishment operation.
6. Cancellation remains future-drop safe; do not spawn detached dial tasks.
7. `create_connection(target)` retains its existing local-resolution and
   private/internal-target rejection behavior before handing the final
   destination to Eggress. Pass the already selected IP literal to Eggress
   where needed to avoid silently converting this path to remote DNS.
8. The existing explicit domain-resolution path for SOCKS5/Tor must retain its
   remote-domain behavior or fail closed; do not accidentally make it local
   DNS merely because an adapter is easier.
9. SOCKS4 remains IP-targeted on paths where the current implementation
   requires IP resolution. Do not silently reinterpret it as SOCKS4a.
10. `ProxyType::Https` semantics must be preserved until separately proven.
    Do not start TLS-to-proxy merely because Eggress can express it if current
    Eggsec behavior is plaintext CONNECT under that enum.
11. Interception direct-upstream `TcpStream::connect` paths are not part of
    this migration; they serve a different MITM/server contract.
12. No public API/type signature may change solely because Eggress returns
    `BoxStream`. Inventory public low-level helpers before deleting or
    changing them.
13. No `toml`, `pproxy-compat`, `udp`, `extended`, `ssh`, `quic`,
    `legacy-crypto`, `pproxy-legacy`, or `insecure-tls` Eggress feature
    is enabled in the initial integration.
14. Ring-only TLS policy and Rust 1.89 remain unchanged.

## Workstream 0 — freeze the pre-adoption evidence

Before edits, record:

```sh
git rev-parse HEAD
git status --short
cargo tree -p eggsec-web-proxy
cargo tree -p eggsec-web-proxy -e features
cargo tree -d
cargo check -p eggsec-web-proxy --no-default-features
cargo check -p eggsec-web-proxy --features web-proxy
cargo test -p eggsec-web-proxy -- --test-threads=1
bash scripts/check-architecture-guards.sh
```

Also record current compiled/package counts where the repository's prior
network-dependency plans did so.

Capture the current public low-level API surface in:

- `socks.rs`;
- `http_connect.rs`;
- `lib.rs::ProxyManager`.

Search the whole workspace for references. Distinguish:

- production internal call sites;
- tests;
- Python/TUI/engine wrappers;
- externally documented/public symbols.

Do not delete a public compatibility symbol merely because the workspace has
no internal caller.

## Workstream 1 — re-open the Eggress decision with 1.0.8 evidence

Update `architecture/egress_reuse_decision.md` as a current decision record
without erasing the 1.0.6 history.

The record must state:

- 1.0.6 URI/routing/wholesale-stack rejection remains historically valid;
- 1.0.8 added `eggress-outbound`, creating a new listener-free protocol
  execution seam;
- `eggress-routing`, `eggress-embed`, runtime/server, and advanced
  transports remain rejected for Eggsec;
- only the specialized `eggsec-web-proxy` edge is accepted;
- `eggsec-transport` remains independent because Eggress does not currently
  expose Eggsec's authorized-resolution binding seam.

Measure the published 1.0.8 graph rather than relying only on the sibling
workspace source tree.

Required comparison:

```sh
cargo tree -p eggsec-web-proxy
cargo tree -p eggsec-web-proxy -e features
cargo tree -d
```

before and after the candidate dependency.

Specifically record whether the published `eggress-outbound` dependency
widens Tokio to `fs`, `signal`, `rt-multi-thread`, or other features not
otherwise required by the isolated web-proxy package.

Do not hide this widening. If it is judged unacceptable for Eggsec's
capability-minimization policy, stop the production dependency addition,
record the exact upstream prerequisite, and leave the phase blocked rather
than using a Git/path override.

## Workstream 2 — add only the narrow published dependencies

Preferred manifest shape:

```toml
eggress-outbound = { version = "=1.0.8", default-features = false }
eggress-uri = { version = "=1.0.8" }
```

Place dependencies at the narrowest owning crate. Do not make them root
workspace dependencies unless repository convention or multiple consumers
actually require it.

Requirements:

- exact 1.0.8 pin for the first qualification;
- no Git dependency;
- no sibling path dependency;
- no `[patch]`;
- no umbrella `eggress` facade;
- no `eggress-embed`;
- no optional Eggress feature not needed for HTTP/SOCKS TCP chains.

Run `cargo deny`/repository dependency policy immediately after lockfile
resolution. Record checksums/selected versions from `Cargo.lock`.

## Workstream 3 — implement one Eggsec-owned adapter boundary

Create a narrow internal module, e.g.
`crates/eggsec-web-proxy/src/eggress_outbound.rs` (name may vary).

Its responsibilities are only:

- convert one `ProxyEntry` into an Eggress `ProxyHopSpec`;
- convert an Eggsec-selected proxy vector into ordered `ProxyChainSpec`;
- construct `OutboundConnector::from_chain`;
- execute with the appropriate destination representation and timeout;
- map typed Eggress errors into `WebProxyError` without leaking
  credentials;
- expose only the stream/metadata shape needed by the existing Eggsec
  caller.

Do not move pool/rotation/health state into this adapter.

Prefer native structs:

```text
ProxyEntry
  -> ProtocolSpec
  -> EndpointSpec
  -> optional CredentialSpec
  -> ProxyHopSpec
  -> ProxyChainSpec
  -> OutboundConnector::from_chain()
```

Do not serialize a proxy URI and parse it back.

### Protocol mapping

Start from current behavior, not aspirational semantics:

```text
Socks4 -> Socks4
Socks5 -> Socks5
Tor    -> Socks5
Http   -> Http
Https  -> preserve current Eggsec behavior until audited
```

For `Https`, add a characterization test before selecting `tls=true`.
If current callers/config/docs demonstrably mean TLS to the proxy, correct the
implementation with an explicit compatibility note and test. Otherwise retain
the current CONNECT behavior and record the naming debt separately.

### Credentials

Convert credentials at the last possible boundary and keep plaintext lifetime
short. Tests must assert that representative username/password values do not
appear in:

- `Debug` of adapter state;
- mapped `WebProxyError`;
- typed Eggress errors;
- configuration/log keys already documented as redacted.

Do not log native `ProxyChainSpec` with unrestricted formatting unless its
redaction guarantee is explicitly verified.

## Workstream 4 — establish deterministic protocol parity fixtures

Before switching production call sites, build local-only fixtures for:

1. SOCKS5 no-auth success;
2. SOCKS5 username/password success;
3. SOCKS5 auth rejection;
4. SOCKS5 proxy-reported destination failure;
5. SOCKS5 domain-target behavior;
6. SOCKS5 IPv4 and IPv6 target encoding where supported;
7. SOCKS4 IP-target success;
8. SOCKS4 unsupported-domain behavior matching the chosen Eggsec path;
9. HTTP CONNECT success;
10. HTTP CONNECT Basic auth;
11. HTTP CONNECT 4xx/5xx failure;
12. bounded oversized/malformed proxy response behavior as applicable;
13. connect timeout;
14. cancellation/drop;
15. two-hop SOCKS5 chain ordering;
16. at least one mixed HTTP/SOCKS chain if Eggsec elects to expose the
    newly supported composition through its existing chain interface;
17. failure at hop 0 versus later-hop handshake produces stable, redacted
    diagnostics;
18. no route silently falls back direct.

The fixture should not require Internet access, root, external Tor, or a
manually managed proxy process.

If current behavior contains a bug that Eggress would change, record it as a
separate behavior correction. Do not disguise it as parity.

## Workstream 5 — migrate ProxyManager production execution

Route the production `ProxyManager` connection methods through the adapter
after the parity fixtures exist.

Preserve the semantic split:

### Locally resolved target path

For `create_connection(target)` and current chain paths that already call
Eggsec `resolve_target`:

- keep Eggsec resolution/private-address rejection;
- pass the selected IP + port to Eggress as the final target;
- do not hand the original hostname to Eggress and allow an independent
  final-target DNS lookup.

This is not the canonical `NetworkAuthority` path, but preserving the
existing resolution decision avoids a migration-induced semantic change.

### Explicit remote-domain path

For the existing SOCKS5/Tor domain path:

- preserve domain-at-proxy behavior;
- only protocols already supporting this path may use it;
- keep SOCKS4 rejection;
- add a fixture proving the proxy receives the domain rather than a locally
  resolved IP.

### Chaining

Use the Eggsec-selected ordered proxies to construct the Eggress chain.

Do not let Eggress perform route selection. Eggress executes the already
selected route.

The existing Eggsec chain API may remain SOCKS-only if that is part of its
public contract. Supporting mixed chains is optional in this phase and must not
silently broaden a public capability surface. The parity fixture may exercise
Eggress's mixed-chain ability internally without exposing it.

## Workstream 6 — handle stream typing without API regression

Use Eggress's generic async stream internally where the migrated path needs the
live connection.

Do not downcast or assume a `BoxStream` is always `TcpStream`.

Before changing any public function returning concrete `TcpStream`:

1. determine whether it is part of a published/documented compatibility
   surface;
2. search workspace/downstream examples;
3. preserve a compatibility wrapper when required.

Preferred end state:

- production `ProxyManager` and private execution paths use the Eggress
  generic stream;
- obsolete private raw-handshake helpers are deleted;
- public compatibility helpers, if any, are either delegated without semantic
  loss or retained/deprecated with a documented removal boundary.

Do not claim all duplicate code is removed if a public compatibility shim
still owns a separate handshake implementation.

## Workstream 7 — remove proven duplicate production machinery

After production parity passes:

- remove private SOCKS handshake/auth/error code no longer used;
- remove private HTTP CONNECT framing/parsing code no longer used;
- remove duplicate chain orchestration no longer used;
- remove now-unused `base64` or other dependencies only if no remaining
  web-proxy owner exists;
- retain Reqwest in this phase if health probes still require it;
- update `outbound.rs` comments so they no longer incorrectly claim custom
  SOCKS/HTTP CONNECT is the production reason to retain the specialized path.

Run `cargo machete` only if already part of repository tooling; otherwise use
Cargo tree/source searches and compiler evidence. Do not add tooling solely for
this cleanup.

## Workstream 8 — replace architecture guard Check 106

The old guard says no Eggress edge is ever allowed. Replace it with a
release-specific narrow boundary.

The new guard should enforce all of the following:

- `eggsec-web-proxy` may depend on the explicitly approved
  `eggress-outbound` / `eggress-uri` line;
- `eggsec-transport` may not depend on any Eggress crate;
- `eggsec-transport-eggfetch` may not depend on any Eggress crate;
- `eggsec-core`, `eggsec-policy`, runtime/daemon DTO layers, and unrelated
  domain crates may not gain Eggress;
- `eggress-embed`, `eggress-runtime`, `eggress-server`,
  `eggress-routing`, advanced protocol/transport crates remain forbidden;
- unexpected Eggress optional features remain forbidden;
- the current decision record exists and names 1.0.8.

Prefer manifest-aware checks where practical. Do not create a guard that
requires incidental source filenames.

Update `docs/CI_ARCHITECTURE_GUARDS.md` and relevant architecture index/docs.

## Required focused verification

```sh
cargo fmt --all --check
cargo check -p eggsec-web-proxy --no-default-features
cargo check -p eggsec-web-proxy --features web-proxy
cargo test -p eggsec-web-proxy -- --test-threads=1
cargo test -p eggsec --features web-proxy --test proxy_adapter_smoke -- --test-threads=1
cargo tree -p eggsec-web-proxy
cargo tree -p eggsec-web-proxy -e features
cargo tree -d
bash scripts/check-architecture-guards.sh
make check-deps
make check-feature-profiles
make check
make check-msrv
```

Run relevant Python tests only if the public Python proxy surface or generated
bindings/stubs change. A pure internal engine migration should not create
gratuitous Python churn, but existing proxy Python smoke coverage should still
be run if it transitively exercises the migrated behavior.

## Expected files touched

Likely:

```text
Cargo.lock
crates/eggsec-web-proxy/Cargo.toml
crates/eggsec-web-proxy/src/lib.rs
crates/eggsec-web-proxy/src/eggress_outbound.rs        # or equivalent
crates/eggsec-web-proxy/src/socks.rs                   # shrink/remove if safe
crates/eggsec-web-proxy/src/http_connect.rs            # shrink/remove if safe
crates/eggsec-web-proxy/src/outbound.rs
crates/eggsec-web-proxy/tests/*
crates/eggsec/tests/proxy_adapter_smoke.rs              # if needed
architecture/egress_reuse_decision.md
architecture/proxy.md                                   # if current
docs/CI_ARCHITECTURE_GUARDS.md
scripts/check-architecture-guards.sh
plans/README.md
this plan
```

Do not touch interception implementation files merely to make the migration
look broader.

## Stop conditions

Stop and record a blocker rather than weakening Eggsec when:

- published 1.0.8 cannot satisfy required HTTP/SOCKS parity;
- credentials appear in diagnostics;
- an Eggress failure falls back direct;
- preserving local-versus-remote DNS behavior is impossible;
- the published feature graph violates an accepted dependency/capability
  constraint and cannot be tolerated without an upstream release;
- adoption requires `eggress-embed`/runtime/server;
- implementation would require changing `eggsec-transport` authorization
  semantics.

## Acceptance criteria

Phase A is complete only when:

1. the 1.0.8 decision record is updated without falsifying the 1.0.6 history;
2. the published dependency graph is recorded and accepted;
3. only the specialized web-proxy crate owns the Eggress edge;
4. deterministic local protocol parity exists for the supported SOCKS/HTTP
   paths;
5. production ProxyManager execution uses Eggress for the migrated routes;
6. local-resolution and remote-domain semantics remain explicit;
7. no proxy failure silently becomes direct traffic;
8. credential redaction is proven;
9. timeouts/cancellation are proven;
10. duplicate private handshake/chain code is removed where compatibility
    permits;
11. public compatibility symbols are not broken accidentally;
12. Check 106 is rewritten as a narrow positive boundary;
13. Reqwest health behavior has not been silently altered;
14. focused tests, dependency policy, feature profiles, `make check`, and
    MSRV validation are green;
15. the plan contains a completion record with exact final SHA and residual
    debt for Phase B.
