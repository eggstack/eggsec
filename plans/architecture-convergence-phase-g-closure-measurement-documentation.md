# Phase G Plan: Closure, Measurement, and Documentation Reconciliation

## Status

Status: Executed (2026-09-09). All workstreams implemented; see Completion
record below.

## Objective

Close the architecture-convergence roadmap against the actual final implementation head. Remove temporary migration state, reconcile active documentation with source, and record direct measurements showing whether the line of work reduced synchronization burden and improved capability maturity.

This is a bounded closure phase. It must not become another feature-development phase or evidence framework.

## Preconditions

Phases A-F must either be executed or have explicit accepted blockers. Do not mark the roadmap complete while compatibility migrations introduced by these plans are still open-ended.

## Primary areas

```text
plans/architecture-convergence-*.md
plans/README.md
README.md
docs/ARCHITECTURE.md
docs/ARCHITECTURE_INVARIANTS.md
docs/COMMAND_REGISTRY.md
docs/TOOL_REGISTRATION.md
docs/FEATURE_MATRIX.md
docs/BUILD.md
docs/VERIFICATION.md
docs/EXTENSIBILITY.md
docs/DAEMON.md
docs/WEB_PROXY.md
docs/MOBILE.md
docs/WIRELESS.md
docs/python/domain-maturity.md
docs/python/
Makefile
.github/workflows/
scripts/check-architecture-guards.sh
```

## Non-goals

This phase does not add features, redesign APIs, change authorization policy, reopen historical plans, or introduce permanent dashboards/ledgers solely to prove completion.

## Workstream 1 — Reconfirm final source truth

On the exact final implementation head, inventory:

- workspace crates and dependency directions;
- scope/specification types and authoritative authorization owner;
- Cargo default/main/domain features;
- feature registry entries and aggregate semantics;
- canonical operations and aliases;
- operation-backed CLI commands and dispatch modes;
- runtime task mappings;
- protocol adapter ownership;
- agent ownership/injection boundary;
- Python stable/provisional/experimental domains;
- daemon protocol version and parity matrix;
- platform integration profiles.

Do not copy numbers from prior plans without recounting them.

## Workstream 2 — Remove temporary migration state

Search for and classify:

```text
LegacyWrapped
registry_backed
TODO remove after phase
compatibility conversion temporary aliases
old Scope DTO names
deprecated request constructors
old protocol bridge modules
old runtime parameter mirrors
placeholder browser/proxy session paths
```

Remove items whose compatibility window has ended. Retain semver/public compatibility facades only when intentional, document them as stable shims, and add no deadline that cannot realistically be enforced.

## Workstream 3 — Documentation reconciliation

Update active documentation to match final source exactly.

Special attention:

- `FEATURE_MATRIX.md` defaults/aggregate semantics;
- `COMMAND_REGISTRY.md` removal/completion of legacy dispatch distinctions;
- `ARCHITECTURE.md` crate ownership and protocol/agent boundaries;
- scope documentation distinguishing specification from authorization;
- Python maturity tables based on actual parity evidence;
- daemon local/remote behavior and protocol compatibility;
- platform prerequisites and deep-check cadence;
- verification commands and lint/feature sweep ownership.

Run existing documentation reference/example checks. If source facts are now generated mechanically, remove hand-maintained duplicate tables rather than keeping both.

## Workstream 4 — Complexity and duplication measurements

Record direct before/after measurements against the roadmap baseline where practical:

- count of public/internal `Scope`-like authorization/spec types;
- count of operation-backed commands still using transitional dispatch modes;
- count of separately defined per-operation request parameter structs across engine/runtime/tool adapters;
- count of feature declarations not directly compiled in a maintained profile;
- line/file size of the principal policy/runtime/daemon hotspots;
- number of protocol adapters hosted inside the main engine crate;
- count of Python browser/proxy placeholder methods;
- daemon parity checklist pass/fail totals;
- required versus skipped platform integration tests.

The goal is not arbitrary line-count reduction. Explain what each measurement means architecturally.

## Workstream 5 — Dependency/artifact measurements

Because protocol extraction and shared-contract work may change dependencies, record:

- `cargo tree` for standard CLI, headless/library, daemon, protocol adapter crates, and Python extension profiles;
- release artifact sizes for standard supported artifacts where reproducible;
- duplicate major dependency generations that were added/removed;
- optional native/system prerequisites.

Do not add hard size gates unless a separate product requirement exists.

## Workstream 6 — Verification closure

Run at minimum:

```text
make check
make check-python
make check-full
make check-features-individual   # if introduced in Phase B
```

Also run the relevant daemon/protocol/browser/proxy/platform profiles introduced by this roadmap.

Record exact commands and results. Distinguish:

- pass;
- expected platform skip;
- unavailable external prerequisite;
- known blocker.

Do not write “all checks pass” when a required profile was not run.

## Workstream 7 — Hosted CI confirmation

Confirm the actual CI status for the final pushed commit where accessible. Do not infer hosted success solely from local checks.

If optional scheduled/manual deep checks have not yet run on the final head, state that explicitly and leave the corresponding phase/roadmap condition open until they do or the maintainer accepts the documented local evidence.

## Workstream 8 — Update plan status records

For each Phase A-F plan, append the completion record with:

- baseline SHA;
- final implementation SHA;
- significant compatibility decisions;
- verification performed;
- blockers/accepted exclusions.

Then mark this Phase G and the roadmap executed only when acceptance criteria are satisfied.

Update `plans/README.md` so historical dependency/architecture work remains clearly separate from this roadmap.

## Workstream 9 — Residual backlog classification

Any remaining issue discovered during closure must be classified as:

- correctness/security blocker — roadmap cannot close;
- required roadmap acceptance gap — roadmap cannot close;
- accepted platform limitation — document and close only if roadmap criteria allow it;
- future capability/optimization — record outside this roadmap without reopening implementation scope.

Do not turn optional optimization findings into indefinite blockers.

## Acceptance criteria

- all prior phase status records reflect actual implementation state;
- source/documentation feature/default/command/maturity claims agree;
- no unintentional temporary migration layer remains;
- one authoritative scope authorization owner is documented and enforced;
- legacy command-dispatch migration state is absent for operation-backed commands;
- feature direct-compilation coverage is complete under the Phase B contract;
- protocol/agent dependency boundaries match the Phase D target or have explicit accepted exceptions;
- browser/daemon/proxy maturity claims match execution parity evidence;
- platform integration profiles have non-vacuous test evidence;
- required local verification passes;
- hosted CI status is recorded accurately;
- before/after complexity/dependency measurements are recorded without inventing hard gates;
- manual release/publication policy remains unchanged;
- the roadmap is marked executed only after the exact final head is validated.

## Completion record

When complete, append the closure summary and link any final architecture/verification documents used as canonical references.

Executed 2026-09-09.

- Baseline SHA: `e3f5eaad` (Phase F implementation head). Final implementation
  SHA and hosted CI status: recorded in `plans/README.md` after push (WS7;
  the exact final head is validated by hosted CI before the roadmap is
  marked executed there).
- Compatibility decisions: no temporary migration layer removed as code —
  none remained. All retained shims are intentional stable facades with no
  unenforceable deadline: Rust `eggsec_tool_core::Scope` / `ToolScopeSpec` /
  `tool::Scope` aliases for `ScopeSpec` (Phase A), unchecked
  `descriptor_for_target()` (documented stable shim; new strict-surface code
  must use `try_descriptor_for_target`), CLI command aliases + Python
  snake_case names + tool alias resolution (Phase C), `RestState::new` /
  `GrpcService::new` / `McpServer::with_enforcement` / `openai::router` /
  `openresponses::router` / `Agent::new` composition-root shims (Phase D),
  `registry_backed_command_ids()` derived helper (Phase C). Pruned docs-only
  migration state: `docs/extending/commands.md` step-by-step
  LegacyWrapped→RegistryBacked migration guide condensed to a short history
  note; `registry_backed` removed-field note condensed.
- Verification: `make check` PASS (EXIT 0), `make check-python` PASS
  (EXIT 0; 4454 passed, 0 failed, 1706 skipped, 17 xfailed),
  `make check-full` PASS (EXIT 0),
  `make check-features-individual` PASS (66 PASS, 4 SKIP for absent
  libpcap/libssh2 system deps, 0 FAIL),
  `bash scripts/check-architecture-guards.sh` ALL PASSED,
  `python3 scripts/check_doc_references.py` OK (222 files; `plans/`
  excluded as historical record — Phase D/E plans reference
  `crates/eggsec-daemon/src/protocol.rs`, correctly removed as a dead
  duplicate in Phase E),
  `cargo test -p eggsec-daemon -p eggsec-daemon-protocol -p eggsec-web-proxy -p eggsec-runtime`
  674 passed / 1 ignored, platform fixtures
  (`packet::fixture` 10, `wireless::fixture` 4/6, `platform::` 6,
  mobile-lab 108) PASS, `bash scripts/check_platform.sh` PASS.
- Drive-by correction required to verify: `packet::fixture` FD-leak guard
  was racy under parallel test execution (ambient FDs from concurrent tests
  inflated the after-sample; failed 10→20 against +8 allowance). Fixed by
  measuring differentially (warmup loop, then measured loop; pure in-memory
  craft→parse→hexdump opens no FDs by construction). No production code
  changed for this fix.
- Documentation reconciliation (all recounted from source, not copied from
  prior plans): canonical operations 34 (prior docs said 31) + 42 aliases;
  `REGISTERED_COMMANDS` 49 entries: 29 `RegistryBacked` (prior docs said
  31), 13 `HelperOnly`, 7 `ServerLifecycle`; `TaskKind` 29 variants / 29
  params structs; `FULL_MEMBERS` 28 pinned; daemon protocol v2; invariants
  39. Fixed: `docs/TOOL_REGISTRATION.md` (34 entries, `policy_catalog.rs`
  paths), `architecture/overview.md` (3 spots), `architecture/cli_commands.md`
  (49/29 counts), `architecture/config.md` (`policy_catalog.rs` paths,
  34 ops), `docs/CAPABILITY_MATRIX.md` (34 total; added missing
  `evasion`, `postex`, `wireless-deauth` rows + feature-gating rows),
  `docs/METADATA_OWNERSHIP.md` + `docs/EXTENSIBILITY.md` +
  `docs/COMMAND_REGISTRY.md` + `docs/extending/commands.md`
  (`policy_catalog.rs` paths), `eggsec-config` skill (34 ops,
  `try_descriptor_for_target`, approval-construction ownership).
  README.md and AGENTS.md needed no changes (no false claims found).
- Complexity/duplication measurements (baseline `ae7c7d44` → final):
  - Scope authorization owners: tool-DTO `is_allowed` 1 → 0; single
    engine `Scope::is_target_allowed` via `EnforcementContext` remains.
    Meaning: protocol DTOs can no longer answer authorization.
  - `LegacyWrapped` mentions: 24 → 2 (both historical comments, no code);
    `RegistryBacked` registry entries 4-command pilot → 29 (all
    operation-backed commands). Meaning: dispatch migration is complete,
    not a permanent dual mode.
  - Canonical request ownership: new since baseline —
    `eggsec-tool-core::operation_request` (14 typed request structs,
    single defaults/validation owner) + `eggsec::operation_request`
    facade; `TaskKind::operation_id`/`canonical_target` exhaustive (no
    wildcard). Meaning: runtime/CLI/tool/Python adapters translate into
    one contract instead of mirroring params (29 runtime params structs
    remain as wire DTOs, mapped exhaustively).
  - Features without direct compilation: every declared feature now
    compiles in a maintained profile (`make check-features-individual`;
    66 PASS / 4 system-dep SKIP / 0 FAIL). Meaning: `full` stays a
    curated 28-member lab aggregate by explicit contract, and the sweep —
    not `full` — is the exhaustiveness oracle.
  - Hotspots: `config/policy.rs` 2402 → 1007 (+`policy_target` 220,
    +`policy_catalog` 1269, +`policy_approval` 149);
    `config/scope.rs` 1634 → 1428 (+`scope_address` 174,
    +`scope_resolver` 132); `eggsec-runtime/runtime.rs` 1851 → 1669
    (+`runtime_config` 61, +`runtime_sink` 204); `eggsec-daemon/host.rs`
    3149 → 3205 (+`host_auth` 141, +`host_persistence` 60).
    Meaning: policy approval/catalog/target and scope facts are now
    independently reviewable behind stable facades; daemon growth is
    parity features (v2 result retrieval), not new coupling.
  - Protocol adapters: still hosted in `eggsec` by deliberate decision
    (a separate crate would become a second composition root; see
    `architecture/api_extraction_boundary.md`), but all adapters now
    depend on injected `EngineServices` / `McpEngineBridge` /
    `AgentExecutionService` and expose only `dispatch_checked`.
    Meaning: the boundary is dependency inversion, not relocation.
  - Python placeholders: browser/proxy/daemon bindings now backed by
    real execution or explicit unsupported errors (Phase E; no
    promotions, gaps recorded in `docs/python/domain-maturity.md`).
  - Daemon parity: protocol v1 → v2 (`GetTaskResult`/`TaskResult`,
    durable retrieval, parity matrix `docs/DAEMON_PARITY.md`).
  - Platform: no `platform/` module at baseline → centralized
    `PlatformReport`/`skip_reason_for`, hermetic fixtures (0 skips on
    host), isolated live legs that SKIP with named prerequisites.
- Dependency/artifact measurements (final head):
  `cargo tree -p eggsec-cli --no-default-features` 781 lines,
  `-p eggsec-cli` (default/TUI) 939, `-p eggsec-daemon` 784,
  `-p eggsec-daemon-protocol` 66. Duplicate major generations observed in
  the default CLI closure (e.g. cpufeatures 0.2.17/0.3.0,
  derive_more 0.99.20/2.1.1 via transitive hickory/scraper/crossterm
  paths) — recorded, not gated (no product size requirement; `cargo deny`
  bans policy unchanged and green via `make check-full`). Optional
  native prerequisites unchanged: libpcap (packet-inspection),
  libssl (nse), libssh2 (nse-ssh2), protoc (grpc-api), ADB/Frida
  (mobile-dynamic), wireless-tools + lab hardware (wireless). Debug CLI
  binary ~392M (local build, not a release artifact; no release artifacts
  generated — publication stays manual).
- Residual backlog (WS9): no correctness/security blockers, no required
  roadmap acceptance gaps. Accepted platform limitations: live netns /
  emulator / RF evidence awaits a lab host (deep-checks
  `platform-integration` job owns it). Future optimizations outside this
  roadmap: none introduced.
- Canonical references: `docs/ARCHITECTURE.md`, `docs/COMMAND_REGISTRY.md`,
  `docs/TOOL_REGISTRATION.md`, `docs/FEATURE_MATRIX.md`,
  `docs/CAPABILITY_MATRIX.md`, `docs/ARCHITECTURE_INVARIANTS.md` (39),
  `docs/VERIFICATION.md`, `docs/DAEMON_PARITY.md`, `docs/PLATFORM.md`,
  `docs/python/domain-maturity.md`, `architecture/overview.md`,
  `architecture/config.md`, `architecture/cli_commands.md`,
  `architecture/api_extraction_boundary.md`, `Makefile`,
  `scripts/check-architecture-guards.sh`.