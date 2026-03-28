# Ruby Plugin Overhaul Plan

## Overview
Update the Ruby plugin system to be compatible with magnus 0.8.2 API, which requires all functions registered with `magnus::function!` to have `ruby: &Ruby` as their first parameter.

## Current State
- Magnus upgraded from 0.7.1 to 0.8.2
- 40+ functions in `api.rs` need updating to include `ruby: &Ruby` parameter
- Unsafe `Send`/`Sync` implementations added for `RubyBridge`
- `runtime_error()` helper function added but uses `Ruby::get().unwrap()` (should be updated to accept `ruby: &Ruby`)
- Python plugin issues resolved
- Ruby plugin compilation still fails with 33+ errors

## Magnus 0.7 to 0.8 API Changes Affecting Our Code

### 1. Function Registration
- **0.7:** Functions could be registered without `ruby: &Ruby` parameter
- **0.8:** All functions registered with `magnus::function!` must have `ruby: &Ruby` as first parameter
- **Impact:** All 40+ API functions need signature updates

### 2. Error Creation
- **0.7:** `Error::runtime("message")` or `Error::runtime_error("message")`
- **0.8:** `Error::new(ruby.exception_runtime_error(), "message")`
- **Impact:** All error creation calls need updating

### 3. Ruby Method Access
- **0.7:** `ruby.module()` to get top-level module
- **0.8:** `ruby.class_object()` to get top-level Object class
- **Impact:** Bridge code needs updating

### 4. Type Conversions
- **0.7:** Some implicit type conversions worked
- **0.8:** More explicit type annotations required
- **Impact:** Multiple `try_convert()` calls need type annotations

## Required Changes

### 1. Update `runtime_error()` Helper Function
**File:** `crates/slapper-ruby/src/api.rs`

**Current:**
```rust
fn runtime_error(msg: impl Into<std::string::String>) -> Error {
    let ruby = Ruby::get().unwrap();
    Error::new(ruby.class_runtime_error(), msg)
}
```

**Proposed:**
```rust
fn runtime_error(ruby: &Ruby, msg: impl Into<std::string::String>) -> Error {
    Error::new(ruby.class_runtime_error(), msg)
}
```

**Impact:** All calls to `runtime_error()` will need to pass `ruby` parameter.

### 2. Update Function Signatures (40 Functions)

#### HTTP Functions (5 functions)
- `http_post(ruby: &Ruby, url: String, body: String) -> Result<magnus::RHash, Error>`
- `http_put(ruby: &Ruby, url: String, body: String) -> Result<magnus::RHash, Error>`
- `http_delete(ruby: &Ruby, url: String) -> Result<magnus::RHash, Error>`
- `http_request(ruby: &Ruby, method: String, url: String) -> Result<magnus::RHash, Error>`

#### Scanner Functions (3 functions)
- `tcp_connect(ruby: &Ruby, host: String, port: u16) -> Result<bool, Error>`
- `scan_port(ruby: &Ruby, host: String, port: u16) -> Result<bool, Error>`
- `grab_banner(ruby: &Ruby, host: String, port: u16) -> Result<String, Error>`

#### Fuzzer Functions (4 functions)
- `fuzz_param(ruby: &Ruby, url: String, param: String, payloads: Vec<String>, options: Vec<String>) -> Result<Vec<magnus::RHash>, Error>`
- `fuzz_header(ruby: &Ruby, url: String, header: String, payloads: Vec<String>, options: Vec<String>) -> Result<Vec<magnus::RHash>, Error>`
- `fuzz_cookie(ruby: &Ruby, url: String, cookie: String, payloads: Vec<String>, options: Vec<String>) -> Result<Vec<magnus::RHash>, Error>`
- `fuzz_path(ruby: &Ruby, url: String, paths: Vec<String>) -> Result<Vec<magnus::RHash>, Error>`

#### Reporting Functions (6 functions)
- `report_finding(ruby: &Ruby, title: String, severity: String, description: String, location: String) -> Result<(), Error>`
- `report_vulnerability(ruby: &Ruby, title: String, severity: String, description: String, location: String, evidence: String) -> Result<(), Error>`
- `report_info(ruby: &Ruby, title: String, message: String) -> Result<(), Error>`
- `report_success(ruby: &Ruby, title: String, message: String) -> Result<(), Error>`
- `report_warning(ruby: &Ruby, title: String, message: String) -> Result<(), Error>`
- `report_error(ruby: &Ruby, title: String, message: String) -> Result<(), Error>`

#### Metasploit Functions (13 functions)
- `msf_connect(ruby: &Ruby, url: String, username: String, password: String) -> Result<bool, Error>`
- `msf_connect_with_token(ruby: &Ruby, url: String, token: String) -> Result<bool, Error>`
- `msf_connected(ruby: &Ruby) -> Result<bool, Error>`
- `msf_disconnect(ruby: &Ruby) -> Result<bool, Error>`
- `msf_version(ruby: &Ruby) -> Result<String, Error>`
- `msf_list_modules(ruby: &Ruby, module_type: String) -> Result<Vec<String>, Error>`
- `msf_module_info(ruby: &Ruby, module_type: String, module_name: String) -> Result<magnus::RHash, Error>`
- `msf_execute_module(ruby: &Ruby, module_type: String, module_name: String, options: Vec<String>) -> Result<magnus::RHash, Error>`
- `msf_generate_payload(ruby: &Ruby, payload_name: String, options: Vec<String>) -> Result<String, Error>`
- `msf_list_sessions(ruby: &Ruby) -> Result<Vec<magnus::RHash>, Error>`
- `msf_session_info(ruby: &Ruby, session_id: String) -> Result<magnus::RHash, Error>`
- `msf_session_write(ruby: &Ruby, session_id: String, command: String) -> Result<String, Error>`
- `msf_session_read(ruby: &Ruby, session_id: String) -> Result<String, Error>`
- `msf_session_stop(ruby: &Ruby, session_id: String) -> Result<bool, Error>`

#### Encoder Functions (3 functions)
- `encoder_list(ruby: &Ruby) -> Result<Vec<String>, Error>`
- `encoder_encode(ruby: &Ruby, payload: String, encoder_name: String, options: Vec<String>) -> Result<String, Error>`
- `encoder_compatible_payloads(ruby: &Ruby, encoder_name: String) -> Result<Vec<String>, Error>`

#### Session Functions (5 functions)
- `session_list(ruby: &Ruby) -> Result<Vec<magnus::RHash>, Error>`
- `session_interact(ruby: &Ruby, session_id: String) -> Result<bool, Error>`
- `session_write(ruby: &Ruby, session_id: String, command: String) -> Result<String, Error>`
- `session_read_output(ruby: &Ruby, session_id: String) -> Result<String, Error>`
- `session_shell_upgrade(ruby: &Ruby, session_id: String, lhost: String, lport: String) -> Result<bool, Error>`

### 3. Update Function Bodies
For each updated function:

1. **Replace `Ruby::get().unwrap()`** with the `ruby` parameter
2. **Update `runtime_error()` calls** to pass `ruby` parameter
3. **Remove `let ruby = Ruby::get().unwrap();`** lines where present
4. **Update hash creation** to use `ruby.hash_new()` instead of `Ruby::get().unwrap().hash_new()`

### 4. Update Helper Functions
- `detect_vulnerability()` - no change needed (not registered with function!)
- `get_msf_client()` - no change needed (not registered with function!)
- `get_runtime()` - no change needed (not registered with function!)

### 5. Update Bridge Code
**File:** `crates/slapper-ruby/src/bridge.rs`

**Issue 1:** `module()` method not found on `Ruby` struct
- **Current:** `self.ruby.module().const_get::<_, magnus::RModule>("Slapper")`
- **Fix:** Replace with `self.ruby.class_object().const_get::<_, magnus::RModule>("Slapper")`
- **Reason:** In magnus 0.8, `module()` method was removed. Use `class_object()` to get the top-level `Object` class.

**Issue 2:** `ruby.exception_runtime_error()` vs `ruby.class_runtime_error()`
- **Current:** `Error::new(ruby.class_runtime_error(), msg)`
- **Fix:** Replace with `Error::new(ruby.exception_runtime_error(), msg)`
- **Reason:** API changed in magnus 0.8

### 6. Update Loader Code
**File:** `crates/slapper-ruby/src/loader.rs`

Current loader code uses `RubyPluginAdapter` which contains `Arc<Mutex<RubyBridge>>`. This should work with the unsafe `Send`/`Sync` implementations.

### 7. Additional API Issues
**File:** `crates/slapper-ruby/src/api.rs`

**Issue 1:** `module_type` field not found on `ModuleInfo`
- **Location:** Around line 709
- **Fix:** Check the actual field name in the magnus 0.8 `ModuleInfo` struct
- **Workaround:** Remove or comment out the line accessing `module_type`

**Issue 2:** `SessionType: IntoValue` trait bound not satisfied
- **Location:** Around lines 822 and 856
- **Fix:** Convert `SessionType` to string or implement `IntoValue` trait
- **Workaround:** Use `.to_string()` or similar conversion

**Issue 3:** Type annotation needed for `try_convert()`
- **Location:** Multiple places in bridge.rs and api.rs
- **Fix:** Add explicit type annotations like `.try_convert::<String>()`

## Function Update Summary Table

| Category | Functions | Current Issue | Required Change |
|----------|-----------|---------------|-----------------|
| HTTP | 4 | Missing `ruby: &Ruby` param | Add param, update bodies |
| Scanner | 3 | Missing `ruby: &Ruby` param | Add param, update bodies |
| Fuzzer | 4 | Missing `ruby: &Ruby` param | Add param, update bodies |
| Reporting | 6 | Missing `ruby: &Ruby` param | Add param, update bodies |
| Metasploit | 13 | Missing `ruby: &Ruby` param | Add param, update bodies |
| Encoder | 3 | Missing `ruby: &Ruby` param | Add param, update bodies |
| Session | 5 | Missing `ruby: &Ruby` param | Add param, update bodies |
| **Total** | **38** | | |

## Estimated Effort by Category
- **Phase 1 (Helper Functions):** 1 hour
- **Phase 2 (HTTP):** 2 hours
- **Phase 3 (Scanner):** 1 hour  
- **Phase 4 (Fuzzer):** 2 hours
- **Phase 5 (Reporting):** 1 hour
- **Phase 6 (Metasploit):** 4 hours (highest complexity)
- **Phase 7 (Encoder/Session):** 2 hours
- **Phase 8 (Testing):** 2 hours
- **Total:** 15 hours

## Implementation Steps

### Phase 1: Helper Functions (1 hour)
1. Update `runtime_error()` to accept `ruby: &Ruby` parameter
2. Update all calls to `runtime_error()` to pass `ruby` parameter
3. Test compilation

### Phase 2: HTTP Functions (2 hours)
1. Update `http_post`, `http_put`, `http_delete`, `http_request` signatures
2. Update function bodies to use `ruby` parameter
3. Test compilation

### Phase 3: Scanner Functions (1 hour)
1. Update `tcp_connect`, `scan_port`, `grab_banner` signatures
2. Update function bodies
3. Test compilation

### Phase 4: Fuzzer Functions (2 hours)
1. Update all 4 fuzzer functions
2. Update function bodies (note: these functions use `Ruby::get().unwrap()`)
3. Test compilation

### Phase 5: Reporting Functions (1 hour)
1. Update all 6 reporting functions
2. Update function bodies
3. Test compilation

### Phase 6: Metasploit Functions (4 hours)
1. Update all 13 Metasploit functions
2. Update function bodies
3. Test compilation

### Phase 7: Encoder & Session Functions (2 hours)
1. Update all 8 encoder and session functions
2. Update function bodies
3. Test compilation

### Phase 8: Integration Testing (2 hours)
1. Run full test suite with `--features ruby-plugins`
2. Test with actual Ruby plugin if available
3. Verify no regressions in Python plugins

## Risk Assessment

### High Risk
- **Breaking Changes:** All Ruby plugins will need to be compatible with the new API
- **Testing Complexity:** Need to test with actual Ruby plugins (may not be available)

### Medium Risk
- **Performance:** Additional `ruby` parameter passing may have negligible performance impact
- **Memory:** No significant memory changes expected

### Low Risk
- **Backward Compatibility:** Ruby plugins should work as before if they follow the API

## Success Criteria
1. `cargo check --lib -p slapper --features ruby-plugins` compiles successfully
2. `cargo check --lib -p slapper --features full` compiles successfully
3. Existing tests pass
4. No new warnings introduced

## Timeline
- **Total Estimated Time:** 15 hours
- **Critical Path:** Phase 6 (Metasploit Functions)
- **Dependencies:** Phase 1 must be completed first

## Code Patterns to Change

### Pattern 1: Function Signature
**Before:**
```rust
fn function_name(param1: Type1, param2: Type2) -> Result<ReturnType, Error>
```

**After:**
```rust
fn function_name(ruby: &Ruby, param1: Type1, param2: Type2) -> Result<ReturnType, Error>
```

### Pattern 2: Ruby Instance Access
**Before:**
```rust
let ruby = Ruby::get().unwrap();
let hash = ruby.hash_new();
```

**After:**
```rust
let hash = ruby.hash_new();
```

### Pattern 3: Error Creation
**Before:**
```rust
.map_err(|e| Error::runtime(e.to_string()))?;
```

**After:**
```rust
.map_err(|e| runtime_error(ruby, e.to_string()))?;
```

### Pattern 4: Runtime Error Helper
**Before:**
```rust
fn runtime_error(msg: impl Into<std::string::String>) -> Error {
    let ruby = Ruby::get().unwrap();
    Error::new(ruby.class_runtime_error(), msg)
}
```

**After:**
```rust
fn runtime_error(ruby: &Ruby, msg: impl Into<std::string::String>) -> Error {
    Error::new(ruby.exception_runtime_error(), msg)
}
```

### Pattern 5: Bridge Code Module Access
**Before:**
```rust
self.ruby.module().const_get::<_, magnus::RModule>("Slapper")
```

**After:**
```rust
self.ruby.class_object().const_get::<_, magnus::RModule>("Slapper")
```

## Implementation Checklist
- [ ] Update `runtime_error()` helper function
- [ ] Update HTTP functions (4)
- [ ] Update Scanner functions (3)
- [ ] Update Fuzzer functions (4)
- [ ] Update Reporting functions (6)
- [ ] Update Metasploit functions (13)
- [ ] Update Encoder functions (3)
- [ ] Update Session functions (5)
- [ ] Fix bridge.rs module access
- [ ] Fix additional API issues
- [ ] Run cargo check with ruby-plugins
- [ ] Run cargo check with full features
- [ ] Run tests
- [ ] Update documentation

## Testing Strategy

### Unit Tests
1. Create minimal Ruby plugin for testing
2. Test each module (HTTP, Scanner, Fuzzer, etc.) individually
3. Test error handling paths

### Integration Tests
1. Test with existing Ruby plugins (if available)
2. Test plugin loading and execution
3. Test thread safety with concurrent operations

### Regression Tests
1. Ensure Python plugin functionality unchanged
2. Ensure NSE functionality unchanged
3. Ensure base compilation works

## Rollback Plan
If issues arise during implementation:
1. Revert to magnus 0.7.1 temporarily
2. Keep changes in a feature branch
3. Document all API changes for future reference

## Alternative Approaches Considered

### Approach 1: Use `method!` macro instead of `function!`
- **Pros:** Might not require `ruby: &Ruby` parameter
- **Cons:** Limited to methods on structs, not free functions
- **Verdict:** Not suitable for current architecture

### Approach 2: Create wrapper functions that capture Ruby instance
- **Pros:** Minimal changes to existing functions
- **Cons:** Adds complexity and potential memory leaks
- **Verdict:** Rejected due to complexity

### Approach 3: Use unsafe `Ruby::get().unwrap()` everywhere
- **Pros:** No signature changes needed
- **Cons:** Violates magnus 0.8 API contract, potential safety issues
- **Verdict:** Rejected as unsafe and non-idiomatic

## Notes
1. The magnus 0.8 API change is mandatory for thread safety and API consistency
2. All functions must have `ruby: &Ruby` as first parameter for `magnus::function!` macro
3. Consider creating a macro to reduce boilerplate for function registration
4. Document the pattern for future Ruby plugin development
5. Keep backward compatibility with existing Ruby plugins where possible