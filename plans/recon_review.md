# Recon Architecture Review

**Document:** architecture/recon.md
**Reviewed:** 2026-05-31
**Accuracy:** Medium

## Verified Claims
- DNS (`dns_records.rs`), Subdomain Discovery (`subdomain.rs`), WHOIS (`whois.rs`), Geolocation (`geolocation.rs`) all exist
- Tech Detection (`techdetect.rs`), Content Analysis (`content.rs`), JavaScript Analysis (`js.rs`), Wayback Machine (`wayback.rs`), CORS Testing (`cors.rs`), API Schema (`api_schema.rs`) all exist
- CVE Lookup (`cve.rs`), Secret Detection (`secrets.rs`), Threat Intelligence (`threatintel.rs`) all exist
- Cloud Enumeration (`cloud/`), Container Discovery (`containers.rs`) exist
- Email Discovery (`email.rs`), Email Security (`email_security.rs`) exist
- Recon Runner (`runner.rs`) exists at `crates/slapper/src/recon/runner.rs`
- `run_full_recon()` function verified at `crates/slapper/src/recon/runner.rs:479`
- 17 modules in full recon pipeline verified via `FULL_RECON_PIPELINE_MODULES` at `crates/slapper/src/recon/mod.rs:350-368`
- Sequential dependencies: takeover after subdomain_enum, cve after tech_detection verified at `crates/slapper/src/recon/runner.rs:593,681-686`
- FullReconResult aggregates results with error tracking verified at `crates/slapper/src/recon/mod.rs:117-157`
- FxHashMap/FxHashSet usage across recon modules verified
- `rustc_hash::FxHashMap` and `FxHashSet` imports verified at `crates/slapper/src/recon/mod.rs:110`

## Discrepancies
- [ASN Lookup]: Documented as "ASN Lookup (`asn.rs`)" in the core capabilities section, but `asn.rs` exists in the filesystem yet is NOT registered as a `pub mod` in `mod.rs` and is listed as "intentionally detached" in the test at `crates/slapper/src/recon/mod.rs:412-423`. It's not part of the recon module's public API.
- [Parallel execution count]: Documented as "14 tasks via `tokio::join!`", but actual `tokio::join!` in `run_full_recon()` has 13 tasks (reverse_dns, geo_lookup, threat_intel, ssl, whois, subdomain_enum, dns_records, tech_detection, js_analysis, wayback_check, content_analysis, cors_check, email_security) at `crates/slapper/src/recon/runner.rs:531-570`. The cloud detection runs separately after the join.
- [FxHashMap count]: Documented as "55 total collections across 14 components", but actual count is 45 across the files checked. The discrepancy may be due to counting methods or additional files not checked. (priority: low)
- [Git Secrets]: Documented as feature-gated (`git-secrets`), verified at `crates/slapper/src/recon/mod.rs:91`
- [API Schema]: Documented as feature-gated, but no feature gate visible at `crates/slapper/src/recon/mod.rs:80` - it's always included. (priority: low)
- [Undocumented modules]: `asn.rs`, `cve_lookup.rs`, `dns_enhanced.rs`, `ftp_auth.rs`, `smtp_auth.rs`, `ssh_auth.rs`, `ssl_audit.rs` exist in the filesystem but are not part of the recon module's public API (listed as "intentionally detached" in tests). These are not documented in the architecture doc.

## Bugs Found
- None identified

## Improvement Opportunities
- [Parallel task count]: Update documentation to reflect 13 parallel tasks (cloud runs separately). (priority: low)
- [Detached modules]: Add a section documenting the intentionally detached modules (asn, cve_lookup, dns_enhanced, ftp_auth, smtp_auth, ssh_auth, ssl_audit) and their purposes. (priority: medium)
- [API Schema feature gate]: Verify if api_schema should be feature-gated as documented. (priority: low)

## Stale Items
- [FxHashMap count table]: The table listing FxHashMap/FxHashSet per component has some inaccuracies. Recommend re-verifying counts against actual codebase. (priority: low)
