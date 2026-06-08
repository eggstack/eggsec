# Slapper Crate Modularization Refactor: Second Pass Handoff Plan

## Purpose

The first modularization pass created `crates/slapper-core` and moved dependency-light shared primitives such as `Severity`, `SensitiveString`, and centralized constants. This second pass should do two things:

1. Clean up the first pass so `slapper-core` is accurate, minimal, and documented correctly.
2. Extract the TUI into a dedicated `slapper-tui` crate to isolate `ratatui`/`crossterm` and reduce main-crate adapter coupling.

This pass should remain conservative. Do not extract scanner, WAF, fuzzer, recon, loadtest, API, MCP, agent, packet, or stress modules yet.

## Current repo shape

The workspace currently contains:

```text
crates/
  slapper-core/
  slapper/
  slapper-nse/
```

`slapper-core` currently exposes:

```rust
pub mod constants;
pub mod types;

pub use types::Severity;
```

The main `slapper` crate depends on `slapper-core`, but still owns almost all subsystems, including CLI, command dispatch, config, errors, findings, scanner, fuzzer, WAF, recon, load testing, output, TUI, API/tool integrations, packet/stress modules, and optional integrations.

The first pass was structurally successful but shallow. The next pass should make the split more useful by isolating a heavy adapter subsystem.

## Non-goals

Do not change TUI behavior.

Do not redesign the TUI.

Do not extract API/MCP/gRPC code in this pass.

Do not extract scanner/web/security modules in this pass.

Do not move `SlapperError` into `slapper-core` unless required for `slapper-tui` extraction and demonstrably low risk.

Do not move `config` wholesale into `slapper-core`.

Do not remove `slapper/src/tui` until imports are migrated and builds pass.

Do not change CLI command names, feature names, output formats, config formats, or serialization behavior.

Do not introduce broad `pub use slapper::*` or `pub use slapper_core::*` prelude-style exports.

## Success criteria

After this pass:

1. Workspace includes a new `crates/slapper-tui` crate.
2. `ratatui` and `crossterm` are dependencies of `slapper-tui`, not direct mandatory dependencies of the main `slapper` library unless still needed elsewhere.
3. The main `slapper` crate no longer declares a large `pub mod tui;` implementation module directly, or retains only a thin compatibility re-export behind a feature.
4. The `slapper` binary still builds and can launch the same TUI behavior.
5. `slapper-core` manifest and docs accurately reflect what it contains.
6. `slapper-core` dependencies are pruned to what is actually used.
7. Compile-time baseline documentation exists and records current/post-pass checks.
8. `cargo check -p slapper-core` remains fast and independent of heavy runtime/UI dependencies.
9. `cargo check -p slapper-tui` works independently.
10. `cargo check -p slapper --no-default-features` still works.
11. Feature-gated checks that previously worked still work.

## Part 1: Clean up first-pass core extraction

### 1. Fix `slapper-core` package description

The current manifest description may imply that errors and scope enforcement live in `slapper-core`, while the actual crate currently contains only `constants` and `types`.

Update `crates/slapper-core/Cargo.toml` description to something accurate:

```toml
description = "Dependency-light domain types, constants, and shared primitives for Slapper"
```

Do not mention errors, config, or scope enforcement unless those modules are actually moved.

### 2. Prune `slapper-core` dependencies

Inspect `crates/slapper-core/src/**/*.rs`.

Remove any dependency from `crates/slapper-core/Cargo.toml` that is not used by the exposed code.

Expected likely minimal set at the start of this pass:

```toml
[dependencies]
serde = { workspace = true }
subtle = "2"
zeroize = { version = "1", features = ["derive"] }
```

Keep `serde_json`, `thiserror`, `anyhow`, `chrono`, `url`, `regex`, `tracing`, `sha2`, `hex`, `ipnetwork`, `rustc-hash`, `toml`, and `serde_yaml_neo` only if they are directly used in `slapper-core` after inspection.

Do not retain dependencies just because future passes might use them.

### 3. Update `slapper-core` crate docs

Update `crates/slapper-core/src/lib.rs` to say exactly what lives there now:

```rust
//! Slapper Core - dependency-light domain types and shared primitives.
//!
//! This crate contains stable shared types and constants used across the
//! Slapper workspace. It intentionally avoids runtime, UI, network, API,
//! database, packet, browser, and agent dependencies.
//!
//! Keep this crate small. Subsystem-specific behavior belongs in subsystem
//! crates or the main `slapper` engine crate.
```

Keep the note that `SlapperError` and `OutputFormat` remain in the main crate if still true.

### 4. Update main crate docs

Update `crates/slapper/src/lib.rs` crate-level documentation so it no longer reads as though the main crate owns all supporting primitives directly.

Add a short section:

```rust
//! ## Workspace Crates
//!
//! - `slapper-core`: dependency-light shared types and constants.
//! - `slapper-nse`: optional Nmap NSE compatibility support.
//! - `slapper-tui`: terminal UI adapter crate.
//!
//! The main `slapper` crate owns the assessment engine, command dispatch,
//! and feature-gated integrations.
```

If the `tui` module is removed from `slapper`, update the module list accordingly.

### 5. Add compile-time tracking document

Create:

```text
architecture/compile_time_baseline.md
```

If pre-first-pass numbers are not available, state that explicitly.

Use this structure:

```markdown
# Compile Time Baseline

## Context

The first crate-splitting pass had already landed before this file was added, so no pre-first-pass baseline is available.

## Current baseline before second pass

Date:
Rust version:
Cargo version:
Machine:

Commands:

```bash
cargo check -p slapper-core
cargo check -p slapper --no-default-features
cargo check -p slapper-tui
cargo check --workspace --all-targets --no-default-features
cargo check -p slapper --features nse
cargo check -p slapper --features rest-api
cargo check -p slapper --features stress-testing
```

## Post-second-pass measurements

Fill this in after the refactor.
```

Record wall-clock times if available. If a command fails, record whether the failure is new or pre-existing.

## Part 2: Extract `slapper-tui`

### Rationale

The TUI is a good second extraction target because it is an adapter layer with heavy dependencies (`ratatui`, `crossterm`) and should not be on the hot path for scanner/security-engine changes.

The desired dependency direction is:

```text
slapper-core
    ↑
slapper
    ↑
slapper-tui
    ↑
slapper binary / CLI dispatch
```

However, the exact final relationship may depend on current code organization. In the safest version, `slapper-tui` depends on `slapper` and calls public engine/config/types APIs. The main `slapper` library should not depend on `slapper-tui`, to avoid a cycle.

If the current binary is inside the `slapper` package and directly calls `crate::tui`, move only enough entrypoint glue so the binary can call into `slapper_tui`.

### Target crate

Create:

```text
crates/slapper-tui/
  Cargo.toml
  src/
    lib.rs
```

Add it to the root workspace:

```toml
[workspace]
members = [
    "crates/slapper-core",
    "crates/slapper",
    "crates/slapper-nse",
    "crates/slapper-tui",
]
resolver = "2"
```

You may place `slapper-tui` after `slapper` because it depends on `slapper`.

Suggested manifest:

```toml
[package]
name = "slapper-tui"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Terminal UI adapter for Slapper"

[dependencies]
slapper-core = { path = "../slapper-core" }
slapper = { path = "../slapper", default-features = false }

ratatui = "0.30"
crossterm = { version = "0.29", features = ["event-stream"] }

tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
chrono = { workspace = true }
chrono-tz = { workspace = true }
parking_lot = { workspace = true }
futures = "0.3"
async-channel = "2"
```

Do not blindly add all dependencies above. Start with what the moved TUI module actually needs. Keep `slapper-tui` thinner than the main crate.

### Avoid dependency cycles

This is the most important implementation constraint.

Bad:

```text
slapper -> slapper-tui -> slapper
```

Good:

```text
slapper-core -> slapper -> slapper-tui
```

Also acceptable during transition:

```text
slapper-core -> slapper
slapper-core -> slapper-tui
```

if the TUI can be made to depend only on core and small public interfaces. But do not force this in the second pass.

If extracting `slapper-tui` creates a cycle, stop and instead introduce a small interface module in `slapper` that the TUI depends on. Do not make the engine depend on the TUI.

## TUI migration strategy

### 1. Inspect current TUI module

Inspect:

```text
crates/slapper/src/tui/
crates/slapper/src/tui.rs
crates/slapper/src/commands/
crates/slapper/src/main.rs
```

Determine whether TUI is a directory module or single file, and identify its public entrypoints.

Likely entrypoint examples:

```rust
run_tui(...)
start_tui(...)
TuiApp::run(...)
```

Do not rename entrypoints unless necessary.

### 2. Copy/move TUI source into `slapper-tui`

Move:

```text
crates/slapper/src/tui/** -> crates/slapper-tui/src/**
```

If the old module is `crates/slapper/src/tui/mod.rs`, it should become:

```text
crates/slapper-tui/src/lib.rs
```

or:

```text
crates/slapper-tui/src/lib.rs
crates/slapper-tui/src/app.rs
crates/slapper-tui/src/widgets.rs
...
```

Preserve internal module layout as much as possible.

### 3. Update imports inside TUI code

Replace internal `crate::...` imports that refer to engine modules with `slapper::...`.

Examples:

```rust
use crate::config::SlapperConfig;
```

becomes:

```rust
use slapper::config::SlapperConfig;
```

or, where re-exported:

```rust
use slapper::{SlapperConfig, Scope};
```

Replace core types with direct `slapper_core` imports where appropriate:

```rust
use slapper_core::types::Severity;
```

Do not use `slapper::*`.

### 4. Add a compatibility shim only if needed

If many call sites expect `slapper::tui::...`, keep a thin feature-gated compatibility module in the main crate temporarily.

Option A, preferred if call sites can be updated:

Remove or stop exposing `pub mod tui;` from `slapper/src/lib.rs`.

Option B, transitional:

In `slapper/src/lib.rs`:

```rust
#[cfg(feature = "tui")]
pub mod tui {
    pub use slapper_tui::*;
}
```

But this makes `slapper` depend on `slapper-tui`, which can create cycles if `slapper-tui` depends on `slapper`. Avoid this unless you redesign dependencies so `slapper-tui` does not depend on `slapper`.

Because of cycle risk, prefer updating call sites to reference `slapper_tui` directly from the binary/command handler layer.

### 5. Decide where the binary lives

Currently the `slapper` package has:

```toml
[[bin]]
name = "slapper"
path = "src/main.rs"
```

It is acceptable for the `slapper` package binary to depend on `slapper-tui` if Cargo permits the package to have both lib and bin dependencies. The cleaner long-term design is a separate `slapper-cli` crate, but do not force that in this pass unless needed to avoid cycles.

Recommended transition:

1. Keep the binary in `crates/slapper`.
2. Add `slapper-tui` as a dependency only if needed by `src/main.rs` or command handlers.
3. Avoid making the `slapper` library depend on `slapper-tui`.
4. If Cargo package-level dependencies make that impossible without a cycle, stop and instead create a `slapper-cli` crate as a thin binary crate that depends on both `slapper` and `slapper-tui`.

### 6. Fallback: create `slapper-cli` if necessary

If the package-level dependency model makes `slapper` depend on `slapper-tui` and `slapper-tui` depend on `slapper`, create:

```text
crates/slapper-cli/
  Cargo.toml
  src/main.rs
```

Then set the binary target there and eventually remove `[[bin]]` from `crates/slapper/Cargo.toml`.

Target dependency direction:

```text
slapper-core -> slapper
slapper-core -> slapper-tui
slapper -> slapper-tui   # avoid if tui needs engine
slapper -> slapper-cli   # no, binary depends on libraries
slapper-cli -> slapper
slapper-cli -> slapper-tui
```

Actual desired:

```text
slapper-core
  ↑
slapper
  ↑
slapper-tui
  ↑
slapper-cli
```

or:

```text
slapper-core
  ↑       ↑
slapper   slapper-tui
   ↑       ↑
   slapper-cli
```

Use the second form only if TUI can operate through core-level state/events without depending on the full engine.

Do not do a full CLI extraction unless cycle avoidance requires it.

## Cargo feature cleanup

### 1. Add a `tui` feature if absent

In `crates/slapper/Cargo.toml`, add or retain a feature that controls TUI availability:

```toml
[features]
default = []
tui = []
```

If the TUI lives entirely in `slapper-tui`, then the feature may belong to the binary/CLI package rather than the engine crate. Use the minimal feature design that preserves existing commands.

### 2. Remove direct TUI dependencies from the main library path

If possible, remove from `crates/slapper/Cargo.toml`:

```toml
ratatui = "0.30"
crossterm = { version = "0.29", features = ["event-stream"] }
arboard = "3.4"
```

only if they are used exclusively by the TUI.

If these crates are used outside TUI, leave them temporarily and document why.

### 3. Avoid feature leakage

Do not make scanner/security-engine code conditional on the TUI feature.

Do not let `slapper-core` know about TUI features.

## Build and test commands

Run after core cleanup:

```bash
cargo fmt
cargo check -p slapper-core
cargo test -p slapper-core
cargo check -p slapper --no-default-features
```

Run after adding `slapper-tui` but before moving all call sites:

```bash
cargo check -p slapper-tui
cargo check -p slapper --no-default-features
```

Run final checks:

```bash
cargo fmt
cargo check -p slapper-core
cargo check -p slapper-tui
cargo check -p slapper --no-default-features
cargo check --workspace --all-targets --no-default-features
cargo test -p slapper-core
cargo test -p slapper --lib --no-default-features
cargo test --workspace --no-default-features
```

Then run important feature checks:

```bash
cargo check -p slapper --features nse
cargo check -p slapper --features rest-api
cargo check -p slapper --features stress-testing
```

If the binary supports TUI behind a feature or via a CLI package, run the appropriate binary check, for example:

```bash
cargo check -p slapper --bin slapper
```

or:

```bash
cargo check -p slapper-cli
```

Record all results in `architecture/compile_time_baseline.md`.

## Documentation updates

Update:

```text
architecture/overview.md
crates/slapper/src/lib.rs
crates/slapper-core/src/lib.rs
README.md if it has crate layout or TUI dependency notes
```

In `architecture/overview.md`, update the crate layout section to include:

```markdown
- `slapper-tui`: terminal UI adapter built on `ratatui`/`crossterm`. Depends on Slapper engine APIs but should not be required for engine-only builds.
```

If the TUI module index still points to `crates/slapper/src/tui/`, update it to `crates/slapper-tui/src/`.

If a compatibility shim remains, document it as transitional.

## Final handoff report

When done, report:

1. Whether `slapper-core` dependencies were pruned.
2. Final `slapper-core` dependency list.
3. Whether `slapper-tui` was created.
4. Whether `ratatui`/`crossterm` moved out of the main crate dependency set.
5. Whether a dependency cycle was encountered.
6. Whether `slapper-cli` was required or avoided.
7. Which TUI entrypoints were preserved.
8. Which imports were updated.
9. Which compatibility shims remain.
10. Which commands were run and their results.
11. Any pre-existing failures.
12. Compile-time notes from `architecture/compile_time_baseline.md`.

## Stop conditions

Stop and report instead of forcing the refactor if:

1. Extracting TUI creates a dependency cycle that cannot be resolved without a broader CLI extraction.
2. TUI code depends deeply on private main-crate internals that would require large public API changes.
3. Moving TUI requires changing scanner/security behavior.
4. Feature combinations break in ways unrelated to import/path changes.
5. The diff becomes too broad to review safely.

If one of these occurs, complete only the core cleanup and compile-time documentation, then recommend a smaller interface-extraction pass before retrying TUI extraction.

## Suggested next pass after this one

If this pass succeeds, the next likely extraction candidates are:

```text
slapper-output
slapper-api / slapper-agent-api
slapper-scan
```

Choose based on timing data and edit frequency.

If engine work is the main hot path, extract `slapper-scan` next.

If adapter dependencies dominate compile time, extract API/gRPC/MCP next.

If report generation pulls awkward dependencies or causes churn, extract `slapper-output` next.

