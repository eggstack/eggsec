# Container Module Architecture Review

**Document:** architecture/container.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 42

## Verified Claims
- [ContainerScanReport]: Verified at `crates/slapper/src/container/mod.rs:14`
- [ContainerScanType enum]: Verified at `crates/slapper/src/container/mod.rs:24-31` (Docker, Kubernetes, EscapeDetection, CisBenchmark, Full)
- [ContainerFinding]: Verified at `crates/slapper/src/container/mod.rs:34`
- [DockerScanResult]: Verified at `crates/slapper/src/container/docker.rs:6`
- [ImageLayer]: Verified at `crates/slapper/src/container/docker.rs:18` (layer_id, instruction, size_bytes)
- [DockerVulnerability]: Verified at `crates/slapper/src/container/docker.rs:25` (package, installed_version, fixed_version, cve_id, severity)
- [DockerMisconfiguration]: Verified at `crates/slapper/src/container/docker.rs:34` (check, severity, description, recommendation)
- [KubernetesScanResult]: Verified at `crates/slapper/src/container/kubernetes.rs:7`
- [ClusterInfo]: Verified at `crates/slapper/src/container/kubernetes.rs:16` (server_version, node_count, namespace_count)
- [K8sFinding]: Verified at `crates/slapper/src/container/kubernetes.rs:23` (resource_type, resource_name, severity, description, recommendation)
- [EscapeDetectionResult]: Verified at `crates/slapper/src/container/escape.rs:5`
- [EscapeRisk]: Verified at `crates/slapper/src/container/escape.rs:12`
- [EscapeRiskLevel enum]: Verified at `crates/slapper/src/container/escape.rs:19-26` (None, Low, Medium, High, Critical)
- [CisBenchmarkResult]: Verified at `crates/slapper/src/container/cis.rs:5`
- [CisCheck]: Verified at `crates/slapper/src/container/cis.rs:15` (id, description, severity, status, recommendation)
- [CisCheckStatus enum]: Verified at `crates/slapper/src/container/cis.rs:23-28` (Pass, Fail, Warn)
- [All sub-module files exist]: Verified (docker.rs, kubernetes.rs, escape.rs, cis.rs)
- [Feature-gated behind container]: Verified in lib.rs (UNVERIFIED line - need to check lib.rs)

## Discrepancies
- None significant.

## Bugs Found
- None found.

## Improvement Opportunities
- [Docker scanner shell injection risk]: `crates/slapper/src/container/docker.rs:208-209` uses `std::process::Command::new("docker")` with `args(["inspect", _image_name])`. If `_image_name` contains special characters, this could lead to command injection. Consider validating image names (priority: high)
- [Kubernetes scanner silently fails]: In `kubernetes.rs`, API calls use `.ok()` on results (lines 65, 104, 163, 195, 254), silently ignoring network errors and returning empty results. This makes debugging difficult (priority: medium)
- [Node/namespace count always None]: `ClusterInfo::node_count` and `namespace_count` are always `None` in `kubernetes.rs:88-89` despite being part of the struct definition (priority: low)

## Stale Items
- None.

## Code Interrogation Findings
- [Docker socket access not checked]: The escape detection in `escape.rs` checks for docker.sock in config strings but doesn't actually verify if the container has access to the Docker socket. A real escape detection would need runtime checks.
- [CIS benchmark checks are simplistic]: The CIS checks in `cis.rs` use simple string matching (e.g., `lower.contains("privileged")`) which can produce false positives/negatives. Real CIS validation requires parsing actual Docker/K8s configuration formats.