# Recon Architecture Review
**Document:** architecture/recon.md
**Reviewed:** 2026-05-31
**Accuracy:** Medium
**Lines Reviewed:** 106

## Verified Claims
- DNS enumeration (A, AAAA, MX, TXT, CNAME, NS, SOA, CAA): Verified at `recon/dns_records.rs`
- Subdomain discovery via crt.sh, Threatminer: Verified at `recon/subdomain.rs`
- WHOIS lookup: Verified at `recon/whois.rs`
- ASN lookup via ARIN RDAP (standalone): Verified at `recon/asn.rs`
- Geolocation with MaxMind, ipapi, ip-api.com, ipwho.is, ip2c: Verified at `recon/geolocation.rs`
- Tech detection for CMS, frameworks, web servers, languages, CDNs: Verified at `recon/techdetect.rs`
- Content analysis for 80+ sensitive files: Verified at `recon/content.rs`
- JavaScript analysis for endpoints and secrets: Verified at `recon/js.rs`
- Wayback Machine historical URLs: Verified at `recon/wayback.rs`
- CORS testing: Verified at `recon/cors.rs`
- API schema discovery (feature-gated): Verified at `recon/api_schema.rs`
- CVE lookup with built-in database + NVD API: Verified at `recon/cve.rs`
- Secret detection with 25+ regex patterns: Verified at `recon/secrets.rs`
- Git secrets scanning (feature-gated): Verified at `recon/git_secrets.rs`
- Threat intelligence (VirusTotal, Shodan, AlienVault OTX): Verified at `recon/threatintel.rs`
- Cloud enumeration (AWS, GCP, Azure, Firebase, Heroku, GitHub): Verified at `recon/cloud/mod.rs`
- Container discovery (feature-gated): Verified at `recon/containers.rs`
- Email discovery: Verified at `recon/email.rs`
- Email security (SPF, DKIM, DMARC, STARTTLS, BIMI): Verified at `recon/email_security.rs`
- Full pipeline modules list (17 modules): Verified at `recon/mod.rs:350-368`
- Standalone modules (asn, cve_lookup, dns_enhanced, ftp_auth, smtp_auth, ssh_auth, ssl_audit): Verified at `recon/mod.rs:412-423`
- Parallel execution via tokio::join!: Verified at `recon/runner.rs:545-570`
- Sequential dependencies (takeover after subdomain, cve after tech_detection): Verified at `recon/runner.rs:592-690`
- FullReconResult aggregation: Verified at `recon/mod.rs:117-157`

## Discrepancies
- Document states "55 total collections across 14 components" for FxHashMap/FxHashSet - Actual count is 66+ per `recon/AGENTS.override.md:57`
- Document states "IAM Analysis: Privilege escalation pattern detection with 12 known patterns" - Actual is 13 patterns per `recon/AGENTS.override.md:44`
- Document lists 13 parallel tasks but tokio::join! actually has 13 tasks (reverse_dns, geolocation, threat_intel, ssl, whois, subdomain_enum, dns_records, techdetect, js_analysis, wayback_check, content_analysis, cors_check, email_security) - Verified correct at `recon/runner.rs:545-570`
- Document states cloud detection runs separately (feature-gated) - Verified correct at `recon/runner.rs:585-590`

## Bugs Found
- [Bug]: Document claims "55 total collections across 14 components" but actual count is 66+ per AGENTS.override.md (`recon/AGENTS.override.md:57`)

## Improvement Opportunities
- [Item]: Update FxHashMap/FxHashSet count from 55 to 66+ in the performance optimizations table (priority: high)
- [Item]: Update IAM pattern count from 12 to 13 (priority: medium)
- [Item]: Consider documenting the `ReconStep<T>` enum pattern for graceful degradation (`recon/runner.rs:18-35`) (priority: low)
- [Item]: Document the `set_stage()` function for progress tracking (`recon/runner.rs:704-707`) (priority: low)
- [Item]: Add note about `FULL_RECON_PIPELINE_MODULES` constant for programmatic access (`recon/mod.rs:350-368`) (priority: low)

## Stale Items
- [Item]: The performance optimizations table lists specific file locations - These should be verified periodically as code evolves (priority: low)
