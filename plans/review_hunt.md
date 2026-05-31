# Hunt Architecture Review

**Document:** architecture/hunt.md
**Reviewed:** 2026-05-31
**Accuracy:** High
**Lines Reviewed:** 32

## Verified Claims

| Claim | Status | Evidence |
|-------|--------|----------|
| `HuntReport` in `hunt/mod.rs` | ✅ Verified | `crates/slapper/src/hunt/mod.rs:24-33` - struct with `target`, `attack_chains`, `business_logic`, `race_conditions`, `authz_bypasses`, `session_issues`, `total_findings` |
| `HuntConfig` in `hunt/mod.rs` | ✅ Verified | `crates/slapper/src/hunt/mod.rs:110-119` - struct with 5 boolean flags, `concurrency`, `timeout_ms` |
| `AttackChain` in `hunt/chain.rs` | ✅ Verified | `crates/slapper/src/hunt/chain.rs` - module exists, `AttackChain` type used in `mod.rs:27` |
| `BusinessLogicFlaw` in `hunt/business.rs` | ✅ Verified | `crates/slapper/src/hunt/business.rs` - module exists, type used in `mod.rs:28` |
| `RaceCondition` in `hunt/race.rs` | ✅ Verified | `crates/slapper/src/hunt/race.rs` - module exists, type used in `mod.rs:29` |
| `AuthzBypass` in `hunt/authz.rs` | ✅ Verified | `crates/slapper/src/hunt/authz.rs` - module exists, type used in `mod.rs:30` |
| `SessionIssue` in `hunt/session.rs` | ✅ Verified | `crates/slapper/src/hunt/session.rs` - module exists, type used in `mod.rs:31` |
| `run_hunt()` orchestrator | ✅ Verified | `crates/slapper/src/hunt/mod.rs:69-108` - dispatches to all 5 sub-modules based on `HuntConfig` flags |
| Sub-module functions: `detect_attack_chains`, `check_business_logic`, `check_race_conditions`, `check_authz_bypass`, `check_session_security` | ✅ Verified | All 5 functions called in `run_hunt()` at lines 73, 80, 87, 94, 101 |
| "Fully implemented" status | ✅ Verified | All sub-modules have function signatures and `run_hunt()` orchestrates them |
| Feature-gated with `advanced-hunting` | ✅ Verified | `crates/slapper/src/lib.rs:94-98` - `#[cfg(feature = "advanced-hunting")] pub mod hunt;` |

## Discrepancies

### 1. HuntConfig Missing `target` Field

**Severity:** Informational

The document describes `HuntConfig` as "Configuration controlling which hunt categories to run." The actual struct (`hunt/mod.rs:110-119`) has:
- `check_attack_chains: bool`
- `check_business_logic: bool`
- `check_race_conditions: bool`
- `check_authz_bypass: bool`
- `check_session: bool`
- `concurrency: usize`
- `timeout_ms: u64`

The target URL is passed as a separate parameter to `run_hunt(target: &str, config: HuntConfig)`, not stored in `HuntConfig`. This is a design choice, not an error, but the document could clarify that `HuntConfig` controls *what* to check while the target is passed separately.

**Evidence:**
- `crates/slapper/src/hunt/mod.rs:69` - `pub async fn run_hunt(target: &str, config: HuntConfig) -> Result<HuntReport>`
- `crates/slapper/src/hunt/mod.rs:110-119` - `HuntConfig` struct has no `target` field

### 2. Hunt Module is Feature-Gated

**Severity:** Informational

The document does not mention that the hunt module requires the `advanced-hunting` feature flag. This is significant because the module is conditionally compiled:
- `crates/slapper/src/lib.rs:94-98` - `#[cfg(feature = "advanced-hunting")] pub mod hunt;`
- `crates/slapper/src/tui/app/mod.rs:82-83` - `#[cfg(feature = "advanced-hunting")] pub hunt: tabs::HuntTab`

Users who build without `advanced-hunting` will not have access to the hunt module.

**Evidence:**
- `crates/slapper/src/lib.rs:94-98` - feature gate
- `crates/slapper/src/tui/workers/security.rs:11-12` - `#[cfg(feature = "advanced-hunting")]`

## Bugs

No bugs found in the document. All structural claims are accurate.

## Improvements

### 1. Document Feature Gate Requirement

The document should mention that the hunt module requires the `advanced-hunting` feature flag. This is critical for users who want to use the hunt functionality.

### 2. Add Implementation Status Details

The document says "Fully implemented" but doesn't detail what each sub-module actually checks. For example:
- `chain.rs` - What attack chains are detected? (privilege escalation, data exfiltration, RCE)
- `business.rs` - What business logic flaws are checked? (price manipulation, workflow bypass)
- `race.rs` - What race condition patterns are tested? (TOCTOU, double-submit)
- `authz.rs` - What authorization bypasses are tested? (IDOR, privilege escalation)
- `session.rs` - What session issues are checked? (fixation, token leakage)

The document's file descriptions provide brief summaries, but more detail would help users understand the module's capabilities.

### 3. Document HuntConfig Defaults

The document doesn't mention the default values for `HuntConfig`. From the code (`hunt/mod.rs:121-132`):
- All 5 check flags default to `true`
- `concurrency` defaults to `10`
- `timeout_ms` defaults to `30000` (30 seconds)

Adding these defaults would help users understand the out-of-box behavior.

## Stale Items

No stale items found. All claims match current codebase state.
