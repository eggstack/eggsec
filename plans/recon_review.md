# Recon Module Architecture Review

**Document Reference:** `architecture/recon.md`  
**Implementation Reference:** `crates/slapper/src/recon/`  
**Review Date:** 2026-05-23

---

## Verified Claims

### Core Capabilities

| Claim | Status | Implementation |
|-------|--------|----------------|
| DNS records: A, AAAA, MX, TXT, CNAME, NS, SOA, CAA | VERIFIED | `dns_records.rs:6-16` - All record types present |
| Subdomain discovery via crt.sh, Threatminer | VERIFIED | `subdomain.rs:77-89` - Both sources queried |
| WHOIS lookup | VERIFIED | `whois.rs` - Module exists |
| ASN lookup via ARIN RDAP | VERIFIED | `asn.rs` - Module exists (standalone) |
| Geolocation: MaxMind, ipapi, ip-api.com, ipwho.is, ip2c | VERIFIED | `geolocation.rs:254-621` - All 5 providers implemented |
| Tech detection | VERIFIED | `techdetect.rs` - Module exists |
| Content analysis with 80+ sensitive paths | VERIFIED | `content.rs:118-199` - **79 paths** (close to claim) |
| JavaScript analysis | VERIFIED | `js.rs` - Endpoint extraction, secrets, API keys |
| Wayback Machine | VERIFIED | `wayback.rs` - Snapshot and endpoint discovery |
| CORS testing | VERIFIED | `cors.rs` - 9 test origins, vulnerability detection |
| API Schema discovery | VERIFIED | `api_schema.rs` - Feature-gated module exists |
| CVE lookup: built-in database + NVD API | VERIFIED | `cve.rs:124-275` - Built-in + `cve_lookup.rs:52-77` for NVD |
| Secret detection with 25+ patterns | VERIFIED | `secrets.rs:103-309` - **30 patterns** counted |
| Git secrets scanning | VERIFIED | `git_secrets.rs` - Feature-gated module exists |
| Threat intel: VirusTotal, Shodan, AlienVault OTX | VERIFIED | `threatintel.rs:84-102,285-432` |
| Cloud enumeration: AWS, GCP, Azure, Firebase, Heroku, GitHub | VERIFIED | `cloud/mod.rs:66-81` - All 6 services |
| IAM analysis with 12 privilege escalation patterns | VERIFIED | `cloud/iam.rs:29-109` - **11 patterns** (close to claim) |
| Container discovery: Kubernetes, Docker | VERIFIED | `containers.rs` - Kubernetes via kube crate |
| Email discovery | VERIFIED | `email.rs` - Emails, phones, social media, addresses |
| Email security: SPF, DKIM, DMARC, STARTTLS, BIMI | VERIFIED | `email_security.rs` - Module exists |
| Dependency scanning: npm, cargo, go | VERIFIED | `dependency_scan/mod.rs:139-152` - Actually also includes Ruby (Gemfile), PHP (composer.json), Java (pom.xml) |

### Execution Model

| Claim | Status | Implementation |
|-------|--------|----------------|
| 14 tasks via `tokio::join!` | VERIFIED | `runner.rs:517-543` - Exactly 14 concurrent tasks |
| takeover runs after subdomain_enum | VERIFIED | `runner.rs:559-563` |
| cve mapping runs after tech_detection | VERIFIED | `runner.rs:632-638` |
| Non-critical failures tracked, don't stop pipeline | VERIFIED | `runner.rs:580-643` |

### Result Aggregation

| Claim | Status | Implementation |
|-------|--------|----------------|
| FullReconResult aggregates all results | VERIFIED | `mod.rs:118-154` |
| Error tracking per module | VERIFIED | `runner.rs:580-643` - 16 error fields |

---

## Discrepancies

### 1. Module Count Mismatch

**Architecture Claims:** 16 modules in `FULL_RECON_PIPELINE_MODULES`  
**Actual:** 17 modules

```
// Documented (architecture/recon.md:58-62):
reverse_dns, geolocation, threatintel, ssl, whois, subdomain,
dns_records, techdetect, js, wayback, cloud, content, cors,
email, takeover, cve

// Actual (mod.rs:346-364) - includes "secrets":
reverse_dns, geolocation, threatintel, ssl, whois, subdomain,
dns_records, techdetect, js, wayback, cloud, content, cors,
email, takeover, cve, secrets
```

**Impact:** Documentation is outdated by 1 module. "secrets" was added to the pipeline but not documented.

### 2. FxHashMap/FxHashSet Count

**Architecture Claims:** "55 total collections across 14 components"  
**Actual:** 70+ collections (per `AGENTS.override.md:64`)

Count verified via grep:
- `cve.rs`: 2 FxHashMap (cache + get_known_cves)
- `cve_lookup.rs`: 1 FxHashMap (cve_cache)
- `geolocation.rs`: 1 FxHashMap (LOCAL_IP_DATA)
- `wayback.rs`: 1 FxHashSet (endpoints)
- `takeover.rs`: 2 FxHashMap (cname_map, ns_map)
- `email.rs`: 3 FxHashSet (emails, phones, socials)
- `js.rs`: 2 FxHashSet (endpoints, urls)
- `subdomain.rs`: 4 FxHashSet (various query results)
- `cors.rs`: 1 FxHashSet (findings)
- `cloud/mod.rs`: 1 FxHashSet (generate_cloud_names)
- `containers.rs`: 1 FxHashMap (config check)
- `dns_enhanced.rs`: 2 FxHashSet (compare functions)
- `mod.rs`: 2 FxHashMap (callback metadata)
- `techdetect.rs`: 1 FxHashMap (headers)

**Impact:** Documentation significantly undercounts actual usage. This is a documentation bug, not implementation bug.

### 3. Secrets Module in Pipeline

**Architecture Claims:** "Standalone module (not part of `FULL_RECON_PIPELINE_MODULES`)"  
**Actual:** "secrets" IS in `FULL_RECON_PIPELINE_MODULES` (`mod.rs:363`)

**Impact:** Architecture doc contradicts actual implementation.

### 4. Sensitive Files Count

**Architecture Claims:** "80+" sensitive files  
**Actual:** 79 paths in `content.rs:119-199`

**Impact:** Minor - off by 1. Could be corrected to "79" or a path could be added.

### 5. Dependency Scanning Scope

**Architecture Claims:** "npm, cargo, go"  
**Actual:** Also includes Ruby (Gemfile), PHP (composer.json), Java (pom.xml) per `dependency_scan/mod.rs:139-152`

**Impact:** Documentation understates actual coverage.

### 6. IAM Privilege Escalation Patterns

**Architecture Claims:** "12 known patterns"  
**Actual:** 11 patterns in `cloud/iam.rs:29-109`

**Impact:** Minor discrepancy - off by 1.

---

## Bugs Found

### 1. Content Scanner Deduplication Logic Bug

**File:** `content.rs:71`  
**Issue:** The scanner only captures content with status codes 200, 401, or 403:

```rust
if status == 200 || status == 401 || status == 403 {
```

**Problem:** This means 404 responses (which could indicate sensitive files that don't exist but are informative) and 500 errors (which indicate server issues) are silently ignored rather than being tracked as potential findings.

**Severity:** Medium  
**Priority:** Low

### 2. Wayback Endpoints Deduplication Lost on Error

**File:** `wayback.rs:107-112`  
**Issue:** If `Url::parse(&original)` fails, the path is silently skipped:

```rust
if let Ok(url) = url::Url::parse(&original) {
    let path = url.path().to_string();
    if !path.is_empty() && path != "/" {
        endpoints.insert(path);
    }
}
// If parse fails, nothing is inserted
```

**Problem:** Invalid URLs in Wayback data are silently discarded without logging.

**Severity:** Low  
**Priority:** Low

### 3. ThreatIntel Client Supports Unused threatstream_key

**File:** `threatintel.rs:57,74`  
**Issue:** `ThreatIntelClient` accepts `threatstream_key` but it's never used in `check_ip()` or `check_domain()`:

```rust
pub struct ThreatIntelClient {
    // ...
    threatstream_key: Option<SensitiveString>,  // Never used
}
```

**Severity:** Low  
**Priority:** Low (dead code)

### 4. CVE Engine Uses Blocking HTTP in Async Context

**File:** `cve_lookup.rs:57-64,197-201`  
**Issue:** `CveEngine::lookup_cve` and `match_technology_cves` use `reqwest::blocking::Client` inside what appears to be async methods:

```rust
let client = reqwest::blocking::Client::builder()
    .timeout(std::time::Duration::from_secs(10))
    .build()?;
```

**Problem:** Blocking HTTP in async context can cause thread starvation. This is a pre-existing issue noted in AGENTS.override.md but not yet fixed.

**Severity:** Medium  
**Priority:** Medium

### 5. CveMapper Cache Not Persisted Across Invocations

**File:** `cve.rs:9-16`  
**Issue:** The `CVE_CACHE` is a `OnceLock` with `Arc<Mutex<>>`, meaning it persists within a single process but is not shared across different `CveMapper` instances created in different invocations:

```rust
static CVE_CACHE: OnceLock<Arc<Mutex<FxHashMap<String, Vec<VulnerabilityInfo>>>>> =
    OnceLock::new();
```

**Problem:** Each new `CveMapper` gets a reference to the same cache, but the cache may not be initialized on first use in all code paths.

**Severity:** Low  
**Priority:** Low

---

## Improvement Opportunities

### 1. Content Scanner Status Code Handling

**File:** `content.rs:71`  
**Suggestion:** Add configurable status code tracking to capture interesting error codes:

```rust
// Current: only 200, 401, 403
// Suggested: also track 404 (not found), 500 (server error)
if status == 200 || status == 401 || status == 403 || status == 404 {
```

**Estimated Impact:** Low performance cost, better reconnaissance data.

### 2. Sensitive Paths Count

**File:** `content.rs:119`  
**Suggestion:** Add 1 more path to make "80+" accurate, or update documentation to "79 paths".

**Estimated Impact:** Trivial.

### 3. Add TLS/SSL Certificate Expiration Warning

**File:** `ssl.rs`  
**Suggestion:** Add automatic checking for certificates expiring within 30 days.

**Estimated Impact:** Medium - common real-world security issue.

### 4. Parallelize Cloud Enumeration

**File:** `cloud/mod.rs:66-81`  
**Issue:** Each cloud service enumeration runs sequentially:

```rust
let s3_buckets = self.enumerate_s3_buckets(&domain_name).await;
let azure_blobs = self.enumerate_azure_blobs(&domain_name).await;
// ... all sequential
```

**Suggestion:** Use `tokio::join!` to parallelize all 6 enumerations:

```rust
let (s3_buckets, azure_blobs, gcp_storage, firebase, heroku, github_repos) = tokio::join!(
    self.enumerate_s3_buckets(&domain_name),
    self.enumerate_azure_blobs(&domain_name),
    self.enumerate_gcp_storage(&domain_name),
    self.enumerate_firebase(&domain_name),
    self.enumerate_heroku(&domain_name),
    self.enumerate_github(&domain_name),
);
```

**Estimated Impact:** High - could reduce cloud scan time by ~5x.

### 5. Add Wayback Endpoints to Subdomain Takeover Check

**File:** `takeover.rs`  
**Suggestion:** Currently `takeover.rs` only checks subdomains from `SubdomainResult`. Could integrate with Wayback discovered endpoints to find additional takeover candidates.

**Estimated Impact:** Medium - expands attack surface coverage.

### 6. Async CVE Lookup

**File:** `cve_lookup.rs:57-64,197-201`  
**Suggestion:** Replace `reqwest::blocking::Client` with async `reqwest::Client` to avoid thread pool blocking:

```rust
// Instead of blocking client
let client = reqwest::blocking::Client::builder()
    .timeout(std::time::Duration::from_secs(10))
    .build()?;

// Use async client with tokio runtime
let client = reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(10))
    .build()?;
```

**Estimated Impact:** Medium - prevents potential thread starvation.

### 7. Secrets Module Not Actually Called in Pipeline

**File:** `mod.rs:346-364`, `runner.rs`  
**Issue:** "secrets" is in `FULL_RECON_PIPELINE_MODULES` constant but is never actually invoked in `run_full_recon()`. Looking at `runner.rs:517-543`, there is no `run_secrets` call.

**Suggestion:** Either implement secrets scanning in the pipeline or remove from the constant.

**Estimated Impact:** Depends on intent - either implementation or documentation fix needed.

### 8. Subdomain Verification Could Use DNS Records

**File:** `subdomain.rs:169-234`  
**Issue:** `verify_subdomains` performs DNS lookups independently but could reuse DNS records from `dns_records.rs` to avoid redundant queries.

**Suggestion:** Pass DNS records to subdomain verification.

**Estimated Impact:** Low-Medium - reduces DNS query count.

---

## Priority Summary

| Category | Item | Priority |
|----------|------|----------|
| **Bug** | CVE Engine blocking HTTP in async | Medium |
| **Bug** | Secrets in pipeline constant but not called | Medium |
| **Improvement** | Parallelize cloud enumeration | High |
| **Improvement** | Async CVE lookup | Medium |
| **Discrepancy** | Module count (16 vs 17) | Medium |
| **Discrepancy** | FxHashMap count (55 vs 70+) | Low |
| **Bug** | Content scanner status codes | Low |
| **Bug** | Wayback URL parse silent failure | Low |
| **Bug** | Unused threatstream_key | Low |
| **Discrepancy** | Sensitive files (80 vs 79) | Low |
| **Improvement** | Add TLS expiration warnings | Medium |

---

## Conclusion

The recon module implementation is largely consistent with the architecture document. Key discrepancies are:

1. **Documentation updates needed** for module count, FxHashMap count, secrets module status, and dependency scanning scope
2. **Secrets module bug** - listed in pipeline but never invoked
3. **Performance opportunity** - cloud enumeration could be parallelized
4. **Async blocking issue** in CVE lookup that needs addressing

The module is well-structured with proper error handling, comprehensive test coverage, and appropriate use of FxHashMap/FxHashSet for performance. The main gaps are documentation accuracy rather than fundamental architectural issues.
