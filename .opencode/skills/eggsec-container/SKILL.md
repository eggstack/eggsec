---
name: eggsec-container
description: "Container and Kubernetes security scanning - use when working with Docker image checks, Kubernetes pod specs, container escape detection, or CIS benchmark validation."
---

# Eggsec Container Skill

Container security scanning module (Docker configs/images, Kubernetes specs, escape-risk detection, CIS checks).

## Module Location

`crates/eggsec/src/container/`

## Key Types

- `ContainerScanReport` (`mod.rs`) - Aggregated scan report
- `ContainerScanType` - Scan category enum (Docker, Kubernetes, ...)
- `ContainerFinding` - Per-check finding
- `DockerScanner` (`docker.rs`) - Docker config/image checks
- `KubernetesScanner` (`kubernetes.rs`) - K8s manifest/cluster checks (uses kube 4.x / k8s-openapi 0.28 behind the `container` feature)
- `EscapeDetector` (`escape.rs`) - `analyze_docker_config()` / `analyze_k8s_pod_spec()` returning `EscapeDetectionResult` with `EscapeRisk` entries (`EscapeRiskLevel`)
- CIS benchmark checks in `cis.rs`

## Feature Gate

`container = ["kube", "k8s-openapi"]` - no system deps beyond that.

## Integration Points

- Recon uses this module via `recon/containers.rs` (`ContainerScanResult` alias, feature-gated)
- Python bindings expose `scan_docker_image` / `scan_kubernetes` as stable operations

## Patterns

### Adding a New Container Check

1. Implement the check in the relevant scanner (`docker.rs`, `kubernetes.rs`, `cis.rs`, `escape.rs`)
2. Emit a `ContainerFinding` with severity and evidence
3. Wire into the report aggregation in `mod.rs`
4. Add tests

## Testing

```bash
cargo check -p eggsec --features container
cargo test --lib -p eggsec container::
```

## Resources

- `crates/eggsec/src/container/AGENTS.override.md` - Module guidance
- `architecture/container.md` - Architecture deep dive
