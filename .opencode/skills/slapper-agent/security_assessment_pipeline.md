---
name: security_assessment_pipeline
description: "Complete security assessment pipeline combining recon, scanning, fuzzing, and reporting"
triggers:
  - pipeline
  - assessment
  - full scan
  - comprehensive
  - complete
  - report
  - workflow
metadata:
  category: pipeline
  tools: [pipeline]
  scope: targets
---

## Overview

The security pipeline runs comprehensive assessments by chaining multiple security tools in sequence: reconnaissance, port scanning, endpoint discovery, fuzzing, and reporting.

## Pipeline Stages

| Stage | Tools | Duration |
|-------|-------|----------|
| Recon | DNS, Subdomains, SSL | ~2 min |
| Scanning | Ports, Endpoints | ~5 min |
| Fuzzing | SQLi, XSS, etc. | ~15 min |
| Reporting | JSON, SARIF, HTML | ~1 min |

## Usage

### Full Pipeline

```bash
slapper pipeline --target https://example.com --output report.json
```

### Quick Assessment

```bash
slapper pipeline --target https://example.com --profile quick
```

### Web Focus

```bash
slapper pipeline --target https://example.com --profile web
```

### API Focus

```bash
slapper pipeline --target https://example.com --profile api
```

### Deep Scan

```bash
slapper pipeline --target https://example.com --profile deep
```

## Profiles

| Profile | Coverage | Duration |
|---------|----------|----------|
| quick | Fast checks, top findings | 5 min |
| web | Web-focused (endpoints, XSS, SQLi) | 15 min |
| api | API testing (GraphQL, OAuth) | 20 min |
| full | Comprehensive all checks | 45 min |
| vuln | Vulnerability-focused | 30 min |

## Output Formats

```bash
slapper pipeline --target https://example.com --format json --output report.json
slapper pipeline --target https://example.com --format html --output report.html
slapper pipeline --target https://example.com --format sarif --output results.sarif
slapper pipeline --target https://example.com --format junit --output results.xml
```

## Triggers

Keywords: pipeline, assessment, full, comprehensive, complete, workflow, report, scan, all, everything, audit, security assessment

## Best Practices

1. Start with quick profile to identify critical issues
2. Use full profile for periodic comprehensive assessments
3. Save reports for trend analysis over time
4. Use SARIF for integration with GitHub Security
5. Set up scheduled pipeline runs via agent