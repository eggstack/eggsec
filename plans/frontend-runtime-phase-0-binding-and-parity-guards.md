# Phase 0 Plan: Approval Binding and Frontend Parity Guards

## Status

Status: Executed.

Depends on: none.

Baseline: `f48b3f37a273e1e53735d98acbb0cb0bcd607260`.
Starting HEAD: `252d6dfc3634c22e1aa82f0c2dc15cf2b0057e49` (plan-index commit; code baseline).
Final: commit containing this record (see `git log` for Phase 0 execution).

## Objective

Before changing dispatch topology, make the current frontend/runtime contract mechanically observable. Fix stale approval-cache reuse in the TUI, then add exhaustive parity tests that identify where CLI commands, canonical operations, TUI tabs/actions, runtime tasks, and Cargo features intentionally correspond and where they intentionally do not.

This phase is deliberately test-heavy. Later phases should be able to delete duplicate mappings with confidence instead of preserving them because their behavior is unknown.

## Confirmed starting state

The engine already has strong authorization primitives. `ApprovedOperation` stores the approved `OperationDescriptor`, execution surface/profile, and policy decision, and dispatch binding is validated before an executor runs. Preserve this design.

The TUI's `EnforcementFacade` additionally caches an approval to avoid repeated prompts/evaluation. At the audited baseline, cache reuse is selected by operation name rather than the complete descriptor/target contract. A changed target for the same operation can therefore select an approval that was issued for the previous descriptor and only be rejected later by the engine binding guard. This should be corrected at the cache boundary.

Separately, surface metadata is currently split across:

```text
crates/eggsec/src/config/policy*.rs            canonical operation/policy metadata
crates/eggsec/src/commands/registry.rs         CLI command registration
crates/eggsec/src/cli/mod.rs                   actual Clap command tree
crates/eggsec/src/operation_request.rs         canonical request adapters
crates/eggsec-runtime/src/request.rs           runtime TaskKind/wire metadata
crates/eggsec-tui/src/tabs/mod.rs              Tab enum, Tab::all, help routing
crates/eggsec-tui/src/tabs/spec.rs             TabSpec metadata
crates/eggsec-tui/src/app/action_spec.rs       four-action metadata pilot
crates/eggsec-tui/src/app/command.rs           palette/alias -> Tab/actions
```

The goal of this phase is not to consolidate them yet. The goal is to establish an explicit support matrix and executable drift detectors before Phase 1/2 removes redundant ownership.

## Workstream 0.1 — Make cached approval identity exact

Change `EnforcementFacade` so a cached `ApprovedOperation` is reusable only when it is bound to the exact descriptor currently being requested.

Preferred implementation order:

1. Reuse the engine's existing request-binding/equivalence primitive if one is exposed without weakening encapsulation.
2. If no reusable predicate exists, add a narrow method on the approval/descriptor API such as `matches_descriptor(&OperationDescriptor) -> bool` or an immutable descriptor identity/fingerprint type.
3. Do not duplicate a hand-maintained subset of descriptor fields in TUI code. Future policy-relevant descriptor fields must automatically participate in cache identity.

Invalidation must occur when any input capable of changing evaluation semantics changes, including at minimum target/operation descriptor, scope, execution policy/profile, manual override state where applicable, and reloaded config/scope state if Phase 2 implements live reload.

Regression cases:

- same operation + same descriptor may reuse approval;
- same operation + different target must not reuse approval;
- same operation + changed policy-relevant option must not reuse approval;
- changed scope/policy generation invalidates prior cached approval;
- a stale token must never trigger a second operator approval for the wrong descriptor or produce a confusing late binding failure when the frontend can detect the mismatch earlier;
- downstream engine binding validation remains in place even after the cache is corrected.

This is a correctness/UX hardening change, not a reason to relax or remove engine-side `ApprovedOperation` validation.

## Workstream 0.2 — Create an explicit surface support matrix

Add a test-owned or source-owned data model that can classify every public surface entry. The matrix must answer, at minimum:

| Field | Meaning |
| --- | --- |
| canonical operation ID | `None` for true helper/UI/lifecycle items |
| CLI command/alias | actual Clap-visible command when present |
| command dispatch class | operation-backed / multiplexer / helper / lifecycle |
| TUI tab/action | stable IDs when present |
| runtime task kind | wire/runtime representation when supported |
| required Cargo feature | canonical feature ownership |
| surface status | supported / intentionally unavailable / UI-only / compatibility alias |
| target kind | host/url/path/interface/no-target/etc. where relevant |

Do not create another production registry merely to make this table exist. Prefer deriving the matrix in tests from existing canonical registries plus a small explicit exception list. Phase 2 will decide the final TUI production metadata owner.

The matrix must explicitly classify known asymmetric surfaces rather than treating asymmetry as failure. Examples include settings/history/report UI, daemon/server lifecycle commands, output-only CLI flags, and operation families represented by a runtime multiplexer.

## Workstream 0.3 — Reflect the real Clap tree

Use `clap::CommandFactory` on the actual `Cli` type to inspect the compiled command tree. Tests should compare registry claims against what Clap really exposes under the active feature set.

Required assertions:

- every `cli_visible` command registration that is enabled in the current feature profile exists in the Clap tree;
- every operation-backed top-level Clap command is represented by the command registry or an explicit multiplexer exception;
- aliases resolve to the intended command/operation without creating a second canonical operation identity;
- helper and lifecycle commands are classified explicitly;
- disabled features do not leave registry tests green while the actual Clap variant disappeared;
- command-name tests are exhaustive over the reflected tree, not a curated spot check.

Keep help text formatting out of these tests unless it encodes a contract; test semantic command identity rather than snapshots of prose.

## Workstream 0.4 — Audit TUI operation and feature parity

Write parity tests over `Tab`, `TabSpec`, `TUI_ACTION_SPECS`, and the canonical command/operation metadata. Resolve the following audited-baseline discrepancies explicitly:

- `waf` TUI operation identity versus canonical `waf-detect`;
- `scan-pipeline` TUI operation identity versus canonical `pipeline`;
- stress tab/command feature ownership (`stress-testing`);
- packet tab/command feature ownership (`packet-inspection`);
- proxy/intercept naming and feature ownership, which represent related but not necessarily identical capabilities;
- runtime-only/UI-only families such as compliance/storage/integrations/workflow/vulnerability management;
- canonical operations without TUI tabs, including any intentionally CLI/programmatic-only operation.

For every discrepancy, choose one of:

1. normalize to the canonical operation/feature identifier;
2. retain as an explicit compatibility/UI alias with a tested conversion;
3. mark the surface intentionally unsupported/UI-only.

Do not leave raw string differences undocumented after this phase.

## Workstream 0.5 — Runtime mapping guards

`TaskKind` is a wire/runtime type, so one-to-one parity with canonical operations is not required. What is required is exhaustive and explicit conversion.

Add tests that prove:

- every `TaskKind` has a deliberate canonical operation mapping or explicit unsupported classification;
- `TaskKind::operation_id()` and engine `operation_request::runtime_adapters::operation_id_for_task_kind()` cannot silently disagree;
- target extraction agrees between runtime and engine adapters;
- runtime surface conversion is explicit and round-trippable for all supported values;
- adding a new runtime kind or canonical runtime-supported operation causes a test/compile failure until mapping is supplied.

If maintaining both the runtime methods and engine mapping is unnecessary, Phase 3 will remove one owner. Phase 0 only makes disagreement visible.

## Workstream 0.6 — Feature-profile parity tests

Exercise at least these profiles:

```text
--no-default-features
default build
stress-testing
packet-inspection
nse
headless-browser
compliance
database/external-integrations/finding-workflow/vuln-management
wireless
wireless-advanced
db-pentest
web-proxy
c2
full/curated broad profile where supported
```

The exact matrix may use representative grouped builds to control CI cost, but `make check-features-individual` remains the completeness oracle for feature declaration changes.

Parity tests must distinguish compile-time absence from runtime unavailable state. A TUI tab that intentionally remains visible as an unavailable shell needs an explicit `availability`/feature state rather than an implicit mismatch with Clap.

## Workstream 0.7 — Add architecture guards for the new invariants

Extend `scripts/check-architecture-guards.sh` only for simple structural invariants that grep can reliably prove. Do not encode semantic parity in shell regexes when Rust tests can inspect typed metadata.

Good static guards include preventing reintroduction of deprecated migration enums or frontend imports across forbidden crate boundaries. Semantic checks such as operation aliases, target kinds, or feature support belong in Rust tests.

## Acceptance criteria

Phase 0 is complete when all of the following are true:

- TUI approval-cache reuse requires exact descriptor/request binding, not operation-name equality;
- downstream dispatch binding validation remains intact;
- the actual Clap tree is reflected in exhaustive registry parity tests;
- every TUI tab/action is classified as operation-backed, UI-only/helper, lifecycle, unavailable, or compatibility alias;
- audited operation-ID/feature discrepancies have explicit resolutions and regression tests;
- runtime task-to-operation/target mapping disagreement is mechanically detectable;
- feature-disabled versus unavailable-visible behavior is intentional and tested;
- new tests are part of the mandatory repository verification path, not only optional/manual checks.

## Verification

At minimum:

```text
make fmt
make test-feature-matrix
make test-architecture-guards
make check
make check-feature-profiles
```

Run `make check-features-individual` if Cargo feature declarations or exhaustive feature mappings are modified. Run focused TUI tests under default, `stress-testing`, and `packet-inspection` profiles because those are current audit targets.

## Handoff notes

Do not begin Phase 1 by deleting the existing registry match or runtime mappings before these guards are green. The value of this phase is to freeze current intended behavior, identify intentional asymmetry, and convert unknown drift into named failures.

## Completion record (Phase 0 executed 2026-09-10)

Baseline: `f48b3f37`; starting HEAD `252d6dfc`; final: commit containing this record.

Support-matrix counts (final):
- `REGISTERED_COMMANDS`: 51 entries (29 operation-backed incl. multiplexers/subcommand identities; helpers/lifecycle remainder).
- `ALL_OPERATION_METADATA`: 34 canonical operations; 44 aliases in `ALL_OPERATION_METADATA_ALIASES`.
- Runtime `TaskKind`: 29 variants, all mapped (engine `operation_id_for_task_kind` agrees with `TaskKind::operation_id`).
- TUI `TAB_SPECS`: 33 entries; `Tab::all()` 21 base + 12 gated. 26 operation-backed (incl. 2 availability shells).
- Reflected Clap tree (rest-api profile): core 27 unconditional + feature-gated remainder; `oauth` primary normalized.

Mismatch resolutions:
- 0.1 cache: `ApprovedOperation::matches_descriptor()` (derived `PartialEq`) + scope fingerprint/policy hash/surface/profile/override generation; `toggle_posture`/override changes invalidate; `bundle.rs` uses same predicate. Engine `validate_request_binding` unchanged (pinned by test).
- 0.4 TUI: `waf`→`waf-detect`, `scan-pipeline`→`pipeline` normalized (aliases retained + tested); stress/packet features set to canonical (`stress-testing`/`packet-inspection`) as always-visible availability shells; proxy (helper) vs intercept (`proxy-intercept`) distinct; compliance/storage/integrations/workflow/vuln operation-backed; no-tab ops (`waf-bypass`, `remote`, `search`, `mobile-static/dynamic`, `evasion`, `postex`, `wireless-deauth` via active-mode) classified intentional.
- 0.3 Clap: `oauth` primary normalized to `oauth` (`o-auth` alias); `mobile-dynamic`/`wireless-deauth` subcommand identities; `proxy`/`daemon`/`session`/`task`/`codegg-mcp` explicit Clap-only exceptions; `remote-serve`→`remote` rename documented; `storage` helper remains visible as unavailable shell when `database` disabled.
- 0.5 runtime: dual-owner agreement tests (operation_id, target extraction), surface round-trip, explicit bridge failures for targetless/interface kinds.
- 0.6 profiles: tests track compiled feature set (absence vs unavailable-shell); `check-features-individual` remains oracle (no Cargo feature declarations changed; Clap attribute-only fix for oauth).

Verification evidence (local, before push):
- `cargo fmt --all --check`: pass (after `cargo fmt --all`).
- `cargo test -p eggsec-tui --lib parity`: 10 passed.
- `cargo test -p eggsec-tui --lib app::enforcement_facade`: 21 passed.
- `cargo test -p eggsec --features rest-api --test frontend_surface_matrix`: 15 passed.
- TUI `tabs::spec` under `stress-testing`/`packet-inspection`: pass.
- `make test-feature-matrix`: pass.
- `make test-architecture-guards` / `scripts/check-architecture-guards.sh`: ALL PASSED (incl. new Checks 79–81).
- `make check`: pass (fmt, no-default check, clippy, doc tests, package tests incl. new matrix, output tests, TUI lib tests, guards).
- `make check-feature-profiles`: pass.
- `check-features-individual` skipped (no Cargo `[features]` changes; oracle unchanged).
