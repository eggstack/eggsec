# Architecture Review Plan

**Status:** COMPLETED
**Created:** 2026-06-02
**Completed:** 2026-06-02
**Purpose:** Systematic review of all architecture documents, verification against codebase, bug/improvement discovery, and stale item pruning.

---

## Scope

All `.md` files in `architecture/` **except** `review_plan.md`. This excludes this meta-document itself.

**Total documents:** 46

---

## Module-to-Document Mapping

| # | Document | Source Module(s) | Lines | Review Output |
|---|----------|-------------------|-------|---------------|
| 1 | `overview.md` | cross-cutting | ~800 | `plans/review_overview.md` |
| 2 | `config.md` | `src/config/` | ~110 | `plans/review_config.md` |
| 3 | `cli_commands.md` | `src/cli/`, `src/commands/` | ~101 | `plans/review_cli_commands.md` |
| 4 | `error.md` | `src/error/` | ~49 | `plans/review_error.md` |
| 5 | `tui.md` | `src/tui/` | ~1715 | `plans/review_tui.md` |
| 6 | `output.md` | `src/output/` | ~261 | `plans/review_output.md` |
| 7 | `pipeline.md` | `src/pipeline/` | ~135 | `plans/review_pipeline.md` |
| 8 | `feature_matrix.md` | cross-cutting | ~101 | `plans/review_feature_matrix.md` |
| 9 | `findings.md` | `src/findings/` | ~33 | `plans/review_findings.md` |
| 10 | `ai_agents.md` | `src/ai/`, `src/agent/` | ~219 | `plans/review_ai_agents.md` |
| 11 | `recon.md` | `src/recon/` | ~106 | `plans/review_recon.md` |
| 12 | `defense_lab.md` | cross-cutting | ~125 | `plans/review_defense_lab.md` |
| 13 | `fuzzer.md` | `src/fuzzer/` | ~121 | `plans/review_fuzzer.md` |
| 14 | `waf.md` | `src/waf/` | ~95 | `plans/review_waf.md` |
| 15 | `scanner.md` | `src/scanner/` | ~78 | `plans/review_scanner.md` |
| 16 | `nse_integration.md` | `slapper-nse/` | ~109 | `plans/review_nse_integration.md` |
| 17 | `hunt.md` | `src/hunt/` | ~32 | `plans/review_hunt.md` |
| 18 | `distributed.md` | `src/distributed/` | ~93 | `plans/review_distributed.md` |
| 19 | `loadtest.md` | `src/loadtest/` | ~140 | `plans/review_loadtest.md` |
| 20 | `networking.md` | `src/packet/`, `utils/network.rs` | ~70 | `plans/review_networking.md` |
| 21 | `proxy.md` | `src/proxy/` | ~37 | `plans/review_proxy.md` |
| 22 | `websocket.md` | `src/websocket/` | ~30 | `plans/review_websocket.md` |
| 23 | `wireless.md` | `src/wireless/` | ~25 | `plans/review_wireless.md` |
| 24 | `auth.md` | `src/auth/` | ~42 | `plans/review_auth.md` |
| 25 | `browser.md` | `src/browser/` | ~30 | `plans/review_browser.md` |
| 26 | `compliance.md` | `src/compliance/` | ~29 | `plans/review_compliance.md` |
| 27 | `container.md` | `src/container/` | ~31 | `plans/review_container.md` |
| 28 | `diff.md` | `src/diff/` | ~23 | `plans/review_diff.md` |
| 29 | `integrations.md` | `src/integrations/` | ~31 | `plans/review_integrations.md` |
| 30 | `notify.md` | `src/notify/` | ~29 | `plans/review_notify.md` |
| 31 | `storage.md` | `src/storage/` | ~27 | `plans/review_storage.md` |
| 32 | `supply_chain.md` | `src/supply_chain/` | ~27 | `plans/review_supply_chain.md` |
| 33 | `vuln.md` | `src/vuln/` | ~36 | `plans/review_vuln.md` |
| 34 | `workflow.md` | `src/workflow/` | ~30 | `plans/review_workflow.md` |
| 35 | `auth_context.md` | `src/auth/` | ~27 | `plans/review_auth_context.md` |
| 36 | `constants.md` | cross-cutting | ~32 | `plans/review_constants.md` |
| 37 | `types.md` | `src/types/` | ~35 | `plans/review_types.md` |
| 38 | `utils.md` | `src/utils/` | ~30 | `plans/review_utils.md` |
| 39 | `probe.md` | `src/probe.rs` | ~28 | `plans/review_probe.md` |
| 40 | `stress.md` | `src/stress/` | ~28 | `plans/review_stress.md` |
| 41 | `macros.md` | `src/macros.rs` | ~22 | `plans/review_macros.md` |
| 42 | `logging.md` | `src/logging/` | ~18 | `plans/review_logging.md` |
| 43 | `generated.md` | generated protobuf | ~12 | `plans/review_generated.md` |
| 44 | `notify.md` | `src/notify/` | ~29 | `plans/review_notify.md` |
| 45 | `container.md` | `src/container/` | ~31 | `plans/review_container.md` |
| 46 | `compliance.md` | `src/compliance/` | ~29 | `plans/review_compliance.md` |

---

## Subagent Dispatch Plan

8 subagents launch in parallel. Each gets a batch of documents grouped by module affinity and size.

### Agent 1 — Core Architecture (4 docs)
**Documents:** `overview.md`, `config.md`, `cli_commands.md`, `error.md`
**Write to:** `plans/review_overview.md`, `plans/review_config.md`, `plans/review_cli_commands.md`, `plans/review_error.md`
**Focus:** Config loading, CLI dispatch, error taxonomy, cross-cutting claims. Verify `SlapperConfig` fields, command match arms, error enum variants.

### Agent 2 — TUI (1 doc, largest)
**Documents:** `tui.md`
**Write to:** `plans/review_tui.md`
**Focus:** Tab count (28+), event loop, key handling, overlays, session persistence, quick switch. Verify tab enum variants, component structure, state management.

### Agent 3 — Output & Pipeline (4 docs)
**Documents:** `output.md`, `pipeline.md`, `feature_matrix.md`, `findings.md`
**Write to:** `plans/review_output.md`, `plans/review_pipeline.md`, `plans/review_feature_matrix.md`, `plans/review_findings.md`
**Focus:** Output formats (8), pipeline stages (7), feature flag accuracy, findings schema.

### Agent 4 — AI & Recon (3 docs)
**Documents:** `ai_agents.md`, `recon.md`, `defense_lab.md`
**Write to:** `plans/review_ai_agents.md`, `plans/review_recon.md`, `plans/review_defense_lab.md`
**Focus:** AI client, MCP integration, provider enum, cache, planner, recon runner, defense-lab profiles.

### Agent 5 — Security Modules (5 docs)
**Documents:** `fuzzer.md`, `waf.md`, `scanner.md`, `nse_integration.md`, `hunt.md`
**Write to:** `plans/review_fuzzer.md`, `plans/review_waf.md`, `plans/review_scanner.md`, `plans/review_nse_integration.md`, `plans/review_hunt.md`
**Focus:** Payload types (30), WAF products (34), scanner paths (261), NSE libraries (169), probe classification.

### Agent 6 — Network & Infrastructure (6 docs)
**Documents:** `distributed.md`, `loadtest.md`, `networking.md`, `proxy.md`, `websocket.md`, `wireless.md`
**Write to:** `plans/review_distributed.md`, `plans/review_loadtest.md`, `plans/review_networking.md`, `plans/review_proxy.md`, `plans/review_websocket.md`, `plans/review_wireless.md`
**Focus:** Coordinator/worker protocol, load patterns, raw sockets, packet capture, proxy modes, websocket pub/sub.

### Agent 7 — Supporting Modules (11 docs)
**Documents:** `auth.md`, `browser.md`, `compliance.md`, `container.md`, `diff.md`, `integrations.md`, `notify.md`, `storage.md`, `supply_chain.md`, `vuln.md`, `workflow.md`
**Write to:** `plans/review_auth.md`, `plans/review_browser.md`, `plans/review_compliance.md`, `plans/review_container.md`, `plans/review_diff.md`, `plans/review_integrations.md`, `plans/review_notify.md`, `plans/review_storage.md`, `plans/review_supply_chain.md`, `plans/review_vuln.md`, `plans/review_workflow.md`
**Focus:** Authentication patterns, headless browser, compliance checks, container detection, diff engine, integration hooks, notification channels, storage backends, supply chain, vuln DB, workflow engine.

### Agent 8 — Utility & Type Modules (7 docs)
**Documents:** `auth_context.md`, `constants.md`, `types.md`, `utils.md`, `probe.md`, `stress.md`, `macros.md`, `logging.md`, `generated.md`
**Write to:** `plans/review_auth_context.md`, `plans/review_constants.md`, `plans/review_types.md`, `plans/review_utils.md`, `plans/review_probe.md`, `plans/review_stress.md`, `plans/review_macros.md`, `plans/review_logging.md`, `plans/review_generated.md`
**Focus:** Verify type definitions, constants values, utility functions, probe classification, stress testing, macro definitions, logging configuration, generated protobuf code.

---

## Subagent Instructions

Each subagent MUST:

1. **Read the architecture document(s)** assigned to it
2. **Read the corresponding source module(s)** in `crates/slapper/src/` (or `slapper-nse/` for NSE)
3. **Run the Review Checklist** (below) against every claim in the document
4. **Write findings** to the designated `plans/review_<module>.md` file(s)

### Review Checklist

For each document, verify:

- [ ] **File paths**: All referenced file paths exist in the codebase
- [ ] **Line numbers**: Cited line numbers match actual code locations
- [ ] **Type definitions**: All `struct`, `enum`, `trait` names exist and match signatures
- [ ] **Method signatures**: Documented methods exist with correct parameters and return types
- [ ] **Error handling**: Described error patterns are actually implemented
- [ ] **Configuration**: Schema details, defaults, and environment variables are current
- [ ] **Dependencies**: Referenced crates and feature flags are accurate
- [ ] **Known issues**: Any "TODO", "known limitation", or "planned" items still apply
- [ ] **Undocumented changes**: New patterns, optimizations, or APIs not yet in docs
- [ ] **Deprecated content**: Patterns marked deprecated that should be removed from doc
- [ ] **Statistics**: Counts of modules, files, tabs, payloads, etc. match reality
- [ ] **Cross-references**: Links between architecture docs are valid
- [ ] **Code interrogated**: Search for potential bugs, edge cases, missing error handling, performance issues, security concerns, and improvement opportunities

### Output Format

Each review file MUST use this structure:

```markdown
# <Module> Architecture Review

**Document:** architecture/<module>.md
**Reviewed:** <date>
**Accuracy:** <High/Medium/Low>
**Lines Reviewed:** <N>

## Verified Claims
- [Claim 1]: Verified at <file:line>
- [Claim 2]: Verified at <file:line>

## Discrepancies
- [Claim X]: Documented as <X>, but actual implementation is <Y> (<file:line>)

## Bugs Found
- [Bug 1]: <Description> (<file:line>)
- [Bug 2]: <Description> (<file:line>)

## Improvement Opportunities
- [Improvement 1]: <Description> (priority: high/medium/low)
- [Improvement 2]: <Description> (priority: high/medium/low)

## Stale Items
- [Item 1]: <Why it's stale and recommended action>

## Code Interrogation Findings
- [Finding 1]: <Potential issue discovered in code>
```

---

## Phase 1: Parallel Document Reviews

8 subagents launch in parallel. Each agent reads its assigned architecture doc(s), reads corresponding source module(s), verifies claims against code, interrogates code for bugs and improvements, and writes `plans/review_<module>.md` files.

**Output:** 46 review files in `plans/` directory.

---

## Phase 2: Stale Item Detection

After all reviews complete, a consolidation agent will:

1. **Scan for orphaned documents**: Architecture docs that reference modules that no longer exist
2. **Check for missing documents**: Modules that exist but have no architecture doc
3. **Verify statistics**: Tab count, payload types, WAF products, NSE libraries, output formats, CLI commands, modules
4. **Cross-reference consistency**: Ensure docs don't contradict each other
5. **Identify deprecated content**: Items marked as deprecated/stub/TODO that are no longer accurate

**Output:** `plans/stale_items.md` with findings and recommended actions.

---

## Phase 3: Consolidation

1. Verify all 46 review files exist: `ls plans/review_*.md | wc -l`
2. Extract high-priority items to `plans/review_consolidated.md`
3. Update this document with final status
4. Commit to main

---

## Constraints

- **No code changes**: Reviews identify and document only. Do NOT edit source files.
- **No assumptions**: If a claim cannot be verified, mark it as "UNVERIFIED" with reason.
- **Line references**: All claims must cite `<file:line>` for traceability.
- **Scope**: Only review what the document claims. Don't expand scope beyond the doc's topic.
- **Working directory**: All work stays in `/home/sugarwookie/projects/slapper/`.
- **Subagent writes**: Each subagent writes its own `plans/review_*.md` files. Do not overwrite another agent's output.
- **Improvement plans only**: `plans/review_*.md` files contain findings and recommendations, not direct code changes.

---

## Notes

- Cross-cutting docs (`overview.md`, `feature_matrix.md`, `defense_lab.md`) require checking against ALL modules, not just one.
- `tui.md` is the largest doc (~1715 lines); its agent should focus on structural claims (tab count, event loop, state management) rather than pixel-level details.
- `nse_integration.md` spans a separate crate (`slapper-nse/`); agent must check both crates.
- Feature flags in `Cargo.toml` at root and `crates/slapper/Cargo.toml` must be cross-referenced for `feature_matrix.md`.
- Several new architecture docs were added in Wave 7: `auth_context.md`, `constants.md`, `types.md`, `utils.md`, `probe.md`, `stress.md`, `macros.md`, `logging.md`, `generated.md` - ensure these are reviewed for accuracy.

---

## File Cleanup

Before committing, verify and clean up:

1. Remove any `plans/*_review.md` files that don't match the `plans/review_*.md` naming convention
2. Ensure no stale review files exist from prior runs
3. Check `plans/plan.md` - archive or remove if all items are resolved

---

## Execution Summary

- [x] Phase 1: 8 subagents complete document reviews (43 docs → 43 review files)
- [x] Phase 2: Stale item detection and reporting → `plans/stale_items.md`
- [x] Phase 3: Consolidation and commit to main → `plans/review_consolidated.md`

---

## Key Findings Summary

### Critical Issues (4)
1. **Defense-Lab stage counts incorrect** - `pipeline.md:136-142` shows wrong stage counts for all 5 profiles
2. **Storage module is stub** - `storage/postgres.rs:6-7` is not connected to real database
3. **VulnAssessment is stub** - `vuln/mod.rs:37-40` cannot hold structured findings
4. **Docker shell injection risk** - `container/docker.rs:208-209` vulnerable to command injection

### Documentation Accuracy Issues (5)
6. **k8s-openapi warning stale** - `feature_matrix.md:58-63` describes resolved issue
7. **Recon module count 17 vs 18** - `recon.md` text says 17 but array has 18
8. **Error line reference wrong** - `error.md:56` says mod.rs:56 but actual is mod.rs:82
9. **AI agents path missing prefix** - `ai_agents.md` says `alerts/routing.rs` should be `agent/alerts/routing.rs`
10. **StressConfig names wrong** - `stress.md` says `rate_limit` and `threads` but actual is `rate_pps` and `concurrency`

### Cross-Cutting Concerns
- **23 HIGH priority items** identified across 43 reviewed documents
- **Multiple silent error suppression** patterns using `let _ =` that should log warnings
- **Several modules are stubs** (storage, vuln) that need implementation or documentation updates
- **Historical bug fix tables** in tui.md and scanner.md are stale and should be archived

### Statistical Findings
- **43 architecture docs** reviewed (46 planned, but 3 were duplicates in mapping table)
- **41 modules** in `crates/slapper/src/`
- **169 NSE libraries** in `slapper-nse/src/libraries/`
- **34 WAF products** verified
- **30 fuzzing payload types** verified
- **261 scanner endpoints** verified
- **28 tabs** in TUI (20 base + 8 feature-gated)

### Top 4 Recommended Actions
1. Fix defense-lab stage counts in `architecture/pipeline.md`
2. Document storage as stub or implement SQLx integration
3. Implement proper VulnAssessment or document as placeholder
4. Add input validation for Docker image names