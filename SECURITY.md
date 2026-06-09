# Security Policy

## Status

**This is a pre-1.0 project. Security support is best-effort.**

## Reporting Security Vulnerabilities

We take security vulnerabilities seriously. If you discover a security vulnerability in Eggsec, please report it responsibly.

### How to Report

**Do NOT open a public GitHub issue for security vulnerabilities.**

Instead, please report vulnerabilities by:

1. **GitHub Security Advisory**: Use GitHub's private vulnerability reporting at https://github.com/dbowm91/eggsec/security/advisories/new

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
| Initial Response | Best effort |
| Vulnerability Confirmation | Best effort |
| Fix Development | Depends on severity and maintainer availability |
| Security Advisory Publication | After fix is released |

## Authorized Use

- **Only test systems you own** or have explicit written permission to test
- **Document authorization** in your scope file
- **Respect scope boundaries** — never test systems outside your authorized scope
- Unauthorized security testing may violate laws in your jurisdiction

## Scope Controls

Eggsec includes scope controls to prevent unauthorized testing:

- **Explicit scope requirement**: Can enforce that all targets must be explicitly authorized
- **CIDR-based rules**: Define allowed/excluded IP ranges
- **Domain patterns**: Use wildcards for domain matching
- **Port restrictions**: Limit which ports can be scanned

## Data Protection

- **Secrets handling**: API keys, tokens, and passwords are marked as sensitive
- **Output sanitization**: Logs and reports can redact sensitive values
- **No telemetry**: Eggsec does not send any data to external services

## Credential Management

```bash
# Use environment variables for sensitive data
export EGGSEC_PROXY_AUTH="user:pass"
export EGGSEC_BEARER_TOKEN="secret-token"

# Never commit credentials to version control
# Add to .gitignore:
# .env
# *credentials*
# *.key
# *.pem
```

## Output Handling

```bash
# Be careful with output files containing sensitive data
# Use restrictive permissions:
chmod 600 scan-results.json

# Avoid logging sensitive data
./eggsec scan target.com --log-level warn
```

## Network Safety

- **Rate limiting**: Built-in rate limiting to avoid overwhelming targets
- **Jitter support**: Random delays for stealth and rate limit avoidance
- **Connection limits**: Configurable concurrency limits

```toml
# Always use scope controls
require_explicit_scope = true

# Limit concurrency to avoid DoS
[scan]
default_concurrency = 10
rate_limit_per_second = 50
```

## Usage Guidelines

- Only test systems you own or have explicit written permission to test
- Use low concurrency and rate limiting
- Monitor target systems during testing
- Have rollback procedures ready
- Keep records of authorization for all testing activities
- Consult legal counsel if uncertain about authorization

## Known Security Considerations

### Inherent Risks

As a security testing tool, Eggsec inherently:

- Sends potentially malicious payloads to targets
- May trigger security alerts on target systems
- Could cause service disruption if used incorrectly

### Mitigation

- Always test in authorized environments first
- Use low concurrency and rate limiting
- Monitor target systems during testing
- Have rollback procedures ready

## Acknowledgments

We thank all security researchers who have responsibly disclosed vulnerabilities.
