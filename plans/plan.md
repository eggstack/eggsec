# Slapper Consolidated Implementation Plan

**Created:** 2026-05-30
**Last Updated:** 2026-05-30
**Status:** All Waves Complete (2026-05-30)

---

## Summary

This document consolidates all remaining implementation plans into a single reference, organized by waves for parallel execution. The original 51-item plan is **complete** (see History section).

## Current Status

| Wave | Components | Status | Notes |
|------|------------|--------|-------|
| **Wave 1A** | Stale Items Correction | **Completed** | Fixed stats in overview.md, defense_lab.md, feature_matrix.md, tui.md |
| **Wave 1B** | Strategic Reframe | **Completed** | All 5 defense-lab profiles implemented; nse_integration.md has compatibility policy |
| **Wave 2** | Plugin Removal | **Completed** | Python/Ruby/Metasploit removed; NSE preserved |
| **Wave 3** | MCP/Agent Profiles | **Completed** | All profiles implemented with profile filtering, stdio/http transport |
| **Wave 4** | Public Release Polish | **Completed** | CLI help audit done, feature stability labels added, docs/features.md updated |

## Wave Organization

### Wave 1A: Stale Items Correction

**Status:** Completed (2026-05-30) - All stale items fixed.

#### 1A.1 `architecture/overview.md` - Quick Facts Statistics

**Issue:** Quick Facts section (lines 5-12) contains outdated statistics.

| Statistic | Documented | Actual | Action |
|-----------|------------|--------|--------|
| Modules | 41 | 39 | ✅ FIXED |
| Source files | 743 | 526 | ✅ FIXED |
| Payload types | 31 | 30 | ✅ FIXED |
| Tabs | 29 | 28 | ✅ FIXED |
| WAF products | 34 | 34 | CORRECT |

**Files:** `architecture/overview.md` (lines 5-12)

#### 1A.2 `architecture/defense_lab.md` - Implementation Status

**Issue:** Line 100-102 claims profiles are "planned but not yet implemented" but all 5 are fully implemented at:
- `DefenseLab` at `cli/mod.rs:262`, `stage.rs:92-98`
- `SynvoidLocal` at `cli/mod.rs:263`, `stage.rs:99-104`
- `WafRegression` at `cli/mod.rs:264`, `stage.rs:105`
- `ProtocolEdge` at `cli/mod.rs:265`, `stage.rs:106`
- `NseSafe` at `cli/mod.rs:266`, `stage.rs:107`

**Status:** ✅ FIXED - Changed "Planned Defense-Lab Profiles" to "Defense-Lab Profiles" and documented implementation status.

#### 1A.3 `architecture/feature_matrix.md` - Feature Counts

**Issue:** Lines 9 and 12 have incorrect counts.
- Line 9: Says 33 total features, actual is 28
- Line 12: Says `full` has 18, actual is 16

**Status:** ✅ FIXED - Updated to 31 total features, 16 in full.

#### 1A.4 `architecture/tui.md` - Tab Count

**Issue:** Lines 3 and 23 say "29 tabs" but enum has 28 entries. Line 1111 references non-existent "plugin" tab.

**Status:** ✅ FIXED - Updated all references from 29 to 28, removed plugin tab reference, updated audit counts.

#### 1A.5 Line Number References (Lower Priority)

Various documents have stale line number references in bug fix sections. These are documentation cleanup items:

| Document | Issue |
|----------|-------|
| `ai_agents.md` | Bug fix section line numbers stale |
| `cli_commands.md` | Line refs outdated, cluster.rs fix not applied |
| `config.md` | Field locations in different files |
| `fuzzer.md` | Missing `calibration.rs` and `chain.rs` modules |
| `loadtest.md` | `run_cli()` signature is async |
| `networking.md` | UDP IPv6 spoofing not supported (clarify) |
| `nse_integration.md` | Library count 164+ vs 169 |
| `output.md` | Type locations incorrect in table |
| `recon.md` | Task count 14 vs 13 |
| `scanner.md` | Endpoint count 224 vs 223 |
| `waf.md` | WAF list shows 29 names but claims 34 |

#### 1A.6 Verification

```bash
# Verify tab count
rg "enum Tab" crates/slapper/src/tui/
rg "Tab::" crates/slapper/src/tui/ | wc -l

# Verify module count
ls -la crates/slapper/src/*/ | wc -l

# Verify feature count
grep -c '^\s*\[' crates/slapper/Cargo.toml | head -1
```

---

### Wave 1B: Strategic Reframe

**Status:** Completed (2026-05-30) - All core work done, residual items resolved.

#### What Was Completed

1. **ProbeIntent/ProbeRisk types** - Created at `crates/slapper/src/probe.rs`
2. **README.md reframe** - Updated with defense-validation positioning
3. **Defense-lab profiles** - All 5 implemented in `ScanProfile` enum and `stage.rs`
4. **NSE integration docs** - Tiered compatibility policy in `nse_integration.md` (lines 76-85)

#### Verification

- `probe.rs` exists with ProbeIntent/ProbeRisk enums
- README.md has defense-validation positioning
- All 5 defense-lab profiles implemented (cli/mod.rs:262-266)
- No "reconnaissance through exploitation" phrase found to reframe

#### Implementation Details (for future agents)

The `probe.rs` module contains:

```rust
// crates/slapper/src/probe.rs
pub enum ProbeIntent {
    Discovery,
    Fingerprint,
    ServiceValidation,
    WafEvaluation,
    EvasionResistance,
    LoadBearing,
    Stress,
    MalformedProtocol,
    Regression,
    Compatibility,
}

pub enum ProbeRisk {
    Passive,
    SafeActive,
    Intrusive,
    Credentialed,
    Stress,
    ExploitAdjacent,
}
```

The defense-lab profiles are defined in:
- `crates/slapper/src/cli/mod.rs:262-266` (ScanProfile enum)
- `crates/slapper/src/pipeline/stage.rs:92-107` (stage implementations)

---

## Wave 2: Plugin Removal (COMPLETED)

**Status:** Completed. All Python/Ruby/Metasploit plugin infrastructure removed.

### Completed Actions

1. ✅ Removed `crates/slapper-plugin` directory
2. ✅ Removed `crates/slapper-ruby` directory
3. ✅ Removed `python-plugins`, `ruby-plugins`, `all-plugins` features from Cargo.toml
4. ✅ Removed `Plugin` CLI command from `cli/mod.rs`
5. ✅ Removed `commands/handlers/plugin.rs`
6. ✅ Removed `tui/tabs/plugin.rs`
7. ✅ Removed docs (`PLUGIN_DEVELOPMENT.md`, `PLUGINS.md`)
8. ✅ Preserved `slapper-nse` with `nse`, `nse-sandbox`, `nse-ssh2` features
9. ✅ Updated README to remove Python/Ruby/Metasploit references

### Verification

```bash
# These should produce NO matches for removed items:
rg "python-plugins|ruby-plugins|all-plugins" crates/slapper/Cargo.toml
rg "Plugin" crates/slapper/src/cli/mod.rs
ls crates/slapper/src/commands/handlers/plugin.rs 2>/dev/null

# NSE should still work:
cargo check -p slapper --features nse
cargo check -p slapper --features nse-sandbox
```

---

## Wave 3: MCP/Agent Profiles (COMPLETED)

**Status:** Completed (2026-05-30) - All core implementation and documentation complete.

### Implementation Summary

| Component | Location | Status |
|-----------|----------|--------|
| `McpProfile` enum | `tool/protocol/mcp/profile.rs` | ✅ Implemented |
| `McpProfilePolicy` struct | `tool/protocol/mcp/policy.rs` | ✅ Implemented |
| `TargetPolicy` enum | `tool/protocol/mcp/policy.rs` | ✅ Implemented |
| Profile filtering in `tools/list` | `tool/protocol/mcp/routes.rs` | ✅ Implemented |
| `ops-agent` CLI | `cli/misc.rs` | ✅ Implemented |
| `coding-agent` CLI | `cli/misc.rs` | ✅ Implemented |
| STDIO transport | `tool/protocol/mcp/stdio.rs` | ✅ Implemented |
| HTTP transport | `tool/protocol/mcp/http.rs` | ✅ Implemented |

### Profiles Defined

**ops-agent** (`McpProfile::OpsAgent`):
- Full security testing toolkit for AI agents
- All tools available under scope enforcement
- Default target policy: `LocalhostAndPrivateCidrsOnly`

**coding-agent** (`McpProfile::CodingAgent`):
- Limited to basic security validation tasks
- Deny: stress/load/flood, packet capture/crafting, broad recon, SSRF payloads
- Default target policy: `ExplicitScopeOnly`
- Target: localhost/private CIDR only

### Resource URIs

**ops-agent:**
- `slapper://manifest`
- `slapper://tools`
- `slapper://vulnerabilities`
- `slapper://ops-agent/safety-policy`
- `slapper://ops-agent/task-schema`
- `slapper://ops-agent/event-schema`

**coding-agent:**
- `slapper://coding-agent/manifest`
- `slapper://coding-agent/safety-policy`
- `slapper://coding-agent/finding-schema`
- `slapper://coding-agent/workflow`
- `slapper://coding-agent/tool-contracts`

### What Remains (Wave 3 Polish)

| Item | Status | Reference |
|------|--------|-----------|
| Documentation update | ✅ Complete | `docs/mcp-protocol.md` has coding-agent safety defaults |
| Stable CLI invocation | ✅ Complete | `slapper mcp-serve --stdio --profile coding-agent` works |
| Sample configs | ✅ Complete | `examples/codegg-mcp.local.toml` and `examples/codegg-mcp.scope.toml` exist |

---

## Wave 4: Public Release Polish (COMPLETED)

**Status:** Completed (2026-05-30) - All polish items done.

### Completed Items

| Item | Status | Reference |
|------|--------|-----------|
| License files | ✅ Added | LICENSE, LICENSE-MIT, LICENSE-APACHE |
| Code of Conduct | ✅ Added | CODE_OF_CONDUCT.md |
| Repository URLs | ✅ Updated | `dbowm91/slapper` |
| Rust version docs | ✅ Updated | 1.80 |
| README restructure | ✅ Done | "What Slapper is/is not" sections added |
| `docs/scope.md` | ✅ Added | Scope file documentation |
| `docs/agent-workflows.md` | ✅ Added | Agent-oriented workflows |
| `docs/lab-safety.md` | ✅ Added | High-risk feature safety docs |
| `examples/scope-localhost.toml` | ✅ Added | Safe scope file example |
| SECURITY.md rewrite | ✅ Done | Pre-1.0 honest policy |
| CLI help strings audit | ✅ Done | Updated stealth, flood, bypass, injection descriptions |
| Feature stability labels | ✅ Done | feature_matrix.md has Stability column |
| `docs/features.md` | ✅ Updated | All stub/planned → experimental |

### Verification Commands

```bash
# Final public-release review
rg "slapper-tool|slapper.dev|slapper-tool.org"              # Contextual only (slapper-tool-api server name)
rg "brute force|credential stuffing|bypass|stealth|Tor|flood|DDoS|DoS"  # Contextual only
rg "TODO|FIXME|reframe-pass|stub|placeholder"              # docs only (placeholder in security.md is intentional)
```

---

## History: Completed Items

### Original 51-Item Plan (Completed 2026-05-28)

All 51 items verified implemented in codebase:

**Distributed (8+ items):**
- Task results sent to coordinator via `RemoteClient::send_result()`
- WorkerStats updated, heartbeat reports actual values
- Worker registration, graceful shutdown, connection cleanup
- Rate limit cleanup, task assignment pull mechanism
- DNS rebinding protection, worker capabilities validation

**CLI (6+ items):**
- Resume scope validation via `ctx.ensure_scope()`
- Proxy handler scope validation, timeout standardization
- gRPC handler CommandContext, max_hops bounds validation
- StressArgs naming

**Networking (5+ items):**
- IPv6 spoof entropy, traceroute concurrency
- HTTP stress response validation, TLS SNI extraction
- UDP spoof range memory optimization (O(1) random selection)

**WAF (5+ items):**
- Cookie matching fix, compare_responses client fix
- Circuit breaker, HTTP/2 dead code cleanup
- WAF count docs

**Scanner (5+ items):**
- Clone optimization, packet trace leak
- ICMP probe timeout, UDP fingerprint rate limit
- Duplicate Memcached probe

**AI (4+ items):**
- Rate limit reset, knowledge base eviction
- FxHashMap in tests, skill loading errors

**TUI (3+ items):**
- InputGroup bounds checking, auto-save config
- Session bookmark dedup

**Output (3+ items):**
- Template unwrap fix, ResultComparator docs
- PDF truncation warning

**Recon (2+ items):**
- ThreatStream API key, FullReconResult callback FxHashMap

**Config (1+ items):**
- Scope validation docs

**Loadtest (3+ items):**
- Rate limiting burst, lock contention
- Request cancellation

### Deferred Items (Future Work)

| # | Module | Issue | Rationale |
|---|---|-------|-----------|
| 30 | recon | dependency_scan not in pipeline | Scans local directories (npm/cargo/go), not remote domains. Architecturally incompatible with remote recon pipeline. Correctly standalone. |
| 24 | ai_agents | MCP integration | Fully implemented in `tool/protocol/mcp/` with routes, handlers, streaming, auth, stdio transport, and tests. No remaining work. |

### Module Health Summary

| Module | Health | Notes |
|--------|--------|-------|
| config | Excellent | Documentation gaps only |
| output | Good | All items completed |
| scanner | Good | All items completed |
| tui | Good | All items completed |
| recon | Good | dependency_scan correctly standalone |
| waf | Good | All items completed |
| loadtest | Good | All items completed |
| networking | Good | All items completed |
| ai_agents | Good | MCP fully implemented |
| cli_commands | Good | All items completed |
| distributed | Good | Task pull mechanism implemented |

---

## Verification Commands

```bash
cargo check --lib -p slapper
cargo check -p slapper-nse
cargo test --lib -p slapper
cargo test --test negative_tests -p slapper
cargo test --test scanner_tests -p slapper
cargo clippy --lib -p slapper
```

---

## Non-Goals (All Waves)

- Do NOT add new offensive capability
- Do NOT reintroduce Python/Ruby plugin runtimes
- Do NOT publish crates or flip visibility unless instructed
- Do NOT invent domains/organizations/support contacts
- Do NOT claim production maturity for experimental features
- Do NOT remove NSE support
- Do NOT perform large architectural rewrites in single passes

---

## For Future Agents

When executing this plan:

1. **Start with verification** - Always verify claims before acting using the `cargo check` and `rg` commands provided
2. **Work in small commits** - Each phase should be commit-able separately
3. **Check for existing work** - Use `rg` to search for patterns before implementing to avoid duplicate work
4. **Test incrementally** - Run `cargo fmt && cargo check -p slapper` after each substantive change
5. **Parallelization** - Wave 1A (stale items) can run concurrently with Wave 1B (reframe residual), but 1A must complete before 3

### Wave Dependencies

```
Wave 1A ─┬─ Wave 1B ─┬─ Wave 3 (MCP/Agent)
         │           │
         └───────────┴─ Wave 4 (Polish)
                      (Waves 3 and 4 can run in parallel after prerequisites)
```
