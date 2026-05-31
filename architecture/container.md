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
| `KubernetesScanResult` | `container/kubernetes.rs` | Kubernetes manifest security results |
| `EscapeDetectionResult` | `container/escape.rs` | Container escape risk assessment |
| `CisBenchmarkResult` | `container/cis.rs` | CIS benchmark compliance results |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `ContainerScanReport`, `ContainerScanType`, `ContainerFinding` |
| `docker.rs` | Docker image analysis (secrets, privileges, exposed ports) |
| `kubernetes.rs` | Kubernetes manifest security checks (RBAC, privileges, network policy) |
| `escape.rs` | Container escape detection (shared namespaces, capabilities, mounts) |
| `cis.rs` | CIS Docker/Kubernetes benchmark validation |

## Implementation Status

Fully implemented. All sub-modules define result types and scanning logic. Feature-gated behind appropriate flags.
