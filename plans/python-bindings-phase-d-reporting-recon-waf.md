# Phase D Plan: Reporting, Passive Recon, and WAF Detection

## Objective

Extend the Python bindings from scanner primitives into usable defensive validation workflows by adding findings/reporting, passive recon, and WAF detection. This phase should let users compose scans and produce structured outputs suitable for Python pipelines, dashboards, CI artifacts, and regression testing.

This phase should avoid aggressive fuzzing, bypass, stress, and raw-packet features. The focus is structured observation, evidence, classification, and reporting.

## Dependencies

This phase assumes Phase C is complete:

- `Client` and `AsyncClient` exist.
- Sync and async port scanning work.
- Endpoint discovery works.
- Service fingerprinting works.
- Scope checks and DTO serialization are tested.
- Client lifecycle and cancellation behavior are documented.

## Public Python API additions

Add findings and report types:

```python
Finding
FindingSet
Evidence
Report
Severity
```

Add report construction:

```python
report = eggsec.Report()
report.add_result(port_scan_result)
report.add_result(endpoint_scan_result)
report.add_result(fingerprint_result)
report.write_json("eggsec-results.json")
report.write_markdown("eggsec-report.md")
```

Add passive recon methods:

```python
client.recon_dns("example.com")
client.inspect_tls("https://example.com")
client.detect_technology("https://example.com")
```

Add WAF detection:

```python
result = client.detect_waf("https://example.com")
print(result.vendor, result.confidence, result.evidence)
```

Add async equivalents where the underlying Rust API is async or network-bound:

```python
await client.recon_dns(...)
await client.inspect_tls(...)
await client.detect_technology(...)
await client.detect_waf(...)
```

## Reporting and findings design

The Python reporting API should be ergonomic but not heavyweight. It should not require pandas, rich, jinja, or other Python dependencies.

Every result object from earlier phases should be convertible into a report item. If direct conversion is not straightforward, add adapter methods:

```python
result.to_findings()
result.to_rows()
```

`to_rows()` should produce a list of dictionaries, suitable for optional downstream pandas usage:

```python
import pandas as pd

df = pd.DataFrame(result.to_rows())
```

Do not make pandas a dependency.

Report outputs to support in this phase:

- JSON
- newline-delimited JSON if easy
- Markdown

SARIF can be included if the Rust output layer already supports it cleanly. Otherwise defer SARIF to Phase E or F.

## DTO requirements

`Finding` should expose:

```python
id
title
severity
target
category
evidence
description
recommendation
metadata
to_dict()
to_json()
```

`Evidence` should expose:

```python
kind
value
source
confidence
metadata
```

`Report` should expose:

```python
findings
metadata
add_finding(...)
add_result(...)
to_dict()
to_json()
write_json(path)
write_markdown(path)
```

`Severity` should map to existing Eggsec severity values and should be stable in Python.

## Passive recon scope

Start with passive or low-impact recon operations only. Candidate functions:

- DNS resolution and record lookup
- TLS certificate inspection
- HTTP header and technology detection
- non-invasive version/banner classification where already exposed through scanner/fingerprinting

Do not add invasive crawling, brute force, active fuzzing, bypass generation, credential checks, or exploit-like behavior in this phase.

## WAF detection scope

Expose WAF detection as evidence-based classification.

Required result fields:

```python
url
vendor
product
confidence
evidence
status_code_observations
headers_observed
elapsed_ms
```

Only include fields that exist or can be cleanly derived. Avoid overclaiming product identity when evidence is weak. Preserve confidence rather than returning a boolean-only `protected` value.

WAF bypass and validation testing should be deferred to Phase F. This phase may expose detection and passive evidence only.

## Safety and enforcement

All recon and WAF operations must honor `Scope`.

Automation mode should not allow manual override paths.

The docs should distinguish:

- passive recon
- WAF detection
- WAF validation
- fuzzing/bypass testing
- load/stress testing

This distinction is important for both user expectations and future safety posture.

## Documentation

Add:

```text
docs/python/reports.md
docs/python/recon.md
docs/python/waf.md
examples/python/recon_report.py
examples/python/waf_detection.py
examples/python/scan_to_pandas.py
```

The WAF docs should show detection only and explicitly state that active validation/bypass profiles are later-phase APIs.

The reporting docs should show how to aggregate multiple results into one JSON/Markdown report.

## Tests

Add Python tests:

```text
test_report.py
test_findings.py
test_recon_dns.py
test_tls_inspection.py
test_waf_detection.py
test_rows_output.py
```

Use local fixtures where possible:

- local HTTP server with known headers
- local TLS fixture if feasible
- deterministic DNS tests only where stable; otherwise isolate with mockable Rust/Python fixture boundaries

For WAF detection, prefer deterministic fixture inputs or known synthetic header patterns instead of relying on external services.

## Validation commands

Run:

```bash
cargo check -p eggsec-python
cd crates/eggsec-python
maturin develop
pytest python/tests
python ../../examples/python/recon_report.py
python ../../examples/python/waf_detection.py
python ../../examples/python/scan_to_pandas.py
```

If a test requires network access, mark it separately and keep the default suite local/offline where possible.

## Acceptance criteria

Python users can aggregate scanner/fingerprint results into a report.

Reports can be serialized to dict, JSON, and Markdown.

`to_rows()` works for tabular downstream use without requiring pandas.

Passive recon APIs are scoped and tested.

WAF detection returns structured evidence and confidence.

WAF detection docs do not imply bypass/fuzz/stress capability in this phase.

Automation mode preserves strict enforcement behavior.

Examples demonstrate realistic Python composition of scanner, recon, WAF detection, and reporting.

## Out of scope

PyPI publishing, wheel CI, type stubs, full API docs, WAF bypass, fuzzing, load testing, stress testing, NSE, database modules, cloud modules, wireless, packet inspection, and daemon client bindings are out of scope for this phase.
