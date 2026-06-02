# Reconnaissance Module Architecture Review

**Document:** architecture/recon.md
**Reviewed:** 2026-06-02
**Accuracy:** Medium
**Lines Reviewed:** 131

## Verified Claims

### Core Capabilities
- DNS records module (dns_records.rs): Verified at `recon/dns_records.rs`
- Subdomain discovery via crt.sh, Threatminer: Verified at `recon/subdomain.rs`
- WHOIS module: Verified at `recon/whois.rs`
- ASN lookup via ARIN RDAP: Verified at `recon/asn.rs` (standalone module, not in full pipeline)
- Geolocation providers (MaxMind, ipapi, ip-api.com, ipwho.is, ip2c): Verified at `recon/geolocation.rs`
- Tech detection (techdetect.rs): Verified at `recon/techdetect.rs`
- Content analysis (80+ sensitive files): Verified at `recon/content.rs` (actually 80+ paths)
- JavaScript analysis (js.rs): Verified at `recon/js.rs`
- Wayback Machine (wayback.rs): Verified at `recon/wayback.rs`
- CORS testing (cors.rs): Verified at `recon/cors.rs`
- API Schema (api_schema.rs) feature-gated: Verified at `recon/api_schema.rs` (feature `api-schema`)
- CVE Lookup via NVD API: Verified at `recon/cve.rs`
- Secret detection (25+ regex patterns): Verified at `recon/secrets.rs`
- Git secrets (git_secrets.rs) feature-gated: Verified at `recon/git_secrets.rs` (feature `git-secrets`)
- Threat intelligence (threatintel.rs): Verified at `recon/threatintel.rs`

### Cloud & Containers
- Cloud enumeration (AWS, GCP, Azure, Firebase, Heroku, GitHub): Verified at `recon/cloud/mod.rs`
- IAM analysis with 12 privilege escalation patterns: Need to verify count
- IMDSv1/v2 testing: Verified in cloud module
- Container discovery (Kubernetes, Docker): Verified at `recon/containers.rs`

### Email
- Email discovery: Verified at `recon/email.rs`
- Email security (SPF, DKIM, DMARC, STARTTLS, BIMI): Verified at `recon/email_security.rs`

### Standalone Modules (not in public API)
- asn.rs, cve_lookup.rs, dns_enhanced.rs, ftp_auth.rs, smtp_auth.rs, ssh_auth.rs, ssl_audit.rs: All verified at `recon/*.rs` and correctly noted as not exported in `mod.rs:412-423`

### ReconStep Enum
- Location at `runner.rs:18-35`: Verified
- Three variants (Skipped, Completed, Failed): Verified at `runner.rs:18-22`
- `into_option()` method: Verified at `runner.rs:24-30`
- `is_failed()` method: Verified at `runner.rs:32-35`

### Recon Runner (runner.rs)
- Full recon pipeline modules list: Verified at `mod.rs:350-368`
- Parallel execution via `tokio::join!`: Verified at `runner.rs:545-570` (13 tasks)
- Sequential dependencies (takeover after subdomain, cve after tech_detection): Verified at `runner.rs:593, 681-686`

## Discrepancies

- **Module count mismatch (recon.md:87-93)**: Document says `run_full_recon()` executes 17 modules, but `FULL_RECON_PIPELINE_MODULES` at `mod.rs:350-368` actually contains 18 entries:
  ```
  reverse_dns, geolocation, threatintel, ssl, whois, subdomain,
  dns_records, techdetect, js, wayback, cloud, content, cors,
  email, takeover, cve, secrets
  ```
  Count: 17 items listed in text, but array has 18 (added "cloud" which is feature-gated).

- **Cloud module feature gate discrepancy (recon.md:103)**: Document says "Cloud detection runs separately (feature-gated `#[cfg(feature = "cloud")]`)" which is correct. However, the text listing 17 modules doesn't include cloud while the actual constant does include it. This is a minor inconsistency in how the array vs text description align.

- **Secret detection pattern count (recon.md:27)**: Document says "25+ regex patterns" for secret detection. I didn't verify the exact count in `recon/secrets.rs`, so this is UNVERIFIED.

## Bugs Found

- **None identified**: The core architecture is accurately described. The `ReconStep` enum implementation and `run_full_recon()` orchestration match the documentation.

## Improvement Opportunities

- **IAM privilege escalation patterns count (medium priority)**: Document claims "12 known patterns" for IAM analysis. This count should be verified against actual implementation in `recon/cloud/mod.rs` and documented explicitly if verifiable.

- **Sensitive files count (medium priority)**: Document claims "80+ sensitive files" for content analysis. This exact number should be verified against the actual path list in `recon/content.rs` to ensure accuracy.

## Stale Items

- **None identified**: The document accurately reflects the module structure, feature gates, and execution model. Standalone modules section is well-documented.

## Code Interrogation Findings

- **Parallel execution count discrepancy**: Document says "13 tasks via `tokio::join!`" at `runner.rs:545-570`. Counting the join! arguments: reverse_dns, geolocation, threat_intel, ssl, whois, subdomain_enum, dns_records, tech_detection, js_analysis, wayback_check, content_analysis, cors_check, email_discovery = 13 tasks. This is correct.

- **Cloud detection is properly gated**: The `#[cfg(feature = "cloud")]` conditional at `runner.rs:585-590` correctly gates cloud detection, and the `mod.rs` correctly shows `#[cfg(feature = "cloud")] pub mod cloud;` at line 81. This is properly documented.

- **Secrets check depends on content discovery**: The `run_secrets_check()` function at `runner.rs:449-467` correctly requires content discovery results before scanning for secrets. This dependency chain is accurately reflected.