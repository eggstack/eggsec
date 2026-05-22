# Reconnaissance Module

The Reconnaissance module focuses on passive and active information gathering about a target before performing more intrusive scanning or fuzzing.

## Core Capabilities (`src/recon/`)

### Network & Infrastructure

- **DNS (`dns_records.rs`)**: Comprehensive DNS enumeration, including A, AAAA, MX, TXT, CNAME, NS, SOA, and CAA records.
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
- **API Schema (`api_schema.rs`)**: OpenAPI/Swagger and GraphQL schema discovery (feature-gated).

### Vulnerability Mapping

- **CVE Lookup (`cve.rs`)**: Mapping identified software versions to known CVEs using built-in database + NVD API.
- **Secret Detection (`secrets.rs`)**: Detecting API keys, tokens, and credentials via 25+ regex patterns.
- **Git Secrets (`git_secrets.rs`)**: Scanning git repositories for committed secrets (feature-gated).
- **Threat Intelligence (`threatintel.rs`)**: Checking targets against VirusTotal, Shodan, and AlienVault OTX.

### Cloud & Containers (`cloud/`, `containers.rs`)

- **Cloud Enumeration**: Identifying AWS, GCP, Azure, Firebase, Heroku, and GitHub hosting with service enumeration.
- **IAM Analysis**: Privilege escalation pattern detection with 12 known patterns.
- **Metadata Testing**: IMDSv1/v2 testing for AWS/GCP/Azure.
- **Storage Testing**: S3 bucket security tests.
- **Container Discovery**: Detecting Kubernetes and Docker environments (feature-gated).

### Email

- **Email Discovery (`email.rs`)**: Extracting email addresses, phone numbers, and social media from web content.
- **Email Security (`email_security.rs`)**: Analyzing SPF, DKIM, DMARC, STARTTLS, and BIMI records.

### Dependency Scanning (`dependency_scan/`)

- **npm**: Scanning package.json, package-lock.json, and yarn.lock files.
- **cargo**: Scanning Cargo.toml and Cargo.lock files.
- **go**: Scanning go.mod and go.sum files.

## Recon Runner (`runner.rs`)

The `runner.rs` file orchestrates all these recon tasks, running them in parallel via `tokio::join!` to maximize efficiency.

### Full Recon Pipeline Modules

`run_full_recon()` executes these 16 modules:

```
reverse_dns, geolocation, threatintel, ssl, whois, subdomain,
dns_records, techdetect, js, wayback, cloud, content, cors,
email, takeover, cve
```

### Execution Model

**Parallel Execution (14 tasks via `tokio::join!`)**:
```
reverse_dns, geolocation, threat_intel, ssl, whois, subdomain_enum,
dns_records, tech_detection, js_analysis, wayback_check, cloud_detection,
content_analysis, cors_check, email_discovery
```

**Sequential Dependencies**:
- `takeover` runs after `subdomain_enum` completes
- `cve` mapping runs after `tech_detection` completes

### Result Aggregation

`FullReconResult` aggregates all results with error tracking for each module. Non-critical failures are tracked but don't stop the pipeline.
