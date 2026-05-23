# Architecture Overview Review

Review date: 2026-05-23
Document: `architecture/overview.md`
Codebase: `/Users/davidbowman/projects/slapper`

---

## Verified Claims

### Accurate Counts

| Claim | Document | Implementation | Status |
|-------|----------|----------------|--------|
| Commands enum | "35+ variants" | 36 variants (37 minus AuthTest which is always present) | ✅ Accurate |
| Payload types | "31" | 31 PayloadType variants in `fuzzer/payloads/mod.rs:38-70` | ✅ Accurate |
| WAF products | "34" | 34 `signatures.insert` calls in `waf/data/patterns.rs` | ✅ Accurate |
| TUI tabs | "29" | 29 Tab variants in `tui/tabs/mod.rs:84-114` | ✅ Accurate |
| Pipeline profiles | "11" | 11 ScanProfile variants in `pipeline/stage.rs:31-93` | ✅ Accurate |
| Workspace crates | "4" | slapper, slapper-plugin, slapper-nse, slapper-ruby | ✅ Accurate |

### Verified Design Patterns

| Claim | Implementation | Status |
|-------|----------------|--------|
| Severity enum canonical | `types.rs:11-23` - single definition, re-exported | ✅ Verified |
| TabError structured | `tui/app/tab_error.rs:4-12` with 7 categories | ✅ Verified |
| FxHashMap usage | 204 matches across codebase for performance | ✅ Verified |
| Session persistence condition | `pipeline/executor.rs:118` - only `*.session` or `*.session.json` | ✅ Verified |
| Scope private IP blocking | `config/scope.rs:51-63` - TargetScope::parse() | ✅ Verified |
| SensitiveString type | `types.rs:128-248` - zeroize, constant-time eq | ✅ Verified |
| Builder pattern | Pipeline::from_args(), FuzzEngine::new(args).run(), etc. | ✅ Verified |
| SlapperError via thiserror | `error/mod.rs:43+` | ✅ Verified |

---

## Discrepancies

### 1. SecurityTool Trait Incomplete Documentation

**Location**: `architecture/overview.md:215-223` vs `tool/traits.rs:144-205`

**Issue**: The documented `SecurityTool` trait shows only 3 methods:
```rust
pub trait SecurityTool: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(&self, target: &Target, args: Value) -> Result<Value>;
    fn capabilities(&self) -> Vec<Capability>;
}
```

**Actual trait** has 9 methods:
- `id()`, `name()`, `category()`, `description()`
- `async fn execute(request: ToolRequest) -> ToolResult<ToolResponse>`
- `fn validate()`, `fn capabilities()`, `fn supported_protocols()`, `fn output_schema()`

**Impact**: Medium - Documentation would mislead an integrator trying to implement the trait.

---

### 2. ToolRegistry Uses std::collections::HashMap

**Location**: `tool/registry.rs:2,24,31`

**Issue**: Despite the design principle stating "Uses `rustc_hash::FxHashMap`/`FxHashSet` instead of std collections for performance" (`architecture/overview.md:161`), the `ToolRegistry` uses `std::collections::HashMap`:

```rust
use std::collections::HashMap;  // Line 2
tools: Arc<RwLock<HashMap<String, Arc<dyn SecurityTool>>>>,  // Line 24
```

**Impact**: Medium - Performance anti-pattern in a critical path component (tool abstraction layer).

---

### 3. ScanProfile Is Not an Enum

**Location**: `architecture/overview.md:334` vs `config/scan.rs:179-185`

**Issue**: The Quick Reference table says "Pipeline profiles: 11" and `Stage::from_profile()` is documented as taking a `ScanProfile` enum. However, `ScanProfile` in `config/scan.rs:179-185` is a `struct`, not an enum:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanProfile {
    pub name: String,
    pub http: Option<super::http::HttpConfig>,
    pub scan: Option<ScanConfig>,
    pub fuzz: Option<FuzzProfile>,
}
```

The 11 profiles are hardcoded in `Stage::from_profile()` which takes `ScanProfile` from `cli::ScanProfile` (an enum).

**Impact**: Low - Terminology confusion but not a functional bug.

---

### 4. Module Count Discrepancy

**Location**: `architecture/overview.md:328`

**Issue**: Document says "41 modules in `crates/slapper/src/`". Counting directories shows:
- Directories: ~42 (agent, ai, auth, browser, cli, commands, compliance, config, container, distributed, error, fuzzer, hunt, integrations, loadtest, logging, notify, output, packet, pipeline, proxy, recon, scanner, storage, stress, supply_chain, tool, tui, utils, vuln, waf, websocket, wireless, workflow)
- Plus files: `constants.rs`, `lib.rs`, `main.rs`, `macros.rs`, `nse_tool.rs`, `types.rs`, `generated/` (directory)

Actual module count is closer to 43 directories.

**Impact**: Low - Minor documentation inaccuracy.

---

## Bugs Found

### 1. ToolRegistry HashMap Performance Bug

**Priority**: Medium

**File**: `tool/registry.rs:2,24,31`

**Description**: `ToolRegistry` uses `std::collections::HashMap` instead of `FxHashMap`. In high-throughput tool lookup scenarios, this could cause performance degradation.

**Fix**: Replace `HashMap` with `FxHashMap`:
```rust
use rustc_hash::FxHashMap;
// ...
tools: Arc<RwLock<FxHashMap<String, Arc<dyn SecurityTool>>>>,
```

---

## Improvement Opportunities

### 1. Document SecurityTool Trait Completely

**Priority**: Medium

**Location**: `architecture/overview.md:215-225`

**Fix**: Update the trait documentation to reflect all 9 methods. Consider showing the actual trait signature from `tool/traits.rs:144-205`.

---

### 2. Convert ToolRegistry to FxHashMap

**Priority**: Medium

**Location**: `tool/registry.rs:1-224`

**Fix**: Replace std `HashMap` with `FxHashMap` for consistency with performance guidelines and to avoid `parking_lot::RwLock` + `HashMap` combination (RwLock already provides internal locking; the HashMap inside doesn't need to be concurrent-safe).

---

### 3. Clarify ScanProfile vs ScanProfileEnum

**Priority**: Low

**Location**: `architecture/overview.md` and `architecture/pipeline.md`

**Fix**: The documentation refers to `ScanProfile` as if it were the enum driving `Stage::from_profile()`. Clarify that `cli::ScanProfile` is the enum (11 variants), while `config::scan::ScanProfile` is a configuration struct for custom profiles.

---

### 4. Update Module Count

**Priority**: Low

**Location**: `architecture/overview.md:328`

**Fix**: Update "41 modules" to "43 modules" or clarify counting methodology.

---

### 5. Add Missing Architecture Docs Indicator

**Priority**: Low

**Location**: `architecture/overview.md:302-323`

**Fix**: The section "Modules Without Detailed Docs" is comprehensive. Consider adding a table indicating which modules have AGENTS.override.md files for AI agent guidance, since the current table only mentions "candidates for future deep dives."

---

## Priority Summary

| Finding | Type | Priority | Effort |
|---------|------|----------|--------|
| SecurityTool trait documentation incomplete | Discrepancy | Medium | Low |
| ToolRegistry uses std HashMap instead of FxHashMap | Bug | Medium | Low |
| ScanProfile terminology confusion | Discrepancy | Low | Low |
| Module count inaccurate (41 vs 43) | Discrepancy | Low | Trivial |
| Update SecurityTool trait in docs | Improvement | Medium | Low |
| Convert ToolRegistry to FxHashMap | Improvement | Medium | Low |

---

## Summary

The architecture overview document is **largely accurate** with only minor discrepancies. Key metrics (31 payload types, 34 WAF products, 29 TUI tabs, 36 commands) all match implementation. The main issues are:

1. **Documentation gap**: SecurityTool trait is incompletely documented
2. **Performance bug**: ToolRegistry uses std HashMap contrary to stated design principles
3. **Minor terminology issues**: ScanProfile struct vs enum confusion

Overall the architecture is well-documented and the code matches the stated design principles except for the ToolRegistry HashMap issue.