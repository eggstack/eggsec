# Phase A — Dependency baseline and network-policy invariants

Status: Executed (2026-09-12)

Date: 2026-09-11

Depends on: none

## Purpose

Create a reproducible dependency/security baseline before changing transport implementations, and convert the current network authorization assumptions into executable invariants. This phase is measurement and guard work only; do not migrate HTTP clients yet.

## Workstream 1 — Artifact-specific dependency baseline

Capture the dependency graph for the actual supported artifacts/profiles rather than only the workspace union. At minimum record:

```text
cargo tree -p eggsec --no-default-features
cargo tree -p eggsec-cli
cargo tree -p eggsec-cli --features full
cargo tree -p eggsec-agent
cargo tree -p eggsec-nse --features nse
cargo tree -p eggsec-web-proxy --features web-proxy
cargo tree -d
cargo tree -e features
cargo tree -i reqwest
cargo tree -i rustls
cargo tree -i tokio-rustls
cargo tree -i hickory-resolver
cargo tree -i openssl
cargo tree -i native-tls
```

If a profile requires native prerequisites, run it in the same provisioned environment as Deep Checks and distinguish source/build failures from missing host packages.

Record for each artifact:

- direct dependency count;
- duplicate-version families from `cargo tree -d`;
- HTTP/TLS/DNS/proxy implementation owners;
- native dependencies and build scripts;
- default and explicitly enabled Cargo features;
- release binary size for `eggsec-cli` where practical;
- clean build/check wall-clock time only as supporting evidence, not a hard acceptance threshold.

Commit a concise machine-readable or Markdown baseline under `docs/architecture/` or another retained documentation location. Do not put generated full tree dumps in `plans/`.

## Workstream 2 — Inventory concrete client leakage

Mechanically inventory every use of:

```text
reqwest::
rustls::
tokio_rustls::
hickory_resolver::
lookup_host
Client::builder
RequestBuilder
```

Classify each use as:

- ordinary outbound HTTP;
- TLS/interception server behavior;
- raw protocol/networking behavior;
- compatibility-only behavior;
- process-host/integration behavior.

Explicitly flag APIs where a concrete library type crosses a domain boundary. `auth_context::apply_auth_context_to_request(reqwest::RequestBuilder, ...)` is a known example and must be included.

The inventory becomes the migration checklist for Phases B-D. Add an architecture guard only for durable forbidden-boundary patterns, not for every implementation detail.

## Workstream 3 — Pin outbound authorization semantics

Locate the canonical scope/authorization types established by the completed dependency and architecture roadmaps. Do not invent a second scope model.

Add or consolidate tests proving the canonical policy behavior for outbound network requests. At minimum cover:

1. authorized hostname and authorized resolved IP succeeds;
2. authorized hostname resolving only to an out-of-scope IP is rejected before connection;
3. mixed authorized/unauthorized DNS answers do not permit connecting to an unauthorized address;
4. DNS re-resolution on a later connection/retry is rechecked;
5. same-origin redirect inside scope succeeds when redirect following is enabled;
6. redirect to a different in-scope origin is separately authorized;
7. redirect to an out-of-scope host/IP is rejected before dispatch;
8. URL userinfo is rejected or handled according to the existing strict policy and never leaks to diagnostics;
9. cross-origin redirect does not forward authorization/cookie/proxy-authorization secrets;
10. direct IP targets are checked without a DNS bypass;
11. proxy use does not turn authorization of the proxy endpoint into authorization of an arbitrary target, or vice versa; document the two distinct authorization decisions;
12. insecure-TLS mode changes certificate verification only and never changes target authorization.

Use local deterministic DNS/HTTP fixtures or injectable fakes. No tests should require Internet access.

## Workstream 4 — Define migration parity matrix

Create a retained parity table for all Reqwest behavior EggSec actually relies on. Include, as applicable:

- HTTP methods and arbitrary headers;
- request bodies/form/query/JSON;
- streaming and maximum body handling;
- cookies;
- redirects and redirect limit;
- HTTP/1.1 and HTTP/2 requirements;
- SOCKS/HTTP proxies and `NO_PROXY` semantics;
- per-request and connect/read/write/pool timeout behavior;
- TLS roots/custom trust/insecure mode/client identity/SNI overrides where used;
- compression/decompression and anti-decompression-bomb limits;
- blocking call sites;
- auth-context header/cookie injection;
- tracing/metrics requirements;
- retry behavior and replayability;
- connection metadata required by scanners.

For each row record: current owner, required behavior, Eggfetch support, known semantic difference, and migration disposition (`direct`, `adapter`, `upstream prerequisite`, `remain specialized`).

Do not require Eggfetch feature parity with unused Reqwest features merely because they are currently enabled in Cargo.

## Workstream 5 — Baseline security policy state

Record the current state of:

- `deny.toml` advisory/license/ban/source policy;
- `.cargo/audit.toml` ignore set;
- Cargo.lock source kinds (registry/git/path);
- Actions refs and workflow permissions;
- whether Dependabot/Renovate or equivalent automated dependency/action updates exist;
- which dependency/security checks run on pull requests versus scheduled/manual workflows.

This is the input to Phase F.

## Required verification

```text
cargo metadata --locked --format-version 1
cargo tree -d
cargo tree -e features
cargo deny check
make check
make check-python
make test-architecture-guards
```

Run the new scope/redirect/DNS regression tests explicitly as well.

## Acceptance criteria

1. A retained per-artifact dependency baseline exists.
2. Every direct Reqwest/Rustls/DNS owner is classified.
3. Concrete-client type leakage across EggSec domain boundaries is enumerated.
4. Scope enforcement tests cover DNS resolution, redirect hops, re-resolution, direct IPs, and proxy semantics.
5. A behavior parity matrix determines which Eggfetch capabilities are ready and which need upstream work.
6. Current Cargo Deny/Audit/Actions/update policy is recorded without changing policy yet.
7. No HTTP client migration occurs in this phase.

## Expected files touched

- dependency/architecture documentation;
- focused scope/network-policy tests and local fixtures;
- possibly `scripts/check-architecture-guards.sh` for durable concrete-client boundary checks;
- this plan completion record.
