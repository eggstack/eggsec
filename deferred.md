# Deferred Items — Detailed Implementation Plan

Items from `fullplan.md` Deferred section, ordered by dependency and impact.

---

## 1. Unified Plugin Trait

**Priority:** Medium — foundation for other plugin work
**Effort:** Large
**Depends on:** Nothing
**Blocks:** Items 2 (class-based plugins), 6 (SecurityTool integration)

### Problem

Three separate plugin systems exist with no unified abstraction:

| Crate | Interface | Runtime |
|-------|-----------|---------|
| `slapper-plugin` | `PythonPluginManager` — calls `register_checks()` / `run_check()` | PyO3 |
| `slapper-ruby` | `PluginLoader` + `RubyBridge` — calls `Slapper::Plugin#run()` | Magnus |
| `slapper-nse` | `Executor` — calls Lua scripts directly | mlua |

None implement a shared trait. Plugins cannot participate in the `SecurityTool` abstraction layer (`tool/traits.rs:117`) or be registered in the `ToolRegistry` (`tool/registry.rs:9`).

### Implementation

**Step 1: Define `Plugin` trait in `slapper-plugin`**

File: `crates/slapper-plugin/src/lib.rs` (add after line 53)

```rust
#[async_trait]
pub trait Plugin: Send + Sync {
    fn info(&self) -> &PluginInfo;
    fn language(&self) -> PluginLanguage;
    fn list_checks(&self) -> Vec<PluginCheck>;
    async fn run_check(&self, check_name: &str, target: &str) -> Result<PluginResult>;
    async fn run(&self, target: &str, config: &PluginConfig) -> Result<PluginResult>;
}
```

**Step 2: Implement `Plugin` for existing backends**

- `PythonPluginManager` (python.rs) — wrap existing `run_check`/`get_checks` behind the trait
- `PluginLoader` in slapper-ruby — wrap `run_plugin` behind the trait via an adapter struct

**Step 3: Add `PluginCheck` to slapper-plugin**

File: `crates/slapper-plugin/src/lib.rs` — the `PluginCheck` struct currently only exists in `python.rs:18`. Move it to `lib.rs` as a shared type.

**Step 4: Create unified `PluginRegistry`**

File: `crates/slapper-plugin/src/lib.rs` (new struct)

```rust
pub struct PluginRegistry {
    plugins: Vec<Arc<dyn Plugin>>,
}
```

Methods: `register()`, `discover()`, `run_check()`, `list()`, `get()`.

### Files to modify

| File | Change |
|------|--------|
| `crates/slapper-plugin/src/lib.rs` | Add `Plugin` trait, `PluginRegistry`, move `PluginCheck` from python.rs |
| `crates/slapper-plugin/src/python.rs` | `PluginCheck` now imported from `lib.rs`; add `Plugin` impl for `PythonPluginManager` |
| `crates/slapper-ruby/src/loader.rs` | Add adapter struct implementing `Plugin` trait |
| `crates/slapper-ruby/Cargo.toml` | Add `slapper-plugin` as dependency |
| `crates/slapper-plugin/Cargo.toml` | Add `async-trait` dependency |

### Verification

```bash
cargo check --lib -p slapper-plugin --features python-plugins
cargo check --lib -p slapper --features python-plugins,ruby-plugins
```

---

## 2. Python Class-Based Plugin Support

**Priority:** Medium
**Effort:** Medium
**Depends on:** Item 1 (unified trait)
**Blocks:** Nothing

### Problem

`docs/PLUGINS.md` documents a class-based API:

```python
class MyPlugin:
    def run(self, target, config) -> dict: ...

PLUGINS = [MyPlugin]
```

But the actual Rust implementation (`python.rs:32-77`) only supports function-based plugins:

```python
def register_checks() -> list[dict]: ...
def run_check(name, target) -> list[str]: ...
```

The documentation is aspirational. Neither `PLUGINS` list scanning nor class detection exists.

### Implementation

**Step 1: Add class detection in `load_plugins()`**

File: `crates/slapper-plugin/src/python.rs:32-77`

After importing a module, scan for a `PLUGINS` list:
```python
# Look for PLUGINS = [MyPlugin, ...]
if hasattr(module, 'PLUGINS'):
    for plugin_class in module.PLUGINS:
        instance = plugin_class()
        # Extract name, version from instance properties
        # Store for later invocation
```

Fall back to current function-based approach if `PLUGINS` is not found.

**Step 2: Call `instance.run(target, config)`**

When running a class-based plugin, call the `run` method with target and config dict, parse the returned dict for `findings` list.

**Step 3: Update `docs/PLUGINS.md`**

Document both class-based and function-based interfaces, noting that class-based is the preferred API going forward.

### Files to modify

| File | Change |
|------|--------|
| `crates/slapper-plugin/src/python.rs` | Add `PLUGINS` list scanning, class instantiation, `run()` invocation |
| `docs/PLUGINS.md` | Document both interfaces |

### Verification

```bash
cargo check --lib -p slapper --features python-plugins
```

---

## 3. Plugin Documentation

**Priority:** Low
**Effort:** Small
**Depends on:** Items 1, 2 (interface should be stable first)
**Blocks:** Nothing

### Problem

`docs/PLUGINS.md` exists (485 lines) but describes a class-based API that doesn't match the current implementation. No NSE plugin documentation exists. No Ruby plugin developer guide exists (only inline doc comments in `api.rs`).

### Implementation

**Step 1: Fix `docs/PLUGINS.md`**

Update to accurately reflect both function-based (current) and class-based (planned) Python interfaces. Remove the assertion that `PLUGINS = [MyPlugin]` works today.

**Step 2: Create `docs/NSE_SCRIPTS.md`**

Document:
- How NSE scripts are loaded from directories
- Script structure (categories, portrule, hostrule, action)
- Available Lua libraries (`io`, `os`, `http`, `dns`, `stdnse`, `shortport`, etc.)
- Security considerations (io.popen gives shell access)

**Step 3: Create `docs/RUBY_PLUGINS.md`**

Document:
- Ruby plugin structure (metadata comment convention: `# Name:`, `# Version:`, etc.)
- Available `Slapper::*` modules: HTTP, Scanner, Fuzzer, Report, Metasploit, Encoder, Session
- Metasploit integration (connect, execute_module, sessions)
- Example plugin skeleton

**Step 4: Create `docs/PLUGIN_DEVELOPMENT.md`** (unified guide)

Overview document linking to Python, Ruby, and NSE guides. Covers plugin discovery, directory structure, configuration, and output format.

### Files to create/modify

| File | Action |
|------|--------|
| `docs/PLUGINS.md` | Fix inaccuracies, add both interfaces |
| `docs/NSE_SCRIPTS.md` | New — NSE developer guide |
| `docs/RUBY_PLUGINS.md` | New — Ruby developer guide |
| `docs/PLUGIN_DEVELOPMENT.md` | New — unified overview |

---

## 4. Plugin Sandboxing

**Priority:** Medium — security concern
**Effort:** Large
**Depends on:** Nothing
**Blocks:** Nothing

### Problem

NSE Lua scripts have unrestricted access to:

| Library | Dangerous Functions | Risk |
|---------|-------------------|------|
| `io` | `io.popen(cmd)` — runs `sh -c <cmd>` on Unix | **Arbitrary command execution** |
| `io` | `io.open(path, mode)` — reads/writes any file | **Filesystem access** |
| `os` | `os.getenv(name)` — reads env vars | **Credential leak** |
| `os` | `os.setenv(name, value)` — modifies process env | **Process corruption** (uses `unsafe`) |
| `os` | `os.remove(path)` — deletes files | **Data destruction** |
| `os` | `os.rename(old, new)` — renames files | **File manipulation** |

`os.execute` is already blocked (returns failure status at `os.rs:109-119`), but `io.popen` provides equivalent shell access, making the block meaningless.

There is no sandbox feature flag, no path restriction, no capability system.

### Implementation

**Phase A: Sandbox feature flag**

Add `sandbox` feature to `crates/slapper-nse/Cargo.toml`. When enabled, restrict dangerous operations.

**Phase B: Restrict `io.popen`**

File: `crates/slapper-nse/src/libraries/io.rs:199-283`

When sandboxed:
- Option 1: Return error "io.popen disabled in sandbox mode"
- Option 2: Allowlist commands (e.g., only `nmap`, `curl`, `dig`)
- Option 3: Log and audit all `popen` calls without blocking (observability)

Recommended: Option 1 (disable) as default sandbox behavior.

**Phase C: Restrict `io.open` file paths**

File: `crates/slapper-nse/src/libraries/io.rs:21-86`

When sandboxed:
- Restrict to a configurable working directory (e.g., `--sandbox-dir /tmp/slapper-nse`)
- Reject paths containing `..` or absolute paths outside sandbox dir
- Use `canonicalize()` to resolve symlinks before checking

**Phase D: Restrict `os` functions**

File: `crates/slapper-nse/src/libraries/os.rs`

When sandboxed:
- `os.getenv`: Return empty string or configurable allowlist
- `os.setenv`/`os.unsetenv`: Return error (already uses `unsafe`)
- `os.remove`/`os.rename`: Restrict to sandbox directory
- `os.chdir`: Restrict to sandbox directory

**Phase E: Configuration**

Add to `SlapperConfig` or as CLI flags:
```rust
pub struct SandboxConfig {
    pub enabled: bool,
    pub allowed_dir: Option<PathBuf>,
    pub allowed_commands: Vec<String>,  // for io.popen allowlist
    pub log_violations: bool,
}
```

Pass through `Executor::new()` or `ExecutorCore::new()`.

### Files to modify

| File | Change |
|------|--------|
| `crates/slapper-nse/Cargo.toml` | Add `sandbox` feature |
| `crates/slapper-nse/src/libraries/io.rs` | Add sandbox checks to `popen`, `open`, `write`, `read` |
| `crates/slapper-nse/src/libraries/os.rs` | Add sandbox checks to `getenv`, `setenv`, `remove`, `rename`, `chdir` |
| `crates/slapper-nse/src/executor_core.rs` | Accept `SandboxConfig`, pass to library registration |
| `crates/slapper-nse/src/executor.rs` | Accept `SandboxConfig` in constructor |

### Verification

```bash
cargo check --lib -p slapper-nse --features nse,sandbox
cargo test --lib -p slapper-nse --features nse,sandbox
```

---

## 5. Output Consolidation

**Priority:** Low
**Effort:** Medium
**Depends on:** Nothing
**Blocks:** Nothing

### Problem

Two parallel output implementations exist:

| Implementation | Location | Used By |
|---------------|----------|---------|
| Standalone functions | `output/convert.rs` (368 lines) | `commands/handlers/report.rs` |
| Builder-pattern modules | `output/html.rs`, `output/sarif.rs`, `output/junit.rs` | Nothing (re-exported but unused) |

`convert_to_*` functions are called from two locations:
- `commands/handlers/report.rs:19-21`
- `tui/tabs/settings.rs:273-278`

The dedicated modules have richer APIs (Chart.js HTML, full SARIF hierarchy, proper XML JUnit) but are never called.

### Implementation

**Step 1: Update `report.rs` to use dedicated builder modules**

File: `crates/slapper/src/commands/handlers/report.rs`

Replace:
```rust
let html = convert::convert_to_html(&report);
```

With:
```rust
let summary = ScanSummary::from(&report);
let findings: Vec<Finding> = report.findings.iter().map(Finding::from).into();
let html_report = HtmlReport::new(summary, findings);
let html = html_report.generate();
```

Similar for SARIF (`SarifBuilder`) and JUnit (`JUnitBuilder`).

**Step 2: Add conversion impls**

File: `crates/slapper/src/output/convert.rs` — add `From<ScanReportData>` impls for the builder types so the conversion is seamless.

**Step 3: Deprecate or delete `convert.rs` inline renderers**

Keep `load_scan_report()` (JSON loading) and the data structs. Remove `convert_to_html()`, `convert_to_sarif()`, `convert_to_junit()` since they duplicate the builder modules.

**Step 4: Keep `convert_to_csv()` and `convert_to_markdown()`**

These don't have dedicated builder equivalents. Either keep them in `convert.rs` or move to `output/csv.rs` and `output/markdown.rs`.

### Files to modify

| File | Change |
|------|--------|
| `crates/slapper/src/commands/handlers/report.rs` | Use builder modules instead of `convert::` functions |
| `crates/slapper/src/tui/tabs/settings.rs` | Use builder modules instead of `convert::` functions |
| `crates/slapper/src/output/convert.rs` | Remove duplicate HTML/SARIF/JUnit functions; add `From` impls |
| `crates/slapper/src/output/mod.rs` | Update re-exports |

### Verification

```bash
cargo check --lib -p slapper
cargo test --lib -p slapper
```

---

## 6. Split Commands Enum

**Priority:** Low
**Effort:** Medium
**Depends on:** Nothing
**Blocks:** Nothing

### Problem

`Commands` enum in `cli/mod.rs` has **26 variants** (18 always, 8 behind `#[cfg(feature)]`), all in one flat enum.

### Implementation

Group variants into subcommand enums:

```rust
#[derive(Subcommand)]
pub enum Commands {
    // Scan operations
    #[command(subcommand)]
    Scan(ScanCommands),

    // Attack operations
    #[command(subcommand)]
    Attack(AttackCommands),

    // Recon operations
    Recon(ReconArgs),

    // Tool operations
    #[command(subcommand)]
    Tool(ToolCommands),

    // Infrastructure
    #[command(subcommand)]
    Infra(InfraCommands),
}

#[derive(Subcommand)]
pub enum ScanCommands {
    Ports(PortScanArgs),
    Endpoints(EndpointScanArgs),
    Fingerprint(FingerprintArgs),
    Run(ScanArgs),
    Resume(ResumeArgs),
}

#[derive(Subcommand)]
pub enum AttackCommands {
    Fuzz(FuzzArgs),
    Waf(WafArgs),
    WafStress(WafStressArgs),
    Graphql(GraphQlArgs),
    OAuth(OAuthArgs),
    #[cfg(feature = "stress-testing")]
    Stress(StressArgs),
}

#[derive(Subcommand)]
pub enum ToolCommands {
    Packet(PacketArgs),
    #[cfg(feature = "nse")]
    Nse(NseArgs),
    #[cfg(feature = "python-plugins")]
    Plugin(PluginArgs),
    Report(ReportArgs),
}

#[derive(Subcommand)]
pub enum InfraCommands {
    Cluster(ClusterArgs),
    Notify(NotifyArgs),
    Remote(RemoteArgs),
    Exec(ExecArgs),
    #[cfg(feature = "stress-testing")]
    Proxy(ProxyArgs),
    #[cfg(feature = "stress-testing")]
    Icmp(IcmpArgs),
    #[cfg(feature = "stress-testing")]
    Traceroute(TracerouteArgs),
    #[cfg(feature = "rest-api")]
    Serve(ServeArgs),
    #[cfg(feature = "mcp-server")]
    McpServe(McpServeArgs),
}
```

This changes CLI invocation from `slapper scan-ports` to `slapper scan ports`. This is a **breaking change** to the CLI interface.

### Files to modify

| File | Change |
|------|--------|
| `crates/slapper/src/cli/mod.rs` | Restructure `Commands` enum into sub-enums |
| `crates/slapper/src/commands/handlers/*.rs` | Update match arms to handle nested enums |

### Verification

```bash
cargo check --lib -p slapper --features full
cargo test --lib -p slapper --features full
```

### Warning

This is a **breaking CLI change**. All user scripts, CI configs, and documentation referencing `slapper scan-ports` would need updating to `slapper scan ports`. Consider adding backward-compatible aliases or deferring until a major version bump.

---

## 7. Unwrap/Expect Audit

**Priority:** Medium
**Effort:** Medium
**Depends on:** Nothing
**Blocks:** Nothing

### Problem

~150 `.unwrap()` calls in non-test source code, ~40 `.expect()` calls. Key risk areas:

| File | Count | Risk |
|------|-------|------|
| `nse/src/libraries/nmap.rs` | 38 unwrap | Lua script library — could panic on malformed input |
| `nse/src/output.rs` | 35 unwrap | Lua output parsing |
| `nse/src/libraries/stdnse.rs` | 18 unwrap | Lua standard library |
| `tool/state.rs` | 15 unwrap | Tool state management |
| `config/loader.rs` | 14 unwrap | Config loading |
| `distributed/io.rs` | 13 unwrap | Distributed I/O |
| `ruby/src/api.rs` | 13 unwrap | Ruby API bridge |

Common hot-path pattern: `ProgressStyle::default_bar().template("...").unwrap()` in `fuzzer/engine/execution.rs:26,71`, `scanner/ports/spoofed.rs:102`, `loadtest/runner.rs:256`.

### Implementation

**Tier 1: Critical hot paths (do first)**

Replace `ProgressStyle::template().unwrap()` in 4 locations with `.unwrap_or_else(|_| ProgressStyle::default_bar())`. This prevents panic if template string is ever malformed.

Files:
- `fuzzer/engine/execution.rs:26,71`
- `scanner/ports/spoofed.rs:102`
- `loadtest/runner.rs:256`

**Tier 2: NSE Lua libraries (high risk)**

`nmap.rs` (38), `output.rs` (35), `stdnse.rs` (18) — these unwrap inside Lua library callbacks. If Lua passes unexpected types, the entire process panics.

Fix: Replace `.unwrap()` with `.map_err(|e| mlua::Error::RuntimeError(e.to_string()))?` inside Lua callback functions. This converts panics into Lua errors that scripts can handle.

**Tier 3: Config and I/O**

`config/loader.rs` (14), `distributed/io.rs` (13) — unwrap on config parsing and network I/O.

Fix: Replace with `.context("descriptive message")?` using anyhow context.

**Tier 4: Remaining (~70 calls)**

Audit each remaining unwrap. Most will be acceptable (iterator on known-length collections, format on known-valid inputs). Mark acceptable ones with `// SAFETY:` comments explaining why they can't fail.

### Files to modify

| Tier | Files | Changes |
|------|-------|---------|
| 1 | `fuzzer/engine/execution.rs`, `scanner/ports/spoofed.rs`, `loadtest/runner.rs` | Replace template unwraps |
| 2 | `nse/src/libraries/nmap.rs`, `nse/src/output.rs`, `nse/src/libraries/stdnse.rs` | Convert to `?` with mlua errors |
| 3 | `config/loader.rs`, `distributed/io.rs` | Add `.context()` |
| 4 | Remaining ~40 files | Audit + document acceptable ones |

### Verification

```bash
cargo check --lib -p slapper --features full
cargo test --lib -p slapper --features full
cargo clippy --lib -p slapper --features full -- -D warnings
```

---

## Execution Order

```
Independent (can be done in parallel):
  1. Unified Plugin trait
  7. Unwrap/expect audit

Dependent chain:
  1. Unified Plugin trait
  └── 2. Python class-based plugins
      └── 3. Plugin documentation

Independent (can be done anytime):
  4. Plugin sandboxing
  5. Output consolidation
  6. Split Commands enum (WARNING: breaking CLI change)

Recommended sequence:
  Phase A: Items 1, 4, 7 (foundation, security, robustness)
  Phase B: Items 2, 5 (depends on 1, independent)
  Phase C: Items 3, 6 (documentation, CLI redesign — lowest risk last)
```

---

## Verification Commands

```bash
# After each item
cargo check --lib -p slapper --features full
cargo test --lib -p slapper --features full
cargo clippy --lib -p slapper --features full -- -D warnings

# Plugin-specific
cargo check --lib -p slapper-plugin --features python-plugins
cargo check --lib -p slapper-ruby --features ruby-plugins
cargo check --lib -p slapper-nse --features nse,sandbox
```

---

## Success Criteria

| Item | Criterion |
|------|-----------|
| 1. Unified Plugin trait | `Plugin` trait defined; Python and Ruby backends implement it |
| 2. Class-based plugins | `PLUGINS = [MyPlugin]` pattern works in Python |
| 3. Plugin documentation | All 3 plugin types documented with accurate examples |
| 4. Plugin sandboxing | `io.popen` disabled by default; filesystem restricted |
| 5. Output consolidation | `report.rs` uses builder modules; `convert.rs` renderers removed |
| 6. Split Commands enum | Commands grouped into subcommand categories |
| 7. Unwrap audit | Hot-path unwraps eliminated; NSE library unwraps converted to errors |
