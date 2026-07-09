# Passive Reconnaissance Guide

Passive reconnaissance gathers information about a target without sending intrusive traffic. These operations use DNS resolution, TLS inspection, and HTTP header analysis.

## DNS Enumeration

Enumerates DNS records for a domain.

### Using `Client`

```python
scope = eggsec.Scope.allow_hosts(["example.com"])
client = eggsec.Client(scope, mode="manual")
dns = client.recon_dns("example.com")
```

### Convenience Function

```python
dns = eggsec.recon_dns("example.com")
```

### Async

```python
future = eggsec.async_recon_dns("example.com")
for r in future:
    if r is not None:
        dns = r
```

### `DnsRecordSet`

| Field | Type | Description |
|-------|------|-------------|
| `domain` | `str` | Queried domain |
| `a_records` | `list[str]` | IPv4 addresses |
| `aaaa_records` | `list[str]` | IPv6 addresses |
| `cname_records` | `list[str]` | Canonical names |
| `mx_records` | `list[MxRecord]` | Mail exchange records |
| `txt_records` | `list[str]` | Text records |
| `ns_records` | `list[str]` | Name servers |
| `caa_records` | `list[str]` | Certificate authority records |
| `soa_record` | `SoaRecord \| None` | Start of authority |

### `MxRecord`

| Field | Type | Description |
|-------|------|-------------|
| `preference` | `int` | MX priority |
| `exchange` | `str` | Mail server hostname |

### `SoaRecord`

| Field | Type | Description |
|-------|------|-------------|
| `mname` | `str` | Primary name server |
| `rname` | `str` | Responsible person |
| `serial` | `int` | Serial number |
| `refresh` | `int` | Refresh interval (seconds) |
| `retry` | `int` | Retry interval (seconds) |
| `expire` | `int` | Expiration time (seconds) |
| `minimum` | `int` | Minimum TTL (seconds) |

## TLS Inspection

Analyzes TLS/SSL configuration for a host.

### Using `Client`

```python
tls = client.inspect_tls("example.com")
```

### Convenience Function

```python
tls = eggsec.inspect_tls("example.com")
```

### `TlsInspectionResult`

| Field | Type | Description |
|-------|------|-------------|
| `has_ssl` | `bool` | TLS available |
| `certificate` | `TlsCertificateInfo \| None` | Certificate details |
| `supported_versions` | `list[str]` | TLS versions (e.g. "TLSv1.3") |
| `cipher_suites` | `list[str]` | Supported cipher suites |
| `issues` | `list[SslIssue]` | Security issues found |

### `TlsCertificateInfo`

| Field | Type | Description |
|-------|------|-------------|
| `subject` | `str` | Certificate subject |
| `issuer` | `str` | Certificate authority |
| `valid_from` | `str` | Not valid before |
| `valid_until` | `str` | Not valid after |
| `serial_number` | `str` | Serial number |
| `signature_algorithm` | `str` | Signature algorithm |
| `public_key_algorithm` | `str` | Public key type |
| `key_size` | `int` | Key size in bits |
| `is_expired` | `bool` | Expired flag |
| `days_until_expiry` | `int` | Days until expiration |
| `subject_alternative_names` | `list[str]` | SANs |

### `SslIssue`

| Field | Type | Description |
|-------|------|-------------|
| `severity` | `str` | Issue severity |
| `code` | `str` | Issue code |
| `description` | `str` | Human-readable description |

## Technology Detection

Identifies web technologies from HTTP responses.

### Using `Client`

```python
tech = client.detect_technology("https://example.com")
```

### Convenience Function

```python
tech = eggsec.detect_technology("https://example.com")
```

### `TechDetectionResult`

| Field | Type | Description |
|-------|------|-------------|
| `url` | `str` | Scanned URL |
| `status_code` | `int` | HTTP status code |
| `tech_stack` | `TechStack` | Detected technologies |

### `TechStack`

| Field | Type | Description |
|-------|------|-------------|
| `servers` | `list[str]` | Web servers (e.g. "nginx") |
| `frameworks` | `list[str]` | Frameworks (e.g. "Django") |
| `languages` | `list[str]` | Programming languages |
| `databases` | `list[str]` | Database technologies |
| `cdns` | `list[str]` | CDN providers |
| `cms` | `list[str]` | Content management systems |
| `javascript` | `list[str]` | JavaScript libraries |
| `other` | `list[str]` | Other detected technologies |

## Combining with Reports

All recon results can be incorporated into reports:

```python
report = eggsec.Report({"title": "Recon Report"})

dns = eggsec.recon_dns("example.com")
for ip in dns.a_records:
    report.add_finding(eggsec.Finding(
        title=f"A record: {ip}",
        severity=eggsec.Severity.INFO,
    ))

tls = eggsec.inspect_tls("example.com")
if tls.has_ssl and tls.certificate.is_expired:
    report.add_finding(eggsec.Finding(
        title="Expired certificate",
        severity=eggsec.Severity.HIGH,
    ))

report.write_json("recon.json")
```
