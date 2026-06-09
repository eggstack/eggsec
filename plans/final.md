# Slapper Modularization: Final Stabilization Handoff Plan

## Purpose

Slapper has completed several useful modularization passes:

- `slapper-core` now owns dependency-light shared primitives.
- `slapper-tui` owns the terminal UI.
- `slapper-cli` owns the user-facing binary named `slapper`.
- `slapper-output` owns portable report/output adapters.
- `slapper-tool-core` owns protocol-neutral tool DTOs.
- `slapper` remains the main assessment engine crate.
- `slapper-nse` remains optional Nmap NSE compatibility support.

This final handoff plan should close the modularization phase cleanly. It should not start a large scanner/API/MCP rewrite. The goal is to leave the repo coherent, documented, and ready for future incremental extraction when needed.

The only optional extraction in this pass is the low-risk agent coordination boundary identified in `architecture/api_extraction_boundary.md`: `crates/slapper/src/tool/agents/`. If that extraction becomes messy, stop and document the blockers. Do not force it.

## Current target architecture

Expected workspace shape at the end of this phase:

```text
crates/
  slapper-core/        # Dependency-light shared domain primitives
  slapper-tool-core/   # Protocol-neutral tool DTOs/data structures
  slapper-output/      # Portable output/report adapters
  slapper/             # Main engine: scanners, command dispatch, config/scope, runtime integrations
  slapper-nse/         # Optional Nmap NSE compatibility support
  slapper-tui/         # Terminal UI adapter
  slapper-cli/         # User-facing binary package; binary name `slapper`
```

Optional if clean:

```text
  slapper-agent/       # Tool-agent coordination primitives from tool/agents/
```

Do not add `slapper-api` in this pass unless it is already trivial and explicitly approved in local review. The existing boundary note shows API/MCP extraction still has meaningful coupling.

## Non-goals

Do not extract scanner, WAF, fuzzer, recon, loadtest, packet, stress, or NSE modules.

Do not extract MCP server internals.

Do not extract REST/OpenAI/OpenResponses/gRPC adapters in this pass.

Do not move `SlapperError` wholesale into `slapper-core`.

Do not move config/scope wholesale into `slapper-core`.

Do not redesign TUI, CLI, output schemas, report formats, command names, feature names, or config formats.

Do not remove compatibility shims unless all call sites are updated and checks pass.

Do not introduce wildcard re-exports or a broad prelude.

Do not create cyclic dependencies.

## Success criteria

At completion:

1. Workspace crate docs accurately describe all existing crates.
2. `architecture/overview.md` has no stale crate ownership claims.
3. `crates/slapper/src/lib.rs` has no stale crate ownership claims.
4. `architecture/compile_time_baseline.md` reflects the final modularization state.
5. `architecture/api_extraction_boundary.md` remains accurate after any final changes.
6. `slapper-output` dependency notes explain why `tokio` remains, or `tokio` is removed if unused.
7. `slapper-tool-core` dependencies are confirmed as used.
8. Feature forwarding in `slapper-cli` and `slapper-tui` is coherent.
9. If `slapper-agent` is extracted, it compiles and does not depend on the main `slapper` crate.
10. If `slapper-agent` is not extracted, blockers are documented precisely.
11. Core checks pass:
    - `cargo check -p slapper-core`
    - `cargo check -p slapper-tool-core`
    - `cargo check -p slapper-output`
    - `cargo check -p slapper --no-default-features`
    - `cargo check -p slapper-tui`
    - `cargo check -p slapper-cli`
12. Workspace no-default check passes or any failure is documented as pre-existing.

## Part 1: Correct remaining documentation drift

### 1. Fix `architecture/overview.md`

Known stale line to fix:

The `slapper` crate description may still say it owns “TUI/API adapters.” Since TUI is extracted, change this to:

```markdown
- **`slapper`**: main engine, CLI command model/dispatch, assessment modules, remaining API/agent adapters, feature-gated integrations, and the canonical `SlapperError` type.
```

Do not imply the main crate owns the binary or TUI.

### 2. Fix supporting module descriptions

In `architecture/overview.md`, the supporting module index may still say `types.rs` contains `Severity` and `SensitiveString`.

Update to reflect reality:

```markdown
| [`slapper-core`](../crates/slapper-core/) | Dependency-light shared types (`Severity`, `SensitiveString`) and constants | [types.md](types.md) |
| [`types.rs`](../crates/slapper/src/types.rs) | Main-crate compatibility facade plus CLI-facing types such as `OutputFormat` | [types.md](types.md) |
| [`constants.rs`](../crates/slapper/src/constants.rs) | Compatibility facade over core constants plus any engine-local constants | [constants.md](constants.md) |
```

Adjust wording based on the actual files.

### 3. Update `crates/slapper/src/lib.rs` only if needed

Confirm its workspace crate list is current:

```rust
//! - `slapper-core`
//! - `slapper-tool-core`
//! - `slapper-output`
//! - `slapper-nse`
//! - `slapper-tui`
//! - `slapper-cli`
```

If `slapper-agent` is added in this pass, add it to this list.

### 4. Update compile tracking

Update:

```text
architecture/compile_time_baseline.md
```

Add a final section:

```markdown
## Final modularization stabilization pass

### Workspace state

### Commands run

### Results

### Final interpretation

This completes the initial crate modularization phase. Further splits should be driven by measured compile-time hot paths or clearly isolated adapter boundaries.
```

Do not invent timing numbers. If wall-clock numbers are unavailable, record pass/fail only.

## Part 2: Dependency audit closeout

### 1. `slapper-output`

Inspect imports in:

```text
crates/slapper-output/src/
```

Confirm whether each manifest dependency is used:

```toml
slapper-core
serde
serde_json
chrono
rustc-hash
quick-xml
unicode-normalization
lru
uuid
hostname
tokio
```

If `tokio` is used only in `session` or `schedule`, add a comment to `crates/slapper-output/Cargo.toml`:

```toml
# Used by async session/schedule helpers in the output crate.
tokio = { workspace = true }
```

If `tokio` is unused, remove it and run checks.

Do not add heavy dependencies to `slapper-output`.

### 2. `slapper-tool-core`

Inspect imports in:

```text
crates/slapper-tool-core/src/
```

Confirm these dependencies are used:

```toml
slapper-core
serde
serde_json
chrono
rustc-hash
parking_lot
uuid
toml
```

Remove any unused dependency.

Do not let `slapper-tool-core` depend on `slapper`, API server crates, scanner implementation modules, or TUI/output crates.

### 3. Main `slapper`

Inspect `crates/slapper/Cargo.toml` for stale dependencies after prior extractions.

Candidate dependencies to verify:

```text
quick-xml
unicode-normalization
lru
hostname
handlebars
uuid
```

Remove only if unused in `crates/slapper/src`.

Do not remove dependencies merely because they are also present in another crate.

## Part 3: Feature forwarding closeout

### 1. Validate `slapper-cli` features

Inspect `crates/slapper-cli/Cargo.toml`.

Ensure forwarded features exist in target crates.

Expected examples:

```toml
nse = ["slapper/nse", "slapper-tui/nse"]
rest-api = ["slapper/rest-api", "slapper-tui/rest-api"]
stress-testing = ["slapper/stress-testing", "slapper-tui/stress-testing"]
pdf = ["slapper/pdf"]
full = ["slapper/full", "slapper-tui/full"]
```

If a feature is engine-only, do not forward it to `slapper-tui`.

If a feature enables optional TUI tabs, forwarding to both may be correct.

### 2. Validate `slapper-tui` features

Inspect `crates/slapper-tui/Cargo.toml`.

Confirm it does not own engine semantics. It may forward features to `slapper` only to compile optional views/tabs.

Do not redesign TUI tabs in this pass.

### 3. Run feature checks

Run:

```bash
cargo check -p slapper-cli
cargo check -p slapper-cli --features nse
cargo check -p slapper-cli --features rest-api
cargo check -p slapper-cli --features stress-testing
cargo check -p slapper-cli --features pdf
```

Run `full` if practical:

```bash
cargo check -p slapper-cli --features full
```

If `full` is too slow or fails for unrelated reasons, document that.

## Part 4: Optional low-risk extraction: `slapper-agent`

The API boundary note identifies `crates/slapper/src/tool/agents/` as lower-coupling than MCP or REST/gRPC adapters. This optional extraction should be attempted only if it stays small and acyclic.

### 1. Candidate files

Consider moving:

```text
crates/slapper/src/tool/agents/mod.rs
crates/slapper/src/tool/agents/registry.rs
crates/slapper/src/tool/agents/scheduler.rs
crates/slapper/src/tool/agents/lifecycle.rs
crates/slapper/src/tool/agents/communication.rs
crates/slapper/src/tool/agents/delegation.rs
crates/slapper/src/tool/agents/aggregator.rs
```

Do not move:

```text
crates/slapper/src/agent/
crates/slapper/src/tool/dispatcher.rs
crates/slapper/src/tool/registry.rs
crates/slapper/src/tool/implementations/
crates/slapper/src/tool/protocol/
```

### 2. Create crate only if clean

If imports are limited to shared DTOs/constants and common dependencies, create:

```text
crates/slapper-agent/
  Cargo.toml
  src/lib.rs
```

Suggested manifest:

```toml
[package]
name = "slapper-agent"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Agent coordination primitives for Slapper"

[dependencies]
slapper-core = { path = "../slapper-core" }
slapper-tool-core = { path = "../slapper-tool-core" }

serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
tokio = { workspace = true }
parking_lot = { workspace = true }
uuid = { workspace = true }
tracing = { workspace = true }
```

Add only dependencies actually used.

Update root workspace members if created.

### 3. Constants problem

If `tool/agents/` depends on constants from `crate::constants`, prefer one of these:

Option A: Use constants already in `slapper-core::constants` if available.

Option B: Move only pure numeric defaults into `slapper-core::constants`, such as:

```rust
DEFAULT_MAX_RETRIES
DEFAULT_SCHEDULER_RETRY_DELAY_MS
DEFAULT_LEASE_DURATION_MS
```

Option C: Define agent-local constants in `slapper-agent` if they are only used by agent coordination.

Do not make `slapper-agent` depend on the main `slapper` crate just for constants.

### 4. Compatibility shim

If extraction succeeds, update `crates/slapper/src/tool/mod.rs` or `crates/slapper/src/tool/agents/mod.rs` with a compatibility facade:

```rust
pub use slapper_agent::*;
```

or module-level re-exports that preserve existing paths.

Do not break `crate::tool::agents::AgentRegistry`-style imports.

### 5. Stop conditions for extraction

Stop and do not extract `slapper-agent` if:

1. The candidate files depend on `ToolRegistry`, `ToolDispatcher`, concrete tool implementations, scanner/fuzzer/WAF modules, or command handlers.
2. The extraction would require `slapper-agent -> slapper`.
3. The extraction changes agent behavior.
4. The compatibility shim becomes complex.
5. The diff becomes too large to review.

If blocked, update `architecture/api_extraction_boundary.md` with exact blockers and leave extraction for a later pass.

## Part 5: API boundary note update

Update:

```text
architecture/api_extraction_boundary.md
```

If `slapper-agent` extraction succeeds:

- Mark `tool/agents/` extraction as done.
- Update dependency graph.
- Revise next recommended phase to gRPC adapter extraction.

If it does not succeed:

- Add exact blockers.
- Keep proposed next pass as either `slapper-agent` prep or gRPC adapter prep.

Do not remove the MCP coupling warnings. They are important.

## Build/test checklist

Run:

```bash
cargo fmt
cargo check -p slapper-core
cargo check -p slapper-tool-core
cargo check -p slapper-output
cargo check -p slapper --no-default-features
cargo check -p slapper-tui
cargo check -p slapper-cli
```

If `slapper-agent` is created:

```bash
cargo check -p slapper-agent
cargo test -p slapper-agent
```

Run broader checks:

```bash
cargo check --workspace --all-targets --no-default-features
cargo test -p slapper-core
cargo test -p slapper-tool-core
cargo test -p slapper-output
cargo test -p slapper --lib --no-default-features
cargo test --workspace --no-default-features
```

Run feature checks:

```bash
cargo check -p slapper-cli --features nse
cargo check -p slapper-cli --features rest-api
cargo check -p slapper-cli --features stress-testing
cargo check -p slapper-cli --features pdf
cargo check -p slapper-cli --features full
```

If `full` fails for unrelated or pre-existing reasons, document it precisely.

## Final report requirements

Report:

1. Documentation corrections made.
2. Dependency audit results for `slapper-output`.
3. Dependency audit results for `slapper-tool-core`.
4. Main `slapper` stale dependency audit result.
5. Feature forwarding review result.
6. Whether `slapper-agent` was extracted.
7. If extracted, final dependency graph.
8. If not extracted, exact blockers.
9. Updates made to `architecture/api_extraction_boundary.md`.
10. Commands run and results.
11. Any known pre-existing failures.
12. Whether this modularization phase can be considered complete.

## Recommended stopping point

This should be the final pass of the initial modularization phase.

After this pass, do not continue splitting crates unless one of the following is true:

1. Compile-time measurements show a specific hot path.
2. A module boundary is already clean and low-risk.
3. A future feature requires a separate crate for API/agent/frontend reuse.
4. The main crate remains painful to iterate on despite the frontend/output/tool DTO splits.

Likely future candidates, in order:

```text
1. slapper-agent      # if not completed in this pass
2. slapper-api grpc   # gRPC adapter first; cleanest API adapter
3. slapper-api rest   # REST/OpenAI/OpenResponses after gRPC
4. MCP adapter        # last; deeply coupled
5. slapper-scan       # only after measured compile-time need
```

