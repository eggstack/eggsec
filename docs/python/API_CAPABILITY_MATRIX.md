# API Capability Matrix

Machine-readable capability matrix for the `eggsec-python` crate. This is the
**authoritative source of truth** for native capabilities, Python exports,
operation maturity, and validation status.

> **Release boundary**: pre-1.0 stable-core release candidate. Only the
> twenty-two-operation stable-core set carries a compatibility promise. All other
> domains are provisional or experimental until they satisfy the graduation
> checklist below.

---

## 1. Stability Classification Legend

| Classification | Guarantee | Deprecation Policy |
|----------------|-----------|-------------------|
| **stable** | Will not change without a major version bump. Full API and behavioral contract. | Deprecation window before removal. |
| **stable (limited)** | Correct behavior guaranteed; output format may expand with new optional fields. | Deprecation window before removal. |
| **provisional** | API shape accepted; implementation works but lacks full backend validation or end-to-end tests. | May change or be removed without a deprecation window. |
| **experimental** | Platform-sensitive, hazardous, incomplete, or subject to substantial change. | May change or be removed without notice. |
| **deprecated** | Will be removed. Replacement documented. | Scheduled for removal. |

A compiled feature being available does **not** imply a compatibility guarantee.
Use `domain_maturity()` for the release state of whole capability areas.

---

## 2. Graduation Checklist

A domain must satisfy **all** of the following to move from provisional or
experimental to stable:

1. **Canonical operation ID** and request/result DTO registered in the
   `OperationRegistry`.
2. **Sync and async dispatch** through the common policy gate
   (`EnforcementContext::evaluate()`).
3. **Structured errors**, typed events, cancellation, and serialization tests.
4. **Deterministic fixtures** and local/daemon contract coverage where relevant.
5. **Documentation**, type stubs, and wheel-profile coverage.

---

## 3. Feature Gate Mapping

| Python Feature | Engine Feature | System Dep | Default Wheel | Notes |
|----------------|----------------|------------|---------------|-------|
| *(always)* | — | — | yes | Core scanning, recon, WAF, findings |
| `websocket` | `websocket` | none | no | WebSocket security testing |
| `git-secrets` | `git-secrets` | none | no | Git secret detection |
| `sbom` | `sbom` | none | no | SBOM generation |
| `db-pentest` | `db-pentest` | none (drivers) | no | Database pentest (requires `eggsec-db-lab`) |
| `db-pentest-mongodb` | `db-pentest-mongodb` | none | no | MongoDB pentest |
| `db-pentest-redis` | `db-pentest-redis` | none | no | Redis pentest |
| `web-proxy` | `web-proxy` | none | no | Web proxy MITM (requires `eggsec-web-proxy`) |
| `mobile` | `mobile` | none | no | APK/IPA static analysis |
| `mobile-dynamic` | `mobile-dynamic` | ADB + device | no | Android dynamic testing |
| `packet-inspection` | `packet-inspection` | `libpcap-dev` | no | Packet capture |
| `stress-testing` | `stress-testing` | none | no | Stress testing (raw sockets) |
| `nse` | `nse` | `libssl-dev` | no | Nmap NSE scripts (requires `eggsec-nse`) |
| `container` | `container` | none | no | K8s/Docker scanning |
| `daemon-client` | — | none | no | Daemon session access |
| `headless-browser` | `headless-browser` | Chrome | no | DOM XSS, SPA routes, client checks |
| `advanced-hunting` | `advanced-hunting` | none | no | Attack chains, business logic, race conditions |
| `compliance` | `compliance` | none | no | Compliance mapping (OWASP, HIPAA, PCI, SOC2) |
| `wireless` | `wireless` | hardware + root | no | WiFi recon |
| `evasion` | `evasion` | none | no | Evasion detection |
| `postex` | `postex` | none | no | Post-exploitation simulation |
| `c2` | `c2` | none | no | C2 simulation (depends on postex+evasion) |
| `ai-integration` | `ai-integration` | external provider | no | AI-assisted analysis |
| `full-no-system` | — | none | no | Aggregate: `websocket`, `git-secrets`, `sbom`, `container` |

---

## 4. Capability Matrix

### 4.1 Stable Core Operations (10)

These operations are the stable release boundary. They share the mandatory
policy/audit gate, typed payloads, structured errors, audit decisions, and
sync/async dispatch.

| Operation ID | Display Name | Owning Rust Module | Cargo Feature | Default Wheel | Python Export | Canonical ID | Request DTO | Payload DTO | Risk | Maturity | Known Blockers |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `scan-ports` | Port Scanning | `scanner` | — | yes | `scan_ports` / `async_scan_ports` | `scan-ports` | `PortScanRequest` | `PortScanResult` | `safe_active` | stable | — |
| `scan-endpoints` | Endpoint Discovery | `scanner` | — | yes | `scan_endpoints` / `async_scan_endpoints` | `scan-endpoints` | `EndpointScanRequest` | `EndpointScanResult` | `safe_active` | stable | — |
| `fingerprint-services` | Service Fingerprinting | `scanner` | — | yes | `fingerprint_services` / `async_fingerprint_services` | `fingerprint-services` | `FingerprintRequest` | `FingerprintScanResult` | `passive` | stable | — |
| `recon` | DNS Reconnaissance | `recon` | — | yes | `recon_dns` / `async_recon_dns` | `recon` | `ReconDnsRequest` | `DnsRecordSet` | `passive` | stable | — |
| `tls-inspect` | TLS Certificate Inspection | `recon` | — | yes | `inspect_tls` / `async_inspect_tls` | `tls-inspect` | `TlsInspectRequest` | `TlsInspectionResult` | `passive` | stable | — |
| `tech-detect` | Technology Detection | `recon` | — | yes | `detect_technology` / `async_detect_technology` | `tech-detect` | `TechDetectRequest` | `TechDetectionResult` | `passive` | stable | — |
| `waf-detect` | WAF Detection | `waf` | — | yes | `detect_waf` / `async_detect_waf` | `waf-detect` | `WafDetectRequest` | `WafDetectionResult` | `passive` | stable | — |
| `waf-validate` | WAF Bypass Validation | `waf_validation` | — | yes | `validate_waf` / `async_validate_waf` | `waf-validate` | `WafValidateRequest` | `WafScanResult` | `intrusive` | stable | Requires explicit scope |
| `http-fuzz` | HTTP Fuzzing | `waf_validation` | — | yes | `fuzz_http` / `async_fuzz_http` | `http-fuzz` | `FuzzRequest` | `FuzzResult` | `intrusive` | stable | Requires explicit scope |
| `load-test` | HTTP Load Testing | `loadtest` | — | yes | `load_test_http` / `async_load_test_http` | `load-test` | `LoadTestRequest` | `LoadTestResult` | `load_test` | stable | Requires explicit scope; risk-gated by policy |

### 4.2 Mandatory Promotion Candidates (12)

These operations have canonical IDs, registered metadata, and sync/async
dispatch. They are provisionally stable and targeted for promotion to the
stable-core set.

| Operation ID | Display Name | Owning Rust Module | Cargo Feature | Default Wheel | Python Export | Canonical ID | Request DTO | Payload DTO | Risk | Maturity | Known Blockers |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `git-secrets` | Git Secret Scanning | `git_secrets` | `git-secrets` | no | `scan_git_secrets` / `async_scan_git_secrets` | `git-secrets` | `GitSecretRequest` | `GitSecretsReport` | `passive` | provisional | Deterministic fixture coverage pending |
| `sbom` | SBOM Generation | `sbom` | `sbom` | no | `generate_sbom` / `async_generate_sbom` | `sbom` | `SbomRequest` | `SbomReport` | `passive` | provisional | Deterministic fixture coverage pending |
| `consolidated-recon` | Consolidated Reconnaissance | `consolidated_recon` | — | yes | `run_consolidated_recon` / `async_run_consolidated_recon` | `consolidated-recon` | `ConsolidatedReconConfig` | `ConsolidatedReconReport` | `passive` | provisional | Common-engine event and daemon parity pending |
| `graphql` | GraphQL Security Assessment | `graphql` | — | yes | `graphql_test` / `async_graphql_test` | `graphql` | `GraphQLTestConfig` | `GraphQLTestResult` | `intrusive` | provisional | Common-engine event and daemon parity pending |
| `oauth` | OAuth/OIDC Assessment | `oauth` | — | yes | `oauth_test` / `async_oauth_test` | `oauth` | `OAuthTestConfig` | `OAuthTestResult` | `intrusive` | provisional | Common-engine event and daemon parity pending |
| `auth-assess` | Authentication Assessment | `auth_assess` | — | yes | `auth_test` / `async_auth_test` | `auth-assess` | `AuthTestConfig` | `AuthTestReport` | `credential_testing` | provisional | Deterministic integration fixtures pending |
| `db-pentest` | Database Penetration Testing | `db_pentest` | `db-pentest` | no | `db_probe` / `async_db_probe` | `db-pentest` | `DbPentestConfig` | `DbPentestReport` | `db_pentest` | provisional | Typed errors/events and fixture coverage pending |
| `nse-run` | NSE Script Execution | `nse` | `nse` | no | `nse_run` / `async_nse_run` | `nse-run` | `NseConfig` | `NseReport` | `safe_active` | provisional | Stable operation mapping and fixture coverage pending |
| `scan-docker` | Docker Image Scanning | `container` | `container` | no | `scan_docker_image` / `async_scan_docker_image` | `scan-docker` | `DockerScanRequest` | `DockerScanResult` | `passive` | provisional | Deterministic fixture coverage pending |
| `scan-k8s` | Kubernetes Cluster Scanning | `container` | `container` | no | `scan_kubernetes` / `async_scan_kubernetes` | `scan-k8s` | `KubernetesScanRequest` | `KubernetesScanResult` | `passive` | provisional | Deterministic fixture coverage pending |
| `mobile-static` | APK/IPA Static Analysis | `mobile` | `mobile` | no | `analyze_apk` / `async_analyze_apk`, `analyze_ipa` / `async_analyze_ipa` | `mobile-static` | `MobileStaticRequest` | `MobileScanReport` | `passive` | provisional | Dynamic device behavior is platform dependent |
| `web-proxy` | Web Proxy / MITM | `proxy` | `web-proxy` | no | `create_proxy_manager`, `async_add_proxy`, `async_proxy_health_check` | `web-proxy` | `ProxyConfig` | `InterceptSessionResult` | `traffic_interception` | provisional | Traffic interception semantics remain hazardous |

### 4.3 Conditional Candidates (2)

These operations require optional feature flags and are subject to additional
platform constraints.

| Operation ID | Display Name | Owning Rust Module | Cargo Feature | Default Wheel | Python Export | Canonical ID | Request DTO | Payload DTO | Risk | Maturity | Known Blockers |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `browser-test` | Headless Browser Assessment | `browser_assess` | `headless-browser` | no | `browser_test` / `async_browser_test` | `browser-test` | `BrowserTestConfig` | `BrowserTestReport` | `safe_active` | experimental | Platform and browser runtime dependent |
| `hunt-test` | Advanced Vulnerability Hunting | `hunt` | `advanced-hunting` | no | `hunt_test` / `async_hunt_test` | `hunt-test` | `HuntTestConfig` | `HuntReport` | `intrusive` | experimental | Full backend validation pending |

### 4.4 Additional Existing Domains (Provisional/Experimental)

These domains are present in the Python bindings but carry provisional or
experimental status. They may require system dependencies, privileges, external
providers, or hazardous lab authorization.

| Operation ID | Display Name | Owning Rust Module | Cargo Feature | Default Wheel | Python Export | Canonical ID | Request DTO | Payload DTO | Risk | Maturity | Known Blockers |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `websocket-probe` | WebSocket Security Testing | `websocket` | `websocket` | no | `websocket_probe` / `async_websocket_probe` | `websocket-probe` | `WebSocketTestConfig` | `WebSocketReport` | `safe_active` | provisional | Backend validation pending |
| `websocket-fuzz` | WebSocket Fuzzing | `websocket` | `websocket` | no | `websocket_fuzz` / `async_websocket_fuzz` | `websocket-fuzz` | `WebSocketTestConfig` | `FuzzTestResult` | `intrusive` | provisional | Backend validation pending |
| `stress-test` | Stress Testing / DoS Simulation | `stress` | `stress-testing` | no | `stress_test` / `async_stress_test` | `stress-test` | `StressConfig` | `StressResult` | `stress_test` | experimental | Raw sockets, IP spoofing; hazardous simulation |
| `packet-capture` | Packet Capture and Analysis | `packet_inspection` | `packet-inspection` | no | `parse_pcap`, `list_network_interfaces` | `packet-capture` | `CaptureConfig` | `PacketInfo` | `raw_packet` | experimental | Platform/system dependency and lifecycle coverage pending |
| `traceroute` | Network Traceroute | `packet_inspection` | `packet-inspection` | no | `run_traceroute` / `async_run_traceroute`, `traceroute` | `traceroute` | `TracerouteConfig` | `TracerouteResult` | `raw_packet` | experimental | Platform/system dependency pending |
| `wireless-scan` | Wireless Network Scanning | `wireless` | `wireless` | no | `wireless_scan` / `async_wireless_scan` | `wireless-scan` | `WirelessScanConfig` | `WirelessScanResult` | `raw_packet` | experimental | Hardware and privilege dependent |
| `evasion-scan` | Evasion Technique Detection | `evasion` | `evasion` | no | `evasion_scan` / `async_evasion_scan` | `evasion-scan` | `EvasionScanConfig` | `EvasionReport` | `evasion_testing` | experimental | Defense-validation domain, not stable-core |
| `postex-scan` | Post-Exploitation Simulation | `postex` | `postex` | no | `postex_scan` / `async_postex_scan` | `postex-scan` | `PostexScanConfig` | `PostexReport` | `post_exploitation` | experimental | Hazardous simulation domain |
| `c2-scan` | C2 Simulation | `c2` | `c2` | no | `c2_scan` / `async_c2_scan` | `c2-scan` | `C2ScanConfig` | `C2Report` | `c2_operation` | experimental | Hazardous simulation domain; depends on postex+evasion |
| `ai-analyze` | AI-Assisted Finding Analysis | `ai_postprocess` | `ai-integration` | no | `ai_analyze_finding` / `async_ai_analyze_finding`, `ai_generate_payloads`, `ai_suggest_waf_bypass` | `ai-analyze` | `AiAnalysisRequest` | `AiAnalysisResult` | `passive` | experimental | Provider-dependent advisory behavior |
| `mobile-dynamic` | Android Dynamic Analysis | `mobile` | `mobile-dynamic` | no | `list_mobile_devices`, `dynamic_mobile_analysis` | `mobile-dynamic` | `DynamicMobileRequest` | `DynamicMobileReport` | `remote_execution` | experimental | Platform and device dependent |
| `db-probe-mongodb` | MongoDB Probe | `db_pentest` | `db-pentest-mongodb` | no | `db_probe_mongodb` | `db-pentest` | `DbPentestConfig` | `DbPentestReport` | `db_pentest` | provisional | Typed errors/events pending |
| `db-probe-redis` | Redis Probe | `db_pentest` | `db-pentest-redis` | no | `db_probe_redis` | `db-pentest` | `DbPentestConfig` | `DbPentestReport` | `db_pentest` | provisional | Typed errors/events pending |

### 4.5 Infrastructure / Cross-Cutting Operations

These are not standalone scan operations but provide essential infrastructure
that all domains depend on.

| Operation ID | Display Name | Owning Rust Module | Cargo Feature | Default Wheel | Python Export | Canonical ID | Request DTO | Payload DTO | Risk | Maturity | Known Blockers |
|---|---|---|---|---|---|---|---|---|---|---|---|
| *(n/a)* | Daemon Session Management | `daemon` | `daemon-client` | no | `daemon_connect`, `async_daemon_*` | *(n/a)* | *(various)* | `DaemonResponse` | *(varies)* | provisional | Transport capability negotiation and reconnect contract pending |
| *(n/a)* | Distributed Coordination | `distributed` | — | yes | `distributed_task_types`, `distributed_generate_psk` | *(n/a)* | *(various)* | `DistributedTaskResult` | *(varies)* | experimental | Remote lifecycle contract pending |
| *(n/a)* | Notification Delivery | `notification` | — | yes | `notify_scan_started`, `notify_scan_complete`, `notify_findings`, `notify_error` | *(n/a)* | `WebhookConfig` | `WebhookEvent` | `passive` | provisional | — |
| *(n/a)* | Compliance Mapping | `compliance` | `compliance` | no | *(class-only, no functions)* | *(n/a)* | `ComplianceFramework` | `ComplianceReport` | `passive` | stable (feature-gated) | Feature-gated; no standalone operation |

---

## 5. Domain Maturity Summary

| Domain | Maturity | Required Gates |
|--------|----------|----------------|
| `stable-core` | **stable** | Canonical registry, policy gate, typed results, sync/async tests |
| `consolidated-recon` | provisional | Common-engine event and daemon parity pending |
| `graphql` | provisional | Common-engine event and daemon parity pending |
| `oauth` | provisional | Common-engine event and daemon parity pending |
| `authentication` | provisional | Deterministic integration fixtures pending |
| `daemon` | provisional | Transport capability negotiation and reconnect contract pending |
| `nse` | provisional | Stable operation mapping and fixture coverage pending |
| `database` | provisional | Typed errors/events and fixture coverage pending |
| `websocket` | provisional | Backend validation pending |
| `browser` | experimental | Platform and browser runtime dependent |
| `proxy` | experimental | Traffic interception semantics remain hazardous |
| `packet-inspection` | experimental | Platform/system dependency and lifecycle coverage pending |
| `mobile` | experimental | Dynamic device behavior is platform dependent |
| `wireless` | experimental | Hardware and privilege dependent |
| `evasion` | experimental | Defense-validation domain, not stable-core |
| `postex` | experimental | Hazardous simulation domain |
| `c2` | experimental | Hazardous simulation domain |
| `distributed` | experimental | Remote lifecycle contract pending |
| `ai` | experimental | Provider-dependent advisory behavior |

---

## 6. Runtime Introspection

Use these functions to query the matrix at runtime:

```python
import eggsec

# List all registered operations with metadata
for op in eggsec.OperationRegistry.all_operations():
    print(f"{op.operation_id}: risk={op.default_risk}, feature={op.feature_required}")

# Check domain maturity
maturity = eggsec.domain_maturity()
for domain, info in maturity.items():
    print(f"{domain}: {info['state']} — {info['required_gates']}")

# Check feature availability
matrix = eggsec.feature_matrix()
for feature, available in matrix.items():
    print(f"{feature}: {'available' if available else 'not compiled'}")

# Per-symbol stability
surface = eggsec.api_surface()
for name, info in surface.items():
    if info['stability'] != 'stable':
        print(f"{name}: {info['stability']}")
```

---

## 7. Operation Risk Tiers

| Risk Level | Level | Operations |
|------------|-------|------------|
| `passive` | 0 | `fingerprint-services`, `recon`, `tls-inspect`, `tech-detect`, `waf-detect`, `git-secrets`, `sbom`, `consolidated-recon`, `scan-docker`, `scan-k8s`, `mobile-static`, `ai-analyze` |
| `safe_active` | 1 | `scan-ports`, `scan-endpoints`, `nse-run`, `browser-test`, `websocket-probe` |
| `intrusive` | 2 | `waf-validate`, `http-fuzz`, `graphql`, `oauth`, `hunt-test`, `websocket-fuzz` |
| `load_test` | 3 | `load-test` |
| `stress_test` | 4 | `stress-test` |
| `raw_packet` | 5 | `packet-capture`, `traceroute`, `wireless-scan` |
| `credential_testing` | 6 | `auth-assess` |
| `db_pentest` | 7 | `db-pentest`, `db-probe-mongodb`, `db-probe-redis` |
| `traffic_interception` | 8 | `web-proxy` |
| `evasion_testing` | 10 | `evasion-scan` |
| `post_exploitation` | 11 | `postex-scan` |
| `c2_operation` | 12 | `c2-scan` |
| `remote_execution` | 13 | `mobile-dynamic` |

---

## 8. Sync/Async Parity

All operations with an `async_*` variant are registered in the
`ASYNC_OPERATION_IDS` constant in `operation_metadata.rs`. The full list:

`scan-ports`, `scan-endpoints`, `fingerprint-services`, `recon`,
`tls-inspect`, `tech-detect`, `waf-detect`, `waf-validate`, `http-fuzz`,
`load-test`, `git-secrets`, `sbom`, `mobile-static`, `db-pentest`,
`scan-docker`, `scan-k8s`, `packet-capture`, `traceroute`, `stress-test`,
`nse-run`, `wireless-scan`, `evasion-scan`, `postex-scan`, `c2-scan`,
`browser-test`, `hunt-test`, `ai-analyze`.

---

*Generated from source of truth in `operation_metadata.rs`, `domains.rs`,
`lib.rs`, and `STABILITY_CLASSIFICATIONS.md`. Update this file when adding
or promoting operations.*
