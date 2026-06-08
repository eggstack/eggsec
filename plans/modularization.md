# Slapper Crate Modularization Refactor: First Pass Handoff Plan

## Purpose

Slapper has grown into a large Rust security assessment toolkit with many semi-independent subsystems inside the main `slapper` crate. The immediate goal of this refactor is to reduce rapid-iteration compile time and improve architectural boundaries without attempting a full rewrite or a broad crate explosion.

This first pass should extract a small, dependency-light `slapper-core` crate and establish a repeatable pattern for future extractions. The first pass is intentionally conservative: move stable shared types and primitives, preserve behavior, avoid changing command semantics, and avoid extracting scanner/web/API/TUI crates until the dependency direction is proven.

## Repository context

The workspace currently contains:

```text
crates/
  slapper/
  slapper-nse/
```

The main `slapper` crate still owns most product functionality, including CLI parsing, command handlers, config, scanner, fuzzer, WAF, recon, load testing, pipeline, TUI, output, distributed mode, proxying, packet support, stress testing, tool/API/agent integrations, and optional AI/browser/database/container/SBOM/PDF integrations.

The existing architecture documentation already describes major module seams:

```text
User Interfaces:
  cli/
  tui/
  REST/API/MCP/OpenAI agent interfaces

Command Dispatch:
  commands/

Core Security Modules:
  scanner/
  fuzzer/
  waf/
  recon/
  loadtest/
  auth/
  proxy/
  stress/
  packet/
  pipeline/

Infrastructure:
  config/
  distributed/
  output/
  storage/
  workflow/

Integration:
  ai/
  nse/
  browser/
  integrations/
  notify/

Supporting:
  types.rs
  constants.rs
  error/
  findings/
  logging/
  utils/
  auth_context/
```

This plan starts by extracting only the supporting/core domain layer.

## Non-goals for this pass

Do not extract `scanner`, `waf`, `fuzzer`, `recon`, `loadtest`, `tui`, `api`, `agent`, `packet`, or `stress` into separate crates yet.

Do not change CLI behavior.

Do not alter feature semantics except where necessary to compile after moving code.

Do not attempt to optimize release profile settings.

Do not remove functionality.

Do not convert all module paths in one enormous change if it makes the diff unreviewable.

Do not introduce a “prelude” that re-exports the whole system and recreates the monolith.

## Success criteria

After this pass:

1. The workspace contains a new `crates/slapper-core` crate.
2. `slapper-core` compiles with a small dependency set and does not depend on the main `slapper` crate.
3. The main `slapper` crate depends on `slapper-core`.
4. Shared domain primitives are imported from `slapper-core` rather than being defined inside the main crate.
5. Existing tests pass, or failures are limited to path/import churn with clear fixes.
6. `cargo check -p slapper-core` is fast and independent of heavy optional dependencies.
7. `cargo check -p slapper --no-default-features` still works.
8. Feature-gated builds used by the repo still compile.
9. Compile timing baseline and post-refactor timing measurements are recorded in a Markdown note.

## Measurement baseline

Before moving code, collect compile/check timing data. Create a file at:

```text
architecture/compile_time_baseline.md
```

Record the date, machine if known, Rust version, and these command results.

Use:

```bash
rustc --version
cargo --version
cargo clean
cargo check --workspace --all-targets --no-default-features
cargo clean
cargo check -p slapper --no-default-features
cargo clean
cargo check -p slapper --features rest-api
cargo clean
cargo check -p slapper --features nse
cargo clean
cargo check -p slapper --features stress-testing
```

If any feature combination does not compile on the current main branch, record it as pre-existing and continue. Do not fix unrelated feature breakage as part of the crate split unless it blocks the extraction.

Also run:

```bash
cargo build --timings -p slapper --no-default-features
```

Commit or save the generated timing summary path in the baseline note if practical. The exact HTML does not need to be committed unless repo convention allows it.

Then, after the refactor, append the same command set under a “Post-refactor measurements” heading. The goal is not necessarily a large cold-build improvement from the first pass. The goal is to prove a clean independent core crate and establish measurement discipline for later extraction.

## Target new crate: `slapper-core`

Create:

```text
crates/slapper-core/
  Cargo.toml
  src/
    lib.rs
```

Add it to the workspace members in root `Cargo.toml`:

```toml
[workspace]
members = [
    "crates/slapper-core",
    "crates/slapper",
    "crates/slapper-nse",
]
resolver = "2"
```

Prefer placing `slapper-core` before `slapper` in the member list because `slapper` will depend on it.

### `slapper-core` package metadata

Use workspace metadata where possible:

```toml
[package]
name = "slapper-core"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Core domain types, errors, config primitives, and scope enforcement for Slapper"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
anyhow = { workspace = true }
chrono = { workspace = true }
url = { workspace = true }
regex = { workspace = true }
tracing = { workspace = true }
```

This dependency list is a starting point, not a mandate. Keep it smaller if the moved code permits. Do not add `tokio`, `reqwest`, `ratatui`, `crossterm`, `axum`, `tonic`, `pnet`, `headless_chrome`, `sqlx`, `kube`, `printpdf`, `slapper-nse`, or other heavy/integration dependencies to `slapper-core`.

If a candidate module requires a heavy dependency, leave that module in `slapper` for now.

## Candidate moves for this pass

Move only modules that are stable, shared, and dependency-light.

Primary candidates:

```text
crates/slapper/src/types.rs        -> crates/slapper-core/src/types.rs
crates/slapper/src/constants.rs    -> crates/slapper-core/src/constants.rs
crates/slapper/src/error/          -> crates/slapper-core/src/error/
```

Secondary candidates, only if dependency-light after inspection:

```text
crates/slapper/src/findings/       -> crates/slapper-core/src/findings/
crates/slapper/src/config/         -> crates/slapper-core/src/config/
crates/slapper/src/auth_context/   -> crates/slapper-core/src/auth_context/
```

Be conservative with `config/` and `auth_context/`. Move them only if they do not drag in CLI, TUI, filesystem watcher, network, command handler, or runtime-specific concerns. If config is mixed, split only the pure data model/scope enforcement pieces into `slapper-core` and leave loading/UI/runtime-specific config behavior inside `slapper`.

Suggested final `slapper-core/src/lib.rs`:

```rust
pub mod constants;
pub mod error;
pub mod types;

// Move these only if dependency-light enough.
pub mod findings;
pub mod config;
pub mod auth_context;

pub use error::{Result, SlapperError};
pub use types::Severity;
```

Do not blindly include modules that were not moved.

## Dependency direction rules

The dependency graph must be acyclic and should follow this direction:

```text
slapper-core
  ↑
slapper
  ↑
binary / adapters
```

For this pass, only `slapper` depends on `slapper-core`.

Rules:

1. `slapper-core` must not depend on `slapper`.
2. `slapper-core` must not depend on `slapper-nse`.
3. `slapper-core` must not contain CLI, TUI, API server, MCP server, gRPC, packet capture, raw socket, NSE runtime, headless browser, SQLx, Kubernetes, PDF, or AI client code.
4. `slapper-core` may define domain types used by those systems.
5. `slapper` may temporarily re-export moved types to reduce import churn.

## Main crate compatibility shim

In `crates/slapper/src/lib.rs`, after moving modules, replace direct module declarations with re-exports where useful.

For example, if `types.rs`, `constants.rs`, and `error/` are moved:

```rust
pub use slapper_core::constants;
pub use slapper_core::error;
pub use slapper_core::types;

pub use slapper_core::{Result, SlapperError};
pub use slapper_core::types::Severity;
```

If downstream internal modules currently use `crate::error::Result` or `crate::types::Severity`, this compatibility shim can preserve most paths during the first pass.

Avoid a broad `pub use slapper_core::*;`. Re-export explicit modules/types only.

Add this dependency to `crates/slapper/Cargo.toml`:

```toml
slapper-core = { path = "../slapper-core" }
```

Use hyphenated package name in Cargo and underscored crate path in Rust:

```rust
use slapper_core::types::Severity;
```

## Import migration strategy

Prefer a two-stage import migration.

Stage 1: Keep compatibility re-exports in `slapper/src/lib.rs` so existing `crate::types`, `crate::error`, and `crate::constants` paths mostly continue to work.

Stage 2: Update internal imports opportunistically to use `slapper_core::...` only where doing so is straightforward and improves clarity.

Do not churn every file unnecessarily. The purpose of the first pass is to establish the crate boundary, not to produce maximal import purity.

Good examples:

```rust
use slapper_core::{Result, SlapperError};
use slapper_core::types::Severity;
```

Acceptable transitional examples:

```rust
use crate::error::Result;
use crate::types::Severity;
```

Bad examples:

```rust
use slapper_core::*;
```

## Feature handling

`slapper-core` should ideally have no features during this pass.

If a moved module currently has conditional code tied to main-crate features, do not move that module yet unless the feature is truly core and can be cleanly represented in `slapper-core`.

Keep the existing feature flags in `crates/slapper/Cargo.toml` for now. Do not relocate feature flags to workspace-level features in this pass.

## Module-by-module checklist

### 1. `types.rs`

Inspect for dependencies and references to main-crate modules.

Move if it only uses standard library and lightweight dependencies.

After moving, update paths:

```rust
crate::types::Severity
```

should continue to work via compatibility re-export, but new code may use:

```rust
slapper_core::types::Severity
```

Confirm that `pub use types::Severity` in the main crate is updated to `pub use slapper_core::types::Severity`.

### 2. `constants.rs`

Move if it contains pure constants and no runtime-specific code.

Keep constants grouped and documented.

If constants are strongly tied to a subsystem, consider leaving those constants with that subsystem rather than forcing all constants into core. The core crate should not become a junk drawer.

### 3. `error/`

Move if the canonical error type is used across domains and does not depend on subsystem-specific concrete types.

If `SlapperError` contains variants wrapping errors from heavy dependencies, revise the variants to avoid heavy concrete types in core. Prefer string/context variants or lightweight standard/library errors for this first pass.

For example, avoid core variants that require:

```rust
reqwest::Error
sqlx::Error
tonic::Status
pnet::...
headless_chrome::...
```

If such variants exist and are not easy to abstract, either keep `error/` in `slapper` for now or split the error type into:

```text
slapper-core::CoreError
slapper::SlapperError
```

However, prefer preserving `SlapperError` if the move is straightforward.

### 4. `findings/`

Move only if it is mostly data models, finding fingerprints, severity, lifecycle status, and serialization.

Do not move if it depends on storage, workflow engines, command handlers, report generation, or database code.

If mixed, extract only pure model definitions into `slapper-core::findings` and leave stores/backends in `slapper`.

### 5. `config/`

This is the highest-risk candidate.

Move only pure config structs and scope enforcement types if feasible.

Keep file loading, directory discovery, watcher/debouncer integration, TUI settings glue, CLI-specific defaults, and environment-specific runtime behavior in `slapper`.

A clean split might look like:

```text
slapper-core/src/config/
  mod.rs              # pure config structs
  scope.rs            # Scope, target validation, authorization boundaries
  defaults.rs         # constants/default value functions if lightweight

slapper/src/config/
  loader.rs           # TOML/YAML filesystem loading
  paths.rs            # directories/project path resolution
  watch.rs            # notify/debouncer logic
```

Do not do this split unless the existing code makes it reasonably straightforward.

## Build/test commands

Run these after each meaningful migration step:

```bash
cargo fmt
cargo check -p slapper-core
cargo check -p slapper --no-default-features
cargo test -p slapper-core
cargo test -p slapper --lib --no-default-features
```

At the end, run broader checks:

```bash
cargo check --workspace --all-targets --no-default-features
cargo check -p slapper --features rest-api
cargo check -p slapper --features nse
cargo check -p slapper --features stress-testing
cargo test --workspace --no-default-features
```

If a feature combination was already broken before the refactor, note it in `architecture/compile_time_baseline.md` and do not attempt unrelated repairs.

## Documentation updates

Update architecture documentation to mention the new crate boundary.

At minimum, update:

```text
architecture/overview.md
crates/slapper/src/lib.rs crate-level docs
```

Add a short section to `architecture/overview.md`:

```markdown
## Crate layout

Slapper is organized as a Cargo workspace. The first-level crate boundary is:

- `slapper-core`: dependency-light domain types, canonical errors, constants, scope/config primitives, and shared finding models.
- `slapper`: main engine, CLI dispatch, assessment modules, TUI/API adapters, and feature-gated integrations.
- `slapper-nse`: optional Nmap NSE compatibility runtime and libraries.

New modules should avoid adding heavy runtime dependencies to `slapper-core`.
```

Update stale wording that implies everything lives in one crate.

If `README.md` has a crate/module architecture section, update it only minimally in this pass.

## Guardrails

Keep changes mechanical and reviewable.

Avoid renaming public types unless unavoidable.

Avoid changing serialization formats.

Avoid changing CLI output.

Avoid changing feature names.

Avoid adding new behavior.

Avoid adding new dependencies to `slapper-core` unless necessary.

Avoid moving mixed runtime modules into core just because they are shared.

If a module extraction becomes complex, stop and leave that module in `slapper`.

## Expected first-pass diff shape

Expected new files:

```text
crates/slapper-core/Cargo.toml
crates/slapper-core/src/lib.rs
crates/slapper-core/src/types.rs
crates/slapper-core/src/constants.rs
crates/slapper-core/src/error/...
architecture/compile_time_baseline.md
```

Expected modified files:

```text
Cargo.toml
crates/slapper/Cargo.toml
crates/slapper/src/lib.rs
architecture/overview.md
```

Possible modified files:

```text
crates/slapper/src/**/*.rs
README.md
architecture/*.md
```

Do not worry if some path updates are required across many files, but prefer compatibility re-exports to reduce unnecessary churn.

## Validation checklist for final handoff report

When complete, report:

1. New crate added and its dependency list.
2. Exact modules moved.
3. Modules intentionally not moved and why.
4. Any compatibility re-exports left in `slapper/src/lib.rs`.
5. All commands run and results.
6. Pre/post timing observations from `architecture/compile_time_baseline.md`.
7. Any pre-existing feature build failures.
8. Any follow-up extraction candidates.

## Suggested next pass after this one

Do not implement this next pass now, but leave notes for it.

Likely second-pass candidates:

```text
slapper-output
slapper-tui
slapper-api or slapper-agent-api
slapper-scan
slapper-web
```

The best second-pass target should be chosen from timing data and edit frequency. If the user mostly edits scanner/probe/recon logic, extract `slapper-scan`. If compile time is dominated by UI/API/report dependencies, extract adapter crates first.

## Architectural intent

The desired long-term shape is:

```text
slapper-core
  ├── slapper-scan
  ├── slapper-web
  ├── slapper-output
  ├── slapper-packet
  ├── slapper-stress
  ├── slapper-nse
  └── adapter crates
        ├── slapper-cli
        ├── slapper-tui
        ├── slapper-api
        └── slapper-agent / slapper-mcp
```

This first pass should not try to reach that final state. Its job is to create the core crate cleanly, keep behavior stable, measure the result, and make the next extraction easier.

