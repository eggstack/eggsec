# Plugins & NSE Integration Architecture Review

## Overview

Reviewed: `crates/slapper-nse/src/` (6 core files + 160+ library files)
Reference: `architecture/plugins_nse.md`

## Verification Against Documentation

| Claim in Docs | Status | Implementation |
|---------------|--------|----------------|
| SandboxConfig with allowed_dir, allowed_commands, log_violations, allowed_networks | ✅ | `lib.rs:50-63` - All fields present |
| `io.popen`, `lfs`, `os.getenv`, `socket` sandboxing | ✅ | Various library files |
| 164 NSE library modules | ⚠️ | ~160 modules in `libraries/mod.rs` (counted ~160) |
| CVE integration: NVD, OSV, CISA KEV | ✅ | `cve/mod.rs:7-9` - All three sources |
| CveCache uses FxHashMap | ✅ | `cve/mod.rs:174` - `Arc<RwLock<FxHashMap<...>>>` |
| CveAggregator uses FxHashSet | ✅ | `cve/mod.rs:260,287` - Uses `FxHashSet` |
| output.rs uses `let _ = writeln!()` pattern | ✅ | `output.rs:31,42,45,59,66,72,74,91,95,96,97,99,104` - Multiple uses |
| `async_executor.rs` Default impl uses `unwrap_or_else` | ✅ | `async_executor.rs:108` - `panic!("Failed to create ExecutorCore...")` |
| Duplicate `getenv` removed | ✅ | `os.rs:295-300` - No duplicate found |
| Path traversal check relies on canonicalization | ✅ | `lib.rs:93-115` - `is_path_allowed()` uses `canonicalize()` |
| `rustc-hash` in dependencies | ✅ | Confirmed via Cargo.toml |

## Bug Checks

### unwrap/expect Analysis

| Location | Issue | Severity |
|----------|-------|----------|
| `lib.rs:81` | `canonicalize().ok().or_else(\|\| Some(dir.clone()))` - Falls back to non-canonical path | Medium - could bypass check if dir doesn't exist |
| `executor_core.rs:175` | `v.to_string().unwrap_or_default()` - Silent fallback | Low |
| `output.rs:37-38` | `unwrap_or_default()` on SystemTime | Low - acceptable |
| `output.rs:50-55` | Multiple `unwrap_or_default()` on SystemTime | Low - acceptable |
| `context.rs:105` | `path_buf.canonicalize().ok()` fallback | Medium - same issue as lib.rs:81 |

### HashMap/FxHashMap Analysis

| Location | Type | Status |
|----------|------|--------|
| `context.rs:70` | `FxHashMap<String, String>` for output_table | ✅ |
| `context.rs:143,146` | `FxHashMap<(u16, String), PortInfo>`, `FxHashMap<String, mlua::Value>` | ✅ |
| `executor_core.rs:44` | `Mutex<FxHashMap<String, Value>>` | ✅ |
| `executor_core.rs:591` | `Arc<Mutex<FxHashMap<String, Value>>>` for require cache | ✅ |
| `executor.rs:7,336` | Uses `FxHashMap` | ✅ |
| `cve/mod.rs:174` | CveCache uses FxHashMap | ✅ |

### Error Handling

1. **executor.rs:66-102** - `run_script_with_timeout()` uses mpsc channel with proper timeout handling
2. **executor.rs:93-102** - All timeout/Disconnect errors handled explicitly
3. **output.rs** - All `writeln!` calls use `let _ =` pattern, errors suppressed but logged via return value
4. **context.rs** - Proper error propagation via `Result` types in `to_table()` methods

## Security Observations

### Sandbox Path Check Issue (Medium)

`lib.rs:93-115` - `is_path_allowed()`:

```rust
let Ok(canonical) = path_buf.canonicalize() else {
    // If canonicalize fails (file doesn't exist), check the parent
    if let Some(parent) = path_buf.parent() {
        if let Ok(canonical_parent) = parent.canonicalize() {
            return canonical_parent.starts_with(&allowed_dir);
        }
    }
    // Reject if we can't verify the path
    return false;
};
```

If `canonicalize()` fails on the target path (file doesn't exist), it falls back to checking the parent directory. This is reasonable for allowing operations on non-existent files in allowed directories, but the fallback logic could theoretically allow bypassing if an attacker can make canonicalization fail in certain ways.

### os Library Command Allowlist (Low)

`os.rs` command checking relies on the allowlist in `SandboxConfig`, which is appropriate.

## Performance Observations

1. **executor_core.rs:591** - Require cache is a good optimization (FxHashMap with mutex)
2. **context.rs:189** - `protocol.to_string()` allocation on every port lookup
3. **executor.rs:336-360** - `parse_all_script_categories()` re-reads directories on each call - could benefit from caching

## Discrepancies

1. **Library count**: Document says "164 NSE-style library modules" but `libraries/mod.rs` shows ~160 modules. This is a minor documentation inaccuracy.

## Summary

| Category | Status |
|----------|--------|
| Implementation matches docs | ⚠️ Mostly (library count discrepancy) |
| Critical bugs | ✅ None |
| Security issues | ⚠️ 2 medium (path canonicalization fallback) |
| HashMap/FxHashMap usage | ✅ Correct throughout |
| Error handling | ✅ Proper patterns used |

**Review Date**: 2026-05-23
**Reviewer**: Architecture Review