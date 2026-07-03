# NSE Milestone 1 Phase 03: Script and Module Loading Hardening

## Purpose

Create a hardened, canonical script and module resolver for `eggsec-nse`.

Script loading is code loading. It must be treated as part of the security boundary, especially for agent/MCP/daemon use. This phase removes ad hoc file reads from execution paths, hardens `require` resolution, and provides structured diagnostics for script/module load behavior.

## Current Problem

Current behavior has several production-readiness gaps:

- CLI script-file execution can read a path directly and execute the returned content.
- `require` resolution constructs candidate file paths from the requested module name.
- Module names are not restricted by a clear grammar before filesystem lookup.
- Filesystem module load failures can be silently skipped.
- Script roots and module roots are not represented as a single policy object.
- Existing sandbox path logic has fallback behavior that can become string-prefix based when canonicalization fails.

These issues are manageable in manual compatibility mode but are not acceptable for automated surfaces.

## Target State

All script and module loading should flow through one resolver that enforces:

- Explicit script source kind.
- Explicit profile-derived script policy.
- Explicit profile-derived module policy.
- Strict module-name grammar.
- Canonical path validation under approved roots.
- Symlink-aware containment checks.
- File extension allowlist.
- Maximum script and module sizes.
- Structured diagnostics.
- Clear separation between built-in scripts, trusted shipped scripts, user-provided script files, and filesystem modules.

## Proposed Script Source Model

Introduce an explicit script source enum:

```rust
pub enum NseScriptSource {
    Builtin { name: String },
    TrustedRegistry { name: String },
    File { path: PathBuf },
    InlineManual { label: String, content: String },
}
```

Recommended semantics:

- `Builtin` is allowed in all profiles if the script is marked safe for that profile.
- `TrustedRegistry` is for future bundled or generated script registries.
- `File` is manual-only unless explicitly allowed by a strict trusted-root policy.
- `InlineManual` is useful for tests and possibly manual CLI, but should not be agent-safe by default.

## Proposed Resolver

Introduce `ScriptResolver` or similar:

```rust
pub struct ScriptResolver {
    script_policy: NseScriptPolicy,
    module_policy: NseModulePolicy,
    diagnostics: Vec<NseLoadDiagnostic>,
}
```

Resolver responsibilities:

- Resolve `NseScriptSource` to content plus metadata.
- Resolve `require` names to built-in modules or filesystem module content.
- Enforce script roots and module roots.
- Enforce file extension allowlists.
- Enforce size limits before returning content.
- Emit structured diagnostics for missing, blocked, oversized, malformed, or failed loads.

## Module Name Grammar

Define and test a strict module name grammar before any filesystem access.

Recommended initial grammar:

- ASCII letters, digits, `_`, `-`, and `.` are allowed.
- Name must not be empty.
- Name must not start with `.`.
- Name must not contain `..` as a path segment or traversal marker.
- Name must not contain `/`, `\`, `:`, `~`, null bytes, glob characters, shell expansion characters, or whitespace.
- Name length should be capped.

Suggested function:

```rust
pub fn validate_nse_module_name(name: &str) -> Result<NseModuleName, NseLoadError>;
```

Keep this conservative. If legitimate NSE module names require more characters later, expand only with tests.

## Path Containment Rules

Resolver path containment should use canonical paths, not string-prefix fallback.

Recommended behavior:

1. Canonicalize the configured root when the resolver is constructed.
2. For existing files, canonicalize the candidate file path.
3. Verify the canonical candidate starts with a canonical approved root using path-component semantics.
4. Reject candidates outside roots.
5. For new or missing paths, do not fall back to string-prefix acceptance. Treat as not found or blocked, depending on context.

For script/module reads, paths should already exist. Therefore missing paths do not need permissive parent fallback.

Symlink behavior:

- Symlinks inside an approved root that resolve inside the approved root may be allowed.
- Symlinks that resolve outside approved roots must be rejected.

## Implementation Steps

### Step 1: Add Resolver Types

Add a new module such as `resolver.rs`.

Types to add:

- `NseScriptSource`.
- `ResolvedNseScript`.
- `NseModuleName`.
- `ResolvedNseModule`.
- `NseLoadError`.
- `NseLoadDiagnostic`.
- `ScriptResolver`.

### Step 2: Move Built-In Script Lookup Behind Resolver

Current built-in scripts can remain simple string templates, but `get_builtin_script` should become an implementation detail behind resolver APIs.

Expected API:

```rust
resolver.resolve_script(NseScriptSource::Builtin { name })
```

This allows built-ins to be checked against profile metadata later.

### Step 3: Migrate CLI Script File Loading

Remove direct script file reads from the CLI run path.

Instead:

1. CLI creates `NseScriptSource::File { path }` when `--script-file` is used.
2. CLI/handler selects a profile.
3. Resolver checks whether the profile allows file scripts.
4. Resolver reads content only after canonical validation and size checks.

If a script file is rejected, return a structured error explaining whether it was blocked by profile, outside allowed roots, missing, oversized, or invalid.

### Step 4: Migrate `require` Resolution

Update Lua `require` handling so filesystem lookup goes through the resolver.

Required behavior:

- Validate module name before lookup.
- Check built-in registered modules first.
- Check approved module roots only if profile allows filesystem modules.
- Try approved extensions such as `.lua` and `.nse` only after validation.
- Return structured load errors instead of silently swallowing filesystem read/eval errors.
- Cache only successful module loads unless there is a reason to cache misses.

### Step 5: Add Diagnostics to Execution Output

Resolver diagnostics should be available to the executor and eventually to `NseRunReport`.

For this phase, expose diagnostics through:

- returned error type.
- verbose CLI output.
- tracing/logging.
- an accessor on executor or run outcome.

Do not lose detail by converting all load errors to generic Lua runtime errors.

### Step 6: Enforce Size Limits

Use `NseExecutionLimits` or script/module policy limits to enforce:

- max script file bytes.
- max inline script bytes.
- max module file bytes.

Reject oversized content before Lua evaluation.

### Step 7: Add Tests

Required tests:

- Built-in script resolves.
- Missing built-in reports not found.
- Script file outside allowed root is rejected.
- Script file inside allowed root resolves.
- Symlink escape is rejected.
- Absolute path is rejected when profile disallows script files.
- `../` traversal in module name is rejected before filesystem lookup.
- Module name with slash is rejected before filesystem lookup.
- Module name with Windows-style backslash or drive prefix is rejected.
- Oversized script is rejected.
- Oversized module is rejected.
- Filesystem module load error is reported, not silently skipped.
- Manual profile can allow script files when configured.
- Agent-safe profile rejects arbitrary script files.

Use temporary directories for resolver tests. Avoid relying on host Nmap paths in tests.

## Integration With Phase 2 Profiles

The resolver should consume the profile's `NseScriptPolicy` and `NseModulePolicy`.

Profile-specific expectations:

- `AgentSafe`: built-ins only unless explicitly configured otherwise.
- `CiSafe`: fixture root only.
- `ManualStrict`: approved roots only.
- `ManualPermissive`: may allow conventional Nmap roots and user script files, but should still validate paths and module names.
- `CompatibilityLab`: may allow broader behavior but must be explicit.

Even permissive profiles should reject traversal module names. Manual permissiveness does not require accepting ambiguous path injection.

## Documentation Updates

Document:

- Supported script source kinds.
- How `--script-file` is handled.
- How `require` paths are resolved.
- Which profiles permit filesystem scripts/modules.
- Why conventional Nmap script paths are not used in agent-safe mode.
- How load errors are reported.

## Verification Commands

Run at least:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse
make test-nse
```

Add resolver unit tests that do not require the `nse` feature if possible, then add feature-gated integration tests for actual Lua `require` behavior.

## Acceptance Criteria

This phase is complete when:

- All script-file execution goes through the resolver.
- Lua `require` filesystem loading goes through the resolver.
- Module names are validated before filesystem access.
- Path containment uses canonical paths and approved roots.
- String-prefix fallback is not used to authorize script/module reads.
- Symlink escape is rejected.
- Oversized scripts/modules are rejected before Lua evaluation.
- Load failures produce structured diagnostics.
- Agent-safe profile cannot load arbitrary script files or ambient Nmap paths.
- Tests cover traversal, symlink, invalid names, size limits, profile blocks, and successful allowed loads.

## Reviewer Checklist

- Search for direct `std::fs::read_to_string` use in NSE execution paths.
- Search for `join(format!("{}...", name))` style require path construction outside resolver.
- Verify invalid module names fail before path construction.
- Verify canonical roots are established before candidate validation.
- Verify tests do not depend on host-specific `/usr/share/nmap` or user home directories.
- Verify diagnostics preserve blocked-vs-missing-vs-invalid distinctions.
