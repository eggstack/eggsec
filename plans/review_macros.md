# Macros Architecture Review

**Document:** architecture/macros.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 50

## Verified Claims
- [run_if_enabled! macro]: Verified at `crates/slapper/src/macros.rs:3-13`
- [stage_task! macro]: Verified at `crates/slapper/src/macros.rs:15-27`
- [recon_stage! macro]: Verified at `crates/slapper/src/macros.rs:29-43`
- [print_if_some! macro]: Verified at `crates/slapper/src/macros.rs:45-52`
- [option_as_result! macro]: Verified at `crates/slapper/src/macros.rs:54-59`
- [format_optional_field helper function]: Verified at `crates/slapper/src/macros.rs:61-65`
- [format_list_field helper function]: Verified at `crates/slapper/src/macros.rs:67-78`

## Discrepancies
- None

## Bugs Found
- None

## Improvement Opportunities
- [Low]: The macro signatures in the documentation show simplified syntax that doesn't fully capture the actual macro patterns (e.g., run_if_enabled! shows `stage` twice which is correct but the documentation format is slightly confusing)
- [Low]: The documentation doesn't mention that `run_if_enabled!` returns `Option<...>` and uses `$crate::recon::set_stage`

## Stale Items
- None

## Code Interrogation Findings
- [Info]: All macros use `#[macro_export]` to make them available at the crate root
- [Info]: `recon_stage!` locks a mutex and uses `unwrap_or_default()` on the body result (macros.rs:39) - could silently swallow errors
- [Info]: `option_as_result!` uses `anyhow::anyhow!` for error creation (macros.rs:57) - requires anyhow dependency
- [Info]: Helper functions are NOT exported with `#[macro_export]` - they're just public functions in the macros module