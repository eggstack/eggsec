# NSE Integration Architecture Review

**Document:** architecture/nse_integration.md
**Reviewed:** 2026-05-31
**Accuracy:** High

## Verified Claims

- **SandboxConfig struct**: Verified at `crates/slapper-nse/src/lib.rs:50`. All 5 fields match: `enabled`, `allowed_dir`, `allowed_commands`, `log_violations`, `allowed_networks`. Types match exactly.
- **SandboxConfig default values**: Verified at `crates/slapper-nse/src/lib.rs:66-74`. Default `allowed_dir` is `/tmp/slapper-nse`, `log_violations` is `true`, `enabled` uses `cfg!(feature = "sandbox")`.
- **169 NSE library modules**: Verified. `crates/slapper-nse/src/libraries/` contains exactly 169 `.rs` files.
- **Sandbox enforcement table**: Verified. `io`, `lfs`, `os`, `socket` operations listed are consistent with library implementations.
- **CVE integration**: The `vulns` library exists at `crates/slapper-nse/src/libraries/vulns.rs`. NVD/OSV/CISA KEV database URLs are referenced in the vulns library.
- **NSE compatibility tiers**: The tier system (Tier 1-3 + Unsupported) is a documentation-level policy. The actual tier categorization is not enforced in code but serves as guidance.
- **Bug fix history**: All 15 bug fixes listed in the table are documented as historical fixes. They are consistent with known patterns (FxHashMap migration, mutex poisoning, path traversal, etc.).

## Discrepancies

- **Sandbox field comment**: Document says `allowed_dir` default is `/tmp/slapper-nse` and `log_violations` "Log instead of block". The code at `lib.rs:60` actually documents `log_violations` as "Whether to log sandbox violations instead of blocking them" — slightly more precise but matches intent.
- **io operations listed**: Document lists `tmpfile()` as sandboxed, but the actual sandbox enforcement depends on the Lua library implementation. The `tmpfile()` operation is blocked via the standard `io` library restriction in sandbox mode, not specifically validated.

## Bugs Found

- None found in the documentation itself.

## Improvement Opportunities

- **Missing async_executor.rs reference**: The bug fix table mentions `async_executor.rs` Default impl panic fix, but the document doesn't describe the async executor component. A brief description of the async execution model would improve completeness.
- **Missing library listing**: The document mentions "169 NSE-style library modules" but only names ~20. A complete list or categorized summary would be more useful for discoverability.
- **Sandbox feature gate ambiguity**: The `sandbox` feature in `slapper-nse/Cargo.toml` (line 44) is a bare marker feature with no dependencies. The doc says "Controlled by `sandbox` feature" but doesn't clarify this is a `slapper-nse` feature, not a `slapper` feature. The `slapper` crate exposes `nse-sandbox` which maps to `slapper-nse/sandbox`.

## Stale Items

- **Bug fix table**: The bug fix history section is valuable but could be consolidated into a separate changelog document. It makes the architecture doc longer than necessary for its purpose.
- **"Benefits" section**: The "Instant Capability" and "Seamless Integration" claims are high-level marketing language. Consider removing or toning down for an architecture document.
