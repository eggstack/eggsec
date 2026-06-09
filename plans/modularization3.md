# Eggsec Modularization: Third Pass Handoff Plan

## Purpose

The previous pass successfully moved the binary entrypoint into `eggsec-cli` and extracted the terminal UI into `eggsec-tui`. The workspace now has a healthier dependency direction:

```text
eggsec-core
  └── eggsec
        └── eggsec-tui
              └── eggsec-cli

eggsec-nse remains optional compatibility support.
```

This third pass should finish the TUI/CLI extraction cleanup, verify that the main engine crate no longer pulls terminal UI dependencies unnecessarily, and then prepare/extract the next low-risk adapter boundary.

The recommended next extraction target is `eggsec-output`, because report/output formatting is an infrastructure/adapter concern and should be easier to isolate than scanner/recon/WAF internals. Do not extract scanner/security-engine modules in this pass.

## Current known state

The workspace currently includes:

```text
crates/eggsec-core
crates/eggsec
crates/eggsec-nse
crates/eggsec-tui
crates/eggsec-cli
```

`eggsec-core` has been cleaned up and now has a small dependency set.

`eggsec-tui` exists and owns the terminal UI module tree.

`eggsec-cli` exists and owns the binary entrypoint. It parses `eggsec::cli::Cli`, initializes logging, launches `eggsec_tui::run(...)` when appropriate, and otherwise dispatches to `eggsec::commands::handle_command(...)`.

The remaining issue from the previous pass is that `crates/eggsec/Cargo.toml` still appears to list terminal UI dependencies such as:

```toml
ratatui = "0.30"
crossterm = { version = "0.29", features = ["event-stream"] }
arboard = "3.4"
```

If these are no longer used by source files in `crates/eggsec`, they should be removed from the engine crate.

## Non-goals

Do not change scanner, WAF, fuzzer, recon, packet, stress, NSE, or agent behavior.

Do not redesign the CLI or TUI.

Do not alter command names, output formats, config file formats, report schemas, or serialized data structures unless necessary to move output code without behavior changes.

Do not extract `eggsec-api`, `eggsec-agent`, `eggsec-scan`, or `eggsec-web` in this pass.

Do not move `EggsecError` or config/scope wholesale into `eggsec-core`.

Do not introduce a broad prelude or wildcard re-export.

Do not merge crates back together.

## Success criteria

After this pass:

1. `crates/eggsec` no longer depends on terminal UI crates unless there is a documented non-TUI use.
2. `cargo check -p eggsec --no-default-features` does not compile terminal UI implementation code.
3. `cargo check -p eggsec-cli` still builds the binary entrypoint.
4. `cargo check -p eggsec-tui` still builds the terminal UI adapter crate.
5. A new `crates/eggsec-output` crate exists if extraction is feasible.
6. Output/reporting code is moved into `eggsec-output` or, if blocked, blockers are documented precisely.
7. The main `eggsec` crate depends on `eggsec-output` only if needed for engine command handlers.
8. Output behavior is preserved.
9. Architecture docs reflect the new crate boundaries.
10. Compile-time tracking is updated with before/after results.

## Part 1: Finish TUI/CLI extraction cleanup

### 1. Search for remaining terminal UI dependency use in `eggsec`

Search within `crates/eggsec/src` for:

```text
ratatui
crossterm
arboard
Terminal
CrosstermBackend
Clipboard
```

If no actual source usage remains outside `crates/eggsec-tui`, remove these dependencies from `crates/eggsec/Cargo.toml`:

```toml
ratatui = "0.30"
crossterm = { version = "0.29", features = ["event-stream"] }
arboard = "3.4"
```

Keep them in `crates/eggsec-tui/Cargo.toml`.

If any source usage remains in `eggsec`, decide whether it belongs in `eggsec-tui` or should remain as non-TUI functionality. If it remains, document the reason in the final handoff report.

### 2. Verify no legacy binary target remains in `eggsec`

Confirm that `crates/eggsec/src/main.rs` is gone.

Confirm `crates/eggsec/Cargo.toml` does not contain:

```toml
[[bin]]
name = "eggsec"
path = "src/main.rs"
```

The binary should be owned by `eggsec-cli`.

If packaging requires the binary name to still be `eggsec`, configure that in `crates/eggsec-cli/Cargo.toml`:

```toml
[[bin]]
name = "eggsec"
path = "src/main.rs"
```

Do not leave the binary named `eggsec-cli` unless that is intentional.

### 3. Verify dependency direction

The intended graph is:

```text
eggsec-core -> eggsec -> eggsec-tui -> eggsec-cli
```

In Cargo terms:

```text
eggsec depends on eggsec-core
eggsec-tui depends on eggsec and eggsec-core
eggsec-cli depends on eggsec and eggsec-tui
```

`eggsec` must not depend on `eggsec-tui`.

`eggsec-core` must not depend on `eggsec`, `eggsec-tui`, or `eggsec-cli`.

### 4. Check feature forwarding

Review feature forwarding in `eggsec-tui` and `eggsec-cli`.

The base commands should work with:

```bash
cargo check -p eggsec
cargo check -p eggsec-cli
cargo check -p eggsec-tui
```

Feature forwarding should be deliberate and minimal. If `eggsec-tui` forwards features for optional tabs, keep that acceptable for now, but ensure base TUI does not require `full`.

For any feature that exists in `eggsec-cli` but does not need to be forwarded to `eggsec-tui`, remove the unnecessary TUI forwarding.

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

- eggsec-core
- eggsec
- eggsec-nse
- eggsec-tui
- eggsec-cli

## Third pass commands

```bash
rustc --version
cargo --version
cargo check -p eggsec-core
cargo check -p eggsec --no-default-features
cargo check -p eggsec-tui
cargo check -p eggsec-cli
cargo check --workspace --all-targets --no-default-features
cargo check -p eggsec-cli --features nse
cargo check -p eggsec-cli --features rest-api
cargo check -p eggsec-cli --features stress-testing
```

## Results before third pass

## Results after third pass

## Notes
```

Record whether commands pass/fail. If wall-clock times are available, include them. If a failure is pre-existing, mark it as such.

## Part 3: Extract `eggsec-output`

### Rationale

Output/report generation is a better next extraction target than scanner internals because it is an adapter/infrastructure concern. It should depend on domain result types, not own scanning behavior.

This extraction should reduce main-crate coupling around formatting/reporting and prepare for later CLI/API separation.

### Target dependency direction

Preferred:

```text
eggsec-core
    ↑
eggsec-output
    ↑
eggsec
    ↑
eggsec-cli
```

However, this may not be immediately possible if output currently depends heavily on `eggsec` engine types.

Acceptable transitional direction:

```text
eggsec-core
    ↑
eggsec
    ↑
eggsec-output
    ↑
eggsec-cli
```

But avoid making `eggsec-output` depend on `eggsec` unless unavoidable. The cleaner design is for output to depend on shared result/finding types, not the full engine.

If output code requires many engine-private types, stop and document blockers rather than forcing a bad crate graph.

### Create the crate

Add:

```text
crates/eggsec-output/
  Cargo.toml
  src/
    lib.rs
```

Update root `Cargo.toml` workspace members:

```toml
members = [
    "crates/eggsec-core",
    "crates/eggsec",
    "crates/eggsec-nse",
    "crates/eggsec-tui",
    "crates/eggsec-cli",
    "crates/eggsec-output",
]
```

Suggested `crates/eggsec-output/Cargo.toml`:

```toml
[package]
name = "eggsec-output"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Report formatting and output adapters for Eggsec"

[dependencies]
eggsec-core = { path = "../eggsec-core" }

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
crates/eggsec/src/output/
crates/eggsec/src/output.rs
crates/eggsec/src/types.rs
crates/eggsec/src/findings/
crates/eggsec/src/commands/
```

Determine what belongs in `eggsec-output`:

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

Do not move command handlers into `eggsec-output`. Command handlers should call output APIs.

### Move output code

Move the output module into:

```text
crates/eggsec-output/src/
```

Possible mapping:

```text
crates/eggsec/src/output/mod.rs       -> crates/eggsec-output/src/lib.rs
crates/eggsec/src/output/*.rs         -> crates/eggsec-output/src/*.rs
crates/eggsec/src/output/templates/*  -> crates/eggsec-output/src/templates/*
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
use eggsec_core::types::Severity;
```

If it needs `OutputFormat`, decide whether `OutputFormat` should move to `eggsec-output`.

Recommended:

Move `OutputFormat` out of `crates/eggsec/src/types.rs` only if it is no longer needed by `clap` derive in the engine crate. If CLI parsing depends on `clap::ValueEnum`, avoid moving `OutputFormat` into `eggsec-output` unless you are comfortable adding `clap` to `eggsec-output`.

Alternative:

Keep CLI-facing `OutputFormat` in `eggsec` for now, and define output APIs that accept either string/enum-like internal format types or use explicit functions per format.

Do not add `clap` to `eggsec-output` unless necessary.

### Finding/result type problem

The likely blocker is that output code depends on `eggsec::findings` or scan result types that still live in the main crate.

If output code depends only on `Severity`, `SensitiveString`, and simple serializable models, move those models into `eggsec-core`.

If output code depends on complex engine types, use one of these strategies:

#### Strategy A: Move pure output-facing models into `eggsec-core`

Move only pure serializable structs/enums that represent findings/results and have no runtime dependencies.

Example:

```text
eggsec-core/src/findings.rs
eggsec-core/src/results.rs
```

Do not move stores, databases, workflows, or mutation engines.

#### Strategy B: Keep output crate dependent on `eggsec` temporarily

This is acceptable only if it avoids a cycle.

Bad:

```text
eggsec -> eggsec-output -> eggsec
```

If the main `eggsec` crate needs to call `eggsec-output`, then `eggsec-output` cannot also depend on `eggsec`.

If this cycle appears, stop and use Strategy A or defer extraction.

#### Strategy C: Leave output extraction incomplete

If the result/finding model is too tangled, stop after documenting the required model split. Do not create a cyclic or awkward dependency graph.

## Integration back into `eggsec`

If extraction succeeds, add to `crates/eggsec/Cargo.toml`:

```toml
eggsec-output = { path = "../eggsec-output" }
```

Update `crates/eggsec/src/lib.rs`.

Preferred:

```rust
pub use eggsec_output as output;
```

or, if keeping compatibility module:

```rust
pub mod output {
    pub use eggsec_output::*;
}
```

Use whichever minimizes import churn and avoids name conflicts.

Remove old output source files only after compilation passes.

If `pdf` feature moved to `eggsec-output`, update feature forwarding in `crates/eggsec/Cargo.toml`:

```toml
pdf = ["eggsec-output/pdf"]
```

If `printpdf` is no longer used directly by `eggsec`, remove the direct `printpdf` dependency from `eggsec`.

## Build/test commands

Run after TUI dependency cleanup:

```bash
cargo fmt
cargo check -p eggsec-core
cargo check -p eggsec --no-default-features
cargo check -p eggsec-tui
cargo check -p eggsec-cli
```

Run after creating `eggsec-output`:

```bash
cargo check -p eggsec-output
cargo check -p eggsec --no-default-features
cargo check -p eggsec-cli
```

Run broader final checks:

```bash
cargo fmt
cargo check --workspace --all-targets --no-default-features
cargo test -p eggsec-core
cargo test -p eggsec-output
cargo test -p eggsec --lib --no-default-features
cargo test --workspace --no-default-features
```

Run feature checks through the CLI crate where user-facing binary features are forwarded:

```bash
cargo check -p eggsec-cli --features nse
cargo check -p eggsec-cli --features rest-api
cargo check -p eggsec-cli --features stress-testing
cargo check -p eggsec-cli --features pdf
```

Also check engine features directly if relevant:

```bash
cargo check -p eggsec --features nse
cargo check -p eggsec --features rest-api
cargo check -p eggsec --features stress-testing
cargo check -p eggsec --features pdf
```

Record results in `architecture/compile_time_baseline.md`.

## Documentation updates

Update:

```text
architecture/overview.md
crates/eggsec/src/lib.rs
README.md if crate layout is mentioned
```

In `architecture/overview.md`, update crate layout:

```markdown
- `eggsec-output`: report formatting and output adapters for JSON, CSV, HTML, SARIF, JUnit, Markdown, and optional PDF output.
```

Update module index paths from:

```text
crates/eggsec/src/output/
```

to:

```text
crates/eggsec-output/src/
```

If TUI paths still point to `crates/eggsec/src/tui/`, update them to `crates/eggsec-tui/src/`.

If CLI docs assume the binary lives in `crates/eggsec/src/main.rs`, update them to `crates/eggsec-cli/src/main.rs`.

## Final handoff report

When complete, report:

1. Whether stale TUI deps were removed from `eggsec`.
2. Whether `eggsec-cli` binary name is still `eggsec`.
3. Final dependency graph among `eggsec-core`, `eggsec`, `eggsec-tui`, `eggsec-cli`, `eggsec-output`.
4. Whether `eggsec-output` was created.
5. Which output modules moved.
6. Which output modules stayed and why.
7. Whether any finding/result models had to move into `eggsec-core`.
8. Whether `printpdf` moved behind `eggsec-output/pdf`.
9. Which commands were run and results.
10. Any pre-existing failures.
11. Compile-time observations.
12. Recommended next extraction target.

## Stop conditions

Stop and report instead of forcing the extraction if:

1. `eggsec-output` would need to depend on `eggsec` while `eggsec` also depends on `eggsec-output`.
2. Output code is too entangled with command execution or scanner internals.
3. Moving output requires changing report schemas.
4. Moving output requires adding `clap`, `ratatui`, `crossterm`, `tokio`, `reqwest`, `sqlx`, or API/server dependencies to `eggsec-output` without a clear reason.
5. The diff becomes too large to review.

If blocked, complete only the TUI dependency cleanup and compile-time documentation, then write a smaller follow-up plan for separating finding/result data models into `eggsec-core`.

## Suggested next pass after this one

If `eggsec-output` succeeds, the next likely candidates are:

```text
eggsec-api or eggsec-agent-api
eggsec-scan
eggsec-web
```

Recommended order:

1. `eggsec-api` / `eggsec-agent-api` if adapter dependencies dominate.
2. `eggsec-scan` if scanner/probe/recon is the main hot edit path.
3. `eggsec-web` if WAF/fuzzer/loadtest HTTP logic is the most active area.

Do not start those in this pass.

