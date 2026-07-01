# Corrective Handoff Plan: Phase 11 CI Architecture Guards

## Objective

Tighten the Phase 11 CI architecture guard implementation before treating it as complete. The Phase 11 implementation landed in the right area and added the expected workflow coverage, feature-profile checks, documentation, Makefile targets, a static guard script, and Model A test enforcement. The remaining issues are operational correctness issues in the CI guard script and required/deep workflow boundaries.

This corrective pass should make the guard job reliable on GitHub-hosted runners and avoid false passes or false failures from shell execution details.

## Current state summary

Phase 11 implementation added or modified:

- `.github/workflows/test.yml`
- `.github/workflows/deep-checks.yml`
- `scripts/check-architecture-guards.sh`
- `docs/CI_ARCHITECTURE_GUARDS.md`
- `AGENTS.md`
- `CONTRIBUTING.md`
- `Makefile`
- `crates/eggsec/tests/tool_registration.rs`

The important improvements landed:

- `architecture-guards` job runs no-default workspace check, core metadata/registry/enforcement tests, report envelope tests, and static architecture guards.
- Feature-profile matrix covers representative optional profiles.
- Deep checks are separated into a scheduled/manual workflow.
- `docs/CI_ARCHITECTURE_GUARDS.md` documents required PR checks, feature-profile checks, optional/deep checks, platform-sensitive checks, and guarded invariants.
- The OpsAgent Model A test now correctly asserts that OpsAgent is strictly broader than the conservative default listing.

Remaining issues to fix:

1. CI runs `./scripts/check-architecture-guards.sh`. If the file lacks executable permissions, GitHub Actions can fail with permission denied.
2. The static guard script fallback path is unsafe: if `rg` is unavailable, callers still pass ripgrep-specific `--glob` arguments into `grep`, while stderr is suppressed. This can silently weaken checks.
3. Required PR workflow still includes a broad `full` feature compile check in the existing `check` matrix. If `full` stays stable on Ubuntu, this is acceptable; if not, it should move to the deep workflow.
4. The `Makefile` feature-profile target does not match the CI feature-profile matrix. That reduces local reproduction fidelity.
5. Documentation should explicitly state whether `ripgrep` is installed in CI or the script is POSIX/grep-safe.

## Non-goals

- Do not redesign the full CI pipeline.
- Do not remove existing security/dependency checks unless they are known broken.
- Do not change enforcement semantics.
- Do not change MCP Model A semantics.
- Do not add new capabilities or expand protocol exposure.
- Do not require platform-sensitive system dependencies in required PR CI.

## Work item 1: Make architecture guard script invocation permission-safe

### Problem

The workflow currently invokes:

```yaml
- name: Architecture guards script
  run: ./scripts/check-architecture-guards.sh
```

When a script is added through APIs or copied without executable mode, Git may store it as a normal text file. GitHub Actions then fails with `permission denied` even though the script content is valid.

### Required change

Use an interpreter invocation in CI:

```yaml
- name: Architecture guards script
  run: bash scripts/check-architecture-guards.sh
```

Also update any docs and Makefile target that run the script directly unless executable mode is guaranteed.

Recommended Makefile target:

```make
# Architecture drift guards (static grep checks)
test-architecture-guards:
	bash scripts/check-architecture-guards.sh
```

### Optional alternative

Commit the executable bit using normal git tooling. If that path is used, still prefer `bash scripts/...` in CI because it is more robust and self-documenting.

### Acceptance criteria

- CI no longer depends on executable file mode for the guard script.
- Local docs show the same invocation style used by CI.

## Work item 2: Make ripgrep usage explicit or make grep fallback correct

### Problem

`check-architecture-guards.sh` detects `rg`, but if `rg` is missing, it falls back to `grep -rn`. Many calls pass ripgrep-only arguments such as `--glob='*.md'` and `--glob='*.rs'`. With the current implementation, grep will treat those as unsupported options or paths, stderr is redirected, and checks can become false passes.

### Choose one model

#### Model A: Require ripgrep

Recommended because the script is already written around ripgrep semantics.

Changes:

- Remove the grep fallback.
- Fail early if `rg` is missing with a clear message.
- Install ripgrep in the GitHub Actions job before running the script.

Script header example:

```bash
if ! command -v rg >/dev/null 2>&1; then
  echo "FAIL: ripgrep (rg) is required for architecture guard checks." >&2
  echo "Install ripgrep locally or add it to the CI image before running this script." >&2
  exit 1
fi
```

Workflow step example:

```yaml
- name: Install ripgrep
  run: sudo apt-get update && sudo apt-get install -y ripgrep
```

Alternative if the runner already includes ripgrep:

```yaml
- name: Verify ripgrep
  run: rg --version
```

Prefer installing or verifying explicitly so failures are clear.

#### Model B: Make grep fallback fully correct

If avoiding a CI package install is preferred, rewrite the script so no call passes ripgrep-specific arguments to the fallback.

Possible approach:

- Use helper functions that take file extensions separately.
- For grep fallback, enumerate files with `find`.
- Avoid all `--glob` arguments unless `rg` is active.

Example:

```bash
search_md_docs() {
  local pattern="$1"
  if command -v rg >/dev/null 2>&1; then
    rg -n --glob='*.md' "$pattern" docs/ 2>/dev/null || true
  else
    find docs -name '*.md' -print0 | xargs -0 grep -nE "$pattern" 2>/dev/null || true
  fi
}
```

### Recommended approach

Use Model A for this pass. It is simpler and avoids a fragile shell abstraction. The script is an architecture guard, not a runtime dependency.

### Acceptance criteria

- The script never silently weakens checks because `rg` is missing.
- CI either installs or explicitly verifies `rg` before running the script.
- `docs/CI_ARCHITECTURE_GUARDS.md` states that the static guard script requires ripgrep, if Model A is chosen.

## Work item 3: Reconcile `full` feature check placement

### Problem

The existing PR `check` matrix includes:

```yaml
- name: full
  args: "-p eggsec --features full"
```

The Phase 11 plan treats broad/all-feature-ish checks as deep/scheduled unless they are known to be stable and not platform-sensitive. The `full` profile includes advanced/lab/domain features and may become brittle as domains grow.

### Required decision

Choose one model.

#### Model A: Keep `full` in required PR CI

Use this only if `cargo check -p eggsec --features full` is known to be stable on `ubuntu-latest` and does not require external services, privileged network operations, browsers, ADB, packet capture libraries, or device/emulator dependencies.

Required changes:

- Document in `docs/CI_ARCHITECTURE_GUARDS.md` that `full` is currently compile-only stable in required PR CI despite being an aggregate profile.
- Add a note that tests for platform-sensitive behavior stay in deep/manual workflows.

#### Model B: Move `full` to deep checks

Recommended unless the team intentionally wants broad aggregate compile checks on every PR.

Required changes:

- Remove `full` from the required `check` matrix in `.github/workflows/test.yml`.
- Keep `cargo check -p eggsec --features full` in `.github/workflows/deep-checks.yml`.
- Update docs to say `full` is deep/scheduled/manual only.

### Recommended approach

Use Model B unless current CI has already proven `full` is stable and cheap. This keeps required PR CI aligned with the Phase 11 design target.

### Acceptance criteria

- Required PR CI does not include platform-sensitive aggregate checks unless explicitly justified.
- Deep workflow retains `full` coverage.
- Docs match the chosen model.

## Work item 4: Align Makefile local reproduction with CI

### Problem

The `Makefile` target `check-feature-profiles` does not match the Phase 11 CI feature-profile matrix. It currently checks profiles such as `wireless`, `nse`, `evasion`, `postex`, and `c2`, but does not match the explicit CI profiles: `tool-api,rest-api`, `grpc-api`, `db-pentest`, `db-pentest-mcp,tool-api,rest-api`, `mobile`, `mobile-dynamic`, `web-proxy`, `web-proxy-mcp,tool-api,rest-api`, and `c2-mcp,tool-api,rest-api`.

### Required change

Update `Makefile` so local feature-profile reproduction matches CI exactly.

Recommended target:

```make
check-feature-profiles:
	cargo check -p eggsec --features tool-api,rest-api
	cargo check -p eggsec --features grpc-api
	cargo check -p eggsec --features db-pentest
	cargo check -p eggsec --features db-pentest-mcp,tool-api,rest-api
	cargo check -p eggsec --features mobile
	cargo check -p eggsec --features mobile-dynamic
	cargo check -p eggsec --features web-proxy
	cargo check -p eggsec --features web-proxy-mcp,tool-api,rest-api
	cargo check -p eggsec --features c2-mcp,tool-api,rest-api
```

If broader local-only targets are still useful, add a separate target:

```make
check-feature-profiles-extended:
	cargo check -p eggsec --features wireless
	cargo check -p eggsec --features nse
	cargo check -p eggsec --features evasion
	cargo check -p eggsec --features postex
	cargo check -p eggsec --features c2
```

### Acceptance criteria

- `make check-feature-profiles` reproduces CI feature-profile checks.
- Any extended checks are named as extended/deep, not CI-equivalent.

## Work item 5: Add a single local CI reproduction target

### Problem

The docs list the full required CI command sequence, but the Makefile does not yet provide a single target that mirrors the architecture guard job exactly.

### Required change

Add a target such as:

```make
check-architecture-ci:
	cargo fmt --all -- --check
	cargo check --workspace --no-default-features
	cargo test -p eggsec --lib
	cargo test -p eggsec --test metadata_consistency
	cargo test -p eggsec --test command_registry
	cargo test -p eggsec --test tool_registration --features rest-api
	cargo test -p eggsec --test feature_matrix
	cargo test -p eggsec --test enforcement_matrix
	cargo test -p eggsec --test enforced_dispatch_regression
	cargo test -p eggsec-output --test report_envelope
	bash scripts/check-architecture-guards.sh
```

Acceptance criteria:

- Contributors can run one Make target before handoff.
- Target command sequence matches `docs/CI_ARCHITECTURE_GUARDS.md` and `.github/workflows/test.yml`.

## Work item 6: Correct docs after workflow/script decisions

Update:

- `docs/CI_ARCHITECTURE_GUARDS.md`
- `CONTRIBUTING.md`
- `AGENTS.md`
- `Makefile` help text

Required doc updates:

- Static guard script invocation should use `bash scripts/check-architecture-guards.sh` unless executable bit is guaranteed.
- If requiring ripgrep, explicitly say `ripgrep` is required for static guards.
- If moving `full` out of required PR checks, update required/deep check tables.
- If keeping `full`, explicitly justify it as compile-only stable.
- Local Makefile target should be listed.

Acceptance criteria:

- Docs, workflow, and Makefile do not disagree about required checks.
- No docs recommend `./scripts/check-architecture-guards.sh` if CI uses `bash scripts/...`.

## Work item 7: Validate script behavior deliberately

Run local checks that exercise both success and dependency behavior.

Required:

```bash
bash scripts/check-architecture-guards.sh
```

If Model A is chosen:

```bash
command -v rg
rg --version
```

Optionally test the failure mode by temporarily shadowing `rg` in a subshell and confirming the script fails clearly:

```bash
PATH=/nonexistent bash scripts/check-architecture-guards.sh
```

Do not commit temporary changes.

Acceptance criteria:

- Script succeeds when `rg` exists.
- Script fails loudly and clearly when `rg` is required but absent.
- Script does not silently pass because grep ignored unsupported arguments.

## Work item 8: Validate workflow syntax and commands

Run or verify:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo test -p eggsec --lib
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --test command_registry
cargo test -p eggsec --test tool_registration --features rest-api
cargo test -p eggsec --test feature_matrix
cargo test -p eggsec --test enforcement_matrix
cargo test -p eggsec --test enforced_dispatch_regression
cargo test -p eggsec-output --test report_envelope
bash scripts/check-architecture-guards.sh
make check-feature-profiles
```

If a local GitHub Actions linter is available, validate workflow YAML. Otherwise rely on GitHub Actions parsing in the next run.

Acceptance criteria:

- The architecture guard job can run without executable-bit dependency.
- Feature-profile checks are reproducible locally.
- Deep workflow remains separate from required PR checks.

## Files likely to change

- `.github/workflows/test.yml`
- `.github/workflows/deep-checks.yml` only if `full` placement is adjusted or docs comments are added
- `scripts/check-architecture-guards.sh`
- `docs/CI_ARCHITECTURE_GUARDS.md`
- `Makefile`
- `AGENTS.md`
- `CONTRIBUTING.md`

## Completion criteria

This corrective pass is complete when:

- CI runs `bash scripts/check-architecture-guards.sh` or the executable bit is guaranteed and documented.
- The guard script requires `rg` explicitly or has a correct grep fallback.
- CI installs/verifies `rg` if the script requires it.
- Required PR CI no longer includes `full` unless explicitly justified as stable and compile-only.
- `make check-feature-profiles` matches the CI feature-profile matrix.
- A single Make target exists for local architecture guard reproduction.
- Docs match workflow behavior and script dependencies.
- Validation commands pass or any skipped platform-sensitive check is documented.

## Handoff note

After this pass, Phase 11 can be considered closed. The next phase should be Phase 12: extensibility handoff and contributor model, focused on documenting how to add domains, operations, commands, tools, reports, feature gates, and tests without violating the architecture guards.
