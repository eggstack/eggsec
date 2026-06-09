# Eggsec Modularization: Final Agent Shim Correction Plan

## Purpose

The final stabilization pass mostly completed the modularization phase, but review found one remaining architectural inconsistency:

`eggsec-agent` now exists as a workspace crate, but `crates/eggsec/src/tool/agents/` still appears to contain local duplicated implementation files instead of acting as a compatibility facade over `eggsec-agent`.

This plan is intentionally narrow. It should fix the duplication, correct documentation, and re-run the final checks. Do not start another broad extraction.

## Current problem

The workspace includes:

```text
crates/eggsec-agent/
```

and the main `eggsec` crate depends on it.

However, the old files under:

```text
crates/eggsec/src/tool/agents/
```

still appear to contain actual implementation modules:

```text
aggregator.rs
communication.rs
delegation.rs
lifecycle.rs
registry.rs
scheduler.rs
mod.rs
```

The intended end state is that `eggsec-agent` owns those implementations, and `eggsec::tool::agents::*` remains available only as a compatibility re-export.

## Non-goals

Do not extract any API, MCP, REST, gRPC, scanner, WAF, fuzzer, recon, loadtest, packet, stress, or NSE modules.

Do not change agent behavior.

Do not change public type names.

Do not remove `eggsec::tool::agents::*` compatibility paths unless every call site is updated and checks pass.

Do not redesign the autonomous `crates/eggsec/src/agent/` module.

Do not introduce a dependency from `eggsec-agent` back to `eggsec`.

Do not create `eggsec-api` in this pass.

## Success criteria

After this pass:

1. `crates/eggsec-agent/src/` owns the agent coordination implementations.
2. `crates/eggsec/src/tool/agents/` no longer contains duplicated implementation files.
3. `crates/eggsec/src/tool/agents/mod.rs` is a thin compatibility facade over `eggsec-agent`, or `crates/eggsec/src/tool/mod.rs` re-exports `eggsec_agent` as `agents`.
4. Existing imports such as `crate::tool::agents::AgentRegistry` still work.
5. `architecture/overview.md` lists `eggsec-agent` in the crate layout.
6. `architecture/api_extraction_boundary.md` accurately states that `tool/agents/` was extracted and now re-exported.
7. `architecture/compile_time_baseline.md` accurately reflects the final state.
8. Core checks pass.

## Part 1: Verify duplicated files

Compare these files:

```text
crates/eggsec/src/tool/agents/registry.rs
crates/eggsec-agent/src/registry.rs

crates/eggsec/src/tool/agents/scheduler.rs
crates/eggsec-agent/src/scheduler.rs

crates/eggsec/src/tool/agents/lifecycle.rs
crates/eggsec-agent/src/lifecycle.rs

crates/eggsec/src/tool/agents/communication.rs
crates/eggsec-agent/src/communication.rs

crates/eggsec/src/tool/agents/delegation.rs
crates/eggsec-agent/src/delegation.rs

crates/eggsec/src/tool/agents/aggregator.rs
crates/eggsec-agent/src/aggregator.rs
```

If they are identical or semantically equivalent, proceed with shim replacement.

If they differ, inspect the diff and preserve the intended latest implementation in `eggsec-agent`.

Do not delete code until the canonical `eggsec-agent` implementation is confirmed.

## Part 2: Replace local implementation with compatibility shim

Preferred approach:

Keep only:

```text
crates/eggsec/src/tool/agents/mod.rs
```

and replace its contents with:

```rust
//! Compatibility facade for agent coordination primitives.
//!
//! The implementation lives in the `eggsec-agent` crate. This module preserves
//! existing `eggsec::tool::agents::*` import paths during the workspace
//! modularization transition.

pub use eggsec_agent::*;
```

Then delete:

```text
crates/eggsec/src/tool/agents/aggregator.rs
crates/eggsec/src/tool/agents/communication.rs
crates/eggsec/src/tool/agents/delegation.rs
crates/eggsec/src/tool/agents/lifecycle.rs
crates/eggsec/src/tool/agents/registry.rs
crates/eggsec/src/tool/agents/scheduler.rs
```

Alternative approach if module path behavior requires it:

Instead of keeping `crates/eggsec/src/tool/agents/mod.rs`, remove the `agents/` directory and in `crates/eggsec/src/tool/mod.rs` use:

```rust
pub mod agents {
    //! Compatibility facade for agent coordination primitives.
    //!
    //! The implementation lives in the `eggsec-agent` crate.

    pub use eggsec_agent::*;
}
```

Use whichever approach causes less churn.

## Part 3: Resolve missing re-exports

After replacing with `pub use eggsec_agent::*`, run checks.

If code fails because previously re-exported names are not exposed by `eggsec-agent`, add those public re-exports to `crates/eggsec-agent/src/lib.rs`.

Known names previously exported by `crates/eggsec/src/tool/agents/mod.rs` may include:

```rust
AgentCapability
AgentMessage
CapabilityAdvertisement
HealthMetrics
HealthStatus
InterAgentChannel
MessageType
MultiAgentCoordinator

DelegationRequest
DelegationResponse

AgentHealth
HealthIssue
LifecycleConfig
LifecycleEvent
LifecycleEventType
LifecycleManager

AgentInfo
AgentRegistry
AgentStatus

ScheduledTask
TaskOutcome
TaskPriority
TaskQueue
TaskScheduler
TaskStatus

AggregatedResult
ResultAggregator
```

Ensure `eggsec-agent` exports every public type expected by existing call sites.

Do not expose private helper types unnecessarily.

## Part 4: Confirm dependency direction

The intended dependency direction is:

```text
eggsec-core -> eggsec-agent
eggsec-agent -> no eggsec dependency

eggsec -> eggsec-agent
```

Confirm `crates/eggsec-agent/Cargo.toml` does not depend on:

```text
eggsec
eggsec-output
eggsec-tui
eggsec-cli
eggsec-nse
axum
tonic
pnet
sqlx
headless_chrome
```

Current `eggsec-agent` may depend on `reqwest`. Verify this is actually used. If it is used only by `communication` or callback/webhook dispatch, keep it and document why. If unused, remove it.

## Part 5: Documentation corrections

### 1. `architecture/overview.md`

Add `eggsec-agent` to the crate layout section:

```markdown
- **`eggsec-agent`**: agent coordination primitives extracted from `eggsec::tool::agents` (registry, scheduler, lifecycle, communication, delegation, aggregation). Depends on `eggsec-core` but not the main engine crate.
```

Also ensure the `eggsec` crate description does not claim ownership of TUI or extracted agent coordination primitives.

Suggested `eggsec` wording:

```markdown
- **`eggsec`**: main engine, CLI command model/dispatch, assessment modules, remaining API/agent adapters, feature-gated integrations, and the canonical `EggsecError` type.
```

### 2. `architecture/api_extraction_boundary.md`

Ensure Phase 2 says:

```markdown
### Phase 2: Extract tool agent coordination to eggsec-agent (DONE)

Implementation lives in `crates/eggsec-agent/src/`.
The `eggsec` crate preserves `eggsec::tool::agents::*` as a compatibility facade.
```

Do not claim local files were moved if implementation files still remain locally. After this pass, the statement should be true.

### 3. `architecture/compile_time_baseline.md`

Update the final stabilization section to explicitly state:

```markdown
- `eggsec-agent` owns the implementation.
- `eggsec::tool::agents` is a compatibility facade over `eggsec-agent`.
- Duplicate local implementation files were removed from `crates/eggsec/src/tool/agents/`.
```

## Part 6: Build/test checklist

Run:

```bash
cargo fmt
cargo check -p eggsec-agent
cargo test -p eggsec-agent
cargo check -p eggsec --no-default-features
cargo check -p eggsec-cli
cargo check -p eggsec-tui
cargo check -p eggsec-output
cargo check -p eggsec-tool-core
cargo check -p eggsec-core
```

Run workspace no-default checks:

```bash
cargo check --workspace --all-targets --no-default-features
cargo test --workspace --no-default-features
```

Run selected feature checks:

```bash
cargo check -p eggsec-cli --features nse
cargo check -p eggsec-cli --features rest-api
cargo check -p eggsec-cli --features stress-testing
cargo check -p eggsec-cli --features pdf
```

Run `full` if practical:

```bash
cargo check -p eggsec-cli --features full
```

If `full` fails due to known unrelated/pre-existing issues, document precisely.

## Final handoff report

Report:

1. Whether duplicated `crates/eggsec/src/tool/agents/*.rs` files were removed.
2. Final contents of `crates/eggsec/src/tool/agents/mod.rs` or equivalent shim location.
3. Any additional public re-exports added to `eggsec-agent`.
4. Whether `eggsec-agent` depends on `eggsec` — it should not.
5. Whether `reqwest` remains in `eggsec-agent`, and why.
6. Docs updated.
7. Commands run and results.
8. Any pre-existing failures.
9. Whether the initial modularization phase can now be considered complete.

## Stop conditions

Stop and report instead of forcing changes if:

1. Local `tool/agents` files have behavior not present in `eggsec-agent`.
2. Replacing local modules with a shim causes widespread breakage.
3. `eggsec-agent` would need to depend on `eggsec`.
4. Public type re-exports become ambiguous or conflicting.
5. The diff expands beyond this narrow correction.

## Completion definition

This modularization phase is complete when:

```text
crates/eggsec-agent/src/*.rs       owns implementation
crates/eggsec/src/tool/agents/     is shim-only or removed
eggsec::tool::agents::*            still works
architecture docs                   describe the final crate graph accurately
cargo check/test baseline            passes or known failures are documented
```

After this, stop crate-splitting work unless compile-time measurements justify another targeted pass.

