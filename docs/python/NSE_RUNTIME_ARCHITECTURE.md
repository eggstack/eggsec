# NSE Runtime Architecture

## Overview

The NSE (Nmap Scripting Engine) runtime bridges Nmap's Lua-based scripting
engine into the Eggsec Rust security toolkit. The Python bindings expose this
runtime through `eggsec.nse`, providing script execution, library introspection,
and metadata querying.

## Architecture Layers

```
┌─────────────────────────────────────────────────────┐
│  Python Surface (eggsec.nse)                        │
│  nse_run, nse_list_libraries, nse_list_scripts, ... │
├─────────────────────────────────────────────────────┤
│  PyO3 Bindings (crates/eggsec-python/src/nse.rs)    │
│  DTOs, conversion, sync/async dispatch              │
├─────────────────────────────────────────────────────┤
│  Eggsec NSE Engine (crates/eggsec-nse)              │
│  NseExecutor, ScriptResolver, NseRunReport          │
├─────────────────────────────────────────────────────┤
│  Lua 5.4 VM (mlua)                                 │
│  Script interpretation, library loading             │
├─────────────────────────────────────────────────────┤
│  NSE Library Wrappers (src/libraries/)              │
│  stdnse, http, dns, socket, ssl, ...                │
└─────────────────────────────────────────────────────┘
```

## Parser / Compiler / Interpreter Boundary

1. **Parser**: Lua source is parsed by mlua's Lua 5.4 parser into bytecode.
   No separate compiler stage; parsing and compilation happen together.

2. **Compiler**: mlua compiles parsed Lua source into Lua bytecode
   internally. Eggsec does not modify the compilation pipeline.

3. **Interpreter**: The Lua 5.4 VM executes bytecode. Eggsec wraps
   dangerous operations (io.popen, filesystem access, network I/O)
   through the capability context for sandbox enforcement.

## Metadata, Categories, Dependencies, Rules, Libraries

### Library Registry (`resolver/registry.rs`)

The static `LIBRARY_REGISTRY` contains 43 entries (24 main + 19 auxiliary)
describing each standard Nmap Lua library:

- **NseLibraryCategory**: Core, Protocol, Utility, Exploit, Auth
- **NseSandboxSideEffect**: None, FileSystemRead, FileSystemWrite,
  NetworkAccess, ProcessExecution, EnvAccess
- **NseFallbackBehavior**: HardFail, GracefulDegrade, Skip
- **EnforcementStatus**: Wrapped, PartiallyWrapped, ManualOnly, Deferred, Pure

### Script Resolution

`ScriptResolver` enforces profile-derived policies:
- Module name grammar validation (ASCII alphanumeric + `_`, `-`, `.`)
- Canonical path validation under approved roots
- Symlink-aware containment checks
- File extension allowlists
- Maximum script/module sizes

### Built-in Scripts

Six built-in scripts are compiled into the binary:
`default`, `discovery`, `banner`, `http-headers`, `dns-check`, `ssl-cert`

These are defined in `resolver/mod.rs` (`BUILTIN_SCRIPT_NAMES`) and
`lib.rs` (`get_builtin_script()`).

## Arguments, Target Context, Sandbox Controls

### NseConfig

| Field | Type | Description |
|-------|------|-------------|
| target | String | Target host or IP address |
| script | String | Script name or inline Lua content |
| script_args | Option<String> | Comma-separated key=value arguments |
| script_file | Option<String> | Path to external script file |
| json | bool | JSON output mode |
| verbose | bool | Verbose output |

### NseTargetContext

Provides host, port, and service information for scripts to tailor behavior:
host_ip, hostname, port, protocol, service_name, service_product,
service_version, os_detection.

### NseSandboxPolicy

Controls script execution restrictions:
- `allow_filesystem` / `allowed_dirs`: Filesystem access scope
- `allow_network` / `allowed_cidrs`: Network access scope
- `max_lua_instructions`: Instruction budget (default: 1,000,000)
- `max_output_bytes`: Output size limit (default: 1 MiB)
- `max_network_ops`: Network operation count limit
- `max_memory_bytes`: Lua memory limit (default: 64 MiB)

## Instruction / Memory Accounting, Timeout, Cancellation

### Limits (`limits.rs`)

- **Wall clock timeout**: Configurable per-execution
- **Lua instruction budget**: Hard cap on VM instructions
- **Output bytes**: Maximum script output size
- **Script bytes**: Maximum source code size
- **Module bytes**: Maximum required module size
- **Network operations**: Count of allowed network calls
- **Filesystem operations**: Count of allowed file I/O calls
- **Lua memory bytes**: Hard cap on VM memory allocation

### Cancellation

`NseCancellationTok` supports cooperative cancellation. The Python binding
exposes `CancellationToken` for external cancellation.

## Output Conversion

The `NseRunReport` aggregates:
- Script output (content, line count, has_output flag)
- Library use reports (loaded, side effects, fallback behavior)
- Rule evaluation results (matched, exactness, summary)
- Capability events (denials, warnings, allowed operations)
- Structured evidence items (service fingerprints, version info, etc.)
- Compatibility status and fidelity assessment
- Execution statistics (elapsed time, instruction count, I/O counts)

The Python `NseReportPy` mirrors this structure with `to_dict()` and
`to_json()` serialization.

## CLI Coupling

The NSE engine is decoupled from CLI via `run_cli_with_profile()`.
The Python bindings use `NseExecutor::with_profile()` directly,
bypassing CLI output formatting.

## Python API Surface

### Functions (stable)

| Function | Description |
|----------|-------------|
| `nse_run(target, script, ...)` | Execute an NSE script (sync) |
| `async_nse_run(target, script, ...)` | Execute an NSE script (async) |
| `nse_list_libraries()` | List all registered library names |
| `nse_list_libraries_detailed()` | Library descriptors with full metadata |
| `nse_get_library_descriptor(name)` | Look up a library by name |
| `nse_list_scripts(category=...)` | List built-in scripts with metadata |
| `nse_get_script_metadata(name)` | Get metadata for a specific script |
| `nse_run_with_config(config)` | Execute with full NseConfigPy |
| `nse_validate_script(script)` | Validate script syntax without execution |

### Types (stable)

| Type | Description |
|------|-------------|
| `NseConfigPy` | Script execution configuration |
| `NseReportPy` | Execution report with output and diagnostics |
| `NseLibraryUsePy` | Per-library usage report |
| `NseRuleEvaluationPy` | Rule evaluation result |
| `NseScriptMetadataPy` | Script metadata (name, category, description) |
| `NseSandboxPolicyPy` | Sandbox configuration |
| `NseTargetContextPy` | Target host/port/service context |
| `NseLibraryDescriptorPy` | Library registry metadata |
| `NseArgumentPy` | Script argument (name, value, type) |
| `NseLibraryRegistryPy` | Library registry query interface |

### Provisional

The following are implemented but not yet part of the stable-core
operation registry:

- `NseLibraryRegistryPy` — query interface for the library registry
- `nse_validate_script()` — syntax validation without execution
- `NseArgumentPy` — structured argument representation

## Feature Gating

NSE support requires the `nse` feature flag:

```toml
[dependencies]
eggsec-nse = { path = "../eggsec-nse", features = ["nse"] }
```

For Python bindings:
```bash
cargo check -p eggsec-python --features nse
```

## Known Limitations

1. **Built-in scripts only**: The Python surface currently exposes only
   the six built-in scripts. External script loading via `script_file`
   requires profile permission.

2. **AgentSafe profile**: Default execution uses the `AgentSafe` profile.
   Other profiles (ManualPermissive, CiStrict) are not yet exposed
   through the Python API.

3. **No dynamic script discovery**: Script metadata comes from a static
   table matching `get_builtin_script()` names. Dynamic script loading
   from the filesystem is not exposed.

4. **Sandbox policy passthrough**: `NseSandboxPolicyPy` is defined but
   not yet wired into the execution path. Sandbox behavior is controlled
   by the compiled-in `SandboxConfig`.
