# MCP/Agent Findings Harness Plan

This document tracks deferred and remaining work items. All completed items have been removed.

---

## Implementation Checklist

### Completed Items

All items from this plan have been implemented:

- **Phase 1 (Harness)**: FuzzResult→Finding conversion, run_cli_with_callback for Fuzzer/Scanner/Recon/Pipeline
- **Phase 2 (Event System)**: trigger_event(), severity type mismatch fixed
- **Phase 3 (Memory)**: Target path collision fixed, resolved_findings implemented, AlertRouter memory leak fixed
- **Phase 4 (MCP Improvements)**: Dead sampling types removed, dynamic vulnerability catalog, session pagination
- **Phase 5 (Alerting)**: Email alerts, PagerDuty alerts, scheduler consumer, lifecycle ping

### Remaining Items (Manual Testing)

- [ ] Phase 1.3: Verify authentication (manual testing)
- [ ] Phase 1.4: Test MCP server (manual testing)

These require manual testing, not code implementation.

---

## Notes

- All `run_cli_with_callback` functions are gated behind `#[cfg(feature = "tool-api")]`
- Tool implementations (ScannerTool, ReconTool, PipelineTool, FuzzerTool) all return findings via callback
- See `AGENTS.md` for implementation history
