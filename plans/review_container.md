# Container Architecture Review
**Document:** architecture/container.md
**Reviewed:** 2026-05-31
**Accuracy:** Medium
**Lines Reviewed:** 31

## Verified Claims
- `ContainerScanReport` struct: Verified at `crates/slapper/src/container/mod.rs:14` with fields `target`, `scan_type`, `docker`, `kubernetes`, `escape_risks`, `cis_benchmarks`, `findings`
- `ContainerScanType` enum: Verified at `crates/slapper/src/container/mod.rs:25` with variants `Docker`, `Kubernetes`, `EscapeDetection`, `CisBenchmark`, `Full`
- `ContainerFinding` struct: Verified at `crates/slapper/src/container/mod.rs:34` with fields `category`, `severity`, `title`, `description`, `recommendation`
- `DockerScanResult` struct: Verified at `crates/slapper/src/container/docker.rs:6` with fields `image_name`, `base_image`, `layers`, `vulnerabilities`, `misconfigurations`, `exposed_ports`, `running_as_root`, `has_healthcheck`
- `KubernetesScanResult` struct: Verified at `crates/slapper/src/container/kubernetes.rs:7` with fields `cluster_info`, `rbac_issues`, `network_policy_issues`, `pod_security_issues`, `secret_exposure`
- `EscapeDetectionResult` struct: Verified at `crates/slapper/src/container/escape.rs:5` with fields `target`, `escape_risks`, `risk_level`
- `CisBenchmarkResult` struct: Verified at `crates/slapper/src/container/cis.rs:5` with fields `benchmark_version`, `total_checks`, `passed`, `failed`, `warnings`, `checks`
- All files present: `mod.rs`, `docker.rs`, `kubernetes.rs`, `escape.rs`, `cis.rs` - verified
- Docker image analysis (secrets, privileges, exposed ports): Verified in `docker.rs`
- Kubernetes manifest security checks (RBAC, privileges, network policy): Verified in `kubernetes.rs`
- Container escape detection (shared namespaces, capabilities, mounts): Verified in `escape.rs`
- CIS Docker/Kubernetes benchmark validation: Verified in `cis.rs`

## Discrepancies
- **Feature-gating claim**: Documented as "Feature-gated behind appropriate flags" in Implementation Status. Actual: No `#[cfg(feature = ...)]` attributes found anywhere in `container/mod.rs` or its submodules. The container module is always compiled regardless of features (`crates/slapper/src/container/mod.rs:1-66`)

## Bugs Found
- None

## Improvement Opportunities
- The document does not mention additional types: `ImageLayer` (`docker.rs:18`), `DockerVulnerability` (`docker.rs:25`), `DockerMisconfiguration` (`docker.rs:34`), `ClusterInfo` (`kubernetes.rs:16`), `K8sFinding` (`kubernetes.rs:23`), `EscapeRisk` (`escape.rs:12`), `EscapeRiskLevel` (`escape.rs:20`), `CisCheck` (`cis.rs:14`), `CisCheckStatus` (`cis.rs:23`). These are important types used within the sub-modules.
- Consider adding feature-gating if container scanning is meant to be optional (e.g., Docker CLI not always available)

## Stale Items
- The "Feature-gated behind appropriate flags" claim is stale and should be corrected to "Always compiled" or feature-gating should be added
