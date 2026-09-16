# Crate boundary and reusable-library consolidation roadmap

Status: Executed (2026-09-16). All four phases implemented; see each phase
plan's completion record (Phase D closes the roadmap).

Date: 2026-09-16

Baseline: `04eb25b6b6614c6d6f38c4fb8d0615f26a736cb0`

Baseline tree: `11314015e7dd24ad382add0fdfc52783e96b648c`

## Purpose

Follow the executed architecture-convergence and network-dependency hardening work with a bounded ownership pass. The workspace is already substantially decomposed; this roadmap is not permission for another broad crate-splitting campaign. It addresses the remaining places where a crate owns responsibilities that do not match its advertised boundary, and it promotes code to a new crate only when the move removes dependency pressure, creates a durable capability boundary, or yields a genuinely reusable library surface.

The audit that produced this roadmap identified three high-confidence ownership problems:

1. `eggsec-output` is both a report formatter and a data/orchestration owner. Domain crates depend on it for report DTOs even when they do not need HTML/JUnit/SARIF/CSV behavior, while the same crate also owns scan scheduling and frontend-oriented session persistence.
2. `crates/eggsec/src/config/` now contains the complete authorization/enforcement engine in addition to configuration loading. Policy, scope, approval-token, operation-metadata, and decision code have become a distinct semantic subsystem.
3. `crates/eggsec/src/utils/` remains a mixed ownership bucket. It contains domain helpers, UI helpers, Reqwest client construction/pooling, rate control, fault-tolerance primitives, parsing, redaction, privilege detection, and scanner-specific knowledge.

A fourth area, HTTP load testing, is a promising future crate/library boundary but is not ready to extract while it still constructs Reqwest clients and imports engine configuration, terminal progress, report, and common CLI-derived types.

## Relationship to prior decisions

This roadmap preserves the 2026-09-13 Phase E decisions in `architecture/capability_segregation.md`:

- do not create `eggsec-net` merely to move target/DNS code;
- do not create a web-client crate merely because several modules speak HTTP;
- do not create an evidence/signing crate for unrelated HMAC use cases;
- do not add a generic `eggsec-utils` crate;
- keep `eggsec-transport` dependency-light and implementation-neutral;
- keep `eggsec-core` small.

The candidates in this roadmap are different. `eggsec-report-model` separates pure report data contracts from formatters and removes inappropriate `eggsec-output` edges. The proposed `eggsec-policy` boundary owns the complete authorization semantic domain rather than extracting only network-resolution policy. Both must still pass measured dependency/cycle gates before the move is accepted.

## Confirmed current state

At the baseline:

- the workspace has 18 members;
- `eggsec-core` is the dependency-light shared primitive crate and must remain so;
- `eggsec-transport` is already a strong reusable boundary with no workspace dependencies and only `bytes`, `http`, `url`, and `thiserror` in production;
- `eggsec-transport-eggfetch` provides the concrete Eggfetch adapter and must remain below the engine policy layer;
- `eggsec-output` owns report DTOs, normalized evidence envelopes, formatters, baseline/trend/dedup logic, scan scheduling/cron/queue code, and scan/TUI-like session persistence;
- `eggsec-db-lab`, `eggsec-mobile-lab`, and `eggsec-web-proxy` consume `eggsec-output` report types, creating a wider dependency relationship than their DTO needs require;
- `config/` owns `policy.rs`, `policy_approval.rs`, `policy_catalog.rs`, `policy_decision.rs`, `policy_target.rs`, `scope.rs`, scope/address/resolver/spec/transport modules, feature registry, and ordinary configuration loading;
- `policy_decision.rs` is a major standalone hotspot and the policy/scope cluster is materially larger than ordinary configuration code;
- `utils/` contains 20 submodules with unrelated ownership;
- rate-control behavior exists in both `utils::rate_limiter` and `eggsec-output::schedule`, while `eggsec-tool-core` separately and appropriately owns protocol/config/status DTOs;
- `eggsec-agent` already owns a richer task scheduler with priority, delayed retry, leasing, lease reclamation, assignment, and task outcomes;
- `utils::client_pool` pre-creates multiple `reqwest::Client` values even though each client already owns an internal connection pool;
- `loadtest::runner` directly constructs Reqwest clients and imports engine error/config/common HTTP/report/progress concerns.

## Target architecture

The preferred end-state is:

```text
                         eggsec-core
                         /    |    \
                        /     |     \
                       v      v      v
          eggsec-tool-core  eggsec-report-model  eggsec-policy
                                  |                 |
                     +------------+                 |
                     |                              |
                     v                              v
                eggsec-output                    eggsec
             rendering/analysis             composition root
                     ^                       /            \
                     |                      v              v
         domain report producers      eggsec-transport   specialized I/O
                                     / contract only \
                                              |
                                              v
                                 eggsec-transport-eggfetch
                                              |
                                              v
                                         eggfetch-core

eggsec-runtime -> eggsec-ui-model / eggsec-daemon-protocol
```

`eggsec-policy` must not depend on `eggsec-transport`; the engine owns the adapter that translates transport checkpoint facts into policy decisions. This preserves the existing transport leaf invariant and avoids recreating the rejected `eggsec-net` middle layer.

## Ordered plans

1. `crate-boundary-consolidation-phase-a-ownership-and-primitive-cleanup.md`
2. `crate-boundary-consolidation-phase-b-report-model-extraction.md`
3. `crate-boundary-consolidation-phase-c-policy-enforcement-extraction.md`
4. `crate-boundary-consolidation-phase-d-loadtest-resilience-reuse-closure.md`

Phase A is mandatory preparation and creates no crate. Phase B is the first extraction because it has the clearest measurable dependency payoff. Phase C first makes policy evaluation independently compilable, then creates `eggsec-policy` only after the dependency/cycle gate passes. Phase D decouples load testing, evaluates whether another crate is justified, evaluates reusable resilience primitives only against real consumers, and records closure measurements.

## Global invariants

1. No phase adds offensive/security-testing capability; this is ownership, dependency, and maintainability work only.
2. A new crate is not accepted because a module is large. It must remove an inappropriate dependency edge, isolate a capability/dependency set, or expose a stable independently useful contract.
3. `eggsec-core` remains free of runtime, HTTP/TLS implementations, UI, database, browser, packet, and agent behavior.
4. `eggsec-transport` remains the implementation-neutral scoped HTTP contract and must not depend on Eggfetch, Eggsec policy, runtime, or frontends.
5. `eggsec-policy` if created must be deterministic and dependency-light: no Tokio, Reqwest, Rustls, Axum, tonic, Clap, TUI, database, process, or filesystem ownership.
6. Concrete DNS acquisition and the `NetworkAuthority` transport bridge remain outside `eggsec-policy`; policy may evaluate supplied destination facts but does not become a second network stack.
7. `eggsec-report-model` if created owns data contracts only. It may not perform filesystem I/O, rendering, scheduling, terminal output, network access, or async runtime work.
8. No `eggsec-utils` crate is created. Existing utility code must move toward semantic owners or remain local until a real shared abstraction exists.
9. Public paths used by internal/front-end consumers should be preserved through re-exports where doing so does not create a dependency inversion. Where preservation would require an invalid edge, record the pre-1.0 API change explicitly rather than creating a cycle.
10. Existing authorization, scope, approval-binding, transport checkpoint, and feature-profile tests remain authoritative throughout the moves.
11. The default/no-default feature behavior established by Phase E remains unchanged unless a measured dependency result requires a separately documented adjustment.
12. Every extraction must leave the workspace path-dependency graph acyclic.

## New-crate decision rule

Before adding any workspace member in this roadmap, record all of the following in the implementing phase completion record:

- exact modules/types moving;
- direct and transitive dependency graph before/after;
- artifact(s) that stop paying for dependencies or stop depending on an unrelated crate;
- proposed public API and compatibility strategy;
- path-dependency cycle analysis;
- compile/check/test impact for isolated and workspace builds;
- why an internal module boundary is insufficient;
- rejected alternative.

If the move does not demonstrate a concrete improvement, retain an internal boundary and record the extraction as rejected.

## Global verification baseline

At minimum each phase must keep these profiles green, adding narrower package-specific checks as needed:

```text
cargo check --workspace --no-default-features
cargo check -p eggsec
cargo check -p eggsec-cli
cargo check -p eggsec-tui
cargo check -p eggsec-python
cargo tree -d
make check-feature-profiles
make check-features-individual
make check
make check-python
make test-architecture-guards
```

When a new leaf/model crate is added, also run it in isolation with `--no-default-features` where applicable and inspect `cargo tree -p <crate>`.

## Global completion criteria

The roadmap is complete when:

- report DTO consumers no longer need the formatter/orchestration dependency surface solely to exchange report data;
- scheduling/session ownership no longer lives in `eggsec-output` unless a completion record proves that an apparently misplaced module has an output-specific responsibility;
- the utility namespace is materially smaller and remaining modules have documented ownership rationale;
- duplicate runtime rate-control implementations are removed or intentionally differentiated with tests and documentation;
- the Reqwest multi-client pool is removed or retained only with measured evidence that it provides behavior a normal shared client/transport does not;
- policy/enforcement is either extracted into a dependency-light crate or a measured decision record explains the blocker after the internal boundary has been cleaned;
- concrete DNS/transport behavior remains outside the policy crate and transport remains independent of policy implementation;
- load testing no longer owns terminal UI or concrete HTTP-client construction in its core executor;
- any resilience/load-test library extraction is driven by actual independent consumers, not speculative reuse;
- architecture docs and guards describe the final dependency direction;
- Phase D records before/after crate edges, relevant dependency graphs, and residual debt.

## Handoff discipline

Each phase must append a completion record to its plan with baseline/final SHAs, exact commands, dependency deltas, public API changes, compatibility shims, architecture-guard changes, and residual debt. Do not delete the plan after execution. If a proposed extraction is rejected at its gate, record the rejection and proceed to the next phase rather than forcing the crate into existence.
