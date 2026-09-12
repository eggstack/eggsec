# Network dependency and supply-chain hardening roadmap

Status: Ready for handoff

Date: 2026-09-11

Baseline: `c49ced023e302ea4a32d0e06045818d42b3bbdea`

## Purpose

Follow the completed dependency/architecture and frontend/runtime convergence work with a bounded pass focused on residual dependency ownership, outbound-network policy, and software-supply-chain hardening.

The current workspace is already meaningfully decomposed by domain. The remaining problem is capability ownership: `eggsec`, `eggsec-agent`, `eggsec-nse`, and `eggsec-web-proxy` independently own overlapping HTTP/TLS behavior, while the engine crate still carries process-host and broad networking dependencies. The sibling `eggfetch` and `eggress` projects now provide enough lower-level networking functionality to consolidate some of that ownership, but only if EggSec's authorization/scope semantics remain authoritative.

This roadmap must not expand offensive capability. It reduces duplicated networking implementations and narrows dependency/capability boundaries while preserving the existing defense-validation authorization model.

## Confirmed current state

At the baseline SHA:

- `crates/eggsec` directly owns `reqwest`, `rustls`, `tokio-rustls`, `webpki-roots`, Hickory DNS, `rcgen`, crypto primitives, and many feature-specific integrations.
- `eggsec-agent` directly depends on Reqwest + Rustls.
- `eggsec-nse` directly depends on Reqwest + Rustls; enabling NSE additionally pulls Lua/native TLS/OpenSSL compatibility dependencies.
- `eggsec-web-proxy` directly owns its interception TLS stack and also pulls Reqwest for ordinary outbound client work.
- `eggsec-core` remains dependency-light and must stay so.
- `eggsec-cli` already exists as a process host, yet `eggsec` still defaults to the `cli` feature.
- workspace-level Tokio enables a broad feature set; Cargo workspace dependency features are additive, so every `tokio.workspace = true` consumer inherits that baseline capability set.
- `eggfetch-core` is an async Hyper/Rustls engine with request/response types, redirect handling, auth, pooling, proxy support, cookies, compression, phase-aware timeouts, retries, and HTTP/1-3 feature gates.
- `eggfetch-core` currently performs direct-connector DNS with `tokio::net::lookup_host`; it does not expose the resolver/connection authorization callback required for EggSec to enforce post-resolution scope at the transport boundary.
- Eggfetch redirect handling already strips sensitive headers appropriately, but EggSec must additionally authorize every redirect target before dispatch.
- `eggress-uri` is a small parser/model crate with credential-redacting proxy endpoint models.
- `eggress-routing` has useful host/CIDR/port routing semantics, but it carries materially more runtime/routing dependencies; adoption is therefore a measured decision, not an unconditional goal.
- normal PR CI runs `make check`; `cargo deny check` is currently only in optional/weekly `make check-full`.
- `deny.toml` has two documented advisory exceptions, while `.cargo/audit.toml` contains a much broader historical ignore set.
- GitHub Actions currently use floating major/stable refs rather than full commit SHAs.
- no Dependabot configuration is present under `.github/` at the baseline.

## External research constraints incorporated into this roadmap

- Cargo workspace dependency features are additive to member-selected features, and Cargo feature unification uses the union of enabled features. Tokio capability minimization therefore requires narrowing the workspace baseline and selecting features per member, not merely editing leaf manifests.
- `cargo-deny` supports `unknown-registry = "deny"`, `unknown-git = "deny"`, and `required-git-spec = "rev"`; use these to make dependency source trust explicit.
- GitHub's current secure-use guidance states that a full-length action commit SHA is the immutable pinning mechanism for third-party actions.
- GitHub Dependency Review can block pull requests that introduce vulnerable dependencies and supports read-only `contents` permissions. It is complementary to RustSec/cargo-deny rather than a replacement.
- Cargo Audit ignores are unconditional advisory suppressions. A large stale ignore list must not remain a second, weaker policy source alongside Cargo Deny.

## Target architecture

```text
process hosts
  eggsec-cli / eggsec-daemon / eggsec-python
                     |
                     v
                  eggsec
          orchestration + domains
                     |
          +----------+----------+
          |                     |
          v                     v
 eggsec-transport          specialized I/O
 policy + DTO contract     DB/raw/packet/etc.
          |
          v
 eggsec-transport-eggfetch
          |
          v
     eggfetch-core
          |
   optional narrow reuse
          v
 egress-uri / selected low-level policy primitives
```

Domain crates must not instantiate arbitrary outbound HTTP clients or choose TLS/resolver policy independently once migrated. `eggsec-web-proxy` remains the owner of inbound/interception/MITM behavior; only its ordinary outbound helper requests move to the shared transport.

## Security invariants

Every implementation phase must preserve these invariants:

1. EggSec authorization/scope policy remains authoritative over all sibling-library routing behavior.
2. A hostname being authorized does not implicitly authorize every address returned by later DNS resolution.
3. Scope is re-evaluated after URL canonicalization, DNS resolution, redirects, retries that re-resolve, and any proxy/upstream route selection that changes the network destination.
4. Redirects may never broaden authorized scope merely because the HTTP client supports following them.
5. Authorization, cookies, proxy credentials, and other sensitive headers are never forwarded to an unauthorized or cross-origin destination.
6. Insecure TLS remains explicit, opt-in, auditable, and feature/policy gated.
7. NSE compatibility dependencies remain isolated; migrating HTTP transport must not spread OpenSSL/native-tls/Lua dependencies into ordinary builds.
8. The interception proxy retains its MITM/server TLS boundary; a client library does not replace that behavior.
9. No migration is accepted solely because manifest dependency lines decrease. Final artifact graphs, feature sets, and security behavior must improve or remain bounded.
10. `eggsec-core` remains free of concrete HTTP/TLS/runtime implementations.

## Ordered plans

1. [`network-dependency-phase-a-baseline-invariants.md`](network-dependency-phase-a-baseline-invariants.md)
2. [`network-dependency-phase-b-scoped-transport-contract.md`](network-dependency-phase-b-scoped-transport-contract.md)
3. [`network-dependency-phase-c-eggfetch-readiness-adapter.md`](network-dependency-phase-c-eggfetch-readiness-adapter.md)
4. [`network-dependency-phase-d-outbound-client-migration.md`](network-dependency-phase-d-outbound-client-migration.md)
5. [`network-dependency-phase-e-egress-and-capability-segregation.md`](network-dependency-phase-e-egress-and-capability-segregation.md)
6. [`network-dependency-phase-f-supply-chain-ci-hardening.md`](network-dependency-phase-f-supply-chain-ci-hardening.md)
7. [`network-dependency-phase-g-closure-measurement.md`](network-dependency-phase-g-closure-measurement.md)

Phases A-C are foundational and ordered. Phase D performs consumer migrations only after the transport contract and Eggfetch hooks are proven. Phase E then evaluates selective Egress reuse and finishes capability segregation against the stabilized transport seam. Phase F may proceed in parallel once Phase A records the dependency baseline, except changes that would invalidate migration measurements should be coordinated. Phase G is last.

## Global non-goals

- No new scanning, exploitation, persistence, C2, wireless, or evasion capability.
- No wholesale rewrite of EggSec networking around Egress.
- No direct dependency from low-level sibling networking crates back into EggSec.
- No dependency cycle among `eggsec`, `eggfetch`, and `eggress`.
- No replacement of the interception proxy with Eggfetch.
- No gratuitous one-module-per-crate decomposition.
- No branch-protection or organization-administration changes that cannot be represented in repository code; document such optional settings separately.

## Global completion criteria

The roadmap is complete only when:

- the supported EggSec outbound HTTP paths use a single scope-aware transport seam;
- migrated domains no longer expose Reqwest types in public/internal cross-domain APIs;
- Reqwest is removed from every migrated crate and any remaining use has an explicit documented owner/reason;
- Eggfetch has the hooks required to authorize resolved destinations and redirect hops without bypassing EggSec policy;
- any Egress dependency has demonstrated semantic and dependency-graph value; otherwise the decision record explicitly rejects it;
- `eggsec` can build as a library with `default = []` and process-host dependencies stay in process-host crates;
- Tokio features are selected by crate rather than inherited as one broad workspace capability baseline;
- Cargo Deny/source policy is a required dependency-change/PR gate and Cargo Audit policy cannot silently diverge;
- third-party Actions are immutable-SHA pinned and automated update handling is configured or explicitly documented;
- before/after artifact dependency graphs and required security regression tests are recorded in Phase G.

## Handoff discipline

Each phase must append a completion record containing baseline/final SHAs, exact commands, changed dependency paths, security tests, and residual debt. If a sibling-repository prerequisite is required, record its repository, commit/release, API contract, and EggSec integration pin before marking the dependent EggSec phase complete.
