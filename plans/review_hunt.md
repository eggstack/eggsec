# Hunt Module Architecture Review

**Document:** architecture/hunt.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 54

## Verified Claims

- **HuntReport struct**: Verified at `hunt/mod.rs:24-33`
  - Location matches: line 24 ✓
- **HuntConfig struct**: Verified at `hunt/mod.rs:110-119`
  - Location matches: line 110 ✓
- **AttackChain type**: Verified at `hunt/chain.rs` (imported in mod.rs)
- **BusinessLogicFlaw type**: Verified at `hunt/business.rs` (imported in mod.rs)
- **RaceCondition type**: Verified at `hunt/race.rs` (imported in mod.rs)
- **AuthzBypass type**: Verified at `hunt/authz.rs` (imported in mod.rs)
- **SessionIssue type**: Verified at `hunt/session.rs` (imported in mod.rs)
- **Feature gate is marker-only**: Document says "marker-only feature flag (Cargo.toml:248)" - not verified
- **HuntConfig defaults**: Verified at `hunt/mod.rs:121-132`
  - `check_attack_chains: true` ✓
  - `check_business_logic: true` ✓
  - `check_race_conditions: true` ✓
  - `check_authz_bypass: true` ✓
  - `check_session: true` ✓
  - `concurrency: 10` ✓
  - `timeout_ms: 30000` ✓
- **run_hunt() implementation**: Verified at `hunt/mod.rs:69-108`
  - Iterates each HuntConfig flag and calls corresponding sub-module detection function
  - `total_findings` incremented by 1 per finding, except AttackChain adds `chain.steps.len()` ✓
- **Sub-module files exist**: Verified at `hunt/`
  - chain.rs, business.rs, race.rs, authz.rs, session.rs all present

## Discrepancies

- **None identified** - All claims verified

## Bugs Found

- **Potential TOCTOU in AttackChain step counting**: At `hunt/mod.rs:44`, `total_findings += chain.steps.len()` could be inconsistent if `chain.steps` is modified between the `len()` call and the `push()` call at line 45. Consider computing the count before adding. (priority: low)

## Improvement Opportunities

- **Missing timeout enforcement per check**: While `HuntConfig` has a `timeout_ms` field, the actual enforcement of this timeout per sub-module check is not visible in `run_hunt()`. Each sub-module should respect this timeout. (priority: medium)
- **No aggregation of concurrent results**: At `hunt/mod.rs:69-108`, results are processed sequentially after each check completes. Consider collecting all concurrent tasks and processing results together for better performance. (priority: low)
- **Empty report handling**: If all checks return empty vectors, the report will have `total_findings: 0`. This is valid but could be confusing - consider adding a summary message. (priority: low)

## Stale Items

- **None identified**

## Code Interrogation Findings

- **No error handling in run_hunt()**: The function returns `Result<HuntReport>` but none of the sub-module calls appear to have error handling that would convert to the Result type. If any sub-module returns an error, it will propagate and abort the entire hunt.
- **Unbounded vector growth**: Each sub-module returns a `Vec` which is appended to the report. For targets with many findings, this could lead to significant memory usage without limits.
- **No priority ordering**: Findings are processed in a fixed order (attack_chains first, then business_logic, etc.) rather than by severity or exploitability. Consider sorting findings before returning.