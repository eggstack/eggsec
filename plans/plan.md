# Slapper Improvement Plan

**Date**: 2026-04-25
**Status**: ACTIVE DEVELOPMENT
**Priority**: High

---

## Executive Summary

This document consolidates all improvement plans into a single implementation roadmap. Items are organized into waves based on parallelization potential, with dependencies noted for items that require prior completion.

**Current State**:
- 1,107+ passing tests (base)
- 1,345 passing tests (with full features, 9 pre-existing AI test failures)
- ~19 clippy warnings (TUI-specific acceptable)
- 470+ source files
- 30 payload types in fuzzer
- 29 TUI tabs
- 29 recon modules

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
Focus: gRPC implementation, plugin architecture, capability gaps

### Wave F: Documentation (Parallel - Can run alongside E)
Focus: Discrepancy fixes, new guides, skills standardization

---

## Wave A: Security Hardening

**Priority**: CRITICAL
**Team**: A (can work in parallel with B, C, D)
**Target**: Close critical security bypass vectors

### A.1: Regex ReDoS Prevention

**Issue**: The `regex` crate allows building regexes from untrusted input without size limits. 7 locations in slapper-nse bypass the safe `build_regex()` helper.

**Locations requiring fix**:
| File | Line | Risk |
|------|------|------|
| `slapper-nse/src/libraries/match_lib.rs` | 87 | CRITICAL |
| `slapper-nse/src/libraries/matchs.rs` | 47, 56 | CRITICAL |
| `slapper-nse/src/libraries/lpeg.rs` | 155, 179, 202 | CRITICAL |
| `slapper-nse/src/libraries/re.rs` | 221 | CRITICAL |

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

**Python - Add these patterns** (`slapper-plugin/src/security.rs`):
```rust
// CRITICAL - code execution
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

**Ruby - Add these patterns**:
```rust
Regex::new(r"(?i)(instance_eval|class_eval|module_eval)\(").unwrap(),
Regex::new(r"(?i)%x\{").unwrap(),
Regex::new(r"(?i)Marshal\.load").unwrap(),
Regex::new(r"(?i)RubyVM::InstructionSequence").unwrap(),
Regex::new(r"(?i)\brequire\b").unwrap(),
Regex::new(r"(?i)\bload\b").unwrap(),
Regex::new(r"(?i)\bsend\(").unwrap(),
```

**Python - Remove high false positives** (causes noise without security benefit):
- `r"(?i)getattr\("` - too common
- `r"(?i)chr\("` - trivially bypassed

**Ruby - Remove high false positives**:
- `r"(?i)\bopen\b"` - too generic
- `r"(?i)\beval\b"` - redundant with `eval(`
- `r"(?i)Shellwords\.escape"` - no direct execution risk

**Verification**:
```bash
cargo test -p slapper-plugin --features python-plugins,ruby-plugins
```

---

### A.3: Config File Permissions

**Issue**: `check_config_file_permissions()` in `types.rs:250-303` is never called.

**Fix**: Call after config loads in `config/loader.rs`:
```rust
// After config.validate() at line 51:
check_config_file_permissions(&canonical_path);

// After loading scope file at line 82:
check_config_file_permissions(&canonical_path);
```

**Note**: Function returns `()` (unit), logs warnings via `tracing::warn!`.

---

### A.4: Plugin Timeout Enforcement (Process Isolation)

**Issue**: Timeouts are advisory-only. Python GIL and Ruby VM continue executing after timeout.

**Recommended Solution**: Process-based plugin runner

**New module**: `slapper-plugin/src/process_runner.rs`

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

### B.1: Fix Failing AI-integration Tests (9 tests)

**Test 1-2: Skills Trigger Extraction** (`agent/skills.rs:103-126`)

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

**Test 8: Content Extraction** (`ai/client.rs:463`)

**Issue**: Test expects 3 lines but 4 are returned.

**Fix**: Change assertion from `assert_eq!(content.len(), 3)` to `assert_eq!(content.len(), 4)`

---

**Test 9: WAF Knowledge Base** (`ai/waf_bypass.rs:117-134`)

**Issue**: `record_success()` updates existing entry instead of adding when values conflict with pre-populated data.

**Fix**: Use unique test values (e.g., "cloudflare_test_xyz", "payload_test_xyz") that don't conflict with pre-populated data

---

### B.2: Add Missing Default Implementations (8 types)

All are SAFE - zero-sized structs with all fields having Default:

| Type | File |
|------|------|
| CargoScanner | `recon/dependency_scan/cargo/mod.rs:10` |
| NpmScanner | `recon/dependency_scan/npm/mod.rs:10` |
| GoScanner | `recon/dependency_scan/go/mod.rs:10` |
| StressTab | `tui/tabs/stress.rs:63` |
| ReportTab | `tui/tabs/report.rs:78` |
| OAuthTab | `tui/tabs/oauth.rs:58` |
| GraphQlTab | `tui/tabs/graphql.rs:55` |
| ClusterTab | `tui/tabs/cluster.rs:64` |

**Implementation Pattern**:
```rust
impl Default for TypeName {
    fn default() -> Self {
        Self::new()
    }
}
```

---

### B.3: Remove Dead Code (2 items)

**Item 1: `ParsedDependency` struct**
- Location: `recon/dependency_scan/mod.rs:61`
- Analysis: Never constructed (zero references), `DependencyInfo` provides equivalent functionality
- Action: REMOVE the struct definition

**Item 2: `TabDispatcher::is_input_focused()`**
- Location: `tui/app/dispatch.rs:80-82`
- Analysis: Method never called through dispatcher wrapper
- Action: REMOVE the dispatcher method

---

### B.4: Address Remaining Clippy Warnings

| Warning Type | Count | Action |
|------------|------- |--------|
| map_or simplification | 2 | Apply suggestion |
| casting unnecessary | 2 | Apply suggestion |
| unused import | 1 | Remove |
| variable mutable | 1 | Review |

**Note**: TUI-specific warnings are acceptable per AGENTS.md guidelines.

---

## Wave C: Performance Improvements

**Priority**: HIGH
**Team**: C (can work in parallel with A, B, D)
**Target**: 20-40% performance improvement in fuzzer/scanner hot paths

### C.1: Fix Clone Storm in Fuzzer Execution Loop

**Priority**: CRITICAL
**Location**: `fuzzer/engine/execution.rs:101-140`

**Issue**: Per-payload worker loop clones 13+ values per iteration.

**Current Pattern** (lines 102-113):
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
    client: Arc<Client>,    // Changed from Client
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

**Locations** (all SAFE - no untrusted input):
| File | Module | Usage |
|------|--------|-------|
| `fuzzer/api_schema/mod.rs:5` | 5 | `generate_auth_bypass_payloads()` |
| `fuzzer/targets/api.rs:3` | 3 | OpenAPI spec parsing |
| `fuzzer/redos_detect.rs:4` | 4 | Cache compiled patterns |
| `scanner/cms/mod.rs:14` | 14 | CMS fingerprinting |

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

**Locations**:
| File | Line | Current | Change |
|------|------|---------|--------|
| `scanner/fingerprint.rs` | 234 | `tokio::sync::Mutex<u64>` | `AtomicU64` |
| `scanner/ports/spoofed.rs` | 138 | `tokio::sync::Mutex<u64>` | `AtomicU64` |

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

Consider preallocation or `Url::join()`.

---

### C.5: Hot Path Arc<Mutex<Vec>> → DashMap (4 locations)

**Priority**: MEDIUM
**Target**: Lock contention on concurrent appends

**Locations**:
| File | Type | Hot Path? | Recommendation |
|------|------|-----------|----------------|
| `tool/history.rs:26` | `Arc<RwLock<Vec<ExecutionEntry>>>` | Yes | DashMap keyed by ID |
| `distributed/queue.rs:26` | `Arc<RwLock<VecDeque<Task>>>` | Yes | Channel-based queue |
| `tool/agents/communication.rs:150` | `Arc<RwLock<Vec<AgentMessage>>>` | Yes | DashMap keyed by message ID |
| `tool/implementations/oast.rs:29` | `Arc<RwLock<Vec<Interaction>>>` | Yes | DashMap keyed by interaction ID |

**Migration Pattern**:
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

**Location**: `tui/components/input.rs:39, 76, 132`

**The Bug**: `cursor_pos` field is used as both byte offset AND character position inconsistently.

**Root Cause**: Struct designed for byte offsets (line 81's `insert()` takes byte index), but `with_value()`, `apply_autocomplete()`, and `move_end()` treat it as character count.

**Solution**: Standardize on byte offsets throughout:

```rust
// FIX at line 37-43 (with_value method):
pub fn with_value(mut self, value: impl Into<String>) -> Self {
    let v = value.into();
    self.cursor_pos = v.len();  // Use byte length, not char count
    self.value = v;
    self
}

// FIX at line 74-77 (apply_autocomplete method):
pub fn apply_autocomplete(&mut self, suggestion: &str) {
    self.value = suggestion.to_string();
    self.cursor_pos = self.value.len();  // Use byte length
}

// FIX at line 131-133 (move_end method):
pub fn move_end(&mut self) {
    self.cursor_pos = self.value.len();  // Use byte length, not char count
}
```

---

### D.2: Hardcoded Colors (CRITICAL)

**Issue**: Tabs use `Color::Green`, `Color::Red`, `Color::Yellow` directly instead of `tc!()` theme macro.

**Affected Files** (39 instances across 5 files):

| File | Lines | Should Be |
|------|-------|-----------|
| `tabs/recon.rs` | 136, 153, 179, 199, 211, 226, 381, 385, 399 | `tc!(success/warning/error/text_dim)` |
| `tabs/waf.rs` | 124, 128, 131, 138, 142, 150, 162, 174, 196, 203, 205, 212, 224, 374, 387 | `tc!(warning/error/success/info/text_dim)` |
| `tabs/scan_ports.rs` | 115, 120, 123, 129, 132, 138, 288, 294 | `tc!(warning/info/success/text_dim)` |
| `components/selector.rs` | 245, 247, 303, 305, 355, 357 | `tc!(selected/border)` |
| `components/scrollable.rs` | 104 | `tc!(border)` |

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

**Location**: `tui/tabs/auth.rs` (entire file - 116 lines)

**Problems**:
1. No InputGroup - uses raw `String` fields instead of component
2. No Focus Tracking - all handlers empty or return false
3. Missing Required Methods - `progress()` not implemented
4. No Input Rendering - only shows static text

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

**Location**: `tui/ui.rs:710, 716, 721`

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

**Issue**: 10+ shortcuts exist but aren't documented.

| Shortcut | Action | Missing From |
|----------|--------|---------------|
| `Ctrl+Z` | Pause/Resume | Help + CommandPalette |
| `Ctrl+T` | Theme toggle | Help + CommandPalette |
| `Ctrl+V` | Paste | Help + CommandPalette |
| `Ctrl+Y` | Resume when paused | Help + CommandPalette |
| `Ctrl+B` | Bookmark | Help + CommandPalette |
| `Space` | Toggle help | Help only |
| `w` / `b` | Word forward/backward | Help only |
| `H` / `L` | Home/End | Help only |

**Fix**: Add to `global_commands` vector in help.rs

---

### D.6: Inconsistent Error Handling (MEDIUM)

**Location**: Multiple tabs

**Pattern Quality**:
- ✓ Good: ReconTab - dedicated error block with `tc!(error)`
- ⚠ Mediocre: FuzzTab - in status bar, can truncate
- ✗ Bug: SettingsTab - always green even for errors

**Solution**: Adopt ReconTab pattern across all tabs:
1. Use dedicated error block (not status bar)
2. Use `tc!(error)` for color (theme-aware)
3. Store error in both `state` and separate field

---

### D.7: HistoryTab Search Unavailable (MEDIUM)

**Location**: `tabs/history.rs:170-186`

**Issue**: Search method EXISTS but NO UI to access it.

**Solution**: Add search input field:
```rust
// Add to HistoryTab render:
// - Add search InputField to struct
// - Render in top area when focused
// - Bind '/' key to activate search
// - Call search() method on input
```

---

### D.8: SettingsTab Missing Progress Indicator (MEDIUM)

**Location**: `tabs/settings/main.rs`

**Current**: Hardcoded returns `AppState::Idle` and `0.0` progress.

**Blocking Operations** (no progress shown):
- `save_config()` - blocking file write
- `add_schedule()` - blocking file operation
- `convert_report()` - blocking I/O

**Solution**: Add progress tracking:
```rust
pub progress: ProgressGauge,
pub state: AppState,
pub pending_operation: Option<PendingOp>,

// Update TabState:
fn state(&self) -> AppState { self.state.clone() }
fn progress(&self) -> f64 { self.progress.percent() as f64 }
```

---

### D.9: Validation Feedback Missing (MEDIUM)

**Location**: `tui/components/input.rs`

**Issue**: Validation methods exist, message NOT displayed.

**Solution**: Display validation message in `InputField::render()`:
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

**Issue**: gRPC implementation uses manual Rust structs instead of protobuf-generated code.

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

### E.2: Fix Feature Flag Inconsistencies (MEDIUM)

**Issue 1**: `full` feature references wrong name (`websocket` instead of `ws-api`)

**Location**: `Cargo.toml:237`

**Fix**: Change `full = [..., "websocket", ...]` to `full = [..., "ws-api", ...]`

**Issue 2**: Missing `#[cfg(not(...))]` arms for TUI tabs

**Location**: `tui/tabs/mod.rs`

**Fix**: Add dual-arm pattern for feature-gated tab variants:
```rust
#[cfg(feature = "nse")]
Tab::Nse => handler.handle_nse(),
#[cfg(not(feature = "nse"))]
Tab::Nse => {
    Err("NSE support not compiled".into())
},
```

---

### E.3: Consolidate Empty Feature Flags (LOW)

**Issue**: 12 features have no Cargo dependencies but enable code.

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

**Fix**:
```rust
use std::sync::RwLock;

pub struct PluginRegistry {
    plugins: RwLock<Vec<Arc<dyn Plugin>>>,
}

impl PluginRegistry {
    pub fn register(&self, plugin: Arc<dyn Plugin>) {
        self.plugins.write().unwrap().push(plugin);
    }

    pub async fn run_check(&self, check_name: &str, target: &str) -> Result<Vec<PluginResult>> {
        let plugins = self.plugins.read().unwrap();
        // ... iterate safely
    }
}
```

**E.4.2: AST-Based Security Analysis (MEDIUM)**

**Reference**: [PyAegis](https://github.com/mnbplus/PyAegis) - AST taint analysis

**New module**: `slapper-plugin/src/ast_scanner.rs`

**Config option**:
```rust
pub enum DetectionMode {
    Regex,    // Current, fast
    Ast,      // Slower, more accurate
    Strict,   // Both required
}
```

---

### E.5: PDF Pagination Fix (LOW)

**Location**: `output/pdf.rs:80`

**Issue**: Only renders first 30 findings, rest silently dropped.

**Fix Options**:
- Option A (recommended): Add pagination
- Option B (quick fix): Increase limit to 100

**Effort**: 2 days

---

### E.6: Auto-Calibration System (Capability Gap)

**Priority**: HIGH
**Reference**: plan7.md Phase 3

**Goal**: Implement ffuf-style smart calibration

**Components**:
1. `fuzzer/calibration.rs` - Sample-based calibration
2. `fuzzer/filters.rs` - Pre-scan filters (status, size, word, line, time, regex)
3. CLI integration: `--calibrate`, `-fc`, `-fs`, `-fw`, `-fl`, `-ft`, `-fr`

**Effort**: 500 lines

---

### E.7: Subdomain Enumeration Enhancement (Capability Gap)

**Priority**: HIGH
**Reference**: plan7.md Phase 2

**Current**: 2 sources (crt.sh, ThreatMiner)
**Target**: Match Amass with 40+ OSINT sources

**Phases**:
1. Certificate Transparency Sources (CertSpotter, Censys, Digitorus, Facebook CT)
2. Passive DNS (VirusTotal, Shodan, SecurityTrails, DNSDB, AlienVault OTX)
3. WHOIS/ASN (Reverse WHOIS, ASN Lookup, IPinfo)
4. API Key Management via `config/recon_sources.toml`

**Effort**: 650 lines

---

### E.8: Community Template Ecosystem (Capability Gap)

**Priority**: HIGH
**Reference**: plan7.md Phase 1

**Goal**: Add Nuclei-compatible template support

**Components**:
1. `scanner/templates/schema.rs` - Template format definition
2. `scanner/templates/loader.rs` - YAML parser, variable substitution
3. `scanner/templates/registry.rs` - Index by tags, severity, CVE
4. CLI: `slapper template run -t cve -s high`

**Effort**: 600 lines

---

## Wave F: Documentation

**Priority**: MEDIUM
**Parallel**: Can run alongside Wave E
**Target**: Match documentation to actual codebase

### F.1: Fix Documentation Discrepancies

**F.1.1: Payload Types (6 missing)**

Add to documentation:
- `nosql` - NoSQL Injection
- `xpath` - XPath Injection
- `expression` - Expression Injection
- `prototype` - Prototype Pollution
- `race` - Race Condition
- `massassign` - Mass Assignment

**Files requiring updates**:
- `docs/CAPABILITIES.md` - Add 6 missing payload types
- `README.md` - Update "20+ payload types" → "30 payload types"
- `lib.rs` - Update comment on line 16 from "22" to "30"
- `fuzzer/mod.rs` - Update "22 payload types" to "30"

---

**F.1.2: Recon Modules (11 missing)**

Add to documentation:
- `secrets` - Secret Detection
- `git_secrets` - Git Repository Secrets (feature: `git-secrets`)
- `api_schema` - API Schema Discovery (feature: `api-schema`)
- `email_security` - Email Security (SPF/DKIM/DMARC)
- `ssl_audit` - TLS Security Audit
- `ssh_auth` - SSH Authentication Testing
- `ftp_auth` - FTP Authentication Testing
- `smtp_auth` - SMTP Authentication Testing
- `containers` - Container Security (feature: `container`)
- `takeover` - Subdomain Takeover
- `dependency_scan/*` - NPM/Cargo/Go dependency scanning

---

**F.1.3: Feature Flags (17 missing)**

Add to documentation:
- `ai-integration` - AI analysis, payload generation
- `websocket` - WebSocket security testing
- `ws-api` - WebSocket API server support
- `headless-browser` - Headless Chrome for DOM XSS
- `database` - Database storage (sqlx)
- `container` - Kubernetes container security
- `cloud` - Cloud security scanning
- `api-schema` - OpenAPI schema-based fuzzing
- `sbom` - SBOM generation
- `git-secrets` - Git secrets scanning
- `pdf` - PDF report generation
- `wireless` - WiFi security testing
- `insecure-tls` - **SECURITY WARNING** - TLS bypass
- `advanced-hunting` - Threat hunting
- `compliance` - Compliance scanning
- `external-integrations` - Jira/GitHub/GitLab
- `finding-workflow` - Finding lifecycle
- `vuln-management` - Vulnerability triage

---

### F.2: Create New Conceptual Documents

**F.2.1: `docs/VULNERABILITY_GUIDE.md`** (NEW)

Educational reference explaining vulnerability classes, attack variants, and detection methods.

**Sections**: SQL Injection, XSS, SSRF, Path Traversal, Open Redirect, ReDoS, Command Injection, XXE, LDAP Injection, NoSQL Injection, JWT Attacks, OAuth/OIDC, GraphQL, Prototype Pollution, Race Conditions, Mass Assignment

---

**F.2.2: `docs/SCAN_STRATEGY.md`** (NEW)

Decision guide for choosing scan profiles and understanding tradeoffs.

**Sections**: Profile Comparison Matrix, Scenario-Based Recommendations, Custom Stage Chains, Speed vs Thoroughness Tradeoffs, Stealth Considerations, CI/CD Integration

---

**F.2.3: `docs/FEATURE_GUIDE.md`** (NEW)

Detailed documentation for undocumented feature flags.

**Sections**: All 17 undocumented features with descriptions, configuration, and security warnings

---

### F.3: Expand Existing Documentation

**README.md**:
- Add "When to Test" column to payload types
- Update scan profile descriptions with use cases
- Add brief concept explainers before examples

**docs/CAPABILITIES.md**:
- Add "When to Use" column
- Add Attack Variant column
- Expand recon modules table with details

**docs/USAGE.md**:
- Add scenario-based testing section
- Add configuration recommendations

---

### F.4: Skills Standardization

**Structure Requirements** for all 52 skills:
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
- A.2: Plugin Security Patterns (2-3h)
- A.3: Config File Permissions (1-2h)
- A.4: Plugin Timeout Enforcement (High effort - start early)

**Team B - Code Quality**:
- B.1: Fix 9 Failing Tests (Medium effort - high priority)
- B.2: Add Default Implementations (Low effort)
- B.3: Remove Dead Code (Low effort)
- B.4: Clippy Warnings (Low effort)

**Team C - Performance**:
- C.1: Clone Storm Fix (4-6h - highest impact)
- C.2: FxHashMap Migration (1-2h)
- C.3: AtomicU64 Counters (1-2h)
- C.4: String Allocations (2-3h)
- C.5: DashMap Migration (3-4h)

**Team D - TUI**:
- D.1: UTF-8 Cursor Fix (Low effort - CRITICAL)
- D.2: Hardcoded Colors (Medium effort - 39 instances)
- D.3: AuthTab Rewrite (High effort - 250 lines)
- D.4: Help Overlay Fix (Low effort)
- D.5-D.10: Medium/Low priority items

### Phase 2: Feature Completion (Weeks 5-8)

- E.1: gRPC Implementation (1-2 weeks)
- E.2: Feature Flag Fixes (1 day)
- E.3: Empty Feature Consolidation (2 days)
- E.4: Plugin Architecture (ongoing)
- E.5: PDF Pagination (2 days)
- E.6-E.8: Capability Gaps (ongoing)

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

# Full features
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
| Test failures | 0 | 9 | AI-integration tests (Wave B) |
| Clippy warnings | <10 | ~19 | Accept TUI-specific |
| ReDoS protected | 100% | No | Wave A |
| Plugin patterns | 30+ | 17 Python, 18 Ruby | Wave A |
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

**Cross-Wave Dependencies**:
- E.1 (gRPC) is independent - can start in Phase 1
- D.3 (AuthTab) depends on B.2 (Default implementations) for InputGroup

---

## Notes

- Some findings are **intentional design decisions** for security testing tools
- TLS bypass capability required for testing self-signed certificates
- Plugin pattern detection must balance security vs false positives
- Config permission enforcement should warn, not block (for usability)
- Benchmark before/after for each performance optimization

---

*Last updated: 2026-04-25*
*Status: ACTIVE DEVELOPMENT*