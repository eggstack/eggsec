# Architecture Review Summary

**Review Date:** 2026-06-09

## Aggregate Statistics

| Metric | Count |
|--------|-------|
| Documents Reviewed | 11 |
| Total Verified Claims | 227 |
| Total Discrepancies | 47 |
| Total Bugs Found | 32 |
| Total Improvement Opportunities | 72 |

## Bug Counts by Severity

| Severity | Count |
|----------|-------|
| HIGH | 6 |
| MEDIUM | 13 |
| LOW | 13 |

### HIGH Severity Bugs

| Bug | Module | Description |
|-----|--------|-------------|
| BUG 3 | distributed | Task results never sent to coordinator - entire result aggregation system is non-functional |
| BUG 1 | distributed | WorkerStats never updated - coordinator gets no useful worker load info |
| BUG 2 | distributed | Heartbeat reports hardcoded zero values regardless of actual state |
| B1 | cli_commands | Resume command bypasses scope validation - session files can contain out-of-scope targets |
| B2 | cli_commands | Stress handler missing scope validation |
| Bug 1 | loadtest | Rate limiting initial state causes burst exceeding intended rate |

### MEDIUM Severity Bugs

| Bug | Module | Description |
|-----|--------|-------------|
| Bug 1 | recon | ThreatStream API key hardcoded to None - integration is dead code |
| Bug 2 | waf | Cookie matching uses `unwrap_or(0)` - silently pushes incorrect cookie name |
| Bug 1 | tui | Duplicate key binding 'b' (Ctrl+b and normal 'b') |
| Bug 2 | tui | InputGroup field access in fuzz.rs reset() without bounds check |
| B1 | networking | RateLimiter can spin at 100% CPU when target_pps=1 |
| B4 | networking | IPv6 spoof range entropy reduced for host_bits <= 16 |
| Bug 3 | output | PDF generator silently truncates findings to 30 with no warning |
| B1 | ai_agents | Rate limit counter never resets - persists across agent lifetime |
| B2 | ai_agents | Knowledge base eviction removes all failed entries indiscriminately |
| B3 | cli_commands | Proxy handler missing scope validation |
| B4 | cli_commands | load_passwords reads files without path validation (traversal risk) |
| B6 | cli_commands | handle_no_command references non-existent cli.config field |
| BUG 4 | distributed | Heartbeat status always reports "idle" regardless of actual state |

## Cross-Module Issues

Patterns that appear in multiple modules:

1. **Missing scope/target validation** — Found in `cli_commands` (resume, stress, proxy), `distributed` (worker capabilities unvalidated)
2. **Silent error suppression / missing error handling** — Found in `recon` (dead code), `waf` (cookie lookup), `loadtest` (histogram recording), `distributed` (results dropped)
3. **FxHashMap migration incomplete** — Found in `recon` (FullReconResult callback uses HashMap), `ai_agents` (test code uses std HashMap)
4. **Rate limiting issues** — Found in `loadtest` (initial burst), `networking` (spin loop at low rates), `ai_agents` (never resets)
5. **Dead code / incomplete implementations** — Found in `recon` (ExploitDB, Alexa, zone transfer, threatstream), `waf` (HTTP/2 smuggling), `distributed` (task results never sent)
6. **Documentation drift** — Found in `recon` (pipeline modules missing from FULL_RECON_PIPELINE_MODULES), `waf` (WAF count mismatch), `networking` (line number offsets), `tui` (SharedHistory mutex type)

## Top 10 Highest-Impact Improvements

1. **[CRITICAL] Send task results back to coordinator** — Distributed system's result aggregation is fundamentally broken. Workers drop results instead of sending via `RemoteClient::report_result()`. — `distributed`
2. **[HIGH] Add scope validation to resume command** — Session files can contain targets outside configured scope. Modify `handle_resume` to accept `CommandContext` and validate targets. — `cli_commands`
3. **[HIGH] Implement worker stats and heartbeat reporting** — Coordinator receives static zero values. Update `process_task()` and heartbeat to report actual worker state. — `distributed`
4. **[HIGH] Fix rate limiting initial burst** — Initialize `next_allowed_at` to `TokioInstant::now()` instead of `now - min_interval` to prevent burst on startup. — `loadtest`
5. **[HIGH] Add path validation to load_passwords** — Use `validate_path_string()` before reading wordlist files to prevent path traversal. — `cli_commands`
6. **[HIGH] Implement MCP support** — Architecture documents MCP integration but no code exists. Would enable external AI platform integration. — `ai_agents`
7. **[MEDIUM] Cache compiled regexes in template matcher** — Every `search_pattern()` call with Regex mode rebuilds regex. Use `FxHashMap<String, Regex>` with lazy compilation. — `scanner`
8. **[MEDIUM] Replace RateLimiter spin loop with Semaphore** — Current atomic spin at `target_pps=1` causes 100% CPU. Use `tokio::sync::Semaphore`. — `networking`
9. **[MEDIUM] Add PDF truncation warning** — Silently truncates findings to 30. Add warning when `findings.len() > 30`. — `output`
10. **[MEDIUM] Standardize timeout defaults across CLI commands** — Timeout values vary from 2s to 30s with no documented standard. Create constants module. — `cli_commands`

## Recommended Priority Order

| Priority | Module | Issue | Effort |
|----------|--------|-------|--------|
| 1 | distributed | Send task results to coordinator (BUG 3) | Low |
| 2 | distributed | Update WorkerStats and heartbeat (BUG 1, 2, 4) | Medium |
| 3 | cli_commands | Add scope validation to resume (B1) | Low |
| 4 | cli_commands | Add path validation to load_passwords (B4) | Low |
| 5 | loadtest | Fix rate limiting initial burst | Low |
| 6 | networking | Replace RateLimiter spin loop with Semaphore | Low |
| 7 | networking | Fix IPv6 spoof entropy calculation | Low |
| 8 | ai_agents | Persist rate limit budget across restarts | Medium |
| 9 | ai_agents | Implement MCP support | High |
| 10 | scanner | Cache compiled regexes in template matcher | Low |
| 11 | output | Add PDF truncation warning | Low |
| 12 | waf | Fix cookie matching fallible index lookup | Low |
| 13 | waf | Add circuit breaker to WAF detection | Medium |
| 14 | cli_commands | Standardize timeout defaults | Low |
| 15 | recon | Wire up threatstream_key or remove dead code | Low |
| 16 | cli_commands | Add scope validation to proxy handler | Low |
| 17 | distributed | Implement worker registration tracking | Medium |
| 18 | distributed | Add task request/pull mechanism | High |
| 19 | distributed | Add graceful worker shutdown | Medium |
| 20 | tui | Fix InputGroup bounds checking in reset methods | Low |

## Per-Module Summary

| Module | Bugs | Discrepancies | Improvements | Key Issues |
|--------|------|---------------|--------------|------------|
| recon | 4 | 4 | 8 | Dead code (ExploitDB, Alexa, zone transfer, threatstream); pipeline doc drift |
| waf | 3 | 5 | 7 | Cookie matching bug; HTTP/2 smuggling dead code; no circuit breaker on detection |
| scanner | 2 | 1 | 4 | Duplicate Memcached probe; regex caching opportunity |
| tui | 2 | 4 | 7 | Duplicate 'b' key binding; InputGroup bounds check in reset |
| loadtest | 2 | 2 | 4 | Rate limiting initial burst; auth header re-encoding |
| networking | 3 | 6 | 8 | RateLimiter spin loop; IPv6 spoof entropy; UDP range OOM risk |
| output | 1 | 4 | 6 | PDF silent truncation; template registration unwraps |
| config | 0 | 2 | 1 | No bugs found; undocumented validate() and with_cidr() methods |
| ai_agents | 3 | 5 | 8 | Rate limit never resets; knowledge base eviction bug; MCP not implemented |
| cli_commands | 7 | 5 | 12 | Resume scope bypass; missing scope checks; path traversal risk |
| distributed | 4 | 9 | 7 | Task results never sent; worker stats static; heartbeat hardcoded |

## Module Health Assessment

| Module | Health | Notes |
|--------|--------|-------|
| config | Excellent | No bugs, minimal discrepancies, comprehensive validation |
| output | Good | Minor PDF truncation issue, otherwise solid implementation |
| scanner | Good | Minor cleanup needed, strong test coverage |
| tui | Good | Bounds checking issues systematically addressed |
| recon | Fair | Multiple dead code stubs, pipeline documentation drift |
| waf | Fair | Cookie matching logic issue, HTTP/2 dead code |
| loadtest | Fair | Rate limiting bug needs fix |
| networking | Fair | RateLimiter spin loop, IPv6 entropy issues |
| ai_agents | Fair | Rate limit persistence gap, MCP documentation mismatch |
| cli_commands | Needs Work | Multiple scope validation gaps, path traversal risk |
| distributed | Needs Work | Core functionality broken (task results, stats, heartbeat) |
