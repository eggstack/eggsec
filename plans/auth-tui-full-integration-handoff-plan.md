# Handoff Plan: Full Integration of Auth Tab (Credential Cracking & Password Attacks Loadout) — Updated 2026-06-11

**Current Status**: Most registration, dispatch, app wiring, and task system plumbing is now complete. Remaining work is focused on making `AuthTab` actually produce and trigger tasks through the new infrastructure.

---

## Current State Summary (as of latest inspection)

**Completed**:
- `Tab::Auth` fully registered (enum, `all()`, dispatch matches in `tabs/mod.rs`)
- `TabStore` integration
- Full `TabSpec` with correct `risk_group: Intrusive` + `direct_launch: true`
- App-level wiring (`current_tab_target`, `build_current_task`, `copy_cli_equivalent`)
- `TaskConfig::Auth` + `TaskResult::Auth` + dispatch in `TaskRunner`
- Basic `workers/auth.rs` using `AuthEngine`

**Remaining (Actionable)**:
- Implement `build_task_config()` in `AuthTab`
- Update `handle_enter()` to trigger the shared task system
- Decide on final UI scope for `auth.rs` (current version is simplified)
- Minor polish + testing

---

## Remaining Implementation — Exact Code Changes Needed

### 1. Implement `build_task_config()` in `AuthTab` (Most Important)

Add this method to `impl AuthTab` in `crates/eggsec-tui/src/tabs/auth.rs`:

```rust
use crate::workers::TaskConfig;

impl AuthTab {
    pub fn build_task_config(&self) -> Option<TaskConfig> {
        let target = self.target()?.to_string();
        if target.is_empty() {
            return None;
        }

        Some(TaskConfig::Auth {
            target,
            username: self.username().map(|s| s.to_string()),
            password_list: self.password_list().map(|s| s.to_string()),
            credential_file: None, // Add field if you expand inputs later
            max_attempts: 50,      // TODO: Make configurable from UI
            concurrency: 5,
            timeout: 30,
        })
    }
}
```

Also add these helper methods if missing (they exist in the current simplified version):

```rust
pub fn target(&self) -> Option<&str> { ... }
pub fn username(&self) -> Option<&str> { ... }
pub fn password_list(&self) -> Option<&str> { ... }
```

### 2. Update `handle_enter()` to Trigger Real Task Execution

Replace or augment the current `handle_enter` logic:

```rust
fn handle_enter(&mut self) {
    if self.is_running() {
        self.stop();
        return;
    }

    if self.focus_area == AuthFocusArea::Results {
        return;
    }

    let target = self.target().map(|s| s.to_string()).unwrap_or_default();
    if target.is_empty() {
        self.set_error_state(TabError::Target("Target URL is required".to_string()));
        return;
    }

    if self.is_input_focused() {
        self.inputs.blur();
    }

    // Trigger the shared task system (this is what makes policy enforcement + progress work)
    // The actual spawning happens in App::handle_enter via build_current_task()
    self.start();
}
```

### 3. (Optional but Recommended) Restore Richer UI

The current `auth.rs` is quite basic. If you want the fuller experience from the earlier expansion, merge in these improvements:

- Add more `InputField`s (Max Attempts, Concurrency, Timeout, Credential File)
- Add `AuthTestSelection` enum + selection logic
- Improve `render()` with findings list and severity colors
- Keep the `run_tests` stub or remove it once `build_task_config` is wired

Would you like me to provide the full merged richer version as a patch?

### 4. Minor Polish

- Ensure `primary_target()` returns `Some` only when valid
- Add `stop()` that can signal cancellation if a task is running
- Update any tests that assume the old simple structure

---

## Updated Phased Plan (with Current Status)

**Phase 1–3**: ✅ Mostly complete (registration + app wiring)
**Phase 4**: ✅ Core task system done. Remaining = `build_task_config()` + `handle_enter` wiring
**Phase 5**: Remaining polish + testing

---

## Quick Implementation Checklist

- [ ] Add `build_task_config()` to `AuthTab`
- [ ] Update `handle_enter()` to call `self.start()` after validation
- [ ] Verify `TaskConfig::Auth` arm in `TaskRunner` is reached
- [ ] Test end-to-end (enter target → press Enter → see progress + Auth result)
- [ ] Confirm policy confirmation overlay appears for high-risk runs

---

*This updated plan focuses on the concrete remaining code changes. The heavy lifting is done.*
