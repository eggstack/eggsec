# Implementation Plan

> **Status**: Historical - All items completed and merged. See AGENTS.md for implementation summary.

---

## Completed Implementation (2026-05)

All 47 items from the original plan were completed across 3 waves:

| Wave | Items | Description |
|------|-------|-------------|
| Wave 1 | 6 items | Production safety - panic prevention and critical performance fixes |
| Wave 2 | 13 items | Error handling improvements within specific modules |
| Wave 3 | 28 items | Cleanup, documentation, and optional enhancements |

Key implementations:
- **AI planner.rs** - Clock skew panic prevention with `unwrap_or_else`
- **Tool planner.rs** - HashSet→FxHashSet at 9 locations
- **Fuzzer api.rs** - HashMap→FxHashMap at 10 locations
- **NSE smbauth.rs** - Removed duplicate function definitions
- **Recon email/js.rs** - LazyLock regex `unwrap()`→`expect()` at 32 locations
- **Networking capture.rs** - Error propagation instead of silent suppression
- **Output convert/markdown.rs** - `Result<String, E>` error propagation
- **NSE HashMap replacements** - 7 library files updated to `FxHashMap`

---

## Future Items (Deferred)

These items require design work or are low priority:

| Item | Module | Status | Notes |
|------|--------|--------|-------|
| Per-worker metrics aggregation | `loadtest/runner.rs` | Pending | Atomic counters for lock contention |
| Async file I/O conversion | `pipeline/session.rs` | Acceptable | Current blocking I/O acceptable for infrequent checkpointing |
| `from_utf8_lossy` optimization | `networking/parse_impl.rs` | Informational | May not apply - parse may need allocation |
| CacheKeyBuilder separator collision | `ai/cache.rs` | Informational | Current colon separator unlikely to cause issues |

---

*Archived: 2026-05-28*