# Reconnaissance Module

The Reconnaissance module focuses on passive and active information gathering about a target before performing more intrusive scanning or fuzzing.

## Core Capabilities (`src/recon/`)

### Network & Infrastructure

- **DNS (`dns_records.rs`)**: Comprehensive DNS enumeration, including A, AAAA, MX, TXT, CNAME, NS, SOA, and CAA records.
- **Reverse DNS (`reverse_dns.rs`)**: Reverse DNS lookup for IP addresses.
- **Subdomain Discovery (`subdomain.rs`)**: Finding subdomains using crt.sh, Threatminer, and DNS verification.
- **WHOIS (`whois.rs`)**: Extracting ownership and network registration information.
- **ASN Lookup (`asn.rs`)**: ASN lookup via ARIN RDAP (standalone, not in full pipeline).
- **Geolocation (`geolocation.rs`)**: Identifying the physical location of target IPs using MaxMind, ipapi, ip-api.com, ipwho.is, and ip2c.

### Web & Technology

- **Tech Detection (`techdetect.rs`)**: Identifying the software stack (CMS, frameworks, web servers, languages, CDNs) using signatures and HTTP headers.
- **Content Analysis (`content.rs`)**: Scanning for 80+ sensitive files and directories.
- **JavaScript Analysis (`js.rs`)**: Analyzing page content and JavaScript files for endpoints, API keys, and secrets.
- **Wayback Machine (`wayback.rs`)**: Retrieving historical URLs for a domain to find forgotten or retired endpoints.
- **CORS Testing (`cors.rs`)**: Identifying misconfigured Cross-Origin Resource Sharing policies.
- **API Schema (`api_schema.rs`)**: OpenAPI/Swagger and GraphQL schema discovery.

### Vulnerability Mapping

- **CVE Lookup (`cve.rs`)**: Mapping identified software versions to known CVEs using built-in database + NVD API.
- **Secret Detection (`secrets.rs`)**: Detecting API keys, tokens, and credentials via 25 regex patterns.
- **Git Secrets (`git_secrets.rs`)**: Scanning git repositories for committed secrets (feature-gated).
- **Threat Intelligence (`threatintel.rs`)**: Checking targets against VirusTotal, Shodan, and AlienVault OTX.

### Cloud & Containers (`cloud/`, `containers.rs`)

- **Cloud Enumeration**: Identifying AWS, GCP, Azure, Firebase, Heroku, and GitHub hosting with service enumeration.
- **IAM Analysis**: Privilege escalation pattern detection with 12 known patterns.
- **Metadata Testing**: IMDSv1/v2 testing for AWS/GCP/Azure.
- **Storage Testing**: S3 bucket security tests.
- **Container Discovery (`containers.rs`)**: Detecting Kubernetes and Docker environments (feature-gated on `container` feature).

### Email

- **Email Discovery (`email.rs`)**: Extracting email addresses, phone numbers, and social media from web content.
- **Email Security (`email_security.rs`)**: Analyzing SPF, DKIM, DMARC, STARTTLS, and BIMI records (standalone, not in full pipeline).

### Utilities

- **Progress Spinner (`spinner.rs`)**: Terminal progress indicator for recon operations.

### Module Entry Points (`mod.rs`)

- `run_cli(args, config)` - CLI entry point for recon. Starts a spinner, runs `run_full_recon()`, stops spinner, writes output.
- `run_cli_with_callback(args, config, callback)` - CLI entry point with callback for streaming findings (feature-gated: `tool-api`). Iterates CVE, technology, and takeover results, invoking the callback for each finding.
- `SpinnerGuard` - Manages the terminal spinner lifecycle. Starts a background thread that ticks the spinner until stopped.
- `write_recon_output(recon, args, has_spinner)` - Writes recon results to stdout or file (JSON or human-readable format).

### Standalone Modules (not in public API)

These modules exist in `src/recon/` but are **not** exported via `mod.rs` and are not part of the full recon pipeline. They are available for direct invocation or internal use only.

- **ASN Lookup (`asn.rs`)**: Standalone ASN lookup via ARIN RDAP (`AsnLookup`, `AsnInfo`, `IpRange`).
- **CVE Engine (`cve_lookup.rs`)**: Dedicated CVE lookup engine with caching (`CveEngine`, `CveEntry`, `CvssSeverity`, `TechnologyMatch`).
- **DNS Enhanced (`dns_enhanced.rs`)**: Enhanced DNS enumeration with wordlist-based discovery (`DnsEnumerator`, `DnsEnumResult`).
- **FTP Auth (`ftp_auth.rs`)**: FTP banner grabbing and authentication testing (`FtpAuthResult`, `FtpAuthAttempt`).
- **SMTP Auth (`smtp_auth.rs`)**: SMTP banner grabbing and authentication testing via LOGIN/PLAIN mechanisms (`SmtpAuthResult`, `SmtpAuthAttempt`).
- **SSH Auth (`ssh_auth.rs`)**: SSH banner grabbing and limited authentication probing (`SshAuthResult`, `SshAuthAttempt`).
- **SSL Audit (`ssl_audit.rs`)**: TestSSL-like TLS security auditing with certificate analysis, protocol checking, and cipher suite evaluation (`SslAuditReport`, `SslGrade`, `SslCheck`, `SslFinding`).

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `ReconStep<T>` | `recon/runner.rs:18-35` | Internal enum for step execution outcome |

### ReconStep Enum

`ReconStep<T>` is a three-variant enum used internally by `runner.rs` to track whether each recon step succeeded, failed, or was skipped:

```rust
enum ReconStep<T> {
    Skipped,
    Completed(T),
    Failed,
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `into_option()` | `fn into_option(self) -> Option<T>` | Returns `Some(T)` for `Completed`, `None` otherwise |
| `is_failed()` | `fn is_failed(&self) -> bool` | Returns `true` only for `Failed` |

Each `run_*` function in `runner.rs` returns `ReconStep<ModuleResult>`. The `run_full_recon()` orchestrator calls `.is_failed()` on each result to populate error strings on `FullReconResult`, and `.into_option()` to extract the successful values.

## Recon Runner (`runner.rs`)

The `runner.rs` file orchestrates all these recon tasks, running them in parallel via `tokio::join!` to maximize efficiency.

### Full Recon Pipeline Modules

`run_full_recon()` executes these 16 modules:

```
reverse_dns, geolocation, threatintel, ssl, whois, subdomain,
dns_records, techdetect, js, wayback, cloud, content, cors,
email, takeover, cve, secrets
```

The `FULL_RECON_PIPELINE_MODULES` constant in `mod.rs` lists the canonical module names.

### Execution Model

**Parallel Execution (14 tasks via `tokio::join!`)**:
```
reverse_dns, geolocation, threat_intel, ssl, whois, subdomain_enum,
dns_records, tech_detection, js_analysis, wayback_check,
content_analysis, cors_check, email_discovery
```
Cloud detection runs separately (feature-gated `#[cfg(feature = "cloud")]`).

**Sequential Dependencies** (run after the parallel block):
- `takeover` runs after `subdomain_enum` completes
- `cve` mapping runs after `tech_detection` completes
- `secrets` scanning runs after `content_analysis` completes

### Result Aggregation

`FullReconResult` aggregates all results with error tracking for each module. Non-critical failures are tracked but don't stop the pipeline.

## Performance Optimizations

The recon module uses `rustc_hash::FxHashMap` and `FxHashSet` instead of `std::collections` equivalents for improved performance (55 total collections across 14 components):

| Component | File | Type |
|-----------|------|------|
| `CveMapper.cache` | `cve.rs` | `FxHashMap` |
| `CveEngine.cve_cache` | `cve_lookup.rs` | `FxHashMap` |
| `LOCAL_IP_DATA` | `geolocation.rs` | `FxHashMap` |
| `WaybackClient.endpoints` | `wayback.rs` | `FxHashSet` |
| `TakeoverDetector.cname_map`/`ns_map` | `takeover.rs` | `FxHashMap` |
| `EmailDiscoveryClient` collections | `email.rs` | `FxHashSet` |
| `JsAnalyzer` collections | `js.rs` | `FxHashSet` |
| `SubdomainEnumerator` collections | `subdomain.rs` | `FxHashSet` |
| `CorsAnalyzer.findings` | `cors.rs` | `FxHashSet` |
| `CloudScanner.generate_cloud_names` | `cloud/mod.rs` | `FxHashSet` |
| `ContainerScanner.check_container_config` | `containers.rs` | `FxHashMap` |
| `compare_dns_records` | `dns_enhanced.rs` | `FxHashSet` |
| `FullReconResult` callback metadata | `mod.rs` | `FxHashMap` |
