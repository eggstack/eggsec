# Eggsec Security Documentation

This document provides security-related information for using Eggsec, including TLS verification, secure configuration, and responsible disclosure guidelines.

## Table of Contents

- [TLS Verification](#tls-verification)
- [The `--insecure` Flag](#the---insecure-flag)
- [Security Considerations](#security-considerations)
- [Scope and Authorization](#scope-and-authorization)
- [Credential Handling](#credential-handling)
- [Output Security](#output-security)
- [Responsible Disclosure](#responsible-disclosure)

## TLS Verification

### Default Behavior

By default, Eggsec performs full TLS certificate verification for all HTTPS connections. This includes:

- Validating certificate chains of trust
- Checking certificate expiration dates
- Verifying hostnames match the certificate Common Name (CN) or Subject Alternative Names (SANs)
- Rejecting self-signed certificates

### Configuration

TLS verification can be configured in `eggsec.toml`:

```toml
[http]
verify_tls = true          # Enable TLS verification (default)
timeout_secs = 30          # Request timeout in seconds
follow_redirects = true     # Follow HTTP redirects
max_redirects = 10          # Maximum number of redirects
```

### Feature: `insecure-tls`

The `insecure-tls` feature enables a `TlsClient` implementation that bypasses certificate verification. This is used internally by distributed cluster communication.

```bash
cargo build --release --features insecure-tls
```

> **Warning**: The `insecure-tls` feature should only be enabled for testing in isolated environments.

## The `--insecure` Flag

### Overview

The `--insecure` flag (`-k` shorthand) disables TLS certificate verification for HTTP requests. When enabled, the client accepts any certificate, including:

- Self-signed certificates
- Expired certificates
- Certificates with mismatched hostnames
- Certificates signed by untrusted Certificate Authorities

### Usage

```bash
# Skip TLS verification for a single scan
eggsec scan-endpoints https://localhost:8443 --insecure

# Skip TLS verification for fuzzing
eggsec fuzz https://dev-server.local/api -t sqli --insecure

# Skip TLS verification in the TUI
# The TUI carries an `insecure` option in its operation options
# (crates/eggsec-tui/src/app/options.rs). Note the TUI deliberately never
# emits `--insecure` when replaying a saved session as CLI argv.
```

### CLI Help

```
--insecure    Skip TLS certificate verification
```

### Security Implications

> **Warning**: Using `--insecure` exposes your connections to man-in-the-middle (MITM) attacks.

When `--insecure` is enabled, attackers on your network can:

1. **Intercept Traffic**: Read all data transmitted between Eggsec and the target server, including:
   - Session tokens and authentication cookies
   - API keys and credentials
   - Sensitive personal or business data

2. **Impersonate Servers**: Successfully pose as the target server without a valid certificate, allowing them to:
   - Inject malicious content into responses
   - Collect credentials entered by the user
   - Exfiltrate data from request bodies

3. **Bypass Authentication**: Capture and replay authentication tokens, potentially gaining unauthorized access to protected resources.

### When to Use `--insecure`

The `--insecure` flag is appropriate in the following scenarios:

| Scenario | Example |
|----------|---------|
| Local development servers | `https://localhost:8443` with self-signed cert |
| Staging environments behind SSL terminators | Load balancers that handle TLS |
| Air-gapped testing environments | Isolated lab networks |
| Testing with expired certificates | Legacy internal systems |
| Certificate issues unrelated to security | Misconfigured internal CAs |

### When NOT to Use `--insecure`

| Scenario | Risk |
|----------|------|
| Production systems | Exposes sensitive data to interception |
| Public internet scanning | Attackers can intercept your traffic |
| Untrusted networks | Coffee shops, airports, hotels, and other public networks |
| Real user credentials | Credentials could be captured |

### Best Practices

1. **Use Explicit Scope Files**: Define allowed targets in a scope file to prevent accidental scanning of unintended systems.

2. **Use Isolated Networks**: When testing with `--insecure`, ensure you're on a trusted, isolated network.

3. **Prefer Proper Certificates**: Whenever possible, install proper TLS certificates on target systems instead of bypassing verification.

4. **Log Usage**: If you must use `--insecure`, log when and why for security auditing.

5. **Avoid Credentials**: Don't transmit real credentials when using `--insecure` mode.

## Security Considerations

### General Guidelines

1. **Only Scan Authorized Targets**
   - Always obtain written permission before testing any system
   - Use scope files to define allowed targets
   - Respect the boundaries of your authorization

2. **Understand the Legal Landscape**
   - Laws vary by jurisdiction; consult legal counsel
   - Many countries have laws against unauthorized access (CFAA, GDPR, etc.)
   - Bug bounty programs have specific terms and conditions

3. **Minimize Impact**
   - Use rate limiting to avoid overwhelming targets
   - Schedule scans during low-traffic periods when appropriate
   - Monitor for unintended side effects

4. **Protect Scan Results**
   - Store results securely with appropriate access controls
   - Encrypt reports containing sensitive findings
   - Delete results when no longer needed

### Testing on Production Systems

> **Warning**: Testing on production systems carries inherent risks. Use with caution and only with explicit authorization.

If you must test on production systems:

1. Schedule during maintenance windows
2. Have rollback plans for any changes
3. Monitor for service disruptions
4. Have incident response contacts ready
5. Consider using read-only tests first

### Denial of Service Considerations

Some Eggsec modules can generate significant load:

| Module | Risk Level | Mitigation |
|--------|-----------|------------|
| Fuzzing | Medium | Scope-limited payload types; `--adaptive-rate` tunes throughput |
| Load Testing | High | Only use on systems you own |
| Stress Testing | Critical | Requires explicit authorization: `[execution_policy].allow_stress_testing = true` plus `allow_stress_test = true` in the scope file |
| Grammar Fuzzing | Low-Medium | CPU-intensive but low network impact |

### Information Disclosure

Eggsec is designed to discover information about target systems. Be aware that:

- Scan results may reveal sensitive system information
- Endpoint discovery can expose hidden or internal paths
- Technology detection reveals software versions
- CVE mappings may indicate exploitable vulnerabilities

## Scope and Authorization

### Scope Files

Use a scope file (passed via `--scope`) to define explicit authorization
boundaries. `load_scope()` (`crates/eggsec/src/config/loader.rs:59`) reads
this shape, and it fails closed when an explicit `--scope` path does not
exist:

```toml
require_explicit_scope = true
max_requests_per_second = 100

[[allowed_targets]]
pattern = "*.example.com"
description = "Authorized lab web application"

[[allowed_targets]]
cidr = "10.0.0.0/8"

[[excluded_targets]]
pattern = "internal.example.com"

excluded_ports = [22, 3389]
```

A named-but-missing scope file is an error rather than an empty
(allow-all) scope, so a typo in `--scope` cannot silently disable
enforcement.

Note the distinction: this is the **scope-file** format above. On the
protocol surfaces (REST/MCP/gRPC/tool/Python) the transport DTO is
`ScopeSpec` from `eggsec-tool-core`
(`crates/eggsec-tool-core/src/request.rs:213`) — `allowed_patterns`,
`excluded_patterns`, `allowed_ips`, `allow_subdomains` — which the engine
converts to its authoritative `Scope` via `eggsec::config::scope_from_spec`
(fail-closed). Effective authorization is the **intersection** of the
configured engine scope and the request spec: both must allow, either may
deny.

### Scope Enforcement

`EnforcementContext::evaluate()` is the mandatory pre-dispatch gate for
all surfaces: targets outside the loaded scope are refused before any
tool runs. CLI/TUI run a permissive profile where the operator may
override with `--allow-*` flags and a reason; REST/MCP/agent/CI run
strict profiles that fail closed.

### Verifying Scope

```bash
# Dry-run to verify scope application
eggsec scan example.com --scope scope.toml --dry-run

# Explain how the loaded scope classifies a target
eggsec scope-explain --scope scope.toml --target target.example.com
eggsec scope-explain --scope scope.toml --target 10.0.0.5 --json
```

There is no `eggsec scope test` subcommand — `scope-explain` is the
diagnostic surface for checking scope matching.

## Credential Handling

### SensitiveString

Eggsec uses `SensitiveString` for credential storage, which provides:

- Automatic zeroization on drop
- Constant-time equality comparison
- Redacted display in logs and output

### Best Practices

1. **Use Environment Variables**: Pass credentials via environment variables rather than command-line arguments (which may appear in process listings).

2. **Avoid Hardcoding**: Never hardcode credentials in configuration files committed to version control.

3. **Use Secret Management**: Integrate with secret management systems when available.

4. **Rotate Credentials**: Don't use production credentials for testing.

### API Key Handling

API keys for external services (recon APIs, notifications) are stored in:

```toml
[recon.apis.ipapi]
api_key = "your-api-key"

[notifications.webhooks]
secret = "webhook-secret"
```

Ensure these files have appropriate access controls (`chmod 600`).

## Output Security

### Default Output

By default, results go to stdout only. Persisted output is opt-in via the
`[output]` section in `eggsec.toml`:

```toml
[output]
save_results = true
results_dir = "./reports"
```

Diagnostics are emitted through `tracing`. The `logging-subscriber`
feature installs a subscriber (stderr by default, or a configured file
sink); there is no implicit `eggsec.log` and no implicit
`eggsec-results.json`.

### Securing Output Files

```bash
# Set restrictive permissions
chmod 600 ./reports/report.json

# Encrypt sensitive reports
gpg --encrypt ./reports/report.json
```

### SARIF Output for CI/CD

When integrating with CI/CD systems, SARIF files may contain sensitive information:

```bash
# Upload to secure storage
aws s3 cp results.sarif s3://secure-bucket/sarif/

# Use signed URLs for sharing
aws s3 presign s3://secure-bucket/sarif/results.sarif
```

### HTML/JSON Reports

Reports may contain:
- Target URLs and endpoints discovered
- Vulnerability findings with evidence
- Technology stack information
- Request/response samples

Store and transmit securely.

## Responsible Disclosure

### Disclosure Policy

> **Note**: This is a placeholder for responsible disclosure guidelines. Modify and implement according to your organization's policies.

If you discover vulnerabilities using Eggsec:

1. **Verify**: Confirm the vulnerability exists and document reproduction steps.

2. **Assess**: Evaluate the security impact and affected systems.

3. **Report**: Contact the appropriate party:
   - Internal security team for company systems
   - Designated security contact for bug bounty programs
   - System owner for third-party software

4. **Cooperate**: Provide additional information if requested.

5. **Allow Time**: Give reasonable time for remediation before public disclosure.

### Reporting Templates

#### Internal Report Template

```
Vulnerability Report
====================
Target: [URL/IP]
Tool Used: Eggsec
Date: [Discovery Date]
Severity: [Critical/High/Medium/Low]

Description:
[Detailed description of the vulnerability]

Reproduction Steps:
1. [Step 1]
2. [Step 2]
...

Impact:
[Security impact assessment]

Remediation Recommendation:
[Suggested fix]
```

#### External Disclosure Template

```
Security Advisory
=================
Vulnerable Product: [Product Name]
Affected Versions: [Version Range]
Fixed Version: [Version if known]
Severity: [CVSS Score if calculated]

Vulnerability Type: [e.g., SQL Injection, XSS, etc.]
CVE (if assigned): [CVE-XXXX-XXXXX]

Description:
[Technical description]

Proof of Concept:
[Code/script to reproduce]

Timeline:
- Discovery: [Date]
- Vendor Notification: [Date]
- Vendor Response: [Date]
- Fix Released: [Date]
- Public Disclosure: [Date]

References:
- [Vendor advisory link]
- [CVE database link]
- [Related vulnerabilities]
```

### Coordinated Disclosure

For critical vulnerabilities affecting many systems:

1. Notify affected vendors simultaneously when possible
2. Allow reasonable time for coordinated patches
3. Align public disclosure timing across vendors
4. Consider CVSS scoring and potential impact

---

## Security Contacts

> **Note**: Replace with your organization's security contact information.

For security concerns related to Eggsec itself:
- Email: security@example.com
-PGP Key: [Key fingerprint]
-HackerOne: [If applicable]

For vulnerability reports in systems you're authorized to test:
- Internal Security Team: security@yourcompany.com
- Bug Bounty Program: https://bugbounty.yourcompany.com
