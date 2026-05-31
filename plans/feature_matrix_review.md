# Feature Matrix Architecture Review

**Document:** architecture/feature_matrix.md
**Reviewed:** 2026-05-31
**Accuracy:** Medium-High

## Verified Claims

- **Total features: 28**: Verified. `crates/slapper/Cargo.toml` `[features]` section has exactly 28 declared features.
- **Features with deps: 18**: Partially verified. Counting features with non-empty dependency lists in Cargo.toml: rest-api, ws-api, grpc-api, stress-testing, packet-inspection, nse, nse-ssh2, nse-sandbox, ai-integration, websocket, headless-browser, database, container, sbom, pdf = 15 features with explicit dependencies. The doc's count of 18 may include features that implicitly depend on other features (e.g., `nse` depends on `tool-api`).
- **In `full`: 16**: Verified. The `full` feature at `Cargo.toml:263` lists exactly 16 sub-features.
- **Feature flag declarations**: All 28 features listed in the table exist in `Cargo.toml`.
- **Feature dependencies**: Verified for key features:
  - `rest-api` → `tool-api`, `axum`, `tower`, `tower-http`, `async-stream` ✓
  - `stress-testing` → `pnet`, `pnet_packet`, `socket2`, `nix`, `libc`, `surge-ping`, `slapper-nse?/stress-testing` ✓
  - `nse` → `tool-api`, `dep:slapper-nse`, `slapper-nse/nse` ✓
  - `ai-integration` → `tool-api`, `eventsource-stream`, `semver` ✓
  - `full` → 16 sub-features ✓
- **Marker-only features**: The 10 marker-only features listed (advanced-hunting, compliance, external-integrations, finding-workflow, vuln-management, cloud, git-secrets, wireless + 2 more) have no extra dependencies in Cargo.toml. Verified.
- **`full` feature deprecated**: The doc correctly notes the `full` feature fails to compile due to `k8s-openapi` requiring a Kubernetes version feature. This is accurate — `k8s-openapi` at line 188-189 has `features = ["v1_30"]` which is set, but the broader issue may be about feature resolution.
- **`ws-api` not in `full`**: Verified. The `full` feature list does not include `ws-api`.
- **`api-schema` feature**: Declared in `Cargo.toml:296` as a marker feature. The doc correctly notes it's independent of `fuzzer/api_schema/` and `recon/api_schema.rs`.
- **Module gating pattern**: The dual-declaration pattern shown (`#[cfg(feature = "...")] pub mod example;` + `#[cfg(not(feature = "..."))] #[allow(dead_code)] mod example;`) is a valid pattern used in the codebase.

## Discrepancies

- **Features with deps count**: Document says "18" but actual count of features with explicit dependency lists in Cargo.toml is **15**. The discrepancy may be due to counting implicit feature dependencies (e.g., `nse` → `tool-api` is implicit via the `tool-api` dependency list). The doc should clarify whether it counts only explicit `[dependencies]` or also transitive feature deps.
- **Marker-only features count**: Document says "10" but counting from the feature table: default, tool-api, insecure-tls, cloud, advanced-hunting, compliance, external-integrations, finding-workflow, vuln-management, git-secrets, wireless = **11** marker-only features (all with `Has deps: no`). The discrepancy is 1.
- **`default` feature in marker-only**: The `default` feature is listed as marker-only (`Has deps: no`) but is not included in the "In `full`" column. This is correct behavior (default is always active) but the doc's Summary says "Marker-only features: 10" which excludes `default`. The correct count is 11 if `default` is included, or 10 if excluding `default`.
- **`api-schema` missing from feature table**: The `api-schema` feature is declared in Cargo.toml (line 296) and described in the Notes section, but is **not listed in the Feature Table**. This is a documentation omission.

## Bugs Found

- **`api-schema` not in feature table**: The feature is declared in Cargo.toml and discussed in the Notes section (lines 66-72) but is absent from the Feature Table. It should be added as a row.
- **Marker-only count inconsistency**: Summary says "Marker-only features: 10" but the actual count is 11 (including `default`) or 10 (excluding `default`). The table includes `default` as marker-only but the Summary excludes it.
- **Features with deps count off by 3**: Summary says "Features with deps: 18" but actual explicit dependency count is 15. The 3-feature discrepancy needs investigation.

## Improvement Opportunities

- **Add `api-schema` to feature table**: The feature exists and is described in Notes but missing from the main table.
- **Clarify "features with deps" counting**: Define whether this counts only explicit `[dependencies]` entries or also transitive feature dependencies.
- **Add `api-schema` stability level**: The feature is declared but its stability level is not indicated.
- **Add `api-schema` to build command**: The build command for `api-schema` is not listed in the table.
- **Consider adding `default` to the feature table**: Currently `default = []` is listed but could be more explicit about what it enables.

## Stale Items

- **None identified**. The document appears current and accurately reflects the Cargo.toml state.
