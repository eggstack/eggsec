# Stale Items Analysis

**Reviewed:** 2026-05-31
**Total Architecture Docs:** 35
**Total Review Files:** 34

## Orphaned Architecture Docs

- `review_plan.md`: Meta-document describing the review process itself. No corresponding source module exists — this is expected as it's a process document, not module documentation.

## Uncovered Source Modules

| Module | Notes |
|--------|-------|
| `auth_context/` | Parses auth context files; no dedicated architecture doc |
| `generated/` | Auto-generated protobuf code (`slapper.tool.v1.rs`); may not need dedicated doc |
| `logging/` | Structured logging with tracing; only mentioned in `overview.md` table |
| `stress/` | Stress testing module; mentioned in `feature_matrix.md` and `cli_commands.md` but no dedicated doc |
| `utils/` | Utility functions; mentioned in `feature_matrix.md` but no dedicated doc |
| `macros.rs` | Utility macros; only mentioned in `overview.md` table |
| `constants.rs` | Constants like `SUPPORTED_WAF_COUNT`; no dedicated doc |
| `types.rs` | Core types (`Severity`, `OutputFormat`); no dedicated doc |
| `probe.rs` | ICMP probing/target classification; referenced by `overview.md` and `defense_lab.md` but no dedicated doc |

## Statistical Drift

| Metric | Documented | Actual | Status |
|--------|-----------|--------|--------|
| Source files | N/A | 523 | ℹ️ No doc makes specific claim |
| Modules | 39 (AGENTS.md) | 39 dirs | ✅ Match |
| `mod` declarations | N/A | 821 | ℹ️ No doc makes specific claim |
| Tabs | 28 (tui.md) | 28 variants | ✅ Match |
| Payload types | 30 (AGENTS.md) | 30 variants | ✅ Match |
| WAF products | 34 (constants.rs) | 34 (`SUPPORTED_WAF_COUNT`) | ✅ Match |
| NSE libraries | 169 (nse_integration.md) | 169 .rs files in libraries/ | ✅ Match |
| Output formats | 8 (AGENTS.md) | 8 (`Pretty`, `Json`, `Compact`, `Html`, `Csv`, `Sarif`, `Junit`, `Markdown`) | ✅ Match |
| CLI commands | "35+ variants" (cli_commands.md) | 36 total variants (42 `#[command]` annotations, 10 feature-gated) | ✅ Match |
| Tests | "1324 base, 1469+" (AGENTS.md) | Not verified in this pass | ℹ️ |

## Duplicate Content

- **MCP integration**: `overview.md` (lines 23, 62, 295-296) has brief table references to MCP. `ai_agents.md` (lines 91-168) has full detailed coverage. This is appropriate — `overview.md` cross-references `ai_agents.md` and doesn't duplicate content.
- **Logging**: `overview.md` (line 158) mentions `logging/` in a table. No other doc covers it. Not duplicated, just minimally documented.
- **Probe types**: `overview.md` (line 99) and `defense_lab.md` (lines 85-98) both reference `probe.rs`. Both appropriately reference the source file rather than duplicating definitions.
- **No significant content duplication found.** Cross-references between docs are done via links, not copy-paste.

## Dead References

- **None found.** All file links in `overview.md`, `review_plan.md`, `ai_agents.md`, and other architecture docs resolve correctly to existing files in `architecture/` or the source tree.
- All referenced types (`CommandContext`, `ScanProfile`, `ToolSelector`, `McpProfile`, `McpProfilePolicy`, `PipelineReport`, `RunManifest`) exist in the codebase.
- All referenced paths (`commands/handlers/`, `tool/protocol/mcp/`, `auth/mod.rs`, etc.) exist.

## Review File Status

All 34 review files exist with meaningful content (28-182 lines each):

| File | Lines | Status |
|------|-------|--------|
| `review_ai_agents.md` | 45 | ✅ |
| `review_auth.md` | 35 | ✅ |
| `review_browser.md` | 30 | ✅ |
| `review_cli_commands.md` | 59 | ✅ |
| `review_compliance.md` | 32 | ✅ |
| `review_config.md` | 65 | ✅ |
| `review_container.md` | 32 | ✅ |
| `review_defense_lab.md` | 42 | ✅ |
| `review_diff.md` | 28 | ✅ |
| `review_distributed.md` | 49 | ✅ |
| `review_error.md` | 88 | ✅ |
| `review_feature_matrix.md` | 70 | ✅ |
| `review_findings.md` | 61 | ✅ |
| `review_fuzzer.md` | 62 | ✅ |
| `review_hunt.md` | 91 | ✅ |
| `review_integrations.md` | 32 | ✅ |
| `review_loadtest.md` | 40 | ✅ |
| `review_networking.md` | 44 | ✅ |
| `review_notify.md` | 36 | ✅ |
| `review_nse_integration.md` | 78 | ✅ |
| `review_output.md` | 78 | ✅ |
| `review_overview.md` | 110 | ✅ |
| `review_pipeline.md` | 68 | ✅ |
| `review_proxy.md` | 42 | ✅ |
| `review_recon.md` | 50 | ✅ |
| `review_scanner.md` | 84 | ✅ |
| `review_storage.md` | 35 | ✅ |
| `review_supply_chain.md` | 33 | ✅ |
| `review_tui.md` | 182 | ✅ |
| `review_vuln.md` | 38 | ✅ |
| `review_waf.md` | 104 | ✅ |
| `review_websocket.md` | 41 | ✅ |
| `review_wireless.md` | 36 | ✅ |
| `review_workflow.md` | 34 | ✅ |
