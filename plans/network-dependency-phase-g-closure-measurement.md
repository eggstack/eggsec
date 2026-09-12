# Phase G — Verification, measurement, and closure

Status: Ready for handoff

Date: 2026-09-11

Depends on: Phases A-F

## Purpose

Close the roadmap with reproducible evidence that dependency consolidation and crate segregation improved the supported artifacts without weakening scope enforcement, TLS policy, feature compatibility, or frontend/runtime invariants.

This phase is reconciliation and measurement. Do not introduce new architecture here except to correct a defect exposed by closure testing.

## Workstream 1 — Re-run the Phase A dependency baseline

Using the same commands/profiles captured in Phase A, regenerate the final graphs and compare:

- direct dependency counts per supported artifact;
- duplicate-version families;
- Reqwest presence and inverse paths;
- Rustls/Tokio-Rustls ownership;
- DNS resolver ownership;
- OpenSSL/native-tls presence and feature isolation;
- Tokio enabled feature sets per artifact;
- native build dependencies;
- `eggsec-cli` release binary size;
- clean build/check timing as secondary evidence.

A successful result is not defined as “fewest crates.” It is defined as fewer duplicated implementation owners, narrower capability reach, and no unjustified second HTTP/TLS stack in normal artifacts.

Any remaining direct `reqwest`, `rustls`, `tokio-rustls`, `webpki-roots`, `hickory-resolver`, `openssl`, or `native-tls` owner must appear in the final decision record with its reason and feature/artifact scope.

## Workstream 2 — Prove network authorization invariants end-to-end

Run the final scope-aware Eggfetch adapter through deterministic integration fixtures covering:

- allowed DNS resolution and connect;
- denied resolved address;
- mixed DNS answers;
- re-resolution after reconnect/retry;
- same-origin redirect;
- separately authorized cross-origin redirect;
- denied redirect destination;
- loopback/private/out-of-scope redirect attempt;
- direct IP target;
- cross-origin auth/cookie/proxy-auth stripping;
- proxy endpoint versus ultimate-target authorization;
- remote-DNS SOCKS disposition;
- SNI/Host override disposition;
- insecure TLS not altering scope policy;
- cancellation during resolve/connect/read;
- HTTP/3 disposition (either fully scoped/tested or mechanically disabled in EggSec adapter).

Where possible, assert the actual connected peer address exposed by the adapter/fixture so tests prove policy-to-socket binding rather than only policy function invocation.

## Workstream 3 — Run the complete repository verification contract

At minimum:

```text
cargo fmt --all -- --check
cargo check --workspace --no-default-features
cargo +1.88 check --workspace --no-default-features
cargo +1.88 check -p eggsec-cli --no-default-features
cargo deny check
make clippy
make clippy-domain
make check-feature-profiles
make check-features-individual
make test-architecture-guards
make check
make check-python
make check-full
```

Also run focused tests for:

- `eggsec-transport`;
- `eggsec-transport-eggfetch`;
- `eggsec-agent`;
- NSE with its real feature profile;
- web proxy with interception features;
- CLI default/no-default profiles;
- TUI broad/full profiles from the completed corrective work;
- daemon embedded/remote dispatch parity tests affected by composition wiring.

Run hosted CI/Deep Checks and record run links/IDs if available to the implementer.

## Workstream 4 — Validate feature isolation

Demonstrate that normal/minimal artifacts do not accidentally pull specialized dependency islands.

Required assertions include:

- `eggsec-core` remains dependency-light with no Tokio/HTTP/TLS implementation stack;
- plain `eggsec --no-default-features` does not enable CLI/process-host dependencies;
- non-NSE builds do not pull NSE-only Lua/OpenSSL/native-tls compatibility dependencies;
- non-web-proxy builds do not pull MITM/interception-only certificate/server dependencies solely through the new transport;
- HTTP/3/QUIC does not appear in EggSec artifacts unless deliberately enabled after scoped support is proven;
- test-only Tokio features do not become a production workspace baseline;
- Eggfetch features enabled by EggSec match documented consumer needs.

Add durable architecture/feature tests for any invariant that cannot be confidently inferred from ordinary compilation.

## Workstream 5 — Security policy closure

Verify:

- Cargo Deny is a normal PR/merge gate;
- `deny.toml` source policy fails unknown git/registries as intended;
- approved advisory exceptions exactly match the narrative exception record;
- Cargo Audit has been removed/reconciled and cannot hide a broader set;
- all action uses are immutable full-SHA pins;
- Dependabot/equivalent covers Cargo and GitHub Actions;
- workflow permissions are least-privilege;
- Dependency Review status/disposition is documented.

Attempt one safe negative test per policy where practical, using a temporary/local branch rather than committing bad dependencies: demonstrate that an unknown git source/wildcard/advisory-introducing fixture would fail the intended gate.

## Workstream 6 — Document Egress decision

Record one of these explicit outcomes:

### Adopted

For every adopted `eggress-*` crate, record:

- version/SHA;
- exact API used;
- duplicate code/dependencies removed;
- before/after artifact graph delta;
- ownership boundary;
- why the dependency direction cannot cycle.

### Rejected/deferred

If Egress reuse does not reduce the graph or creates mismatched semantics, document the rejection as a positive result. Include the measured reason, e.g. `eggress-routing` introduces runtime/routing dependencies for functionality already covered by EggSec scope policy, while `eggress-uri` may remain a future convergence candidate.

Do not leave an unresolved “maybe use Egress later” claim without evidence/disposition.

## Workstream 7 — Documentation and plan closure

Update architecture/dependency documentation to reflect the actual final topology, not the aspirational roadmap diagram.

Update `plans/README.md` to mark this roadmap executed only after all acceptance criteria are met. Preserve all phase documents and append completion records rather than deleting them.

Each phase completion record should contain:

- baseline and final SHA;
- files/crates changed;
- dependency removals/additions;
- exact verification commands/results;
- upstream Eggfetch/Egress release/SHA where applicable;
- intentional exceptions/deferred items;
- owner and removal criterion for residual debt.

## Final acceptance criteria

The roadmap may be marked executed only when all are true:

1. Outbound HTTP consumers covered by the migration use the mandatory EggSec scope-aware transport contract.
2. Actual resolved/connected destinations are policy-bound and tested; no authorize-then-independent-reresolve TOCTOU remains in the supported adapter path.
3. Redirects cannot escape scope and sensitive credentials do not cross unauthorized origins.
4. Reqwest has been removed from migrated crates; every residual use is explicitly justified.
5. The interception proxy retains its specialized MITM/server boundary without forcing those dependencies into ordinary client-only builds.
6. `eggsec` has an empty library default feature set and process-host dependencies are appropriately owned.
7. Tokio capability features are narrowed by consuming crate and tested in isolated package profiles.
8. Egress reuse has a measured adopt/reject decision; no umbrella dependency was added without graph value.
9. Cargo dependency source/advisory/license checks are normal CI gates and policy files agree.
10. GitHub Actions are immutable-SHA pinned, update automation exists, and workflow permissions are least-privilege.
11. MSRV, no-default, feature sweep, TUI/daemon/Python, domain, and hosted CI checks are green.
12. Before/after dependency measurements are retained and show reduced duplicate ownership/capability reach even if raw crate count is not strictly lower.
13. No new offensive capability or authorization bypass was introduced.

## Closure artifact

Produce a concise retained report under `docs/architecture/` summarizing:

```text
baseline SHA
final SHA
Eggfetch version/SHA
Egress version/SHA or rejected disposition
removed direct dependencies by crate
remaining sensitive dependency owners
before/after key cargo-tree measurements
security fixture results
CI/supply-chain policy state
remaining debt
```

Link that report from this plan's completion record and from the relevant architecture documentation.
