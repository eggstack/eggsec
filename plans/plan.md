# Slapper Consolidated Implementation Plan

**Created:** 2026-05-30
**Last Updated:** 2026-05-30
**Status:** In Progress

---

## Summary

This document consolidates all remaining implementation plans into a single reference, organized by waves for parallel execution. The original 51-item plan is complete (see History section).

## Wave Organization

| Wave | Components | Status | Dependencies |
|------|------------|--------|--------------|
| **Wave 1** | Documentation Foundation | Pending | None |
| **Wave 2** | Plugin Removal | Pending | None |
| **Wave 3** | MCP/Agent Profiles | Pending | Wave 1 |
| **Wave 4** | Public Release Polish | Pending | Wave 2 |

**Parallelization Strategy:**
- Wave 1A (stale_items) and Wave 1B (reframe docs) can run in parallel
- Wave 3 (agents) can start after Wave 1 completes
- Wave 4 (polish) requires Wave 2 completion first
- Waves 3 and 4 can run in parallel after their prerequisites

---

## Wave 1: Documentation Foundation

### 1A: Stale Items Correction (stale_items.md)

**Purpose:** Fix incorrect statistics and outdated references in architecture documentation.

#### 1A.1 `architecture/overview.md` - Quick Facts Statistics

**Issue:** Quick Facts section contains outdated statistics.

| Statistic | Documented | Actual | Action |
|-----------|------------|--------|--------|
| Modules | 41 | 39 | UPDATE |
| Source files | 743 | 526 | UPDATE |
| Payload types | 31 | 30 | UPDATE |
| NSE libraries | 164+ | 169 | UPDATE |
| Tabs | 29 | 28 | UPDATE |

**Files:** `architecture/overview.md` (lines 5-12), `architecture/tui.md` (lines 3, 23)

#### 1A.2 `architecture/defense_lab.md` - Implementation Status

**Issue:** Claims profiles are "planned but not yet implemented" but all 5 are fully implemented.

**Reality:** All 5 profiles implemented at:
- `DefenseLab` at `cli/mod.rs:262`, `stage.rs:92-98`
- `SynvoidLocal` at `cli/mod.rs:263`, `stage.rs:99-104`
- `WafRegression` at `cli/mod.rs:264`, `stage.rs:105`
- `ProtocolEdge` at `cli/mod.rs:265`, `stage.rs:106`
- `NseSafe` at `cli/mod.rs:266`, `stage.rs:107`

**Files:** `architecture/defense_lab.md` (lines 100-102), `architecture/pipeline.md` (lines 88-100)

#### 1A.3 `architecture/feature_matrix.md` - Feature Counts

**Issue:** Incorrect feature counts.

| Statistic | Documented | Actual |
|-----------|------------|--------|
| Total features | 33 | 28 |
| In `full` | 18 | 16 |

**Files:** `architecture/feature_matrix.md`

#### 1A.4 `architecture/tui.md` - Tab Count

**Issue:** Says "29 tabs" but enum has 28 entries. Line 1111 references non-existent "plugin" tab.

**Files:** `architecture/tui.md` (lines 3, 23, 1111)

#### 1A.5 Line Number References

**Issue:** Various documents have stale line number references.

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

**Files:** Corresponding `architecture/*.md` files

#### 1A.6 Verification

```bash
# Verify tab count
rg "enum Tab" crates/slapper/src/tui/
rg "Tab::" crates/slapper/src/tui/ | wc -l

# Verify module count
ls -la crates/slapper/src/*/ | wc -l

# Verify feature count
rg "^\s*\[\[bin\]\]" crates/slapper/Cargo.toml | wc -l
```

---

### 1B: Strategic Reframe (reframe.md)

**Purpose:** Reframe Slapper from broad offensive/pentest toolkit to Rust-native scoped security assessment and defense-validation engine.

#### 1B.1 Update README.md

**Strategic reframe to adopt:**

> Slapper is a Rust-native security assessment and defense-validation engine for scoped testing of live systems. It combines high-level application security checks, low-level protocol probing, controlled load-bearing tests, WAF evaluation, and optional Nmap NSE compatibility to help developers and security teams understand, reproduce, and harden real attack surfaces. Slapper is designed for authorized testing, local lab validation, and agent-readable regression workflows, not arbitrary exploitation or unscoped scanning.

**Specific edits:**
1. First paragraph with strategic reframe
2. "Why Slapper?" - scoped repeatable testing, Rust-native, WAF/defense validation
3. Feature table relabels:
   - "Stress Testing" → "Controlled stress / load-bearing validation"
   - "WAF bypass" → "WAF evaluation and evasion-resistance testing"
   - "Automation" → "Repeatable assessment and regression profiles"
4. "Intended Use and Guardrails" section (authorized testing, scope files expected, intrusive/stress require opt-in)
5. "Relationship to Nmap/NSE" section
6. "Defense-Lab Mode" section

**Files:** `README.md`

#### 1B.2 Update `architecture/overview.md`

**Changes:**
- Replace "full assessment pipeline from reconnaissance through exploitation" with defense-validation language
- Add "Architectural Principles" section with 7 principles:
  1. Scope enforcement is a core invariant
  2. Slapper-native Rust probes are the curated core
  3. NSE is a compatibility and knowledge layer
  4. Low-level packet/protocol testing belongs in controlled defense-lab workflows
  5. Outputs structured for humans, CI, and agents
  6. Intrusive/stress behavior must be explicit and budgeted
  7. Profiles compile into clear probe plans

**Files:** `architecture/overview.md`

#### 1B.3 Update `architecture/nse_integration.md`

**Changes:**
- Tone down "full integration" claims
- Add "NSE Compatibility Policy" tiers:
  - Tier 1: Safe discovery/version/default scripts
  - Tier 2: Service-specific scripts requiring credentials
  - Tier 3: Intrusive/brute-force requiring explicit opt-in
  - Unsupported: Scripts requiring unrestricted access
- Add "NSE as Knowledge Source" section
- Add "Sandbox Defaults" section

**Files:** `architecture/nse_integration.md`

#### 1B.4 Create `architecture/defense_lab.md`

**New document content outline:**

1. **Purpose:** Local controlled testing against Synvoid, repeatable adversarial traffic, WAF/protocol regression validation
2. **Core workflow:** Build/run Synvoid → run Slapper defense-lab profile → collect observations → compare baseline → convert regressions
3. **Probe categories:** TCP/IP behavior, malformed packets, TLS variants, HTTP smuggling, WAF classification, rate-limit behavior, load-bearing
4. **Safety model:** localhost/private defaults, explicit scope, rate/concurrency budgets, stress behind gates, no unscoped scanning
5. **Output model:** Run manifest, probe suite, observations, findings, latency histograms, baseline diff
6. **Future integration:** Synvoid metrics import, agent loop, golden baselines, CI profiles

**Files:** `architecture/defense_lab.md` (NEW)

#### 1B.5 Create `crates/slapper/src/probe.rs`

**New types:**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProbeRisk {
    Passive,
    SafeActive,
    Intrusive,
    Credentialed,
    Stress,
    ExploitAdjacent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbeMetadata {
    pub id: String,
    pub name: String,
    pub intent: ProbeIntent,
    pub risk: ProbeRisk,
    pub requires_explicit_scope: bool,
    pub requires_budget: bool,
    pub compatibility_source: Option<String>,
}
```

**Implementation:**
- Create `crates/slapper/src/probe.rs`
- Export from `crates/slapper/src/lib.rs`
- Add unit tests for serialization
- Do NOT force existing scanner/fuzzer calls to use it in first pass

#### 1B.6 Add Defense-Lab Profile Metadata

**Profile names to add/document:**
- `defense-lab`: Local/private-scope controlled probe suite
- `synvoid-local`: Localhost/container/private lab for Synvoid validation
- `waf-regression`: WAF payload and evasion-resistance regression
- `protocol-edge`: Malformed protocol, TCP/TLS/HTTP edge behavior
- `nse-safe`: Sandboxed safe/default/version/discovery NSE scripts only

**Note:** If adding real profiles is invasive, add TODO placeholders and docs only.

**Files:** `crates/slapper/src/pipeline/`, `crates/slapper/src/config/`, `crates/slapper/src/cli/`

#### 1B.7 Run Manifest Direction

**Schema direction for `DefenseLabRunManifest`:**
- `schema_version`, `run_id`, `started_at`, `ended_at`
- `slapper_version`, `target_scope`, `profile`, `probe_intents`
- `risk_budget`, `feature_flags`
- `observations`, `findings`, `artifacts`
- `baseline_id`, `diff_summary`

**Files:** `docs/BASELINES_AND_DIFFS.md` or `architecture/output.md`

#### 1B.8 Nmap/NSE Relationship Cleanup

**Search and replace terms:**

| From | To |
|------|----|
| "exploitation" | "assessment", "validation", "controlled adversarial testing" |
| "bypass" | "evasion-resistance testing" |
| "autonomous agent" | "agent-readable orchestration", "scheduled assessment agent" |
| "full NSE integration" | "optional NSE compatibility" |
| "vast majority of NSE scripts" | "broad practical compatibility for useful script categories" |

**Commands:**
```bash
rg "reconnaissance through exploitation" -n
rg "Metasploit clone" -n
rg "full integration" -n
```

#### 1B.9 Verification

```bash
cargo fmt
cargo check --workspace
cargo check -p slapper
cargo check -p slapper-nse --no-default-features
```

---

## Wave 2: Plugin Removal (plugins.md)

**Purpose:** Remove Python/Ruby/Metasploit plugin subsystems, preserve NSE as optional compatibility.

**Critical:** This wave must complete before Wave 4 (Public Release Polish) since README restructuring depends on clean plugin removal.

### Phase 1: Inventory References

```bash
rg -n "python-plugins|ruby-plugins|all-plugins|slapper-plugin|slapper-ruby|PluginArgs|PluginCommand|handle_plugin|PythonPlugin|RubyPlugin|Metasploit|metasploit|Msf|msf|PLUGINS|plugin list|run-plugin|list-plugins|PLUGIN_DEVELOPMENT|PLUGINS.md" .
```

**Classify into 4 groups:**
1. Python/Ruby/Metasploit code to delete
2. Python/Ruby/Metasploit refs to remove from docs/config/examples
3. Generic concepts to rename/migrate if still useful
4. NSE references to keep and reword as compatibility/runtime

### Phase 2: Cargo Workspace Cleanup

**Edit root `Cargo.toml`:**
```toml
# REMOVE:
"crates/slapper-plugin",
"crates/slapper-ruby",

# KEEP:
"crates/slapper-nse",
```

**Edit `crates/slapper/Cargo.toml`:**
```toml
# REMOVE optional deps:
slapper-plugin = { path = "../slapper-plugin", optional = true }
slapper-ruby = { path = "../slapper-ruby", optional = true }

# REMOVE features:
python-plugins = ...
ruby-plugins = ...
all-plugins = ...

# UPDATE full feature:
full = ["stress-testing", "packet-inspection", "rest-api", "nse", "ai-integration", "websocket", ...]
```

### Phase 3: Delete Removed Crates

```bash
rm -rf crates/slapper-plugin
rm -rf crates/slapper-ruby
```

### Phase 4: Remove Plugin CLI Surface

**Remove from `crates/slapper/src/cli/mod.rs`:**
```rust
#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
Plugin(PluginArgs),
```

**Delete:**
```bash
rm crates/slapper/src/commands/handlers/plugin.rs
```

### Phase 5: Remove Plugin Module Exports and TUI Surface

**Remove:**
- `crates/slapper/src/tui/tabs/plugin.rs`
- Plugin tab imports, enum variants, routing/rendering
- Plugin module re-exports in `lib.rs`
- Ruby module re-exports

### Phase 6: Config Cleanup

**Remove from config:**
```toml
[plugins]
[plugins.python]
[plugins.ruby]
paths.plugins_dir
```

**Consider renaming for NSE if needed:**
```rust
nse_scripts_dir
check_packs_dir
```

### Phase 7: Preserve and Reposition NSE CLI

**Reword NSE help text:**
```
Run Nmap NSE-compatible scripts through Slapper's optional Lua/NSE compatibility runtime.
```

### Phase 8: Remove Python/Ruby/Metasploit Docs

```bash
rm docs/PLUGIN_DEVELOPMENT.md
rm docs/PLUGINS.md
```

### Phase 9: README Repositioning

**Remove from README:**
- Python plugin support
- Ruby plugin support
- Ruby plugins, Metasploit integration
- `all-plugins` feature
- Ruby dependency instructions

**Keep/Add in README:**
```text
nse | Optional Nmap NSE-compatible Lua runtime | Approved NSE-compatible discovery/service scripts
nse-sandbox | Restrict Lua/NSE filesystem/process behavior | Safer execution of untrusted NSE scripts
nse-ssh2 | Optional SSH2-backed NSE compatibility | SSH-oriented NSE-compatible checks
```

### Phase 10: Capabilities and Agent/MCP Docs Cleanup

**Update:**
- `docs/CAPABILITIES.md`
- `docs/AGENT.md`
- `docs/API_TESTING.md`
- `docs/NSE_SCRIPTS.md`

### Phase 11: Tests and Snapshots

```bash
# Remove tests covering deleted plugin systems
rg -n "plugin|Plugin|python-plugins|ruby-plugins|slapper-plugin|slapper-ruby|Metasploit|msf" tests

# Update CI matrix entries for plugin features
```

### Phase 12: Final Ripgrep Gate

```bash
rg -n "python-plugins|ruby-plugins|all-plugins|slapper-plugin|slapper-ruby|Python plugin|Ruby plugin|Metasploit|metasploit|MSF|msf|run-plugin|list-plugins|PLUGIN_DEVELOPMENT|PLUGINS.md" .
```

### Phase 13: Build and Test Matrix

```bash
cargo fmt --all --check
cargo check -p slapper
cargo check -p slapper --features nse
cargo check -p slapper --features nse-sandbox
cargo check -p slapper --features nse-ssh2
cargo check -p slapper --features full
cargo test -p slapper
cargo test -p slapper-nse --features nse
```

### Phase 14: Migration Note (Optional)

```markdown
## Python/Ruby Plugin Runtime Removal

Slapper no longer includes Python or Ruby plugin runtimes, including Metasploit-oriented Ruby integration. NSE support remains as optional Nmap NSE compatibility via `nse`, `nse-sandbox`, and `nse-ssh2` features.
```

---

## Wave 3: MCP/Agent Profiles (agents.md)

**Purpose:** Productionize autonomous security agent and MCP as two operating profiles.

### Phase 1: Audit and Encode Profile Contract

**Create `crates/slapper/src/tool/protocol/mcp/policy.rs`:**

```rust
pub struct McpProfilePolicy {
    pub profile: McpProfile,
    pub default_target_policy: TargetPolicy,
    pub allowed_tool_ids: ToolSelector,
    pub denied_tool_ids: ToolSelector,
    pub allowed_capabilities: ToolSelector,
    pub denied_capabilities: ToolSelector,
    pub max_concurrency: usize,
    pub max_timeout_ms: u64,
    pub max_batch_size: usize,
    pub allow_streaming: bool,
    pub allow_sessions: bool,
    pub allow_plan_endpoint: bool,
    pub require_explicit_scope: bool,
    pub allow_external_network: bool,
    pub allow_stress_testing: bool,
    pub allow_packet_features: bool,
    pub allow_broad_recon: bool,
}

pub enum TargetPolicy {
    ExplicitScopeOnly,
    LocalhostAndPrivateCidrsOnly,
    ScopeOrLocalDevOnly,
    AnyWithScopeEngine,
}

pub enum ToolSelector {
    All,
    None,
    Exact(Vec<String>),
    Category(Vec<String>),
    Capability(Vec<String>),
}
```

**Acceptance:**
- Unit tests prove `CodingAgent` cannot call non-visible tools
- Unit tests prove direct `tools/call` with denied tool returns policy error
- `initialize` reports profile metadata from policy

### Phase 2: Filter MCP Tool Discovery by Profile

**Update `McpServer::handle_tools_list`:**

```rust
fn visible_tools_for_profile(&self) -> Vec<ToolInfo> { ... }
```

**Coding-agent default allowlist:**
- Basic HTTP validation of single explicit URL
- Header/security-header inspection
- CORS validation
- TLS/certificate validation
- Endpoint validation with provided URL
- WAF regression (localhost/private CIDR only)
- CVE/technology mapping (user-provided evidence only)

**Coding-agent default deny:**
- Stress/load/flood primitives
- Packet capture/crafting/sending
- Broad subdomain enumeration
- WHOIS/ASN/threat-intel
- Cloud asset enumeration
- SSRF payloads
- Command injection, deserialization exploits
- Root privileges required
- Stealth/evasion mode

**Acceptance:**
- `slapper mcp-serve --stdio --profile coding-agent` returns bounded tool list
- `slapper mcp-serve --stdio --profile ops-agent` returns broader list

### Phase 3: Enforce Profile Policy in `tools/call`

**Add `validate_profile_call()`:**
```rust
fn validate_profile_call(
    &self,
    tool_id: &str,
    capability: Option<&str>,
    arguments: &serde_json::Value,
    target_value: &str,
) -> Result<ValidatedMcpCall, McpError>
```

**Checks:**
- Tool ID allowed for profile
- Capability allowed
- Target policy
- Timeout/concurrency budgets
- Denied arguments (stealth, proxy rotation, raw packet mode, stress modes)

**Acceptance:**
- Coding-agent direct calls to denied tools fail even if client guesses name
- Excessive timeout/concurrency fails or clamps deterministically
- Public internet targets fail without scope

### Phase 4: Formalize Target Scope for Coding-Agent

**Default behavior:**
- Allow: `localhost`, `127.0.0.0/8`, `::1`
- Allow private lab networks only if explicitly enabled or scoped (RFC1918, ULA, Docker bridge)
- Deny public internet by default
- Deny link-local metadata: `169.254.169.254`, cloud metadata hostnames

**Tests required:**
- `localhost`, `127.0.0.1`, `::1`, `10.0.0.5`, `192.168.1.10`, `172.16.0.5`
- `169.254.169.254`, `example.com`
- Decimal/octal/hex IPv4 variants
- Hostnames resolving to private/metadata IPs

### Phase 5: Split Profile-Specific Resource Manifests

**ops-agent resources:**
- `slapper://manifest`, `slapper://tools`, `slapper://vulnerabilities`
- `slapper://ops-agent/safety-policy`, `slapper://ops-agent/task-schema`, `slapper://ops-agent/event-schema`

**coding-agent resources:**
- `slapper://coding-agent/manifest`, `slapper://coding-agent/safety-policy`
- `slapper://coding-agent/finding-schema`, `slapper://coding-agent/workflow`
- `slapper://coding-agent/tool-contracts`

### Phase 6: Productionize MCP Transport Behavior

**HTTP mode:**
- Support single JSON-RPC + batched arrays
- Max batch size profile-policy driven
- Request IDs in tracing spans
- Consistent `Content-Type: application/json`

**STDIO mode:**
- Support single-object JSON-RPC messages
- No logs to stdout (stderr or file only)
- Flush each response line
- Preserve JSON-RPC IDs

**SSE mode:**
- Stream events with request ID, event type, progress
- Bounded backlog, clear lagged event semantics

### Phase 7: Stable Coding-Agent Output Schemas

**New struct `CodingAgentFindingReport`:**
```rust
pub struct CodingAgentFindingReport {
    pub schema_version: String,
    pub target: String,
    pub profile: String,
    pub run_id: String,
    pub started_at: String,
    pub completed_at: String,
    pub status: String,
    pub summary: CodingAgentSummary,
    pub findings: Vec<CodingAgentFinding>,
    pub evidence: Vec<CodingAgentEvidence>,
    pub recommended_next_steps: Vec<String>,
    pub patch_relevance: Vec<PatchRelevanceHint>,
    pub limits: AppliedRuntimeLimits,
}
```

**Do NOT include:** Exploit payload dumps, raw secrets/headers/tokens

### Phase 8: Harden Autonomous Agent Runtime

**Add `AgentRuntimeStatus`:**
```rust
pub struct AgentRuntimeStatus {
    pub running: bool,
    pub started_at: Option<DateTime<Utc>>,
    pub last_tick_at: Option<DateTime<Utc>>,
    pub next_tick_at: Option<DateTime<Utc>>,
    pub portfolio_targets_total: usize,
    pub portfolio_targets_enabled: usize,
    pub last_scan_started_at: Option<DateTime<Utc>>,
    pub last_scan_completed_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub scans_completed: u64,
    pub scans_failed: u64,
    pub alerts_sent: u64,
}
```

**Implementation tasks:**
1. Wire `slapper agent status` to real state
2. Persist runtime metadata for post-crash inspection (JSON state file, atomic write/rename)
3. Add graceful shutdown
4. Add controlled scan budgets (per-target timeout, per-agent concurrency, per-profile depth, per-target cooldown)
5. Config watcher failures don't crash agent loop
6. One bad target doesn't block all scheduled scans

### Phase 9: Make Agent API Routes Production-Safe

**Tasks:**
- Authentication: Support `Authorization: Bearer` and `X-API-Key`, constant-time comparison
- Agent registration: Validate name/capabilities, reject duplicates
- Task creation: Validate task_type, payload schema, max payload size, profile policy for target
- Leasing: Min/max lease durations, capability matching
- Result submission: Max result size, sanitize errors, track metrics
- Lifecycle: Expose health, mark stale agents offline
- Callback URL: SSRF-resistant validation, reject forbidden IP ranges

### Phase 10: Codegg-Specific Server Ergonomics

**Stable invocation:**
```bash
slapper mcp-serve --stdio --profile coding-agent
```

**Optional alias:**
```bash
slapper codegg-mcp --stdio
```

**Sample configs:**
- `examples/codegg-mcp.local.toml`
- `examples/codegg-mcp.scope.toml`

**No AI dependency:** `coding-agent` must be deterministic by default

### Phase 11: Update Documentation

**Files to update:**
- `docs/mcp-protocol.md`
- `docs/AGENT.md`
- `architecture/ai_agents.md`
- `docs/CAPABILITIES.md`
- `.opencode/skills/slapper-agent/mcp_protocol.md`

**Doc changes:**
1. Explain single MCP implementation with multiple profiles
2. Document profile names: `ops-agent`, `coding-agent`
3. Startup examples
4. Coding-agent safety defaults
5. Ops-agent expectations
6. Structured output schemas
7. Migration from old mental model

### Phase 12: Tests and Validation Matrix

**Profile tests:**
- `McpProfile::default() == OpsAgent`
- `coding-agent` serde roundtrip
- Policy for `coding-agent` denies broad categories
- Policy for `ops-agent` preserves broad tools under scope

**Discovery tests:**
- `tools/list` for coding-agent returns only allowed tools
- `tools/list-by-category` for coding-agent filtered
- `resources/list` for coding-agent returns only safe resources
- `resources/read` denies mismatched resources

**Call tests:**
- Coding-agent allowed local validation succeeds
- Coding-agent denied stress/load/packet fails before dispatcher
- Coding-agent public target fails without scope
- Coding-agent excessive timeout/concurrency fails or clamps

**Transport tests:**
- HTTP single request object
- HTTP batch request array
- Batch over profile max
- STDIO single/batch request line
- Malformed JSON-RPC line

**Agent runtime tests:**
- `run_once` updates state
- One failed target doesn't abort remaining
- Shutdown updates running state
- Portfolio save failure surfaced/logged
- Constraint violations skip target and record

**Agent API tests:**
- Bearer auth parsing
- X-API-Key auth parsing
- Invalid callback URL rejection
- Private/loopback callback rejection
- Capability-aware leasing
- Oversized payload/result rejection

### Phase 13: Implementation Order for MiMo 2.5

1. Read "Current repository state to preserve" files
2. Add MCP profile policy module with tests only (no behavior changes)
3. Wire `initialize` safety metadata to policy module
4. Filter `tools/list`, `tools/list-by-category`, manifests by profile
5. Add call-time policy enforcement in `tools/call`
6. Add target policy tests for coding-agent
7. Add single-object JSON-RPC support for HTTP and stdio
8. Add structured coding-agent output wrapper
9. Harden agent status/runtime metadata
10. Harden agent API auth/task validation
11. Update docs
12. Run tests and fix regressions

### Guardrails

- Deny-by-default for coding-agent
- Authorization comes from profile, scope, config, API key (not model/client name)
- Prompts guide clients; enforcement is in Rust code
- No secrets in tool outputs, logs, events, or MCP errors
- No normal logs to stdout in stdio MCP mode
- Do NOT broaden Codegg profile to "make tests pass"
- Do NOT remove NSE support; if exposed, coding-agent defaults to safe categories only
- Do NOT expose stress testing to coding-agent by default

### Definition of Done

- One MCP core with two production-grade profiles
- `ops-agent` supports existing workflows
- `coding-agent` exposes narrow, deterministic, local/scope-bound surface
- Tool discovery AND execution both profile-enforced
- Target scope and budgets enforced at call time
- HTTP and stdio support standard single-request JSON-RPC + batches
- Coding-agent outputs structured for Codegg consumption
- Agent runtime has real status/state persistence and safe shutdown
- Agent API routes production-grade
- Docs clearly explain profile model and Codegg integration

---

## Wave 4: Public Release Polish (polish.md)

**Purpose:** Prepare Slapper for public release. Legally clean, technically reproducible, narratively coherent.

### Phase 1: Repository Identity and Metadata Cleanup

#### 1.1 Fix Repository URLs

Update stale `slapper-tool/slapper` references to `dbowm91/slapper`.

**Files:**
- `Cargo.toml`
- `crates/slapper/Cargo.toml`
- `README.md`
- `CONTRIBUTING.md`
- `SECURITY.md`
- `docs/`

#### 1.2 Normalize Crate Metadata

```toml
description = "Scope-enforced Rust security assessment engine for defense validation and regression testing"

keywords = ["security", "defense-validation", "waf", "scanner", "testing"]
```

Avoid: `pentesting`, `fuzzer`, `vulnerability-scanner`

#### 1.3 Align Rust Version Docs

Update `CONTRIBUTING.md` from 1.70 to match workspace 1.80.

**Acceptance:** `rg "1\.70|Rust 1\.70"` returns no stale claims.

### Phase 2: Legal and Governance Files

#### 2.1 Add License Files

```bash
LICENSE-MIT     # Canonical MIT text
LICENSE-APACHE  # Canonical Apache-2.0 text
LICENSE         # Short dual-license explanation
```

#### 2.2 Add Code of Conduct

Add `CODE_OF_CONDUCT.md` using Contributor Covenant or project-specific policy.

#### 2.3 Rewrite SECURITY.md

**Required content:**
- Authorized-use policy
- How to report vulnerabilities
- Preferred private reporting channel
- Scope controls and safe operation guidance
- Sensitive-data handling guidance

**Remove:**
- `security@slapper-tool.org` unless domain exists
- `https://github.com/slapper-tool/slapper/security/advisories` unless real
- PGP key URL unless real
- "All known vulnerabilities have been fixed" unless backed by audit
- Fixed vulnerability list referencing absent packages

### Phase 3: README Restructure

#### 3.1 Replace README Landing

**Recommended structure:**
1. Title + one-paragraph positioning
2. "What Slapper is" section
3. "What Slapper is not" section
4. Safety model: scope file, config, rate limits, dry-run planning
5. Quick start using localhost only
6. Core workflows (scoped assessment, WAF/defense validation, CI regression, agent/MCP, NSE compatibility)
7. Feature flags table with status labels
8. Installation/build instructions
9. Documentation links
10. Responsible-use notice

#### 3.2 Add "What Slapper is not"

```markdown
## What Slapper is not

Slapper is not an exploitation framework, botnet component, credential attack platform, or tool for unscoped internet scanning. Some modules can generate aggressive traffic or security-test payloads, so advanced capabilities are feature-gated and intended for systems you own, operate, or have explicit authorization to test.
```

#### 3.3 Safe Workflow Examples

```bash
slapper config validate --config slapper.toml
slapper plan --scope examples/scope-localhost.toml --target http://127.0.0.1:8080
slapper scan 127.0.0.1 --profile quick --scope examples/scope-localhost.toml --json
```

#### 3.4 Move Advanced Examples to Docs

**Create:**
- `docs/lab-safety.md`
- `docs/advanced-features.md`
- `docs/cli.md`

**Move out of README:**
- Stress/flood testing
- Proxy pool/Tor examples
- WAF bypass/evasion examples
- Auth brute-force examples
- Distributed scanning examples
- Raw packet operations

### Phase 4: Safety Defaults and CLI Language Audit

#### 4.1 Audit CLI Help Strings

**Changes:**
- `Attack operations` → `Assessment operations` or `Validation operations`
- "Detect and bypass WAFs" → "Evaluate WAF detection and evasion resistance"
- "brute force, credential stuffing, MFA" → "Validate authentication controls in authorized environments"

#### 4.2 Scope-First Enforcement

**Priority commands to check:**
- `fuzz`, `waf`, `waf-stress`, `auth-test`, `stress`, `proxy`, `cluster`, `packet`, `remote`, `exec`

**Recommended:**
- Warn loudly if no scope file for high-risk commands
- Require explicit `--scope` for most dangerous commands
- Always allow `plan` and `doctor` without network

#### 4.3 Review Stealth/Proxy Language

**Describe `--stealth` as:**
> "randomized timing/header behavior for lab realism and false-positive testing"

### Phase 5: Feature Flags and Maturity Labels

#### 5.1 Create Feature Status Table

**Columns:** Feature flag, Status (stable/experimental/stub/planned/lab-only), Purpose, Extra dependencies, Safety notes

**Review flags:**
- `stress-testing`, `packet-inspection`, `nse`, `nse-ssh2`, `nse-sandbox`
- `rest-api`, `grpc-api`, `ai-integration`, `websocket`
- `headless-browser`, `database`, `container`, `sbom`
- `advanced-hunting`, `compliance`, `external-integrations`
- `finding-workflow`, `vuln-management`, `cloud`, `git-secrets`, `wireless`

#### 5.2 Clarify NSE Plugin Boundary

```markdown
Python and Ruby arbitrary plugin runtimes are intentionally not part of Slapper's public extension model. Optional NSE support exists for curated compatibility with Nmap NSE workflows and should be used with sandboxing where possible.
```

### Phase 6: Reproducibility and Packaging

#### 6.1 Commit Cargo.lock

```bash
# Remove from .gitignore
# Commit workspace lockfile
```

#### 6.2 Check Package Contents

```bash
cargo package -p slapper --allow-dirty --list
```

#### 6.3 Add Installation Notes

- Build from source
- `cargo install --path crates/slapper`
- Feature-specific build examples

### Phase 7: CI and Security Workflow Hardening

#### 7.1 Make Audit Behavior Intentional

**Current:** `cargo audit --deny warnings` with `continue-on-error: true`

**Recommended:**
- Fail on vulnerabilities by default
- Use `audit.toml` for documented exceptions only

#### 7.2 Fix Secret Scanning

**Replace:** `pip install gitLeaks` → `gitleaks/gitleaks-action` or pinned binary

#### 7.3 Validate Cargo-Deny Configuration

```bash
cargo deny check
```

#### 7.4 Add PR Hygiene

- `pull_request_template.md`
- Issue templates: bug report, feature request, security concern
- Release checklist

### Phase 8: Documentation Additions

#### 8.1 Add `docs/scope.md`

**Include:**
- What a scope file is
- Allowed/excluded domains, CIDR ranges, port restrictions
- Example localhost scope, example internal lab scope
- How scope is enforced, known limitations

#### 8.2 Add `examples/scope-localhost.toml`

```toml
[scope]
allowed = ["127.0.0.0/8", "localhost", "::1"]
```

#### 8.3 Add `docs/agent-workflows.md`

**Sections:**
- Why agents use Slapper
- Tool/API/MCP surfaces
- Scope-first execution, CI/regression usage
- Scheduled defensive assessments, coding-agent defense-lab usage
- Output formats: JSON, SARIF, JUnit
- Human approval boundaries

#### 8.4 Add `docs/lab-safety.md`

**Include:**
- Stress testing risks, packet/raw socket risks
- WAF evasion-resistance testing risks
- Proxy/Tor risks, auth testing risks
- Rate/concurrency limits, private lab recommendation
- Monitoring and rollback expectations

### Phase 9: Tests and Validation

```bash
cargo fmt --all -- --check
cargo clippy --lib -p slapper -- -D warnings
cargo check -p slapper
cargo check -p slapper --features rest-api
cargo check -p slapper --features nse
cargo check -p slapper --features nse,nse-sandbox
cargo test --lib -p slapper
```

### Phase 10: Final Public-Release Review

```bash
rg "slapper-tool|slapper.dev|slapper-tool.org"
rg "brute force|credential stuffing|bypass|stealth|Tor|flood|DDoS|DoS"
rg "TODO|FIXME|reframe-pass|stub|placeholder"
rg "password|token|secret|api[_-]?key|bearer"
```

### Suggested Commit Sequence

1. `docs: add public repo polish plan`
2. `chore: fix repository metadata and stale URLs`
3. `chore: add license and governance files`
4. `docs: rewrite security policy for pre-1.0 release`
5. `docs: restructure README around scoped defense validation`
6. `docs: add scope, agent workflow, and lab safety docs`
7. `chore: commit Cargo.lock for reproducible builds`
8. `ci: harden audit and secret scanning workflows`
9. `docs: label feature maturity and advanced capabilities`
10. `test: validate public release checks`

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
|---|--------|-------|-----------|
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
