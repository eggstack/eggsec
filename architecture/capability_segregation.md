# Capability Segregation Decisions (Phase E WS2–WS4 + Phase C policy extraction + Phase D loadtest/resilience closure + Phase G knowledge-corpus evaluation)

Status: Phase E decided 2026-09-13 (no new crates; all WS4 candidates rejected).
Phase C (crate-boundary consolidation, 2026-09-16) extracts `eggsec-policy`
below; `eggsec-net` / web-client / evidence rejections remain in force.
Phase D (crate-boundary consolidation, 2026-09-16) decouples load testing
internally and rejects both `eggsec-loadtest` and `eggsec-resilience`
(see Phase D section below).
Phase G (2026-10-05) evaluates data-shaped **knowledge-corpus** modules and
proposes three leaf crates under `plans/adrs/ADR-0005-knowledge-corpus-crate-ownership.md`;
no Phase E WS4 or Phase D rejection is reopened.
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
   `ScopeAuthority`, plus 12 invariant + 12 contract tests; re-gate every
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

## Phase C — `eggsec-policy` (authorization/enforcement semantics): ACCEPT

`eggsec-net` (above) remains rejected. `eggsec-policy` is a different
boundary: not a network middle layer, but the complete deterministic
authorization semantic domain (policy vocabulary, execution policy,
descriptors, catalog, scope data + pure matching, decisions, approval
tokens, evaluation over explicit inputs).

- Dependency delta: +1 crate (`crates/eggsec-policy`), +1 edge
  (`eggsec` → `eggsec-policy`). Removed edges: engine `config` no longer
  owns policy semantics (facades only); `scope_spec.rs` no longer duplicates
  matching logic (delegates to `evaluate_facts`); lab-report conversion is an
  engine free function (orphan-rule-forced, same behavior). No removed
  third-party deps (policy needs `serde`/`serde_json`/`thiserror`/`url`/
  `ipnetwork`/`sha2`/`uuid`/`hex`/`rustc-hash`, all already in the lockfile).
- API boundary: `eggsec-policy` owns `OperationRisk`/`OperationMode`/
  `ExecutionProfile`/`ExecutionSurface`/`IntendedUse`/`Capability`/
  `DenialClass`, `ExecutionPolicy`, `OperationDescriptor`, `OperationTarget`
  + `normalize_target`, `OperationMetadata` catalog, `Scope`/`TargetScope`/
  `ScopeRule`/`ScopeSource`/`LoadedScope` + `evaluate_facts`/
  `evaluate_addresses`, `AddressClass`/`classify_address`, `PolicyDecision`/
  `EnforcementOutcome`/`EnforcementError`/`ConfirmationClass`/
  `ManualOverride`/`PreflightResult`, `ApprovedOperation`, `EnabledFeatures`,
  and pure `evaluate_operation_policy`/`evaluate_enforcement`/
  `EnforcementContext` (explicit features + supplied `TargetScope` facts).
  Engine keeps config loading, `feature_registry` → `EnabledFeatures`
  mapping, DNS acquisition, `ScopeAuthority`, `ScopeSpec` conversion, and an
  I/O-enabled `EnforcementContext` facade (same call signatures; resolves
  then delegates). `eggsec::config::*` paths remain compatibility facades.
- Why not an internal module: the policy cluster (~10k lines across 10
  modules) is now larger than ordinary config code, has an independent
  test story (80 kernel tests run with a 9-crate closure vs the engine's
  full closure), and is the only authorization owner — the same rationale
  that justified `eggsec-report-model` (Phase B). An internal boundary would
  not remove the engine's ownership ambiguity or the duplicate-matching risk.
- Cycle analysis: `eggsec-policy` → nothing workspace (leaf). Engine →
  policy one-way. `eggsec-transport` ↔ policy: no edge either direction
  (guard Check 122). Workspace path graph stays acyclic (Check 108).
- Migration cost: moved 8 modules (~4.5k non-test lines) + 80 pure tests;
  updated ~30 call sites (trait imports, bridge functions, 3 TUI session
  conversions, 2 Python modules); deleted 2 orphan-violating impls
  (`From<&PolicyDecision>`, `TryFrom<&ScopeSpec>` → free functions).
  Approval-token construction sites unchanged in behavior (guard Check 76
  allowlists the kernel).
- Measured effect: `cargo tree -p eggsec-policy` shows 9 direct deps, zero
  forbidden (no Tokio/HTTP/TLS/filesystem/frontend/engine); `cargo test -p
  eggsec-policy` 80 passed in isolation; `make check` green (2830 engine
  tests + guards Checks 121–123).
- Retained engine bridges: `policy_bridge/{features,resolver,transport}`
  (`current_enabled_features`, `resolve_target_facts*`,
  `ScopeResolution`, `ScopeAuthority`, `load_scope_from_file`).
- Extraction gates (C0/C1): all passed — pure set compiles without engine/
  transport/Tokio/HTTP/filesystem/frontend deps; one-way engine dependency;
  config deserializes/re-exports policy types without duplication; policy
  tests run isolated with a narrower graph; ownership is unambiguous.

## Phase D — loadtest decoupling + `eggsec-loadtest` / `eggsec-resilience`: REJECT both

Load testing was decoupled internally (`plan` / `executor` / `metrics` /
`progress` core + `adapter` above it + `backend` Reqwest `HttpTransport`
behind the `eggsec-transport` seam; `indicatif` only in CLI `run_cli`;
`tui_mode` removed from the core contract; per-worker sharded metrics with a
CAS global pacer). The decoupling is retained; the crate extractions were
evaluated against the roadmap's decision rule and rejected:

- Gate D1 (`eggsec-loadtest`): REJECT. The core still touches engine-owned
  helpers (`utils::parse_headers`, `utils::http::tool_user_agent`,
  `utils::formatting::preserve_all`, `install_tls_provider`, `constants`,
  `Scope`/`policy_bridge` in the backend); the only dependency that would
  leave the engine closure is `hdrhistogram` (`indicatif` stays for
  scanner/fuzzer/pipeline/stress regardless). Single consumer (the engine);
  no independent library surface or second consumer; package/versioning cost
  unjustified. Criteria 4–5 of Gate D1 fail; an internal boundary is
  sufficient.
- Gate D2 (`eggsec-resilience`): REJECT. Rate-control/circuit-breaker
  consumers are all inside the `eggsec` crate (`waf`, `ai`, tool protocol);
  no second crate or sibling project demonstrated; extracting would create
  adapters around existing implementations rather than removing duplication.
  Primitives stay in `utils::{rate_limiter, circuit_breaker}` with explicit
  semantics and tests. `fuzzer::rate_limit` (lock-free consecutive-error
  limiter) and the loadtest `GlobalPacer` (CAS slot allocator, no mutex on
  the hot path) are intentionally operation-local, not duplicates: the shared
  token buckets would serialize (`SharedRateLimiter`) or mismatch semantics.
- `eggsec-transport` remains the reference leaf boundary (unchanged, not
  renamed); `eggsec-policy` reuse discussion is deferred (no non-Eggsec
  consumer; API stays as extracted in Phase C).
- Incidental cleanup: `utils::cache::ApiCache` (zero production consumers,
  flagged in Phase A) removed.

## Phase F — `eggsec-udp-scan` (UDP port scanning): ACCEPT as a separate crate

**Decision:** accept a new domain crate `crates/eggsec-udp-scan`, dependent on
`libc` only. It authorizes nothing, resolves no DNS, renders no output, and
never acquires privilege — it *reports* that privilege is required.

**Why a crate and not engine code.** Three reasons, in order of weight:

1. **The primitives are incompatible, not merely different.** TCP scanning
   works by handshake: a failed `connect` is a definitive `closed`, so the
   result is a 2-state boolean list. UDP has no handshake. A send always
   "succeeds" and the answer lives in a negative, rate-limited, out-of-band
   signal. The existing per-port `Option<PortResult>` shape cannot express
   "a correlated negative that may never arrive".
2. **The authorization facts differ.** A TCP port scan needs
   `Capability::ActiveProbe`. Receiving unsolicited ICMP errors needs
   `Capability::RawPacketProbe` and, on Linux, a platform privilege gate.
   Those belong in different metadata records, not one union type.
3. **The result type differs in kind.** TCP is a boolean list. UDP is a
   four-state lattice with per-state evidence *plus a host-level verdict that
   can invalidate the per-port claims*. Forcing that into
   `PortResult { port, status: String, service: String }` makes the state
   stringly-typed and pushes the state machine onto every consumer.

**What it shares with the TCP scanner: shape, not code.** It mirrors the
bounded-concurrency admission discipline so operators see consistent progress
semantics, and reuses the engine's result-mapping boundary. It does not
generalise, reuse, or subclass the TCP scan loop.

### Platform findings (measured, not assumed)

The usual "unprivileged tier" designs start from `IP_RECVERR` on Linux
connected sockets. Both halves of that premise were checked against this
repository's development platform (darwin/arm64, `euid != 0`) and **neither
holds**:

| Claim | Measured result |
| --- | --- |
| `socket2` 0.5 exposes `IP_RECVERR` | No such accessor. `Socket::as_raw()` is `pub(crate)`, so even a `libc::setsockopt` shim is impossible against a `socket2` socket. |
| `IP_RECVERR` exists on macOS/BSD | Undeclared. The constant does not compile. |
| macOS unprivileged ICMP | `socket(AF_INET, SOCK_DGRAM, IPPROTO_ICMPV4)` **succeeds unprivileged** and delivers port-unreachable for UDP probes, with the originating IPv4 header attached. `SOCK_RAW` for the same protocol returns `EPERM`. |

So the crate uses `libc` directly rather than `socket2`, and the unprivileged
tier is available on macOS/BSD and *not* on Linux — the inverse of the usual
assumption. The ICMP parser accepts both delivery shapes (raw socket strips
the IPv4 header, datagram socket keeps it) and the tests assert the two parse
identically.

### Honesty contract

The crate's load-bearing invariant is that **closed is provable and open is
not**. Silence is ambiguous across four cases, so it is reported as
`open|filtered` and never as `open`, and a port list from a host that produced
no attributable ICMP is explicitly marked not meaningful via `HostState`.
Reporting `open` requires a protocol-specific probe, which is a different
operation with different authorization requirements (arbitrary protocol
payloads on the wire is materially closer to packet injection than to
`ActiveProbe`) and is out of scope for this crate.

### Supply chain

`libc` is already a direct workspace dependency (`crates/eggsec/Cargo.toml:131`)
and already in `Cargo.lock`; `socket2` and `pnet` are likewise already present.
`deny.toml` needs no new entry and no new exception: the crate introduces no
new graph node. `pnet`/`pnet_packet` are deliberately **not** used — a raw or
datagram ICMP socket is a plain BSD socket, so pulling in `pnet` would drag the
`libpcap-dev` system dependency in for no benefit.

## Phase G — Knowledge-corpus modules (2026-10-05): PROPOSED, 3 leaf crates

Full decision: `plans/adrs/ADR-0005-knowledge-corpus-crate-ownership.md`.
Roadmap: `plans/subsystems/security-knowledge-corpus-roadmap.md`.
Implementation plans: `plans/implementation/security-knowledge-corpus/001…005`.

**Status: proposed — not implemented.** Nothing in this section is a code change yet.

### Why this category is new

Phases A–F evaluated *logic* clusters: policy semantics, report contracts, load-test
internals, web-client vs interception proxy, evidence crypto. Phase G evaluates a
category none of them addressed — **domain knowledge expressed as data**, where the
coupling is near zero because the module's job is to hold a body of knowledge rather
than to reach outward. The engine's large modules (`fuzzer`, `recon`, `scanner`, `waf`,
`distributed`) are *not* in this category: they are executor bodies whose coupling is
their function, and Phase C/D/WS4's rejections of them stand unchanged.

### Dependency delta (measured, not assumed)

| Module | Lines | `crate::` coupling | Workspace deps after | Tests |
|---|---|---|---|---|
| `fuzzer/payloads/` pure-data subset | 7,084 | ~0 | `eggsec-core` | 233 |
| `recon/secrets.rs` | 492 | 1 (`Severity`) | `eggsec-core` | 11 |
| `scanner/service_data.rs` | 302 | 0 | none | 21 |

All three were validated by extraction spike: copied into scratch crates and compiled
with their full original test suites passing. The payload corpus needed exactly four
mechanical edits (two `Severity` import rewrites, one `crate::fuzzer::payloads` path
rewrite, one `$crate` macro path); the service table needed **none**.

The decisive structural fact: `Severity` is already owned by `eggsec-core`, a
zero-internal-dependency leaf (`crates/eggsec/src/types.rs:16` re-exports it). Any module
whose only engine reference is `Severity` is one import away from being a leaf crate —
the same rationale that justified `eggsec-policy` in Phase C.

### Proposals

| Crate | Contents | Rationale |
|---|---|---|
| `eggsec-service-db` | port→service tables, banner heuristics | zero workspace deps, zero consumers, 21 tests; `nmap-services`-style corpus |
| `eggsec-secrets` | 25 credential patterns, entropy scoring | pure regex+entropy over strings; gitleaks/trufflehog-adjacent |
| `eggsec-payloads` | 34 data-only payload modules | largest closure reduction; SecLists/ffuf-adjacent |

Each lands as `publish = false` with an engine re-export facade, so **no consumer import
changes** — the `eggsec-python` exhaustive matches over 30 `SecretType` and 40
`PayloadType` variants compile untouched.

**Status: `eggsec-service-db` implemented (milestone 002).** 302 lines moved verbatim,
21 tests green in isolation, single dependency edge (`rustc-hash`) confirmed by
`cargo tree`. The engine reaches it only through
`pub use eggsec_service_db as service_data;`, and `git diff` shows **zero** changes in
`scanner/ports/`, `eggsec-python`, `eggsec-tui`, `eggsec-mobile-lab`, or
`crates/eggsec/tests/`. Check 114's canonical-owner pin moved to
`crates/eggsec-service-db/src/lib.rs` (and now also fails if the old engine-side file
reappears, or if the engine facade is dropped). **Check 148** enforces the leaf invariant
for every corpus crate and is demonstrated to fail on a forbidden dependency.
`cargo test -p eggsec-service-db --tests` is registered in `make check`.

**Status: `eggsec-secrets` implemented (milestone 003).** 492 lines moved with **exactly
one** source change — `pub use crate::types::Severity` → `pub use
eggsec_core::types::Severity`. The 25-pattern corpus, 30 `SecretType` variants, 3
`Confidence` tiers, and the entropy gate are byte-identical; the 11-test count is
unchanged. `cargo tree` shows `eggsec-core` as the only workspace edge. The engine facade
`pub use eggsec_secrets as secrets;` preserved `eggsec::recon::secrets::*` with an
**empty** diff across `crates/eggsec-python/`, `crates/eggsec-mobile-lab/`,
`crates/eggsec-tui/`, and `crates/eggsec/tests/` — the milestone's defining constraint,
since the bindings match all 30 variants exhaustively. `git_secrets.rs` (subprocess
orchestration) stays engine-side. **Check 149** pins the owner, the facade, and the
entropy constant. `cargo test -p eggsec-secrets --tests` is registered in `make check`.

**Status: `eggsec-payloads` implemented (milestone 004).** The 34 pure-data payload
modules (7,084 lines) moved to `crates/eggsec-payloads`; **233 tests pass**, matching the
baseline count exactly. The 6 live-probe modules stay engine-side. The four mechanical
edits were applied (two `Severity` imports, one module-path rewrite across 11 files, the
`$crate` macro path) — plus one design change that the plan anticipated: the
cross-variant caches **cannot** live in the corpus crate, because building them requires
all 40 variants including the engine's. They moved to a new engine-side
`fuzzer/payloads/mod.rs` that owns the union, dispatching the 6 advanced types locally
and delegating the other 34. `git diff` shows **zero** changes in `eggsec-python` or
`eggsec-tui`, so `waf_validation.rs`'s exhaustive 40-arm `parse_payload_type` compiles
untouched.

The specific trap this milestone called out — a `Vec::new()` stub for the 6 relocated
types — was designed out rather than shipped: `eggsec-payloads::get_payloads` routes those
types to a documented `unreachable!`, because a silent empty vector reads as "this type has
no payloads", which is false. Three layers guard the seam: **check 150** (structure —
probe modules in place, no `Vec::new` stub, caches engine-side and still `LazyLock`) and
the new engine integration suite `tests/fuzzer_payload_corpus_seam.rs` (behavior — all 6
advanced types return non-empty payloads, all 40 variants resolve, the cached union
equals the per-type sum, and `is_advanced()` matches the split). Verified directly:
GraphQL 15, OAuth 22, Jwt 25, Idor 21, Ssti 27, Grpc 14 payloads, with all 40 distinct
types present in the cached view.

### Explicit rejections carried forward from this evaluation

| Candidate | Lines | Why not |
|---|---|---|
| `fuzzer/payloads/` live-probe 6 | 4,354 | mixes generation with `reqwest` execution; splitting the module would duplicate the seam |
| `vuln/` | 1,273 | spike found a live `crate::error` seam in `cvss.rs`/`exploit.rs`; not yet free |
| `scanner/endpoints.rs` `DEFAULT_ENDPOINTS` | 347 paths | table is pure `&[&str]`, but the file carries `reqwest`/`cli`/`tool-api` coupling |
| `recon/techdetect.rs` | 538 | fingerprint table separable; the file owns an HTTP client |
| `compliance/` | 793 | serde-only and zero I/O — genuinely viable, but low value and 5 consumers |
| `supply_chain/` | 1,962 | serde-only coupling, filesystem I/O, domain-specific |
| `websocket/` | 1,262 | cleanest mechanically (1 path) but only 2 consumers |
| `c2/` + `postex/` | 3,795 | domain crates defensible; `cli::*Args` + `output::convert` coupling |
| `eggsec-resilience` | — | Phase D rejection re-affirmed: `RateLimiter` is keyed on `&str` REST client identity, not a pacing primitive; `governor`/`failsafe` cover the generic shape better |
| `eggsec-utils` | — | Phase A rejection re-affirmed: 13 unrelated files, no coherent boundary |

### Defect found and dispositioned: `utils/redaction.rs` removed (Phase G)

`utils/redaction.rs` — 366 lines, 26 tests, **zero production consumers**. Every
repo-wide `redact_sensitive`/`redact_json` match is a different local function or a
string literal. `eggsec-transport` maintains a narrower debug-redaction surface
(`redacted_headers_debug`, `redact_url_for_debug`) that is related but not equivalent,
and that crate must stay exactly `bytes`/`http`/`url`/`thiserror` per check 108, so
unification is not free.

**Disposition: deleted (milestone 001, Option 3).** The three adoption candidates were
evaluated and rejected on evidence, not assumed:

- `findings::Evidence` — `Evidence::new` always sets `redacted: false`, but the only
  production `Finding` construction (`dispatch/security.rs`, the `search_cve` storage
  mode) sets `evidence: vec![]`. There is no populated evidence path to mask.
- Secret detection — already carries a deliberate, tested masking policy that differs
  from `redaction.rs`: `SecretFinding::value_preview` truncates to 20 chars + `"..."`,
  asserted by `recon/secrets.rs::test_value_preview_truncation`. Adopting regex masking
  there would be a different policy, which is the milestone's own stop condition.
- `nse_bridge.rs` — maps external NSE evidence and already declares redaction
  declaratively via `.with_redaction(RedactionState::None)`.

The decisive observation: **the repo's redaction contract is already declarative**, not a
regex masker. `eggsec-report-model`'s `RedactionState` (`None`/`FullyRedacted`/
`PartiallyRedacted`/`Summarized`) is carried per evidence item and consumed by
`eggsec-output`, `eggsec-db-lab`, and `eggsec-mobile-lab`. `utils/redaction.rs` was a
second, orphaned implementation of a concept the report model already tracks. Deleting
it removes a duplicate rather than a capability.

Deleted test count: **26**. Disclosed as a real loss of tested behavior, not cleanup.

**Check 147** now fails if the file reappears, if `utils/mod.rs` re-exposes the module,
or if an engine-local `fn redact_sensitive`/`fn redact_json` reappears under
`crates/eggsec/src/`.

### Publication: DEFERRED (ADR-0006, accepted 2026-10-05)

All three corpus crates remain internal `publish = false` leaves. ADR-0005 decision 3
separated extraction from publication, and **ADR-0006 records the decision**: defer.

The evidence, in one place:

1. **The corpora are still moving.** 14 commits touched the corpus paths since
   2026-06-01, including `e05711ab Expand payload repository: +550 payloads across 10 new
   modules`. Published payload text is consumer-visible *behavior*, so every later corpus
   improvement would become a semver question.
2. **Release qualification is expensive and already proven so.** `nse-runtime-extraction`
   M007C ran `cargo semver-checks` and found a 74-function major break; `0.2.1` was not
   published. Separately, `scripts/release-package-graph.py` requires that a published
   package's deps not be private and share the release version — so publishing
   `eggsec-secrets`/`eggsec-payloads` means publishing `eggsec-core` too.
3. **No external consumer has been identified.** This is the gate that is unmet, and the
   plan's own stop condition when it is.
4. **`eggsec-payloads` would ship a panicking public API.** Its `get_payloads` routes the
   6 engine-owned advanced types to a documented `unreachable!` — correct behind an engine
   facade, wrong for a general-purpose library whose own enum offers those variants.

Reopening requires all four: a named external consumer; one release cycle without
variant or content changes; a corpus update policy with an owner; and a library-appropriate
payload API. `eggsec-service-db` is closest to ready (no workspace dep, no panic paths);
`eggsec-payloads` is least (needs API design, not just qualification).

Publication is **not** rejected — it is blocked on evidence. If approved later it gets its
own roadmap, and the engine re-export facades stay permanent (ADR-0005 decision 2).

### What would invalidate this decision

If a corpus crate acquires an engine, transport, or frontend dependency; if the engine
re-export facades prove removable (they are not — they are the compatibility contract);
or if the phase-D `eggsec-resilience` rationale is withdrawn, the corresponding
extraction should be folded back into the engine. As with `eggsec-udp-scan`, nothing
mechanically enforces this paragraph — the crates earn their place on the argument
recorded here.

## Guards

- Check 107 fails if `crates/eggsec-net`, `crates/eggsec-web-client`,
  `crates/eggsec-evidence`, or `crates/eggsec-signing` appears, and requires
  this record.
- Check 108 (manifest-graph, `python3` + `tomllib`) enforces the preserved
  direction: DTO crates touch no transport/TLS/HTTP impl; `eggsec-transport`
  stays exactly `bytes`/`http`/`url`/`thiserror`; engine touches no frontend
  crates; path-dependency graph is acyclic.
- Checks 121–123 (Phase C): `eggsec-policy` stays dependency-light (no
  Tokio/HTTP/TLS/filesystem/frontend/engine/transport deps, no `cfg!`
  feature queries, no resolver/authority behavior); engine → policy one-way
  with transport independent; engine policy modules stay facades (no
  redefined core types).
- Check 107 forbids `eggsec-net`, `eggsec-web-client`, `eggsec-evidence`, and
  `eggsec-signing`. `eggsec-udp-scan` is deliberately **not** on that list, but
  nothing mechanically forces the justification above to stay accurate. The
  crate earns its exception on the argument recorded here; if that argument is
  withdrawn, the crate should be folded back into the engine.
- Checks 124–126 (Phase D): loadtest core owns no Reqwest/indicatif/Clap/
  config (core files import none; `indicatif` only in `cli`-gated `run_cli`);
  no `eggsec-loadtest` / `eggsec-resilience` / `eggsec-utils` crate appears;
  `utils::cache` stays removed.
- **Phase G (in progress):** check 114 pins
  `crates/eggsec-service-db/src/lib.rs` as the single canonical service-table owner, and
  fails if the old `crates/eggsec/src/scanner/service_data.rs` reappears or the engine's
  `pub use eggsec_service_db as service_data` facade is dropped. **Check 148** enforces the
  leaf invariant across every extracted corpus crate: `eggsec-core` is the only permitted
  workspace dependency (it is a zero-internal-dependency leaf owning `Severity`), no
  runtime/network/TLS/frontend/persistence dependency, and no `Scope` /
  `ApprovedOperation` / `Capability` reference. `eggsec-service-db` is held to a
  stricter bar separately, since it needs no workspace edge at all. **Check 149**
  additionally pins secret
  detection's canonical owner and engine facade, and freezes the entropy gate at `3.5`
  scoped only to `SecretType::AwsSecretKey`, because retuning or widening it would
  silently change what the scanner detects. **Check 150** polices the payload corpus/probe
  seam: the 6 live-probe modules stay engine-side, the corpus crate fails loudly instead
  of stubbing them, and the cross-variant caches stay engine-side and `LazyLock`.

*Last verified against source: 2026-09-16 (Phase D closure); spot re-verified 2026-09-25: engine `default = []` (`crates/eggsec/Cargo.toml:281`), workspace Tokio `default-features = false` (root `Cargo.toml:47`) with engine narrow set (no `test-util`; `crates/eggsec/Cargo.toml:34`), `eggsec-transport` exactly `bytes`/`http`/`url`/`thiserror` (`crates/eggsec-transport/Cargo.toml:15-18`), no `crates/eggsec-net|web-client|evidence|signing|loadtest|resilience` in workspace members, `eggsec-policy` leaf (no Tokio/HTTP/TLS/filesystem/frontend/engine/transport edge; Checks 121–126 present in `scripts/check-architecture-guards.sh`)*

*Phase G section added 2026-10-05 (analysis only; no code, manifest, or guard changed). Verified at that date: `fuzzer/payloads/` 34 of the 40 payload-type modules free of `reqwest` (7,084 pure-data lines, 233 tests); `recon/secrets.rs` 492 lines / 11 tests / 1 `crate::` reference; `scanner/service_data.rs` 302 lines / 21 tests / 0 `crate::` references / 0 external consumers; `utils/redaction.rs` 366 lines / 26 tests / 0 consumers; check 114 pin at line 3121 and check 126 at line 3457 of `scripts/check-architecture-guards.sh` (baseline `979dca67`); `Severity` re-exported from `eggsec-core` at `crates/eggsec/src/types.rs:16`.*

*Phase G milestone 001 executed 2026-10-05: `utils/redaction.rs` deleted (366 lines, 26 tests), `pub mod redaction;` removed from `crates/eggsec/src/utils/mod.rs`, and check 147 added in the shape of check 126 — verified to fail on all three of its conditions when the file, the module declaration, and engine-local `redact_sensitive`/`redact_json` are artificially reintroduced. No manifest change: `regex` remains a direct engine dependency for 12 other modules.*

*Phase G milestones 002 and 003 executed 2026-10-05: `eggsec-service-db` and `eggsec-secrets` created as internal `publish = false` leaves, both re-exported through permanent engine facades with zero consumer diffs. Checks 114, 148, and 149 present and demonstrated to fail. Milestone 004 remains unimplemented. Correction applied during 003: `build_patterns()` holds **25** patterns covering 20 of 30 `SecretType` variants (the repo's `architecture/recon.md` was right; an earlier draft of this Phase G record and the plans said 26).*
