# Sync API Reference

The Python bindings provide a synchronous API over the async Rust engine. The GIL is released during all network I/O, so other Python threads run freely while scans execute.

Phase C adds endpoint discovery, service fingerprinting, and an async client. Phase D adds findings/reporting, passive recon (DNS, TLS, tech detection), and WAF detection. See [Async API](async-api.md), [Endpoint Discovery](endpoint-discovery.md), [Service Fingerprinting](service-fingerprinting.md), [Reconnaissance](recon.md), [WAF Detection](waf.md), and [Findings & Reports](reports.md) for details.

## Functions

### `eggsec.scan_ports(target, ports, scope, *, concurrency=100, timeout_ms=5000)`

Convenience function for a single scoped port scan. Internally creates an ephemeral `Client`.

**Parameters:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `target` | `str` | *(required)* | Hostname or IP to scan |
| `ports` | `list[int]` | *(required)* | Port numbers to scan |
| `scope` | `Scope` | *(required)* | Authorization scope |
| `concurrency` | `int` | `100` | Max concurrent connections |
| `timeout_ms` | `int` | `5000` | Connection timeout (ms) |

**Returns:** `PortScanResult`

**Raises:**

- `EnforcementError` — target or a port is outside scope
- `ScanError` — scan failed (network error, DNS resolution, etc.)

```python
import eggsec

scope = eggsec.Scope.allow_hosts(["127.0.0.1"])
result = eggsec.scan_ports("127.0.0.1", [80, 443], scope)
```

### `eggsec.recon_dns(domain, scope) -> DnsRecordSet`

Enumerate DNS records for a domain. See [Reconnaissance](recon.md).

### `eggsec.inspect_tls(host, scope) -> TlsInspectionResult`

Inspect TLS/SSL configuration. See [Reconnaissance](recon.md).

### `eggsec.detect_technology(url, scope) -> TechDetectionResult`

Detect web technologies from HTTP headers. See [Reconnaissance](recon.md).

### `eggsec.detect_waf(url, scope) -> WafDetectionResult`

Detect Web Application Firewall protection. See [WAF Detection](waf.md).

### `eggsec.validate_waf(url, scope, *, bypass=False, test_type=None) -> WafDetectionResult`

Validate WAF bypass techniques against a target. Requires scope enforcement before dispatch. See [WAF Detection](waf.md).

**Parameters:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `url` | `str` | *(required)* | Target URL |
| `scope` | `Scope` | *(required)* | Authorization scope |
| `bypass` | `bool` | `False` | Enable bypass techniques |
| `test_type` | `str \| None` | `None` | Specific test type to run |

**Returns:** `WafDetectionResult`

**Raises:** `EnforcementError`, `ScanError`

### `eggsec.fuzz_http(url, scope, payload_type="all", *, method="GET", param=None, concurrency=10, timeout=30) -> FuzzResult`

Perform HTTP fuzzing against a target. Requires scope enforcement before dispatch.

**Parameters:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `url` | `str` | *(required)* | Target URL |
| `scope` | `Scope` | *(required)* | Authorization scope |
| `payload_type` | `str` | `"all"` | Payload category |
| `method` | `str` | `"GET"` | HTTP method |
| `param` | `str \| None` | `None` | Target parameter |
| `concurrency` | `int` | `10` | Max concurrent requests |
| `timeout` | `int` | `30` | Request timeout (seconds) |

**Returns:** `FuzzResult`

**Raises:** `EnforcementError`, `ScanError`

### `eggsec.load_test_http(url, total_requests, concurrency, timeout_secs, scope, *, method="GET") -> LoadTestResult`

Perform HTTP load testing against a target. Requires scope enforcement before dispatch.

**Parameters:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `url` | `str` | *(required)* | Target URL |
| `total_requests` | `int` | *(required)* | Total requests to send (must be > 0) |
| `concurrency` | `int` | *(required)* | Concurrent workers (must be > 0) |
| `timeout_secs` | `int` | *(required)* | Request timeout in seconds (must be > 0) |
| `scope` | `Scope` | *(required)* | Authorization scope |
| `method` | `str` | `"GET"` | HTTP method |

**Returns:** `LoadTestResult`

**Raises:** `EnforcementError`, `ScanError`, `ValueError` (if caps are violated)

## Classes

### `Client(scope, *, mode="manual", concurrency=100, timeout_ms=5000)`

Reusable scan client bound to a scope. Preferred for repeated scans.

**Constructor parameters:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `scope` | `Scope` | *(required)* | Authorization scope |
| `mode` | `str` | `"manual"` | `"manual"` or `"automation"` |
| `concurrency` | `int` | `100` | Default max concurrent connections |
| `timeout_ms` | `int` | `5000` | Default connection timeout (ms) |

**Raises:** `ValueError` — if `mode` is not `"manual"` or `"automation"`.

#### `Client.scan_ports(target, ports, *, concurrency=None, timeout_ms=None) -> PortScanResult`

Scan ports on a target. Keyword-only parameters override client defaults for this call.

**Raises:** `EnforcementError`, `ScanError`

#### `Client.scan_endpoints(config: EndpointScanConfig) -> EndpointScanResult`

Scan HTTP endpoints on a web server. The `base_url` host must be in scope.

**Raises:** `EnforcementError`, `ScanError`

#### `Client.fingerprint_services(host, ports, *, concurrency=None, timeout_ms=None) -> FingerprintScanResult`

Fingerprint services on open ports by analyzing banners and response patterns.

**Raises:** `EnforcementError`, `ScanError`

#### `Client.recon_dns(domain) -> DnsRecordSet`

Enumerate DNS records for a domain.

**Raises:** `EnforcementError`, `ScanError`

#### `Client.inspect_tls(host) -> TlsInspectionResult`

Inspect TLS/SSL configuration for a host.

**Raises:** `EnforcementError`, `ScanError`

#### `Client.detect_technology(url) -> TechDetectionResult`

Detect web technologies from HTTP response headers.

**Raises:** `EnforcementError`, `ScanError`

#### `Client.detect_waf(url) -> WafDetectionResult`

Detect Web Application Firewall protection.

**Raises:** `EnforcementError`, `ScanError`

#### `Client.validate_waf(url, *, bypass=False, test_type=None) -> WafDetectionResult`

Validate WAF bypass techniques. Uses the client's internal scope for enforcement.

**Raises:** `EnforcementError`, `ScanError`

#### `Client.fuzz_http(url, payload_type="all", *, method="GET", param=None, concurrency=10, timeout=30) -> FuzzResult`

Perform HTTP fuzzing. Uses the client's internal scope for enforcement.

**Raises:** `EnforcementError`, `ScanError`

#### `Client.load_test_http(url, total_requests, concurrency, timeout_secs, *, method="GET") -> LoadTestResult`

Perform HTTP load testing. Uses the client's internal scope for enforcement.

**Raises:** `EnforcementError`, `ScanError`, `ValueError`

#### `Client.scope -> Scope`

Read-only property. Returns the client's scope.

#### `Client.mode -> str`

Read-only property. Returns the client's execution mode.

```python
client = eggsec.Client(
    scope=eggsec.Scope.allow_hosts(["10.0.0.0/24"]),
    concurrency=200,
)
result = client.scan_ports("10.0.0.1", [22, 80])
```

#### Context Manager

```python
with eggsec.Client(scope) as client:
    result = client.scan_ports("10.0.0.1", [80, 443])
# Client resources released
```

### `Scope`

Frozen scope configuration. Created via static factory methods, not a constructor.

#### `Scope.allow_hosts(hosts: list[str]) -> Scope`

Allow specific hostnames or CIDRs.

```python
scope = eggsec.Scope.allow_hosts(["example.com", "10.0.0.0/8"])
```

#### `Scope.allow_cidrs(cidrs: list[str]) -> Scope`

Allow only CIDR ranges.

```python
scope = eggsec.Scope.allow_cidrs(["192.168.0.0/16", "10.0.0.0/8"])
```

#### `Scope.deny_all() -> Scope`

Deny all targets. Useful as a safety default.

#### `Scope.from_file(path: str) -> Scope`

Load scope from a TOML or YAML file. Raises `ScopeError` on parse failure.

#### `Scope.is_target_allowed(target: str) -> bool`

Check if a target is within scope.

#### `Scope.is_port_allowed(port: int) -> bool`

Check if a port is within scope.

### `PortScanResult`

Immutable scan result. Returned by `scan_ports()` and `Client.scan_ports()`.

| Field | Type | Description |
|-------|------|-------------|
| `target` | `str` | Scanned host |
| `open_ports` | `list[OpenPort]` | Open ports found |
| `scanned_ports` | `int` | Total ports attempted |
| `elapsed_ms` | `int` | Scan duration in ms |
| `stats` | `ScanStats` | Aggregate stats |

Methods:

- `to_dict() -> dict` — convert to a plain Python dictionary
- `to_json() -> str` — serialize to a JSON string

### `OpenPort`

| Field | Type | Description |
|-------|------|-------------|
| `port` | `int` | Port number |
| `protocol` | `str` | Always `"tcp"` (Phase B) |
| `service` | `str` | Service name (e.g. `"http"`, `"ssh"`) |
| `banner` | `str \| None` | Banner text, if captured |
| `confidence` | `float` | Service detection confidence (0.0–1.0) |

### `ScanStats`

| Field | Type | Description |
|-------|------|-------------|
| `ports_scanned` | `int` | Ports attempted |
| `total_open` | `int` | Open ports found |
| `elapsed_ms` | `int` | Duration in ms |

### `PortRange`

Helper for building port lists. Use static factory methods.

```python
ports = eggsec.PortRange.list([22, 80, 443])
ports = eggsec.PortRange.range(1, 1024)
ports = eggsec.PortRange.top_100()

# Access the list via the .ports property
client.scan_ports("10.0.0.1", ports.ports)
```

### `TimingPreset`

Scan timing profiles. Not yet wired to scan parameters in Phase B; provided for API completeness.

```python
preset = eggsec.TimingPreset.normal()
```

Available: `paranoid`, `sneaky`, `polite`, `normal`, `aggressive`, `insane`.

### `EndpointScanConfig`

Configuration for endpoint discovery scans. See [Endpoint Discovery](endpoint-discovery.md).

### `EndpointScanResult`

Result of an endpoint scan. See [Endpoint Discovery](endpoint-discovery.md).

### `EndpointFinding`

Individual endpoint finding. See [Endpoint Discovery](endpoint-discovery.md).

### `FingerprintScanResult`

Result of a service fingerprint scan. See [Service Fingerprinting](service-fingerprinting.md).

### `ServiceFingerprintResult`

Individual service fingerprint. See [Service Fingerprinting](service-fingerprinting.md).

### `FingerprintEvidence`

Evidence supporting a fingerprint detection.

### `FingerprintConfidence`

Confidence level for a fingerprint detection.

### `Severity`

Finding severity enum. Values: `CRITICAL`, `HIGH`, `MEDIUM`, `LOW`, `INFO`.

```python
eggsec.Severity.HIGH
eggsec.Severity.from_str("high")  # case-insensitive
```

### `Evidence`

Supporting evidence for a finding.

| Field | Type | Description |
|-------|------|-------------|
| `kind` | `str` | Evidence type (e.g. `"header"`, `"body"`) |
| `value` | `str` | Evidence content |

### `Finding`

Individual security finding. See [Findings & Reports](reports.md).

### `FindingSet`

Collection of findings with filtering and bulk export. See [Findings & Reports](reports.md).

### `Report`

Aggregated findings document. See [Findings & Reports](reports.md).

### `DnsRecordSet`

DNS enumeration result. See [Reconnaissance](recon.md).

### `MxRecord`

MX record entry. See [Reconnaissance](recon.md).

### `SoaRecord`

SOA record entry. See [Reconnaissance](recon.md).

### `TlsInspectionResult`

TLS inspection result. See [Reconnaissance](recon.md).

### `TlsCertificateInfo`

Certificate details. See [Reconnaissance](recon.md).

### `SslIssue`

TLS security issue. See [Reconnaissance](recon.md).

### `TechDetectionResult`

Technology detection result. See [Reconnaissance](recon.md).

### `TechStack`

Detected technologies. See [Reconnaissance](recon.md).

### `WafDetectionResult`

WAF detection result. See [WAF Detection](waf.md).

## Callbacks and Sinks

The callback system provides push-based notification during scan execution.
All sinks isolate errors -- a failing callback is logged and never
propagates to the caller.

### Using sinks with scans

```python
from eggsec import (
    Scope, Client, AuditSink, FindingSink,
    ArtifactSink, ProgressSink, EventConsumer,
)

def on_audit(event):
    print(f" audit: {event['operation_id']}")

def on_finding(finding):
    print(f" finding: {finding['title']}")

def on_progress(pct, msg):
    print(f" {pct:.0f}% {msg}")

client = Client(Scope.allow_hosts(["example.com"]))

# Sinks are wired into the scan pipeline
# (API varies by operation; consult individual tool docs)
```

### BackpressureChannel

For high-throughput scenarios, `BackpressureChannel` drops the oldest
event when the buffer is full:

```python
from eggsec import BackpressureChannel, EventEnvelope

channel = BackpressureChannel(capacity=256)
channel.send(event)
received = channel.try_recv()  # EventEnvelope or None
print(channel.total_dropped)   # events lost to backpressure
```

### CallbackScheduler

Queues callbacks with bounded capacity:

```python
from eggsec import CallbackScheduler

scheduler = CallbackScheduler(capacity=1000)
scheduler.enqueue(event)
events = scheduler.drain()
```

## Buffers and Paginated Results

### BinaryBuffer

Zero-copy binary buffer with PEP 3118 support:

```python
from eggsec import BinaryBuffer

buf = BinaryBuffer(b"\x00\x01\x02")
print(len(buf))           # 3
print(buf.hex())          # "000102"
mv = memoryview(buf)      # zero-copy memoryview
```

### LazyArtifact

Deferred file loading for large artifacts:

```python
from eggsec import LazyArtifact, ArtifactMeta

meta = ArtifactMeta("capture.pcap", "pcap", "application/octet-stream", 1024000)
artifact = LazyArtifact("/tmp/capture.pcap", meta)

# Metadata available without I/O
print(artifact.name(), artifact.size_bytes())

# Load on demand
data = artifact.load()  # returns BinaryBuffer
artifact.unload()       # free memory
```

### PaginatedResults

Page-based iteration:

```python
from eggsec import PaginatedResults

results = PaginatedResults(items, page_size=50)
print(results.total_pages())

# Iterate page by page
for page_num in range(results.total_pages()):
    page = results.get_page(page_num)
    process(page)

# Or iterate item by item
for item in results:
    process(item)
```

## Exception Hierarchy

```
EggsecError
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

All exceptions inherit from `EggsecError`, which inherits from Python's built-in `Exception`.
