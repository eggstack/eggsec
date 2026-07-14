# API Reference

Complete reference for the `eggsec` Python package. All types are exported
from the top-level `eggsec` namespace.

```python
import eggsec
```

---

## Module-level functions

### `scan_ports`

```python
eggsec.scan_ports(
    target: str,
    ports: list[int],
    scope: Scope,
    *,
    concurrency: int = 100,
    timeout_ms: int = 5000,
) -> PortScanResult
```

Perform a TCP port scan. Creates an ephemeral `Client` internally; for
repeated scans against the same scope, use `Client` directly.

| Parameter | Type | Description |
|---|---|---|
| `target` | `str` | Hostname or IP to scan. |
| `ports` | `list[int]` | Port numbers to scan. |
| `scope` | `Scope` | Scope defining authorized targets. |
| `concurrency` | `int` | Max concurrent connections. |
| `timeout_ms` | `int` | Connection timeout in milliseconds. |

**Returns:** `PortScanResult`
**Raises:** `EnforcementError`, `ScanError`

---

### `async_scan_ports`

```python
eggsec.async_scan_ports(
    target: str,
    ports: list[int],
    scope: Scope,
    *,
    concurrency: int = 100,
    timeout_ms: int = 5000,
) -> PyFuture
```

Async version of `scan_ports`. Returns a `PyFuture` that resolves to
`PortScanResult`.

---

### `scan_endpoints`

```python
eggsec.scan_endpoints(
    base_url: str,
    endpoints: list[str],
    scope: Scope,
    *,
    concurrency: int = 20,
    timeout_ms: int = 30000,
    include_404: bool = False,
    verify_tls: bool = True,
) -> EndpointScanResult
```

Perform endpoint discovery against a web server.

| Parameter | Type | Description |
|---|---|---|
| `base_url` | `str` | Base URL to scan (e.g. `"https://example.com"`). |
| `endpoints` | `list[str]` | Paths to probe (e.g. `["admin", "login"]`). |
| `scope` | `Scope` | Scope defining authorized targets. |
| `concurrency` | `int` | Max concurrent requests. |
| `timeout_ms` | `int` | Request timeout in milliseconds. |
| `include_404` | `bool` | Include 404 responses in results. |
| `verify_tls` | `bool` | Verify TLS certificates. |

**Returns:** `EndpointScanResult`
**Raises:** `EnforcementError`, `ScanError`

---

### `async_scan_endpoints`

```python
eggsec.async_scan_endpoints(
    base_url: str,
    endpoints: list[str],
    scope: Scope,
    *,
    concurrency: int = 20,
    timeout_ms: int = 30000,
    include_404: bool = False,
    verify_tls: bool = True,
) -> PyFuture
```

Async version of `scan_endpoints`.

---

### `fingerprint_services`

```python
eggsec.fingerprint_services(
    target: str,
    ports: list[int],
    scope: Scope,
    *,
    concurrency: int = 100,
    timeout_ms: int = 2000,
) -> FingerprintScanResult
```

Perform service fingerprinting on target ports.

| Parameter | Type | Description |
|---|---|---|
| `target` | `str` | Hostname or IP to fingerprint. |
| `ports` | `list[int]` | Ports to fingerprint. |
| `scope` | `Scope` | Scope defining authorized targets. |
| `concurrency` | `int` | Max concurrent connections. |
| `timeout_ms` | `int` | Connection timeout in milliseconds. |

**Returns:** `FingerprintScanResult`
**Raises:** `EnforcementError`, `ScanError`

---

### `async_fingerprint_services`

```python
eggsec.async_fingerprint_services(
    target: str,
    ports: list[int],
    scope: Scope,
    *,
    concurrency: int = 100,
    timeout_ms: int = 2000,
) -> PyFuture
```

Async version of `fingerprint_services`.

---

### `recon_dns`

```python
eggsec.recon_dns(domain: str) -> DnsRecordSet
```

Perform DNS resolution and record enumeration for a domain.

| Parameter | Type | Description |
|---|---|---|
| `domain` | `str` | Domain name to look up (e.g. `"example.com"`). |

**Returns:** `DnsRecordSet`
**Raises:** `NetworkError`

---

### `async_recon_dns`

```python
eggsec.async_recon_dns(domain: str) -> PyFuture
```

Async version of `recon_dns`.

---

### `inspect_tls`

```python
eggsec.inspect_tls(host: str, *, port: int = 443) -> TlsInspectionResult
```

Inspect TLS certificate and configuration for a host.

| Parameter | Type | Description |
|---|---|---|
| `host` | `str` | Hostname to inspect (e.g. `"example.com"`). |
| `port` | `int` | TLS port (default `443`). |

**Returns:** `TlsInspectionResult`
**Raises:** `NetworkError`

---

### `async_inspect_tls`

```python
eggsec.async_inspect_tls(host: str, *, port: int = 443) -> PyFuture
```

Async version of `inspect_tls`.

---

### `detect_technology`

```python
eggsec.detect_technology(url: str) -> TechDetectionResult
```

Detect technology stack from HTTP response headers and body.

| Parameter | Type | Description |
|---|---|---|
| `url` | `str` | Full URL to inspect (e.g. `"https://example.com"`). |

**Returns:** `TechDetectionResult`
**Raises:** `EnforcementError`, `NetworkError`

---

### `async_detect_technology`

```python
eggsec.async_detect_technology(url: str) -> PyFuture
```

Async version of `detect_technology`.

---

### `detect_waf`

```python
eggsec.detect_waf(url: str, scope: Scope) -> WafDetectionResult
```

Detect WAF by making an HTTP request to the target URL. Performs passive
detection only -- no bypass or validation testing.

| Parameter | Type | Description |
|---|---|---|
| `url` | `str` | Target URL to test (e.g. `"https://example.com"`). |
| `scope` | `Scope` | Scope defining authorized targets. |

**Returns:** `WafDetectionResult`
**Raises:** `EnforcementError`, `NetworkError`

---

### `async_detect_waf`

```python
eggsec.async_detect_waf(url: str, scope: Scope) -> PyFuture
```

Async version of `detect_waf`.

---

### `validate_waf`

```python
eggsec.validate_waf(
    url: str,
    scope: Scope,
    *,
    bypass: bool = False,
    test_type: str | None = None,
) -> WafDetectionResult
```

Validate WAF bypass techniques against a target. Scope is enforced before
any engine work is dispatched.

| Parameter | Type | Description |
|---|---|---|
| `url` | `str` | Target URL to test. |
| `scope` | `Scope` | Scope defining authorized targets. |
| `bypass` | `bool` | Enable bypass techniques. |
| `test_type` | `str \| None` | Specific test type to run. |

**Returns:** `WafDetectionResult`
**Raises:** `EnforcementError`, `ScanError`

---

### `async_validate_waf`

```python
eggsec.async_validate_waf(
    url: str,
    scope: Scope,
    *,
    bypass: bool = False,
    test_type: str | None = None,
) -> PyFuture
```

Async version of `validate_waf`.

---

### `fuzz_http`

```python
eggsec.fuzz_http(
    url: str,
    scope: Scope,
    payload_type: str = "all",
    *,
    method: str = "GET",
    param: str | None = None,
    concurrency: int = 10,
    timeout: int = 30,
) -> FuzzResult
```

Perform HTTP fuzzing against a target. Scope is enforced before any engine
work is dispatched.

| Parameter | Type | Description |
|---|---|---|
| `url` | `str` | Target URL. |
| `scope` | `Scope` | Scope defining authorized targets. |
| `payload_type` | `str` | Payload category (e.g. `"all"`, `"xss"`). |
| `method` | `str` | HTTP method. |
| `param` | `str \| None` | Target parameter. |
| `concurrency` | `int` | Max concurrent requests. |
| `timeout` | `int` | Request timeout in seconds. |

**Returns:** `FuzzResult`
**Raises:** `EnforcementError`, `ScanError`

---

### `async_fuzz_http`

```python
eggsec.async_fuzz_http(
    url: str,
    scope: Scope,
    payload_type: str = "all",
    *,
    method: str = "GET",
    param: str | None = None,
    concurrency: int = 10,
    timeout: int = 30,
) -> PyFuture
```

Async version of `fuzz_http`.

---

### `load_test_http`

```python
eggsec.load_test_http(
    url: str,
    total_requests: int,
    concurrency: int,
    timeout_secs: int,
    scope: Scope,
    *,
    method: str = "GET",
) -> LoadTestResult
```

Perform HTTP load testing against a target. Scope is enforced before any
engine work is dispatched. `total_requests`, `concurrency`, and
`timeout_secs` must all be greater than zero.

| Parameter | Type | Description |
|---|---|---|
| `url` | `str` | Target URL. |
| `total_requests` | `int` | Total requests to send. |
| `concurrency` | `int` | Concurrent workers. |
| `timeout_secs` | `int` | Request timeout in seconds. |
| `scope` | `Scope` | Scope defining authorized targets. |
| `method` | `str` | HTTP method. |

**Returns:** `LoadTestResult`
**Raises:** `EnforcementError`, `ScanError`, `ValueError`

---

### `async_load_test_http`

```python
eggsec.async_load_test_http(
    url: str,
    total_requests: int,
    concurrency: int,
    timeout_secs: int,
    scope: Scope,
    *,
    method: str = "GET",
) -> PyFuture
```

Async version of `load_test_http`.

---

### `features`

```python
eggsec.features() -> dict[str, bool]
```

Return a dictionary of feature flags and whether they are enabled in the
current build.

```python
>>> eggsec.features()
{'core': True, 'scanner': True, 'async-api': True, 'nse': False, ...}
```

---

### `has_feature`

```python
eggsec.has_feature(name: str) -> bool
```

Check whether a specific feature is enabled.

| Parameter | Type | Description |
|---|---|---|
| `name` | `str` | Feature name (e.g. `"scanner"`, `"nse"`). |

---

### `build_info`

```python
eggsec.build_info() -> dict
```

Return build metadata as a dictionary. Keys include `version`,
`rust_crate_version`, `package_name`, `target_triple`, and
`binding_version`.

```python
>>> eggsec.build_info()
{'version': '0.1.0', 'rust_crate_version': '0.1.0', ...}
```

---

## Classes

### `Scope`

```python
class Scope:
    frozen = True
```

Controls which targets and ports are authorized for scanning. Scope
violations raise `EnforcementError`.

#### Static constructors

```python
Scope.allow_hosts(hosts: list[str]) -> Scope
```

Create a scope allowing specific hosts. Entries containing `/` are treated
as CIDR ranges; others are hostnames or IPs.

```python
Scope.allow_cidrs(cidrs: list[str]) -> Scope
```

Create a scope allowing specific CIDR ranges.

```python
Scope.deny_all() -> Scope
```

Create a scope that denies all targets.

```python
Scope.from_file(path: str) -> Scope
```

Load a scope from a TOML or YAML file.

#### Methods

```python
Scope.is_target_allowed(target: str) -> bool
```

Check if a target is allowed by this scope.

```python
Scope.is_port_allowed(port: int) -> bool
```

Check if a port is allowed by this scope.

#### Example

```python
from eggsec import Scope

scope = Scope.allow_hosts(["example.com", "10.0.0.0/8"])
print(scope.is_target_allowed("example.com"))   # True
print(scope.is_target_allowed("evil.com"))      # False
print(scope.is_port_allowed(80))                 # True
```

---

### `Client`

```python
class Client:
    def __init__(
        self,
        scope: Scope,
        *,
        mode: str = "manual",
        concurrency: int = 100,
        timeout_ms: int = 5000,
    ) -> None: ...
```

Synchronous client for performing scoped security scans. The GIL is
released during network I/O.

| Parameter | Type | Default | Description |
|---|---|---|---|
| `scope` | `Scope` | (required) | Scope defining authorized targets. |
| `mode` | `str` | `"manual"` | `"manual"` or `"automation"`. |
| `concurrency` | `int` | `100` | Max concurrent connections. |
| `timeout_ms` | `int` | `5000` | Connection timeout in ms. |

#### Properties

| Property | Type | Description |
|---|---|---|
| `scope` | `Scope` | The client's scope. |
| `mode` | `str` | The client's mode. |

#### Methods

```python
Client.scan_ports(
    target: str,
    ports: list[int],
    *,
    concurrency: int | None = None,
    timeout_ms: int | None = None,
) -> PortScanResult
```

Perform a TCP port scan. `concurrency` and `timeout_ms` override the
client defaults when provided.

```python
Client.scan_endpoints(
    base_url: str,
    endpoints: list[str],
    *,
    concurrency: int | None = None,
    timeout_ms: int | None = None,
    include_404: bool = False,
    verify_tls: bool = True,
) -> EndpointScanResult
```

Perform endpoint discovery against a web server.

```python
Client.fingerprint_services(
    target: str,
    ports: list[int],
    *,
    concurrency: int | None = None,
    timeout_ms: int | None = None,
) -> FingerprintScanResult
```

Perform service fingerprinting on target ports.

```python
Client.recon_dns(domain: str) -> DnsRecordSet
```

Perform passive DNS reconnaissance on a domain.

```python
Client.inspect_tls(host: str, *, port: int = 443) -> TlsInspectionResult
```

Inspect TLS certificate and configuration for a host.

```python
Client.detect_technology(url: str) -> TechDetectionResult
```

Detect technology stack from HTTP response headers and body.

```python
Client.detect_waf(url: str) -> WafDetectionResult
```

Detect WAF by making an HTTP request to the target URL.

```python
Client.validate_waf(
    url: str,
    *,
    bypass: bool = False,
    test_type: str | None = None,
) -> WafDetectionResult
```

Validate WAF bypass techniques. Uses the client's internal scope for enforcement.

```python
Client.fuzz_http(
    url: str,
    payload_type: str = "all",
    *,
    method: str = "GET",
    param: str | None = None,
    concurrency: int = 10,
    timeout: int = 30,
) -> FuzzResult
```

Perform HTTP fuzzing. Uses the client's internal scope for enforcement.

```python
Client.load_test_http(
    url: str,
    total_requests: int,
    concurrency: int,
    timeout_secs: int,
    *,
    method: str = "GET",
) -> LoadTestResult
```

Perform HTTP load testing. Uses the client's internal scope for enforcement.

```python
Client.close() -> None
```

Close the client (no-op for sync client, exists for API consistency).

#### Context manager

```python
with Client(scope) as client:
    result = client.scan_ports("example.com", [80, 443])
```

---

### `AsyncClient`

```python
class AsyncClient:
    def __init__(
        self,
        scope: Scope,
        *,
        mode: str = "manual",
        concurrency: int = 100,
        timeout_ms: int = 5000,
    ) -> None: ...
```

Async client for performing scoped security scans. Provides the same
operations as `Client` but returns `PyFuture` objects that can be awaited
in Python.

Each async operation spawns a background thread with its own Tokio runtime.

#### Properties

| Property | Type | Description |
|---|---|---|
| `scope` | `Scope` | The client's scope. |
| `mode` | `str` | The client's mode. |

#### Methods

All methods return `PyFuture` (awaitable):

```python
AsyncClient.scan_ports(target, ports, *, concurrency=None, timeout_ms=None) -> PyFuture
AsyncClient.scan_endpoints(base_url, endpoints, *, concurrency=None, timeout_ms=None, include_404=False, verify_tls=True) -> PyFuture
AsyncClient.fingerprint_services(target, ports, *, concurrency=None, timeout_ms=None) -> PyFuture
AsyncClient.recon_dns(domain) -> PyFuture
AsyncClient.inspect_tls(host, *, port=443) -> PyFuture
AsyncClient.detect_technology(url) -> PyFuture
AsyncClient.detect_waf(url) -> PyFuture
AsyncClient.validate_waf(url, *, bypass=False, test_type=None) -> PyFuture
AsyncClient.fuzz_http(url, payload_type="all", *, method="GET", param=None, concurrency=10, timeout=30) -> PyFuture
AsyncClient.load_test_http(url, total_requests, concurrency, timeout_secs, *, method="GET") -> PyFuture
AsyncClient.close() -> None
```

#### Context manager

```python
async with AsyncClient(scope) as client:
    result = await client.scan_ports("example.com", [80, 443])
```

---

### `PortScanResult`

```python
class PortScanResult:
    frozen = True
```

Result of a port scan operation.

| Property | Type | Description |
|---|---|---|
| `target` | `str` | Scanned target. |
| `open_ports` | `list[OpenPort]` | Open ports found. |
| `scanned_ports` | `int` | Total ports scanned. |
| `elapsed_ms` | `int` | Scan duration in milliseconds. |
| `stats` | `ScanStats` | Scan statistics. |

#### Methods

```python
PortScanResult.to_dict() -> dict
PortScanResult.to_json() -> str
PortScanResult.to_rows() -> list[dict]
```

---

### `OpenPort`

```python
class OpenPort:
    frozen = True
```

A single open port from a scan result.

| Property | Type | Description |
|---|---|---|
| `port` | `int` | Port number. |
| `protocol` | `str` | Protocol (e.g. `"tcp"`). |
| `service` | `str` | Detected service name. |
| `banner` | `str | None` | Service banner, if captured. |
| `confidence` | `float` | Detection confidence (0.0--1.0). |

---

### `ScanStats`

```python
class ScanStats:
    frozen = True
```

Scan statistics.

| Property | Type | Description |
|---|---|---|
| `ports_scanned` | `int` | Total ports scanned. |
| `total_open` | `int` | Number of open ports. |
| `elapsed_ms` | `int` | Duration in milliseconds. |

---

### `PortRange`

```python
class PortRange:
    frozen = True
```

A port specification for scanning.

#### Static constructors

```python
PortRange.list(ports: list[int]) -> PortRange
```

Create from an explicit list of port numbers.

```python
PortRange.range(start: int, end: int) -> PortRange
```

Create from start to end (inclusive).

```python
PortRange.top_100() -> PortRange
```

Return the top 100 most common ports.

```python
PortRange.top_1000() -> PortRange
```

Return the top 1000 most common ports.

#### Properties

| Property | Type | Description |
|---|---|---|
| `ports` | `list[int]` | The list of port numbers. |

---

### `TimingPreset`

```python
class TimingPreset:
    frozen = True
```

Timing preset for scan speed.

#### Static constructors

```python
TimingPreset.paranoid() -> TimingPreset
TimingPreset.sneaky() -> TimingPreset
TimingPreset.polite() -> TimingPreset
TimingPreset.normal() -> TimingPreset
TimingPreset.aggressive() -> TimingPreset
TimingPreset.insane() -> TimingPreset
```

---

### `EndpointScanConfig`

```python
class EndpointScanConfig:
    frozen = True

    def __init__(
        self,
        base_url: str,
        endpoints: list[str],
        *,
        concurrency: int = 20,
        timeout_ms: int = 30000,
        include_404: bool = False,
        verify_tls: bool = True,
    ) -> None: ...
```

Configuration for an endpoint scan.

| Property | Type | Description |
|---|---|---|
| `base_url` | `str` | Base URL to scan. |
| `endpoints` | `list[str]` | Paths to probe. |
| `concurrency` | `int` | Max concurrent requests. |
| `timeout_ms` | `int` | Request timeout in ms. |
| `include_404` | `bool` | Include 404 responses. |
| `verify_tls` | `bool` | Verify TLS certificates. |

---

### `EndpointFinding`

```python
class EndpointFinding:
    frozen = True
```

A single endpoint result from a scan.

| Property | Type | Description |
|---|---|---|
| `url` | `str` | Full URL of the endpoint. |
| `path` | `str` | Probed path. |
| `status_code` | `int` | HTTP status code. |
| `content_length` | `int | None` | Response content length. |
| `content_type` | `str | None` | Response content type. |
| `redirect_location` | `str | None` | Redirect target, if any. |
| `interesting` | `bool` | Whether the engine flagged this endpoint. |
| `response_time_ms` | `int` | Response time in milliseconds. |

#### Methods

```python
EndpointFinding.to_dict() -> dict
EndpointFinding.to_json() -> str
```

---

### `EndpointScanResult`

```python
class EndpointScanResult:
    frozen = True
```

Result of an endpoint scan operation.

| Property | Type | Description |
|---|---|---|
| `base_url` | `str` | Base URL scanned. |
| `findings` | `list[EndpointFinding]` | Endpoint findings. |
| `endpoints_found` | `int` | Number of endpoints found. |
| `elapsed_ms` | `int` | Scan duration in ms. |
| `stats` | `EndpointScanStats` | Scan statistics. |

#### Methods

```python
EndpointScanResult.to_dict() -> dict
EndpointScanResult.to_json() -> str
EndpointScanResult.to_rows() -> list[dict]
```

---

### `EndpointScanStats`

```python
class EndpointScanStats:
    frozen = True
```

Statistics for an endpoint scan.

| Property | Type | Description |
|---|---|---|
| `endpoints_scanned` | `int` | Total endpoints probed. |
| `endpoints_found` | `int` | Endpoints that returned a response. |
| `interesting_findings` | `int` | Endpoints flagged as interesting. |
| `elapsed_ms` | `int` | Duration in milliseconds. |

---

### `FingerprintEvidence`

```python
class FingerprintEvidence:
    frozen = True
```

Evidence for a service fingerprint match.

| Property | Type | Description |
|---|---|---|
| `probe` | `str` | Probe name that triggered the match. |
| `pattern` | `str` | Pattern that was matched. |
| `matched` | `bool` | Whether the pattern matched. |

---

### `FingerprintConfidence`

```python
class FingerprintConfidence:
    frozen = True
```

Confidence level for a service fingerprint.

| Property | Type | Description |
|---|---|---|
| `score` | `int` | Confidence score (0--100). |
| `level` | `str` | Human-readable level (e.g. `"high"`). |

---

### `ServiceFingerprintResult`

```python
class ServiceFingerprintResult:
    frozen = True
```

A single service fingerprint from a scan.

| Property | Type | Description |
|---|---|---|
| `port` | `int` | Port number. |
| `service` | `str` | Detected service name. |
| `banner` | `str | None` | Service banner. |
| `version` | `str | None` | Detected version. |
| `product` | `str | None` | Product name. |
| `extra` | `str | None` | Additional information. |
| `confidence` | `int` | Detection confidence (0--100). |

#### Methods

```python
ServiceFingerprintResult.to_dict() -> dict
ServiceFingerprintResult.to_json() -> str
```

---

### `FingerprintScanResult`

```python
class FingerprintScanResult:
    frozen = True
```

Result of a service fingerprinting scan.

| Property | Type | Description |
|---|---|---|
| `target` | `str` | Scanned target. |
| `services` | `list[ServiceFingerprintResult]` | Service fingerprints found. |
| `services_identified` | `int` | Number of services identified. |
| `elapsed_ms` | `int` | Scan duration in milliseconds. |

#### Methods

```python
FingerprintScanResult.to_dict() -> dict
FingerprintScanResult.to_json() -> str
FingerprintScanResult.to_rows() -> list[dict]
```

---

## Enums

### `Severity`

```python
class Severity:
    Critical: Severity
    High: Severity
    Medium: Severity
    Low: Severity
    Info: Severity
```

Severity level for findings.

#### Static methods

```python
Severity.from_str(s: str) -> Severity
```

Parse a severity string. Accepts `"critical"`, `"high"`, `"medium"`,
`"low"`, `"info"`, or `"informational"` (case-insensitive).

---

## Findings & Reporting

### `Evidence`

```python
class Evidence:
    frozen = True

    def __init__(
        self,
        kind: str,
        value: str,
        source: str,
        *,
        confidence: float = 1.0,
        metadata: dict[str, str] | None = None,
    ) -> None: ...
```

Evidence supporting a finding.

| Property | Type | Description |
|---|---|---|
| `kind` | `str` | Evidence type (e.g. `"header"`, `"response"`). |
| `value` | `str` | Evidence value. |
| `source` | `str` | Source of the evidence. |
| `confidence` | `float` | Confidence (0.0--1.0). |
| `metadata` | `dict[str, str]` | Additional key-value metadata. |

#### Methods

```python
Evidence.to_dict() -> dict
Evidence.to_json() -> str
```

---

### `Finding`

```python
class Finding:
    frozen = True

    def __init__(
        self,
        id: str,
        title: str,
        severity: Severity,
        target: str,
        category: str,
        description: str,
        *,
        recommendation: str | None = None,
        evidence: list[Evidence] | None = None,
        metadata: dict[str, str] | None = None,
    ) -> None: ...
```

A security finding.

| Property | Type | Description |
|---|---|---|
| `id` | `str` | Unique finding identifier. |
| `title` | `str` | Short title. |
| `severity` | `Severity` | Severity level. |
| `target` | `str` | Affected target. |
| `category` | `str` | Finding category (e.g. `"port-scan"`, `"waf-detection"`). |
| `description` | `str` | Detailed description. |
| `recommendation` | `str | None` | Remediation recommendation. |
| `evidence` | `list[Evidence]` | Supporting evidence items. |
| `metadata` | `dict[str, str]` | Additional key-value metadata. |

#### Methods

```python
Finding.to_dict() -> dict
Finding.to_json() -> str
Finding.to_row() -> dict
```

---

### `FindingSet`

```python
class FindingSet:
    def __init__(self) -> None: ...
```

A mutable collection of findings.

#### Methods

```python
FindingSet.add_finding(finding: Finding) -> None
```

Add a finding to the set.

```python
FindingSet.by_severity(severity: Severity) -> list[Finding]
```

Return findings matching the given severity.

#### Properties

| Property | Type | Description |
|---|---|---|
| `findings` | `list[Finding]` | All findings in the set. |

#### Other methods

```python
FindingSet.to_dicts() -> list[dict]
FindingSet.to_rows() -> list[dict]
len(finding_set) -> int
```

---

### `Report`

```python
class Report:
    def __init__(
        self,
        metadata: dict[str, str] | None = None,
    ) -> None: ...
```

A report aggregating multiple scan results.

| Parameter | Type | Description |
|---|---|---|
| `metadata` | `dict[str, str] | None` | Report metadata (e.g. author, date). |

#### Methods

```python
Report.add_finding(finding: Finding) -> None
```

Add a finding to the report.

```python
Report.add_finding_set(finding_set: FindingSet) -> None
```

Add all findings from a `FindingSet`.

```python
Report.add_result(result) -> None
```

Add results from a scan result object. Accepts `PortScanResult`,
`EndpointScanResult`, `FingerprintScanResult`, or `WafDetectionResult`.
Open ports and discovered endpoints are converted to `Finding` objects
with `Severity.Info`.

```python
Report.to_dict() -> dict
Report.to_json() -> str
Report.to_rows() -> list[dict]
Report.write_json(path: str) -> None
Report.write_markdown(path: str) -> None
```

#### Properties

| Property | Type | Description |
|---|---|---|
| `findings` | `list[Finding]` | All findings in the report. |
| `metadata` | `dict[str, str]` | Report metadata. |

---

## Recon

### `DnsRecordSet`

```python
class DnsRecordSet:
    frozen = True
```

DNS records for a domain.

| Property | Type | Description |
|---|---|---|
| `domain` | `str` | Queried domain. |
| `a` | `list[str]` | A records. |
| `aaaa` | `list[str]` | AAAA records. |
| `cname` | `list[str]` | CNAME records. |
| `mx` | `list[MxRecord]` | MX records. |
| `txt` | `list[str]` | TXT records. |
| `ns` | `list[str]` | NS records. |
| `soa` `SoaRecord | None` | SOA record. |
| `caa` | `list[str]` | CAA records. |

#### Methods

```python
DnsRecordSet.to_dict() -> dict
DnsRecordSet.to_json() -> str
```

---

### `MxRecord`

```python
class MxRecord:
    frozen = True
```

An MX record.

| Property | Type | Description |
|---|---|---|
| `preference` | `int` | MX preference value. |
| `exchange` | `str` | Mail exchange hostname. |

---

### `SoaRecord`

```python
class SoaRecord:
    frozen = True
```

A SOA record.

| Property | Type | Description |
|---|---|---|
| `mname` | `str` | Primary nameserver. |
| `rname` | `str` | Responsible party email. |
| `serial` | `int` | Serial number. |
| `refresh` | `int` | Refresh interval. |
| `retry` | `int` | Retry interval. |
| `expire` | `int` | Expiration time. |
| `minimum` | `int` | Minimum TTL. |

---

### `TlsCertificateInfo`

```python
class TlsCertificateInfo:
    frozen = True
```

TLS certificate information.

| Property | Type | Description |
|---|---|---|
| `subject` | `str` | Certificate subject. |
| `issuer` | `str` | Certificate issuer. |
| `valid_from` | `str` | Not-before date. |
| `valid_until` | `str` | Not-after date. |
| `serial_number` | `str` | Serial number. |
| `signature_algorithm` | `str` | Signature algorithm. |
| `public_key_algorithm` | `str` | Public key algorithm. |
| `key_size` | `int | None` | Key size in bits. |
| `is_expired` | `bool` | Whether the certificate is expired. |
| `days_until_expiry` | `int | None` | Days until expiration. |
| `subject_alternative_names` | `list[str]` | SANs. |

---

### `TlsInspectionResult`

```python
class TlsInspectionResult:
    frozen = True
```

TLS inspection result.

| Property | Type | Description |
|---|---|---|
| `target` | `str` | Inspected host. |
| `has_ssl` | `bool` | Whether SSL/TLS is available. |
| `certificate` | `TlsCertificateInfo | None` | Certificate details. |
| `supported_versions` | `list[str]` | Supported TLS versions. |
| `supported_cipher_suites` | `list[str]` | Supported cipher suites. |
| `issues` | `list[SslIssue]` | Detected issues. |

#### Methods

```python
TlsInspectionResult.to_dict() -> dict
TlsInspectionResult.to_json() -> str
```

---

### `SslIssue`

```python
class SslIssue:
    frozen = True
```

An SSL/TLS issue.

| Property | Type | Description |
|---|---|---|
| `severity` | `str` | Issue severity. |
| `code` | `str` | Issue code. |
| `description` | `str` | Issue description. |

---

### `TechStack`

```python
class TechStack:
    frozen = True
```

Technology stack detected on a target.

| Property | Type | Description |
|---|---|---|
| `servers` | `list[str]` | Server software (e.g. `["nginx"]`). |
| `frameworks` | `list[str]` | Web frameworks (e.g. `["Django"]`). |
| `languages` | `list[str]` | Programming languages (e.g. `["Python"]`). |
| `databases` | `list[str]` | Database systems. |
| `cdns` | `list[str]` | CDN providers. |
| `cms` | `list[str]` | Content management systems. |
| `javascript` | `list[str]` | JavaScript libraries. |
| `other` | `list[str]` | Other detected technologies. |

---

### `TechDetectionResult`

```python
class TechDetectionResult:
    frozen = True
```

Technology detection result.

| Property | Type | Description |
|---|---|---|
| `url` | `str` | Inspected URL. |
| `status_code` | `int` | HTTP status code. |
| `headers` | `dict[str, str]` | Response headers. |
| `tech_stack` | `TechStack` | Detected technology stack. |

#### Methods

```python
TechDetectionResult.to_dict() -> dict
TechDetectionResult.to_json() -> str
```

---

## WAF Detection

### `WafDetectionResult`

```python
class WafDetectionResult:
    frozen = True
```

WAF detection result from HTTP response analysis.

| Property | Type | Description |
|---|---|---|
| `url` | `str` | Tested URL. |
| `detected` | `bool` | Whether a WAF was detected. |
| `vendor` | `str | None` | Detected WAF vendor. |
| `waf_name` | `str | None` | WAF product name. |
| `confidence` | `int` | Detection confidence (0--100). |
| `server_header` | `str | None` | Server response header. |
| `status_code` | `int` | HTTP status code. |
| `request_error` | `str | None` | Error message if the request failed. |
| `matched_headers` | `list[str]` | Headers that matched WAF signatures. |
| `matched_cookies` | `list[str]` | Cookies that matched WAF signatures. |
| `matched_patterns` | `list[str]` | Body patterns that matched. |

#### Methods

```python
WafDetectionResult.to_dict() -> dict
WafDetectionResult.to_json() -> str
```

---

## Version Constants

Module-level constants for schema and ABI versioning (G7):

| Constant | Type | Description |
|---|---|---|
| `SCHEMA_VERSION` | `str` | Finding schema version (`"1.0"`). |
| `PROTOCOL_VERSION` | `str` | Daemon/gRPC protocol version (`"1.0.0"`). |
| `ABI_VERSION` | `str` | Native ABI version (`"1"`). |
| `EVENT_SCHEMA_VERSION` | `str` | Event schema version (`"1.0.0"`). |
| `FINDING_SCHEMA_VERSION` | `str` | Finding schema version for versioned findings. |

```python
import eggsec
print(eggsec.SCHEMA_VERSION)       # "1.0"
print(eggsec.PROTOCOL_VERSION)     # "1.0.0"
print(eggsec.ABI_VERSION)          # "1"
print(eggsec.EVENT_SCHEMA_VERSION) # "1.0.0"
```

## Policy, Configuration & Execution Context (Milestone B)

These types are always available (no feature flags required). They expose the
engine's enforcement model, configuration system, and operation metadata
registry to Python.

### Module-level functions

#### `preflight_operation`

```python
eggsec.preflight_operation(
    operation_id: str,
    target: str | None = None,
) -> PreflightResult
```

Preview the enforcement decision for an operation before dispatch. Returns a
`PreflightResult` with the outcome, suggested CLI flags, scope status, and
risk level.

| Parameter | Type | Description |
|---|---|---|
| `operation_id` | `str` | Operation to preview (e.g. `"port_scan"`). |
| `target` | `str \| None` | Optional target for scope checking. |

---

#### `validate_scope`

```python
eggsec.validate_scope(scope: LoadedScope) -> ScopeValidation
```

Validate a `LoadedScope` and return errors, warnings, and rule counts.

---

#### `audit_event_from_enforcement`

```python
eggsec.audit_event_from_enforcement(
    surface: str,
    operation_id: str,
    target: str | None,
    allowed: bool,
    denied: bool,
    confirmed: bool,
    override_ignored: bool,
    decision_summary: str,
    confirmation_classes: list[str],
    manual_override_reason: str | None,
    scope_source: str,
    scope_path: str | None,
    allow_rule_count: int,
    exclusion_rule_count: int,
    explicit_manifest: bool,
    policy_hash: str,
    correlation_id: str | None = None,
) -> EnforcementAuditEvent
```

Create an audit event from an enforcement outcome.

---

#### `audit_event_from_preflight`

```python
eggsec.audit_event_from_preflight(
    surface: str,
    operation_id: str,
    target: str | None,
    allowed: bool,
    denied: bool,
    decision_summary: str,
    confirmation_classes: list[str],
    scope_source: str,
    scope_path: str | None,
    allow_rule_count: int,
    exclusion_rule_count: int,
    explicit_manifest: bool,
    policy_hash: str,
    correlation_id: str | None = None,
) -> EnforcementAuditEvent
```

Create an audit event from a preflight result.

---

#### `emit_audit_event`

```python
eggsec.emit_audit_event(event: EnforcementAuditEvent) -> None
```

Emit an audit event to the logging sink.

---

### Classes

#### `EggsecConfig`

```python
class EggsecConfig:
    frozen = True
```

Full configuration model. Load from file or use defaults.

##### Static constructors

```python
EggsecConfig.load(path: str | None = None) -> EggsecConfig
EggsecConfig.default_path() -> str
```

##### Properties

| Property | Type | Description |
|---|---|---|
| `http` | `HttpConfig` | HTTP client settings. |
| `scan` | `ScanConfig` | Scan behavior settings. |
| `output` | `OutputConfig` | Output formatting settings. |
| `recon` | `ReconConfig` | Reconnaissance settings. |
| `profiles` | `dict[str, str]` | Named execution profiles. |
| `proxies` | `list[ProxyConfigEntry]` | Proxy configuration entries. |
| `remote` | `RemoteConfig` | Remote worker configuration. |
| `ai` | `AiConfig \| None` | AI integration settings. |
| `search` | `SearchConfig \| None` | Search engine settings. |
| `paths` | `PathsConfig` | File path settings. |
| `alert_channels` | `dict[str, AlertChannelConfig]` | Alert notification channels. |

##### Methods

```python
EggsecConfig.save(path: str | None = None) -> None
EggsecConfig.validate() -> list[str]  # empty list = valid
```

---

#### `SensitiveString`

```python
class SensitiveString:
    frozen = True
```

Zeroized secret wrapper. Value is not displayed in repr/str.

```python
SensitiveString.new(value: str) -> SensitiveString
sensitive.expose_secret() -> str
sensitive.is_empty() -> bool
```

---

#### `LoadedScope`

```python
class LoadedScope:
    frozen = True
```

Enriched scope with source tracking and validation.

##### Static constructors

```python
LoadedScope.default_empty() -> LoadedScope
```

##### Properties

| Property | Type | Description |
|---|---|---|
| `source` | `ScopeSource` | Where the scope was loaded from. |
| `path` | `str \| None` | File path, if loaded from file. |
| `is_explicit` | `bool` | Whether scope was explicitly provided. |
| `allowed_targets` | `list[ScopeRule]` | Allow rules. |
| `excluded_targets` | `list[ScopeRule]` | Exclusion rules. |
| `allowed_ports` | `list[int]` | Allowed ports. |
| `excluded_ports` | `list[int]` | Excluded ports. |
| `max_requests_per_second` | `int` | Rate limit. |

##### Methods

```python
LoadedScope.is_target_allowed(target: str) -> bool
LoadedScope.is_port_allowed(port: int) -> bool
LoadedScope.is_excluded(target: str) -> bool
LoadedScope.explain(target: str) -> ScopeExplanation
```

---

#### `OperationRegistry`

Static registry of all registered operations. All methods are static.

```python
OperationRegistry.all_operations() -> list[OperationMetadataView]
OperationRegistry.find(operation_id: str) -> OperationMetadataView | None
OperationRegistry.find_by_tool_id(tool_id: str) -> OperationMetadataView | None
```

#### Additional static methods (G1)

```python
OperationRegistry.operation_count() -> int
```

Total number of registered operations.

```python
OperationRegistry.operations_for_feature(feature: str) -> list[OperationMetadataView]
```

Return operations requiring a specific feature flag.

```python
OperationRegistry.operations_for_surface(surface: str) -> list[OperationMetadataView]
```

Return operations supporting a specific execution surface (`"cli"`, `"tui"`,
`"mcp"`, `"rest"`, `"agent"`, `"grpc"`).

```python
OperationRegistry.operation_ids() -> list[str]
```

All canonical operation identifiers.

```python
OperationRegistry.operation_names() -> list[str]
```

All human-readable operation display names.

---

#### `OperationMetadataView`

Read-only view of an operation's metadata.

| Property | Type | Description |
|---|---|---|
| `operation_id` | `str` | Unique operation identifier. |
| `operation_name` | `str` | Human-readable operation name. |
| `default_risk` | `OperationRisk` | Default risk level. |
| `default_mode` | `OperationMode` | Default operating mode. |
| `target_policy` | `TargetPolicyKind` | Target policy requirement. |
| `request_schema` | `str \| None` | JSON schema reference for the request type. |
| `result_schema` | `str \| None` | JSON schema reference for the result type. |
| `feature_required` | `str \| None` | Feature flag required, or None if always available. |
| `python_async_available` | `bool` | Whether an async variant exists. |
| `supported_surfaces` | `list[str]` | Execution surfaces supporting this operation. |
| `default_timeout_ms` | `int \| None` | Suggested timeout in milliseconds. |
| `required_features` | `list[str]` | Feature flags required (legacy field). |
| `required_capabilities` | `list[Capability]` | Capabilities required. |
| `target_required` | `bool` | Whether a target is required (legacy field). |

```python
OperationMetadataView.descriptor_for_target(target: str | None = None) -> OperationDescriptor
```

Create a mutable `OperationDescriptor` for a specific target.

---

#### `OperationDescriptor`

Mutable descriptor for a specific target. Created from
`OperationMetadataView.descriptor_for_target()`. Required by
`EnforcementContext.evaluate()`.

| Property | Type | Description |
|---|---|---|
| `operation_id` | `str` | Operation identifier. |
| `operation_label` | `str` | Human-readable label. |
| `risk` | `OperationRisk` | Risk level. |
| `mode` | `OperationMode` | Operation mode. |
| `intended_uses` | `list[IntendedUse]` | Intended use categories. |
| `required_capabilities` | `list[Capability]` | Required capabilities. |
| `requires_explicit_scope` | `bool` | Whether explicit scope is required. |
| `requires_target` | `bool` | Whether a target is required. |
| `requires_network` | `bool` | Whether network access is required. |
| `required_features` | `list[str]` | Required feature flags. |
| `target_policy` | `TargetPolicyKind` | Target policy kind. |

---

#### `EnforcementContext`

Policy evaluation gate. Mandatory pre-dispatch check for all surfaces.

##### Static constructors

```python
EnforcementContext.manual_permissive(policy: ExecutionPolicy, scope: LoadedScope) -> EnforcementContext
EnforcementContext.manual_guarded(policy: ExecutionPolicy, scope: LoadedScope) -> EnforcementContext
EnforcementContext.ci_strict(policy: ExecutionPolicy, scope: LoadedScope) -> EnforcementContext
EnforcementContext.mcp_strict(policy: ExecutionPolicy, scope: LoadedScope) -> EnforcementContext
EnforcementContext.agent_strict(policy: ExecutionPolicy, scope: LoadedScope) -> EnforcementContext
EnforcementContext.for_surface(surface: ExecutionSurface, policy: ExecutionPolicy, scope: LoadedScope) -> EnforcementContext
```

##### Methods

```python
EnforcementContext.evaluate(descriptor: OperationDescriptor) -> EnforcementOutcome
EnforcementContext.approve(surface: ExecutionSurface, descriptor: OperationDescriptor) -> ApprovedOperation
EnforcementContext.approve_manual(surface: ExecutionSurface, descriptor: OperationDescriptor, override: ManualOverride | None = None) -> ApprovedOperation
EnforcementContext.policy_hash() -> str
```

---

#### `ExecutionPolicy`

Risk-level policy configuration.

```python
ExecutionPolicy.default() -> ExecutionPolicy
ExecutionPolicy.from_config(config: EggsecConfig) -> ExecutionPolicy
```

| Property | Type | Description |
|---|---|---|
| `allow_passive` | `bool` | Allow passive-risk operations. |
| `allow_read_only` | `bool` | Allow read-only operations. |
| `allow_low_risk` | `bool` | Allow low-risk operations. |
| `allow_medium_risk` | `bool` | Allow medium-risk operations. |
| `allow_elevated_risk` | `bool` | Allow elevated-risk operations. |
| `allow_high_risk` | `bool` | Allow high-risk operations. |
| `allow_destructive` | `bool` | Allow destructive operations. |
| `allow_intrusive` | `bool` | Allow intrusive operations. |
| `allow_credential_access` | `bool` | Allow credential access. |
| `allow_network_intrusion` | `bool` | Allow network intrusion. |
| `allow_denial_of_service` | `bool` | Allow denial of service. |
| `allow_data_exfiltration` | `bool` | Allow data exfiltration. |
| `allow_privilege_escalation` | `bool` | Allow privilege escalation. |
| `allow_persistence` | `bool` | Allow persistence techniques. |
| `allow_agent_autonomous` | `bool` | Allow autonomous agent operations. |
| `require_confirmation_above_medium` | `bool` | Require confirmation above medium risk. |
| `require_confirmation_above_high` | `bool` | Require confirmation above high risk. |
| `allowed_capabilities` | `list[str]` | Allowed capability names. |
| `denied_capabilities` | `list[str]` | Denied capability names. |

---

#### `ManualOverride`

Override flags for manual (CLI/TUI) surfaces.

```python
ManualOverride(
    reason: str = "",
    assume_yes: bool = False,
    allow_out_of_scope: bool = False,
    allow_explicit_exclusion: bool = False,
    allow_high_risk: bool = False,
    allow_intrusive: bool = False,
    allow_credential_access: bool = False,
    allow_network_intrusion: bool = False,
    allow_denial_of_service: bool = False,
    allow_data_exfiltration: bool = False,
) -> ManualOverride
```

---

#### `ExecutionSurface`

Surface identification. Static constants:

| Constant | Description |
|---|---|
| `ExecutionSurface.CLI_MANUAL` | CLI interactive |
| `ExecutionSurface.TUI_MANUAL` | TUI interactive |
| `ExecutionSurface.CLI_MANUAL_STRICT` | CLI strict mode |
| `ExecutionSurface.TUI_MANUAL_STRICT` | TUI strict mode |
| `ExecutionSurface.MCP_SERVER` | MCP server |
| `ExecutionSurface.SECURITY_AGENT` | Security agent |
| `ExecutionSurface.CI` | CI pipeline |
| `ExecutionSurface.REST_API` | REST API |
| `ExecutionSurface.GRPC_API` | gRPC API |

Properties: `name`, `label`, `is_manual`, `is_agent_controlled`.

---

#### `ExecutionProfile`

Enforcement profile. Static constants:

| Constant | Description |
|---|---|
| `ExecutionProfile.manual_permissive` | Operator-directed, supports overrides |
| `ExecutionProfile.manual_guarded` | Operator-directed, guarded |
| `ExecutionProfile.ci_strict` | CI hard enforcement |
| `ExecutionProfile.mcp_strict` | MCP/REST strict |
| `ExecutionProfile.agent_strict` | Agent strict |

Properties: `name`, `is_strict`, `is_automated`.

---

#### `PreflightResult`

Pre-dispatch policy preview.

| Property | Type | Description |
|---|---|---|
| `operation_id` | `str` | Operation being previewed. |
| `target` | `str \| None` | Target, if provided. |
| `outcome` | `str` | `"allow"`, `"confirm"`, or `"deny"`. |
| `requires_confirmation` | `bool` | Whether confirmation is needed. |
| `confirmation_classes` | `list[str]` | Confirmation categories. |
| `suggested_cli_flags` | `list[str]` | Suggested CLI flags. |
| `warnings` | `list[str]` | Policy warnings. |
| `scope_status` | `str \| None` | Scope validation status. |
| `risk_level` | `str \| None` | Assessed risk level. |
| `surface` | `str \| None` | Surface name. |
| `profile` | `str \| None` | Profile name. |

---

#### `ApprovedOperation`

Authorization token from `EnforcementContext.approve()`.

| Property | Type | Description |
|---|---|---|
| `operation_id` | `str` | Approved operation. |
| `target` | `str \| None` | Approved target. |
| `risk` | `str` | Risk level. |
| `mode` | `str` | Operation mode. |
| `surface` | `str` | Execution surface. |
| `policy_hash` | `str` | Policy hash at approval time. |
| `audit_event_id` | `str` | Audit trail identifier. |

---

#### `EnforcementAuditEvent`

Audit trail entry for enforcement decisions.

| Property | Type | Description |
|---|---|---|
| `event_id` | `str` | Unique event identifier. |
| `timestamp` | `str` | ISO 8601 timestamp. |
| `surface` | `str` | Execution surface. |
| `profile` | `str` | Enforcement profile. |
| `operation_id` | `str` | Operation evaluated. |
| `target` | `str \| None` | Target, if any. |
| `outcome` | `AuditOutcome` | Decision outcome. |
| `decision_summary` | `str` | Human-readable decision. |
| `manual_override` | `ManualOverrideAudit \| None` | Override details, if used. |
| `scope` | `ScopeAudit` | Scope information. |
| `policy_hash` | `str` | Policy hash. |

```python
EnforcementAuditEvent.to_dict() -> dict
```

---

#### `AuditOutcome`

Decision outcome enum. Constants: `allow`, `warn`, `confirmed`, `deny`, `confirmation_required`.

```python
AuditOutcome.to_dict() -> dict[str, str]
```

---

#### `OperationRisk`

Risk level enum. Static constructors: `passive`, `read_only`, `low`, `medium`,
`elevated`, `high`, `destructive`, `intrusive`, `credential_access`,
`network_intrusion`, `denial_of_service`, `data_exfiltration`,
`privilege_escalation`, `persistence`, `agent_autonomous`.

Properties: `name`, `level` (int).

---

#### `Capability`

Capability enum. Static constructors: `passive_fingerprint`, `active_fingerprint`,
`port_scan`, `service_detection`, `vulnerability_scan`, `exploit_simulation`,
`credential_test`, `web_crawl`, `web_fuzz`, `web_inject`, `proxy_required`,
`packet_capture`, `packet_injection`, `stress_test`, `denial_of_service`,
`nse_script`, `database_query`, `mobile_static`, `mobile_dynamic_analysis`.

---

## Events (G2)

See [Events](events.md) for the full event protocol guide.

### `EventEnvelope`

Versioned wrapper for all events. Contains `schema_version`, `event_id`,
`timestamp_ms`, `correlation_id`, `event_type`, and `payload`.

```python
EventEnvelope(
    event_type: str,
    payload: object,
    *,
    event_id: str | None = None,
    timestamp_ms: int | None = None,
    correlation_id: str | None = None,
    schema_version: str | None = None,
)
```

### Typed event payloads

| Class | Key fields |
|---|---|
| `PlanningEvent` | `operation_id`, `target`, `scope_summary` |
| `PreflightEvent` | `outcome`, `confirmations_required`, `suggested_flags` |
| `StageLifecycleEvent` | `stage`, `status` |
| `ProgressEvent` | `percentage`, `message`, `items_processed`, `items_total` |
| `FindingEvent` | `finding_id`, `severity`, `title`, `auto_added` |
| `ArtifactEvent` | `artifact_name`, `kind`, `mime_type`, `size_bytes` |
| `CancellationEvent` | `reason`, `cancelled_by` |
| `FailureEvent` | `error_type`, `error_message`, `is_retryable` |
| `CompletionEvent` | `status`, `stats`, `duration_ms` |

### `EventStream`

Push-based event stream with filtering and iteration. See [Events](events.md).

```python
EventStream(event_log: EventLog | None = None)
EventStream.empty() -> EventStream
```

### `wrap_event`

```python
eggsec.wrap_event(
    event_type: str,
    payload: object,
    *,
    correlation_id: str | None = None,
    event_id: str | None = None,
) -> EventEnvelope
```

---

## Callbacks and Sinks (G3)

See [Callbacks](callbacks.md) for the full callback/sink guide.

### `AuditSink`

Receives enforcement audit events.

```python
AuditSink(callback: Callable[[dict], None])
```

### `FindingSink`

Receives findings as they are discovered.

```python
FindingSink(callback: Callable[[object], None])
```

### `ArtifactSink`

Receives artifact metadata when artifacts are produced.

```python
ArtifactSink(callback: Callable[[object], None])
```

### `ProgressSink`

Receives progress updates.

```python
ProgressSink(callback: Callable[[float, str], None])
```

### `EventConsumer`

Receives versioned `EventEnvelope` dicts.

```python
EventConsumer(callback: Callable[[dict], None])
```

### `AsyncCallback`

Wraps an `async def` handler for use from Rust callbacks.

```python
AsyncCallback(callback: CoroutineFunction)
```

### `CallbackScheduler`

Bounded callback queue with backpressure.

```python
CallbackScheduler(capacity: int = 1000)
```

### `BackpressureChannel`

Bounded in-process channel that drops oldest events when full.

```python
BackpressureChannel(capacity: int = 256)
```

---

## Domains (G1)

### `DomainDescriptorPy`

Describes a capability domain -- what it can do, how it integrates with
surfaces, and what feature gates control its availability.

| Property | Type | Description |
|---|---|---|
| `id` | `str` | Domain identifier (e.g. `"db-pentest"`). |
| `display_name` | `str` | Human-readable name. |
| `description` | `str` | Brief purpose description. |
| `category` | `str` | Classification (e.g. `"standard-assessment"`). |
| `required_feature` | `str \| None` | Cargo feature flag, or None if always available. |
| `operations` | `list[str]` | Operation IDs provided by this domain. |
| `is_available` | `bool` | Whether the domain is available in this build. |

```python
DomainDescriptorPy.to_dict() -> dict
```

### `DomainRegistry`

Static registry of domain descriptors.

```python
DomainRegistry.all_domains() -> list[DomainDescriptorPy]
DomainRegistry.available_domains() -> list[DomainDescriptorPy]
DomainRegistry.find(domain_id: str) -> DomainDescriptorPy | None
```

---

## Buffers and Paginated Results (G5)

### `BinaryBuffer`

Binary buffer with PEP 3118 zero-copy support.

```python
BinaryBuffer(data: bytes)
BinaryBuffer.from_bytes(data: bytes) -> BinaryBuffer
BinaryBuffer.from_hex(hex_str: str) -> BinaryBuffer
```

| Method | Returns | Description |
|---|---|---|
| `to_bytes()` | `bytes` | Raw bytes. |
| `memoryview()` | `memoryview` | PEP 3118 memoryview. |
| `hex()` | `str` | Hex-encoded representation. |
| `len()` | `int` | Number of bytes. |

Supports `len()`, `==`, and the buffer protocol (`memoryview(buf)`).

### `LazyArtifact`

Deferred artifact loading. Holds path and metadata without reading file content.

```python
LazyArtifact(path: str | Path, metadata: ArtifactMeta)
```

| Method | Returns | Description |
|---|---|---|
| `name()` | `str` | Artifact name (no I/O). |
| `kind()` | `str` | Artifact type. |
| `mime_type()` | `str` | MIME type. |
| `size_bytes()` | `int` | Size from metadata. |
| `content_hash()` | `str \| None` | Content hash, if available. |
| `path()` | `Path` | File path on disk. |
| `load()` | `BinaryBuffer` | Read file into memory. |
| `unload()` | `None` | Free loaded data. |
| `is_loaded()` | `bool` | Whether content is in memory. |

### `ArtifactMeta`

Metadata about an artifact (no content).

```python
ArtifactMeta(
    name: str,
    kind: str,
    mime_type: str,
    size: int,
    *,
    content_hash: str | None = None,
)
```

### `PaginatedResults`

Page-based iteration over a pre-materialized list.

```python
PaginatedResults(items: list, page_size: int = 100)
```

| Method | Returns | Description |
|---|---|---|
| `total_pages()` | `int` | Number of pages. |
| `get_page(page)` | `list` | Items on a specific page (0-indexed). |
| `get_page_info(page)` | `dict` | Page items with metadata (`has_next`, `has_prev`, etc.). |
| `to_list()` | `list` | All items. |
| `reset()` | `None` | Reset iterator position. |
| `count()` | `int` | Total item count. |

Supports the iterator protocol (`for item in results`) and `len()`.

---

## Introspection and Deprecation (G6)

### `api_surface()`

```python
eggsec.api_surface() -> dict[str, dict]
```

Returns a machine-readable dict of all exported names, their stability
level, and deprecation info. Keys are names; values are dicts with
`"stability"`, `"deprecated"`, and optionally `"deprecated_with"`.

```python
>>> surface = eggsec.api_surface()
>>> surface["scan_ports"]
{'stability': 'stable', 'deprecated': False}
```

### `feature_matrix()`

```python
eggsec.feature_matrix() -> dict[str, dict]
```

Returns a dict of all features with availability, description, and whether
system dependencies are required.

```python
>>> matrix = eggsec.feature_matrix()
>>> matrix["nse"]
{'available': False, 'description': 'Nmap NSE script execution', 'requires_system_deps': True}
```

### `api_surface_version()`

```python
eggsec.api_surface_version() -> dict
```

Returns package version, schema version, protocol version, ABI version,
and the list of available feature names.

```python
>>> info = eggsec.api_surface_version()
>>> info["schema_version"]
'1.0'
```

### `DeprecatedWarning`

Custom deprecation warning class (subclass of `DeprecationWarning`).

```python
DeprecatedWarning(msg: str | None = None)
```

### `deprecated_warning()`

```python
eggsec.deprecated_warning(msg: str) -> None
```

Emit a `DeprecatedWarning` via Python's `warnings` module.

### Experimental subpackage

The `eggsec.experimental` namespace is available for preview APIs.
Experimental APIs may change or be removed without notice.

```python
import eggsec.experimental  # available
```

---

## Exceptions

All exceptions inherit from `EggsecError`, which inherits from Python's
built-in `Exception`.

```
Exception
  └── EggsecError
        ├── ConfigError
        ├── ScopeError
        ├── EnforcementError
        ├── NetworkError
        ├── ScanError
        ├── TimeoutError
        ├── FeatureUnavailableError
        ├── SerializationError
        └── InternalError
```

| Exception | Description |
|---|---|
| `EggsecError` | Base exception for all eggsec errors. |
| `ConfigError` | Configuration is invalid or missing. |
| `ScopeError` | Scope file could not be read or parsed. |
| `EnforcementError` | Target or port is outside the allowed scope. |
| `NetworkError` | Network connectivity, DNS resolution, or HTTP error. |
| `ScanError` | Scan operation failed (payload, runtime, IO). |
| `TimeoutError` | Operation exceeded its timeout. |
| `FeatureUnavailableError` | Requested feature is not enabled in this build. |
| `SerializationError` | Failed to parse or serialize data. |
| `InternalError` | Unexpected internal engine error. |

### Catching exceptions

```python
import eggsec
from eggsec import Scope, EnforcementError, ScanError

scope = Scope.allow_hosts(["example.com"])
try:
    result = eggsec.scan_ports("evil.com", [80], scope)
except EnforcementError as e:
    print(f"Scope violation: {e}")
except ScanError as e:
    print(f"Scan failed: {e}")
except eggsec.EggsecError as e:
    print(f"Engine error: {e}")
```

---

## Release 2: Network Programmability

### Network Configuration (`eggsec.network`)

| Type | Description |
|------|-------------|
| `TargetPy` | Network target specification (host, port, scheme, path) |
| `ResolvedTargetPy` | DNS resolution result with IPs and timing |
| `ConnectionConfigPy` | Connection timeout and retry configuration |
| `TimeoutConfigPy` | Distinguished phase timeouts (connect, read, write, TLS, idle) |
| `RetryPolicyPy` | Retry policy with backoff configuration |
| `ProxyRoutePy` | Proxy route configuration (type, host, port, auth, no-proxy) |
| `SocketEndpointPy` | Socket endpoint info (address, port, family, loopback) |
| `ConnectionTimingPy` | Timing breakdown (DNS, TCP, TLS, TTFB, total) |
| `ConnectionMetadataPy` | Full connection metadata (endpoints, protocol, TLS, bytes) |
| `NetworkEvidencePy` | Evidence from network operations |
| `TranscriptEntryPy` | Single transcript entry (sent/received) |
| `NetworkTranscriptPy` | Ordered transcript collection |

**Functions:**
- `resolve_target_sync(target, timeout_ms=5000, max_results=100)` — Synchronous DNS resolution
- `async_resolve_target(target, timeout_ms=5000, max_results=100)` — Async DNS resolution

### TCP Sessions (`eggsec.transport`)

| Type | Description |
|------|-------------|
| `TcpConfigPy` | TCP connection configuration |
| `TcpSessionPy` | Managed TCP session (context manager) with transcript and byte counters |
| `TcpConnectResultPy` | Connection result with endpoints and timing |
| `TcpReadResultPy` | Read result with data, eof flag, and timing |
| `TcpWriteResultPy` | Write result with byte count and timing |

**Session properties:** `is_closed`, `config`, `transcript`, `bytes_sent`, `bytes_received`

**Functions:**
- `tcp_connect_probe(host, port, timeout_ms=5000)` — Single-shot TCP connect check
- `banner_probe(host, port, timeout_ms=5000, max_banner_bytes=4096)` — Connect and read banner

### UDP Sessions (`eggsec.transport`)

| Type | Description |
|------|-------------|
| `UdpConfigPy` | UDP socket configuration |
| `UdpSocketPy` | Managed UDP socket (context manager) with transcript and byte counters |
| `UdpSendResultPy` | Send result with byte count |
| `UdpRecvResultPy` | Receive result with data and truncation flag |
| `UdpRecvFromResultPy` | Receive result with source address |

**Session properties:** `is_closed`, `bytes_sent`, `bytes_received`, `transcript`

### Protocol Probes (`eggsec.probes`)

| Type | Description |
|------|-------------|
| `DnsQueryConfigPy` | DNS query configuration |
| `DnsRecordPy` | Individual DNS record |
| `DnsQueryResultPy` | DNS query result with records and metadata |
| `TlsProbeConfigPy` | TLS inspection configuration |
| `CertificateInfoPy` | TLS certificate information |
| `TlsProbeResultPy` | TLS probe result with cipher and version info |
| `TlsIssuePy` | TLS security issue |
| `HttpProbeConfigPy` | HTTP probe configuration |
| `HttpProbeResultPy` | HTTP probe result with headers, body, and timing |
| `UdpProbeConfigPy` | UDP reachability probe configuration |
| `UdpProbeResultPy` | UDP probe result with reachability and response data |

**Functions:**
- `dns_query(domain, record_types=None, resolver=None, timeout_ms=5000)` — DNS lookup
- `tls_probe(host, port=443, sni=None, timeout_ms=10000, verify_certificate=True)` — TLS inspection
- `http_probe(url, method="GET", timeout_ms=10000, follow_redirects=True)` — HTTP probe
- `udp_probe(host, port, payload=None, timeout_ms=5000, max_response_size=65535, retries=2)` — UDP reachability probe

### HTTP Client (`eggsec.http_client`)

| Type | Description |
|------|-------------|
| `HttpRequestPy` | HTTP request with duplicate-preserving headers and response size limit |
| `HttpHeadersPy` | Case-insensitive header container |
| `HttpResponsePy` | Full HTTP response with timing, TLS metadata, and chunked body iteration |
| `HttpCookiePy` | HTTP cookie with security attributes |
| `RedirectEntryPy` | Redirect history entry |
| `TlsMetadataPy` | TLS connection metadata |
| `HttpTimingPy` | Request timing breakdown |
| `HttpClientConfigPy` | Client pool and timeout configuration |
| `HttpClientPy` | Sync HTTP client (context manager) |
| `AsyncHttpClientPy` | Async HTTP client (async context manager) |
| `RedactConfigPy` | Sensitive header/body redaction configuration |

**HttpResponsePy methods:** `redacted_headers()`, `iter_body_chunks(chunk_size)`, `body_bytes_limited(max_bytes)`

**Functions:**
- `create_http_client(config)` — Create sync HTTP client
- `async_create_http_client(config)` — Create async HTTP client

### WebSocket Sessions (`eggsec.websocket`)

| Type | Description |
|------|-------------|
| `WebSocketSessionConfigPy` | Session configuration |
| `WebSocketMessagePy` | Received message (text/binary/ping/pong) |
| `WebSocketFramePy` | WebSocket frame details |
| `WebSocketCloseInfoPy` | Close event information |
| `WebSocketHandshakePy` | Handshake result |
| `WebSocketSessionPy` | Sync WebSocket session (context manager) with message batch receive |
| `AsyncWebSocketSessionPy` | Async WebSocket session (async context manager) |
| `WebSocketAssessmentConfigPy` | Assessment configuration |
| `WebSocketAssessmentResultPy` | Assessment result with findings |

**WebSocketMessagePy fields:** `is_text`, `is_binary`, `is_ping`, `is_pong`, `text_content`, `data`, `size`

**WebSocketSessionPy methods:** `recv_available(max_count)`, `transcript()`

**Functions:**
- `websocket_assess(url, timeout_ms=30000)` — Comprehensive WebSocket assessment
- `async_websocket_assess(url, timeout_ms=30000)` — Async version
