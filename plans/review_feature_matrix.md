# Feature Matrix Architecture Review

**Document:** architecture/feature_matrix.md
**Reviewed:** 2026-05-31
**Accuracy:** Medium
**Lines Reviewed:** 101

## Verified Claims

- **Total features: 28 (line 8)**: Confirmed — 28 feature entries in `crates/slapper/Cargo.toml:213-296`
- **In `full`: 16 (line 11)**: Confirmed — `full` feature at `Cargo.toml:263` lists 16 sub-features
- **Feature table entries (lines 16-45)**: All 28 features declared in Cargo.toml match the table
- **`default` = [] (line 18)**: Confirmed at `Cargo.toml:214`
- **`tool-api` = [] (line 19)**: Confirmed at `Cargo.toml:217`
- **`insecure-tls` = [] (line 20)**: Confirmed at `Cargo.toml:221`
- **`rest-api` deps: tool-api, axum, tower, tower-http, async-stream (line 21)**: Confirmed at `Cargo.toml:224`
- **`ws-api` deps: axum/ws (line 22)**: Confirmed at `Cargo.toml:227`
- **`grpc-api` deps (line 23)**: Confirmed at `Cargo.toml:230`
- **`stress-testing` deps (line 24)**: Confirmed at `Cargo.toml:233`
- **`packet-inspection` deps (line 25)**: Confirmed at `Cargo.toml:236`
- **`nse` deps (line 26)**: Confirmed at `Cargo.toml:239`
- **`nse-ssh2` deps (line 27)**: Confirmed at `Cargo.toml:242`
- **`nse-sandbox` deps (line 28)**: Confirmed at `Cargo.toml:245`
- **`ai-integration` deps (line 29)**: Confirmed at `Cargo.toml:266`
- **`websocket` deps (line 30)**: Confirmed at `Cargo.toml:269`
- **`headless-browser` deps (line 31)**: Confirmed at `Cargo.toml:272`
- **`database` deps (line 32)**: Confirmed at `Cargo.toml:275`
- **`container` deps (line 33)**: Confirmed at `Cargo.toml:278`
- **`cloud` = [] (line 34)**: Confirmed at `Cargo.toml:281`
- **`sbom` deps (line 35)**: Confirmed at `Cargo.toml:284`
- **`advanced-hunting` = [] (line 36)**: Confirmed at `Cargo.toml:248`
- **`compliance` = [] (line 37)**: Confirmed at `Cargo.toml:251`
- **`external-integrations` = [] (line 38)**: Confirmed at `Cargo.toml:254`
- **`finding-workflow` = [] (line 39)**: Confirmed at `Cargo.toml:257`
- **`vuln-management` = [] (line 40)**: Confirmed at `Cargo.toml:260`
- **`git-secrets` = [] (line 41)**: Confirmed at `Cargo.toml:287`
- **`wireless` = [] (line 42)**: Confirmed at `Cargo.toml:293`
- **`pdf` deps: printpdf (line 43)**: Confirmed at `Cargo.toml:290`
- **`api-schema` = [] (line 44)**: Confirmed at `Cargo.toml:296`
- **`full` deps (line 45)**: Confirmed at `Cargo.toml:263`
- **`full` stability: Deprecated (line 45)**: Document claims deprecated. Partially stale — see Stale Items below
- **Module gating pattern (lines 90-101)**: The dual-declaration pattern is confirmed in `lib.rs` — e.g., `compliance` at lines 77-81, `container` at lines 84-88, `hunt` at lines 94-98, `integrations` at lines 99-103, `storage` at lines 113-117, `supply_chain` at lines 120-124, `vuln` at lines 128-132, `workflow` at lines 136-140
- **Marker-only features list (lines 83-86)**: `advanced-hunting`, `compliance`, `external-integrations`, `finding-workflow`, `vuln-management`, `cloud`, `git-secrets`, `wireless` — all confirmed as empty feature arrays
- **Primary module mappings**: All verified against actual module locations in `lib.rs`

## Discrepancies

- **"Features with deps" count: 18 (line 9)**: Document says 18 features have dependencies. Actual count is **16**: `rest-api`, `ws-api`, `grpc-api`, `stress-testing`, `packet-inspection`, `nse`, `nse-ssh2`, `nse-sandbox`, `full`, `ai-integration`, `websocket`, `headless-browser`, `database`, `container`, `sbom`, `pdf`. The sum 18 + 12 = 30 does not equal the stated total of 28. (`crates/slapper/Cargo.toml:213-296`)
- **"Marker-only features" count: 12 (line 10)**: This count is correct. The 12 marker-only features are: `default`, `tool-api`, `insecure-tls`, `advanced-hunting`, `compliance`, `external-integrations`, `finding-workflow`, `vuln-management`, `cloud`, `git-secrets`, `wireless`, `api-schema`. However, 16 + 12 = 28, so the "Features with deps" count should be 16, not 18.
- **`ws-api` not in `full` (line 22)**: Doc correctly shows `ws-api` is NOT in `full`. Confirmed — `full` at `Cargo.toml:263` does not include `ws-api`. (`crates/slapper/Cargo.toml:263`)
- **`tool-api` primary module (line 19)**: Doc says primary module is `tool/`. However, `tool-api` is a marker-only feature that gates `pub mod tool` at `lib.rs:142-143` via `#[cfg(any(feature = "tool-api", feature = "rest-api", feature = "grpc-api"))]`. The module path is correct but the gating logic is more nuanced. (`crates/slapper/src/lib.rs:142-143`)
- **`insecure-tls` stability: Testing-only (line 20)**: Confirmed by the comment at `Cargo.toml:219-220`: "WARNING: This feature introduces security vulnerabilities. Never use in production."

## Bugs Found

- **Mathematical inconsistency in summary (lines 8-11)**: The Summary table states "Features with deps | 18" and "Marker-only features | 12". The sum (30) does not match "Total features | 28". The correct "Features with deps" count is 16. (`architecture/feature_matrix.md:8-11`)

## Improvement Opportunities

- **Correct "Features with deps" count (priority: high)**: Change from 18 to 16 to match actual Cargo.toml definitions and make the sum consistent with the total of 28. (`architecture/feature_matrix.md:9`)
- **Document `tool-api` gating nuance (priority: medium)**: The `tool` module is gated by `any(feature = "tool-api", feature = "rest-api", feature = "grpc-api")` at `lib.rs:142-143`, not just `tool-api`. The table entry at line 19 implies `tool-api` alone gates the module. (`crates/slapper/src/lib.rs:142-143`)
- **Document `packet` module dual-gating (priority: medium)**: The `packet` module is gated by `any(feature = "packet-inspection", feature = "stress-testing")` at `lib.rs:157-158`, meaning it's available with either feature. This cross-feature dependency isn't captured in the table. (`crates/slapper/src/lib.rs:157-158`)
- **Document `agent` module gating (priority: low)**: The `agent` module is gated by `#[cfg(feature = "rest-api")]` at `lib.rs:148-149`. This is not listed in the feature table's primary module column for `rest-api`. (`crates/slapper/src/lib.rs:148-149`)
- **Document `ai` module gating (priority: low)**: The `ai` module is gated by `#[cfg(feature = "ai-integration")]` at `lib.rs:145-146`. Correct in the table. (`crates/slapper/src/lib.rs:145-146`)

## Stale Items

- **`full` feature "currently fails to compile" claim (lines 60-63)**: The doc states `full` "currently fails to compile due to a pre-existing `k8s-openapi` issue: the `container` feature pulls in `k8s-openapi` which requires a Kubernetes version feature (e.g., `v1_30`) to be enabled. This is not set in `Cargo.toml`." However, `Cargo.toml:188-189` now includes `features = ["v1_30"]` in the `[dependencies.k8s-openapi]` section, which should resolve the compilation issue. The "fails to compile" claim may be stale. Recommendation: Verify with `cargo check -p slapper --features full` and update accordingly. (`crates/slapper/Cargo.toml:186-189`)
- **`full` stability: Deprecated (line 45)**: The doc marks `full` as Deprecated. The Cargo.toml comment at line 262 says "# Full build with all features" without explicit deprecation. The deprecation was likely added in the architecture doc due to the compilation issue. If the k8s-openapi fix resolves the compilation, the deprecation status should be reconsidered. (`crates/slapper/Cargo.toml:262-263`)
- **`ws-api` "may need additional wiring" (lines 76-79)**: The doc says `ws-api` "is functional but may need additional wiring for full WebSocket pub/sub support." This is a vague claim that should be verified or removed if the feature is fully implemented. (`architecture/feature_matrix.md:76-79`)
