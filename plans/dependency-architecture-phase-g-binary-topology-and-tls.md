# Phase G Plan: Daemon/TUI Topology, TLS Provider, and Duplicate Dependency Cleanup

## Status

Ready for implementation after Phase F.

## Objective

Reduce the dependency and native-code footprint of Eggsec's primary artifacts
without removing functionality by:

- splitting daemon client protocol/transport from daemon server persistence;
- preventing the default TUI/CLI artifact from linking server-only SQLite and
  daemon internals merely to connect to a daemon;
- selecting one intended Rustls crypto provider per artifact;
- narrowing Tokio and reqwest feature ownership;
- aligning duplicate dependency generations where compatible;
- producing explicit standard, headless, daemon, Python, and full artifact
  profiles.

## Preconditions

Phase F must establish a clean engine/application boundary. Do not perform this
phase by moving daemon/server dependencies back into the engine or by creating a
new umbrella crate that recreates the same reachability.

## Scope

Primary files:

```text
Cargo.toml
Cargo.lock
crates/eggsec-cli/Cargo.toml
crates/eggsec-cli/src/daemon_cli.rs
crates/eggsec-tui/Cargo.toml
crates/eggsec-tui/src/
crates/eggsec-runtime/Cargo.toml
crates/eggsec-runtime/src/
crates/eggsec-daemon/Cargo.toml
crates/eggsec-daemon/src/
crates/eggsec/Cargo.toml
crates/eggsec-web-proxy/Cargo.toml
crates/eggsec-nse/Cargo.toml
crates/eggsec-python/Cargo.toml
crates/eggsec-agent/Cargo.toml
```

A new dependency-light daemon protocol/client crate is allowed if existing
`eggsec-runtime` cannot own the transport-neutral client contract without
violating its purpose. Choose the smallest coherent boundary.

## Non-goals

This phase does not:

- remove daemon mode, SQLite persistence, TUI, clipboard support, TLS, proxy,
  Python, or full-feature builds;
- rewrite SQLite persistence in Rust merely to avoid C code;
- merge all binaries into one universal executable;
- split every feature into a separate binary;
- replace Rustls;
- optimize the `full` aggregate at the expense of standard artifacts;
- update every optional upstream driver; Phase H owns broad modernization;
- add CI publication or release automation.

## Artifact profiles to preserve

Define and document at least:

### Standard interactive CLI/TUI

Human-facing CLI with default TUI. It may include terminal UI dependencies but
must not link daemon server persistence merely for client connectivity.

### Headless CLI

No TUI, clipboard, theme compression, or daemon server dependencies. It retains
all headless command capabilities enabled by its feature set.

### Daemon client

Protocol/client transport and session operations only. No SQLite server storage,
server listener, RBAC implementation, or daemon process CLI unless explicitly
building the server.

### Daemon server

Persistence, session host, selected transports, process CLI, and optional full
executor.

### Python extension

Engine/library capabilities and explicitly enabled domains. No CLI/TUI/server
persistence reachability.

### Full developer/lab build

All supported optional capabilities. It is not a size target and remains
explicitly non-default.

## Workstream 1 — Measure current artifact dependency topology

Record before-change measurements for each profile:

```bash
cargo tree -p eggsec-cli -e features
cargo tree -p eggsec-cli --no-default-features -e features
cargo tree -p eggsec-daemon -e features
cargo tree -p eggsec-python -e features
cargo tree -d
```

Build release artifacts where supported and record sizes. Identify whether the
standard CLI/TUI graph includes:

```text
rusqlite/sqlite3
axum server paths
server persistence code
multiple Rustls crypto providers
multiple Tower generations
multiple tokio-tungstenite generations
multiple Quick-XML generations
native-tls/OpenSSL paths
broad image/clipboard platform stacks
```

Do not infer success from manifest edits alone; measure the final linked
artifacts.

## Workstream 2 — Split daemon protocol/client from server implementation

Choose one of these bounded designs:

### Preferred: protocol/client crate

Create a small crate, tentatively `eggsec-daemon-client` or
`eggsec-daemon-protocol`, containing:

- request/response/event DTOs not already in `eggsec-runtime`;
- socket/transport-neutral client trait;
- Unix-socket client implementation if sufficiently small and portable;
- session attach/create/list operations;
- protocol version/capability negotiation;
- no persistence, server listener, SQLite, RBAC database, or full executor.

`eggsec-tui` and `eggsec-cli` depend on this crate for daemon mode.

### Acceptable: split modules/features within existing crates

If a new crate would only duplicate `eggsec-runtime`, move client DTOs to
`eggsec-runtime` and place the concrete client behind a narrow feature that does
not activate server dependencies. This is acceptable only if Cargo feature
unification cannot accidentally pull server-only dependencies into clients.

The daemon server depends on the protocol/runtime layer, not vice versa.

## Workstream 3 — Remove server-only dependencies from TUI and default CLI

After client extraction:

- replace `eggsec-tui -> eggsec-daemon` with the protocol/client dependency;
- replace CLI daemon-client feature dependency similarly;
- ensure default TUI mode retains embedded execution;
- ensure daemon mode retains connect, attach, create, resume, event streaming,
  and cancellation behavior;
- keep `eggsec-daemon` as the binary/server package;
- verify bundled `rusqlite` is absent from standard client graphs.

Do not copy daemon authorization logic into the TUI. Session/runtime enforcement
remains server/engine-owned.

## Workstream 4 — Select one Rustls provider

Inventory every direct Rustls declaration and transitive provider activation.
The current pattern of enabling `ring` while retaining defaults may activate both
`ring` and AWS-LC.

Choose one provider policy for primary artifacts. The default recommendation is
a ring-only profile when it supports all required algorithms and platforms, but
the implementation must validate:

- TLS client/server behavior;
- certificate generation/signing requirements;
- web proxy MITM certificates;
- reqwest/hyper integration;
- tonic/gRPC;
- Python wheel target compatibility;
- macOS/Windows/Linux builds;
- FIPS requirements, if any are actually claimed.

Configure direct Rustls dependencies with `default-features = false` and explicit
provider/features. Align rcgen and downstream assumptions. Do not compile both
providers accidentally.

If an artifact genuinely requires a different provider, document it and ensure
that artifact still links only one provider.

## Workstream 5 — Narrow Tokio features per crate

The workspace may centralize versions, but consuming crates should request only
needed features. Inventory use of:

```text
rt
rt-multi-thread
macros
net
io-util
fs
process
signal
time
sync
```

Apply crate-local feature selections. Because Cargo unifies features within a
final graph, evaluate the result per artifact rather than expecting every crate's
manifest to remain minimal in isolation.

Do not remove required multi-thread runtime behavior from scanner/load paths
without benchmarks and correctness tests.

## Workstream 6 — Align reqwest feature profiles

Build on Phase F's client ownership. Ensure:

- core async clients do not enable blocking unless used;
- SOCKS is activated only by proxy-capable artifacts;
- cookies are activated only by session/auth functionality;
- HTTP/2 is activated where actually supported/tested;
- JSON/form/query features are owned by relevant clients;
- engine, proxy, Python, agent, and daemon dev-dependency declarations use a
  compatible reqwest line and coherent TLS configuration.

Avoid two independent high-level HTTP client abstractions.

## Workstream 7 — Align duplicate dependency generations

Use `cargo tree -d` and prioritize duplicates with material code size or security
impact:

1. Tower/Tower-HTTP generations;
2. tokio-tungstenite/tungstenite generations;
3. Quick-XML generations after Phase E direct upgrade;
4. Rustls/rustls-webpki generations;
5. URL/IDNA generations;
6. time/chrono support paths;
7. LRU and hashing utilities;
8. socket2/nix versions where direct manifests can align.

For each duplicate:

- identify direct owners;
- determine whether a compatible version alignment exists;
- measure whether alignment changes primary artifacts;
- avoid `[patch]` overrides that force incompatible APIs;
- retain duplicates when upstream constraints make consolidation risky.

The goal is fewer unnecessary duplicate generations, not an artificial zero.

## Workstream 8 — Review TUI-only heavy dependencies

Ratatui/crossterm are core to the TUI and remain. Review optional/large TUI
support dependencies:

```text
arboard and platform image/clipboard stack
lzma-rs theme packaging
arc-swap
URL/base64 helpers
```

Confirm they are absent from headless and Python artifacts. Feature-gate clipboard
support only if doing so does not degrade default TUI behavior unexpectedly.
The user-visible TUI remains fully capable by default unless a separate minimal
TUI profile is documented.

## Workstream 9 — Introduce reproducible artifact-size checks without a hard gate

Add a small local script or documented command set that reports:

```text
artifact path
profile/features
file size
stripped/unstripped state
direct/transitive crate count
selected notable dependencies
```

Do not make exact byte size a mandatory CI gate; compiler and dependency patch
updates create legitimate variation. A coarse regression threshold may be used
later only after stable baselines exist.

## Tests

Required tests/checks include:

- daemon client protocol round-trip with an in-process or temporary Unix socket;
- TUI daemon-mode attach/connect behavior;
- default embedded TUI behavior;
- daemon server persistence tests;
- protocol version/capability mismatch handling;
- TLS provider installation and client/server handshake;
- proxy certificate generation and handshake when web-proxy is enabled;
- reqwest client profile behavior for redirects/proxies/cookies where owned;
- headless CLI and Python builds without server/TUI dependencies.

Avoid privileged or external-network tests.

## Validation commands

```bash
cargo fmt --all -- --check
cargo check -p eggsec-cli
cargo check -p eggsec-cli --no-default-features
cargo check -p eggsec-cli --no-default-features --features daemon-client
cargo check -p eggsec-tui
cargo check -p eggsec-daemon
cargo test -p eggsec-daemon
cargo check -p eggsec-python
make check-python
cargo tree -p eggsec-cli -d
cargo tree -p eggsec-python -d
```

Feature checks as changed:

```bash
cargo check -p eggsec --features rest-api,grpc-api
cargo check -p eggsec --features web-proxy
cargo check -p eggsec --features nse
```

Use portability checks locally or in the optional workflow; do not add a new
mandatory matrix in this phase.

## Migration sequence

1. Record baseline graphs/sizes.
2. Introduce protocol/client boundary with compatibility tests.
3. Migrate CLI daemon client.
4. Migrate TUI daemon mode.
5. remove client dependencies on daemon server crate.
6. configure one Rustls provider.
7. narrow Tokio and reqwest features.
8. align high-value duplicate generations.
9. rebuild and record final artifact graphs/sizes.
10. update build/daemon documentation.

## Rollback considerations

If protocol extraction introduces instability, retain the protocol types in
`eggsec-runtime` and concrete client in a separate narrow module/crate. Do not
restore TUI dependency on server persistence as the permanent rollback.

If one Rustls provider fails a required platform, document and use a
platform/artifact-specific provider choice rather than re-enabling both globally.

## Acceptance criteria

1. Standard TUI/CLI client graphs do not include `rusqlite` solely for daemon
   client support.
2. Daemon server retains persistence and all existing server behavior.
3. Daemon client protocol has a dependency-light owner.
4. TUI embedded and daemon modes both pass direct tests.
5. Headless CLI excludes TUI and daemon server dependencies.
6. Python excludes TUI and daemon server persistence.
7. Each primary artifact links one intended Rustls crypto provider.
8. Tokio features are selected by consuming crates and verified per artifact.
9. Reqwest blocking/SOCKS/cookies/HTTP2 features have explicit owners.
10. High-value duplicate Tower/WebSocket/XML/TLS generations are reduced where
    compatible, with retained duplicates documented.
11. User-visible daemon, TUI, proxy, TLS, and Python capability is preserved.
12. Before/after dependency trees and release artifact sizes are recorded.
13. No exact-byte mandatory CI gate is added.
14. No package or release is published.
