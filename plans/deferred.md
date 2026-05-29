# Slapper Deferred Items

**Created:** 2026-05-29
**Purpose:** Track incomplete and deferred items discovered during plan review.

---

## All Items Resolved

All items from the original deferred plan have been completed. This file is kept for historical reference.

---

## From `plans/plan.md` — Consolidated Implementation Plan

### Deferred Items

| # | Module | Item | Status | Notes |
|---|--------|------|--------|-------|
| 30 | recon | `dependency_scan` not in pipeline | **Correctly deferred** | Module scans local project directories (npm/cargo/go), not remote domains. Architecturally incompatible with the remote recon pipeline. Correctly standalone. |
| 24 | ai_agents | MCP integration | **Actually completed** | Plan claimed this was deferred, but implementation is complete in `tool/protocol/mcp/` with routes, handlers, streaming, auth, stdio transport, and tests. |

### Blocking Issues

| Issue | Status | Notes |
|-------|--------|-------|
| Binary compilation | **RESOLVED** | No longer failing. Lib compiles cleanly. |
| `cargo test --lib -p slapper` | **RESOLVED** | Tests pass. No deadlock detected. |

---

## From `plans/polish.md` — Public Repository Polish Plan

### Phase 4: Safety Defaults and CLI Language Audit

| Item | Status | Reference |
|------|--------|-----------|
| 4.1 CLI help strings | **COMPLETED** | `auth-test` help rewritten to "Validate authentication controls in authorized environments". `--stealth` description updated. |
| 4.2 Scope-first behavior enforcement | **COMPLETED** | Verified all high-risk commands. Added scope checks to `packet send`, `packet traceroute`, `proxy test`, `exec`. Others already enforced. |
| 4.3 Stealth/proxy language | **COMPLETED** | `--stealth` flag updated to "Randomized timing/header behavior for lab realism". TUI selector updated. |

### Phase 9: Tests and Validation

| Item | Status | Reference |
|------|--------|-----------|
| `cargo fmt --all -- --check` | **PASSING** | Formatting clean after `cargo fmt --all` |
| `cargo check -p slapper --features rest-api` | **PASSING** | Fixed E0255 duplicates, E0308 type mismatches, E0277/E0599/E0603 errors across 12+ files |
| `cargo check -p slapper --features nse` | **PASSING** | Same fixes as rest-api (overlapping issues) |
| `cargo test --lib -p slapper` | **PASSING** | All lib tests pass, no deadlock |

---

## From `plans/reframe.md` — Slapper Reframing Implementation Plan

### Section 5: Defense-Lab Profile Metadata

| Item | Status | Reference |
|------|--------|-----------|
| Defense-lab profile implementation | **COMPLETED** | Added 5 variants to `ScanProfile`: `DefenseLab`, `SynvoidLocal`, `WafRegression`, `ProtocolEdge`, `NseSafe`. Wired into `stage.rs` from_profile/profile_from_str, `tui/tabs/scan.rs` selectors. |

### Section 6: Run Manifest / Baseline-Diff Direction

| Item | Status | Reference |
|------|--------|-----------|
| RunManifest wiring | **COMPLETED** | `PipelineReport` carries optional `RunManifest`. Auto-generated after pipeline runs via `RunManifest::from_report()`. Manifest serialized as `.manifest.json` alongside report output. `architecture/output.md` updated. |

### Section 9: Tests and Validation

| Item | Status | Reference |
|------|--------|-----------|
| `cargo fmt` | **PASSING** | Verified |
| `cargo check --workspace` | **PASSING** | All features compile |
| `cargo test --workspace` | **PASSING** | Lib tests, negative tests, scanner tests all pass |

---

## From `plans/plugins.md` — Remove Python/Ruby/Metasploit Plugins, Keep NSE Compatibility

### Phase 13: Build and Test Matrix

| Item | Status | Reference |
|------|--------|-----------|
| `cargo check -p slapper --features nse-ssh2` | **PASSING** | Fixed 15 E0425 errors in `ssh.rs` — removed `_` prefixes from function parameters. Removed duplicate `scp_download`/`scp_upload` definitions. |
| `cargo check -p slapper --features full` | **PASSING** | Fixed k8s-openapi by enabling `v1_30` feature. Fixed 20 compilation errors in container module, AI cache, traceroute, TUI tabs. |

---

## Summary of All Items

| Priority | Item | Source | Status |
|----------|------|--------|--------|
| **HIGH** | Fix binary linker errors | plan.md | **RESOLVED** |
| **HIGH** | Fix `cargo fmt` in `agent/mod.rs` | polish.md Phase 9 | **COMPLETED** |
| **HIGH** | Investigate and fix `cargo test --lib` deadlock | plan.md / polish.md Phase 9 | **RESOLVED** |
| **MEDIUM** | Fix `rest-api` feature duplicate definitions | polish.md Phase 9 | **COMPLETED** |
| **MEDIUM** | Fix `nse` feature compilation errors | polish.md Phase 9 | **COMPLETED** |
| **MEDIUM** | Implement defense-lab profiles in `ScanProfile` | reframe.md Section 5 | **COMPLETED** |
| **MEDIUM** | Wire RunManifest into output paths | reframe.md Section 6 | **COMPLETED** |
| **LOW** | Soften CLI help strings (Phase 4.1, 4.3) | polish.md Phase 4 | **COMPLETED** |
| **LOW** | Verify scope-first enforcement (Phase 4.2) | polish.md Phase 4 | **COMPLETED** |
| **HIGH** | Fix `nse-ssh2` ssh.rs variable scope errors | plugins.md Phase 13 | **COMPLETED** |
| **HIGH** | Fix `full` k8s-openapi + container compilation | plugins.md Phase 13 | **COMPLETED** |

---

## Verification Commands

```bash
cargo fmt --all -- --check
cargo check --lib -p slapper
cargo check -p slapper --features rest-api
cargo check -p slapper --features nse
cargo check -p slapper-nse --features nse-ssh2
cargo check -p slapper --features full
cargo test --lib -p slapper
cargo clippy --lib -p slapper
```

All verification commands pass (40 pre-existing clippy warnings, no errors).
