# Phase E — Egress reuse decision and capability segregation

Status: Ready for handoff

Date: 2026-09-11

Depends on: Phases A-D

## Purpose

Evaluate selective reuse of `eggress` primitives only where they reduce duplicated policy/code without widening the final artifact graph, then finish the remaining process-host/runtime capability segregation in EggSec.

This phase has explicit decision gates. “Use another Eggstack crate” is not itself an architectural win.

## Workstream 1 — Egress API/graph decision record

Evaluate these candidates independently:

### `eggress-uri`

Current characteristics are favorable: it is small and owns proxy endpoint/chain parsing plus credential-redacted URI models. Compare it against Eggfetch/EggSec proxy URI parsing and redaction.

Adopt only if it can become the canonical representation without semantic loss for the proxy forms EggSec/Eggfetch support. Specific questions:

- Can the model represent EggSec's required HTTP/SOCKS routes without importing unrelated protocols into policy logic?
- Are credential serialization/debug guarantees at least as strict as current Eggfetch behavior?
- Would adoption remove duplicate parsing/redaction code rather than add an adapter layer indefinitely?
- Is dependency direction acyclic (`eggress-uri` -> Eggfetch/EggSec consumer), with no dependency back to EggSec?

### `eggress-routing`

Current routing supports host exact/suffix/regex, destination CIDR/port, upstream selection, and related policy. It is also materially heavier (`regex`, `ipnet`, `arc-swap`, Tokio, tracing, routing/core models).

Do not adopt it simply for CIDR matching. Measure:

```text
cargo tree -p eggress-routing
cargo tree -p eggsec-transport-eggfetch before/after candidate integration
cargo tree -d before/after
```

Adopt only if EggSec/Eggfetch actually need shared route/upstream policy and the dependency increase is offset by removal of equivalent code/dependencies. EggSec authorization remains authoritative; Egress routing may select an allowed route but may not decide what is authorized to scan.

### Protocol/transport crates

Default disposition: do not consume `eggress-embed`, `eggress-runtime`, server, protocol, SSH, QUIC, or proxy stacks wholesale from EggSec in this roadmap. Reconsider individual low-level crates only with a specific duplication case and graph evidence.

## Preferred ecosystem dependency direction

Preserve this acyclic direction where reuse is justified:

```text
narrow egress primitives
        |
        v
     eggfetch
        |
        v
 eggsec adapter
        |
        v
  eggsec domains
```

Direct `eggsec -> eggress-routing` is acceptable only if the policy is genuinely EggSec-specific and cannot naturally live below Eggfetch; document why. Never introduce `eggfetch -> eggsec` or `eggress -> eggsec`.

## Workstream 2 — Make `eggsec` library-default empty

The dedicated `eggsec-cli` process-host crate already owns Clap/log subscriber/process concerns. Remove `cli` from `eggsec` default features and target:

```toml
[features]
default = []
```

Move remaining CLI-only definitions/dispatch/config parsing out of the engine crate when doing so does not duplicate canonical operation metadata. `eggsec-cli` should depend on the engine; the engine should not require CLI capability to be a normal library.

Add checks for:

```text
cargo check -p eggsec --no-default-features
cargo check -p eggsec
cargo check -p eggsec-cli --no-default-features
cargo check -p eggsec-cli
```

Preserve command compatibility and TUI/daemon dispatch invariants from the completed frontend/runtime roadmaps.

## Workstream 3 — Narrow workspace Tokio feature inheritance

Current workspace Tokio enables a broad feature set including multi-thread runtime, networking, filesystem, process, I/O std, signals, and test utilities. Cargo workspace dependency features are additive, so every inherited Tokio dependency starts from that broad baseline.

Refactor to the narrowest safe workspace baseline, preferably:

```toml
tokio = { version = "1", default-features = false }
```

Then select features per crate according to actual capability needs.

Expected ownership examples:

- DTO/domain-only crates: no Tokio where possible, or `sync`/`time` only if genuinely required;
- transport implementations: `rt`, `net`, `time`, `io-util`, `sync` as needed;
- CLI/daemon process hosts: `rt-multi-thread`, `signal`, `process`, `fs`, `io-std` as needed;
- test-only `test-util`: dev dependency/feature, not production baseline.

Run the full feature matrix because Cargo unification can hide missing feature declarations when the whole workspace is built together. Add package-isolated checks for crates whose manifests are intentionally narrow.

## Workstream 4 — Evaluate remaining crate extraction by capability, not module count

After transport migration, inspect `crates/eggsec` again and extract only boundaries that produce measurable dependency/security isolation.

Candidate decision gates:

### `eggsec-net` / target-resolution policy

Create only if canonical DNS/target normalization/scope binding still has enough cohesive implementation to deserve an independently testable dependency boundary. If Phase B's `eggsec-transport` already owns the necessary contract and the canonical policy types live cleanly elsewhere, do not create another crate.

### web assessment client vs interception proxy

If web scanning/crawling/client-domain logic still shares `eggsec-web-proxy` simply because both speak HTTP, split the client assessment domain from interception/server functionality. The split is justified when it lets non-proxy builds avoid `rcgen`, server TLS, h2 interception, WebSocket/protobuf dependencies.

### evidence/signing crypto

Extract an evidence/signing crate only if multiple domain crates independently own HMAC/SHA/key-erasure/bundle-signing semantics and the extraction removes duplicate crypto policy/dependencies. Do not centralize unrelated hashing used by scanners merely for a clean manifest.

For every proposed new crate require a one-page decision record containing: dependency delta, API boundary, capability isolated, cycle analysis, migration cost, and rejected alternative.

## Workstream 5 — Strengthen architecture guards

Add durable guards for the final dependency direction, for example:

- `eggsec-core` may not depend on transport/TLS/HTTP implementation crates;
- domain crates migrated in Phase D may not directly add Reqwest;
- `eggsec-transport` may not depend on Eggfetch/Egress concrete implementations;
- process-host-only dependencies must not reappear as engine defaults;
- no workspace dependency cycle/forbidden crate edge.

Prefer `cargo metadata`/a small graph-check script over brittle grep when checking manifest edges.

## Required verification

```text
cargo check --workspace --no-default-features
cargo check -p eggsec
cargo check -p eggsec-cli
cargo tree -d
cargo tree -e features
make check-feature-profiles
make check-features-individual
make check
make check-python
make test-architecture-guards
```

If an Egress crate is adopted, run its relevant unit/property tests and record exact version/SHA.

## Acceptance criteria

1. Egress reuse is decided per narrow crate with measured before/after dependency graphs.
2. No umbrella Egress runtime is added merely for consolidation optics.
3. EggSec authorization remains above and independent from route-selection policy.
4. `eggsec` defaults to no process-host feature and normal library builds remain supported.
5. Tokio production features are declared by consuming crate rather than inherited as one broad workspace capability set.
6. Any new crate extraction demonstrates a real capability/dependency boundary; unnecessary candidates are explicitly rejected.
7. Architecture guards enforce the intended dependency direction without encoding incidental implementation details.
8. Full feature, TUI, daemon, Python, and no-default profiles remain green.

## Expected files touched

- root and member `Cargo.toml` files;
- `crates/eggsec-cli/` and residual CLI modules in `crates/eggsec/`;
- optionally narrow Egress/Eggfetch integration points;
- optionally one or more justified capability crates;
- architecture graph-check script/tests/docs;
- this plan completion record.
