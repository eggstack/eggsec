# Findings & Reporting Guide

Findings represent individual security observations. Reports aggregate findings into exportable documents.

## Findings

### Creating a Finding

```python
finding = eggsec.Finding(
    title="Open port 8080",
    description="HTTP service detected on non-standard port",
    severity=eggsec.Severity.MEDIUM,
    location="http://10.0.0.1:8080",
)
```

### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `title` | `str` | *(required)* | Short finding title |
| `description` | `str` | `""` | Detailed description |
| `severity` | `Severity` | `INFO` | Severity level |
| `location` | `str` | `""` | Affected URL/asset |
| `evidence` | `list[Evidence]` | `[]` | Supporting evidence |
| `finding_type` | `str` | `""` | Category (e.g. "vulnerability") |
| `cve` | `str` | `""` | CVE identifier |
| `remediation` | `str` | `""` | Fix recommendation |
| `tags` | `list[str]` | `[]` | Classification tags |

### Severity Levels

| Value | Description |
|-------|-------------|
| `Severity.CRITICAL` | Immediate exploitation risk |
| `Severity.HIGH` | Significant security impact |
| `Severity.MEDIUM` | Moderate risk |
| `Severity.LOW` | Minor issue |
| `Severity.INFO` | Informational finding |

### Evidence

```python
finding = eggsec.Finding(
    title="Header injection",
    severity=eggsec.Severity.HIGH,
    evidence=[
        eggsec.Evidence(kind="header", value="X-Custom: injected"),
        eggsec.Evidence(kind="body", value="<script>alert(1)</script>"),
    ],
)
```

### Output Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `to_dict()` | `dict` | Python dictionary |
| `to_json()` | `str` | JSON string |
| `to_row()` | `dict` | Flat dict for DataFrame (evidence as semicolon-joined string) |

## FindingSet

A collection of findings with filtering and bulk export.

```python
finding_set = eggsec.FindingSet()
finding_set.add(finding1)
finding_set.add(finding2)

# Filter by severity
high = finding_set.by_severity(eggsec.Severity.HIGH)

# Bulk export
dicts = finding_set.to_dicts()
rows = finding_set.to_rows()
```

## Report

Aggregates findings from multiple sources into a structured document.

### Creating a Report

```python
report = eggsec.Report({
    "title": "Security Assessment: example.com",
    "target": "example.com",
    "operator": "security_team",
})
```

### Adding Findings

```python
# Individual findings
report.add_finding(finding)

# From a FindingSet
report.add_finding_set(finding_set)

# From scan results (converts open ports / endpoints / fingerprints to findings)
report.add_result(port_scan_result)
report.add_result(endpoint_scan_result)
report.add_result(fingerprint_result)
```

### Exporting

```python
# In-memory
json_str = report.to_json()
dict_data = report.to_dict()
rows = report.to_rows()

# To files
report.write_json("report.json")
report.write_markdown("report.md")
```

## Pandas Integration

Use `to_rows()` for easy DataFrame conversion:

```python
import pandas as pd

finding_set = eggsec.FindingSet()
finding_set.add(finding1)
finding_set.add(finding2)

df = pd.DataFrame(finding_set.to_rows())
print(df.groupby("severity").size())
```
