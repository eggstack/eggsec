# Container Module

## Purpose

Container security scanning for Docker images, Kubernetes configurations, container escape detection, and CIS benchmark validation.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `ContainerScanReport` | `container/mod.rs` | Aggregated container scan results |
| `ContainerScanType` | `container/mod.rs` | Enum: Docker, Kubernetes, EscapeDetection, CisBenchmark, Full |
| `ContainerFinding` | `container/mod.rs` | Container security finding with category and severity |
| `DockerScanResult` | `container/docker.rs` | Docker image analysis results |
| `ImageLayer` | `container/docker.rs:18` | Single Docker image layer (id, instruction, size) |
| `DockerMisconfiguration` | `container/docker.rs:24` | Dockerfile/docker-compose misconfiguration (check, severity, description, recommendation) |
| `KubernetesScanner` | `container/kubernetes.rs` | Kubernetes cluster scanner; `from_in_cluster_config()` reads service account token |
| `KubernetesScanResult` | `container/kubernetes.rs` | Kubernetes cluster scan results with per-category findings |
| `ClusterInfo` | `container/kubernetes.rs:16` | Cluster metadata (server version, node count, namespace count) |
| `K8sFinding` | `container/kubernetes.rs:23` | Kubernetes security finding (resource type/name, severity, description, recommendation) |
| `EscapeDetectionResult` | `container/escape.rs` | Container escape risk assessment |
| `EscapeRisk` | `container/escape.rs:12` | Individual escape risk (risk type, severity, description, recommendation) |
| `EscapeRiskLevel` | `container/escape.rs:20` | Enum: None, Low, Medium, High, Critical |
| `CisBenchmarkResult` | `container/cis.rs` | CIS benchmark compliance results |
| `CisCheck` | `container/cis.rs:15` | Single CIS benchmark check (id, description, severity, status, recommendation) |
| `CisCheckStatus` | `container/cis.rs:24` | Enum: Pass, Fail, Warn |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `ContainerScanReport`, `ContainerScanType`, `ContainerFinding` |
| `docker.rs` | Docker image analysis (secrets, privileges, exposed ports) |
| `kubernetes.rs` | Kubernetes manifest security checks (RBAC, privileges, network policy) |
| `escape.rs` | Container escape detection (shared namespaces, capabilities, mounts) |
| `cis.rs` | CIS Docker/Kubernetes benchmark validation |

## Implementation Status

Fully implemented. All sub-modules define result types and scanning logic.

The top-level `container` module is feature-gated behind `#[cfg(feature = "container")]` at `lib.rs:84-88`. The `recon/containers.rs` module reuses types from the `container` module and delegates Docker scanning to `DockerScanner`.
