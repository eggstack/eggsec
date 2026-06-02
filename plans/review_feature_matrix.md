# Feature Matrix Architecture Review

**Document:** architecture/feature_matrix.md
**Reviewed:** 2026-06-02
**Accuracy:** Medium
**Lines Reviewed:** 101

## Verified Claims

- `default` feature: Verified at `Cargo.toml:214` - `default = []`
- `tool-api` feature: Verified at `Cargo.toml:217` - `tool-api = []`
- `insecure-tls` feature: Verified at `Cargo.toml:221` - `insecure-tls = []`
- `rest-api` feature with deps: Verified at `Cargo.toml:224` - `rest-api = ["tool-api", "axum", "tower", ...]`
- `ws-api` feature with deps: Verified at `Cargo.toml:227` - `ws-api = ["axum/ws"]`
- `grpc-api` feature with deps: Verified at `Cargo.toml:230` - `grpc-api = ["tool-api", "tonic", ...]`
- `stress-testing` feature with deps: Verified at `Cargo.toml:233` - `stress-testing = ["pnet", "pnet_packet", ...]`
- `packet-inspection` feature with deps: Verified at `Cargo.toml:236` - `packet-inspection = ["pnet", "pnet_packet", ...]`
- `nse` feature with deps: Verified at `Cargo.toml:239` - `nse = ["tool-api", "dep:slapper-nse", ...]`
- `nse-ssh2` feature: Verified at `Cargo.toml:242` - `nse-ssh2 = ["nse", "slapper-nse/nse-ssh2"]`
- `nse-sandbox` feature: Verified at `Cargo.toml:245` - `nse-sandbox = ["nse", "slapper-nse/sandbox"]`
- `ai-integration` feature with deps: Verified at `Cargo.toml:266` - `ai-integration = ["tool-api", "eventsource-stream", ...]`
- `websocket` feature with deps: Verified at `Cargo.toml:269` - `websocket = ["tokio-tungstenite"]`
- `headless-browser` feature with deps: Verified at `Cargo.toml:272` - `headless-browser = ["headless_chrome"]`
- `database` feature with deps: Verified at `Cargo.toml:275` - `database = ["sqlx"]`
- `container` feature with deps: Verified at `Cargo.toml:278` - `container = ["kube", "k8s-openapi"]`
- `cloud` feature (no deps): Verified at `Cargo.toml:281` - `cloud = []`
- `sbom` feature with deps: Verified at `Cargo.toml:284` - `sbom = ["cyclonedx-bom", "spdx", ...]`
- `advanced-hunting` feature (no deps): Verified at `Cargo.toml:248` - `advanced-hunting = []`
- `compliance` feature (no deps): Verified at `Cargo.toml:251` - `compliance = []`
- `external-integrations` feature (no deps): Verified at `Cargo.toml:254` - `external-integrations = []`
- `finding-workflow` feature (no deps): Verified at `Cargo.toml:257` - `finding-workflow = []`
- `vuln-management` feature (no deps): Verified at `Cargo.toml:260` - `vuln-management = []`
- `git-secrets` feature (no deps): Verified at `Cargo.toml:287` - `git-secrets = []`
- `wireless` feature (no deps): Verified at `Cargo.toml:293` - `wireless = []`
- `pdf` feature with deps: Verified at `Cargo.toml:290` - `pdf = ["printpdf"]`
- `api-schema` feature (no deps): Verified at `Cargo.toml:296` - `api-schema = []`
- `full` feature deprecated with 16 sub-features: Verified at `Cargo.toml:263`
- `full` feature k8s-openapi issue: Verified - `Cargo.toml:278` has `container = ["kube", "k8s-openapi"]` and `k8s-openapi` at line 186-189 requires `features = ["v1_30"]` which is set
- Module gating pattern with dual-declaration: Verified in `lib.rs` (#[cfg(feature)] and #[cfg(not(feature))] with dead_code allowance)

## Discrepancies

- **Total feature count mismatch**: Document says "Total features: 28" at line 9. Counting actual features in Cargo.toml: default, tool-api, insecure-tls, rest-api, ws-api, grpc-api, stress-testing, packet-inspection, nse, nse-ssh2, nse-sandbox, ai-integration, websocket, headless-browser, database, container, cloud, sbom, advanced-hunting, compliance, external-integrations, finding-workflow, vuln-management, git-secrets, wireless, pdf, api-schema, full = 28 features. The count is correct, but this excludes `default` which is listed separately. If counting non-default features: 27 non-default + 1 default = 28.
- **Features with deps count mismatch**: Document says "Features with deps: 16" at line 10. Counting features with actual dependency arrays (rest-api, ws-api, grpc-api, stress-testing, packet-inspection, nse, nse-ssh2, nse-sandbox, ai-integration, websocket, headless-browser, database, container, sbom, pdf, full) = 16. Correct.
- **Marker-only features count mismatch**: Document says "Marker-only features: 12" at line 11. Counting features with `= []` (no deps): tool-api, insecure-tls, advanced-hunting, compliance, external-integrations, finding-workflow, vuln-management, cloud, git-secrets, wireless, api-schema = 11. Plus `default = []` which is also marker-only = 12. Correct.
- **`ws-api` not in `full`**: Document at line 22 says `ws-api` is "yes" for declared and "yes" for has deps, but "-" for in `full`. Verified correct - `full` at line 263 does NOT include `ws-api`.
- **`nse-ssh2` and `nse-sandbox` not in `full`**: Document shows these with "-" for in `full`. Verified correct.
- **`pdf` not in `full`**: Document shows `pdf` with "-" for in `full`. Verified correct - `full` at line 263 does NOT include `pdf`.

## Bugs Found

- **k8s-openapi feature fix applied**: Document at lines 60-63 states that `full` fails to compile due to `k8s-openapi` requiring a Kubernetes version feature that "must be provided by the final binary crate". However, checking `Cargo.toml:186-189`, the `k8s-openapi` dependency now has `features = ["v1_30"]` set directly. This means the issue described in the document may be outdated - the `full` feature should now compile without requiring the final binary to set the feature.

## Improvement Opportunities

- **Document k8s-openapi fix**: The k8s-openapi issue at lines 58-63 needs to be updated since the feature now includes `features = ["v1_30"]` directly in Cargo.toml. The document should be updated to reflect that this issue has been resolved.
- **Consider adding `ws-api` to `full`**: The `ws-api` feature provides WebSocket support and is functionally complete per the document at lines 76-79. Consider adding it to the `full` feature for completeness.

## Stale Items

- **k8s-openapi issue description**: Lines 58-63 describe a compile issue with `full` due to k8s-openapi. This issue appears to be resolved as `Cargo.toml:189` now includes `features = ["v1_30"]`. The document should be updated to reflect this fix and remove the stale warning.

## Code Interrogation Findings

- **Dependency conflicts**: The `full` feature at line 263 includes 16 sub-features. Some of these have overlapping dependencies (e.g., `axum` appears in both `rest-api` and `ws-api`). Cargo's dependency resolver handles this, but it's worth noting for build times.
- **Feature stability labels**: The document categorizes features as Stable, Beta, Testing-only, Deprecated. The `ws-api` is marked Beta, which aligns with the note at lines 76-79 that "may need additional wiring for full WebSocket pub/sub support".
- **optional dependency pattern**: Several dependencies are declared as `optional = true` without corresponding feature flags (libc at line 104, pnet at line 108, pnet_packet at line 112, socket2 at line 116, nix at line 121, surge-ping at line 126, axum at line 130, etc.). These are enabled only via feature flags, which is correct.