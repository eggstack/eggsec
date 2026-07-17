# Migration Guide: Direct Functions to Engine Operations

This guide covers migrating from direct convenience functions to the unified
`Engine`/`AsyncEngine` dispatch path. The engine path provides consistent
policy enforcement, audit trails, structured errors, and typed event emission
for all twenty-two stable-core operations.

## Why Migrate

| Concern | Direct Functions | Engine Dispatch |
|---------|-----------------|-----------------|
| Policy enforcement | Implicit per-call | Single `EnforcementContext` gate |
| Audit trail | None | `DispatchAuditEvent` per operation |
| Error structure | Exception-based | `OperationError` (versioned DTO) + exceptions |
| Event emission | None | Typed `EventEnvelope` stream |
| Cancellation | Not supported | `CancellationToken` cooperative |
| Pipeline integration | Manual orchestration | `Pipeline` / `AsyncPipeline` |
| Checkpoint resume | Not supported | `CheckpointStore` + versioned schema |

## Before/After Pattern

### Before: Convenience Function

```python
import eggsec

scope = eggsec.Scope.allow_hosts(["127.0.0.1"])
result = eggsec.scan_ports("127.0.0.1", [22, 80, 443], scope)
for port in result.open_ports:
    print(f"  {port.port}: {port.service}")
```

### After: Engine Dispatch

```python
from eggsec import Engine, Scope, PortScanRequest

scope = Scope.allow_hosts(["127.0.0.1"])
engine = Engine(scope)

request = PortScanRequest("127.0.0.1", ports="22,80,443")
result = engine.run_port_scan(request)

if result.status.name() == "Completed":
    payload = result.payload
    for port in payload.open_ports:
        print(f"  {port.port}: {port.service}")
else:
    print(f"Error: {result.error_message}")
```

### After: Generic Dispatch

```python
from eggsec import Engine, Scope, OperationRequest

scope = Scope.allow_hosts(["127.0.0.1"])
engine = Engine(scope)

request = OperationRequest(
    operation="scan-ports",
    target="127.0.0.1",
    metadata={"ports": "22,80,443"},
)
result = engine.run(request)
```

## Async Before/After

### Before

```python
import asyncio
import eggsec

async def main():
    scope = eggsec.Scope.allow_hosts(["127.0.0.1"])
    result = await eggsec.async_scan_ports("127.0.0.1", [22, 80], scope)
    print(result.open_ports)

asyncio.run(main())
```

### After

```python
import asyncio
from eggsec import AsyncEngine, Scope, PortScanRequest

async def main():
    scope = Scope.allow_hosts(["127.0.0.1"])
    engine = AsyncEngine(scope)

    request = PortScanRequest("127.0.0.1", ports="22,80")
    result = await engine.run_port_scan(request)

    if result.status.name() == "Completed":
        for port in result.payload.open_ports:
            print(f"  {port.port}: {port.service}")

asyncio.run(main())
```

## All 22 Stable Operations

### Original Ten (Stable Core)

| # | Operation ID | Old Function | Engine Method | Request Type |
|---|-------------|--------------|---------------|--------------|
| 1 | `scan-ports` | `scan_ports()` | `engine.run_port_scan()` | `PortScanRequest` |
| 2 | `scan-endpoints` | `scan_endpoints()` | `engine.run_endpoint_scan()` | `EndpointScanRequest` |
| 3 | `fingerprint-services` | `fingerprint_services()` | `engine.run_fingerprint()` | `FingerprintRequest` |
| 4 | `recon` | `recon_dns()` | `engine.run_recon_dns()` | `ReconDnsRequest` |
| 5 | `tls-inspect` | `inspect_tls()` | `engine.run_tls_inspect()` | `TlsInspectRequest` |
| 6 | `tech-detect` | `detect_technology()` | `engine.run_tech_detect()` | `TechDetectRequest` |
| 7 | `waf-detect` | `detect_waf()` | `engine.run_waf_detect()` | `WafDetectRequest` |
| 8 | `waf-validate` | `validate_waf()` | `engine.run_waf_validate()` | `WafValidateRequest` |
| 9 | `http-fuzz` | `fuzz_http()` | `engine.run_http_fuzz()` | `FuzzRequest` |
| 10 | `load-test` | `load_test_http()` | `engine.run_load_test()` | `LoadTestRequest` |

### Twelve Promoted Domains

| # | Operation ID | Old Function | Engine Method | Feature Gate |
|---|-------------|--------------|---------------|-------------|
| 11 | `git-secrets` | `scan_git_secrets()` | `engine.run_git_secrets_scan()` | `git-secrets` |
| 12 | `sbom` | `generate_sbom()` | `engine.run_sbom_generation()` | `sbom` |
| 13 | `consolidated-recon` | `run_consolidated_recon()` | `engine.run_consolidated_recon()` | — |
| 14 | `graphql-test` | `graphql_test()` | `engine.run_graphql_test()` | — |
| 15 | `oauth-test` | `oauth_test()` | `engine.run_oauth_test()` | — |
| 16 | `auth-test` | `auth_test()` | `engine.run_auth_test()` | — |
| 17 | `db-pentest` | `db_probe()` | `engine.run_db_probe()` | `db-pentest` |
| 18 | `nse-run` | `nse_run()` | `engine.run_nse_run()` | `nse` |
| 19 | `scan-docker-image` | `scan_docker_image()` | `engine.run_docker_scan()` | `container` |
| 20 | `scan-kubernetes` | `scan_kubernetes()` | `engine.run_kubernetes_scan()` | `container` |
| 21 | `analyze-apk` | `analyze_apk()` | `engine.run_apk_analysis()` | `mobile` |
| 22 | `analyze-ipa` | `analyze_ipa()` | `engine.run_ipa_analysis()` | `mobile` |

### Operation-by-Operation Examples

#### 1. scan-ports

```python
# Old
result = eggsec.scan_ports("10.0.0.1", [22, 80, 443], scope)

# New — typed request
from eggsec import PortScanRequest
request = PortScanRequest("10.0.0.1", ports="22,80,443")
result = engine.run_port_scan(request)

# New — generic dispatch
from eggsec import OperationRequest
request = OperationRequest("scan-ports", "10.0.0.1", metadata={"ports": "22,80,443"})
result = engine.run(request)
```

#### 2. scan-endpoints

```python
# Old
result = eggsec.scan_endpoints("https://example.com", scope)

# New
from eggsec import EndpointScanRequest
request = EndpointScanRequest("https://example.com", paths=["/admin", "/api"])
result = engine.run_endpoint_scan(request)
```

#### 3. fingerprint-services

```python
# Old
result = eggsec.fingerprint_services("10.0.0.1", scope)

# New
from eggsec import FingerprintRequest
request = FingerprintRequest("10.0.0.1", ports=[80, 443])
result = engine.run_fingerprint(request)
```

#### 4. recon_dns

```python
# Old
dns = eggsec.recon_dns("example.com")

# New
from eggsec import ReconDnsRequest
request = ReconDnsRequest("example.com", record_types=["A", "AAAA", "MX"])
result = engine.run_recon_dns(request)
```

#### 5. inspect_tls

```python
# Old
tls = eggsec.inspect_tls("example.com")

# New
from eggsec import TlsInspectRequest
request = TlsInspectRequest("example.com")
result = engine.run_tls_inspect(request)
```

#### 6. detect_technology

```python
# Old
tech = eggsec.detect_technology("https://example.com")

# New
from eggsec import TechDetectRequest
request = TechDetectRequest("https://example.com")
result = engine.run_tech_detect(request)
```

#### 7. detect_waf

```python
# Old
waf = eggsec.detect_waf("https://example.com", scope)

# New
from eggsec import WafDetectRequest
request = WafDetectRequest("https://example.com")
result = engine.run_waf_detect(request)
```

#### 8. validate_waf

```python
# Old
result = eggsec.validate_waf("https://example.com", scope)

# New — requires scope
from eggsec import WafValidateRequest
request = WafValidateRequest("https://example.com", payloads=["<script>alert(1)</script>"])
result = engine.run_waf_validate(request)
```

#### 9. fuzz_http

```python
# Old
result = eggsec.fuzz_http("https://example.com", scope)

# New — requires scope
from eggsec import FuzzRequest
request = FuzzRequest("https://example.com", payload_type="xss", threads=4)
result = engine.run_http_fuzz(request)
```

#### 10. load_test_http

```python
# Old
result = eggsec.load_test_http("https://example.com", scope)

# New — requires scope; risk-gated by policy
from eggsec import LoadTestRequest
request = LoadTestRequest("https://example.com", requests=100, concurrency=10)
result = engine.run_load_test(request)
```

#### 11. scan_git_secrets (feature: `git-secrets`)

```python
# Old
report = eggsec.scan_git_secrets("/path/to/repo", scope)

# New
from eggsec import GitSecretsScanRequest
request = GitSecretsScanRequest("/path/to/repo", max_commits=500)
result = engine.run_git_secrets_scan(request)
```

#### 12. generate_sbom (feature: `sbom`)

```python
# Old
report = eggsec.generate_sbom("/path/to/project", scope)

# New
from eggsec import SbomRequest
request = SbomRequest("/path/to/project", ecosystem="npm", format="cyclonedx")
result = engine.run_sbom_generation(request)
```

#### 13. run_consolidated_recon

```python
# Old
report = eggsec.run_consolidated_recon("example.com", scope)

# New
from eggsec import ConsolidatedReconConfig
config = ConsolidatedReconConfig(
    target="example.com",
    run_dns=True,
    run_ssl=True,
    run_tech_detect=True,
    run_subdomain=False,
)
result = engine.run_consolidated_recon(config)
```

#### 14. graphql_test

```python
# Old
result = eggsec.graphql_test("https://api.example.com/graphql", scope)

# New
from eggsec import GraphQLTestConfig
config = GraphQLTestConfig(target="https://api.example.com/graphql")
result = engine.run_graphql_test(config)
```

#### 15. oauth_test

```python
# Old
result = eggsec.oauth_test("https://auth.example.com", scope)

# New
from eggsec import OAuthTestConfig
config = OAuthTestConfig(target="https://auth.example.com")
result = engine.run_oauth_test(config)
```

#### 16. auth_test

```python
# Old
report = eggsec.auth_test("https://login.example.com", scope)

# New
from eggsec import AuthTestConfig
config = AuthTestConfig(target="https://login.example.com")
result = engine.run_auth_test(config)
```

#### 17. db_probe (feature: `db-pentest`)

```python
# Old
result = eggsec.db_probe("10.0.0.1", scope)

# New — uses SensitiveString for credentials
from eggsec import DbProbeRequest
request = DbProbeRequest(
    "10.0.0.1",
    port=5432,
    database="app_db",
    username="readonly",
    password="s3cret",  # wrapped in SensitiveString internally
)
result = engine.run_db_probe(request)
```

#### 18. nse_run (feature: `nse`)

```python
# Old
result = eggsec.nse_run("10.0.0.1", scope)

# New
from eggsec import NseRunRequest
request = NseRunRequest("10.0.0.1", scripts=["http-enum", "ssl-cert"])
result = engine.run_nse_run(request)
```

#### 19. scan_docker_image (feature: `container`)

```python
# Old
result = eggsec.scan_docker_image("nginx:latest", scope)

# New
from eggsec import DockerImageScanRequest
request = DockerImageScanRequest("nginx:latest")
result = engine.run_docker_scan(request)
```

#### 20. scan_kubernetes (feature: `container`)

```python
# Old
result = eggsec.scan_kubernetes("/path/to/k8s/manifests", scope)

# New
from eggsec import KubernetesScanRequest
request = KubernetesScanRequest("/path/to/k8s/manifests")
result = engine.run_kubernetes_scan(request)
```

#### 21. analyze_apk (feature: `mobile`)

```python
# Old
report = eggsec.analyze_apk("/path/to/app.apk", scope)

# New
from eggsec import ApkAnalysisRequest
request = ApkAnalysisRequest("/path/to/app.apk")
result = engine.run_apk_analysis(request)
```

#### 22. analyze_ipa (feature: `mobile`)

```python
# Old
report = eggsec.analyze_ipa("/path/to/app.ipa", scope)

# New
from eggsec import IpaAnalysisRequest
request = IpaAnalysisRequest("/path/to/app.ipa")
result = engine.run_ipa_analysis(request)
```

## Feature Flag Requirements

The convenience functions and engine methods require the same feature flags.
The feature must be enabled at compile time:

```python
import eggsec

# Check feature availability
features = eggsec.features()
print(features.get("git-secrets", False))   # True if compiled
print(features.get("db-pentest", False))    # True if compiled
print(features.get("container", False))     # True if compiled
print(features.get("mobile", False))        # True if compiled
print(features.get("nse", False))           # True if compiled
```

Build with features:

```bash
maturin develop --features git-secrets,sbom,db-pentest,container,mobile,nse
```

## Error Handling Changes

### Before: Exception-Only

```python
try:
    result = eggsec.scan_ports("10.0.0.1", [80], scope)
except eggsec.NetworkError as e:
    print(f"Network issue: {e}")
except eggsec.TimeoutError as e:
    print(f"Timed out: {e}")
```

### After: Result Status + Exceptions

```python
from eggsec import Engine, Scope, PortScanRequest, EggsecError

scope = Scope.allow_hosts(["10.0.0.1"])
engine = Engine(scope)

try:
    result = engine.run_port_scan(PortScanRequest("10.0.0.1", ports="80"))
except EggsecError as e:
    print(f"Engine error: {e}")
else:
    if result.status.name() == "Completed":
        print(result.payload.open_ports)
    elif result.status.name() == "Failed":
        error = result.error  # OperationError DTO
        print(f"Error type: {error.kind}")
        print(f"Message: {error.message}")
        print(f"Details: {error.details}")
```

### OperationError Structure

```python
error = result.error  # OperationError
print(error.kind)       # "network", "timeout", "scope_denial", etc.
print(error.message)    # Human-readable message
print(error.details)    # Optional additional context dict
print(error.error_message)  # Compatibility alias for message
```

### Error Kind Mapping

| `error.kind` | Python Exception | Description |
|-------------|-----------------|-------------|
| `validation` | `ConfigError` | Invalid configuration |
| `scope_denial` | `ScopeError` | Target not in scope |
| `policy_denial` | `EnforcementError` | Policy denied operation |
| `feature_unavailable` | `FeatureUnavailableError` | Feature not compiled |
| `network` | `NetworkError` | Network connectivity issue |
| `timeout` | `TimeoutError` | Operation timed out |
| `cancellation` | `CancellationError` | Operation cancelled |
| `scan` | `ScanError` | Scan execution failure |
| `serialization` | `SerializationError` | Parse/serialize failure |
| *(other)* | `InternalError` | Unexpected internal error |

## Preflight: Preview Before Dispatch

```python
from eggsec import EnforcementContext, ExecutionPolicy, ExecutionSurface, LoadedScope, OperationRegistry

scope = LoadedScope.default_empty()
policy = ExecutionPolicy.default()
ctx = EnforcementContext.manual_permissive(policy, scope)

# Look up operation
op = OperationRegistry.find("scan-ports")
desc = op.descriptor_for_target("10.0.0.1")

# Preview policy decision (no side effects)
outcome = ctx.evaluate(desc)
print(outcome.outcome_type)  # "allow", "confirm", or "deny"

# Approve (generates audit token)
approved = ctx.approve(ExecutionSurface.CLI_MANUAL, desc)
```

## Pipeline Integration

Engine dispatch integrates directly with the pipeline system:

```python
from eggsec import Pipeline, OperationRequest, Engine, Scope

scope = Scope.allow_hosts(["10.0.0.1"])
engine = Engine(scope)

pipeline = Pipeline("multi-scan")
pipeline.add_step("port-scan", OperationRequest("scan-ports", "10.0.0.1"))
pipeline.add_step("fingerprint", OperationRequest("fingerprint-services", "10.0.0.1"),
                  dependencies=["port-scan"])

result = pipeline.run(engine)
```

See [PIPELINE_SCHEMA.md](PIPELINE_SCHEMA.md) for the full pipeline reference.

## Phase F: Compatibility Policy, Tool/Schema API, and Maturity Governance

Phase F formalizes the compatibility policy, adds a semantic compatibility
checker, defines resource budgets, and tightens domain graduation evidence
requirements. It also completes the tool-core schema integration and
namespace governance started in earlier phases.

### 1. Tool/schema API (Phase A completion)

Release 5 Phase A exposed `eggsec-tool-core` types to Python. Phase F
confirms these as the authoritative tool abstraction for all 22 stable
operations.

```python
from eggsec import ToolRegistry, ToolRequest, SchemaGenerator

# Look up a tool descriptor
descriptor = ToolRegistry.find("scan-ports")
print(descriptor.label)          # "Port Scanner"
print(descriptor.risk)           # "safe_active"

# Generate JSON Schema for request/response
request_schema = SchemaGenerator.request_schema("scan-ports")
response_schema = SchemaGenerator.response_schema("scan-ports")

# Invoke via generic dispatch
request = ToolRequest(tool_id="scan-ports", target="10.0.0.1")
result = engine.invoke_tool(request)
```

### 2. Registry convergence and behavioral invariants (Phase B completion)

Phase B established a single authoritative `OperationExecutorDescriptor`
registry. Phase F enforces the following behavioral invariants via CI:

- One descriptor per operation ID (no duplicates)
- Unique IDs across all descriptors (no collisions)
- Schema identity: `ToolRegistry.find(op_id)` and
  `OperationRegistry.find(op_id)` return equivalent metadata
- Generic dispatch (`Engine.run(request)`) and typed dispatch
  (`Engine.run_port_scan(request)`) produce identical audit decisions

### 3. Canonical namespace structure (Phase C)

Phase C reorganized the Python package into intentional submodules. Phase F
confirms the canonical import paths and deprecates Py-suffixed names.

```python
# Canonical (recommended) — provisional types
from eggsec.net import Target, TcpSession, HttpClient
from eggsec.sessions import BrowserSession, MobileSession
from eggsec.storage import FindingRepository, AssessmentRepository
from eggsec.reporting import StreamingReporter, ReportDiff
from eggsec.daemon import DaemonClient

# Canonical — experimental types
from eggsec.experimental import wireless_scan, evasion_scan

# Deprecated — still works, emits DeprecationWarning
from eggsec import TargetPy, TcpSessionPy, HttpClientPy
```

### 4. Compatibility aliases and deprecations

Py-suffixed names (`TargetPy`, `TcpSessionPy`, etc.) are deprecated. They
emit `DeprecationWarning` when accessed and will be removed at the declared
removal floor. The removal floor is stated in each deprecation warning
message.

To suppress deprecation warnings during migration:

```python
import warnings
warnings.filterwarnings("ignore", category=DeprecationWarning, module="eggsec")

# Now deprecated names work without warnings
from eggsec import TargetPy
```

### 5. Wheel profile and installation changes

Phase F introduces machine-readable wheel profiles for release gating.
Each profile declares:

- Required cargo features
- Expected test counts and skip budgets
- Platform constraints
- Blocking status (whether the profile must pass before release)

```python
import eggsec

info = eggsec.build_info()
print(info["wheel_profile"])     # e.g., "default-wheel"
print(info["compiled_features"]) # list of compiled features
print(info["python_version"])    # e.g., "3.11"
```

```bash
# Validate wheel profile manifest
python scripts/validate_python_profiles.py

# Run a specific profile
python scripts/run_python_profile.py --profile default-wheel
```

### 6. Asyncio cancellation contract

Phase F formalizes the cancellation contract for async operations:

- `CancellationToken.cancel()` sets a cooperative cancellation flag
- The engine checks the token at scheduling points (between operations in a
  pipeline, at explicit yield points within long-running operations)
- When cancellation is detected, the operation returns a partial result with
  `ExecutionStatus.Cancelled` and any findings discovered before the cancel
  point are preserved
- Cancellation is not instantaneous: in-flight network operations will
  complete their current timeout before checking the token

```python
from eggsec import CancellationToken, AsyncEngine, Scope, PortScanRequest

token = CancellationToken()
engine = AsyncEngine(Scope.allow_hosts(["10.0.0.1"]))

# Cancel after 5 seconds
import asyncio
async def cancel_after(delay):
    await asyncio.sleep(delay)
    token.cancel("Timeout exceeded")

async def scan():
    asyncio.create_task(cancel_after(5))
    result = await engine.run_port_scan(
        PortScanRequest("10.0.0.1", ports="1-65535"),
        cancel_token=token,
    )
    # result.status may be Completed or Cancelled
    # findings discovered before cancel are preserved
    return result
```

### 7. Typing improvements (Phase D)

Phase D completed `.pyi` stub coverage and added `__hash__` to all enums.
Phase F confirms:

- All public enums support `__hash__` and `__eq__`
- All frozen pyclasses support `__hash__` and `__eq__`
- `from_dict()` / `from_json()` round-trip on core DTOs
- Context managers on all managed resources (sinks, callbacks, sessions)
- Strict `from_str` with `ValueError` on unknown enum strings

```python
from eggsec import Severity, Finding, ExecutionStatus

# Enum hashing and equality
assert Severity.HIGH == Severity.HIGH
assert hash(Severity.HIGH) == hash(Severity.HIGH)
assert Severity.HIGH != Severity.CRITICAL

# DTO round-trip
finding_dict = finding.to_dict()
finding2 = Finding.from_dict(finding_dict)
assert finding == finding2

# Context manager for callbacks
from eggsec import AuditSink
with AuditSink() as sink:
    # sink is automatically closed on exit
    pass
```

### 8. Maturity changes

Phase F does not promote any new domains to stable. The graduation
checklist remains the sole mechanism for stability promotion. The
compatibility checker applies maturity-aware severity rules:

| Maturity | Breaking change without allowlist |
|----------|----------------------------------|
| stable | **Blocking** — release cannot proceed |
| provisional | **Warning** — logged, not blocking |
| experimental | **Informational** — logged only |

### 9. Known unsupported platforms or subsystem prerequisites

| Platform / Subsystem | Status | Notes |
|---------------------|--------|-------|
| Windows x86_64 | Not built | CI evidence pending |
| macOS x86_64 | Supported | From source only |
| Linux aarch64 | Supported | From source only |
| Mobile dynamic analysis | Requires ADB + emulator | Not available in CI |
| Headless browser | Requires browser backend | Not available in CI |
| Packet live capture | Requires root / CAP_NET_RAW | Manual profile only |
| Stress testing | Requires root | Manual profile only |
| Wireless scanning | Requires root + wireless adapter | Manual profile only |
| Daemon client | Requires running daemon | Integration profile only |

### 10. Performance and size changes

Phase F introduces resource budgets for release gating:

- **Memory**: stable-core operations must not exceed 512 MiB peak RSS in
  the test harness
- **Binary size**: the default wheel must not exceed 15 MiB on x86_64
  Linux
- **Test count**: the default wheel must have at least 2800 passing tests
  (excluding skips/xfails)
- **Skip budget**: the default wheel must not exceed 120 skips (split by
  reason category)

### 11. Migration examples

#### From flat imports to organized submodules

```python
# Before (flat namespace, Phase B era)
from eggsec import TargetPy, TcpSessionPy, HttpClientPy
from eggsec import WirelessNetwork, EvasionTechnique

# After (organized submodules, Phase C+)
from eggsec.net import Target, TcpSession, HttpClient
from eggsec.experimental import WirelessNetwork, EvasionTechnique
```

#### From `api_surface()`-based integrations to tool-core

```python
# Before (introspection-based, Phase G era)
surface = eggsec.api_surface()
if "scan_ports" in surface and surface["scan_ports"]["stability"] == "stable":
    result = eggsec.scan_ports(target, ports, scope)

# After (tool-core registry, Phase A+)
from eggsec import ToolRegistry, ToolRequest
descriptor = ToolRegistry.find("scan-ports")
if descriptor:
    request = ToolRequest(tool_id="scan-ports", target=target)
    result = engine.invoke_tool(request)

# Or use the engine typed methods directly
result = engine.run_port_scan(PortScanRequest(target, ports="22,80,443"))
```

#### From direct functions to engine dispatch

```python
# Before
result = eggsec.scan_ports("10.0.0.1", [22, 80, 443], scope)

# After (full control)
from eggsec import Engine, Scope, PortScanRequest
engine = Engine(Scope.allow_hosts(["10.0.0.1"]))
result = engine.run_port_scan(PortScanRequest("10.0.0.1", ports="22,80,443"))
```

#### From deprecated warning utilities to stdlib

```python
# Before
from eggsec import deprecated_warning, DeprecatedWarning
deprecated_warning("Use new_api() instead")

# After (stdlib)
import warnings
warnings.warn("Use new_api() instead", DeprecationWarning, stacklevel=2)
```
