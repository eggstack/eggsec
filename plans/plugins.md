# Slapper Plan: Remove Python/Ruby/Metasploit Plugins, Keep NSE Compatibility

## Objective

Remove Slapper's Python, Ruby, and Metasploit plugin subsystems from the main codebase while preserving and repositioning NSE support as an optional Nmap-compatibility/runtime layer. The end state should simplify Slapper's maintenance burden, reduce dynamic runtime dependencies, remove arbitrary Python/Ruby plugin execution from the product surface, and clarify Slapper's new positioning as a Rust-native security assessment engine with AI-oriented orchestration and selective NSE compatibility.

This plan is intended for a smaller implementation model. Follow it mechanically. Prefer small, compileable commits or phases. Do not redesign the entire application. The goal is removal, cleanup, and repositioning, not a broad feature rewrite.

## Strategic Decisions

Python plugins should be removed.

Ruby plugins should be removed.

Metasploit integration should be removed with the Ruby plugin system.

The `slapper-plugin` crate should be removed unless it is still needed after cleanup. If any generic result types are genuinely reused outside Python/Ruby plugins, migrate those types to a neutral module in `crates/slapper` or rename them as non-plugin concepts. Do not keep `slapper-plugin` only as dead compatibility scaffolding.

The `slapper-ruby` crate should be removed.

The `slapper-nse` crate should be kept.

NSE should not be described as part of the removed plugin system. It should be described as optional NSE/Nmap compatibility for scriptable probes over Slapper's Rust engine primitives.

The default build should not include NSE unless it already did. NSE should remain feature-gated. If there is a `full` feature, it may keep NSE only if `full` explicitly means every optional subsystem; however, prefer adding or documenting a distinction between core feature bundles and compatibility-heavy feature bundles.

## Current Relevant Codebase Facts

The workspace currently includes these related crates in root `Cargo.toml`:

```toml
members = [
    "crates/slapper",
    "crates/slapper-plugin",
    "crates/slapper-nse",
    "crates/slapper-ruby",
]
```

The main crate currently has optional dependencies on:

```toml
slapper-plugin = { path = "../slapper-plugin", optional = true }
slapper-nse = { path = "../slapper-nse", optional = true }
slapper-ruby = { path = "../slapper-ruby", optional = true }
```

The main crate currently defines features similar to:

```toml
python-plugins = ["dep:slapper-plugin", "slapper-plugin/python-plugins"]
ruby-plugins = ["dep:slapper-plugin", "slapper-plugin/ruby-plugins", "dep:slapper-ruby", "slapper-ruby/ruby-plugins"]
all-plugins = ["python-plugins", "ruby-plugins"]
nse = ["tool-api", "dep:slapper-nse", "slapper-nse/nse"]
nse-ssh2 = ["nse", "slapper-nse/nse-ssh2"]
nse-sandbox = ["nse", "slapper-nse/sandbox"]
full = ["python-plugins", "ruby-plugins", "stress-testing", "packet-inspection", "rest-api", "nse", ...]
```

The CLI currently exposes a `Plugin(PluginArgs)` command behind:

```rust
#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
```

The command handler currently routes `Commands::Plugin(args)` to `handle_plugin(ctx, args)` behind the same feature gate.

Known plugin-related files/directories include at least:

```text
crates/slapper-plugin/
crates/slapper-ruby/
crates/slapper/src/commands/handlers/plugin.rs
crates/slapper/src/tui/tabs/plugin.rs
crates/slapper/src/cli/... plugin args definitions, likely in cli modules
crates/slapper/src/lib.rs or module exports referencing plugin/ruby
README.md
docs/PLUGIN_DEVELOPMENT.md
docs/PLUGINS.md
docs/NSE_SCRIPTS.md
docs/CAPABILITIES.md
```

Do not assume this list is exhaustive. Use ripgrep locally.

## Working Branch

Create a branch before making changes:

```bash
git checkout -b remove-python-ruby-plugins-keep-nse
```

## Phase 1: Inventory References

Run these searches and save the results mentally or in scratch notes:

```bash
rg -n "python-plugins|ruby-plugins|all-plugins|slapper-plugin|slapper-ruby|PluginArgs|PluginCommand|handle_plugin|PythonPlugin|RubyPlugin|Metasploit|metasploit|Msf|msf|PLUGINS|plugin list|run-plugin|list-plugins|PLUGIN_DEVELOPMENT|PLUGINS.md" .
rg -n "nse|NSE|Nmap|nmap" crates README.md docs
```

Classify findings into four groups:

1. Python/Ruby/Metasploit code to delete.
2. Python/Ruby/Metasploit references to remove from docs/config/examples.
3. Generic concepts that should be renamed or migrated if still useful.
4. NSE references to keep and reword as compatibility/runtime, not plugin support.

Do not remove NSE files or features unless they directly depend on Python/Ruby plugin code.

## Phase 2: Cargo Workspace Cleanup

Edit root `Cargo.toml`.

Remove these workspace members:

```toml
"crates/slapper-plugin",
"crates/slapper-ruby",
```

Keep:

```toml
"crates/slapper-nse",
```

Then edit `crates/slapper/Cargo.toml`.

Remove optional dependencies:

```toml
slapper-plugin = { path = "../slapper-plugin", optional = true }
slapper-ruby = { path = "../slapper-ruby", optional = true }
```

Keep:

```toml
slapper-nse = { path = "../slapper-nse", optional = true }
```

Remove features:

```toml
python-plugins = ...
ruby-plugins = ...
all-plugins = ...
```

Update `full` so it no longer includes `python-plugins` or `ruby-plugins`.

Recommended `full` handling:

```toml
full = ["stress-testing", "packet-inspection", "rest-api", "nse", "ai-integration", "websocket", "headless-browser", "database", "container", "sbom", "advanced-hunting", "compliance", "external-integrations", "finding-workflow", "vuln-management", "wireless"]
```

If you want a clearer split, add comments only; avoid introducing a new feature bundle unless required:

```toml
# NSE remains optional compatibility support for Nmap NSE scripts.
# Python/Ruby arbitrary plugin runtimes were intentionally removed.
```

After Cargo edits, run:

```bash
cargo metadata --no-deps
```

Fix feature references until metadata resolves.

## Phase 3: Delete Removed Crates

Delete these directories:

```bash
rm -rf crates/slapper-plugin
rm -rf crates/slapper-ruby
```

Do not delete:

```bash
crates/slapper-nse
```

Run:

```bash
cargo metadata --no-deps
```

Fix any remaining workspace references.

## Phase 4: Remove Plugin CLI Surface

Find the CLI enum variant for plugin management. It likely looks like this:

```rust
#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
#[command(about = "Manage and run security scanning plugins")]
Plugin(PluginArgs),
```

Remove the `Plugin` command variant entirely.

Find and remove `PluginArgs`, `PluginCommand`, plugin list/run args, and any imports only used by them. These may live in `crates/slapper/src/cli/mod.rs` or a submodule under `crates/slapper/src/cli/`.

In command handling, remove:

```rust
#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
Some(Commands::Plugin(args)) => handle_plugin(ctx, args).await,
```

Remove module declaration/export for the plugin handler, likely in `crates/slapper/src/commands/handlers/mod.rs`:

```rust
pub mod plugin;
#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
pub use plugin::*;
```

Delete:

```bash
rm crates/slapper/src/commands/handlers/plugin.rs
```

Run:

```bash
cargo check -p slapper
```

Fix all compiler errors from deleted command types/imports.

## Phase 5: Remove Plugin Module Exports and TUI Surface

Search:

```bash
rg -n "plugin|Plugin|ruby|Ruby|python|Python|metasploit|Metasploit|msf|Msf" crates/slapper/src
```

Remove only references to the removed Python/Ruby/Metasploit plugin system. Be careful not to remove unrelated words such as browser plugin detection if any exist.

Likely removals:

```text
crates/slapper/src/tui/tabs/plugin.rs
plugin tab imports
plugin tab enum variants
plugin tab routing/rendering
plugin module re-exports in lib.rs
ruby module re-exports in lib.rs
```

If the TUI has a plugin tab in a tabs enum, remove that tab and fix index navigation. Do not leave a dead empty tab.

If any config UI references plugin directories or plugin enablement, remove them.

Run:

```bash
cargo check -p slapper
```

## Phase 6: Config Cleanup

Search config code:

```bash
rg -n "plugins|plugin|python|ruby|metasploit|msf" crates/slapper/src/config* crates/slapper/src
```

Remove config fields that exist only for Python/Ruby/Metasploit plugins, such as:

```toml
[plugins]
[plugins.python]
[plugins.ruby]
paths.plugins_dir
```

Only remove `paths.plugins_dir` if it is exclusively used by the removed plugin system. If NSE or future check packs need a script/check directory, rename the config field to something neutral and explicit, such as:

```rust
nse_scripts_dir
check_packs_dir
```

Do not introduce a check pack system in this task. Only avoid leaving stale plugin config.

Update `get_default_config()` or default config templates so they no longer mention Python/Ruby plugins.

Run config tests if present:

```bash
cargo test -p slapper config
```

## Phase 7: Preserve and Reposition NSE CLI

Confirm the NSE command still exists and is gated only by `nse`:

```rust
#[cfg(feature = "nse")]
#[command(about = "Run Nmap NSE scripts for security scanning", long_about = NSE_ABOUT)]
Nse(NseArgs),
```

Keep it.

Review the help/about text for NSE. Reword it if it currently says “plugin” or implies it is part of the old plugin system.

Preferred wording:

```text
Run Nmap NSE-compatible scripts through Slapper's optional Lua/NSE compatibility runtime.
```

Preferred long positioning:

```text
NSE support provides selective compatibility with Nmap Scripting Engine semantics for scriptable discovery and service checks. It is an optional compatibility layer, separate from the removed Python/Ruby plugin runtimes, and should be used for approved scripts within Slapper's scope and execution policy.
```

Run:

```bash
cargo check -p slapper --features nse
```

Also run:

```bash
cargo check -p slapper-nse --features nse
```

If sandbox support exists, run:

```bash
cargo check -p slapper --features nse-sandbox
cargo check -p slapper-nse --features "nse sandbox"
```

## Phase 8: Remove Python/Ruby/Metasploit Docs

Delete these docs if they are exclusively about the removed subsystem:

```bash
rm docs/PLUGIN_DEVELOPMENT.md
rm docs/PLUGINS.md
```

If another doc links to them, remove those links or replace them with NSE/check-pack positioning.

Search docs and README:

```bash
rg -n "Plugin|plugin|Python|python|Ruby|ruby|Metasploit|metasploit|MSF|msf|all-plugins|python-plugins|ruby-plugins|run-plugin|list-plugins|PLUGIN_DEVELOPMENT|PLUGINS.md" README.md docs crates
```

Remove or rewrite all references to Python/Ruby/Metasploit plugin support.

Keep references to NSE, but make terminology precise:

Use:

```text
NSE compatibility
Nmap NSE-compatible scripts
Lua/NSE runtime
scriptable probes
compatibility layer
```

Avoid:

```text
plugins in Python and Ruby
all plugin languages
plugin marketplace
Metasploit integration
Ruby plugins
Python plugins
```

## Phase 9: README Repositioning

Update the top-level README to reflect the new direction.

Recommended new short positioning:

```markdown
# Slapper - Rust Security Assessment Engine

Slapper is a Rust-native security assessment engine for scoped, repeatable testing of live systems. It provides high-performance primitives for reconnaissance, port and service scanning, endpoint discovery, web/API security checks, WAF evaluation, fuzzing, load testing, reporting, and AI-oriented orchestration.

Slapper is not intended to be a Metasploit clone or a general arbitrary-code plugin host. Its core value is a maintainable Rust engine with policy-aware execution, structured outputs, and optional compatibility layers such as Nmap NSE support.
```

Update the feature table:

Remove:

```text
Python plugin support
Ruby plugin support
Ruby plugins, Metasploit integration
all-plugins
```

Keep/add:

```text
nse | Optional Nmap NSE-compatible Lua runtime | Approved NSE-compatible discovery/service scripts
nse-sandbox | Restrict Lua/NSE filesystem/process behavior | Safer execution of untrusted or third-party NSE-compatible scripts
nse-ssh2 | Optional SSH2-backed NSE compatibility | SSH-oriented NSE-compatible checks
```

Update system dependencies:

Remove Ruby dependency instructions:

```text
ruby-dev
ruby-devel
brew install ruby
clang for Ruby plugins
```

Keep NSE-related dependencies if they remain accurate, such as OpenSSL requirements. Note that `slapper-nse` uses vendored OpenSSL when the `nse` feature is enabled, so verify whether system `libssl-dev` is truly required. If vendored OpenSSL removes the need for system OpenSSL, update docs accordingly.

Update build examples:

Remove:

```bash
cargo build --release --features python-plugins
cargo build --release --features ruby-plugins
cargo build --release --features all-plugins
```

Add or keep:

```bash
cargo build --release --features nse
cargo build --release --features nse-sandbox
cargo build --release --features nse-ssh2
```

Update command reference:

Remove any `slapper plugin list`, `slapper plugin run`, `run-plugin`, `list-plugins` examples.

Keep `slapper nse` examples if accurate.

## Phase 10: Capabilities and Agent/MCP Docs Cleanup

Inspect:

```text
docs/CAPABILITIES.md
docs/AGENT.md
docs/API_TESTING.md
docs/NSE_SCRIPTS.md
README.md
```

Remove claims that Slapper supports Python/Ruby plugins or Metasploit integration.

Update AI/agent wording to emphasize typed Rust-native tools and NSE as an optional compatibility layer.

Recommended wording for agent-facing docs:

```markdown
Agent-facing integrations should prefer Slapper's typed Rust-native commands and structured outputs. NSE support is exposed as an optional compatibility capability for approved NSE-compatible checks. Agents should not be given arbitrary script execution by default; prefer allowlisted checks with scope and execution-policy enforcement.
```

If MCP exposes plugin-related tools, remove them. If MCP exposes NSE tools, keep them but ensure they are described as NSE compatibility and ideally require approved script names rather than arbitrary file execution.

Search:

```bash
rg -n "plugin|Plugin|run_plugin|list_plugins|python|ruby|metasploit|nse" crates/slapper/src/api crates/slapper/src/mcp crates/slapper/src/server crates/slapper/src
```

Adjust actual paths based on repo structure.

## Phase 11: Tests and Snapshots

Search tests:

```bash
rg -n "plugin|Plugin|python-plugins|ruby-plugins|slapper-plugin|slapper-ruby|Metasploit|msf" crates tests .github
```

Remove tests that cover deleted plugin systems.

Remove CI jobs or matrix entries for:

```text
python-plugins
ruby-plugins
all-plugins
slapper-plugin
slapper-ruby
Ruby development headers
Python plugin runtime tests
Metasploit tests
```

Keep or add checks for:

```text
cargo check -p slapper
cargo check -p slapper --features nse
cargo check -p slapper --features nse-sandbox
cargo check -p slapper-nse --features nse
cargo test -p slapper-nse --features nse
```

If CI has a `full` feature build, update it after removing plugin features:

```bash
cargo check --workspace --features full
```

If `--workspace --features full` no longer makes sense because `full` is only on `slapper`, use:

```bash
cargo check -p slapper --features full
```

## Phase 12: Final Ripgrep Gate

At the end, this command should produce no references to the removed feature surface except changelog/plan text if intentionally kept:

```bash
rg -n "python-plugins|ruby-plugins|all-plugins|slapper-plugin|slapper-ruby|Python plugin|Ruby plugin|Metasploit|metasploit|MSF|msf|run-plugin|list-plugins|PLUGIN_DEVELOPMENT|PLUGINS.md" .
```

If there are matches, either delete them or rewrite them. Acceptable remaining matches only if they appear in a migration note or changelog explicitly saying the subsystem was removed.

This command may still show lowercase `plugin` in unrelated contexts. Review manually:

```bash
rg -n "plugin|Plugin" README.md docs crates/slapper crates/slapper-nse
```

Any remaining `plugin` wording should be either unrelated or replaced with more precise language such as `NSE compatibility`, `check`, `probe`, `script`, `adapter`, or `integration`.

## Phase 13: Build and Test Matrix

Run the following minimum checks:

```bash
cargo fmt --all --check
cargo check -p slapper
cargo check -p slapper --features nse
cargo check -p slapper --features nse-sandbox
cargo check -p slapper --features nse-ssh2
cargo check -p slapper --features full
cargo test -p slapper
cargo test -p slapper-nse --features nse
```

If `nse-ssh2` fails due to system dependency issues, document that separately and at minimum ensure:

```bash
cargo check -p slapper --features nse
cargo check -p slapper --features nse-sandbox
```

If `full` fails because it includes unrelated optional features with missing system dependencies, do not mask the plugin-removal work. Record the unrelated blocker and run the largest feasible feature subset that includes NSE and excludes removed plugin systems.

## Phase 14: Optional Migration Note

Add a short migration note in `CHANGELOG.md`, `docs/MIGRATION.md`, or a new `docs/REMOVED_PLUGINS.md` only if the project keeps migration docs.

Suggested text:

```markdown
## Python/Ruby Plugin Runtime Removal

Slapper no longer includes the Python or Ruby plugin runtimes, including the prior Metasploit-oriented Ruby integration. These systems were removed to reduce maintenance burden, simplify builds, and align Slapper around its Rust-native assessment engine and AI-oriented typed tool surface.

NSE support remains available as an optional Nmap NSE compatibility layer via the `nse`, `nse-sandbox`, and `nse-ssh2` features. NSE should be treated as a compatibility/runtime feature for approved scripts, not as a general arbitrary-code plugin system.
```

Do not over-explain. This is an internal/private repo cleanup unless the repo is being prepared for external release.

## Non-Goals

Do not implement a new declarative check-pack system in this task.

Do not rewrite the NSE runtime.

Do not attempt full Nmap NSE parity.

Do not remove NSE.

Do not remove core scan/fuzz/WAF/recon/load testing features.

Do not introduce a new Python subprocess plugin replacement.

Do not preserve Ruby/Metasploit as deprecated-but-present code. If it is removed, remove it cleanly.

## Expected Final State

The workspace no longer contains `crates/slapper-plugin` or `crates/slapper-ruby`.

The main crate no longer has `python-plugins`, `ruby-plugins`, or `all-plugins` features.

The main CLI no longer has `slapper plugin ...`, `run-plugin`, or `list-plugins` commands.

The TUI no longer has a plugin tab for Python/Ruby plugin discovery.

Docs no longer advertise Python plugins, Ruby plugins, or Metasploit integration.

README positions Slapper as a Rust-native security assessment engine with AI-oriented orchestration and optional NSE compatibility.

NSE still builds under its feature flags.

`cargo check -p slapper` succeeds.

`cargo check -p slapper --features nse` succeeds.

Final ripgrep has no stale references to removed plugin features.

## Suggested Commit Breakdown

Commit 1: Remove plugin crates from workspace and Cargo features.

Commit 2: Remove plugin CLI, handlers, TUI tab, and config references.

Commit 3: Remove Python/Ruby/Metasploit docs and README references.

Commit 4: Reposition NSE docs and README feature descriptions.

Commit 5: Clean tests/CI and verify build matrix.
