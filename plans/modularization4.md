# Slapper Modularization: Fourth Pass Cleanup and API-Extraction Prep Plan

## Purpose

The previous pass successfully removed stale terminal UI dependencies from the main `slapper` engine crate, extracted `slapper-output`, and added `slapper-tool-core` for core tool DTOs/data structures. The workspace is now meaningfully modular, but there is some documentation drift and a few dependency/feature-boundary questions that should be cleaned up before another large extraction.

This fourth pass should be primarily a stabilization and preparation pass:

1. Correct workspace/crate documentation to reflect the current architecture.
2. Audit `slapper-output` and `slapper-tool-core` dependencies.
3. Tighten feature forwarding between `slapper`, `slapper-tui`, and `slapper-cli`.
4. Verify no stale paths or compatibility shims are misleading future work.
5. Prepare, but do not fully execute unless straightforward, the next adapter extraction: `slapper-api` / `slapper-agent-api`.

The goal is to make the next major split safer, not to churn engine internals.

## Current known workspace shape

The workspace currently includes:

```text
crates/
  slapper-core/       # Dependency-light shared types/constants/primitives
  slapper-tool-core/  # Core data types for tool abstraction layer
  slapper-output/     # Portable output/report formatting modules
  slapper/            # Main assessment engine library; no binary
  slapper-nse/        # Optional NSE compatibility runtime
  slapper-tui/        # Terminal UI adapter
  slapper-cli/        # Binary package, binary named `slapper`
```

The approximate current dependency shape is:

```text
slapper-core
  ├── slapper-tool-core
  ├── slapper-output
  └── slapper
        ├── slapper-output
        ├── slapper-tool-core
        └── slapper-nse optional

slapper-tui -> slapper + slapper-core
slapper-cli -> slapper + slapper-tui
```

`slapper-output` intentionally does not depend on `slapper`. Engine-coupled output modules remain under `crates/slapper/src/output/`.

## Non-goals

Do not extract scanner, WAF, fuzzer, recon, loadtest, packet, stress, or NSE internals.

Do not redesign the CLI, TUI, command handlers, or report schemas.

Do not merge crates.

Do not move `SlapperError` wholesale into `slapper-core`.

Do not move config/scope wholesale into `slapper-core`.

Do not attempt a broad API/MCP extraction unless the interface is already clean enough and the diff remains small.

Do not introduce wildcard re-exports or a large prelude.

Do not remove compatibility re-exports unless all call sites are updated and tests/checks remain clean.

## Success criteria

After this pass:

1. `architecture/overview.md` accurately lists all workspace crates.
2. `crates/slapper/src/lib.rs` accurately describes current crate boundaries.
3. `architecture/compile_time_baseline.md` has corrected crate notes and no stale module names.
4. `slapper-output` dependencies are justified or pruned.
5. `slapper-tool-core` dependencies are justified or pruned.
6. Feature forwarding in `slapper-cli` and `slapper-tui` is reviewed and documented.
7. `cargo check -p slapper --no-default-features` still passes.
8. `cargo check -p slapper-output` still passes.
9. `cargo check -p slapper-tool-core` still passes.
10. `cargo check -p slapper-cli` still passes.
11. API/agent extraction blockers and candidate module boundaries are documented.
12. If a small `slapper-api` shell crate is added, it must not create cycles and must compile.

## Part 1: Documentation correction

### 1. Update `crates/slapper/src/lib.rs` top-level docs

The current docs mention only `slapper-core`, `slapper-nse`, and `slapper-tui`. Update the “Workspace Crates” section to include all current crates.

Suggested wording:

```rust
//! ## Workspace Crates
//!
//! - `slapper-core`: dependency-light shared types and constants.
//! - `slapper-tool-core`: protocol-neutral tool request/response/error/history types.
//! - `slapper-output`: report formatting and output adapters.
//! - `slapper-nse`: optional Nmap NSE compatibility support.
//! - `slapper-tui`: terminal UI adapter crate.
//! - `slapper-cli`: user-facing binary package; binary name is `slapper`.
//!
//! The main `slapper` crate owns the assessment engine, command dispatch,
//! scope/config loading, and feature-gated integrations.
```

Also adjust the architecture module list item for output. It should not imply the main crate fully owns output anymore.

Replace something like:

```rust
//! - **`output`** - Multiple report formats (JSON, HTML, CSV, SARIF, JUnit)
```

with:

```rust
//! - **`output`** - Compatibility facade over `slapper-output` plus engine-coupled report modules
```

If `tool` now partly depends on `slapper-tool-core`, adjust that line as well:

```rust
//! - **`tool`** - Tool registry/execution framework; core DTOs live in `slapper-tool-core`
```

### 2. Update `architecture/overview.md`

Update the crate layout section so it lists:

```markdown
- `slapper-core`
- `slapper-tool-core`
- `slapper-output`
- `slapper`
- `slapper-nse`
- `slapper-tui`
- `slapper-cli`
```

Make sure paths in the module index are current:

```text
TUI paths should point to crates/slapper-tui/src/
Output portable modules should point to crates/slapper-output/src/
Engine-coupled output modules may remain at crates/slapper/src/output/
Tool core DTOs should point to crates/slapper-tool-core/src/
CLI binary should point to crates/slapper-cli/src/main.rs
```

Do not over-document future crates that do not exist yet.

### 3. Fix `architecture/compile_time_baseline.md`

Correct any stale or inaccurate notes.

Known issue to check: the third-pass notes may mention an output module named `template`, while the actual engine-coupled modules appear to include:

```text
pdf
report
report_summary
run_manifest
attack_graph
```

Update the doc to match reality.

Also add a short “Interpretation” section:

```markdown
## Interpretation

The current crate split isolates terminal UI dependencies from the engine crate and moves portable output/tool DTO code into separate crates. The main `slapper` crate remains the largest compile unit because it still owns scanning, web/security modules, API adapters, command dispatch, config, and feature-gated integrations.
```

## Part 2: Dependency audit

### 1. Audit `slapper-output`

Inspect all files under:

```text
crates/slapper-output/src/
```

Compare actual imports with `crates/slapper-output/Cargo.toml`.

Current dependencies include:

```toml
slapper-core
serde
serde_json
chrono
rustc-hash
tracing
quick-xml
unicode-normalization
lru
uuid
hostname
tokio
```

For each dependency, determine whether it is actually used.

Pay special attention to `tokio`. If it is used only for async filesystem/session/scheduling functions, document that. If it is not used, remove it.

Expected actions:

- Remove unused dependencies.
- Keep used dependencies.
- Do not add `slapper`, `clap`, `ratatui`, `crossterm`, `reqwest`, `sqlx`, `axum`, `tonic`, or `pnet` to `slapper-output`.

If `tokio` is used heavily by session/schedule code, consider whether `session` or `schedule` should later move to a separate crate such as `slapper-runs` or `slapper-scheduler`, but do not split that now.

### 2. Audit `slapper-tool-core`

Inspect all files under:

```text
crates/slapper-tool-core/src/
```

Current dependencies include:

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

Remove unused dependencies.

Confirm `slapper-tool-core` does not depend on `slapper`, `slapper-output`, `slapper-tui`, `slapper-cli`, API server crates, or scanner implementation modules.

This crate should remain protocol-neutral DTO/infrastructure, not a registry/executor crate.

### 3. Audit main `slapper` dependencies after extractions

Inspect `crates/slapper/Cargo.toml` for dependencies that may now be stale due to output/tool extraction.

Candidates to verify:

```text
quick-xml
unicode-normalization
lru
uuid
hostname
handlebars
```

Do not remove a dependency just because it appears in `slapper-output`; only remove it if `crates/slapper/src` no longer uses it.

Use `cargo check` after each removal.

### 4. Keep `slapper-core` minimal

Confirm `crates/slapper-core/Cargo.toml` remains minimal.

Do not move new dependencies into `slapper-core` for convenience.

## Part 3: Feature forwarding review

### 1. Review `slapper-cli` feature forwarding

Inspect `crates/slapper-cli/Cargo.toml`.

Current expected pattern:

```toml
nse = ["slapper/nse", "slapper-tui/nse"]
rest-api = ["slapper/rest-api", "slapper-tui/rest-api"]
stress-testing = ["slapper/stress-testing", "slapper-tui/stress-testing"]
pdf = ["slapper/pdf"]
full = ["slapper/full", "slapper-tui/full"]
```

Check that every forwarded feature exists in the target crate.

For features that affect only engine behavior and not TUI rendering, forwarding only to `slapper` is fine.

For features that affect optional TUI tabs, forwarding to both `slapper` and `slapper-tui` may be correct.

Document any intentionally broad forwarding in a short comment in the manifest if helpful.

### 2. Review `slapper-tui` feature forwarding

Inspect `crates/slapper-tui/Cargo.toml`.

The TUI should not own feature semantics for engine modules, but it may need matching features to compile optional tabs.

If possible, make optional tab modules degrade gracefully without needing every integration feature.

Do not perform a major TUI tab redesign in this pass. Just identify obviously unnecessary forwarded features.

### 3. Verify feature checks

Run:

```bash
cargo check -p slapper-cli
cargo check -p slapper-cli --features nse
cargo check -p slapper-cli --features rest-api
cargo check -p slapper-cli --features stress-testing
cargo check -p slapper-cli --features pdf
cargo check -p slapper-cli --features full
```

If `full` is too expensive but should compile, run it if practical. If not run, document that.

Also run:

```bash
cargo check -p slapper --no-default-features
cargo check -p slapper-output
cargo check -p slapper-tool-core
cargo check -p slapper-tui
```

Update `architecture/compile_time_baseline.md` with results.

## Part 4: API / agent extraction preparation

The likely next major extraction is `slapper-api` or `slapper-agent-api`. The presence of `slapper-tool-core` means part of the DTO split has already happened. This pass should map the boundary and optionally create a shell crate only if straightforward.

### 1. Inspect API/tool/agent modules

Inspect:

```text
crates/slapper/src/tool/
crates/slapper/src/agent/
crates/slapper/src/nse_tool.rs
crates/slapper/src/api_schema.rs
crates/slapper/src/commands/handlers/agent*.rs
```

Also inspect feature-gated API dependencies in `crates/slapper/Cargo.toml`:

```toml
axum
tower
tower-http
tonic
prost
prost-types
tonic-prost
tonic-reflection
tokio-stream
async-stream
eventsource-stream
tokio-tungstenite
```

Classify modules into:

```text
A. Protocol-neutral DTOs already in slapper-tool-core.
B. Engine-internal tool registry/execution code that should stay in slapper for now.
C. Server adapters that could move to slapper-api:
   - REST routes
   - MCP protocol adapters
   - OpenAI tool protocol adapters if server-facing
   - gRPC service/server glue
   - WebSocket/event stream glue
D. Autonomous agent scheduling/memory/orchestration that may deserve slapper-agent later.
```

### 2. Write an API extraction boundary note

Create:

```text
architecture/api_extraction_boundary.md
```

Include:

```markdown
# API / Agent Extraction Boundary

## Current owner

## Candidate `slapper-api` modules

## Candidate `slapper-agent` modules

## Must remain in `slapper` for now

## DTOs already in `slapper-tool-core`

## Dependency targets to isolate

## Known blockers

## Proposed next-pass order
```

This is the main deliverable for the next extraction. Be concrete: list file paths and dependencies.

### 3. Optional: create a shell `slapper-api` crate only if trivial

Only do this if it requires no large code moves and no cycles.

Possible shell:

```text
crates/slapper-api/
  Cargo.toml
  src/lib.rs
```

Manifest skeleton:

```toml
[package]
name = "slapper-api"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "API server adapters for Slapper"

[dependencies]
slapper-core = { path = "../slapper-core" }
slapper-tool-core = { path = "../slapper-tool-core" }
slapper = { path = "../slapper", default-features = false }

serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }

axum = { version = "0.8", optional = true }
tower = { version = "0.4", optional = true }
tower-http = { version = "0.6", optional = true, features = ["cors", "trace", "compression-br"] }
tonic = { version = "0.14", optional = true }
prost = { version = "0.14", optional = true }
prost-types = { version = "0.14", optional = true }
tokio-stream = { version = "0.1", optional = true }

[features]
default = []
rest = ["dep:axum", "dep:tower", "dep:tower-http"]
grpc = ["dep:tonic", "dep:prost", "dep:prost-types", "dep:tokio-stream"]
```

Do not move REST/gRPC code in this pass unless it is obviously isolated and small.

If creating the shell crate creates confusion, skip it and just produce the boundary note.

## Part 5: Compatibility shim audit

The repo currently uses compatibility facades, which is acceptable during modularization.

Audit these:

```text
crates/slapper/src/output/mod.rs
crates/slapper/src/tool/mod.rs
crates/slapper/src/types.rs
crates/slapper/src/constants.rs
```

For each, add or verify a brief comment explaining:

```text
- what is re-exported from another crate
- what remains local and why
- whether the shim is temporary or intentionally stable
```

Do not remove shims unless all internal and external references are updated and checks pass.

## Build/test checklist

Run after documentation/dependency cleanup:

```bash
cargo fmt
cargo check -p slapper-core
cargo check -p slapper-tool-core
cargo check -p slapper-output
cargo check -p slapper --no-default-features
cargo check -p slapper-tui
cargo check -p slapper-cli
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

If `full` fails due to pre-existing unrelated issues, document the failure and continue only if base/no-default checks pass.

## Documentation updates summary

Expected modified docs:

```text
architecture/overview.md
architecture/compile_time_baseline.md
architecture/api_extraction_boundary.md
crates/slapper/src/lib.rs
possibly README.md
```

Expected modified manifests:

```text
crates/slapper-output/Cargo.toml
crates/slapper-tool-core/Cargo.toml
crates/slapper/Cargo.toml
crates/slapper-cli/Cargo.toml
crates/slapper-tui/Cargo.toml
```

Only modify manifests when removing unused dependencies or clarifying features.

## Final handoff report

Report:

1. Docs updated and any stale crate/module references corrected.
2. `slapper-output` dependency audit result.
3. `slapper-tool-core` dependency audit result.
4. Main `slapper` stale dependency audit result.
5. Feature forwarding changes, if any.
6. Compatibility shims reviewed.
7. Whether `slapper-api` shell crate was created or deferred.
8. Contents of `architecture/api_extraction_boundary.md`.
9. Commands run and results.
10. Any pre-existing failures.
11. Recommended next pass.

## Stop conditions

Stop and report rather than forcing changes if:

1. Removing a dependency causes non-obvious feature breakage.
2. `slapper-output` or `slapper-tool-core` audit reveals hidden coupling requiring larger refactors.
3. API extraction mapping shows protocol adapters are deeply mixed with engine execution.
4. Creating a `slapper-api` shell crate causes cycles or misleading ownership.
5. Feature forwarding changes break expected CLI builds.

## Recommended next pass after this one

If this pass completes cleanly, the next major extraction should be one of:

```text
slapper-api        # REST/gRPC/WebSocket/MCP server adapters
slapper-agent      # autonomous scheduling/memory/orchestration
slapper-scan       # scanner/probe/recon core, only if edit-path compile time demands it
```

The likely best next step is `slapper-api`, because `slapper-tool-core` already separates DTOs and the main crate still owns heavy API dependencies (`axum`, `tower`, `tonic`, `prost`, etc.).

