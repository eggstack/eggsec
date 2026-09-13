# Capability Segregation Decisions (Phase E WS2–WS4)

Status: Decided 2026-09-13. No new crates (all extraction candidates rejected).
WS2 (empty library default) and WS3 (per-crate Tokio) implemented; this record
covers WS4 extraction evaluations with the required decision fields.

## WS2 — Engine library-default empty: IMPLEMENTED (not a new crate)

- Change: `crates/eggsec/Cargo.toml` `default = ["cli"]` → `default = []`.
  `cli` feature retained (`dep:clap` + `dep:clap_complete`); `cli`, `commands`,
  `dispatch`, `runtime_bridge` modules remain `#[cfg(feature = "cli")]`.
- Explicit opt-in: binary shell (`eggsec-cli`: `eggsec` with `features = ["cli"]`),
  terminal UI (`eggsec-tui`: same), daemon executor
  (`eggsec-daemon/full-executor = ["dep:eggsec", "eggsec/cli"]`).
- Python bindings already built with `default-features = false` (no change).
- Checks: `cargo check -p eggsec`, `cargo check -p eggsec-cli`,
  `cargo check -p eggsec-cli --no-default-features`,
  `cargo check --workspace --no-default-features` (all in `make check`);
  clippy runs both empty-default and `cli`; integration suites request `cli`
  explicitly (`--features rest-api,cli`); guard Check 104.

## WS3 — Per-crate Tokio: IMPLEMENTED (not a new crate)

- Baseline: workspace `tokio = { version = "1", default-features = false }`
  (was 11 features including `test-util`). Each crate declares only what its
  `tokio::` uses require. `test-util` enabled nowhere (no paused-time sites).
- Ownership: engine keeps the broadest set (all except `test-util`; MCP stdio
  needs `io-std`, outputs need `fs`, cluster needs `process`, shutdown needs
  `signal`); process hosts (`cli`, `daemon`) keep `rt-multi-thread` + `signal`;
  DTO crates (`core`, `tool-core`, `ui-model`, `daemon-protocol`) carry no
  Tokio; `output` keeps `io-util` only; `transport`/`eggfetch` keep Tokio in
  dev-deps only (`rt`+`macros`, plus `net`/`time`/`io-util` for fixture tests).
- Checks: `cargo check --workspace --no-default-features` plus the full
  feature matrix (unification cannot hide missing declarations; isolated
  narrow manifests fail loudly); guard Check 105; manifest-graph Check 108.

## WS4 candidate 1 — `eggsec-net` (target-resolution policy): REJECT

- Dependency delta: +1 crate, +1 edge from `eggsec`, `eggsec-transport`,
  and every DNS user (`recon`, `scanner`, `packet`, `nse`, `web-proxy`,
  `python`). No removed edges (hickory stays for engine resolution,
  `std::net::ToSocketAddrs` stays for transport facts).
- API boundary (proposed, rejected): `HostResolver` + `TargetScope` +
  `classify_address()` + `ScopeAuthority` binding moved into `eggsec-net`.
- Capability isolated: none cleanly. Canonical DNS facts (`TransportResolver`)
  already live in `eggsec-transport`; authorization verdicts (`Scope`,
  `LoadedScope`) live in engine `config`; the TOCTOU-closed binding
  (`validate_binding` + `ApprovedBinding`) is the contract between them.
  Moving the middle creates a third scope-adjacent language instead of
  removing one.
- Cycle analysis: `eggsec-transport` (leaf, 4 deps) would need `eggsec-net`
  for resolver types OR `eggsec-net` would need `eggsec-transport` for
  authority checkpoints — either direction widens the leaf or inverts the
  `transport → nothing` invariant (guard Check 100). Engine already depends
  on transport; adding engine → net → transport lengthens the chain with no
  isolation gain (DNS is not a privilege boundary; scope policy is).
- Migration cost: move `TargetScope`, `HostResolver`, `SystemResolver`,
  `ScopeAuthority`, plus 12 invariant + 11 contract tests; re-gate every
  `lookup_host`/hickory call site; high churn, zero dep removal.
- Rejected alternative (chosen): keep Phase B split (transport owns
  checkpoint shape + facts; engine owns policy verdicts). No `eggsec-net`.

## WS4 candidate 2 — Web assessment client vs interception proxy split: REJECT

- Dependency delta: +1 crate (`eggsec-web-client`), −0 crates. Engine would
  depend on both; `eggsec-web-proxy` would shrink to intercept/server only.
- API boundary (proposed, rejected): move scanner/fuzzer/recon HTTP clients
  (`endpoints`, `techdetect`, `hunt`, `cve_lookup`, `oast`) into the new
  client crate.
- Capability isolated: none that feature gating does not already provide.
  Client assessment lives in the engine today, NOT in `eggsec-web-proxy`.
  The proxy crate owns intercept/server TLS (`TlsAcceptor`), H2 demux, WS/gRPC
  decoding, and SOCKS — all `optional` behind `web-proxy` (plus `web-proxy-mcp`,
  `transparent-proxy`, `dynamic-plugins` markers). Non-proxy builds
  (`--no-default-features`, python `default-features = false`) already avoid
  `rcgen`, `h2`, `tokio-tungstenite`, `prost`/`prost-types`. Verified:
  `cargo check -p eggsec --no-default-features` pulls none of them.
- Cycle analysis: client crate would need `eggsec-transport` DTOs + engine
  scope types; engine would need the client crate for scanner/fuzzer — a
  tight two-way capability split with shared auth-context/header helpers.
  Current one-way shape (engine optionally depends on proxy domain) is
  simpler and acyclic.
- Migration cost: move 6+ OOHTTP subsystems plus their fake-based parity
  tests; duplicate `ProxyIntent`/redirect/TLS policy mapping at the new
  boundary; no server-dep removal (engine keeps `reqwest`/`rustls` for those
  subsystems until Phase D per-consumer migration finishes).
- Rejected alternative (chosen): keep the A/B boundary inside
  `eggsec-web-proxy/src/outbound.rs` (side A intercept/server never touches
  the client contract; side B probes stay minimal-reqwest until the adapter
  supports authorized proxy routing). Split only if a consumer needs proxy
  interception types without ANY client capability AND the split deletes
  `rcgen`/`h2`/WS/gRPC from a currently-paying artifact (no such artifact
  exists today).

## WS4 candidate 3 — Evidence/signing crypto crate: REJECT

- Dependency delta: +1 crate (`eggsec-evidence`), −2 small edges (`hmac`/`sha2`
  removed from one of `eggsec` / `eggsec-web-proxy`). Net graph change ≈ 0.
- API boundary (proposed, rejected): shared `sign(bundle, key)` + `verify()`
  + key-erasure (`zeroize`) policy.
- Capability isolated: none shared. The two HMAC uses are unrelated:
  (a) `eggsec-web-proxy/src/intercept/bundle.rs`: HMAC-SHA256 over the
  evidence-manifest canonical form (integrity, `key_id` + `signed_at`
  envelope); (b) `crates/eggsec/src/notify/webhook.rs`: HMAC-SHA256 over
  outbound JSON for `X-Signature-256` (notifier auth). Keys, lifetimes,
  verification parties, and failure modes differ. Scanner hashing elsewhere
  (`md-5`, `sha1`, file digests) is content identification, not signing.
- Cycle analysis: both consumers would depend on the new crate; the crate
  would depend on `hmac`/`sha2`/`hex`/`zeroize` only — acyclic but pointless
  (pure-Rust crypto with no privilege boundary; no TLS/server deps removed).
- Migration cost: unify two different key-management stories into one API
  (forces a false abstraction), move 2 test suites, version the envelope —
  all to save one duplicate `hmac = "0.12"` line.
- Rejected alternative (chosen): keep both HMAC sites local with their own
  key handling (webhook secrets via `SensitiveString` + `expose_secret()`;
  bundle keys via caller-supplied bytes). Centralize only if three or more
  crates share bundle-signing semantics with identical envelope/key-erasure
  policy (threshold not met).

## Guards

- Check 107 fails if `crates/eggsec-net`, `crates/eggsec-web-client`,
  `crates/eggsec-evidence`, or `crates/eggsec-signing` appears, and requires
  this record.
- Check 108 (manifest-graph, `python3` + `tomllib`) enforces the preserved
  direction: DTO crates touch no transport/TLS/HTTP impl; `eggsec-transport`
  stays exactly `bytes`/`http`/`url`/`thiserror`; engine touches no frontend
  crates; path-dependency graph is acyclic.

*Last verified against source: 2026-09-13*
