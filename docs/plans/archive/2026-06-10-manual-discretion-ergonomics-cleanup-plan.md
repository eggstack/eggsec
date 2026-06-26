# Manual Discretion Ergonomics Cleanup Plan

Date: 2026-06-10
Repository: eggstack/eggsec
Purpose: tighten the ergonomics and audit semantics of manual CLI/TUI discretion after `RequireConfirmation` and manual overrides landed. This is a narrow cleanup pass; do not weaken CI/MCP/agent enforcement.

## Current state

The manual discretion pass is implemented:

- `EnforcementOutcome::RequireConfirmation` exists.
- `ManualOverride` and `ConfirmationClass` exist.
- `ManualPermissive` can return `RequireConfirmation` for operator-discretion cases.
- CLI exposes manual-only flags:
  - `--yes`
  - `--allow-out-of-scope`
  - `--allow-excluded-target`
  - `--allow-high-risk`
  - `--allow-nonbaseline-capability`
  - `--manual-override-reason`
- `CommandContext` accepts matching manual overrides and records/audits them.
- MCP and agent treat `RequireConfirmation` as denial before dispatch.

Remaining cleanup items:

1. `--yes` currently permits every `ConfirmationClass`, including high-risk and explicit exclusion. This is broad; either intentionally document it as broad or make it require specific `--allow-*` flags for dangerous classes.
2. Private-resolution and cross-host-redirect confirmation classes exist, but CLI maps both to `--allow-out-of-scope` rather than dedicated flags.
3. Audit output uses `Debug` class names. It would be cleaner to use stable kebab-case class strings.
4. Error messages should distinguish between broad `--yes` and specific `--allow-*` flags.
5. Tests should lock down manual override semantics so future changes do not accidentally make `--yes` too broad or too narrow.

## Goals

- Make manual override behavior explicit and consistent.
- Prefer specific flags for high-risk and explicit-exclusion overrides.
- Add dedicated CLI flags for private-resolution and cross-host redirect cases.
- Make audit records stable and machine-readable.
- Preserve current manual-discretion model.
- Preserve strict CI/MCP/agent behavior.

## Non-goals

- Do not alter MCP, CI, or agent enforcement semantics.
- Do not remove `RequireConfirmation`.
- Do not rework scope matching.
- Do not change high-risk capability classification except as needed for tests.
- Do not introduce interactive TUI modals in this pass unless trivial; this pass is mostly CLI/policy ergonomics.

## Decision point: define `--yes` semantics

Choose one of the following and implement consistently.

### Preferred option: `--yes` is prompt suppression, not a universal risk bypass

Under this model:

- `--yes` can proceed only when every required confirmation class is already permitted by a specific `--allow-*` flag, or when the class is low-risk/out-of-scope only.
- High-risk, explicit exclusion, and non-baseline capability require their specific flags.
- This is safer and more precise.

Suggested rule:

```rust
pub fn permits(&self, class: ConfirmationClass) -> bool {
    match class {
        ConfirmationClass::OutOfScope => self.allow_out_of_scope || self.assume_yes,
        ConfirmationClass::TargetExpansion => self.allow_out_of_scope || self.assume_yes,
        ConfirmationClass::PrivateResolution => self.allow_private_resolution,
        ConfirmationClass::CrossHostRedirect => self.allow_cross_host_redirect,
        ConfirmationClass::ExplicitExclusion => self.allow_explicit_exclusion,
        ConfirmationClass::HighRisk => self.allow_high_risk,
        ConfirmationClass::NonBaselineCapability => self.allow_nonbaseline_capability,
    }
}
```

Then `--yes` means “do not prompt for low-risk/manual scope confirmations,” not “override all risk classes.”

### Alternative option: `--yes` is intentionally broad

If you want `--yes` to be a true operator override, update docs and help text to state clearly:

> `--yes` broadly accepts all manual confirmation classes, including high-risk and explicit exclusions. Use with care.

If taking this option, still add tests that prove it is intentionally broad.

## Pass 1: implement chosen `--yes` semantics

Target: `ManualOverride::permits()` in `crates/eggsec/src/config/policy_decision.rs`.

If using preferred option:

- Remove the unconditional `if self.assume_yes { return true; }`.
- Apply class-specific rules as above.
- Keep `assume_yes` useful for low-risk out-of-scope/target-expansion only.

If using broad option:

- Keep implementation, but rename/comment it explicitly as broad.
- Update CLI help text and docs.

Acceptance criteria:

- Tests cover `--yes` with high-risk and explicit exclusion.
- Help text matches actual behavior.

## Pass 2: add dedicated private-resolution and redirect flags

Target: `crates/eggsec/src/cli/mod.rs` and `crates/eggsec-cli/src/main.rs`.

Add global manual-only flags:

```rust
#[arg(
    long,
    global = true,
    help = "Allow target resolution to private/loopback addresses when detected (manual-only)"
)]
pub allow_private_resolution: bool,

#[arg(
    long,
    global = true,
    help = "Allow cross-host redirect/canonicalization boundary changes (manual-only)"
)]
pub allow_cross_host_redirect: bool,
```

Wire them into `ManualOverride`:

```rust
allow_private_resolution: cli.allow_private_resolution,
allow_cross_host_redirect: cli.allow_cross_host_redirect,
```

Stop mapping both fields to `cli.allow_out_of_scope`.

Acceptance criteria:

- CLI help lists dedicated flags.
- `--allow-out-of-scope` no longer implicitly allows private-resolution or cross-host-redirect classes.
- Error messages recommend the dedicated flags.

## Pass 3: improve required-flag error messages

Target: `CommandContext::evaluate_and_enforce_operation()`.

Currently private-resolution and redirect suggest `--allow-out-of-scope (or specific ... override)`.

Update mapping:

- `PrivateResolution` => `--allow-private-resolution`
- `CrossHostRedirect` => `--allow-cross-host-redirect`
- `OutOfScope` / `TargetExpansion` => `--allow-out-of-scope`
- `ExplicitExclusion` => `--allow-excluded-target`
- `HighRisk` => `--allow-high-risk`
- `NonBaselineCapability` => `--allow-nonbaseline-capability`

Also make the message explicit when `--yes` alone is insufficient, if using the preferred option:

```text
manual confirmation required for: HighRisk, NonBaselineCapability. Re-run with --allow-high-risk --allow-nonbaseline-capability. --yes alone does not permit these classes.
```

Acceptance criteria:

- Users get exact flags to unblock manual confirmation.
- Messages do not imply `--allow-out-of-scope` covers private-resolution or redirects.

## Pass 4: stable confirmation class strings

Add a stable string method:

```rust
impl ConfirmationClass {
    pub fn as_str(&self) -> &'static str { ... }
}
```

Recommended values:

- `out-of-scope`
- `explicit-exclusion`
- `high-risk`
- `nonbaseline-capability`
- `private-resolution`
- `cross-host-redirect`
- `target-expansion`

Use this instead of `format!("{:?}", c)` for:

- warnings: `confirmation required: high-risk`
- audit class records
- JSON/manual override class records
- error class lists

Acceptance criteria:

- Audit records use stable kebab-case strings.
- Human messages remain readable.
- Existing tests updated from `Debug` variants to stable strings.

## Pass 5: audit record precision

Review `PolicyDecision::with_manual_override_record(...)` usage.

Ensure override records include:

- `manual_override_used = true`
- override reason if provided
- stable class strings
- no duplicated classes

If duplicates can occur, dedupe before storing.

Suggested helper:

```rust
fn confirmation_class_strings(classes: &[ConfirmationClass]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    classes.iter().filter_map(|c| {
        let s = c.as_str().to_string();
        if seen.insert(s.clone()) { Some(s) } else { None }
    }).collect()
}
```

Acceptance criteria:

- JSON policy decision includes deterministic class ordering or at least no duplicates.
- Logs and decision payloads agree on class names.

## Pass 6: ensure manual override cannot affect non-manual profiles

Add explicit checks/tests that even if CLI flags are present:

- `--strict-scope` + manual override flags still denies.
- CI command + manual override flags still denies.
- MCP/agent have no path to receive these overrides.

Implementation may already behave this way. Add tests so it remains true.

Acceptance criteria:

- `CommandContext` tests prove `RequireConfirmation` under `ManualGuarded` fails despite manual override flags.
- CLI/main profile selection test or handler test proves CI profile ignores overrides.

## Pass 7: tests

Add focused tests.

Manual override semantics:

1. `--yes` behavior matches chosen semantics for `OutOfScope`.
2. `--yes` behavior matches chosen semantics for `HighRisk`.
3. `--yes` behavior matches chosen semantics for `ExplicitExclusion`.
4. `--allow-high-risk` permits `HighRisk` without permitting `ExplicitExclusion`.
5. `--allow-excluded-target` permits `ExplicitExclusion` without permitting `HighRisk`.
6. `--allow-nonbaseline-capability` permits `NonBaselineCapability`.
7. `--allow-private-resolution` permits `PrivateResolution`.
8. `--allow-cross-host-redirect` permits `CrossHostRedirect`.
9. `--allow-out-of-scope` does not permit `PrivateResolution` or `CrossHostRedirect` if dedicated flags are added.

CommandContext behavior:

10. Required flag messages mention exact missing flags.
11. Manual override audit records stable class strings.
12. Strict profile denies `RequireConfirmation` despite all manual override flags.
13. CI profile denies `RequireConfirmation` despite all manual override flags.

Automated boundary:

14. MCP treats `RequireConfirmation` as denial.
15. Agent treats `RequireConfirmation` as denial.

## Pass 8: docs

Update:

- README safety model.
- `docs/SAFETY.md`.
- CLI examples.
- AGENTS notes if they mention override behavior.

Document chosen `--yes` semantics precisely.

If preferred option:

```text
--yes suppresses prompts for low-risk manual confirmations but does not by itself authorize high-risk operations, explicit exclusions, non-baseline capabilities, private-resolution, or cross-host redirects. Use the specific --allow-* flag for those classes.
```

If broad option:

```text
--yes broadly accepts all manual confirmation classes. This is intended for trusted human operators only and is never available to MCP or autonomous-agent execution.
```

Add examples:

```bash
eggsec scan example.com --allow-out-of-scope --manual-override-reason "authorized client range"

eggsec waf-stress https://lab.example --allow-high-risk --allow-nonbaseline-capability --manual-override-reason "Synvoid regression"

eggsec scan https://example.com --allow-cross-host-redirect --manual-override-reason "known redirect boundary"
```

## Pass 9: validation

Run:

```bash
cargo fmt --all
cargo test -p eggsec --lib enforcement
cargo test -p eggsec --lib commands
cargo test -p eggsec --lib mcp
cargo test -p eggsec --lib agent
```

Then run the normal project checks or AGENTS quick-ref checks. Include exact commands in the commit message.

## Final acceptance criteria

This pass is complete when:

- `--yes` behavior is explicit, documented, and tested.
- Dedicated private-resolution and cross-host-redirect flags exist or the broad mapping is intentionally documented.
- Required-flag errors are precise.
- Manual override audit records use stable class strings.
- Manual override flags remain manual-only.
- CI/MCP/agent strictness is unchanged.
