# NSE Milestone 1 Final Corrective Pass

> **Status: Executed.** See the Milestone 1 closure note in `architecture/nse_integration.md` and regression tests in `crates/eggsec-nse/tests/script_file_policy_tests.rs`. The plan is retained for audit/handoff purposes so future maintainers can trace the Milestone 1 closure history.

## Purpose

This plan is a narrow final corrective pass after the Milestone 1 corrective closure implementation.

The prior closure pass addressed the major structural issues: Lua `require()` now delegates filesystem module loading to `ScriptResolver`, executor construction carries script/module policies, automated profile guard tests were added, and documentation now states the correct cancellation posture. The remaining issues appear smaller, but they are important because they affect manual script-file behavior and future loader safety.

This pass should close those last defects without expanding the NSE feature set.

## Current Assessment

The repo is close to Milestone 1 closure.

Confirmed improvements:

- `ExecutorCore` stores `NseScriptPolicy` and `NseModulePolicy`.
- `setup_require()` delegates filesystem module loading to `ScriptResolver::resolve_module()`.
- `NseExecutor`/`AsyncNseExecutor` have explicit policy/profile construction paths.
- Manual constructors now carry manual-only warnings.
- Profile guard tests exist.
- Resolver docs and architecture notes describe canonical containment and the limits of Rust-side blocking helper cancellation.

Remaining risks:

1. `ManualPermissive` likely rejects all manual script files through resolver symlink-containment logic when `allowed_script_roots` is empty.
2. Resolver path helpers still contain a parent-canonicalization path for non-existent files. Current call sites appear to check existence first, but the helper is too permissive for script/module read authorization and can be misused later.
3. Script-file and module-file root semantics need to be explicit and tested for three distinct cases:
   - unrestricted manual use,
   - restricted-root manual/strict use,
   - agent/CI no-filesystem use.
4. Verification is claimed in commit text, but no GitHub status checks were visible. The final gate needs to be run and recorded in docs or commit notes.

## Non-Goals

Do not add new NSE libraries.

Do not begin Milestone 2 library registry/rule semantics work.

Do not change the project policy distinction between manual CLI/TUI discretion and strict automated agent/MCP operation.

Do not make `ManualPermissive` require explicit roots unless the CLI UX is also updated to configure those roots transparently. Manual script-file usage should remain practical.

Do not remove manual-only compatibility constructors. Keep them, but keep warnings and guardrails.

## Workstream 1: Fix Manual-Permissive Script-File Semantics

### Problem

`ResolvedNseExecutionProfile::manual_permissive()` currently allows script files but leaves `allowed_script_roots` empty. In `resolve_script_file()`, root validation is skipped when the roots list is empty, but `validate_symlink_containment(&path, &self.script_policy.allowed_script_roots)` is still called. Since the roots list is empty, symlink containment cannot succeed and may reject ordinary manual script files.

This likely breaks the intended manual CLI behavior for `--script-file`.

### Required Outcome

Manual-permissive script-file loading must work by design while preserving strict containment for restricted profiles.

Explicit semantics:

- `ManualPermissive`:
  - `allow_script_files = true`
  - empty `allowed_script_roots` means unrestricted manual file loading
  - extension and size limits still apply if configured
  - no root containment check is required when roots are intentionally empty
  - symlink containment is not enforceable without roots and should not reject normal files
- `ManualStrict` / `CompatibilityLab` with roots:
  - script files must canonicalize under one approved root
  - symlink escapes must be rejected
- `AgentSafe` / `CiSafe`:
  - script files must be rejected before path authorization

### Implementation Steps

1. Add explicit helper semantics in `resolver.rs`, for example:

```rust
fn resolve_existing_file_with_policy(
    path: &Path,
    approved_roots: &[PathBuf],
    mode: FileRootMode,
) -> Result<PathBuf, NseLoadError>
```

or a narrower helper:

```rust
fn validate_existing_file_for_read(
    path: &Path,
    approved_roots: &[PathBuf],
    unrestricted_when_roots_empty: bool,
) -> Result<PathBuf, NseLoadError>
```

2. In `resolve_script_file()`:
   - check policy first;
   - check file exists and is a file;
   - check extension;
   - if `allowed_script_roots` is empty and profile policy permits unrestricted manual files, canonicalize the file and allow it;
   - if roots are non-empty, enforce canonical root containment and symlink escape rejection;
   - read only the canonical authorized path.
3. Do not apply unrestricted empty-root behavior to `AgentSafe` or `CiSafe`; those profiles should already fail at `allow_script_files = false`.
4. Consider making this semantic explicit in `NseScriptPolicy`, e.g. by adding an enum later:

```rust
pub enum NseFileRootPolicy {
    DenyFilesystem,
    AllowAnyManual,
    AllowApprovedRoots(Vec<PathBuf>),
}
```

For this pass, prefer the smallest safe change unless the current bool/vector representation becomes too ambiguous.

### Tests

Add tests proving:

- `ManualPermissive` can execute or resolve a real temporary `.nse` script file when `allowed_script_roots` is empty.
- `ManualPermissive` still rejects invalid extensions.
- `ManualPermissive` still enforces `max_script_bytes` if configured.
- `ManualStrict` rejects a script file outside `/tmp/eggsec-nse` or configured roots.
- `ManualStrict` accepts a script file under an approved root.
- `AgentSafe` rejects `NseScriptSource::File` before filesystem authorization.

### Acceptance Criteria

- Manual CLI/TUI script-file usage is not broken by empty roots.
- Restricted profiles still enforce canonical root containment.
- Agent/CI profiles still deny script files.

## Workstream 2: Remove Non-Existent File Authorization from Read Helpers

### Problem

`validate_path_under_roots()` includes fallback logic that canonicalizes a non-existent file's parent and authorizes the constructed child path if the parent is under an approved root. That can make sense for future write/create operations, but script/module loading is read-only and should require an existing file.

Current read call sites appear to check `exists()` first, so this may not be immediately exploitable. It is still a dangerous helper contract for future maintainers.

### Required Outcome

Read-path authorization helpers must only authorize existing files.

If parent-based authorization is ever needed for create/write semantics, it should live in a separate helper with a name that makes the difference explicit.

### Implementation Steps

1. Replace or split `validate_path_under_roots()` into read-specific and create-specific helpers:

```rust
fn validate_existing_path_under_roots(
    path: &Path,
    approved_roots: &[PathBuf],
) -> Result<PathBuf, NseLoadError>
```

2. The read helper should:
   - reject missing paths as `NotFound` or `IoError`, depending on context;
   - canonicalize the exact file path;
   - compare canonical file path against canonical roots using path-component semantics;
   - never authorize based only on the parent directory.
3. Update script/module read paths to use the read helper.
4. Delete parent-canonicalization fallback from the read helper.
5. If tests or sandbox code need parent-based behavior for writes, create a separate helper named accordingly and document it as not valid for reads.

### Tests

Add tests proving:

- non-existent `root/missing.nse` is not authorized by read helper merely because `root` exists;
- non-existent `root/missing.lua` returns `NotFound`/read-specific error rather than an authorized canonical path;
- existing `root/real.nse` is authorized;
- existing `root_evil/real.nse` is rejected when root is `root`;
- symlink `root/link.nse -> outside/real.nse` is rejected.

### Acceptance Criteria

- No read-path helper authorizes non-existent script/module files.
- Parent fallback, if retained anywhere, is impossible to confuse with read authorization.

## Workstream 3: Make Root-Policy Semantics Explicit in Tests and Docs

### Problem

The policy model currently uses bools plus vectors:

- `allow_script_files`
- `allowed_script_roots`
- `allow_filesystem_modules`
- `allowed_module_roots`

The meaning of an empty roots vector differs by context. For manual script files, empty roots may mean unrestricted manual discretion. For filesystem modules, empty roots means no filesystem module loading. For agent profiles, file loading is denied regardless of roots.

This is acceptable if documented and tested, but fragile if implicit.

### Required Outcome

The repo must document and test empty-root semantics clearly.

Expected semantics:

| Policy Area | Bool | Empty Roots Meaning |
|-------------|------|---------------------|
| Manual script files | `allow_script_files = true` | unrestricted manual file selection |
| Strict script files | `allow_script_files = true` | roots must be non-empty and enforced |
| Agent/CI script files | `allow_script_files = false` | denied before root checks |
| Filesystem modules | `allow_filesystem_modules = true` | no filesystem modules if roots empty |
| Agent/CI modules | `allow_filesystem_modules = false` | denied before root checks |

### Implementation Steps

1. Add doc comments to `NseScriptPolicy` and `NseModulePolicy` explaining empty-root behavior.
2. Add tests that assert the table above.
3. Update `architecture/nse_integration.md` and `.opencode/skills/eggsec-nse/SKILL.md` with the same semantics.
4. Ensure any `ManualPermissive` warning says manual script-file loading is intentionally discretionary, not agent-safe.

### Acceptance Criteria

- Empty-root behavior is explicit in code comments, docs, and tests.
- No test has to infer semantics from implementation details.

## Workstream 4: Strengthen Regression Coverage Around CLI/Resolver Integration

### Problem

Unit tests may verify resolver behavior, but the CLI path combines profile resolution, script-file policy checks, resolver calls, executor construction, and script execution. The manual-permissive bug likely sits at this integration boundary.

### Required Outcome

There should be at least one integration test covering manual CLI script-file behavior and at least one covering strict/agent rejection.

### Implementation Steps

1. Add tests in the appropriate NSE test module, likely `crates/eggsec-nse/tests/profile_guard_tests.rs`, `sandbox_tests.rs`, or a new `script_file_policy_tests.rs`.
2. Test `run_cli_with_profile()` where practical. If async CLI testing is too heavy, test the same resolver/executor path used by `run_cli_with_profile()`.
3. Ensure tests cover:
   - manual-permissive script-file success;
   - agent-safe script-file denial;
   - strict outside-root denial;
   - strict inside-root success;
   - invalid extension denial;
   - symlink escape denial.
4. Prefer temporary directories for all filesystem cases.

### Acceptance Criteria

- A future regression in manual `--script-file` behavior fails tests.
- A future relaxation of agent/strict file policy fails tests.

## Workstream 5: Final Verification and Milestone 1 Closure Note

### Required Verification Commands

Run the full gate:

```bash
cargo check -p eggsec-nse
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse
make test-nse
```

If the repo environment requires different feature combinations, document the exact equivalent commands used.

### Documentation Update

After tests pass, update a short closure note in `architecture/nse_integration.md` or `crates/eggsec-nse/AGENTS.override.md` stating:

- Milestone 1 loader policy is closed.
- Manual script-file loading is intentionally discretionary under `ManualPermissive`.
- Strict/agent/CI profiles retain fail-closed file/module policy.
- Rust-side helper cancellation remains Milestone 3 capability-wrapper work.

### Acceptance Criteria

- Verification gate passes or failures are documented with follow-up tasks.
- Docs do not overclaim Rust-side blocking helper cancellation.
- Milestone 1 remaining issues list is updated accurately.

## Recommended Commit Structure

1. Add failing regression tests for manual script-file and empty-root semantics.
2. Fix resolver script-file root handling.
3. Split or tighten read-path authorization helpers.
4. Add/adjust strict, agent, symlink, and missing-file tests.
5. Update policy docs/comments.
6. Run verification gate and update closure note.

## Final Acceptance Criteria

This final corrective pass is complete when:

- `ManualPermissive` can load a real `.nse` script file without configured roots.
- `ManualPermissive` still enforces extension and size checks.
- `ManualStrict` and equivalent restricted profiles enforce canonical root containment.
- `AgentSafe` and `CiSafe` reject arbitrary script files before path authorization.
- Filesystem modules remain resolver-owned and require configured approved roots.
- Read-path authorization cannot authorize non-existent files through parent fallback.
- Tests cover empty-root semantics explicitly.
- The full NSE verification gate has been run and recorded.

## Handoff Notes

Keep this pass small. The goal is not to redesign profiles wholesale. If a larger enum-based policy model is tempting, defer it unless the bool/vector model prevents a correct narrow fix. The immediate closure requirement is to make the existing policy model precise, tested, and accurate for manual versus automated execution.