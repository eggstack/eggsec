# Security Policy

## Reporting Security Vulnerabilities

We take security vulnerabilities seriously. If you discover a security vulnerability in Slapper, please report it responsibly.

### How to Report

**Do NOT open a public GitHub issue for security vulnerabilities.**

Instead, please report vulnerabilities by:

1. **Email**: Send details to security@slapper-tool.org
2. **GitHub Security Advisory**: Use GitHub's private vulnerability reporting at https://github.com/slapper-tool/slapper/security/advisories/new

### What to Include

Please include the following information:

- **Description** of the vulnerability
- **Steps to reproduce** the issue
- **Affected versions**
- **Potential impact**
- **Suggested fix** (if available)

### Response Timeline

| Stage | Timeline |
|-------|----------|
| Initial Response | Within 48 hours |
| Vulnerability Confirmation | Within 7 days |
| Fix Development | Within 30 days (severity dependent) |
| Security Advisory Publication | After fix is released |

## Security Features

### Scope Controls

Slapper includes scope controls to prevent unauthorized testing:

- **Explicit scope requirement**: Can enforce that all targets must be explicitly authorized
- **CIDR-based rules**: Define allowed/excluded IP ranges
- **Domain patterns**: Use wildcards for domain matching
- **Port restrictions**: Limit which ports can be scanned

### Data Protection

- **Secrets handling**: API keys, tokens, and passwords are marked as sensitive
- **Output sanitization**: Logs and reports can redact sensitive values
- **No telemetry**: Slapper does not send any data to external services

### Network Safety

- **Rate limiting**: Built-in rate limiting to avoid overwhelming targets
- **Jitter support**: Random delays for stealth and rate limit avoidance
- **Connection limits**: Configurable concurrency limits

## Usage Guidelines

### Authorization

- **Only test systems you own** or have explicit written permission to test
- **Document authorization** in your scope file
- **Respect scope boundaries** - never test systems outside your authorized scope

### Responsible Disclosure

When vulnerabilities are found in third-party systems:

1. Document findings thoroughly
2. Report to the organization's security team or bug bounty program
3. Follow their disclosure timeline
4. Do not publicly disclose without permission

### Legal Considerations

- Unauthorized security testing may violate laws in your jurisdiction
- Consult legal counsel if uncertain about authorization
- Keep records of authorization for all testing activities

## Security Best Practices

### Configuration Security

```toml
# Always use scope controls
require_explicit_scope = true

# Limit concurrency to avoid DoS
[scan]
default_concurrency = 10
rate_limit_per_second = 50

# Protect sensitive configuration
[http]
# Don't hardcode credentials
# Use environment variables instead
```

### Credential Management

```bash
# Use environment variables for sensitive data
export SLAPPER_PROXY_AUTH="user:pass"
export SLAPPER_BEARER_TOKEN="secret-token"

# Never commit credentials to version control
# Add to .gitignore:
# .env
# *credentials*
# *.key
# *.pem
```

### Output Handling

```bash
# Be careful with output files containing sensitive data
# Use restrictive permissions:
chmod 600 scan-results.json

# Avoid logging sensitive data
./slapper scan target.com --log-level warn
```

## Known Dependency Vulnerabilities

All known vulnerabilities have been fixed in the latest release.

### Fixed Vulnerabilities
- **idna (RUSTSEC-2024-0421)** - Fixed by migrating to hickory-resolver
- **quinn-proto (RUSTSEC-2026-0037)** - Fixed by upgrading to reqwest 0.13 and quinn-proto 0.11.14
- **native-tls (yanked)** - Fixed by upgrading to 0.2.18
- **pyo3 (RUSTSEC-2025-0020)** - Fixed by upgrading to 0.24

### Unmaintained Dependencies (Warnings)
The following dependencies are unmaintained but still functional:
- `fxhash` - Hash algorithm (consider: rustc-hash)
- `number_prefix` - Number formatting (consider: humansize)
- `paste` - String concatenation (consider: concat-string)
- `lru` - LRU cache (will be fixed in ratatui update)

## Known Security Considerations

### Inherent Risks

As a security testing tool, Slapper inherently:

- Sends potentially malicious payloads to targets
- May trigger security alerts on target systems
- Could cause service disruption if used incorrectly

### Mitigation

- Always test in authorized environments first
- Use low concurrency and rate limiting
- Monitor target systems during testing
- Have rollback procedures ready

## Security Audits

We conduct regular security audits:

- **Code Review**: All changes reviewed by maintainers
- **Dependency Scanning**: Automated CVE scanning via `cargo audit`
- **Static Analysis**: Clippy lints and security-focused rules
- **Fuzzing**: Input fuzzing for parsers and network code

## Security Update Policy

- Security fixes are backported to the last 2 major versions
- Security advisories are published on GitHub and sent to users who opt-in
- Critical vulnerabilities are patched within 7 days

## Contact

For security-related questions or concerns:

- Email: security@slapper-tool.org
- PGP Key: https://slapper-tool.org/security.asc

## Acknowledgments

We thank all security researchers who have responsibly disclosed vulnerabilities.
