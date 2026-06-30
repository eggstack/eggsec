> **Status: COMPLETED**

# Phase 11 Handoff Plan: Domain Crate Extraction

## Goal

Begin extracting high-risk, dependency-heavy, or lab-specific domains out of the main `eggsec` crate while keeping enforcement centralized. The main crate should become a composition layer for policy, scope, metadata, dispatch, CLI/TUI/MCP/REST integration, and reports. Domain crates should own domain execution logic, types, and tests, but not decide whether an operation is allowed.

This phase is about reducing coupling and making future safety review easier.

## Current context

The main `eggsec` crate currently owns a broad set of domains and integrations:

- Recon/scanning/fuzzing/WAF.
- Load/stress/raw packet.
- DB pentesting.
- Web proxy / traffic interception.
- Wireless.
- C2 simulation.
- Evasion/post-exploitation lab surfaces.
- MCP/REST/TUI integration and policy enforcement.

The dual-mode enforcement work centralized policy decisions, which is the right prerequisite for extraction. Domain crates should not reimplement enforcement. They should declare metadata and expose execution APIs that accept already-approved context or plain execution inputs from the central dispatcher.

## Extraction principles

1. Enforcement stays central.
2. Domain crates expose metadata, types, and execution functions.
3. Domain crates do not parse global CLI flags directly.
4. Domain crates do not own `ExecutionSurface` or `ExecutionProfile` decisions.
5. Main crate controls protocol exposure and dispatch.
6. Feature flags remain explicit and narrow.
7. Each extraction should preserve existing CLI/TUI/MCP/REST behavior.

## Candidate crates

Prioritize extraction in this order:

1. `eggsec-db-lab`
2. `eggsec-web-proxy`
3. `eggsec-wireless`
4. `eggsec-evasion-lab`
5. `eggsec-postex-lab`
6. `eggsec-c2-lab`
7. `eggsec-load-lab` or `eggsec-stress-lab` if load/stress dependencies justify it

Do not extract everything at once. Each crate should be a small, reviewable pass.

## Workspace layout

Add crates under `crates/`:

```text
crates/eggsec-db-lab/
crates/eggsec-web-proxy/
crates/eggsec-wireless/
crates/eggsec-evasion-lab/
crates/eggsec-postex-lab/
crates/eggsec-c2-lab/
```

Each crate should have:

```text
Cargo.toml
src/lib.rs
src/types.rs
src/runner.rs or domain-specific modules
tests/
```

## Shared dependency strategy

Avoid pulling the entire `eggsec` crate into domain crates if possible. Prefer depending on narrow crates:

- `eggsec-core`
- `eggsec-output`
- `eggsec-tool-core`

If those crates do not contain enough shared types, add narrowly scoped types there rather than depending back on the main crate.

The dependency graph should not become cyclic:

```text
eggsec -> eggsec-db-lab
eggsec-db-lab -> eggsec-core / eggsec-output / eggsec-tool-core
```

Never:

```text
eggsec-db-lab -> eggsec
```

## Metadata integration

If Phase 6 is complete, each domain crate may expose metadata declarations:

```rust
pub const DB_PENTEST_METADATA: OperationMetadata = ...;
```

However, avoid making domain crates depend on main-crate `OperationMetadata` if that causes cycles. Options:

1. Move `OperationMetadata` to a lower-level crate such as `eggsec-tool-core`.
2. Keep metadata declarations in the main crate and only call domain crate execution functions.
3. Provide domain-local lightweight metadata and convert centrally.

Preferred long-term: move metadata primitives to `eggsec-tool-core` if they are stable and needed across crates.

## Step 1: Inventory extraction candidates

Create a short internal inventory before moving code:

For each domain:

- Current modules/files.
- Feature flags.
- External dependencies.
- Public types used by reports/TUI/MCP/REST.
- CLI commands and TUI tabs depending on it.
- Tool registry entries.
- Tests.

Start with DB pentest or web proxy because they are distinct defense-lab surfaces with clear policy classes.

## Step 2: Extract one crate first

Pick one initial crate, preferably `eggsec-db-lab` if it has contained logic, or `eggsec-web-proxy` if traffic-interception isolation is more urgent.

Move only domain logic first:

- Domain request/config types.
- Domain execution runner.
- Domain report fragments if they are not shared widely.
- Domain-specific helpers.

Leave protocol handlers, CLI parsing, TUI tabs, and enforcement in main crate for the first pass.

## Step 3: Add adapter layer in main crate

In the main `eggsec` crate, replace direct internal module calls with adapter calls into the new crate.

Example:

```rust
pub async fn run_db_pentest_cli(args, approved_context) -> Result<DbPentestReport> {
    eggsec_db_lab::run(args.into()).await.map_err(Into::into)
}
```

The adapter should remain responsible for:

- Constructing descriptors through metadata.
- Calling enforcement/preflight.
- Translating errors to Eggsec error/report types.
- Wiring CLI/TUI/MCP/REST.

## Step 4: Preserve feature flags

Update root `Cargo.toml`:

- Add workspace member.
- Add optional dependency from main crate.
- Map existing feature flag to the new crate dependency.

Example:

```toml
[features]
db-pentest = ["dep:eggsec-db-lab", ...]

[dependencies]
eggsec-db-lab = { path = "../eggsec-db-lab", optional = true }
```

Ensure default features do not accidentally pull heavy dependencies.

## Step 5: Move tests

Move domain-specific tests into the new crate.

Keep integration tests in the main crate for:

- CLI wiring.
- TUI wiring.
- MCP/REST exposure.
- Enforcement behavior.

Add one smoke test in main crate proving the adapter calls the domain crate.

## Step 6: Repeat for next domains

After one successful extraction, repeat in smaller passes:

1. Web proxy.
2. Wireless.
3. Evasion lab.
4. Post-exploitation lab.
5. C2 lab.
6. Load/stress lab if justified.

Each extraction should have its own commit or PR-sized pass.

## Step 7: Update docs

Update:

- `architecture/overview.md`
- `architecture/config.md`
- Domain docs such as web proxy/db/wireless docs
- `docs/ENFORCEMENT_MODES.md` if examples mention module layout

Document that enforcement remains in the main control plane.

## Acceptance criteria

For each extracted domain:

- New crate exists and is a workspace member.
- Main crate depends on it optionally behind the existing feature flag.
- No cyclic dependency exists.
- Existing CLI/TUI/MCP/REST behavior is preserved.
- Enforcement still occurs in the main crate before domain execution.
- Domain-specific tests pass in the new crate.
- Main integration tests pass.
- Compile-time dependency footprint improves or at least does not regress for default builds.

## Validation commands

Run after each extraction:

```bash
cargo fmt --all
cargo check --workspace --all-features
cargo test -p eggsec-db-lab
cargo test -p eggsec --features db-pentest,rest-api --lib
cargo test -p eggsec-tui --features db-pentest
```

Adjust crate/feature names for the domain under extraction.

## Non-goals

- Do not move enforcement into domain crates.
- Do not extract all domains in one pass.
- Do not change user-visible behavior intentionally.
- Do not remove feature gates.
- Do not block Phase 12 type-level dispatch on extraction completion.
