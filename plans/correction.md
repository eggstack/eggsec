# Slapper Modularization: Final Agent Shim Correction Plan

## Purpose

The final stabilization pass mostly completed the modularization phase, but review found one remaining architectural inconsistency:

`slapper-agent` now exists as a workspace crate, but `crates/slapper/src/tool/agents/` still appears to contain local duplicated implementation files instead of acting as a compatibility facade over `slapper-agent`.

This plan is intentionally narrow. It should fix the duplication, correct documentation, and re-run the final checks. Do not start another broad extraction.

## Current problem

The workspace includes:

```text
crates/slapper-agent/
```

and the main `slapper` crate depends on it.

However, the old files under:

```text
crates/slapper/src/tool/agents/
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

The intended end state is that `slapper-agent` owns those implementations, and `slapper::tool::agents::*` remains available only as a compatibility re-export.

## Non-goals

Do not extract any API, MCP, REST, gRPC, scanner, WAF, fuzzer, recon, loadtest, packet, stress, or NSE modules.

Do not change agent behavior.

Do not change public type names.

Do not remove `slapper::tool::agents::*` compatibility paths unless every call site is updated and checks pass.

Do not redesign the autonomous `crates/slapper/src/agent/` module.

Do not introduce a dependency from `slapper-agent` back to `slapper`.

Do not create `slapper-api` in this pass.

## Success criteria

After this pass:

1. `crates/slapper-agent/src/` owns the agent coordination implementations.
2. `crates/slapper/src/tool/agents/` no longer contains duplicated implementation files.
3. `crates/slapper/src/tool/agents/mod.rs` is a thin compatibility facade over `slapper-agent`, or `crates/slapper/src/tool/mod.rs` re-exports `slapper_agent` as `agents`.
4. Existing imports such as `crate::tool::agents::AgentRegistry` still work.
5. `architecture/overview.md` lists `slapper-agent` in the crate layout.
6. `architecture/api_extraction_boundary.md` accurately states that `tool/agents/` was extracted and now re-exported.
7. `architecture/compile_time_baseline.md` accurately reflects the final state.
8. Core checks pass.

## Part 1: Verify duplicated files

Compare these files:

```text
crates/slapper/src/tool/agents/registry.rs
crates/slapper-agent/src/registry.rs

crates/slapper/src/tool/agents/scheduler.rs
crates/slapper-agent/src/scheduler.rs

crates/slapper/src/tool/agents/lifecycle.rs
crates/slapper-agent/src/lifecycle.rs

crates/slapper/src/tool/agents/communication.rs
crates/slapper-agent/src/communication.rs

crates/slapper/src/tool/agents/delegation.rs
crates/slapper-agent/src/delegation.rs

crates/slapper/src/tool/agents/aggregator.rs
crates/slapper-agent/src/aggregator.rs
```

If they are identical or semantically equivalent, proceed with shim replacement.

If they differ, inspect the diff and preserve the intended latest implementation in `slapper-agent`.

Do not delete code until the canonical `slapper-agent` implementation is confirmed.

## Part 2: Replace local implementation with compatibility shim

Preferred approach:

Keep only:

```text
crates/slapper/src/tool/agents/mod.rs
```

and replace its contents with:

```rust
//! Compatibility facade for agent coordination primitives.
//!
//! The implementation lives in the `slapper-agent` crate. This module preserves
//! existing `slapper::tool::agents::*` import paths during the workspace
//! modularization transition.

pub use slapper_agent::*;
```

Then delete:

```text
crates/slapper/src/tool/agents/aggregator.rs
crates/slapper/src/tool/agents/communication.rs
crates/slapper/src/tool/agents/delegation.rs
crates/slapper/src/tool/agents/lifecycle.rs
crates/slapper/src/tool/agents/registry.rs
crates/slapper/src/tool/agents/scheduler.rs
```

Alternative approach if module path behavior requires it:

Instead of keeping `crates/slapper/src/tool/agents/mod.rs`, remove the `agents/` directory and in `crates/slapper/src/tool/mod.rs` use:

```rust
pub mod agents {
    //! Compatibility facade for agent coordination primitives.
    //!
    //! The implementation lives in the `slapper-agent` crate.

    pub use slapper_agent::*;
}
```

Use whichever approach causes less churn.

## Part 3: Resolve missing re-exports

After replacing with `pub use slapper_agent::*`, run checks.

If code fails because previously re-exported names are not exposed by `slapper-agent`, add those public re-exports to `crates/slapper-agent/src/lib.rs`.

Known names previously exported by `crates/slapper/src/tool/agents/mod.rs` may include:

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

Ensure `slapper-agent` exports every public type expected by existing call sites.

Do not expose private helper types unnecessarily.

## Part 4: Confirm dependency direction

The intended dependency direction is:

```text
slapper-core -> slapper-agent
slapper-agent -> no slapper dependency

slapper -> slapper-agent
```

Confirm `crates/slapper-agent/Cargo.toml` does not depend on:

```text
slapper
slapper-output
slapper-tui
slapper-cli
slapper-nse
axum
tonic
pnet
sqlx
headless_chrome
```

Current `slapper-agent` may depend on `reqwest`. Verify this is actually used. If it is used only by `communication` or callback/webhook dispatch, keep it and document why. If unused, remove it.

## Part 5: Documentation corrections

### 1. `architecture/overview.md`

Add `slapper-agent` to the crate layout section:

```markdown
- **`slapper-agent`**: agent coordination primitives extracted from `slapper::tool::agents` (registry, scheduler, lifecycle, communication, delegation, aggregation). Depends on `slapper-core` but not the main engine crate.
```

Also ensure the `slapper` crate description does not claim ownership of TUI or extracted agent coordination primitives.

Suggested `slapper` wording:

```markdown
- **`slapper`**: main engine, CLI command model/dispatch, assessment modules, remaining API/agent adapters, feature-gated integrations, and the canonical `SlapperError` type.
```

### 2. `architecture/api_extraction_boundary.md`

Ensure Phase 2 says:

```markdown
### Phase 2: Extract tool agent coordination to slapper-agent (DONE)

Implementation lives in `crates/slapper-agent/src/`.
The `slapper` crate preserves `slapper::tool::agents::*` as a compatibility facade.
```

Do not claim local files were moved if implementation files still remain locally. After this pass, the statement should be true.

### 3. `architecture/compile_time_baseline.md`

Update the final stabilization section to explicitly state:

```markdown
- `slapper-agent` owns the implementation.
- `slapper::tool::agents` is a compatibility facade over `slapper-agent`.
- Duplicate local implementation files were removed from `crates/slapper/src/tool/agents/`.
```

## Part 6: Build/test checklist

Run:

```bash
cargo fmt
cargo check -p slapper-agent
cargo test -p slapper-agent
cargo check -p slapper --no-default-features
cargo check -p slapper-cli
cargo check -p slapper-tui
cargo check -p slapper-output
cargo check -p slapper-tool-core
cargo check -p slapper-core
```

Run workspace no-default checks:

```bash
cargo check --workspace --all-targets --no-default-features
cargo test --workspace --no-default-features
```

Run selected feature checks:

```bash
cargo check -p slapper-cli --features nse
cargo check -p slapper-cli --features rest-api
cargo check -p slapper-cli --features stress-testing
cargo check -p slapper-cli --features pdf
```

Run `full` if practical:

```bash
cargo check -p slapper-cli --features full
```

If `full` fails due to known unrelated/pre-existing issues, document precisely.

## Final handoff report

Report:

1. Whether duplicated `crates/slapper/src/tool/agents/*.rs` files were removed.
2. Final contents of `crates/slapper/src/tool/agents/mod.rs` or equivalent shim location.
3. Any additional public re-exports added to `slapper-agent`.
4. Whether `slapper-agent` depends on `slapper` — it should not.
5. Whether `reqwest` remains in `slapper-agent`, and why.
6. Docs updated.
7. Commands run and results.
8. Any pre-existing failures.
9. Whether the initial modularization phase can now be considered complete.

## Stop conditions

Stop and report instead of forcing changes if:

1. Local `tool/agents` files have behavior not present in `slapper-agent`.
2. Replacing local modules with a shim causes widespread breakage.
3. `slapper-agent` would need to depend on `slapper`.
4. Public type re-exports become ambiguous or conflicting.
5. The diff expands beyond this narrow correction.

## Completion definition

This modularization phase is complete when:

```text
crates/slapper-agent/src/*.rs       owns implementation
crates/slapper/src/tool/agents/     is shim-only or removed
slapper::tool::agents::*            still works
architecture docs                   describe the final crate graph accurately
cargo check/test baseline            passes or known failures are documented
```

After this, stop crate-splitting work unless compile-time measurements justify another targeted pass.

