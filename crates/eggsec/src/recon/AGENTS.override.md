# Recon Module Override

Specialized guidance for the reconnaissance module.

## Key Components

The recon module is organized as follows:

### Core Files (in src/recon/)
- `mod.rs` - Module root with `FullReconResult` struct and `FULL_RECON_PIPELINE_MODULES`
- `runner.rs` - Main orchestration via `run_full_recon()` with parallel task execution
- `spinner.rs` - Terminal progress indicator for recon operations

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
- `api_schema.rs` - OpenAPI/GraphQL schema discovery

### Subdomain Discovery
- `subdomain.rs` - Subdomain enumeration via crt.sh, Threatminer, DNS verification
- `wayback.rs` - Wayback Machine historical URL discovery
- `takeover.rs` - Subdomain takeover detection with 30 service fingerprints

### Security
- `cve.rs` - CVE mapping with built-in database + NVD API integration
- `secrets.rs` - Secret detection with 25 regex patterns (in pipeline, runs after content analysis)
- `ssl.rs` - SSL/TLS certificate analysis
- `threatintel.rs` - Threat intelligence (VirusTotal, Shodan, AlienVault OTX)
- `git_secrets.rs` - Git repository secret scanning (feature-gated)

### Cloud
- `cloud/mod.rs` - Cloud discovery (AWS, GCP, Azure, Firebase, Heroku, GitHub)
- `cloud/services.rs` - Cloud service enumeration (Lambda, API Gateway, CloudFront)
- `cloud/metadata.rs` - IMDSv1/v2 testing for AWS/GCP/Azure
- `cloud/iam.rs` - IAM privilege escalation pattern analysis (12 patterns in `KNOWN_ESCALATION_PATTERNS`)
- `cloud/storage_test.rs` - S3 bucket security tests

### Email
- `email.rs` - Email/phone/social media extraction
- `email_security.rs` - Email security (SPF, DKIM, DMARC, STARTTLS, BIMI)

### Containers
- `containers.rs` - Docker/Kubernetes security scanning (feature-gated on `container` feature)

## Performance Notes

- Use `rustc_hash::FxHashMap`/`FxHashSet` instead of `std::collections::HashMap`/`HashSet`
- Actual FxHashMap/FxHashSet count is ~55 lines across 14 files
- `CveMapper.cache` uses `FxHashMap` (cve.rs:31)
- `CveEngine.cve_cache` uses `FxHashMap` (cve_lookup.rs:33)
- `LOCAL_IP_DATA` in geolocation.rs uses `FxHashMap`
- `WaybackClient.endpoints` uses `FxHashSet` (wayback.rs:86)
- `TakeoverDetector.cname_map`/`ns_map` uses `FxHashMap` (takeover.rs:455-456)
- `EmailDiscoveryClient` methods use `FxHashSet` (email.rs:132,155,174)
- `JsAnalyzer` methods use `FxHashSet` (js.rs:229,287)
- `SubdomainEnumerator` methods use `FxHashSet` (subdomain.rs:74,112,158)
- `CorsAnalyzer.findings` uses `FxHashSet` (cors.rs:43)
- `CloudScanner.generate_cloud_names` uses `FxHashSet` (cloud/mod.rs:342)
- `ContainerScanner.check_container_config` uses `FxHashMap` (containers.rs:243)
- `compare_dns_records` uses `FxHashSet` (dns_enhanced.rs:247,252)
- `FullReconResult` callback metadata uses `FxHashMap` (mod.rs:221,253)

## Bug Fixes

- **secrets.rs:277-283** - Discord token regex was actually a Slack token pattern (`xox[baprs]-...`).
  Replaced with actual Discord bot token format detection.
- **cloud/storage_test.rs:129-133** - `check_s3_public_write` performed a destructive PUT request that
  actually wrote data to target buckets. Changed to OPTIONS request checking the `allow` header.
- **email_security.rs:644-661** - `test_starttls` only tested TCP connectivity, not actual STARTTLS.
  Now sends EHLO, reads the server greeting, and checks for `250-STARTTLS` in the response.
- **email_security.rs:611-612** - `check_starttls` only tested ports 25 and 587, never port 465.
  Added port 465 to the test list. `supports_smtps` was always false.
- **ssl.rs:68-72,76** - Non-443 ports used `http://` instead of `https://`, making SSL analysis
  meaningless for non-standard TLS ports. Now always uses `https://`.
- **cloud/metadata.rs:98** - `imdsv2_required` was set to `true` when the token endpoint responded,
  which meant "IMDSv2 is available" not "IMDSv2 is required". Logic corrected.
- **runner.rs:458** - `run_secrets_check` silently discarded file-read errors via `if let Ok`.
  Now logs with `tracing::debug!` on error.
- **runner.rs:486-488** - User's `dns_concurrency` setting was silently overridden to minimum 10.
  Now only enforces minimum of 1.
- **runner.rs:57** - `resolve_target` silently swallowed URL parse failures with `.ok()`.
  Now logs a warning when the target cannot be parsed as a URL.
- **cve.rs:296-299** - NVD API query did not URL-encode product names (e.g., "C++", ".NET").
  Now uses `urlencoding::encode()`.
- **containers.rs:131-148** - Liveness probe check was inverted: checked if probe was `Some` but had
  no method, instead of checking if probe was `None`. Fixed to `container.liveness_probe.is_none()`.
- **dns_records.rs:35-95** - SOA and CAA record types were declared in the struct but never queried.
  Added actual DNS lookups for `RecordType::SOA` and `RecordType::CAA`.
- **runner.rs:726** - Geo output skipped when only country OR city was present (required both).
  Now displays whichever is available.
- **runner.rs:792** - SSL section emitted empty "ssl\n" when no certificate and no issues.
  Now only emits when there's data to show.
- **runner.rs:825** - CORS output only printed `allows_origin` findings, dropping `is_vulnerable`
  findings (e.g., null origin reflection). Now prints vulnerable findings with `[VULN]` tag.
- **runner.rs:843** - Threat section emitted bare "threat\n" even when both ip_reputation and
  domain_reputation were None. Now only emits when there's data.
- **runner.rs** - Missing DNS records output in `print_recon_results_string`. Added full DNS
  record output (A, AAAA, NS, MX, TXT, SOA).
- **wayback.rs:91** - CDX API `output=json` response was parsed as plain CSV. JSON array brackets
  and surrounding quotes caused malformed timestamps/URLs. Now properly strips `["` and `"]`.
- **cors.rs:193** - Origin reflection with credentials not detected. A server that echoes an
  arbitrary Origin back with `Access-Control-Allow-Credentials: true` is now flagged.
- **subdomain.rs:127** - crt.sh www-stripping dropped the bare domain (e.g., `www.example.com`
  stripped to `example.com` then rejected by `ends_with(.domain)` check). Now keeps bare domain.
- **subdomain.rs:232** - CNAME-only subdomains filtered out (only checked ip_addresses, mx, txt).
  Added `has_cname` to the filter condition.
- **geolocation.rs:348** - MaxMind `lookup_maxmind` set both `isp` and `org` to the `is_anycast`
  boolean string ("Anycast"/"No"). Now sets `isp` to None (MaxMind City DB doesn't have ISP).
- **takeover.rs:416** - NXDOMAIN detection iterated `nxdomain_cnames` but always checked
  `cnames.first()`. Now checks if error contains the specific `nxdomain_cname` pattern.
- **content.rs:259** - `categorize_path` returned empty for many sensitive paths (id_rsa, .htaccess,
  backup, Gemfile, Pipfile, .DS_Store, graphql, etc.). Added categories for these paths.
- **runner.rs:461** - `run_secrets_check` passed web URLs (e.g., `https://example.com/.env`) to
  `scan_file()` which expects local filesystem paths. Now fetches URL content via HTTP and scans
  with `scan_content()`.
- **threatintel.rs:418** - Domain passive DNS used wrong JSON field `"address"` instead of
  `"hostname"` for AlienVault OTX, causing empty results for domain lookups.
- **email.rs:190-194** - LinkedIn social media URLs dropped `/in/` and `/company/` path components.
  Now reconstructs URLs with correct path prefixes.
- **takeover.rs** - Removed overly generic HTTP indicators ("Cloudflare", "ray ID", "wordpress.com",
  "surge.sh", "Site Not Found", "Intercom", "pingdom") that caused false positives.
- **js.rs:25-91** - Secret type labels contained raw regex patterns instead of human-readable names.
  Now uses labels like "API Key", "AWS Secret", "Bearer Token", etc.
- **dns_enhanced.rs:48,89** - Duplicate "ns1" entry in default wordlist removed.

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

(End file - 146 lines)