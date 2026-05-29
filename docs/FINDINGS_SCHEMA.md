# Canonical Findings Schema

Slapper uses a canonical finding schema for consistent security result reporting.

## Finding Structure

Each finding contains:
- **id**: Unique identifier
- **fingerprint**: Stable hash for deduplication across scans
- **title**: Human-readable title
- **description**: Detailed description
- **severity**: Critical, High, Medium, Low, Informational
- **confidence**: Confirmed, High, Medium, Low, Informational
- **finding_type**: Vulnerability, Misconfiguration, InformationLeak, etc.
- **cwe**: CWE identifier (optional)
- **owasp**: OWASP category (optional)
- **cve**: CVE identifier (optional)
- **affected_asset**: Target information
- **location**: Where the issue was found
- **evidence**: Supporting evidence (redacted by default)
- **remediation**: Fix recommendations (optional)

## Fingerprinting

Findings generate stable fingerprints based on:
- Target/asset identifier
- Finding type
- Location path/parameter
- CWE or vulnerability class
- Normalized title

Timestamps and random IDs are NOT included in fingerprints.

## Redaction

Evidence containing secrets is automatically redacted:
- Bearer tokens → `[REDACTED]`
- API keys → `[REDACTED]`
- Private keys → `[REDACTED PRIVATE KEY]`
- Connection strings → `[REDACTED CONNECTION STRING]`

See `crates/slapper/src/findings/mod.rs` for the complete schema.
