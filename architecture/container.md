# Container Module

Container security scanning for Docker images, Kubernetes configurations, container escape detection, and CIS benchmark validation. Feature-gated behind `container`.

See also: [overview.md](overview.md), [findings.md](findings.md), [output.md](output.md), [tui.md](tui.md).

## Role & Responsibilities

- Docker image analysis: inspect images via `docker inspect`, parse Dockerfiles for misconfigurations
- Kubernetes cluster scanning: live API calls for RBAC, network policies, pod security, secret exposure
- Container escape detection: analyze configs for privileged containers, host namespace sharing, dangerous capabilities
- CIS benchmark validation: Docker Benchmark 1.6.0 and Kubernetes Benchmark 1.8.0 compliance checks

## Location & Feature Gating

| Item | Location | Feature |
|------|----------|---------|
| Module declaration | `lib.rs:99-103` | `container` |
| Public module | `lib.rs:99` (`pub mod container`) | `container` |
| Stub module | `lib.rs:101` (`mod container`) | `not(container)` |
| Feature flag | `Cargo.toml` `container` | No extra dependencies (uses reqwest, serde_json already in scope) |

When `container` is disabled, the module compiles with stub types. No container operations are available.

## Files

| File | Lines | Purpose |
|------|-------|---------|
| `container/mod.rs` | 66 | `ContainerScanReport`, `ContainerScanType` enum (5 variants), `ContainerFinding` struct |
| `container/docker.rs` | 343 | `DockerScanner`, `DockerScanResult`, `ImageLayer`, `DockerMisconfiguration`, Dockerfile analysis |
| `container/kubernetes.rs` | 394 | `KubernetesScanner`, `KubernetesScanResult`, `ClusterInfo`, `K8sFinding`, live K8s API scanning |
| `container/escape.rs` | 245 | `EscapeDetector`, `EscapeDetectionResult`, `EscapeRisk`, `EscapeRiskLevel` enum (5 variants) |
| `container/cis.rs` | 366 | `CisBenchmarkChecker`, `CisBenchmarkResult`, `CisCheck`, `CisCheckStatus` enum (3 variants) |
| `crates/eggsec/src/container/AGENTS.override.md` | 24 | Module override: known issues (simplified CIS checks, docker socket not checked) |

## Architecture

### ContainerScanType Enum (`mod.rs:24-31`)

5 variants: `Docker`, `Kubernetes`, `EscapeDetection`, `CisBenchmark`, `Full`

### ContainerScanReport (`mod.rs:14-22`)

| Field | Type | Notes |
|-------|------|-------|
| `target` | `String` | Scan target identifier |
| `scan_type` | `ContainerScanType` | Which sub-scan was performed |
| `docker` | `Option<DockerScanResult>` | Present when scan includes Docker |
| `kubernetes` | `Option<KubernetesScanResult>` | Present when scan includes Kubernetes |
| `escape_risks` | `Option<EscapeDetectionResult>` | Present when escape detection ran |
| `cis_benchmarks` | `Option<CisBenchmarkResult>` | Present when CIS checks ran |
| `findings` | `Vec<ContainerFinding>` | Aggregated findings across sub-scans |

### ContainerFinding (`mod.rs:34-40`)

| Field | Type |
|-------|------|
| `category` | `String` |
| `severity` | `Severity` |
| `title` | `String` |
| `description` | `String` |
| `recommendation` | `String` |

---

## Docker Scanning (`docker.rs`)

### DockerScanner (`docker.rs:32`)

Unit struct — stateless. Created via `DockerScanner::new()` or `DockerScanner::default()`.

### DockerScanResult (`docker.rs:7-15`)

| Field | Type | Notes |
|-------|------|-------|
| `image_name` | `String` | |
| `base_image` | `Option<String>` | Extracted from `Config.Image` |
| `layers` | `Vec<ImageLayer>` | Currently always empty (inspect doesn't expose layers) |
| `misconfigurations` | `Vec<DockerMisconfiguration>` | From Dockerfile analysis + runtime checks |
| `exposed_ports` | `Vec<u16>` | From `Config.ExposedPorts` |
| `running_as_root` | `bool` | From `Config.User` |
| `has_healthcheck` | `bool` | From `Config.Healthcheck` |

### ImageLayer (`docker.rs:17-22`)

| Field | Type |
|-------|------|
| `layer_id` | `String` |
| `instruction` | `String` |
| `size_bytes` | `Option<u64>` |

### DockerMisconfiguration (`docker.rs:24-30`)

| Field | Type |
|-------|------|
| `check` | `String` |
| `severity` | `Severity` |
| `description` | `String` |
| `recommendation` | `String` |

### Docker Scan Flow

1. `scan_image(image_name)` (`docker.rs:45-85`):
   - Calls `inspect_image()` → runs `docker inspect <image>` via `std::process::Command`
   - Parses JSON output, extracts `Config.User`, `Config.Healthcheck`, `Config.Image`, `Config.ExposedPorts`
   - Validates image name characters (`is_valid_image_name()` at `docker.rs:198-208`)
   - Calls `check_misconfigurations()` for runtime-level checks
2. `check_misconfigurations()` (`docker.rs:263-299`):
   - Running as root → High
   - No healthcheck → Low
   - Management ports (22/23/3389) exposed → High
3. `scan_dockerfile(path)` (`docker.rs:87-93`):
   - Reads file via `tokio::fs::read_to_string()`
   - Delegates to `analyze_dockerfile(content)`

### Dockerfile Analysis Checks (`docker.rs:95-196`)

| Check | Severity | Condition | Line |
|-------|----------|-----------|------|
| No specific image tag | Medium | `FROM` with `latest` or no tag | `docker.rs:104-114` |
| Running as root | High | `USER root` or `USER 0` | `docker.rs:116-123` |
| Dangerous port exposed | High | `EXPOSE 22/23/3389` | `docker.rs:125-146` |
| Secret in ENV | Critical | `ENV` contains `PASSWORD/SECRET/API_KEY/TOKEN` | `docker.rs:148-163` |
| ADD instead of COPY | Low | `ADD` for local files (not URLs) | `docker.rs:165-173` |
| No USER instruction | Medium | No `USER` directive anywhere | `docker.rs:176-184` |
| No HEALTHCHECK | Low | No `HEALTHCHECK` directive | `docker.rs:186-193` |

---

## Kubernetes Scanning (`kubernetes.rs`)

### KubernetesScanner (`kubernetes.rs:31-35`)

| Field | Type | Notes |
|-------|------|-------|
| `client` | `reqwest::Client` | Created via `create_insecure_http_client()` |
| `api_server` | `String` | Base URL (trailing `/` stripped) |
| `token` | `Option<String>` | Bearer token for auth |

### Construction

- `new(api_server, token, timeout_secs)` (`kubernetes.rs:38-45`) — explicit API server URL
- `from_in_cluster_config(timeout_secs)` (`kubernetes.rs:47-63`) — reads `/var/run/secrets/kubernetes.io/serviceaccount/token`, uses `KUBERNETES_SERVICE_HOST` env var, falls back to `https://kubernetes.default.svc`

### KubernetesScanResult (`kubernetes.rs:7-13`)

| Field | Type | Count |
|-------|------|-------|
| `cluster_info` | `Option<ClusterInfo>` | |
| `rbac_issues` | `Vec<K8sFinding>` | 0+ |
| `network_policy_issues` | `Vec<K8sFinding>` | 0+ |
| `pod_security_issues` | `Vec<K8sFinding>` | 0+ |
| `secret_exposure` | `Vec<K8sFinding>` | 0+ |

### ClusterInfo (`kubernetes.rs:15-20`)

| Field | Type |
|-------|------|
| `server_version` | `Option<String>` |
| `node_count` | `Option<usize>` |
| `namespace_count` | `Option<usize>` |

### K8sFinding (`kubernetes.rs:22-29`)

| Field | Type |
|-------|------|
| `resource_type` | `String` |
| `resource_name` | `String` |
| `severity` | `Severity` |
| `description` | `String` |
| `recommendation` | `String` |

### K8s API Calls Made

| Method | API Endpoint | Purpose | Line |
|--------|-------------|---------|------|
| `get_cluster_info()` | `GET /version` | Server version | `kubernetes.rs:87-93` |
| `get_item_count("/api/v1/nodes")` | `GET /api/v1/nodes` | Node count | `kubernetes.rs:95` |
| `get_item_count("/api/v1/namespaces")` | `GET /api/v1/namespaces` | Namespace count | `kubernetes.rs:96` |
| `check_rbac()` | `GET /apis/rbac.authorization.k8s.io/v1/clusterroles` | RBAC issues | `kubernetes.rs:141-144` |
| `check_network_policies()` | `GET /apis/networking.k8s.io/v1/networkpolicies` | Network policy count | `kubernetes.rs:205-208` |
| `check_pod_security()` | `GET /api/v1/pods` | Privileged containers | `kubernetes.rs:245` |
| `check_secret_exposure()` | `GET /api/v1/secrets` | Opaque secrets | `kubernetes.rs:312` |

All requests use `bearer_auth(token)` when a token is present (`kubernetes.rs:89-91, 111-113, 146-148, 209-211, 247-249, 314-316`).

### Kubernetes Scan Flow

1. `scan()` (`kubernetes.rs:65-84`) orchestrates all checks:
   - `get_cluster_info()` → version + node/namespace counts (non-fatal on failure)
   - `check_rbac()` → looks for `cluster-admin` ClusterRole with wildcard resources → Critical
   - `check_network_policies()` → if 0 policies defined → High
   - `check_pod_security()` → iterates all pods, checks `securityContext.privileged` → Critical
   - `check_secret_exposure()` → iterates all secrets, flags `Opaque` type → Medium

---

## Escape Detection (`escape.rs`)

### EscapeDetector (`escape.rs:28`)

Unit struct — stateless. Created via `EscapeDetector::new()` or `EscapeDetector::default()`.

### EscapeDetectionResult (`escape.rs:5-9`)

| Field | Type |
|-------|------|
| `target` | `String` |
| `escape_risks` | `Vec<EscapeRisk>` |
| `risk_level` | `EscapeRiskLevel` |

### EscapeRisk (`escape.rs:11-17`)

| Field | Type |
|-------|------|
| `risk_type` | `String` |
| `severity` | `Severity` |
| `description` | `String` |
| `recommendation` | `String` |

### EscapeRiskLevel Enum (`escape.rs:19-26`)

5 variants: `None`, `Low`, `Medium`, `High`, `Critical`

### Escape Risk Catalog (`escape.rs:41-161`)

| Risk Type | Severity | Detection | Line |
|-----------|----------|-----------|------|
| Privileged Container | Critical | `"privileged": true` or `privileged: true` | `escape.rs:45-53` |
| HostPath Mount | High | `hostpath` or `host_path` | `escape.rs:55-63` |
| Host Network | High | `hostnetwork: true` / `host_network: true` | `escape.rs:65-75` |
| Host PID | High | `hostpid: true` / `host_pid: true` | `escape.rs:77-87` |
| Host IPC | Medium | `hostipc: true` / `host_ipc: true` | `escape.rs:89-99` |
| Dangerous capability (5) | High | `SYS_ADMIN`, `NET_ADMIN`, `SYS_PTRACE`, `DAC_READ_SEARCH`, `SYS_MODULE` | `escape.rs:101-117` |
| Container Runtime Socket | Critical | `docker.sock` or `containerd.sock` | `escape.rs:119-126` |

**Total: 7 risk types** (1 privileged + 1 hostpath + 1 hostnetwork + 1 hostpid + 1 hostipc + 5 capabilities counted individually + 1 runtime socket)

### Risk Level Calculation (`escape.rs:149-161`)

`calculate_risk_level()` scans all risks and picks the highest severity present:
- Any `Critical` → `EscapeRiskLevel::Critical`
- Any `High` → `EscapeRiskLevel::High`
- Any `Medium` → `EscapeRiskLevel::Medium`
- Non-empty → `EscapeRiskLevel::Low`
- Empty → `EscapeRiskLevel::None`

### Escape Analysis Flow

1. `analyze_docker_config(config)` (`escape.rs:41-143`) — accepts a string config (YAML/JSON)
2. Case-insensitive search for risk patterns
3. `analyze_k8s_pod_spec(pod_spec)` (`escape.rs:145-147`) — delegates to `analyze_docker_config()` (same pattern matching)

---

## CIS Benchmark Checking (`cis.rs`)

### CisBenchmarkChecker (`cis.rs:30`)

Unit struct — stateless. Created via `CisBenchmarkChecker::new()` or `CisBenchmarkChecker::default()`.

### CisBenchmarkResult (`cis.rs:5-12`)

| Field | Type | Notes |
|-------|------|-------|
| `benchmark_version` | `String` | `"CIS Docker Benchmark 1.6.0"` or `"CIS Kubernetes Benchmark 1.8.0"` |
| `total_checks` | `usize` | |
| `passed` | `usize` | |
| `failed` | `usize` | |
| `warnings` | `usize` | |
| `checks` | `Vec<CisCheck>` | |

### CisCheck (`cis.rs:14-21`)

| Field | Type |
|-------|------|
| `id` | `String` |
| `description` | `String` |
| `severity` | `Severity` |
| `status` | `CisCheckStatus` |
| `recommendation` | `String` |

### CisCheckStatus Enum (`cis.rs:23-28`)

3 variants: `Pass`, `Fail`, `Warn`

### Docker CIS Checks (`cis.rs:93-198`)

7 checks (IDs 1.1–1.7):

| ID | Description | Severity | Fail Condition | Line |
|----|-------------|----------|----------------|------|
| 1.1 | Do not run as root | High | No `user`/`user:` or is `root`/`0` | `cis.rs:97-112` |
| 1.2 | No privileged containers | Critical | `privileged` + `true` | `cis.rs:114-124` |
| 1.3 | No sensitive host mounts | High | `/etc`, `/proc`, `/sys` | `cis.rs:126-136` |
| 1.4 | No host network mode | High | `hostnetwork` or `host_network` | `cis.rs:138-148` |
| 1.5 | Limit memory | Medium | No `memory`/`mem_limit` → Warn | `cis.rs:150-160` |
| 1.6 | Set CPU shares | Low | No `cpu_shares`/`cpus` → Warn | `cis.rs:162-172` |
| 1.7 | No privileged host ports | Medium | `-p` with host port < 1024 | `cis.rs:174-196` |

### Kubernetes CIS Checks (`cis.rs:201-303`)

8 checks (IDs 5.1.1–5.1.8):

| ID | Description | Severity | Fail Condition | Line |
|----|-------------|----------|----------------|------|
| 5.1.1 | No privileged containers | Critical | `privileged: true` | `cis.rs:205-216` |
| 5.1.2 | No privilege escalation | High | `allowprivilegeescalation: true` | `cis.rs:218-228` |
| 5.1.3 | No root user | High | `runasuser: 0` / `run_as_user: 0` | `cis.rs:230-240` |
| 5.1.4 | No added capabilities | High | `capabilities` + `add` → Warn | `cis.rs:242-252` |
| 5.1.5 | No hostPath volumes | High | `hostpath` | `cis.rs:254-264` |
| 5.1.6 | No hostNetwork | High | `hostnetwork: true` / `host_network: true` | `cis.rs:266-276` |
| 5.1.7 | No hostPID | High | `hostpid: true` / `host_pid: true` | `cis.rs:278-288` |
| 5.1.8 | No hostIPC | Medium | `hostipc: true` / `host_ipc: true` | `cis.rs:290-300` |

**Summary**: 7 Docker checks + 8 Kubernetes checks = 15 total CIS checks.

### CIS Check Flow

1. `check_docker(docker_info)` (`cis.rs:43-66`):
   - Runs `docker_checks()` against lowercased input string
   - Counts pass/fail/warn from results
   - Returns `CisBenchmarkResult` with benchmark version `"CIS Docker Benchmark 1.6.0"`
2. `check_kubernetes(k8s_config)` (`cis.rs:68-91`):
   - Runs `kubernetes_checks()` against lowercased input string
   - Returns `CisBenchmarkResult` with benchmark version `"CIS Kubernetes Benchmark 1.8.0"`

## Data Model

### Enum Variant Counts

| Enum | Variants | Location |
|------|:--------:|----------|
| `ContainerScanType` | 5 | `mod.rs:24-31` |
| `EscapeRiskLevel` | 5 | `escape.rs:19-26` |
| `CisCheckStatus` | 3 | `cis.rs:23-28` |

## Public API

| Function/Method | Signature | Feature Gate |
|-----------------|-----------|:---:|
| `DockerScanner::new()` | `fn new() -> Self` | `container` |
| `DockerScanner::scan_image()` | `async fn scan_image(&self, image_name: &str) -> Result<DockerScanResult>` | `container` |
| `DockerScanner::scan_dockerfile()` | `async fn scan_dockerfile(&self, dockerfile_path: &str) -> Result<Vec<DockerMisconfiguration>>` | `container` |
| `KubernetesScanner::new()` | `fn new(api_server: &str, token: Option<String>, timeout_secs: u64) -> Result<Self>` | `container` |
| `KubernetesScanner::from_in_cluster_config()` | `fn from_in_cluster_config(timeout_secs: u64) -> Result<Self>` | `container` |
| `KubernetesScanner::scan()` | `async fn scan(&self) -> Result<KubernetesScanResult>` | `container` |
| `EscapeDetector::new()` | `fn new() -> Self` | `container` |
| `EscapeDetector::analyze_docker_config()` | `fn analyze_docker_config(&self, config: &str) -> EscapeDetectionResult` | `container` |
| `EscapeDetector::analyze_k8s_pod_spec()` | `fn analyze_k8s_pod_spec(&self, pod_spec: &str) -> EscapeDetectionResult` | `container` |
| `CisBenchmarkChecker::new()` | `fn new() -> Self` | `container` |
| `CisBenchmarkChecker::check_docker()` | `fn check_docker(&self, docker_info: &str) -> CisBenchmarkResult` | `container` |
| `CisBenchmarkChecker::check_kubernetes()` | `fn check_kubernetes(&self, k8s_config: &str) -> CisBenchmarkResult` | `container` |

## Integration Points

### Dispatch

- Container scanning is **not** wired into `TaskKind` dispatch (`dispatch/types.rs` has no `Container` variant)
- The `recon/containers.rs` module reuses types from `container` and delegates to `DockerScanner` (per `overview.md`)
- Standalone usage: callers construct scanners directly and call scan methods

### TUI

- No dedicated Container tab in the TUI (not in the 33-tab inventory at `tui.md:130-166`)
- Container types could be used by future tabs or via pipeline stages

### Findings Store

- `ContainerFinding` is a separate type from the canonical `Finding` (5 fields vs. 17 fields)
- No automatic conversion or persistence path — results are returned as `ContainerScanReport`

## Testing

### Docker Tests (`docker.rs:302-342`)

| Test | Lines | What It Verifies |
|------|-------|------------------|
| `test_docker_scanner_creation` | 307-310 | Scanner instantiation |
| `test_analyze_dockerfile_root_user` | 313-318 | `USER root` detected |
| `test_analyze_dockerfile_secret_in_env` | 321-326 | `ENV API_KEY=secret` → Critical |
| `test_analyze_dockerfile_no_user` | 329-334 | No USER instruction detected |
| `test_analyze_dockerfile_dangerous_port` | 337-342 | Port 22 exposed detected |

### Kubernetes Tests (`kubernetes.rs:363-393`)

| Test | Lines | What It Verifies |
|------|-------|------------------|
| `test_k8s_scanner_creation` | 368-371 | Scanner instantiation |
| `test_k8s_finding_creation` | 374-383 | Finding struct fields |
| `test_cluster_info_creation` | 386-393 | ClusterInfo struct fields |

### Escape Tests (`escape.rs:168-244`)

| Test | Lines | What It Verifies |
|------|-------|------------------|
| `test_escape_detector_creation` | 173-176 | Instantiation |
| `test_detect_privileged_container` | 179-188 | `"privileged": true` → Critical |
| `test_detect_hostpath_mount` | 191-199 | `hostPath` detected |
| `test_detect_docker_socket` | 202-210 | `docker.sock` detected |
| `test_detect_dangerous_capabilities` | 213-221 | `SYS_ADMIN` detected |
| `test_clean_config` | 224-230 | Clean config → no risks |
| `test_risk_level_calculation` | 233-244 | Risk level from severity |

### CIS Tests (`cis.rs:306-365`)

| Test | Lines | What It Verifies |
|------|-------|------------------|
| `test_cis_checker_creation` | 311-314 | Instantiation |
| `test_docker_checks_privileged_fail` | 317-324 | `privileged: true` → 1.2 Fail |
| `test_docker_checks_clean_pass` | 327-334 | Clean config → 1.2 Pass |
| `test_k8s_checks_privileged_fail` | 337-344 | `privileged: true` → 5.1.1 Fail |
| `test_benchmark_result_summary` | 347-352 | Counts populated correctly |
| `test_cis_check_creation` | 355-365 | Check struct fields |

## Invariants & Gotchas

1. **String-matching CIS checks**: All CIS and escape checks use case-insensitive string matching on the raw config input. This can produce false positives (e.g., a comment containing "privileged") and false negatives (e.g., nested YAML structure not matched by simple `contains()`). The `crates/eggsec/src/container/AGENTS.override.md` documents this as a known limitation.
2. **Kubernetes uses `reqwest::Client` directly**: Not the `kube` client crate. The scanner makes raw HTTP requests to the K8s API with bearer token auth. This means no automatic API version discovery, schema validation, or watch support.
3. **`from_in_cluster_config()` is not async**: It reads the service account token file synchronously (`std::fs::read_to_string`). This is fine for pod startup but blocks the async runtime briefly.
4. **`analyze_k8s_pod_spec()` delegates to `analyze_docker_config()`**: Both use the same string-matching logic (`escape.rs:145-147`). K8s pod specs have different structure than Docker configs, but the pattern matching is broad enough to catch common cases.
5. **Docker inspect via `std::process::Command`**: The `inspect_image()` method spawns a `docker` CLI process (`docker.rs:217-219`), not using the Docker API. This requires the `docker` binary in PATH and appropriate permissions.
6. **No container runtime socket validation**: The escape detector checks for `docker.sock` in config strings but does not verify actual socket access (`crates/eggsec/src/container/AGENTS.override.md:15`).
7. **No persistent state**: All scanners are stateless unit structs. Scan results are returned directly, not stored.

## Bug Sweep

| Finding | File:Line | Severity | Description |
|---------|-----------|----------|-------------|
| `docker inspect` via process spawn | `docker.rs:217-219` | Info | Uses `std::process::Command` instead of Docker API. Requires `docker` binary in PATH; no timeout on process execution. |
| No request timeout on K8s API | `kubernetes.rs:39` | Low | `create_insecure_http_client(timeout_secs)` sets a timeout, but individual request methods don't enforce timeouts beyond the client-level setting. |
| String-matching false positives | `cis.rs`, `escape.rs` | Medium | All checks use `lower.contains("pattern")` which matches substrings in comments, values, and unrelated contexts. No AST parsing. |
| `lower_contains` helper inconsistency | `escape.rs:164-166` | Low | Converts needle to lowercase but haystack is already lowered — works correctly but the function name suggests it handles both. |
| No process timeout | `docker.rs:217-219` | Medium | `Command::new("docker").args(["inspect", image_name]).output()` has no timeout. A hung docker daemon blocks the async task indefinitely. |

*Last verified against source: 2026-08-25*
