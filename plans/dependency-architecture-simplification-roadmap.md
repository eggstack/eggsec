# Dependency, Architecture, and Verification Simplification Roadmap

## Status

Ready for implementation.

This document is a handoff roadmap. It authorizes corrective engineering and
structural simplification only. It does not authorize feature expansion, removal
of user-visible capability, weakening of scope enforcement, or restoration of
automated package publication.

## Purpose

Eggsec has accumulated a broad set of security-assessment capabilities, multiple
frontends, Python bindings, a daemon/runtime architecture, domain crates, and a
large optional dependency surface. The current design contains several valuable
boundaries, but the repository now has two opposing forms of complexity:

1. security-critical behavior is represented by several partially overlapping
   metadata and policy layers that can drift independently;
2. lightweight consumers still inherit application-layer dependencies because
   the main `eggsec` library remains a composition root for CLI parsing,
   notification, logging, configuration watching, and other adapters.

The result is a repository that is harder to reason about than necessary, larger
binaries than the feature set requires, duplicated dependency generations,
opaque advisory suppression, and a verification contract that validates many
manually mirrored representations while still missing semantic defects.

This roadmap corrects the identified authorization and scope defects first,
then replaces duplicated metadata with generated or shared definitions, then
reduces dependency and binary topology without removing capability. It ends by
modernizing upstream dependencies and simplifying CI around direct behavioral
and boundary checks.

## Non-negotiable outcomes

The completed line of work must preserve all of the following:

- human-operated CLI and TUI remain first-class interfaces;
- strict automated surfaces remain fail-closed and cannot use manual overrides;
- `ApprovedOperation` or its replacement remains an unforgeable proof of the
  exact operation and target authorized for dispatch;
- scope rules remain explicit, auditable, and consistent across CLI, TUI, REST,
  MCP, gRPC, agent, daemon, and Python paths;
- no user-visible capability is removed merely to reduce binary size;
- optional high-cost domains remain available through explicit features or
  separate artifacts;
- Python bindings remain a supported library interface;
- Rust crate and Python package publication remain manual maintainer actions;
- GitHub Actions does not publish to crates.io, PyPI, TestPyPI, or GitHub
  Releases;
- verification remains proportional to the project and favors direct tests over
  generated evidence or static string policing;
- native dependencies are replaced only when a maintained Rust alternative can
  preserve protocol and interoperability behavior.

## Confirmed problem classes

### Authorization token incompleteness

Target-required operation metadata can currently produce a descriptor without a
target. The enforcement layer only requires explicit scope when a target is
present, and dispatch only compares targets when the approved descriptor
contains one. The intended type-level proof therefore depends on entrypoint
validation rather than being self-contained.

### Fail-open feature availability

Runtime feature detection defaults unknown feature names to available. The
runtime matcher, Cargo feature list, operation metadata, domain descriptors, and
tests are maintained through separate representations. A misspelled or omitted
feature can therefore pass policy/preflight checks.

### Metadata ownership drift

Command registration, operation metadata, domain descriptors, tool registration,
frontend visibility, protocol exposure, and feature classifications overlap.
Risk, mode, exposure, and command identity can disagree while each individual
registry remains internally valid.

### Scope and DNS coupling

Scope parsing, hostname resolution, address classification, and authorization
are coupled. Resolver ordering can influence authorization, loopback handling is
inconsistent between literal and hostname forms, and a hostname with multiple
addresses is not represented as a bound authorization set.

### Dependency and advisory debt

The repository carries long-lived advisory ignores, duplicate dependency
versions, outdated optional drivers, mixed TLS providers, and native dependency
paths whose actual necessity is not consistently documented. Some ignores refer
to versions already claimed as upgraded or lack an expiry/review process.

### Dependency boundary leakage

The main engine library contains or depends on CLI and application adapters.
Consequently, Python and other library consumers inherit dependencies such as
Clap, progress UI, logging subscribers, file watching, SMTP, HTML parsing, and
certificate generation even when those facilities are not used.

### Verification overreach and blind spots

Mandatory CI is smaller than it was historically, but it still uses a
hand-curated serial test list, static grep guards, always-on portability jobs,
and manually mirrored feature validation. These checks impose maintenance cost
without reliably validating semantic authorization invariants.

## Target architecture

The target architecture is not a rewrite. It is a controlled simplification of
current boundaries:

```text
leaf contracts and policy types
    eggsec-core / policy-operation metadata / runtime DTOs
                    |
                    v
engine services and domain execution
    scanner, recon, fuzz, WAF, load, optional domain crates
                    |
          typed operation service API
                    |
       +------------+-------------+
       |            |             |
      CLI          TUI         Python/API
  parsing/output  rendering    adapters only

runtime protocol/client DTOs        daemon server
(no persistence/server deps)   (SQLite/transports/server lifecycle)
```

The principal design rules are:

- constructors enforce operation target policy before approval;
- one declarative operation definition owns identity, risk, target policy,
  feature requirements, and surface exposure;
- frontend command parsing is not part of the engine library;
- optional adapters own their optional dependencies;
- daemon clients do not link daemon server persistence;
- one TLS provider is selected for each distributed artifact;
- compile-time feature availability is exhaustive and fails closed;
- direct semantic tests replace duplicated registry snapshots and broad grep
  guards where possible.

## Phase sequence

### Phase A — Authorization token and target-binding correction

Plan: [`dependency-architecture-phase-a-authorization-target-binding.md`](dependency-architecture-phase-a-authorization-target-binding.md)

Make operation construction fallible and target-policy aware. Ensure strict
approval cannot be issued for a target-required operation without a target,
bind normalized target identity into the approval token, validate surface/profile
consistency, and add semantic bypass regression tests.

Exit condition: no strict or manual dispatch path can use an approval token for a
different or absent target, regardless of entrypoint validation.

### Phase B — Scope resolution and address-set correctness

Plan: [`dependency-architecture-phase-b-scope-resolution-correctness.md`](dependency-architecture-phase-b-scope-resolution-correctness.md)

Separate DNS resolution from scope policy. Represent all resolved addresses,
define mixed-address and rebinding behavior, make explicit loopback/private
scope rules consistent, and bind approved address decisions where network
connections require them.

Exit condition: authorization is deterministic across literal IP, hostname, URL,
IPv4/IPv6, and multi-address targets and does not depend on resolver ordering.

### Phase C — Exhaustive feature availability registry

Plan: [`dependency-architecture-phase-c-feature-registry.md`](dependency-architecture-phase-c-feature-registry.md)

Replace fail-open feature matching and duplicated snapshots with one exhaustive
feature registry or generated definition used by policy, domain availability,
diagnostics, and tests.

Exit condition: every feature referenced by operation or domain metadata is
recognized by the exact runtime availability function, and unknown features are
unavailable by default.

### Phase D — Operation, command, domain, and tool metadata consolidation

Plan: [`dependency-architecture-phase-d-metadata-consolidation.md`](dependency-architecture-phase-d-metadata-consolidation.md)

Establish one source of truth for operation identity, risk, mode, target policy,
feature gates, aliases, and surface exposure. Derive command/tool/domain views
where practical and retire transitional registries and documentary snapshots.

Exit condition: contradictory risk, mode, feature, alias, or exposure metadata
cannot compile or pass tests because those views are generated from one
canonical declaration.

### Phase E — Advisory cleanup and dependency security remediation

Plan: [`dependency-architecture-phase-e-advisory-dependency-remediation.md`](dependency-architecture-phase-e-advisory-dependency-remediation.md)

Audit every `cargo-deny` exception, remove stale suppressions, upgrade direct
security-relevant dependencies such as PyO3 and Quick-XML, and establish a
bounded exception format with dependency path, affected API assessment, owner,
and review deadline.

Exit condition: advisory checks are meaningful, current suppressions are
narrowly justified, and no ignore remains solely because it once matched an old
lockfile state.

### Phase F — Engine/application boundary and library-size reduction

Plan: [`dependency-architecture-phase-f-engine-application-boundary.md`](dependency-architecture-phase-f-engine-application-boundary.md)

Move CLI parsing, terminal/application logging, notification transports, file
watching, and other frontend adapters out of the central engine dependency
surface. Expose typed engine services that CLI, TUI, Python, and protocol
adapters consume.

Exit condition: `eggsec-python` and headless/library consumers no longer link
CLI-only or operator-output dependencies unless they explicitly enable an
adapter feature.

### Phase G — Daemon/TUI topology, TLS provider, and duplicate dependency cleanup

Plan: [`dependency-architecture-phase-g-binary-topology-and-tls.md`](dependency-architecture-phase-g-binary-topology-and-tls.md)

Split daemon client protocol from daemon server/persistence, prevent the default
TUI artifact from linking bundled SQLite and server internals, select one Rustls
provider per artifact, narrow Tokio/reqwest features, and align duplicate Tower,
WebSocket, XML, and related dependency generations.

Exit condition: standard CLI/TUI and Python artifacts retain capability while
showing a materially smaller and simpler dependency graph.

### Phase H — Upstream modernization, MSRV, and justified native-dependency reduction

Plan: [`dependency-architecture-phase-h-upstream-msrv-native-deps.md`](dependency-architecture-phase-h-upstream-msrv-native-deps.md)

Modernize optional database, Kubernetes, SQLite, Python, and protocol adapters in
bounded batches. Establish a truthful tested MSRV. Replace native TLS/OpenSSL,
libssh2, SQLite, or protoc dependencies only where maintained Rust alternatives
or checked-in generated sources preserve required behavior.

Exit condition: the declared MSRV is tested, supported upstream lines are used
where feasible, and every remaining native dependency has a documented owner and
reason.

### Phase I — CI and verification simplification

Plan: [`dependency-architecture-phase-i-ci-verification-simplification.md`](dependency-architecture-phase-i-ci-verification-simplification.md)

Replace hand-curated test invocation lists with package/feature-level commands,
move nonessential portability and dependency diagnostics to scheduled/manual
checks, remove redundant architecture grep guards after typed invariants land,
and retain manual release cadence.

Exit condition: mandatory CI is a compact Linux-first behavioral contract with
narrow change-aware Python coverage and optional portability/dependency checks.

### Phase J — Measurement, documentation reconciliation, and closure

Plan: [`dependency-architecture-phase-j-measurement-and-closure.md`](dependency-architecture-phase-j-measurement-and-closure.md)

Measure dependency trees and artifact sizes before and after the structural
changes, document the final crate/dependency boundaries, remove stale metadata
and verification guidance, and record closure without creating another evidence
framework.

Exit condition: the roadmap acceptance criteria are demonstrated with direct
commands, size/dependency deltas, and current documentation.

## Ordering and dependency rules

Phases must normally be implemented in order.

- Phase A precedes metadata refactoring because the authorization proof must be
  corrected before its declarations move.
- Phase B may share target-normalization types with Phase A but must not block the
  immediate targetless-approval correction.
- Phase C precedes Phase D so the consolidated metadata model has a fail-closed
  feature representation.
- Phase D precedes major dependency-boundary work because application adapters
  need a stable typed operation service rather than another transitional
  registry.
- Phase E may begin dependency-path research in parallel, but upgrades that touch
  public APIs should land after Phase D ownership is clear.
- Phase F precedes Phase G because daemon/TUI artifact composition should consume
  the cleaned engine boundary.
- Phase H upgrades optional adapters after core artifact boundaries are stable.
- Phase I is intentionally late so CI is simplified around final direct checks,
  not repeatedly rewritten during every structural phase.
- Phase J is a bounded closure pass, not an invitation to add new abstraction or
  verification layers.

Every phase must leave `main` buildable for the supported default profile. Large
phases should be split into reviewable commits, but temporary compatibility
shims must have an explicit removal point within the same phase or the next
immediate phase.

## Deliberate exclusions

This roadmap does not include:

- removing assessment, proxy, NSE, mobile, database, packet, stress, agent, or
  Python capabilities;
- changing authorization policy to make tests or dependency migrations easier;
- replacing Tokio, reqwest, Rustls, Axum, PyO3, SQLx, Ratatui, or other mature
  dependencies solely to reduce the dependency count;
- rewriting SQLite persistence without demonstrated artifact or operational
  benefit;
- rewriting SSH/NSE compatibility around an unproven Rust implementation;
- introducing a new build system, monorepo tool, package manager, or code
  generator service;
- making `full` the default build profile;
- adding hosted release publication, release bots, or tag-triggered packaging;
- retaining duplicate metadata merely to satisfy historical plans;
- creating evidence bundles, maturity ledgers, or generated compliance reports
  for this corrective line.

## Roadmap-level acceptance criteria

The roadmap is complete only when all of the following are true:

1. A target-required operation cannot be approved without a normalized target.
2. A dispatch request cannot use an approval token for another target.
3. Surface/profile mismatches are rejected before approval.
4. Hostname authorization evaluates a defined address set rather than the first
   resolver result.
5. Explicit loopback/private scope rules behave consistently for literal and
   hostname targets.
6. Unknown compile-time feature names are unavailable.
7. Every operation/domain feature reference is validated against the runtime
   feature registry used in production.
8. Operation risk, mode, target policy, feature requirements, aliases, and
   surface exposure have one canonical owner.
9. Transitional command/tool/domain registries are either derived or removed.
10. `cargo deny check` has no stale version-based ignores and every retained
    exception has an owner and review date.
11. PyO3, Quick-XML, and other direct security-relevant dependencies are on
    supported versions or have a documented bounded blocker.
12. The engine crate no longer unconditionally owns CLI parsing, progress UI,
    terminal logging subscribers, notification transports, or file watching.
13. Python and headless library artifacts do not link those adapters by default.
14. The default TUI path does not link daemon server persistence solely for
    client connectivity.
15. Exactly one intended Rustls crypto provider is linked per primary artifact.
16. Reqwest and Tokio features are selected by consuming crate/profile rather
    than broadly activated without ownership.
17. Duplicate major dependency generations are reduced where API compatibility
    permits.
18. The declared Rust MSRV is tested directly.
19. Remaining native dependencies and external build tools have documented,
    feature-scoped ownership.
20. Mandatory CI uses direct Cargo/Python commands rather than a manually mirrored
    test inventory.
21. Portability, dependency policy, broad feature profiles, and slow diagnostics
    are optional or change-aware unless they protect a demonstrated merge-time
    defect class.
22. GitHub Actions still cannot publish packages or releases.
23. Standard CLI, TUI, daemon, Python, and optional-domain capabilities remain
    available through documented profiles.
24. Before/after artifact size and dependency-tree measurements are recorded for
    the standard CLI/TUI, headless CLI, daemon, and Python extension.
25. Documentation describes the final architecture and verification contract
    without stale transitional claims.

## Handoff guidance

Implementation agents should prefer deleting duplicated representations over
adding synchronization tests between them. A generated table or typed constructor
is better than another architecture guard that compares strings.

Binary-size changes must be measured by artifact profile. A dependency removed
from the headless CLI but retained in the full TUI is still a successful boundary
improvement. Do not use the `full` aggregate as the only size benchmark.

Dependency replacement proposals must identify the current behavior, the
candidate alternative, interoperability requirements, migration cost, and
rollback path. “Pure Rust” is not sufficient justification on its own.

Verification should be narrowly proportional to the defect class. Security
invariants require direct semantic tests. Dependency and portability diagnostics
may be scheduled. Publication remains manual and outside this roadmap's
implementation commands.
