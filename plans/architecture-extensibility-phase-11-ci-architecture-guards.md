# Architecture Extensibility Phase 11: CI Architecture Guards

## Objective

Add CI architecture guards that preserve the enforcement, registry, metadata, feature, and documentation invariants established across Phases 1–10. The goal is to stop regressions at pull-request time without making the workflow brittle, slow, or dependent on platform-specific optional features.

This phase should wire existing tests and a small number of new guard checks into CI. It should not introduce new runtime behavior or new security capabilities.

## Current context

The architecture/extensibility work has stabilized several core invariants:

- `OperationMetadata` is the canonical operation policy metadata layer.
- `DomainDescriptor` is the canonical domain/integration grouping layer.
- `ToolRegistration` distinguishes `mcp_metadata_exposable` from `mcp_default_visible`.
- MCP OpsAgent intentionally uses Model A: profile-expanded metadata-exposable listing, not conservative default listing.
- `mcp_tool_registrations_default_visible()` is the conservative default subset.
- `CommandRegistration` separates `cli_visible`, `tui_visible`, `programmatic_visible`, `cli_interactive_only`, `registry_backed`, and `dispatch_mode`.
- `CommandDispatchMode` distinguishes `RegistryBacked`, `LegacyWrapped`, `CatalogOnly`, `ServerLifecycle`, and `HelperOnly`.
- TUI action specs point back to canonical metadata for pilot actions.
- `eggsec-output` has a normalized report/evidence envelope.
- Feature metadata has a static snapshot checked against `crates/eggsec/Cargo.toml`.
- Plans are retained in `plans/` for handoff/audit continuity.

The previous polish pass resolved the remaining naming and documentation contradictions. This phase should now make those invariants hard to regress.

## Non-goals

- Do not redesign the existing GitHub Actions setup wholesale unless necessary.
- Do not add long-running platform-sensitive jobs to the default PR path.
- Do not enable advanced/lab-only optional features by default.
- Do not broaden MCP/REST/gRPC/agent exposure.
- Do not convert remaining legacy commands to registry-backed dispatch.
- Do not add new scanning, testing, or lab capabilities.

## Design target

Create a layered CI model:

1. **Fast required PR guards**: no-default build, core tests, metadata/registry consistency tests, docs/link sanity checks.
2. **Feature-profile guards**: representative compile checks for optional feature profiles, kept bounded.
3. **Optional scheduled/deep guards**: broader feature combinations and platform-sensitive checks.
4. **Architecture drift guards**: textual/static checks for forbidden bypass patterns and stale terminology.

Prefer explicit, small commands over opaque monolithic scripts. If scripts are added, keep them as wrappers around clearly documented `cargo`/`rg` commands.

## Work item 1: Inventory existing CI and local validation commands

Inspect existing workflow and script files:

- `.github/workflows/**`
- `Makefile` or `justfile`, if present
- `scripts/**`
- `AGENTS.md`
- `docs/FEATURE_MATRIX.md`
- `docs/COMMAND_REGISTRY.md`
- `docs/TOOL_REGISTRATION.md`

Document the current state before adding jobs.

Deliverable:

- Add/update `docs/CI_ARCHITECTURE_GUARDS.md` with:
  - required fast checks;
  - optional/deep checks;
  - feature-profile checks;
  - known platform-sensitive checks;
  - local reproduction commands.

Acceptance criteria:

- A contributor can reproduce the required CI guard set locally.
- The doc distinguishes required PR checks from scheduled/deep checks.

## Work item 2: Add or update required fast PR workflow

Create or update a required fast workflow that runs on pull requests and pushes to the main branch.

Recommended workflow file:

- `.github/workflows/ci.yml`, or
- `.github/workflows/architecture-guards.yml` if CI already has a primary workflow.

Required fast checks:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo test -p eggsec --lib
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --test command_registry
cargo test -p eggsec --test tool_registration
cargo test -p eggsec --test feature_matrix
cargo test -p eggsec --test enforcement_matrix
cargo test -p eggsec --test enforced_dispatch_regression
cargo test -p eggsec-output --test report_envelope
```

Add `cargo test -p eggsec-tui --lib` only if it is stable under no-default CI dependencies. If TUI compile or test behavior depends on terminal/platform assumptions, move it to a separate feature-profile or scheduled job.

Acceptance criteria:

- Fast workflow covers core architecture invariants.
- Workflow avoids platform-sensitive optional dependencies.
- Workflow uses current stable Rust or the repo's declared MSRV policy if already established.

## Work item 3: Add feature-profile compile guards

Add a bounded matrix for representative feature profiles. These can run in the same workflow as separate jobs or in a dedicated workflow.

Recommended feature profiles:

```bash
cargo check -p eggsec --features tool-api,rest-api
cargo check -p eggsec --features grpc-api
cargo check -p eggsec --features db-pentest
cargo check -p eggsec --features db-pentest-mcp,tool-api,rest-api
cargo check -p eggsec --features mobile
cargo check -p eggsec --features mobile-dynamic
cargo check -p eggsec --features web-proxy
cargo check -p eggsec --features web-proxy-mcp,tool-api,rest-api
cargo check -p eggsec --features c2-mcp,tool-api,rest-api
```

If any feature profile requires unavailable system dependencies, mark it as one of:

- scheduled only;
- allowed failure with issue reference;
- documented local/manual profile;
- split into a lighter compile-only profile.

Do not hide failures silently.

Acceptance criteria:

- Representative protocol/domain profiles are checked.
- Advanced/lab-only profiles remain opt-in and explicit.
- Platform-sensitive failures are documented, not ignored without explanation.

## Work item 4: Add architecture drift grep/static checks

Add simple static checks for the highest-value architectural invariants. Prefer small shell scripts or a documented `rg` block. Suggested script:

- `scripts/check-architecture-guards.sh`

Required checks:

1. **No stale command registry terminology**
   - Fail on `manual_only` in command registry/docs/tests unless inside historical plan files.
   - Fail on `interactive_only` where `cli_interactive_only` should be used, excluding historical plan files.

2. **MCP exposure terminology stays split**
   - Ensure `mcp_metadata_exposable` and `mcp_default_visible` both appear in `tool/registration.rs` and `docs/TOOL_REGISTRATION.md`.
   - Fail on text that equates OpsAgent with conservative default listing.

3. **Raw dispatch is not used by strict surfaces**
   - Keep or extend `enforced_dispatch_regression` test.
   - Optional grep for direct `ToolDispatcher::dispatch(` calls outside allowed files/tests.

4. **Plan retention**
   - Ensure key phase plan files still exist:
     - `plans/architecture-extensibility-roadmap.md`
     - `plans/architecture-extensibility-phase-06-command-registry.md`
     - `plans/architecture-extensibility-phase-07-tool-mcp-registration.md`
     - `plans/architecture-extensibility-phase-08-tui-tightening.md`
     - `plans/architecture-extensibility-phase-09-report-evidence-unification.md`
     - `plans/architecture-extensibility-phase-10-feature-matrix-build-profiles.md`
     - this Phase 11 plan file.

5. **Docs current-state contradictions**
   - Fail on old phrases like "MCP listing does not check `mcp_metadata_exposable`" outside historical plan files.
   - Fail on old field names if they appear in current docs.

Acceptance criteria:

- Static guard script is deterministic and runs quickly.
- It excludes `plans/` where historical terminology is expected, unless checking plan retention.
- It is not so broad that comments in old plan files break CI.

## Work item 5: Add docs/reference checks

Use existing metadata tests where possible. Add a lightweight docs guard only if useful.

Recommended checks:

- Verify current docs referenced by metadata exist.
- Verify `docs/COMMAND_REGISTRY.md`, `docs/TOOL_REGISTRATION.md`, `docs/FEATURE_MATRIX.md`, and `docs/METADATA_OWNERSHIP.md` exist.
- Verify `docs/FEATURE_MATRIX.md` mentions Model A or links to `docs/TOOL_REGISTRATION.md` if it discusses MCP exposure.
- Verify `docs/CI_ARCHITECTURE_GUARDS.md` exists.

This can be part of `metadata_consistency.rs` or the shell guard script.

Acceptance criteria:

- Missing current architecture docs fail CI.
- Historical plan files are not parsed as current-state documentation.

## Work item 6: Ensure feature snapshot guard is robust in CI

The `feature_matrix` test now parses `crates/eggsec/Cargo.toml` and compares actual feature keys to the static snapshot. Confirm this works in CI.

Required checks:

- `toml` dev-dependency is declared in the correct crate or workspace.
- `cargo test -p eggsec --test feature_matrix` passes under no-default features.
- Test error messages clearly tell maintainers how to update the snapshot.

Acceptance criteria:

- Adding/removing a Cargo feature without updating the snapshot fails CI.
- Metadata feature strings remain validated against known feature names.

## Work item 7: Encode MCP Model A tests as required guards

Ensure the following tests are included in required CI:

- `ops_agent_registrations_are_metadata_exposable`
- `default_visible_registrations_are_actually_default_visible`
- `ops_agent_is_expanded_metadata_exposable_not_conservative_default`
- `high_risk_operations_not_default_mcp_visible`
- `mcp_metadata_exposable_matches_operation_metadata`

If the existing test comment says OpsAgent is "strictly broader" than conservative default but the assertion only checks superset behavior, fix either the comment or the assertion before wiring CI.

Recommended fix:

- If strict broadness is required, add `assert!(ops_ids.len() > default_ids.len())`.
- If broadness may vary by feature set, change the comment to "at least as broad".

Acceptance criteria:

- CI encodes the chosen Model A semantics without ambiguous comments.

## Work item 8: Keep CI runtime bounded

Set practical guardrails:

- Avoid full feature builds on every PR unless currently cheap.
- Avoid network-dependent tests.
- Avoid requiring Docker, ADB, browser drivers, packet privileges, or system services in required PR jobs.
- Keep optional/deep jobs separate.

If GitHub Actions caching is already present, use it. If not, add basic Rust cache only if low-risk.

Recommended:

- Use `Swatinem/rust-cache` or existing cache convention if already used in the repo/org.
- Do not introduce new external actions unless acceptable for the project.

Acceptance criteria:

- Required CI jobs remain suitable for normal PR iteration.
- Deep/platform-sensitive checks are documented separately.

## Work item 9: Update contributor instructions

Update `AGENTS.md` and/or `CONTRIBUTING.md` if present.

Required content:

- Local command to run before handoff:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --test command_registry
cargo test -p eggsec --test tool_registration
cargo test -p eggsec --test feature_matrix
```

- Reminder that MCP OpsAgent is profile-expanded, not conservative default.
- Reminder that command helper-only fields use `cli_interactive_only`, not old `manual_only` or ambiguous `interactive_only`.
- Reminder that domain crates do not authorize work; enforcement is centralized.

Acceptance criteria:

- New contributors know which local checks correspond to required CI.
- Architecture terminology remains consistent in contributor docs.

## Work item 10: Optional scheduled/deep workflow

If useful, add a scheduled workflow for broader checks. Keep this separate from required PR CI.

Potential scheduled checks:

```bash
cargo check --workspace --all-features
cargo test --workspace --all-features
cargo check -p eggsec --features full
```

Only add these if current dependency/platform behavior makes them practical. If `full` includes advanced/lab-only features with platform assumptions, document it rather than adding a chronically failing scheduled job.

Acceptance criteria:

- No required PR job depends on all-features success unless the repo already supports it.
- Scheduled/deep checks are clearly labeled optional/deep.

## Files likely to change

- `.github/workflows/ci.yml`
- optionally `.github/workflows/architecture-guards.yml`
- optionally `.github/workflows/deep-checks.yml`
- `scripts/check-architecture-guards.sh` or equivalent
- `docs/CI_ARCHITECTURE_GUARDS.md`
- `docs/TOOL_REGISTRATION.md` only if Model A test/comment wording needs small correction
- `docs/COMMAND_REGISTRY.md` only if static guard terminology reveals drift
- `AGENTS.md`
- `CONTRIBUTING.md` if present
- `crates/eggsec/tests/tool_registration.rs` if OpsAgent strict-broader wording/assertion needs correction

## Validation commands

Run locally before handoff:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo test -p eggsec --lib
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --test command_registry
cargo test -p eggsec --test tool_registration
cargo test -p eggsec --test feature_matrix
cargo test -p eggsec --test enforcement_matrix
cargo test -p eggsec --test enforced_dispatch_regression
cargo test -p eggsec-output --test report_envelope
```

Run the static guard if added:

```bash
./scripts/check-architecture-guards.sh
```

Run feature-profile compile checks selected for CI:

```bash
cargo check -p eggsec --features tool-api,rest-api
cargo check -p eggsec --features grpc-api
cargo check -p eggsec --features db-pentest
cargo check -p eggsec --features db-pentest-mcp,tool-api,rest-api
cargo check -p eggsec --features mobile
cargo check -p eggsec --features mobile-dynamic
cargo check -p eggsec --features web-proxy
cargo check -p eggsec --features web-proxy-mcp,tool-api,rest-api
cargo check -p eggsec --features c2-mcp,tool-api,rest-api
```

If CI config changes are committed, verify the workflow syntax either by GitHub Actions or a local YAML linter if available.

## Completion criteria

Phase 11 is complete when:

- Required CI covers no-default build and core metadata/enforcement/registry tests.
- Tool registration Model A semantics are guarded by tests in CI.
- Command registry dispatch-mode and visibility invariants are guarded by tests in CI.
- Feature snapshot vs Cargo feature keys is guarded by CI.
- Static grep guards catch stale terminology and strict-surface bypass patterns without scanning historical plan text incorrectly.
- Current architecture docs are required to exist.
- Contributor docs list the local validation commands.
- Optional/deep/platform-sensitive checks are separated from required PR checks.

## Handoff note

After Phase 11 lands, Phase 12 should focus on extensibility handoff and contributor model: how to add a new domain, operation, command, tool, report bridge, feature gate, and docs entry without violating the architecture guards.
