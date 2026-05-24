# TUI Architecture Review

## Summary Statistics

| Category | Count |
|----------|-------|
| **Verified Claims** | 18 |
| **Discrepancies** | 4 |
| **Bugs Found** | 2 |
| **Improvement Opportunities** | 7 |

---

## Verified Claims

### Core Architecture (Matches Implementation)

1. **App & UI (`app/` module structure)**
   - `App` struct in `app/mod.rs:45-121` - central state container holding all tabs, mode, overlays, theme
   - `runner.rs` - Main event loop using crossterm/ratatui
   - `key_handler.rs` - Priority-based key processing
   - `dispatch.rs` - Routes input to current tab via `TabDispatcher`
   - `state_update.rs` - Async task result handling and routing
   - `task_management.rs` - Maps tabs to `TaskConfig`
   - `task_runtime.rs` - Task lifecycle management

2. **29 Tabs** - Tab enum in `tabs/mod.rs:83-114` has exactly 29 variants (Recon through Vuln)

3. **Tab Traits** (`tabs/mod.rs:864-921`)
   - `TabState` - state(), progress(), reset(), set_error()
   - `TabInput` - handle_focus_next(), handle_char(), handle_enter(), etc.
   - `TabRender` - render(), render_overlays(), breadcrumb()

4. **Components** - All listed components exist in `components/`:
   - `InputField`/`InputGroup` in `input.rs`
   - `Selector`/`Checkbox`/`RadioGroup` in `selector.rs`
   - `ProgressGauge` in `progress.rs`
   - `ScrollableText` in `scrollable.rs`
   - `Popup` in `popup.rs`

5. **Workers** (`workers/mod.rs`) - TaskRunner, Network, Scanner, Fuzzer, Recon, API, Security workers present

6. **Communication Flow** - `app/runner.rs:68-79` correctly spawns async task via `spawn_task()`

7. **Session Management** (`session.rs:60-195`)
   - `SessionManager` auto-saves every 30 seconds
   - Session directory: `~/.slapper/sessions/`

8. **Theme** (`theme.rs:178-254`)
   - `ThemeManager` with dark/light themes
   - `tc!` macro defined at lines 264-268

9. **Key Bindings** - Implementation matches architecture:
   - `Ctrl+C` interrupt/quit
   - `Ctrl+P` command palette
   - `Ctrl+X` quick switch
   - `Ctrl+F` global search
   - `Space` toggle help
   - `hjkl`/Arrows navigation
   - `i` enter insert mode
   - `Esc` return to normal/close overlay
   - `q` quit (when no active task)
   - `g/G` go to top/bottom
   - `n/N` next/prev tab
   - `e` export results
   - `s` save settings

10. **ProgressGauge** correctly guards against division by zero (`components/progress.rs:55-59`)

11. **ScrollableText** correctly handles empty lines (`components/scrollable.rs:135-139`)

12. **SessionState** uses stable IDs with legacy index fallback (`session.rs:23-33`)

13. **SharedHistory** type definition (`state/mod.rs:7`) - `Arc<Mutex<HistoryTab>>`

14. **TabDispatcher** routes to current tab (`dispatch.rs:1-207`)

15. **Fuzzy search** for quick switch (`app/mod.rs:693-710`)

16. **Entry Point** - TUI launches via `handle_no_command()` in `commands/handlers/mod.rs:158-166` when no subcommand and stdout is terminal

17. **Theme colors use FxHashMap** (`theme.rs:179`, `app/mod.rs:113`)

18. **Tab progress implementations** - Most tabs delegate to `ProgressGauge.percent()` which has correct guard

---

## Discrepancies

### 1. "State Management" section describes `SharedHistory` but doesn't mention `parking_lot::Mutex`

**Architecture says:**
```
pub type SharedHistory = Arc<Mutex<HistoryTab>>;
```

**Implementation:** `state/mod.rs:4` uses `parking_lot::Mutex` not `std::sync::Mutex`

**Impact:** Low - parking_lot::Mutex is actually better (non-blocking)

---

### 2. Tab count in architecture vs actual for feature-gated tabs

**Architecture says:** "29 specialized tabs"

**Implementation:** `tabs/mod.rs:83-114` defines exactly 29 Tab variants, but NseTab (line 102) and PluginTab (line 103) are feature-gated. Without features enabled, the actual count of available tabs is lower.

**Impact:** Documentation is accurate for full-featured build but could mislead about availability

---

### 3. "Workers" table shows "Security" worker but doesn't list all task types

**Architecture lists:** Hunt, browser, compliance, storage, integrations under Security worker

**Implementation:** `workers/runner.rs` TaskResult enum shows these are actually separate handler chains via `handle_security_result`, `handle_protocol_result`, `handle_feature_result` in `state_update.rs:58-70`

**Impact:** Low - the categorization is semantic, implementation is correct

---

### 4. Entry point description mentions "via `tui::run()`"

**Architecture says:** "This happens via `handle_no_command()` in `commands/handlers/mod.rs`"

**Implementation:** `commands/handlers/mod.rs:158-166` calls `crate::tui::run()` - this is correct

**Impact:** None - documentation is accurate

---

## Bugs Found

### Bug 1: Duplicate key binding - 'b' appears twice in key_handler.rs

**File:** `crates/slapper/src/tui/app/key_handler.rs:114,124`

```rust
(KeyModifiers::CONTROL, KeyCode::Char('b')) => app.toggle_bookmark(app.current_tab),
// ... later in same match ...
(KeyModifiers::NONE, KeyCode::Char('b')) => app.handle_word_backward(),
```

**Severity:** Medium - The CONTROL+b binding at line 114 is actually reachable (different modifier), but 'b' in normal mode at line 124 would shadow any future binding that should use 'b' combined with shift/ctrl

**Priority:** Medium

**Fix:** Ensure no duplicate patterns in same modifier context

---

### Bug 2: InputGroup field access without bounds check in fuzz.rs reset

**File:** `crates/slapper/src/tui/tabs/fuzz.rs:404-413`

```rust
self.inputs.fields[1].value = "GET".to_string();
self.inputs.fields[1].cursor_pos = 3;
self.inputs.fields[3].value = "0".to_string();
self.inputs.fields[3].cursor_pos = 1;
self.inputs.fields[4].value = "3".to_string();
// ... assumes indices 1, 3, 4, 5, 6 exist
```

**Severity:** Medium - Will panic if InputGroup has fewer than 7 fields

**Priority:** Medium

**Fix:** Add bounds check:
```rust
if self.inputs.fields.len() > 6 {
    self.inputs.fields[1].value = "GET".to_string();
    // ...
}
```

Same pattern exists in `scan.rs:271-274`:
```rust
if self.inputs.fields.len() > 1 {
    self.inputs.fields[1].value = "report.json".to_string();
    self.inputs.fields[1].cursor_pos = 11;
}
```

---

## Improvement Opportunities

### 1. InputGroup::focus_next() panic risk when fields is empty

**File:** `crates/slapper/src/tui/components/input.rs:532`

```rust
let next = (idx + 1) % self.fields.len();  // panics if len() == 0
```

**Impact:** High if called on empty InputGroup

**Priority:** Medium

**Fix:** Early return if `self.fields.is_empty()`

---

### 2. TaskResult handling chain could silently drop results

**File:** `crates/slapper/src/tui/app/state_update.rs:58-70`

```rust
let result = match self.handle_security_result(result) {
    Some(r) => r,
    None => return,
};
// ... if result passes through all three handlers, logs "Unhandled TaskResult variant"
```

**Impact:** Some TaskResult variants may be unintentionally dropped

**Priority:** Low

**Fix:** Add exhaustive match checking or convert to enum with known variants

---

### 3. Auto-save interval hardcoded to 30 seconds but configurable

**File:** `crates/slapper/src/tui/session.rs:45`

```rust
auto_save_interval_secs: 30,
```

**Impact:** The config field exists but is hardcoded to 30 seconds anyway

**Priority:** Low

**Fix:** Allow config to override default

---

### 4. Dispatcher pattern creates new dispatcher multiple times

**File:** `crates/slapper/src/tui/app/mod.rs:349-357`

```rust
fn dispatcher_mut(&mut self) -> TabDispatcher<'_> {
    // Called multiple times in succession (e.g., handle_focus_next at lines 468, 472)
    // Each call recreates the dispatcher
}
```

**Impact:** Minor inefficiency - dispatcher allocation each call

**Priority:** Low

**Fix:** Could cache dispatcher or pass reference

---

### 5. InputGroup field methods not bounds-checked

**File:** `crates/slapper/src/tui/components/input.rs:568-596`

Methods like `focus()`, `insert()`, `backspace()` all assume fields exist at focused index:

```rust
pub fn focus(&mut self, idx: usize) {
    if idx < self.fields.len() {  // safe
        // ...
    }
    // But insert(), backspace(), delete() don't check:
    pub fn insert(&mut self, c: char) {
        if let Some(idx) = self.focused {
            self.fields[idx].insert(c);  // idx could be out of bounds
```

**Priority:** Medium

**Impact:** Could panic if focused index becomes stale

---

### 6. ProgressGauge.percent() uses floating point division

**File:** `crates/slapper/src/tui/components/progress.rs:59`

```rust
((self.current as f64 / self.total as f64) * 100.0).min(100.0) as u16
```

**Impact:** Minor floating point imprecision for large numbers

**Priority:** Low

**Fix:** Use integer math for exact percentage

---

### 7. SessionState bookmarks not deduplicated on restore

**File:** `crates/slapper/src/tui/session.rs:124-145`

When restoring with both new and legacy bookmarks, duplicate stable IDs could be inserted into `FxHashSet`

**Priority:** Low

**Impact:** Minor - set handles duplicates but code is confusing

---

## Priority Summary

| Priority | Count | Items |
|----------|-------|-------|
| **High** | 0 | - |
| **Medium** | 4 | Duplicate key binding 'b', InputGroup bounds in fuzz.rs/scan.rs, InputGroup.insert bounds |
| **Low** | 7 | TaskResult drop chain, hardcoded auto-save, dispatcher caching, floating point, session dedup |

---

## Verified Claims Details

| Claim | Implementation | Status |
|-------|----------------|--------|
| 29 tabs | `tabs/mod.rs:83-114` | Verified |
| TabState/TabInput/TabRender traits | `tabs/mod.rs:864-921` | Verified |
| FxHashMap/FxHashSet usage | Multiple files | Verified |
| 30-second auto-save | `session.rs:45` | Verified |
| Session dir ~/.slapper/sessions | `session.rs:54-57` | Verified |
| tc! macro | `theme.rs:264-268` | Verified |
| ThemeManager with dark/light | `theme.rs:189-198` | Verified |
| TaskRunner async executor | `workers/runner.rs:264-615` | Verified |
| TabDispatcher routing | `dispatch.rs:1-207` | Verified |
| ProgressGauge division guard | `components/progress.rs:56-58` | Verified |
| ScrollableText empty lines guard | `components/scrollable.rs:135-139` | Verified |
| SharedHistory = Arc<Mutex<>> | `state/mod.rs:7` | Verified (parking_lot) |
| Cross-platform entry detection | `commands/handlers/mod.rs:159` | Verified |
| Command palette Ctrl+P | `key_handler.rs:61,290-291` | Verified |
| Quick switch Ctrl+X | `key_handler.rs:48` | Verified |
| Global search Ctrl+F | `key_handler.rs:62` | Verified |
| Help Space/toggle | `key_handler.rs:108` | Verified |
| Export results 'e' | `key_handler.rs:135` | Verified |

---

## Key Discrepancy Details

| Item | Architecture | Implementation | Notes |
|------|--------------|----------------|-------|
| SharedHistory mutex type | `std::sync::Mutex` | `parking_lot::Mutex` | Actually an improvement |
| Tab availability | "29 tabs" | 29 defined, but some feature-gated | Full build = 29 |
| Worker categorization | "Security" worker | Three handler chains | Semantic difference |