# Agent Architecture Plan

This document tracks deferred and remaining work items. All completed items have been removed.

---

## Implementation Checklist

### Completed Items

All code items from this plan have been implemented:

- **Phase 1 (MCP Server)**: HTTP server wired up, STDIO mode implemented
- **Phase 2 (Agent Core)**: Agent core, TargetPortfolio, LongitudinalMemory, AlertRouter, Event system, Agent CLI command
- **Phase 3 (Skill System)**: SkillLoader, SkillRegistry
- **Phase 4 (Search)**: SearchTool implemented, SearchConfig added to SlapperConfig, search result types
- **Phase 5 (Alerting)**: Alert integration, webhook reuse

### Remaining Items (Manual Testing Only)

- [ ] Phase 1.3: Verify authentication (manual testing)
- [ ] Phase 1.4: Test MCP server (manual testing)

These items require manual testing, not code implementation.

---

## Notes

- Agent core is feature-gated behind `#[cfg(feature = "rest-api")]`
- Skill system is feature-gated behind `#[cfg(feature = "ai-integration")]`
- See `AGENTS.md` for implementation history and lessons learned
