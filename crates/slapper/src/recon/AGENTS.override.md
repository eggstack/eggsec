# Recon Module Override

Specialized guidance for the reconnaissance module.

## Key Components

The recon module is organized as follows:

### Core Files (in src/recon/)
- `mod.rs` - Module root with `FullReconResult` struct and `FULL_RECON_PIPELINE_MODULES`
- `runner.rs` - Main orchestration via `run_full_recon()` with parallel task execution

### Network/Infra
- `dns_records.rs` - DNS record enumeration (A, AAAA, MX, TXT, CNAME, NS, SOA, CAA)
- `reverse_dns.rs` - Reverse DNS lookup
- `whois.rs` - WHOIS information gathering
- `asn.rs` - ASN lookup via ARIN RDAP (detached - not in full pipeline)
- `geolocation.rs` - IP geolocation with MaxMind, ipapi, ip-api.com, ipwho.is, ip2c
- `dns_enhanced.rs` - Enhanced DNS (detached - not in full pipeline)

### Web Analysis
- `techdetect.rs` - Technology stack detection (servers, frameworks, CMS, languages, CDNs)
- `content.rs` - Content discovery for 80+ sensitive paths
- `js.rs` - JavaScript analysis for endpoints, secrets, API keys
- `cors.rs` - CORS misconfiguration detection
- `api_schema.rs` - OpenAPI/GraphQL schema discovery (feature-gated)

### Subdomain Discovery
- `subdomain.rs` - Subdomain enumeration via crt.sh, Threatminer, DNS verification
- `wayback.rs` - Wayback Machine historical URL discovery
- `takeover.rs` - Subdomain takeover detection with 20+ service fingerprints

### Security
- `cve.rs` - CVE mapping with built-in database + NVD API integration
- `secrets.rs` - Secret detection with 25+ regex patterns
- `ssl.rs` - SSL/TLS certificate analysis
- `threatintel.rs` - Threat intelligence (VirusTotal, Shodan, AlienVault OTX)
- `git_secrets.rs` - Git repository secret scanning (feature-gated)

### Cloud
- `cloud/mod.rs` - Cloud discovery (AWS, GCP, Azure, Firebase, Heroku, GitHub)
- `cloud/services.rs` - Cloud service enumeration (Lambda, API Gateway, CloudFront)
- `cloud/metadata.rs` - IMDSv1/v2 testing for AWS/GCP/Azure
- `cloud/iam.rs` - IAM privilege escalation pattern analysis
- `cloud/storage_test.rs` - S3 bucket security tests

### Email
- `email.rs` - Email/phone/social media extraction
- `email_security.rs` - Email security (SPF, DKIM, DMARC, STARTTLS, BIMI)

### Containers
- `containers.rs` - Docker/Kubernetes security scanning (feature-gated)

### Dependency Scanning
- `dependency_scan/mod.rs` - Unified interface
- `dependency_scan/npm/` - npm package scanning (package.json, package-lock.json, yarn.lock)
- `dependency_scan/cargo/` - Rust cargo scanning (Cargo.toml, Cargo.lock)
- `dependency_scan/go/` - Go module scanning (go.mod, go.sum)

## Performance Notes

- Use `rustc_hash::FxHashMap`/`FxHashSet` instead of `std::collections::HashMap`/`HashSet`
- `CveMapper.cache` uses `FxHashMap` (cve.rs:31)
- `LOCAL_IP_DATA` in geolocation.rs uses `FxHashMap`
- `WaybackClient.endpoints` uses `FxHashSet` (wayback.rs:86)
- `TakeoverDetector.cname_map`/`ns_map` uses `FxHashMap` (takeover.rs:455-456)
- `EmailDiscoveryClient` methods use `FxHashSet` (email.rs:132,155,174)
- `JsAnalyzer` methods use `FxHashSet` (js.rs:229,287)
- `SubdomainEnumerator` methods use `FxHashSet` (subdomain.rs:74,112,158)

## Error Handling Patterns

- `ReconStep<T>` enum (Skipped/Completed/Failed) for graceful degradation
- Never use `unwrap_or_default()` in async operations
- Use `tracing::warn!` for non-fatal failures

## Detached Modules (not in FULL_RECON_PIPELINE_MODULES)

These modules exist but are not part of `run_full_recon`:
- `asn.rs` - ASN lookup
- `cve_lookup.rs` - Standalone CVE lookup
- `dns_enhanced.rs` - Enhanced DNS enumeration
- `ftp_auth.rs` - FTP auth testing (detached)
- `smtp_auth.rs` - SMTP auth testing (detached)
- `ssh_auth.rs` - SSH auth testing (detached)
- `ssl_audit.rs` - SSL audit (detached)