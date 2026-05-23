# Slapper Implementation Plan

**Date**: 2026-05-23
**Last Updated**: 2026-05-28
**Status**: ✅ ALL WAVES COMPLETED + INCOMPLETE FIXES APPLIED

## Overview

This plan consolidates action items from architecture reviews of all Slapper modules. Items are organized by priority and grouped into implementation waves for parallel execution.

---

## Wave 1: Production Safety (✅ COMPLETED 2026-05-28)

Items that prevent potential panics, data corruption, or security issues.

### 1.1 NSE - Replace std::HashMap with FxHashMap (HIGH) ✅

**File**: `crates/slapper-nse/src/public_api/api.rs`

**Status**: Completed
**Commit**: `bba924a` - "fix(nse): replace std::HashMap with FxHashMap in public_api/api.rs"
**Verification**: `cargo check -p slapper-nse && cargo test --lib -p slapper-nse && cargo clippy --lib -p slapper-nse`

**Fix**: Added `use rustc_hash::FxHashMap;` at top of file and replaced all 8 `std::collections::HashMap` with `FxHashMap`.

**Note**: This fix was confirmed incomplete during 2026-05-28 review - commit `bba924a` only updated the file header comment but did not apply the actual code changes. Fixed in subsequent commit.

### 1.2 Networking - DNS Parsing Bounds Check (MEDIUM) ✅

**File**: `crates/slapper/src/packet/parse_impl.rs:531,551`

**Status**: Completed
**Commit**: `ec76147` - "fix(networking): add DNS parsing bounds check for malformed responses"
**Verification**: `cargo check --lib -p slapper --features packet-inspection`

**Fix**: Added `new_offset >= data.len() ||` check before byte access in both questions and answers parsing loops.

### 1.3 Distributed - Worker Capabilities Mismatch (MEDIUM) ✅

**File**: `crates/slapper/src/distributed/worker.rs:115-123`

**Status**: Completed
**Commit**: `714f55a` - "fix(distributed): derive worker capabilities from TaskType enum"
**Verification**: `cargo check --lib -p slapper --features stress-testing && cargo test --lib -p slapper distributed::`

**Fix**: Created `worker_capabilities()` helper function that derives capabilities from `TaskType` enum variants. Added `Display` impl for `TaskType` in `distributed/mod.rs`.

### 1.4 AI - Knowledge Base Load Silent Failure (LOW) ✅

**File**: `crates/slapper/src/ai/waf_bypass.rs:44`

**Status**: Completed
**Commit**: `b4f5528` - "fix(ai): use unwrap_or_else with logging for knowledge base load"
**Verification**: `cargo check --lib -p slapper --features ai-integration`

**Fix**: Changed `unwrap_or_default()` to `unwrap_or_else()` with `tracing::warn` for better error visibility.

---

## Wave 2: Performance & Correctness (✅ COMPLETED 2026-05-28)

### 2.1 NSE - Additional HashMap/HashSet Replacements (MEDIUM) ✅

**Files & Locations**:

**Status**: Completed
**Commit**: `8e55044` - "fix(nse): replace HashMap/HashSet with FxHashMap/FxHashSet in libraries"

**Fixes Applied**:
- `libraries/http.rs:143` - `HashMap` → `FxHashMap` in `parse_options()`
- `libraries/datafiles.rs:31-33` - `HashMap` → `FxHashMap` in `get_services()`
- `libraries/creds.rs:102,123` - `std::collections::HashSet::new()` → `FxHashSet::default()`

**Verification**: `cargo check -p slapper-nse && cargo test --lib -p slapper-nse`

### 2.2 Distributed - CommandMessage env Field Handling (LOW) ✅

**File**: `crates/slapper/src/distributed/command.rs:146-149`

**Status**: Completed
**Commit**: `c1c169b` - "docs(distributed): clarify env field is intentionally rejected for security"

**Fix**: Added clarifying comment explaining the `env` field is intentionally rejected for security reasons (reserved for future use).

**Verification**: `cargo check --lib -p slapper --features stress-testing`

### 2.3 Recon - Replace unwrap_or_default() (MEDIUM) ✅

**File**: Multiple files in `crates/slapper/src/recon/`

**Status**: Completed
**Commit**: `c485698` - "fix(recon): replace unwrap_or_default() with explicit match and tracing"
**Verification**: `cargo check --lib -p slapper && cargo test --lib -p slapper recon::`

**Fixes Applied** (20 instances across 12 files):
| File | Changes |
|------|---------|
| `cve_lookup.rs` | references field |
| `containers.rs` | pod_name, pod_namespace |
| `email.rs` | context field |
| `js.rs` | full_match |
| `cors.rs` | 3 header extractions |
| `dependency_scan/mod.rs` | 3 file_name extractions |
| `reverse_dns.rs` | hostname_str |
| `ssl_audit.rs` | check.details |
| `cloud/storage_test.rs` | 2 async text() calls |
| `asn.rs` | hostname |
| `techdetect.rs` | async text() call |
| `threatintel.rs` | nameservers |

### 2.4 Fuzzer - Division by Zero Guard (LOW) ✅

**File**: `crates/slapper/src/fuzzer/detection/analyzer.rs:188-190`

**Status**: Completed
**Commit**: `8e55044` (included in nse-libraries-hashmap merge) - "fix(fuzzer): add defensive empty check for IQR calculation"
**Verification**: `cargo check --lib -p slapper && cargo test --lib -p slapper fuzzer::`

**Fix**: Added `if iqr_samples.is_empty() { return; }` check after slice creation to prevent division by zero.

### 2.5 Loadtest - Panic Message Imprecision (LOW) ✅

**File**: `crates/slapper/src/loadtest/metrics.rs:76`

**Status**: Completed
**Commit**: `8e55044` (included in nse-libraries-hashmap merge) - "fix(loadtest): fix imprecise panic message in metrics.rs"
**Verification**: `cargo check --lib -p slapper && cargo test --lib -p slapper loadtest::`

**Fix**: Changed panic message from "3 significant figures is invalid for hdrhistogram" to "Failed to create hdrhistogram".

---

## Wave 3: Documentation & Polish (✅ COMPLETED 2026-05-28)

### 3.1 Config - AlertChannelsConfig Validation (LOW) ✅

**File**: `crates/slapper/src/config/settings.rs`

**Status**: Completed
**Commit**: `5a5e3e3` - "fix(config): add AlertChannelsConfig validation in SlapperConfig::validate()"
**Verification**: `cargo check --lib -p slapper && cargo test --lib -p slapper config::`

**Enhancement**: Added validation for all alert channel types:
- **Webhook**: URL must start with http:// or https://
- **Email**: smtp_host, smtp_port, from, to cannot be empty
- **Slack**: webhook_url must start with http:// or https://
- **PagerDuty**: routing_key cannot be empty

### 3.2 Architecture Documentation Updates (INFO) ✅

**Status**: Completed
**Commit**: `aa1ea59` - "docs(architecture): update documentation for 2026-05-28 fixes"

**Updates Applied**:
| Module | File | Change |
|--------|------|--------|
| TUI | `architecture/tui.md` | Updated payload type count from 30 to 31 |
| Recon | `architecture/recon.md` | Clarified `secrets` module is standalone; Updated FxHashMap count from 13 to 55 |
| Networking | `architecture/networking.md` | Added DNS parsing bounds check note; Updated date to 2026-05-28 |

**Note**: CLI `-o` flag was already present in `GraphQlArgs` and `OAuthArgs` (code already correct, no doc change needed).

---

## Items With No Action Required

The following modules were reviewed and require no code changes:

| Module | Status | Notes |
|--------|--------|-------|
| WAF | ✅ Clean | No bugs found |
| TUI | ✅ Clean | No bugs found |
| Scanner | ✅ Clean | All bug fixes properly applied |
| Pipeline | ✅ Clean | All components match architecture |
| Proxy | ✅ Clean | Implementation matches architecture |
| Stress | ✅ Clean | Implementation matches architecture |
| Tool | ✅ Clean | Implementation matches architecture |

---

## Implementation Order (Historical)

### Wave 1 (4 items - Completed 2026-05-28)
| # | Module | File | Priority | Status |
|---|--------|------|----------|--------|
| 1.1 | NSE | `public_api/api.rs` - Replace 8 std::HashMap with FxHashMap | HIGH | ✅ |
| 1.2 | Networking | `packet/parse_impl.rs:531` - DNS parsing bounds check | MEDIUM | ✅ |
| 1.3 | Distributed | `worker.rs:115-123` - Worker capabilities from TaskType | MEDIUM | ✅ |
| 1.4 | AI | `waf_bypass.rs:44` - unwrap_or_else with logging | LOW | ✅ |

### Wave 2 (5 items - Completed 2026-05-28)
| # | Module | File | Priority | Status |
|---|--------|------|----------|--------|
| 2.1 | NSE | `libraries/http.rs`, `datafiles.rs`, `creds.rs` - 4 more HashMap/HashSet fixes | MEDIUM | ✅ |
| 2.2 | Distributed | `command.rs:146-149` - Document or remove env field handling | LOW | ✅ |
| 2.3 | Recon | Multiple files - Replace 20 unwrap_or_default() calls | MEDIUM | ✅ |
| 2.4 | Fuzzer | `analyzer.rs:188-190` - Add empty IQR check | LOW | ✅ |
| 2.5 | Loadtest | `metrics.rs:76` - Fix panic message | LOW | ✅ |

### Wave 3 (2 items - Completed 2026-05-28)
| # | Module | File | Priority | Status |
|---|--------|------|----------|--------|
| 3.1 | Config | `settings.rs` - Add AlertChannelsConfig validation | LOW | ✅ |
| 3.2 | Docs | Multiple architecture files - Update counts and clarify | INFO | ✅ |

---

## Verification Commands

After implementing changes, verify with:

```bash
# Library checks
cargo check --lib -p slapper
cargo check -p slapper-nse

# Run tests
cargo test --lib -p slapper
cargo test --lib -p slapper-nse

# Clippy
cargo clippy --lib -p slapper
cargo clippy --lib -p slapper-nse

# Feature-specific checks (if applicable)
cargo check --lib -p slapper --features stress-testing
cargo check --lib -p slapper --features packet-inspection
cargo check --lib -p slapper --features ai-integration
```

---

## Summary of Changes (2026-05-28)

| Module | Files Changed | Key Fixes |
|--------|--------------|-----------|
| NSE | 4 | FxHashMap replacements (12 total HashMap/HashSet) |
| Networking | 1 | DNS parsing bounds check |
| Distributed | 2 | Worker capabilities from enum, env field documentation |
| AI | 1 | Knowledge base load with proper error logging |
| Recon | 12 | 20 unwrap_or_default() replaced with explicit match |
| Fuzzer | 1 | Division by zero guard for IQR calculation |
| Loadtest | 1 | Fix imprecise panic message |
| Config | 1 | AlertChannelsConfig validation |
| Docs | 3 | Architecture doc updates for 2026-05-28 |

---

## Notes for Future Agents

1. **NSE module** (`slapper-nse/`) is a separate crate with its own `Cargo.toml`. Always use `cargo check -p slapper-nse` for validation.

2. **Distributed module** has 4 issues total: 1 worker capabilities, 1 env handling, 1 lock contention (documented, not fixing), 1 queue.rs (already fixed).

3. **Recon module** has the most instances of `unwrap_or_default()` - use grep to find all: `rg "unwrap_or_default\(\)" crates/slapper/src/recon/`

4. **FxHashMap imports** should use `use rustc_hash::{FxHashMap, FxHashSet}` at the top of files.

5. **Test code** can use `.unwrap()` and `.expect()` - the architecture guidelines about these apply only to production code.

6. **Networking DNS parsing** issue is in `packet/parse_impl.rs` not `networking/` - the packet module handles raw socket parsing.

7. **CLI `-o` flag** is already present in `GraphQlArgs` and `OAuthArgs` - code was already correct.

8. **AlertChannelsConfig validation** now enforced in `SlapperConfig::validate()` - validates URL format, required fields, etc.