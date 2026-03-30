# Deferred Items

Items from `fullplan.md` that were skipped, deferred, or require future work.

---

## All Items Completed

| Item | Source | Status |
|------|--------|--------|
| Unified Plugin trait | plan.md #8.1 | DONE |
| Python class-based plugins | plan.md #8.2 | DONE |
| Plugin documentation | plan.md #8.3 | DONE |
| Plugin sandboxing | plan.md #8.4 | DONE |
| Output consolidation | plan2.md #14 | DONE |
| Split Commands enum | plan4.md #3.2 | DONE |
| Review unwrap() count | plan3.md #3 | DONE (full audit completed — see details below) |
| REST API timing attack | fullplan.md #1 | DONE |
| Spoofed TCP checksum | fullplan.md #2 | DONE |
| Spoofed fragment flags | fullplan.md #3 | DONE |
| Burst mode payload drop | fullplan.md #4 | DONE |
| expect() in hot paths | fullplan.md #5 | DONE |
| proxy/mod.rs error handling | fullplan.md #6 | DONE |
| XML port scan output | fullplan.md #7 | DONE |
| DEFAULT_MAX_REDIRECTS | fullplan.md #8 | DONE |
| BLOCKED_STATUS_CODES consolidation | fullplan.md #9 | DONE |
| Silent error swallowing in recon | fullplan.md #10 | DONE |
| WAF 3xx redirect logic | fullplan.md #12 | DONE |
| Logging audit | fullplan.md #13 | DONE |
| Plugin directory defaults | fullplan.md #14 | DONE |
| NSE timeout thread safety | fullplan.md #15 | DONE |
| Dead code cleanup | fullplan.md #16-17 | DONE |
| Magnus API compatibility | fullplan.md A1 | DONE |
| Python await fix | fullplan.md B1 | DONE |
| Ruby thread safety (A2) | fullplan.md A2 | DONE (unsafe impl Send/Sync on RubyBridge) |
| Magnus function macros (A3) | fullplan.md A3 | DONE (all macros have &Ruby first param) |
| TUI plugin field (B2) | fullplan.md B2 | DONE (app.plugin exists with cfg-gating) |
| TUI lifetime issue (B3) | fullplan.md B3 | DONE (findings use clone()) |
| Encoder stubs (#19) | fullplan.md #19 | DONE (encode delegates to MSF, compatible uses module info) |
| Arc\<Mutex\> review (#18) | fullplan.md #18 | DONE (audit found no issues — all usages correct) |

---

## Pre-Work: External Dependency Blockers

| Issue | Feature Flag | Details |
|-------|-------------|---------|
| PyO3 incompatible with Python 3.14 | `python-plugins` | PyO3 0.24.2 max is 3.13; needs `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1` env var or PyO3 upgrade. `python-plugins` compiles on Python 3.12 and below. |
| rb-sys stable API missing | `ruby-plugins` | Needs `stable-api-compiled-fallback` feature or rb-sys update. `ruby-plugins` compiles because slapper-ruby uses magnus (not rb-sys directly). |

---

## unwrap() Audit Summary (Completed)

Full audit of ~93 `.unwrap()` calls found 7 in production code and 10 `.expect()` calls.
Fixed in this session:

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `stress/metrics.rs` | 153 | `Mutex::lock().unwrap()` — poisoning panic | `match` with poisoned recovery + warning log |
| `tui/workers/runner.rs` | 196 | `Option::unwrap()` after move | Extract string before moving into Option |
| `tui/workers/runner.rs` | 879 | `Option::unwrap()` after move | Extract string before moving into Option |
| `tui/ui.rs` | 138 | `expect()` on command palette | `let/else` with early return |
| `tui/tabs/proxy.rs` | 262 | `expect()` on HealthChecker creation | `match` with error log + return |
| `tui/tabs/proxy.rs` | 340 | `expect()` on HealthChecker creation | `match` with error log + return |
| `proxy/mod.rs` | 171 | `parse().unwrap()` on constant string | `SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)` |
| `commands/handlers/cluster.rs` | 23 | `duration_since().expect()` on SystemTime | `.unwrap_or_default()` |
| `recon/js.rs` | 65 | `Selector::parse().unwrap()` | `.expect("valid CSS selector")` |
| `recon/js.rs` | 78 | `Selector::parse().unwrap()` | `.expect("valid CSS selector")` |
| `recon/js.rs` | 250 | `Regex::new().unwrap()` | `.expect("valid regex")` |
| `recon/email.rs` | 75 | `Regex::new().unwrap()` | `.expect("valid regex")` |

Remaining (safe): ~86 test-only unwraps, ~20 init-time regex `.expect()` calls (intentional fail-fast), and guarded `.expect()` calls in `packet/cli.rs:47` and `recon/geolocation.rs:149`.
