# NSE Milestone 1 Corrective Closure Plan

## Purpose

This plan closes the remaining gaps after the initial NSE Milestone 1 implementation pass.

The recent implementation materially improved the NSE subsystem by adding execution limits, cancellation primitives, execution profiles, profile-aware CLI wiring, and a hardened script/module resolver. This corrective pass should not broaden the NSE feature surface. It should verify and tighten the implementation until Milestone 1 can be considered closed with high confidence.

The focus is on correctness of enforcement boundaries:

- All script and module loading must truly flow through the resolver.
- Canonicalized module paths must be checked against canonical approved roots.
- Automated surfaces must not inherit manual-permissive behavior accidentally.
- Timeout/cancellation must be meaningful for Lua execution and clearly bounded for Rust-side helpers.
- Tests and architecture guards must make regressions difficult.

## Current State Summary

The repo now contains the major Milestone 1 building blocks:

- `crates/eggsec-nse/src/limits.rs`
- `crates/eggsec-nse/src/profile.rs`
- `crates/eggsec-nse/src/resolver.rs`
- profile-aware executor construction
- profile-aware CLI entry point
- execution-limit tests
- profile tests
- resolver tests
- updated NSE architecture and agent guidance docs

The implementation appears directionally correct, but the review identified several closure risks:

1. The `require()` implementation in `executor_core.rs` still appears to construct filesystem candidates directly and perform direct `std::fs::read_to_string()` in the require path. It validates module names and canonicalizes paths, but it must be verified/refactored so module loading actually delegates to `ScriptResolver` or an equivalent resolver-owned API.
2. Canonicalization alone is not a containment check. Any canonicalized candidate path must be checked against canonical approved roots using path-component semantics.
3. The CLI defaulting to `ManualPermissive` is acceptable for manual use, but automated entry points must be audited so they cannot call compatibility constructors or `run_cli(config)` and silently inherit manual behavior.
4. Lua timeout/instruction limits are improved, but Rust-side helper calls still need explicit cancellation/limit posture. This pass should at least classify remaining direct side effects and block new bypasses.
5. GitHub did not report status checks for the head commit through the connector. The verification gate must be run and documented by the implementer.

## Non-Goals

Do not add new NSE libraries in this pass.

Do not rewrite the full NSE rule engine.

Do not implement Milestone 2's declarative library registry yet, except for small hooks needed by this closure pass.

Do not attempt complete upstream Nmap NSE parity.

Do not make manual CLI/TUI behavior agent-strict by default. Preserve the project's intended manual/automated distinction.

## Workstream 1: Make `require()` Resolver-Owned

### Problem

The hardening goal was that all script and module loading flows through `ScriptResolver`. The current require path appears to perform some hardening inline: module-name validation, extension checks, canonicalization, and direct file reads. Inline hardening is better than the original behavior, but it creates duplicate enforcement logic and can drift from resolver semantics.

### Required Outcome

Lua `require()` filesystem loading must use `ScriptResolver::resolve_module()` or an internal resolver-owned function with the same policy and diagnostics.

There should be one authoritative implementation for:

- module-name validation
- approved module roots
- canonical root containment
- symlink escape rejection
- extension allowlist
- module size limits
- missing vs blocked vs invalid diagnostics
- read error diagnostics

### Implementation Steps

1. Inspect `ExecutorCore::setup_require()` and identify every filesystem read path used by `require()`.
2. Add a resolver handle, resolver policy snapshot, or resolver-owned callback to `ExecutorCore` so `require()` can resolve modules without duplicating path logic.
3. Replace direct candidate construction and direct file reads with resolver calls.
4. Preserve existing built-in/global module lookup order where needed:
   - cached modules
   - `_REQUIRE_MODULES`
   - global table modules
   - resolver-backed filesystem modules only if profile permits
5. When resolver returns filesystem module content, evaluate that content and then cache only successful loads.
6. Propagate resolver diagnostics into executor diagnostics or tracing. Do not squash all resolver errors into generic `module not found` unless Lua compatibility requires the user-facing Lua error; even then, retain structured diagnostics internally.
7. Remove duplicated module-name validation or candidate path construction from `setup_require()` once resolver delegation is in place.

### Tests

Add or adjust tests proving:

- `require("../escape")` fails before filesystem access.
- `require("foo/bar")` fails before filesystem access.
- `require("C:\\temp\\evil")` fails before filesystem access.
- A valid module inside an approved root loads successfully.
- A module outside approved roots is rejected.
- A symlink inside an approved root pointing outside the root is rejected.
- A module over `max_module_bytes` is rejected.
- Filesystem modules are denied when profile/module policy disables them.
- Diagnostics distinguish invalid name, blocked by policy, outside root, oversized, and read failure.

### Acceptance Criteria

- `setup_require()` no longer directly calls `std::fs::read_to_string()` for module files.
- `setup_require()` no longer constructs module file candidates outside resolver-owned code.
- Resolver unit tests and require integration tests cover the same policy outcomes.

## Workstream 2: Enforce Canonical Root Containment Everywhere

### Problem

Canonicalization is necessary but insufficient. A path like `allowed_root/link.lua` can canonicalize to `/etc/passwd` or another out-of-root target if it is a symlink. The canonical candidate must be compared against canonical approved roots.

### Required Outcome

Every script/module filesystem load must prove:

```text
canonical_candidate starts with one canonical_allowed_root using path-component semantics
```

String-prefix checks are not sufficient. `/tmp/allowed_evil` must not match `/tmp/allowed`.

### Implementation Steps

1. Audit `resolver.rs`, `executor_core.rs`, `lib.rs`, and any NSE helper files for path authorization logic.
2. Create or centralize a helper such as:

```rust
fn canonical_child_of(candidate: &Path, roots: &[PathBuf]) -> Result<PathBuf, NseLoadError>
```

3. Canonicalize roots once when constructing the resolver or normalize them into an internal `CanonicalRoot` type.
4. Use `Path::starts_with()` only on canonical `Path` values, not on lossy strings.
5. Remove or bypass any fallback that authorizes by string prefix when canonicalization fails.
6. Treat missing files as `NotFound`, not as a reason to authorize based on parent fallback for script/module reads. Script/module reads require existing files.
7. Add regression tests for sibling-prefix tricks and symlink escapes.

### Tests

Required cases:

- `/tmp/root/file.nse` under `/tmp/root` is allowed.
- `/tmp/root2/file.nse` is not allowed when root is `/tmp/root`.
- symlink from `/tmp/root/link.nse` to `/tmp/outside/file.nse` is rejected.
- symlink from `/tmp/root/link.nse` to `/tmp/root/real.nse` is allowed if symlinks are intended to be allowed.
- canonicalization failure does not fall back to string-prefix authorization.

### Acceptance Criteria

- All script/module path authorization uses canonical root containment.
- No string-prefix authorization remains for script/module reads.
- Tests cover symlink escape and sibling-prefix bypass.

## Workstream 3: Close Automated-Surface Profile Bypasses

### Problem

Manual CLI using `ManualPermissive` by default is intentional. Automated surfaces must not get manual behavior by default. The current implementation added profile-aware APIs, but all caller paths must be audited.

### Required Outcome

Automated surfaces must select `AgentSafe`, `CiSafe`, or another explicit non-manual profile. They must not call compatibility constructors that default to manual-permissive behavior.

### Implementation Steps

1. Search for all uses of:

```text
NseExecutor::new
NseExecutor::with_target
NseExecutor::with_sandbox
AsyncNseExecutor::new
AsyncNseExecutor::with_sandbox
run_cli(config)
run_cli_with_profile(config, None)
ResolvedNseExecutionProfile::manual_permissive
```

2. Classify each call site as:

- manual CLI/TUI
- test fixture
- CI fixture
- daemon/runtime
- agent/MCP/autonomous
- library compatibility path

3. For daemon/runtime/agent/MCP/autonomous paths, require explicit profile input or construct `AgentSafe` from validated target/scope.
4. If such paths do not yet exist, add architecture guard tests so future paths cannot accidentally use manual defaults.
5. Consider marking manual-default constructors with doc comments warning that they are manual-only.
6. Consider adding an enum/marker for execution surface if it can be done without broad churn:

```rust
pub enum NseExecutionSurface {
    ManualCli,
    ManualTui,
    Agent,
    Mcp,
    Daemon,
    Ci,
    CompatibilityLab,
}
```

A helper can then map surface to a required profile.

### Tests and Guards

Add static or unit-level guard tests that fail if:

- `run_cli_with_profile(config, None)` is used outside manual CLI wrapper code.
- `ManualPermissive` is referenced from agent/MCP/daemon crates.
- `NseExecutor::new()` is used in non-test automated code.

The exact mechanism can be simple grep-style tests if the repo already uses them, or Rust tests that inspect source text under `CARGO_MANIFEST_DIR`.

### Acceptance Criteria

- Automated surfaces cannot instantiate manual-permissive NSE runtime by accident.
- Manual defaults remain available only in manual-facing code and compatibility tests.
- Guard tests document and enforce the boundary.

## Workstream 4: Clarify Cancellation Limits for Rust-Side Helpers

### Problem

Lua interruption handles CPU-bound script execution. Rust-side helpers may still perform blocking network or filesystem calls that are not instantly interruptible by Lua hooks.

Milestone 1 does not need a full capability-wrapper refactor, but it must make the current posture explicit and prevent new unbounded bypasses.

### Required Outcome

Every Rust-side side-effecting helper in `eggsec-nse` must be classified as one of:

- already cancellation-aware
- bounded by short local timeout and checked before/after operation
- manual-only compatibility helper
- known Milestone 3 follow-up requiring capability wrapper migration

### Implementation Steps

1. Search `crates/eggsec-nse/src` for direct uses of:

```text
std::fs::
std::net::TcpStream
std::net::UdpSocket
reqwest::blocking
native_tls
ssh2
Command
io::popen
```

2. For each call site, document whether it is script/module loading, resolver-owned, public API, Lua library helper, or test-only.
3. Add cancellation checks before and after calls where a token is already accessible.
4. Ensure blocking network calls have explicit finite timeouts.
5. For call sites that cannot be fixed in this pass, add targeted TODO comments referencing the future capability-wrapper milestone.
6. Add an architecture guard that prevents new direct side-effect APIs in `eggsec-nse` outside approved files/modules.

### Acceptance Criteria

- The repo has an explicit inventory or comments for remaining direct Rust-side side effects.
- New direct side-effecting calls are caught by guard tests or a documented review checklist.
- Existing public API helpers do not falsely inherit Lua hook cancellation guarantees in docs.

## Workstream 5: Verification Gate and Documentation Closure

### Problem

Commit messages state tests pass, but the connector did not report CI statuses. Closure requires a documented verification gate.

### Required Outcome

The implementer must run the full planned gate and update docs/comments if behavior differs from claims.

### Required Commands

Run:

```bash
cargo check -p eggsec-nse
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse
make test-nse
```

If any command is unavailable in the environment, document why and run the closest equivalent.

### Documentation Checks

Update docs if necessary so they accurately state:

- `ManualPermissive` is manual-only.
- Automated profiles are explicit/fail-closed.
- `require()` loading uses the resolver.
- Timeout/instruction limits apply to Lua execution.
- Blocking Rust-side helper cancellation is limited unless the helper uses the cancellation-aware path.
- Any remaining direct side-effect helpers are pending Milestone 3 wrapper migration.

### Acceptance Criteria

- Verification commands pass or failures are documented with corrective commits.
- Architecture docs and AGENTS guidance no longer overclaim resolver or cancellation behavior.
- Known-issues list is updated to remove fixed items and retain real follow-ups.

## Recommended Implementation Order

1. Add/adjust tests that expose the remaining `require()` and containment gaps.
2. Refactor `require()` to delegate module loading to `ScriptResolver`.
3. Centralize canonical root containment and remove string-prefix/path-auth duplication.
4. Audit profile call sites and add automated-surface guard tests.
5. Inventory Rust-side side effects and add cancellation checks where practical.
6. Run verification gate.
7. Update docs/AGENTS/architecture notes with exact final behavior.

## Final Closure Criteria

This corrective pass is complete when:

- `require()` filesystem module loading is resolver-owned.
- Script/module loads cannot escape roots through traversal, symlink, absolute path, or sibling-prefix tricks.
- Agent/MCP/daemon paths cannot silently use `ManualPermissive` defaults.
- Cancellation semantics are accurate in code and docs.
- Guard tests prevent reintroducing direct loader bypasses.
- Full NSE verification gate has been run and documented.

## Handoff Notes

Keep this corrective pass narrow. Avoid broad public API rewrites unless a direct bypass is discovered. Prefer small commits in this order: failing tests, resolver refactor, containment helper, profile guard, side-effect inventory, docs, verification.

If a finding turns out to be a false positive because the implementation already enforces the desired behavior, add or improve a regression test proving it. Do not rely on comments alone for closure.
