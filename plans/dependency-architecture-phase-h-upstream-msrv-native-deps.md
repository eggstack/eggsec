# Phase H Plan: Upstream Modernization, MSRV, and Justified Native-Dependency Reduction

## Status


Status: Executed.
**Executed (Phase H supplement).** Dependency upgrades completed:
- MSRV raised from 1.80 to 1.85 (required for kube 4.x edition 2024)
- `native-tls` made optional behind `nse` feature in eggsec-nse
- gRPC proto generation: checked-in generated Rust file, protoc only for reflection descriptor
- kube upgraded from 0.92 to 4.2, k8s-openapi from 0.22 to 0.28
- MongoDB/BSON upgraded from 2.x to 3.x (run_command API change, bson re-export via mongodb)
- Redis upgraded from 0.25 to 1.x
- Dependency ownership table updated in AGENTS.md
- System dependency docs updated (BUILD.md, AGENTS.md, VERIFICATION.md)
- Native dependency inventory documented in BUILD.md

Deferred:
- PyO3 upgrade from 0.22 to 0.29 (substantial API migration across 40+ files)
- rusqlite upgrade from 0.31 to 0.40 (blocked: libsqlite3-sys linking conflict with sqlx 0.8; requires sqlx 0.9 which needs MSRV 1.94)

## Objective

Move Eggsec onto maintainable upstream dependency lines, establish a truthful
and directly tested Rust MSRV, and reduce native/external build dependencies only
where a maintained Rust alternative or checked-in generated artifact preserves
required behavior.

This phase must avoid two failure modes:

1. freezing security-relevant dependencies to preserve an old nominal MSRV that
   CI does not test;
2. replacing proven native libraries with immature Rust alternatives solely to
   claim a pure-Rust dependency graph.

## Scope

Primary dependency families:

```text
PyO3/maturin
MongoDB/BSON
Redis
Kube/k8s-openapi
Rusqlite/SQLite
SQLx
Tiberius
NSE native-tls/OpenSSL
SSH2/libssh2
prost/tonic/protoc
printpdf/lopdf
headless_chrome
pnet/nix/libc/socket2
```

Primary files:

```text
Cargo.toml
Cargo.lock
rust-toolchain.toml
crates/*/Cargo.toml
crates/eggsec-python/
crates/eggsec-db-lab/
crates/eggsec-daemon/
crates/eggsec-nse/
crates/eggsec-web-proxy/
crates/eggsec/src/commands/handlers/grpc.rs
crates/eggsec/src/tool/protocol/grpc.rs
crates/eggsec/build.rs
proto/
docs/BUILD.md
docs/RELEASING.md
docs/VERIFICATION.md
AGENTS.md
```

Confirm the current dependency graph and upstream release state at implementation
time. Do not use version assumptions from this plan as an upgrade target.

## Non-goals

This phase does not:

- require a completely native-free workspace;
- replace SQLite, OpenSSL, libssh2, or packet syscall bindings without a proven
  behavior-preserving alternative;
- upgrade all optional backends in one commit;
- increase feature scope;
- make every optional backend part of mandatory CI;
- vendor arbitrary upstream source trees;
- add Nix, containers, or a new build system to hide native prerequisites;
- publish packages;
- change manual release cadence.

## Workstream 1 — Determine the real MSRV

The workspace currently declares Rust 1.80 while normal CI uses floating stable.
After Phases E–G, determine the minimum compiler supported by the actual direct
dependency set and public compatibility goals.

Procedure:

1. identify MSRV declarations for each direct dependency and optional feature
   family;
2. distinguish the minimal/default engine profile from optional full/domain
   profiles;
3. choose one workspace MSRV unless a crate-specific higher MSRV is genuinely
   unavoidable and publishable metadata can express it clearly;
4. prefer raising the workspace MSRV over freezing a security-sensitive direct
   dependency on an unsupported line;
5. update `rust-version` and `rust-toolchain.toml` coherently;
6. add an exact MSRV compile job/check rather than relying on stable;
7. document the release-tool Cargo requirement separately if it remains newer
   than the code MSRV.

The chosen MSRV should reflect a maintained Rust release, current direct
upstreams, and the project's small-contributor scope. Avoid supporting a very old
compiler at disproportionate security/maintenance cost.

## Workstream 2 — Modernize Python toolchain dependencies

Coordinate with Phase E's PyO3 migration. Ensure:

- PyO3 and maturin versions are compatible;
- supported Python versions and ABI strategy are explicit;
- wheel build configuration does not force unnecessary system dependencies;
- the stable Python API and stubs remain intact;
- the chosen Rust MSRV supports the maintained PyO3 line;
- deprecated PyO3 compatibility code is removed after migration.

Do not add new Python features.

## Workstream 3 — Upgrade database adapter families independently

Treat each backend as a bounded adapter migration.

### PostgreSQL/MySQL via SQLx

- review SQLx release/MSRV and feature changes;
- ensure only required database backends are enabled;
- avoid SQLx macros/offline metadata where runtime queries suffice, unless macros
  provide clear safety value;
- preserve timeout, TLS, and error classification behavior.

### MSSQL via Tiberius

- update within maintained compatible lines;
- preserve Rustls transport and Tokio compatibility;
- test login failure, timeout, metadata probing, and dry-run behavior;
- keep optional futures/tokio-util activation narrow.

### MongoDB/BSON

- migrate the 2.x API to a maintained line in a dedicated commit;
- review client option, TLS, timeout, and BSON API changes;
- preserve feature gating and avoid pulling MongoDB into generic DB builds unless
  explicitly requested;
- add fake/local integration tests where feasible without an external service.

### Redis

- migrate to a maintained async API line;
- preserve connection-manager behavior only if still needed;
- test failure isolation and timeout behavior;
- keep the feature optional.

Each adapter may retain its own compatibility blocker temporarily. Do not hold
all adapters back until every backend can upgrade simultaneously.

## Workstream 4 — Upgrade Kubernetes adapter

Modernize `kube` and `k8s-openapi` together. Required decisions:

- supported Kubernetes API version range;
- whether one static API feature is sufficient;
- client/runtime features actually used;
- TLS/provider compatibility with Phase G;
- behavior when no cluster/config is available;
- schema/result compatibility for Python bindings if exposed.

Keep container/Kubernetes scanning optional. Do not make a live cluster required
for routine tests.

## Workstream 5 — Upgrade SQLite/Rusqlite while preserving daemon separation

Update the daemon server's Rusqlite line after Phase G ensures it is absent from
client artifacts.

Review:

- bundled SQLite necessity versus system SQLite;
- migration and transaction APIs;
- backup/pragma/connection behavior;
- concurrency assumptions;
- MSRV;
- database file compatibility.

Bundled SQLite is acceptable for reliable daemon distribution. Replacing it with
redb/fjall/sled or another store requires evidence that the daemon data model is
key-value shaped and that migration complexity is justified. This phase should
not perform such a rewrite by default.

## Workstream 6 — Remove unnecessary native TLS paths from NSE

Audit `eggsec-nse` for actual use of:

```text
native-tls
openssl
vendored OpenSSL
DES/legacy crypto
ssh2/libssh2
```

Actions:

- remove unconditional `native-tls` when unused or replaceable by existing
  Rustls code;
- keep OpenSSL only behind the NSE features/protocols that require it;
- determine whether legacy cipher support is required for NSE compatibility;
- preserve script behavior and protocol interoperability;
- document system/build requirements precisely.

Do not replace cryptographic primitives piecemeal without protocol fixtures.
RustCrypto replacements are acceptable only when wire-level compatibility tests
cover the affected algorithms and modes.

## Workstream 7 — Evaluate SSH2/libssh2 replacement, but require proof

Evaluate a maintained Rust SSH implementation only if it supports the NSE SSH
operations actually used:

```text
authentication methods
host-key handling
channel/session operations
algorithm compatibility
timeouts/cancellation
async Tokio integration
```

Required decision record:

- current libssh2 behavior and system dependency cost;
- candidate Rust library maintenance/security history;
- missing algorithms or compatibility gaps;
- binary/build impact;
- migration test fixture plan.

Default outcome may be to retain optional libssh2. A pure-Rust replacement is not
required for phase completion if it would reduce compatibility or security.

## Workstream 8 — Eliminate routine `protoc` dependency

If generated Prost/Tonic source can be checked into the repository without
creating confusing duplication:

1. generate sources with a documented maintainer command;
2. check in deterministic generated Rust files;
3. compile ordinary builds and CI without `protoc`;
4. keep regeneration as an explicit maintainer task;
5. add a manual/optional drift check if needed.

Alternative: use a vendored protoc crate only if checked-in generation is
incompatible with the current build architecture. Do not require hosted CI to
install `protobuf-compiler` solely to compile unchanged schemas.

Generated files must include a clear header and source schema path. Do not edit
them manually.

## Workstream 9 — Review remaining native/system boundaries

### Raw networking

`libc`, `nix`, `socket2`, pnet, and OS APIs are legitimate for raw packet and
packet inspection functionality. Keep them optional/platform-gated. Do not claim
these capabilities can be pure Rust in the sense of avoiding syscalls.

### Headless browser

`headless_chrome` may require an external browser executable. Retain optional
feature gating and improve diagnostics. Do not embed a browser runtime.

### PDF

Upgrade maintained PDF dependencies and contain parser/generator advisories.
Keep PDF output optional.

### Clipboard/TUI platform libraries

Keep them within TUI artifacts as established in Phase G.

For every remaining native or external runtime requirement, document:

```text
owning feature/artifact
platforms
build-time versus runtime requirement
fallback behavior
reason for retention
```

## Workstream 10 — Upstream ownership and update cadence

Create a concise dependency ownership table for major direct families. It should
identify the owning crate/domain and suggested review cadence, not create a
bureaucratic approval process.

Suggested categories:

- security-critical transport/crypto: monthly or advisory-driven;
- Python binding: each supported PyO3/maturin release cycle;
- database/Kubernetes optional adapters: quarterly or compatibility-driven;
- UI/output libraries: advisory/feature-driven;
- dev/benchmark tools: lower priority.

Dependabot/Renovate automation is not required. Manual grouped updates are
acceptable for this repository's size.

## Tests and validation

Use direct per-adapter validation.

Core/MSRV:

```bash
cargo +<msrv> check --workspace --no-default-features
cargo +<msrv> check -p eggsec-cli --no-default-features
cargo +stable check --workspace --no-default-features
```

Python:

```bash
make check-python
```

Database adapters as changed:

```bash
cargo check -p eggsec-db-lab --features db-drivers
cargo check -p eggsec-db-lab --features mssql
cargo check -p eggsec-db-lab --features mongodb
cargo check -p eggsec-db-lab --features redis
```

Other domains:

```bash
cargo check -p eggsec --features container
cargo check -p eggsec-nse --features nse
cargo check -p eggsec-nse --features nse-ssh2
cargo check -p eggsec-daemon
cargo test -p eggsec-daemon
cargo check -p eggsec --features grpc-api
```

Use local service fixtures only where already available. Do not add mandatory
external database, Kubernetes, SSH, or browser infrastructure.

## Commit sequencing

Prefer separate reviewable batches:

1. MSRV decision and exact check;
2. Python toolchain completion;
3. SQLx/Tiberius;
4. MongoDB/BSON;
5. Redis;
6. Kube/k8s-openapi;
7. Rusqlite;
8. NSE TLS/native cleanup;
9. checked-in protobuf generation;
10. native dependency documentation.

A backend migration may be deferred with an explicit blocker rather than making
one giant partial commit.

## Rollback considerations

- Revert individual adapter upgrades independently.
- Retain the truthful raised MSRV if reverting to an old vulnerable dependency
  would be required to lower it.
- If checked-in generated protobuf causes unacceptable churn, use a vendored
  protoc package as a bounded fallback; do not restore undocumented system
  requirements.
- If a pure-Rust SSH/TLS replacement lacks interoperability, retain the optional
  native library and document it.

## Acceptance criteria

1. The workspace declares a deliberate MSRV supported by direct dependencies.
2. Mandatory/representative CI compiles with that exact MSRV.
3. Floating stable remains tested separately where appropriate.
4. Release-tool Cargo requirements are documented separately from code MSRV.
5. PyO3/maturin are on maintained compatible lines.
6. Database adapters are upgraded independently or have explicit current
   blockers.
7. Kube/k8s-openapi are on a coherent supported pair.
8. Rusqlite is current enough for security/support needs and remains server-only.
9. Unused unconditional native-tls/OpenSSL dependencies are removed.
10. Remaining OpenSSL/libssh2 usage is optional, scoped, and documented.
11. No pure-Rust replacement is accepted without protocol/behavior proof.
12. Ordinary gRPC builds no longer require a system `protoc`, or the retained
    requirement is explicitly justified.
13. Raw packet/system syscall dependencies remain feature/platform scoped.
14. Every remaining native/external dependency has an owning feature/artifact and
    documented reason.
15. Optional backend upgrades do not become mandatory live-service CI.
16. No package or release is published.
