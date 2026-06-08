# Slapper Modularization: Third Pass Handoff Plan

## Purpose

The previous pass successfully moved the binary entrypoint into `slapper-cli` and extracted the terminal UI into `slapper-tui`. The workspace now has a healthier dependency direction:

```text
slapper-core
  └── slapper
        └── slapper-tui
              └── slapper-cli

slapper-nse remains optional compatibility support.
```

This third pass should finish the TUI/CLI extraction cleanup, verify that the main engine crate no longer pulls terminal UI dependencies unnecessarily, and then prepare/extract the next low-risk adapter boundary.

The recommended next extraction target is `slapper-output`, because report/output formatting is an infrastructure/adapter concern and should be easier to isolate than scanner/recon/WAF internals. Do not extract scanner/security-engine modules in this pass.

## Current known state

The workspace currently includes:

```text
crates/slapper-core
crates/slapper
crates/slapper-nse
crates/slapper-tui
crates/slapper-cli
```

`slapper-core` has been cleaned up and now has a small dependency set.

`slapper-tui` exists and owns the terminal UI module tree.

`slapper-cli` exists and owns the binary entrypoint. It parses `slapper::cli::Cli`, initializes logging, launches `slapper_tui::run(...)` when appropriate, and otherwise dispatches to `slapper::commands::handle_command(...)`.

The remaining issue from the previous pass is that `crates/slapper/Cargo.toml` still appears to list terminal UI dependencies such as:

```toml
ratatui = "0.30"
crossterm = { version = "0.29", features = ["event-stream"] }
arboard = "3.4"
```

If these are no longer used by source files in `crates/slapper`, they should be removed from the engine crate.

## Non-goals

Do not change scanner, WAF, fuzzer, recon, packet, stress, NSE, or agent behavior.

Do not redesign the CLI or TUI.

Do not alter command names, output formats, config file formats, report schemas, or serialized data structures unless necessary to move output code without behavior changes.

Do not extract `slapper-api`, `slapper-agent`, `slapper-scan`, or `slapper-web` in this pass.

Do not move `SlapperError` or config/scope wholesale into `slapper-core`.

Do not introduce a broad prelude or wildcard re-export.

Do not merge crates back together.

## Success criteria

After this pass:

1. `crates/slapper` no longer depends on terminal UI crates unless there is a documented non-TUI use.
2. `cargo check -p slapper --no-default-features` does not compile terminal UI implementation code.
3. `cargo check -p slapper-cli` still builds the binary entrypoint.
4. `cargo check -p slapper-tui` still builds the terminal UI adapter crate.
5. A new `crates/slapper-output` crate exists if extraction is feasible.
6. Output/reporting code is moved into `slapper-output` or, if blocked, blockers are documented precisely.
7. The main `slapper` crate depends on `slapper-output` only if needed for engine command handlers.
8. Output behavior is preserved.
9. Architecture docs reflect the new crate boundaries.
10. Compile-time tracking is updated with before/after results.

## Part 1: Finish TUI/CLI extraction cleanup

### 1. Search for remaining terminal UI dependency use in `slapper`

Search within `crates/slapper/src` for:

```text
ratatui
crossterm
arboard
Terminal
CrosstermBackend
Clipboard
```

If no actual source usage remains outside `crates/slapper-tui`, remove these dependencies from `crates/slapper/Cargo.toml`:

```toml
ratatui = "0.30"
crossterm = { version = "0.29", features = ["event-stream"] }
arboard = "3.4"
```

Keep them in `crates/slapper-tui/Cargo.toml`.

If any source usage remains in `slapper`, decide whether it belongs in `slapper-tui` or should remain as non-TUI functionality. If it remains, document the reason in the final handoff report.

### 2. Verify no legacy binary target remains in `slapper`

Confirm that `crates/slapper/src/main.rs` is gone.

Confirm `crates/slapper/Cargo.toml` does not contain:

```toml
[[bin]]
name = "slapper"
path = "src/main.rs"
```

The binary should be owned by `slapper-cli`.

If packaging requires the binary name to still be `slapper`, configure that in `crates/slapper-cli/Cargo.toml`:

```toml
[[bin]]
name = "slapper"
path = "src/main.rs"
```

Do not leave the binary named `slapper-cli` unless that is intentional.

### 3. Verify dependency direction

The intended graph is:

```text
slapper-core -> slapper -> slapper-tui -> slapper-cli
```

In Cargo terms:

```text
slapper depends on slapper-core
slapper-tui depends on slapper and slapper-core
slapper-cli depends on slapper and slapper-tui
```

`slapper` must not depend on `slapper-tui`.

`slapper-core` must not depend on `slapper`, `slapper-tui`, or `slapper-cli`.

### 4. Check feature forwarding

Review feature forwarding in `slapper-tui` and `slapper-cli`.

The base commands should work with:

```bash
cargo check -p slapper
cargo check -p slapper-cli
cargo check -p slapper-tui
```

Feature forwarding should be deliberate and minimal. If `slapper-tui` forwards features for optional tabs, keep that acceptable for now, but ensure base TUI does not require `full`.

For any feature that exists in `slapper-cli` but does not need to be forwarded to `slapper-tui`, remove the unnecessary TUI forwarding.

## Part 2: Add compile-time tracking if missing

Create or update:

```text
architecture/compile_time_baseline.md
```

If earlier baseline numbers are missing, state that plainly.

Use this structure:

```markdown
# Compile Time Tracking

## Baseline availability

Pre-first-pass timing data:
Pre-second-pass timing data:
Current baseline before third pass:

## Current workspace shape

- slapper-core
- slapper
- slapper-nse
- slapper-tui
- slapper-cli

## Third pass commands

```bash
rustc --version
cargo --version
cargo check -p slapper-core
cargo check -p slapper --no-default-features
cargo check -p slapper-tui
cargo check -p slapper-cli
cargo check --workspace --all-targets --no-default-features
cargo check -p slapper-cli --features nse
cargo check -p slapper-cli --features rest-api
cargo check -p slapper-cli --features stress-testing
```

## Results before third pass

## Results after third pass

## Notes
```

Record whether commands pass/fail. If wall-clock times are available, include them. If a failure is pre-existing, mark it as such.

## Part 3: Extract `slapper-output`

### Rationale

Output/report generation is a better next extraction target than scanner internals because it is an adapter/infrastructure concern. It should depend on domain result types, not own scanning behavior.

This extraction should reduce main-crate coupling around formatting/reporting and prepare for later CLI/API separation.

### Target dependency direction

Preferred:

```text
slapper-core
    ↑
slapper-output
    ↑
slapper
    ↑
slapper-cli
```

However, this may not be immediately possible if output currently depends heavily on `slapper` engine types.

Acceptable transitional direction:

```text
slapper-core
    ↑
slapper
    ↑
slapper-output
    ↑
slapper-cli
```

But avoid making `slapper-output` depend on `slapper` unless unavoidable. The cleaner design is for output to depend on shared result/finding types, not the full engine.

If output code requires many engine-private types, stop and document blockers rather than forcing a bad crate graph.

### Create the crate

Add:

```text
crates/slapper-output/
  Cargo.toml
  src/
    lib.rs
```

Update root `Cargo.toml` workspace members:

```toml
members = [
    "crates/slapper-core",
    "crates/slapper",
    "crates/slapper-nse",
    "crates/slapper-tui",
    "crates/slapper-cli",
    "crates/slapper-output",
]
```

Suggested `crates/slapper-output/Cargo.toml`:

```toml
[package]
name = "slapper-output"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Report formatting and output adapters for Slapper"

[dependencies]
slapper-core = { path = "../slapper-core" }

serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
chrono = { workspace = true }
tracing = { workspace = true }

# Add only if the moved output code uses them.
handlebars = "6"
quick-xml = { version = "0.31", features = ["serialize"] }
```

If the existing output module needs `printpdf`, keep PDF support feature-gated:

```toml
printpdf = { version = "0.7", optional = true }

[features]
default = []
pdf = ["dep:printpdf"]
```

If SARIF/JUnit/Markdown/HTML support needs additional crates, add them only if actually used.

### Identify output module contents

Inspect:

```text
crates/slapper/src/output/
crates/slapper/src/output.rs
crates/slapper/src/types.rs
crates/slapper/src/findings/
crates/slapper/src/commands/
```

Determine what belongs in `slapper-output`:

Good candidates:

```text
formatters
report serializers
HTML/Markdown/CSV/SARIF/JUnit renderers
output writer helpers
report templates
```

Questionable candidates:

```text
Command handler code
scanner execution logic
config loading
storage queries
finding workflow mutation
```

Do not move command handlers into `slapper-output`. Command handlers should call output APIs.

### Move output code

Move the output module into:

```text
crates/slapper-output/src/
```

Possible mapping:

```text
crates/slapper/src/output/mod.rs       -> crates/slapper-output/src/lib.rs
crates/slapper/src/output/*.rs         -> crates/slapper-output/src/*.rs
crates/slapper/src/output/templates/*  -> crates/slapper-output/src/templates/*
```

Preserve module names where possible.

### Adjust imports

Inside moved output code:

Replace:

```rust
use crate::types::Severity;
```

with:

```rust
use slapper_core::types::Severity;
```

If it needs `OutputFormat`, decide whether `OutputFormat` should move to `slapper-output`.

Recommended:

Move `OutputFormat` out of `crates/slapper/src/types.rs` only if it is no longer needed by `clap` derive in the engine crate. If CLI parsing depends on `clap::ValueEnum`, avoid moving `OutputFormat` into `slapper-output` unless you are comfortable adding `clap` to `slapper-output`.

Alternative:

Keep CLI-facing `OutputFormat` in `slapper` for now, and define output APIs that accept either string/enum-like internal format types or use explicit functions per format.

Do not add `clap` to `slapper-output` unless necessary.

### Finding/result type problem

The likely blocker is that output code depends on `slapper::findings` or scan result types that still live in the main crate.

If output code depends only on `Severity`, `SensitiveString`, and simple serializable models, move those models into `slapper-core`.

If output code depends on complex engine types, use one of these strategies:

#### Strategy A: Move pure output-facing models into `slapper-core`

Move only pure serializable structs/enums that represent findings/results and have no runtime dependencies.

Example:

```text
slapper-core/src/findings.rs
slapper-core/src/results.rs
```

Do not move stores, databases, workflows, or mutation engines.

#### Strategy B: Keep output crate dependent on `slapper` temporarily

This is acceptable only if it avoids a cycle.

Bad:

```text
slapper -> slapper-output -> slapper
```

If the main `slapper` crate needs to call `slapper-output`, then `slapper-output` cannot also depend on `slapper`.

If this cycle appears, stop and use Strategy A or defer extraction.

#### Strategy C: Leave output extraction incomplete

If the result/finding model is too tangled, stop after documenting the required model split. Do not create a cyclic or awkward dependency graph.

## Integration back into `slapper`

If extraction succeeds, add to `crates/slapper/Cargo.toml`:

```toml
slapper-output = { path = "../slapper-output" }
```

Update `crates/slapper/src/lib.rs`.

Preferred:

```rust
pub use slapper_output as output;
```

or, if keeping compatibility module:

```rust
pub mod output {
    pub use slapper_output::*;
}
```

Use whichever minimizes import churn and avoids name conflicts.

Remove old output source files only after compilation passes.

If `pdf` feature moved to `slapper-output`, update feature forwarding in `crates/slapper/Cargo.toml`:

```toml
pdf = ["slapper-output/pdf"]
```

If `printpdf` is no longer used directly by `slapper`, remove the direct `printpdf` dependency from `slapper`.

## Build/test commands

Run after TUI dependency cleanup:

```bash
cargo fmt
cargo check -p slapper-core
cargo check -p slapper --no-default-features
cargo check -p slapper-tui
cargo check -p slapper-cli
```

Run after creating `slapper-output`:

```bash
cargo check -p slapper-output
cargo check -p slapper --no-default-features
cargo check -p slapper-cli
```

Run broader final checks:

```bash
cargo fmt
cargo check --workspace --all-targets --no-default-features
cargo test -p slapper-core
cargo test -p slapper-output
cargo test -p slapper --lib --no-default-features
cargo test --workspace --no-default-features
```

Run feature checks through the CLI crate where user-facing binary features are forwarded:

```bash
cargo check -p slapper-cli --features nse
cargo check -p slapper-cli --features rest-api
cargo check -p slapper-cli --features stress-testing
cargo check -p slapper-cli --features pdf
```

Also check engine features directly if relevant:

```bash
cargo check -p slapper --features nse
cargo check -p slapper --features rest-api
cargo check -p slapper --features stress-testing
cargo check -p slapper --features pdf
```

Record results in `architecture/compile_time_baseline.md`.

## Documentation updates

Update:

```text
architecture/overview.md
crates/slapper/src/lib.rs
README.md if crate layout is mentioned
```

In `architecture/overview.md`, update crate layout:

```markdown
- `slapper-output`: report formatting and output adapters for JSON, CSV, HTML, SARIF, JUnit, Markdown, and optional PDF output.
```

Update module index paths from:

```text
crates/slapper/src/output/
```

to:

```text
crates/slapper-output/src/
```

If TUI paths still point to `crates/slapper/src/tui/`, update them to `crates/slapper-tui/src/`.

If CLI docs assume the binary lives in `crates/slapper/src/main.rs`, update them to `crates/slapper-cli/src/main.rs`.

## Final handoff report

When complete, report:

1. Whether stale TUI deps were removed from `slapper`.
2. Whether `slapper-cli` binary name is still `slapper`.
3. Final dependency graph among `slapper-core`, `slapper`, `slapper-tui`, `slapper-cli`, `slapper-output`.
4. Whether `slapper-output` was created.
5. Which output modules moved.
6. Which output modules stayed and why.
7. Whether any finding/result models had to move into `slapper-core`.
8. Whether `printpdf` moved behind `slapper-output/pdf`.
9. Which commands were run and results.
10. Any pre-existing failures.
11. Compile-time observations.
12. Recommended next extraction target.

## Stop conditions

Stop and report instead of forcing the extraction if:

1. `slapper-output` would need to depend on `slapper` while `slapper` also depends on `slapper-output`.
2. Output code is too entangled with command execution or scanner internals.
3. Moving output requires changing report schemas.
4. Moving output requires adding `clap`, `ratatui`, `crossterm`, `tokio`, `reqwest`, `sqlx`, or API/server dependencies to `slapper-output` without a clear reason.
5. The diff becomes too large to review.

If blocked, complete only the TUI dependency cleanup and compile-time documentation, then write a smaller follow-up plan for separating finding/result data models into `slapper-core`.

## Suggested next pass after this one

If `slapper-output` succeeds, the next likely candidates are:

```text
slapper-api or slapper-agent-api
slapper-scan
slapper-web
```

Recommended order:

1. `slapper-api` / `slapper-agent-api` if adapter dependencies dominate.
2. `slapper-scan` if scanner/probe/recon is the main hot edit path.
3. `slapper-web` if WAF/fuzzer/loadtest HTTP logic is the most active area.

Do not start those in this pass.

