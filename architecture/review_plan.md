# Architecture Review Plan

**Status:** COMPLETE (Review Phase) — Code fixes pending
**Created:** 2026-05-31
**Purpose:** Systematic review of all 34 architecture documents, verification against codebase, bug/ improvement discovery, and stale item pruning.

---

## Scope

All `.md` files in `architecture/` **except** `review_plan.md` (this file). That is **34 documents** covering the full Slapper codebase.

Each document is assigned to a subagent. Subagents do NOT make code changes. They write improvement plans into `plans/`.

---

## Module-to-Document Mapping

| # | Document | Source Module(s) | Lines | Review Output |
|---|----------|-------------------|-------|---------------|
| 1 | `overview.md` | cross-cutting | 406 | `plans/review_overview.md` |
| 2 | `config.md` | `src/config/` | 110 | `plans/review_config.md` |
| 3 | `cli_commands.md` | `src/cli/`, `src/commands/` | 101 | `plans/review_cli_commands.md` |
| 4 | `error.md` | `src/error/` | 49 | `plans/review_error.md` |
| 5 | `tui.md` | `src/tui/` | 1715 | `plans/review_tui.md` |
| 6 | `output.md` | `src/output/` | 261 | `plans/review_output.md` |
| 7 | `pipeline.md` | `src/pipeline/` | 135 | `plans/review_pipeline.md` |
| 8 | `feature_matrix.md` | cross-cutting | 101 | `plans/review_feature_matrix.md` |
| 9 | `findings.md` | `src/findings/` | 33 | `plans/review_findings.md` |
| 10 | `ai_agents.md` | `src/ai/`, `src/agent/` | 219 | `plans/review_ai_agents.md` |
| 11 | `recon.md` | `src/recon/` | 106 | `plans/review_recon.md` |
| 12 | `defense_lab.md` | cross-cutting | 125 | `plans/review_defense_lab.md` |
| 13 | `fuzzer.md` | `src/fuzzer/` | 121 | `plans/review_fuzzer.md` |
| 14 | `waf.md` | `src/waf/` | 95 | `plans/review_waf.md` |
| 15 | `scanner.md` | `src/scanner/` | 78 | `plans/review_scanner.md` |
| 16 | `nse_integration.md` | `slapper-nse/` | 109 | `plans/review_nse_integration.md` |
| 17 | `hunt.md` | `src/hunt/` | 32 | `plans/review_hunt.md` |
| 18 | `distributed.md` | `src/distributed/` | 93 | `plans/review_distributed.md` |
| 19 | `loadtest.md` | `src/loadtest/` | 140 | `plans/review_loadtest.md` |
| 20 | `networking.md` | `src/packet/`, `utils/network.rs` | 70 | `plans/review_networking.md` |
| 21 | `proxy.md` | `src/proxy/` | 37 | `plans/review_proxy.md` |
| 22 | `websocket.md` | `src/websocket/` | 30 | `plans/review_websocket.md` |
| 23 | `wireless.md` | `src/wireless/` | 25 | `plans/review_wireless.md` |
| 24 | `auth.md` | `src/auth/` | 42 | `plans/review_auth.md` |
| 25 | `browser.md` | `src/browser/` | 30 | `plans/review_browser.md` |
| 26 | `compliance.md` | `src/compliance/` | 29 | `plans/review_compliance.md` |
| 27 | `container.md` | `src/container/` | 31 | `plans/review_container.md` |
| 28 | `diff.md` | `src/diff/` | 23 | `plans/review_diff.md` |
| 29 | `integrations.md` | `src/integrations/` | 31 | `plans/review_integrations.md` |
| 30 | `notify.md` | `src/notify/` | 29 | `plans/review_notify.md` |
| 31 | `storage.md` | `src/storage/` | 27 | `plans/review_storage.md` |
| 32 | `supply_chain.md` | `src/supply_chain/` | 27 | `plans/review_supply_chain.md` |
| 33 | `vuln.md` | `src/vuln/` | 36 | `plans/review_vuln.md` |
| 34 | `workflow.md` | `src/workflow/` | 30 | `plans/review_workflow.md` |

---

## Subagent Dispatch Plan

7 subagents launch in parallel. Each gets a batch of documents grouped by module affinity and size.

### Agent 1 — Core Architecture (4 docs, ~666 lines)
**Documents:** `overview.md`, `config.md`, `cli_commands.md`, `error.md`
**Write to:** `plans/review_overview.md`, `plans/review_config.md`, `plans/review_cli_commands.md`, `plans/review_error.md`
**Focus:** Config loading, CLI dispatch, error taxonomy, cross-cutting claims. Verify `SlapperConfig` fields, command match arms, error enum variants.

### Agent 2 — TUI (1 doc, ~1715 lines)
**Documents:** `tui.md`
**Write to:** `plans/review_tui.md`
**Focus:** Tab count (28+), event loop, key handling, overlays, session persistence, quick switch. This is the largest doc — verify tab enum variants, component structure, state management.

### Agent 3 — Output & Pipeline (4 docs, ~530 lines)
**Documents:** `output.md`, `pipeline.md`, `feature_matrix.md`, `findings.md`
**Write to:** `plans/review_output.md`, `plans/review_pipeline.md`, `plans/review_feature_matrix.md`, `plans/review_findings.md`
**Focus:** Output formats (8), pipeline stages (7), feature flag accuracy, findings schema.

### Agent 4 — AI & Recon (3 docs, ~450 lines)
**Documents:** `ai_agents.md`, `recon.md`, `defense_lab.md`
**Write to:** `plans/review_ai_agents.md`, `plans/review_recon.md`, `plans/review_defense_lab.md`
**Focus:** AI client, MCP integration, provider enum, cache, planner, recon runner, defense-lab profiles.

### Agent 5 — Security Modules (5 docs, ~434 lines)
**Documents:** `fuzzer.md`, `waf.md`, `scanner.md`, `nse_integration.md`, `hunt.md`
**Write to:** `plans/review_fuzzer.md`, `plans/review_waf.md`, `plans/review_scanner.md`, `plans/review_nse_integration.md`, `plans/review_hunt.md`
**Focus:** Payload types (30), WAF products (34), scanner paths (261), NSE libraries (169), probe classification.

### Agent 6 — Network & Infrastructure (6 docs, ~395 lines)
**Documents:** `distributed.md`, `loadtest.md`, `networking.md`, `proxy.md`, `websocket.md`, `wireless.md`
**Write to:** `plans/review_distributed.md`, `plans/review_loadtest.md`, `plans/review_networking.md`, `plans/review_proxy.md`, `plans/review_websocket.md`, `plans/review_wireless.md`
**Focus:** Coordinator/worker protocol, load patterns, raw sockets, packet capture, proxy modes, websocket pub/sub.

### Agent 7 — Supporting Modules (11 docs, ~335 lines)
**Documents:** `auth.md`, `browser.md`, `compliance.md`, `container.md`, `diff.md`, `integrations.md`, `notify.md`, `storage.md`, `supply_chain.md`, `vuln.md`, `workflow.md`
**Write to:** `plans/review_auth.md`, `plans/review_browser.md`, `plans/review_compliance.md`, `plans/review_container.md`, `plans/review_diff.md`, `plans/review_integrations.md`, `plans/review_notify.md`, `plans/review_storage.md`, `plans/review_supply_chain.md`, `plans/review_vuln.md`, `plans/review_workflow.md`
**Focus:** Authentication patterns, headless browser, compliance checks, container detection, diff engine, integration hooks, notification channels, storage backends, supply chain, vuln DB, workflow engine.

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

## Improvement Opportunities
- [Improvement 1]: <Description> (priority: high/medium/low)

## Stale Items
- [Item 1]: <Why it's stale and recommended action>
```

---

## Execution Phases

### Phase 1: Parallel Document Reviews — COMPLETE

7 subagents launched. Each agent read its assigned architecture doc(s), read corresponding source module(s), verified claims against code, and wrote `plans/review_<module>.md` files.

**Result:** 34 review files written, all with meaningful content (28-182 lines each).

### Phase 2: Stale Item Detection — COMPLETE

Analysis of 34 architecture docs against codebase:
- **1 orphaned doc**: `review_plan.md` (meta-document)
- **9 uncovered modules**: `auth_context`, `generated`, `logging`, `stress`, `utils`, `macros.rs`, `constants.rs`, `types.rs`, `probe.rs`
- **All statistics match**: Tabs (28), PayloadType (30), WAF (34), NSE libs (169), Output formats (8), CLI commands (36), Modules (39)
- **No dead references** found
- **No significant duplicate content**

Findings written to `plans/stale_items.md`.

### Phase 3: Consolidation — COMPLETE

1. All 34 review files verified: `ls plans/review_*.md | wc -l` = 34 ✅
2. High-priority items extracted to `plans/review_consolidated.md`
3. `architecture/review_plan.md` updated with final status
4. Ready for commit

**Consolidated Summary:**
- 8 high-priority bugs (incorrect SLA calc, missing Discord dispatch, stub DB, dead auth code, feature count math error, wrong FindingLifecycle type, stale lib.rs docstrings, incorrect feature-gate claim)
- 31 high-priority discrepancies
- 29 high-priority improvements
- 17 stale items requiring action
- Document accuracy: 8 High, 13 Medium, 13 need review

---

## Constraints

- **No code changes**: Reviews identify and document only. Do NOT edit source files.
- **No assumptions**: If a claim cannot be verified, mark it as "UNVERIFIED" with reason.
- **Line references**: All claims must cite `<file:line>` for traceability.
- **Scope**: Only review what the document claims. Don't expand scope beyond the doc's topic.
- **Working directory**: All work stays in `/home/sugarwookie/projects/slapper/`.
- **Subagent writes**: Each subagent writes its own `plans/review_*.md` files. Do not overwrite another agent's output.

---

## Notes

- Cross-cutting docs (`overview.md`, `feature_matrix.md`, `defense_lab.md`) require checking against ALL modules, not just one.
- `tui.md` is the largest doc (1715 lines); its agent should focus on structural claims (tab count, event loop, state management) rather than pixel-level details.
- `nse_integration.md` spans a separate crate (`slapper-nse/`); agent must check both crates.
- Feature flags in `Cargo.toml` at root and `crates/slapper/Cargo.toml` must be cross-referenced for `feature_matrix.md`.
- The previous review_plan.md claimed Phase 1 and Phase 2 were COMPLETE but no `plans/*_review.md` files exist. This was a fresh start. All phases now complete with 34 review files, stale_items.md, and review_consolidated.md.

---

## File Cleanup

After all reviews are complete and consolidated:

1. Verify no stale review files exist from prior runs
2. Remove any `plans/*_review.md` files that don't match the `plans/review_*.md` naming convention
3. Archive or remove `plans/plan.md` if all items are resolved (check first)
