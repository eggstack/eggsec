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
eggsec.detect_waf(url: str) -> WafDetectionResult
```

Detect WAF by making an HTTP request to the target URL. Performs passive
detection only -- no bypass or validation testing.

| Parameter | Type | Description |
|---|---|---|
| `url` | `str` | Target URL to test (e.g. `"https://example.com"`). |

**Returns:** `WafDetectionResult`
**Raises:** `EnforcementError`, `NetworkError`

---

### `async_detect_waf`

```python
eggsec.async_detect_waf(url: str) -> PyFuture
```

Async version of `detect_waf`.

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
