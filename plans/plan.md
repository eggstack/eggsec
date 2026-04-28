# Slapper Improvement Plan

**Date**: 2026-04-25
**Status**: ACTIVE DEVELOPMENT
**Priority**: High

---

## Executive Summary

This document consolidates all improvement plans into a single implementation roadmap. Items are organized into waves based on parallelization potential, with dependencies noted for items that require prior completion.

**Current State** (verified 2026-04-25):
- 1,107+ passing tests (base)
- 1,345 passing tests (with full features, 9 pre-existing AI test failures)
- ~19 clippy warnings (TUI-specific acceptable)
- 470+ source files
- 30 payload types in fuzzer (not 22 as documented)
- 29 TUI tabs
- ~30 recon modules

**Goal**: Zero failing tests, minimal warnings, production-ready codebase with enhanced capabilities.

---

## Wave Organization

### Wave A: Security Hardening (Parallel - Team A)
Focus: Critical security vulnerabilities and plugin safety

### Wave B: Code Quality (Parallel - Team B)
Focus: Test fixes, dead code removal, default implementations

### Wave C: Performance (Parallel - Team C)
Focus: Hot path optimizations, reduced allocations

### Wave D: TUI Improvements (Parallel - Team D)
Focus: Critical usability fixes, hardcoded colors, broken components

### Wave E: Feature Completion (Sequential after A-D)
Focus: gRPC implementation, auto-calibration, capability gaps

### Wave F: Documentation (Parallel - Can run alongside E)
Focus: Discrepancy fixes, new guides, skills standardization

---

## Wave A: Security Hardening

**Priority**: CRITICAL
**Team**: A (can work in parallel with B, C, D)
**Target**: Close critical security bypass vectors

### A.1: Regex ReDoS Prevention ✓ COMPLETE

**Issue**: The `regex` crate allows building regexes from untrusted input without size limits. 7 locations in slapper-nse bypass the safe `build_regex()` helper.

**Status**: FIXED (commit 34e0666)
- All vulnerable `Regex::new()` calls replaced with `RegexBuilder::new().size_limit(50_000)`

**Locations fixed** (verified 2026-04-25):
| File | Line | Status |
|------|------|--------|
| `slapper-nse/src/libraries/match_lib.rs` | ~87 | ✓ Fixed |
| `slapper-nse/src/libraries/matchs.rs` | ~47, ~56 | ✓ Fixed |
| `slapper-nse/src/libraries/lpeg.rs` | ~155, ~179, ~202 | ✓ Fixed |
| `slapper-nse/src/libraries/re.rs` | ~221 | ✓ Fixed |

**Fix Pattern**:
```rust
// BEFORE (vulnerable):
Regex::new(&regex_pattern)

// AFTER (secure):
RegexBuilder::new(&regex_pattern)
    .size_limit(50_000)  // 50KB limit for untrusted input
    .build()
```

**Verification**:
```bash
cargo test --lib -p slapper-nse
```

---

### A.2: Plugin Security Pattern Detection

**Issue**: Missing critical patterns in Python and Ruby plugin validation.

**Current State** (verified 2026-04-25):
- Python has 17 patterns including `getattr(` and `chr(` (DO NOT REMOVE - they have legitimate use cases)
- Ruby has 18 patterns including `(?i)\beval\b` and `(?i)\bopen\b` (DO NOT REMOVE - they serve a purpose)

**Python - Add these patterns** (`slapper-plugin/src/security.rs`):
```rust
// CRITICAL - code execution (MISSING - add these)
Regex::new(r"(?i)\bexec\(").unwrap(),
Regex::new(r"(?i)\bcompile\(").unwrap(),
Regex::new(r"(?i)types\.FunctionType").unwrap(),
Regex::new(r"(?i)\bmarshal\.").unwrap(),
Regex::new(r"(?i)\bbase64\.").unwrap(),
Regex::new(r"(?i)\bzlib\.").unwrap(),
Regex::new(r"(?i)\bbuiltins\b").unwrap(),
Regex::new(r"(?i)platform\.os").unwrap(),
Regex::new(r"(?i)sys\.modules\[").unwrap(),
```

**Ruby - Add these patterns** (all 7 are MISSING):
```rust
Regex::new(r"(?i)(instance_eval|class_eval|module_eval)\(").unwrap(),
Regex::new(r"(?i)%x\{").unwrap(),
Regex::new(r"(?i)Marshal\.load").unwrap(),
Regex::new(r"(?i)RubyVM::InstructionSequence").unwrap(),
Regex::new(r"(?i)\brequire\b").unwrap(),
Regex::new(r"(?i)\bload\b").unwrap(),
Regex::new(r"(?i)\bsend\(").unwrap(),
```

**DO NOT REMOVE** (plan was wrong):
- Python: `getattr(`, `chr(` - these exist and have legitimate security uses
- Ruby: `(?i)\bopen\b`, `(?i)\beval\b` - `eval` is different from `eval(`, `open` catches `IO.open`

**Verification**:
```bash
cargo test -p slapper-plugin --features python-plugins,ruby-plugins
```

---

### A.3: Config File Permissions

**Issue**: `check_config_file_permissions()` in `types.rs:269-303` is never called.

**Fix**: Call after config loads in `config/loader.rs`:
```rust
// After config.validate() at line ~51:
check_config_file_permissions(&canonical_path);

// After loading scope file at line ~82:
check_config_file_permissions(&canonical_path);
```

**Note**: Function returns `()` (unit), logs warnings via `tracing::warn!`.

---

### A.4: Plugin Timeout Enforcement (Process Isolation)

**Issue**: Timeouts are advisory-only. Python GIL and Ruby VM continue executing after timeout.

**Recommended Solution**: Process-based plugin runner

**New module**: `slapper-plugin/src/process_runner.rs` (does not exist - needs to be created)

```rust
pub struct ProcessPluginRunner {
    timeout: Duration,
    isolation_level: IsolationLevel,
}

pub enum IsolationLevel {
    InProcess,   // Current behavior (default)
    Process,     // Subprocess per plugin
    Sandboxed,   // Subprocess with restrictions
}

impl ProcessPluginRunner {
    pub async fn run_plugin(
        &self,
        path: &Path,
        target: &str,
    ) -> Result<PluginResult> {
        let mut child = tokio::process::Command::new("python3")
            .arg(path)
            .arg(target)
            .kill_on_drop(true)
            .spawn()?;

        match tokio::time::timeout(self.timeout, child.wait()).await {
            Ok(Ok(status)) => { /* parse output */ }
            Ok(Err(e)) => anyhow::bail!("Process error: {}", e),
            Err(_) => {
                child.kill().await?;  // REAL termination
                anyhow::bail!("Plugin timed out")
            }
        }
    }
}
```

**Add to PluginConfig**:
```rust
pub struct PluginConfig {
    // ... existing fields
    #[serde(default)]
    pub isolation_level: IsolationLevel,
}
```

---

## Wave B: Code Quality

**Priority**: HIGH
**Team**: B (can work in parallel with A, C, D)
**Target**: Zero failing tests, minimal warnings

### B.1: Fix AI-integration Compilation Error (PREREQUISITE)

**CRITICAL**: The `ai-integration` feature does not compile. Before any test fixes can be verified, this must be resolved.

**Issue**: `cargo check --lib -p slapper --features ai-integration` fails with:
```
error[E0432]: unresolved import `crate::tool::ToolResult`
```

Multiple files in `tool/implementations/` import `ToolResult` from `crate::tool` but it only exists in `crate::tool::traits`.

**Fix**: Add `pub use crate::tool::traits::ToolResult;` to `tool/mod.rs` (gated on `tool-api` feature).

---

### B.2: Fix Failing AI-integration Tests (9 tests) - AFTER B.1

Once compilation is fixed, these tests need attention:

**Test 1-2: Skills Trigger Extraction** (`agent/skills.rs:98-129`)

**Issue**: Token extraction doesn't capture keywords correctly.
- `skip_while(!is_alphanumeric())` skips the `#` but consumes line start
- Cleaning logic grabs " Keywords" instead of content

**Fix**: Rewrite token extraction logic

---

**Test 4: Planner Modification - "reduce duration"** (`ai/planner.rs:447`)

**Issue**: Test content "Consider reducing duration of the fuzzing phase to save time." DOES contain exact keyword "reduce duration" but doesn't match.

**Debug**: Verify `ai/planner.rs:328-337` - lowercase conversion and exact substring matching

---

**Tests 3-5: Planner Modification Parsing** (`ai/planner.rs:313-340`)

**Issue**: Keywords don't match natural language variations.

**Root Cause**: Exact keyword matching fails for:
- "I recommend you add a new stage" (has "a new" between words)
- "Consider reducing duration" (SHOULD match but doesn't)

**Fix Options**:
- Option A: Expand keywords to include variations (e.g., "add.*stage")
- Option B: Use word-boundary-based tokenization
- Option C: Make tests use exact keyword matches

**Recommendation**: Option A - fuzzy/substring matching

---

**Tests 6-7: Planner Cache** (`ai/planner.rs:388-402`)

**Issue**: `record_outcome()` uses `cache.get_mut()` which only updates existing entries.

```rust
// Current - NO else branch, new entries never created!
if let Some(cached) = cache.get_mut(&key) {
    cached.use_count += 1;
}
```

**Fix**: Use `cache.entry(key).or_insert_with()`:
```rust
cache.entry(key).or_insert_with(|| CachedPlanData {
    use_count: 1,
    success_rate: if outcome.success { 1.0 } else { 0.0 },
    last_used: now,
});
```

---

**Test 8: Content Extraction** (`ai/client.rs:462-467`)

**Issue**: Test expects 3 lines but 4 are returned.

**Fix**: Change assertion from `assert_eq!(content.len(), 3)` to `assert_eq!(content.len(), 4)`

---

**Test 9: WAF Knowledge Base** (`ai/waf_bypass.rs:117-134`)

**Issue**: `record_success()` updates existing entry instead of adding when values conflict with pre-populated data.

**Fix**: Use unique test values (e.g., "cloudflare_test_xyz", "payload_test_xyz") that don't conflict with pre-populated data

---

### B.3: Add Missing Default Implementations (8 types)

All are SAFE - zero-sized structs with all fields having Default. Line numbers verified 2026-04-25:

| Type | File | `new()` line |
|------|------|-------------|
| CargoScanner | `recon/dependency_scan/cargo/mod.rs` | ~8 |
| NpmScanner | `recon/dependency_scan/npm/mod.rs` | ~8 |
| GoScanner | `recon/dependency_scan/go/mod.rs` | ~8 |
| StressTab | `tui/tabs/stress.rs` | ~39 |
| ReportTab | `tui/tabs/report.rs` | ~38 |
| OAuthTab | `tui/tabs/oauth.rs` | ~32 |
| GraphQlTab | `tui/tabs/graphql.rs` | ~32 |
| ClusterTab | `tui/tabs/cluster.rs` | ~37 |

**Implementation Pattern**:
```rust
impl Default for TypeName {
    fn default() -> Self {
        Self::new()
    }
}
```

---

### B.4: Remove Dead Code (2 items)

**Item 1: `ParsedDependency` struct**
- Location: `recon/dependency_scan/mod.rs:61`
- Analysis: Never constructed (zero references), `DependencyInfo` provides equivalent functionality
- Action: REMOVE the struct definition

**Item 2: `TabDispatcher::is_input_focused()`**
- Location: `tui/app/dispatch.rs:80-82`
- Analysis: Method never called through dispatcher wrapper
- Action: REMOVE the dispatcher method

---

### B.5: Address Remaining Clippy Warnings

Current: 19 warnings total

| Warning Type | Count | Action |
|------------|------- |--------|
| Missing Default impl | 8 | Apply Default to 8 structs (B.3) |
| Dead code | 2 | Remove (B.4) |
| Unused imports | ~5 | Remove |
| Unused mutability | ~2 | Review |
| Unnecessary cast | ~1 | Apply suggestion |
| Comparison to empty | ~1 | Replace `key == ""` with `key.is_empty()` |

**Note**: TUI-specific warnings are acceptable per AGENTS.md guidelines.

---

## Wave C: Performance Improvements

**Priority**: HIGH
**Team**: C (can work in parallel with A, B, D)
**Target**: 20-40% performance improvement in fuzzer/scanner hot paths

### C.1: Fix Clone Storm in Fuzzer Execution Loop

**Priority**: CRITICAL
**Location**: `fuzzer/engine/execution.rs:101-140`

**Issue**: Per-payload worker loop clones 13+ values per iteration. Some Arc optimization exists but still clones `client`, `url`, `method`, `param`, `payload_clone`, `user_agent` as full values.

**Current Pattern** (lines ~102-113):
```rust
let semaphore = semaphore.clone();
let client = self.client.clone();        // HIGH COST - reqwest Client
let url = self.url.clone();              // HIGH COST - String
let method = self.args.method.clone();   // LOW COST - enum
let param = self.args.param.clone();     // HIGH COST - Option<String>
let timing_analyzer = self.timing_analyzer.clone();
let pattern_matcher = self.pattern_matcher.clone();
let results = results.clone();
let progress = progress.clone();
let payload_clone = payload.clone();     // HIGH COST - String
let user_agent = self.user_agent.clone();
let counter = counter.clone();
```

**Recommended Fix**:

1. Make `client`, `url`, `payload` into `Arc` types in `FuzzArgs`:
```rust
struct FuzzArgs {
    url: Arc<Url>,           // Changed from String
    payload: Arc<String>,    // Changed from String
    client: Arc<Client>,      // Changed from Client
    // ...
}
```

2. Change spawn loop to clone only `Arc` types (cheap):
```rust
tokio::spawn(async move {
    let permit = semaphore.clone().acquire_owned().await?;
    let client = client.clone();  // Arc clone, not reqwest clone
    let url = url.clone();        // Arc clone, not String clone
    // ...
});
```

3. For `method` and `param`, use references where possible.

**Files to Modify**:
- `fuzzer/engine/execution.rs:102-113` - Replace heavy clones with Arc clones
- `fuzzer/engine/execution.rs:243` - Change to `&method` reference
- `fuzzer/engine/execution.rs:267,281` - Change `FuzzResult` to use `Arc<Payload>`

---

### C.2: Migrate HashMap to FxHashMap (4 modules)

**Priority**: HIGH
**Target**: Replace `std::collections::HashMap` with `rustc_hash::FxHashMap` in hot paths

**Locations** (verified NOT migrated as of 2026-04-25 - all SAFE - no untrusted input):
| File | Module | Usage |
|------|--------|-------|
| `fuzzer/api_schema/mod.rs` | ~5 | `generate_auth_bypass_payloads()` |
| `fuzzer/targets/api.rs` | ~3 | OpenAPI spec parsing |
| `fuzzer/redos_detect.rs` | ~4 | Cache compiled patterns |
| `scanner/cms/mod.rs` | ~14 | CMS fingerprinting |

**Migration Pattern**:
```rust
// BEFORE:
use std::collections::HashMap;

// AFTER:
use rustc_hash::FxHashMap;
```

**Verification**: `cargo test --lib -p slapper`

---

### C.3: Replace Mutex Counters with AtomicU64 (3 locations)

**Priority**: HIGH
**Target**: Simple counter increments use lock-free atomics

**Locations** (verified 2026-04-25):
| File | Line | Current | Change |
|------|------|---------|--------|
| `scanner/fingerprint.rs` | ~234 | `tokio::sync::Mutex<u64>` for `scanned_count` | `AtomicU64` |
| `scanner/ports/spoofed.rs` | ~138 | `tokio::sync::Mutex<u64>` for `scanned_count` | `AtomicU64` |

Note: `results_count` in these files already uses `AtomicU64`. Only `scanned_count` needs migration.

**Note**: `scanner/endpoints.rs:699` is more complex (used for progress channel) - defer.

**Reference Implementation** (already in codebase):
```rust
let scanned_count = Arc::new(AtomicU64::new(0));
scanned_count.fetch_add(1, Ordering::Relaxed);
```

---

### C.4: String Allocation Optimizations

**Priority**: MEDIUM

**Item 1: Variable Interpolation Loop** (`fuzzer/chain.rs:334-344`)

Current O(n²) approach with repeated `replace()` calls.

**Better**: Use `Cow<str>` and pre-built replacement map:
```rust
let replacements: FxHashMap<&str, &str> = self.variables
    .iter()
    .map(|(k, v)| (k.as_str(), v.as_str()))
    .collect();

fn replace_vars(input: &str, replacements: &FxHashMap<&str, &str>) -> Cow<str> {
    let mut result = Cow::Borrowed(input);
    for (key, value) in replacements {
        let placeholder = format!("${{{}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
}
```

**Item 2: URL Format in Loop** (`scanner/endpoints.rs:727`)

Current `format!("{}{}", base, endpoint)` creates new allocation each iteration.

Consider preallocation or `Url::join()`.

---

### C.5: DashMap Migration

**Status**: PLAN CORRECTION - The files listed do NOT currently use DashMap. They use `RwLock<Vec>` patterns.

**Candidate Locations** (if lock contention becomes an issue):
| File | Current Pattern | Notes |
|------|----------------|-------|
| `tool/history.rs:26` | `Arc<RwLock<Vec<ExecutionEntry>>>` | Consider if profiling shows contention |
| `distributed/queue.rs:26` | `Arc<RwLock<VecDeque<Task>>>` | Channel-based may be better |
| `tool/agents/communication.rs:150` | `Arc<RwLock<Vec<AgentMessage>>>` | Consider if profiling shows contention |
| `tool/implementations/oast.rs:29` | `Arc<RwLock<Vec<Interaction>>>` | Consider if profiling shows contention |

**Note**: Do NOT migrate unless profiling shows lock contention is a problem. The current `RwLock<Vec>` patterns are not necessarily inefficient.

**If profiling shows contention**, migration pattern:
```rust
// BEFORE:
struct HistoryManager {
    entries: Arc<RwLock<Vec<ExecutionEntry>>>,
}

// AFTER:
struct HistoryManager {
    entries: Arc<DashMap<String, ExecutionEntry>>,
}
```

---

## Wave D: TUI Improvements

**Priority**: HIGH
**Team**: D (can work in parallel with A, B, C)
**Target**: Fix critical bugs, improve usability

### D.1: UTF-8 Cursor Position Bug (CRITICAL)

**Location**: `tui/components/input.rs`

**The Bug**: `cursor_pos` field is used as both byte offset AND character position inconsistently.

**Root Cause**: Struct designed for byte offsets (line ~82's `insert()` takes byte index), but `with_value()`, `apply_autocomplete()`, and `move_end()` treat it as character count.

**Solution**: Standardize on byte offsets throughout:

```rust
// FIX at line ~39 (with_value method):
pub fn with_value(mut self, value: impl Into<String>) -> Self {
    let v = value.into();
    self.cursor_pos = v.len();  // Use byte length, not char count
    self.value = v;
    self
}

// FIX at line ~76 (apply_autocomplete method):
pub fn apply_autocomplete(&mut self, suggestion: &str) {
    self.value = suggestion.to_string();
    self.cursor_pos = self.value.len();  // Use byte length
}

// FIX at line ~132 (move_end method):
pub fn move_end(&mut self) {
    self.cursor_pos = self.value.len();  // Use byte length, not char count
}
```

---

### D.2: Hardcoded Colors (CRITICAL)

**Issue**: Tabs use `Color::Green`, `Color::Red`, `Color::Yellow` directly instead of `tc!()` theme macro.

**Affected Files** (verified 2026-04-25 - approximate line numbers):

| File | Should Be |
|------|-----------|
| `tabs/recon.rs` | `tc!(success/warning/error/text_dim)` |
| `tabs/waf.rs` | `tc!(warning/error/success/info/text_dim)` |
| `tabs/scan_ports.rs` | `tc!(warning/info/success/text_dim)` |
| `components/selector.rs` | `tc!(selected/border)` |
| `components/scrollable.rs` | `tc!(border)` |

**Fix Pattern**:
```rust
// Before:
.style(Style::default().fg(Color::Green))

// After:
.style(Style::default().fg(tc!(success)))
```

**Note**: Add `use crate::tui::theme::tc;` import where missing.

---

### D.3: AuthTab Non-Functional (CRITICAL)

**Location**: `tui/tabs/auth.rs` (entire file - ~116 lines)

**Problems** (more severe than originally described):
1. No InputGroup - uses raw `String` fields instead of component
2. No Focus Tracking - all handlers empty or return false
3. Missing Required Methods - `progress()` exists but returns hardcoded `0.0`
4. No Input Validation - appends characters blindly without validation
5. No actual auth functionality implemented

**Solution**: Complete rewrite following `tabs/oauth.rs` pattern. Key changes:

```rust
use crate::tui::components::{InputField, InputGroup, ScrollableText};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};

// Add focus area enum
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AuthFocusArea {
    Inputs,
    Mode,
    Results,
}

pub struct AuthTab {
    pub inputs: InputGroup,           // NEW: Use InputGroup
    pub mode_select: usize,
    pub results: String,
    pub results_view: ScrollableText,
    pub state: AppState,
    pub focus_area: AuthFocusArea,
}

impl AuthTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new()
            .add(InputField::new("Target URL"))
            .add(InputField::new("Username"))
            .add(InputField::new("Password List"))
            .add(InputField::new("Mode").with_value("brute-force"));

        Self {
            inputs,
            mode_select: 0,
            results: String::new(),
            results_view: ScrollableText::new("Results"),
            state: AppState::Idle,
            focus_area: AuthFocusArea::Inputs,
        }
    }
}

// TabState implementation - track actual state and progress
// TabInput implementation - delegate to InputGroup for field navigation
// TabRender implementation - render input fields with proper layout
```

**Estimated Lines**: ~250 lines (complete rewrite)

---

### D.4: Help Overlay Incorrect Information (CRITICAL)

**Location**: `tui/ui.rs:~709-724`

**Current (WRONG)**:
```
Line 710: "[h/l] Tab | [j/k] Nav | [w/b] Word | [gg/Top] [G/Bot] | [n/p] Tab | [q] Quit"
```

**Problem**: h/l moves WITHIN content (cursor left/right), NOT between tabs.

**Actual Tab Switching**:
- `n` / `p` - next/previous tab
- `Shift+H` / `Shift+L` - prev/next tab
- `1-9`, `0` - direct tab jump
- `gg` - go to first tab
- `G` - go to last tab

**Fix in ui.rs**:
```rust
// Line 710 - Change from:
"[h/l] Tab | [j/k] Nav | [w/b] Word | [gg/Top] [G/Bot] | [n/p] Tab | [q] Quit"

// To:
"[n/p] Tab | [Shift+H/L] Tab | [j/k] Nav | [h/l] Cursor | [gg/G] Jump | [q] Quit"
```

---

### D.5: Missing Keyboard Shortcuts in Help (HIGH)

**Location**: `tui/help.rs:511-557`

**Issue**: Some shortcuts exist but aren't documented or may not be wired up.

**Verify and add** if missing:
| Shortcut | Action |
|----------|--------|
| `gg` / `G` | Go to first/last tab (verify two-key combo is wired) |
| `Ctrl+Z` | Pause/Resume |
| `Ctrl+T` | Theme toggle |
| `Ctrl+V` | Paste |
| `Ctrl+Y` | Resume when paused |
| `Ctrl+B` | Bookmark |
| `Space` | Toggle help |
| `w` / `b` | Word forward/backward |
| `H` / `L` | Home/End |

**Fix**: Add to `global_commands` vector in help.rs after verifying shortcuts are wired in input handler.

---

### D.6: Inconsistent Error Handling (MEDIUM)

**Location**: Multiple tabs

**Pattern Quality**:
- Good: ReconTab - dedicated error block with `tc!(error)`
- Mediocre: FuzzTab - in status bar, can truncate
- Bug: SettingsTab - always green even for errors

**Solution**: Adopt ReconTab pattern across all tabs:
1. Use dedicated error block (not status bar)
2. Use `tc!(error)` for color (theme-aware)
3. Store error in both `state` and separate field

---

### D.7: HistoryTab Search - VERIFY BEFORE WORK

**Location**: `tui/tabs/history.rs:170-186`

**CORRECTION**: The plan claimed search was unavailable, but `search()` method **DOES EXIST** and is functional:
```rust
pub fn search(&self, query: &str) -> Vec<&HistoryEntry> {
    if query.is_empty() {
        return self.entries.iter().collect();
    }
    // ... filtering logic
}
```

**Action**: Verify if there's actually a UI issue or if the plan was simply wrong.

---

### D.8: SettingsTab Progress - VERIFY BEFORE WORK

**Location**: `tui/tabs/settings/main.rs:~414-416`

**CORRECTION**: The plan claimed progress indicator was missing, but `SettingsTab` has no async operations. Returning `0.0` is **correct behavior** - Settings is not a scanning operation.

**Action**: Verify if there's actually a missing feature or if the plan was wrong.

---

### D.9: Validation Feedback Missing (MEDIUM)

**Location**: `tui/components/input.rs:~140-292`

**Issue**: Validation methods exist (`validate_url()`, `validate_ip()`, etc.) but NOT automatically triggered on input.

**Current**: InputField has `validation: Option<ValidationResult>` field but no automatic validation on `insert()` or `backspace()`.

**Solution**: Trigger validation on input changes:
```rust
// In insert() method, after inserting character:
if let Some(ref validation) = self.validation {
    if !validation.valid {
        // Update validation state
    }
}
```

Display validation message in `InputField::render()`:
```rust
if let Some(ref validation) = self.validation {
    if !validation.valid && !validation.message.is_empty() {
        let msg_area = Rect { /* below input */ };
        f.render_widget(
            Paragraph::new(validation.message.as_str())
                .style(Style::default().fg(tc!(error))),
            msg_area,
        );
    }
}
```

---

### D.10: Inconsistent Focus Patterns (MEDIUM)

**Issue**: Three different focus patterns across tabs.

**Recommended Standard**: Enum-based (ReconTab pattern)
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReconFocusArea {
    Inputs,
    Options,
    Results,
}

pub focus_area: ReconFocusArea;

fn handle_focus_next(&mut self) {
    self.focus_area = match self.focus_area {
        ReconFocusArea::Inputs => ReconFocusArea::Options,
        ReconFocusArea::Options => ReconFocusArea::Results,
        ReconFocusArea::Results => ReconFocusArea::Inputs,
    };
}
```

**Solution**: Apply enum pattern to AuthTab and HistoryTab.

---

## Wave E: Feature Completion

**Priority**: MEDIUM
**Sequential**: After A, B, C, D complete
**Target**: Fill capability gaps

### E.1: Fix gRPC Implementation (HIGH)

**Issue**: gRPC implementation uses manual Rust structs instead of protobuf-generated code. `proto/tool.proto` does not exist.

**Required Steps**:

1. Create protobuf definitions in `proto/tool.proto`:
```protobuf
syntax = "proto3";

package slapper.tool.v1;

service ToolService {
    rpc ListTools(ListRequest) returns (ListResponse);
    rpc GetTool(GetRequest) returns (ToolDefinition);
    rpc ExecuteTool(ExecuteRequest) returns (stream ExecuteResponse);
    rpc GetCapabilities(GetRequest) returns (CapabilitiesResponse);
}

message ListRequest {
    optional string category = 1;
}
// ... etc
```

2. Enable tonic in `Cargo.toml`:
```toml
[build-dependencies]
prost-build = "0.14"

[dependencies]
tonic = "0.14"
prost = "0.14"
```

3. Rewrite `tool/protocol/grpc.rs` with generated code

4. Add server startup in `commands/handlers/grpc.rs`

**Files to Modify**:
| File | Action |
|------|--------|
| `proto/tool.proto` | Create |
| `Cargo.toml` | Add prost-build, tonic, prost |
| `tool/protocol/grpc.rs` | Rewrite with generated code |
| `tool/mod.rs` | Add `tonic-build` cfg gate |

**Effort**: 1-2 weeks

---

### E.2: Feature Flag Inconsistencies (MEDIUM)

**Issue 1**: `full` feature references wrong name (`websocket` instead of `ws-api`)

**Location**: `Cargo.toml:~237`

**Fix**: Change `full = [..., "websocket", ...]` to `full = [..., "ws-api", ...]`

**Issue 2**: TUI tab feature-gated dispatch

**CORRECTION**: `tui/tabs/mod.rs` already has proper `#[cfg(feature = "...")]` and `#[cfg(not(feature = "..."))]` dual-arm pattern for all feature-gated tabs. This issue may have been fixed or was never present.

**Action**: Verify with `cargo check --lib -p slapper --features nse` (should compile without tab dispatch errors).

---

### E.3: Consolidate Empty Feature Flags (LOW)

**Issue**: Some features have no Cargo dependencies but enable code.

**Recommendation**: Document groupings in `ARCHITECTURE.md`:
```toml
# UI Extensions
ui-extensions = ["advanced-hunting", "compliance", "headless-browser"]

# DevSecOps
devsecops = ["external-integrations", "database", "sbom", "vuln-management"]
```

---

### E.4: Plugin Architecture Improvements

**E.4.1: Thread Safety - PluginRegistry Synchronization**

**Current**: No synchronization on `Vec<Arc<dyn Plugin>>`

**CORRECTION**: `slapper-plugin/src/lib.rs` already uses `Arc<RwLock<Vec<Arc<dyn Plugin>>>` - thread safety is already implemented.

**E.4.2: AST-Based Security Analysis (MEDIUM)**

**CORRECTION**: Current implementation uses REGEX-BASED pattern detection, not AST analysis. The plan incorrectly described existing code.

**Reference**: [PyAegis](https://github.com/mnbplus/PyAegis) - AST taint analysis

**New module** (if desired): `slapper-plugin/src/ast_scanner.rs`

**Config option** (for future):
```rust
pub enum DetectionMode {
    Regex,    // Current, fast
    Ast,      // Slower, more accurate
    Strict,   // Both required
}
```

---

### E.5: PDF Pagination Fix (LOW)

### E.26: Skill System Improvements (IN PROGRESS)

**Issue**: Skill structs lack version field for compatibility.

**Status**: IN PROGRESS (commit agent-wave-e-fixes)
- Added `version: Option<String>` to `Skill` struct
- Added `version: Option<String>` to `SkillMetadata`

### E.24: LongitudinalMemory Unbounded Growth ✓ COMPLETE

**Status**: FIXED
- Added `max_scans_per_target: Option<usize>` config
- FIFO eviction when limit reached

### E.25: Agent handle_status_impl ✓ COMPLETE

**Status**: FIXED
- Shows AI enabled status
- Shows memory directory and stored target count
- Shows monitored/enabled target counts

### E-20: TUI Spinner Animation ✓ COMPLETE

**Status**: FIXED
- Added `spinner_tick: u64` to `App` struct

### E-17: TUI Confirmation Dialogs ✓ COMPLETE

**Status**: FIXED
- Added `PopupKind::Destructive` variant
- Added `destructive_popup()` helper

### E-28: No Update Command for Targets ✓ COMPLETE

**Status**: FIXED
- Added `TargetsCommand::Update` variant

---

### E.6: Auto-Calibration System (HIGH)

**Reference**: plan7.md Phase 3

**Goal**: Implement ffuf-style smart calibration

**Current State**: `calibration.rs` and `filters.rs` do NOT exist in fuzzer/ - this is a real capability gap.

**Components**:
1. `fuzzer/calibration.rs` - Sample-based calibration
2. `fuzzer/filters.rs` - Pre-scan filters (status, size, word, line, time, regex)
3. CLI integration: `--calibrate`, `-fc`, `-fs`, `-fw`, `-fl`, `-ft`, `-fr`

**Effort**: 500 lines

---

### E.7: Subdomain Enumeration Enhancement (HIGH)

**Reference**: plan7.md Phase 2

**Current State** (CORRECTION): Code has 3 sources (crt.sh, alexa, threatminer), NOT 2 as plan stated. Threatminer is correctly present.

**Target**: Match Amass with 40+ OSINT sources

**Phases**:
1. Certificate Transparency Sources (CertSpotter, Censys, Digitorus, Facebook CT)
2. Passive DNS (VirusTotal, Shodan, SecurityTrails, DNSDB, AlienVault OTX)
3. WHOIS/ASN (Reverse WHOIS, ASN Lookup, IPinfo)
4. API Key Management via `config/recon_sources.toml`

**Effort**: 650 lines

---

### E.8: Community Template Ecosystem

**CORRECTION**: `scanner/templates/` ALREADY EXISTS with:
- `marketplace.rs`
- `verify.rs`
- `models.rs`
- `executor.rs`
- `matcher.rs`
- `mod.rs`
- `loader.rs`

The plan incorrectly treated this as a "capability gap" when templates are already implemented.

**Action**: Verify existing implementation is complete and working. If gaps exist, document them specifically.

---

## Wave F: Documentation (Wave G in user request)

**Priority**: MEDIUM
**Parallel**: Can run alongside Wave E
**Target**: Match documentation to actual codebase

### G-1: Feature Flag Accuracy ✅ COMPLETE 2026-04-28

`docs/CAPABILITIES.md` referenced `mcp-server` feature which was removed. Replaced all `mcp-server` references with `rest-api`.

---

### G-4: API.md Accuracy ✅ COMPLETE 2026-04-28

Added deprecation notice to API.md pointing to `cargo doc` for authoritative API documentation (Option C - deprecate in favor of inline docs + `cargo doc`).

---

### F.1: Fix Documentation Discrepancies ✅ COMPLETE 2026-04-28 ✅ COMPLETE 2026-04-28

**F.1.1: Payload Types (verified - 30 exist, docs said 22)** ✅

Completed (G-2, G-10):
- `README.md` - Updated "20+ payload types" → "30 payload types"
- `ARCHITECTURE.md` - Updated "22 payload types" → "30 payload types"
- `docs/CAPABILITIES.md` - Updated "24 payload types" → "30 payload types" and added all missing types

**F.1.2: Recon Modules** ✅

Completed (G-3):
- `docs/CAPABILITIES.md` - Updated "18 modules" → "30+ modules" and added missing modules

**F.1.3: Feature Flags** ✅

Completed (G-13):
- `docs/FEATURES.md` - Added all ~15+ feature flags with descriptions

---

### F.2: Create New Conceptual Documents ✅ COMPLETE 2026-04-28

**F.2.1: `docs/VULNERABILITY_GUIDE.md`** (NEW) ✅

Educational reference explaining vulnerability classes, attack variants, and detection methods.

**Sections**: SQL Injection (G-7), XSS (G-8), SSRF/JWT (G-9), WebSocket Security (G-5), gRPC Security (G-6)

---

**F.2.2: `docs/SCAN_STRATEGY.md`** (NEW)

Decision guide for choosing scan profiles and understanding tradeoffs.

**Sections**: Profile Comparison Matrix, Scenario-Based Recommendations, Custom Stage Chains, Speed vs Thoroughness Tradeoffs, Stealth Considerations, CI/CD Integration

---

**F.2.3: `docs/FEATURE_GUIDE.md`** (NEW) ✅ Partially Complete (covered in FEATURES.md G-13)

Detailed documentation for undocumented feature flags.

**Sections**: All ~15 undocumented features with descriptions, configuration, and security warnings

---

### F.3: Expand Existing Documentation ✅ COMPLETE 2026-04-28

**README.md** ✅:
- Add "When to Test" column to payload types (G-10)
- Update scan profile descriptions with use cases
- Add brief concept explainers before examples

**docs/CAPABILITIES.md** ✅:
- Add "When to Use" column
- Add Attack Variant column (G-15)
- Expand recon modules table with details (G-3)

**docs/USAGE.md** ✅:
- Add scenario-based testing section (G-16)
- Add configuration recommendations (G-17)

---

### F.4: Skills Standardization

**Structure Requirements** for all skills in `slapper_skills/`:
1. YAML frontmatter (name, description, triggers, metadata)
2. Overview section
3. Capabilities section
4. Usage section (CLI examples)
5. Payload reference tables
6. Triggers section
7. Best Practices section

**Review and update** each skill file for:
- All required sections present
- Triggers accurately reflect capabilities
- Examples use current CLI syntax

---

## Implementation Order

### Phase 1: Parallel Implementation (Weeks 1-4)

**Team A - Security**:
- A.1: Regex ReDoS Prevention (2-4h)
- A.2: Plugin Security Patterns (2-3h) - ADD missing patterns, don't remove existing
- A.3: Config File Permissions (1-2h)
- A.4: Plugin Timeout Enforcement (High effort - start early)

**Team B - Code Quality**:
- B.1: Fix AI-integration compilation error (HIGH priority - unblocks B.2)
- B.2: Fix 9 Failing Tests (Medium effort - after B.1)
- B.3: Add Default Implementations (Low effort)
- B.4: Remove Dead Code (Low effort)
- B.5: Clippy Warnings (Low effort)

**Team C - Performance**:
- C.1: Clone Storm Fix (4-6h - highest impact)
- C.2: FxHashMap Migration (1-2h)
- C.3: AtomicU64 Counters (1-2h)
- C.4: String Allocations (2-3h)
- C.5: DashMap (Only if profiling shows contention)

**Team D - TUI**:
- D.1: UTF-8 Cursor Fix (Low effort - CRITICAL)
- D.2: Hardcoded Colors (Medium effort)
- D.3: AuthTab Rewrite (High effort - 250 lines)
- D.4: Help Overlay Fix (Low effort)
- D.5-D.10: Verify before working (some items may be incorrect)

### Phase 2: Feature Completion (Weeks 5-8)

- E.1: gRPC Implementation (1-2 weeks)
- E.2: Feature Flag Fixes (1 day) - fix `full` feature reference
- E.3: Empty Feature Consolidation (2 days)
- E.4: Plugin Architecture (if desired - current implementation is regex-based)
- E.5: PDF Pagination (2 days)
- E.6-E.7: Capability Gaps (ongoing)
- E.8: Verify templates already exist

### Phase 3: Documentation (Weeks 6-10)

- F.1: Discrepancy Fixes (Week 1)
- F.2: New Conceptual Docs (Weeks 2-3)
- F.3: Expand Existing Docs (Weeks 3-4)
- F.4: Skills Standardization (Week 4)

---

## Verification Commands

```bash
# Base build and tests
cargo check --lib -p slapper
cargo test --lib -p slapper

# Full features (may fail due to B.1 issue)
cargo test --lib -p slapper --features rest-api,ai-integration
cargo clippy --lib -p slapper
cargo clippy --lib -p slapper --features rest-api,ai-integration

# Plugin tests
cargo test -p slapper-plugin --features python-plugins,ruby-plugins

# NSE tests
cargo test --lib -p slapper-nse

# Build verification
cargo build --release -p slapper
cargo build --release -p slapper --features full
```

---

## Success Criteria

| Metric | Target | Current | Notes |
|--------|--------|---------|-------|
| AI-integration compiles | Yes | No | B.1 fix required |
| Test failures | 0 | 9+ | Wave B |
| Clippy warnings | <10 | ~19 | Accept TUI-specific |
| ReDoS protected | 100% | Yes (A.1 complete) | Wave A |
| Plugin patterns | 30+ Python, 25+ Ruby | 17 Python, 18 Ruby | Wave A - add missing |
| Fuzzer clones | <5/iter | 13+ | Wave C |
| Hot path HashMap | FxHashMap | HashMap | Wave C |
| TUI theme-aware | 100% | No | Wave D |
| gRPC | Working tonic | Manual structs | Wave E |

---

## Dependencies Summary

**Parallel Waves**: A, B, C, D can run concurrently with 4 teams.

**Sequential Dependencies**:
- Wave E requires A, B, C, D to be largely complete
- Wave F can run alongside Wave E
- B.2 (test fixes) depends on B.1 (compilation fix)

**Cross-Wave Dependencies**:
- E.1 (gRPC) is independent - can start in Phase 1
- D.3 (AuthTab) depends on B.3 (Default implementations) for InputGroup

**Verification Required** (before work):
- D.7 (HistoryTab search) - method already exists
- D.8 (SettingsTab progress) - 0.0 is correct behavior
- E.8 (Templates) - already implemented

---

## Notes

- Some findings are **intentional design decisions** for security testing tools
- TLS bypass capability required for testing self-signed certificates
- Plugin pattern detection must balance security vs false positives
- Config permission enforcement should warn, not block (for usability)
- Benchmark before/after for each performance optimization
- Always verify plan claims against actual codebase before implementing

---

*Last updated: 2026-04-25*
*Status: ACTIVE DEVELOPMENT*
*Verified by: subagent exploration of codebase*