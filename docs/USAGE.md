# Usage Guide

This guide covers common CLI workflows. Use Eggsec only on systems you own or
have explicit authorization to test. Start with the [safety guide](SAFETY.md)
and a [scope file](scope.md). Build options are listed in [BUILD.md](BUILD.md).

## Preview before testing

`plan` shows the stages a scan would run without sending traffic:

```bash
eggsec plan --scope scope.toml --target https://staging.example.com
```

## Common commands

```bash
# Quick port scan and service fingerprinting
eggsec scan example.com --profile quick --scope scope.toml

# Endpoint discovery and web testing
eggsec scan https://staging.example.com --profile web --scope scope.toml

# Reconnaissance
eggsec recon example.com --scope scope.toml

# Test a URL parameter with selected payloads
eggsec fuzz "https://staging.example.com/search?q=test" -t sqli,xss --scope scope.toml

# Detect a WAF without running evasion tests
eggsec waf https://staging.example.com --detect-only --scope scope.toml
```

For individual operations, use `eggsec <command> --help`. Some commands require
optional build features. See [BUILD.md](BUILD.md) and the
[feature matrix](FEATURE_MATRIX.md).

## Scope file

Use an explicit allowlist for an assessment. This example allows one hostname,
excludes an administrative host, and limits the ports Eggsec may scan:

```toml
require_explicit_scope = true
allowed_ports = [80, 443, 8080, 8443]
excluded_ports = [22, 3389]

[[allowed_targets]]
pattern = "staging.example.com"

[[excluded_targets]]
pattern = "admin.staging.example.com"
```

Scope behavior and additional examples are documented in the
[scope guide](scope.md). Hostname patterns match the requested hostname. If a
hostname can resolve to addresses outside your intended boundary, use explicit
address exclusions. Run `eggsec scope-explain --help` for the syntax to inspect a target's decision.

### Localhost lab

Loopback is treated separately by scope policy, but some scanner paths also
require an explicit fixture opt-in. For an authorized local lab, use a scope
file that names the loopback target and run:

```bash
EGGSEC_ALLOW_LOOPBACK_FIXTURE=1 eggsec scan 127.0.0.1 \
  --profile quick --scope examples/scope-localhost.toml \
  --allow-private-resolution
```

This environment variable is a local fixture guard, not authorization. Do not
use it to test systems outside your approved scope.

## Output

Use `--json` for JSON output. Commands that support report formats can write a
file with `--format` and `-o`:

```bash
eggsec scan example.com --profile quick --scope scope.toml --json -o results.json
eggsec report convert results.json -f html -o report.html
eggsec report convert results.json -f sarif -o results.sarif
```

Supported report formats vary by command. Use `eggsec <command> --help` for the
available formats. See [reporting and evidence](REPORT_EVIDENCE_MODEL.md) for
format details.

## CI example

A GitHub Actions job can export SARIF for upload to code scanning. Store the
authorized target in a repository secret and provide a scope file in the
workflow:

```yaml
- name: Run Eggsec
  run: eggsec fuzz "$TARGET_URL" -t sqli,xss --scope scope.toml --format sarif -o results.sarif
  env:
    TARGET_URL: ${{ secrets.TARGET_URL }}

- name: Upload SARIF
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: results.sarif
```

For JUnit output in GitLab CI, declare the XML file as a JUnit report, not a
SAST report:

```yaml
script:
  - eggsec fuzz "$TARGET_URL" -t sqli,xss --scope scope.toml --format junit -o results.xml
artifacts:
  reports:
    junit: results.xml
```

## More guides

- [Pipeline profiles](PIPELINE.md)
- [Build features and platform requirements](BUILD.md), [feature matrix](FEATURE_MATRIX.md), and [platform guide](PLATFORM.md)
- [Authentication testing](AUTH_LAB.md)
- [Database assessment](DATABASE_PENTEST.md)
- [Mobile analysis](MOBILE.md)
- [Wireless testing](WIRELESS.md)
- [Web proxy](WEB_PROXY.md)
- [Safety and scope enforcement](SAFETY.md) and the [scope model](scope.md)
