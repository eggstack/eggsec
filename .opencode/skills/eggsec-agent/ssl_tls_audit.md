---
name: ssl_tls_audit
description: "TestSSL-like TLS/SSL security auditing with certificate analysis and grading"
triggers:
  - ssl audit
  - tls scan
  - tls audit
  - ssl scan
  - cipher strength
  - certificate grade
  - testssl
  - ssl certificate
  - tls configuration
  - weak cipher
metadata:
  category: recon
  tools: [recon, ssl_audit]
  scope: targets
---

## Overview

Eggsec provides TestSSL-like TLS/SSL security auditing capabilities through the `ssl_audit` module. This performs comprehensive security testing of TLS configurations including certificate analysis, protocol version checking, cipher suite evaluation, and vulnerability detection.

## Capabilities

- **Certificate Analysis**: Validate certificates, check expiration, CA chain, subject/issuer
- **Protocol Version Detection**: Detect TLS 1.0, 1.1, 1.2, 1.3 support
- **Cipher Suite Evaluation**: Check for weak ciphers, export ciphers, known vulnerable ciphers
- **Vulnerability Detection**: Detection of known TLS vulnerabilities (heartbleed, POODLE, etc.)
- **Security Grading**: Grade from A+ to F based on overall TLS security posture
- **Finding Documentation**: Each finding includes CVE IDs and remediation recommendations

## Key Types

```rust
// Main audit report
pub struct SslAuditReport {
    pub target: String,
    pub port: u16,
    pub checks: Vec<SslCheck>,
    pub overall_grade: SslGrade,
    pub findings: Vec<SslFinding>,
}

// Individual check result
pub struct SslCheck {
    pub name: String,
    pub description: String,
    pub passed: bool,
    pub severity: Severity,
    pub details: Option<String>,
}

// Security finding
pub struct SslFinding {
    pub title: String,
    pub severity: Severity,
    pub description: String,
    pub recommendation: String,
    pub cve_ids: Vec<String>,
}

// Grade enum: APlus, A, B, C, D, E, F
pub enum SslGrade { ... }
```

## Usage

### CLI Usage

```bash
# Run SSL/TLS audit on a target
eggsec ssl-audit example.com --port 443

# With specific timeout
eggsec ssl-audit example.com --port 443 --timeout 60

# Output in JSON format
eggsec ssl-audit example.com --port 443 --format json
```

### API Usage

```rust
use eggsec::recon::ssl_audit::{SslAuditor, SslGrade};

let auditor = SslAuditor::new()?;
let report = auditor.audit("example.com", 443).await?;

println!("Overall Grade: {}", report.overall_grade.as_str());
for finding in &report.findings {
    println!("[{:?}] {} - {}", finding.severity, finding.title, finding.description);
}
```

## Grading Scale

| Grade | Meaning |
|-------|---------|
| A+ | Excellent - Forward secret, strong ciphers, no known issues |
| A | Good - Minor configuration issues |
| B | Fair - Some weak ciphers or protocol issues |
| C | Poor - Multiple issues, vulnerable ciphers |
| D | Bad - Major vulnerabilities present |
| E | Very Bad - Critical issues |
| F | Fail - Fundamental security failures |

## Common Findings

- **Weak Cipher Suites**: Detected ciphers that are considered weak or export-grade
- **TLS 1.0/1.1 Enabled**: Older protocol versions with known vulnerabilities
- **Certificate Issues**: Expired, self-signed, or mismatched certificates
- **No Forward Secrecy**: Ciphers that don't provide forward secrecy
- **Known Vulnerabilities**: Heartbleed, POODLE, BEAST, FREAK, ROBOT, etc.

## Triggers

Keywords that activate this skill: `ssl audit`, `tls scan`, `cipher strength`, `certificate grade`, `testssl`, `weak cipher`, `tls configuration`
