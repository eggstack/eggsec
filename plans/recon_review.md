# Recon Module Architecture Review

## Executive Summary

Reviewed `architecture/recon.md` against implementation in `crates/slapper/src/recon/`. Overall the documentation is accurate and well-structured. Found several discrepancies between documented counts and actual implementation, plus a few bugs and improvement opportunities.

---

## Verified Claims

### Core Capabilities

| Claim | Status | Location |
|-------|--------|----------|
| DNS records (A, AAAA, MX, TXT, CNAME, NS, SOA, CAA) | Verified | `dns_records.rs:6-16` |
| Subdomain discovery via crt.sh, Threatminer | Verified | `subdomain.rs:100-167` |
| WHOIS extraction | Verified | `whois.rs` (standalone) |
| Geolocation (MaxMind, ipapi, ip-api.com, ipwho.is, ip2c) | Verified | `geolocation.rs:329-621` |
| Tech detection (CMS, frameworks, servers, CDNs) | Verified | `techdetect.rs` |
| Content scanning (sensitive files/directories) | Verified | `content.rs:118-200` |
| JavaScript analysis (endpoints, API keys, secrets) | Verified | `js.rs` |
| Wayback Machine integration | Verified | `wayback.rs:60-126` |
| CORS testing | Verified | `cors.rs` |
| API schema discovery (OpenAPI/GraphQL) | Verified | `api_schema.rs` |
| CVE lookup (built-in + NVD API) | Verified | `cve.rs:136-261` |
| Secret detection with regex patterns | Verified | `secrets.rs:103-309` |
| Git secrets scanning (feature-gated) | Verified | `git_secrets.rs` |
| Threat intelligence (VirusTotal, Shodan, AlienVault OTX) | Verified | `threatintel.rs` |
| Cloud enumeration (AWS, GCP, Azure, Firebase, Heroku, GitHub) | Verified | `cloud/mod.rs:54-72` |
| IAM analysis (privilege escalation patterns) | Verified | `cloud/iam.rs:29-109` |
| Container discovery (Kubernetes, Docker) | Verified | `containers.rs` |
| Email discovery (emails, phones, social media) | Verified | `email.rs` |
| Email security (SPF, DKIM, DMARC, STARTTLS, BIMI) | Verified | `email_security.rs:1-714` |
| Dependency scanning (npm, cargo, go) | Verified | `dependency_scan/mod.rs:137-216` |

### Pipeline Execution

| Claim | Status | Location |
|-------|--------|----------|
| `run_full_recon()` executes 16 modules | Verified | `runner.rs:502-543` |
| 14 tasks via `tokio::join!` | Verified | `runner.rs:517-543` |
| `takeover` runs after `subdomain_enum` | Verified | `runner.rs:560` |
| `cve` runs after `tech_detection` | Verified | `runner.rs:635` |
| Non-critical failures tracked, don't stop pipeline | Verified | `runner.rs:580-638` |

### Performance Optimizations (FxHashMap/FxHashSet Usage)

All FxHashMap/FxHashSet claims are verified:
- `CveMapper.cache` - `cve.rs:31`
- `CveEngine.cve_cache` - `cve_lookup.rs:33`
- `LOCAL_IP_DATA` - `geolocation.rs:27`
- `WaybackClient.endpoints` - `wayback.rs:86`
- `TakeoverDetector.cname_map`/`ns_map` - `takeover.rs:455-456`
- `EmailDiscoveryClient` collections - `email.rs:132,158,177`
- `JsAnalyzer` collections - `js.rs:229,290`
- `SubdomainEnumerator` collections - `subdomain.rs:74,118`
- `CorsAnalyzer.findings` - `cors.rs:43`
- `CloudScanner.generate_cloud_names` - `cloud/mod.rs:342`
- `ContainerScanner.check_container_config` - `containers.rs:251,309`
- `compare_dns_records` - `dns_enhanced.rs:250,255`

---

## Discrepancies

### 1. FxHashMap/FxHashSet Count Mismatch
**Priority: Medium**

**Claim (architecture/recon.md:83):**
> 55 total collections across 14 components

**Actual:**
Found 66 FxHashMap/FxHashSet matches across the recon module (grep count), distributed across more than 14 components.

**Files with additional collections not in the table:**
- `techdetect.rs` - 4 `FxHashMap` instances (lines 23, 60, 477, 511)
- `cve_lookup.rs` - 1 `FxHashMap` instance (line 40)
- `dns_enhanced.rs` - 2 `FxHashSet` instances (lines 250, 255)

**Impact:** Documentation understates the actual FxHashMap/FxHashSet usage. The optimization is still correctly applied, just the count is inaccurate.

**Recommendation:** Update the architecture document to reflect actual counts or remove specific numbers.

---

### 2. Secrets Regex Pattern Count
**Priority: Low**

**Claim (architecture/recon.md:27):**
> Secret Detection (`secrets.rs`): Detecting API keys, tokens, and credentials via 25+ regex patterns.

**Actual:**
Counting `SecretPattern` entries in `build_patterns()` (`secrets.rs:107-309`):
- 24 `SecretPattern` entries

**Impact:** Documentation claims 25+, but there are exactly 24.

**Recommendation:** Change "25+" to "24" in documentation, or add one more pattern.

---

### 3. IAM Privilege Escalation Pattern Count
**Priority: Low**

**Claim (architecture/recon.md:34):**
> IAM Analysis: Privilege escalation pattern detection with 12 known patterns.

**Actual:**
Counting `KNOWN_ESCALATION_PATTERNS` entries (`cloud/iam.rs:29-109`):
- 13 patterns defined

**Impact:** Documentation undercounts by one.

**Recommendation:** Change "12" to "13" in documentation.

---

### 4. Secrets Module Standalone Status
**Priority: Low**

**Claim (architecture/recon.md:27):**
> Secret Detection (`secrets.rs`): Detecting API keys, tokens, and credentials via 25+ regex patterns. **Standalone module (not part of `FULL_RECON_PIPELINE_MODULES`).**

**Actual:**
Verified - `secrets` is NOT in `FULL_RECON_PIPELINE_MODULES` (`mod.rs:346-363`). This is correct.

**Impact:** None - this is actually correctly documented.

---

## Bugs Found

### 1. `CveMapper` Requires Mutable Self for Cache Access
**Priority: High**

**Location:** `cve.rs:45-134`

**Issue:**
```rust
pub async fn map_cves(&mut self, tech_stack: &TechStack) -> Result<CveMapping>
```
The `CveMapper` struct has a `cache: FxHashMap` field (`cve.rs:31`) and `map_cves` requires `&mut self` to update the cache. However, `CveMapper::new()` returns `Result<Self>` and `map_cves` is async. In the public API (`cve.rs:348-350`):

```rust
pub async fn map_cves(tech_stack: &TechStack, nvd_api_key: Option<String>) -> Result<CveMapping> {
    let mut mapper = CveMapper::new(nvd_api_key)?;
    mapper.map_cves(tech_stack).await
}
```

**Problem:** Each call creates a new `CveMapper` instance, so the cache never persists across calls. This defeats the purpose of caching.

**Recommendation:** Either:
1. Make the cache a `LazyLock` static (compile-time constant not possible since it needs NVD API key context)
2. Use an `Arc<Mutex<FxHashMap<...>>>` for shared caching across instances
3. Document that caching only works within a single `CveMapper` instance

---

### 2. Dependency Scan Target File Count
**Priority: Low**

**Location:** `dependency_scan/mod.rs:139-152`

**Issue:**
The architecture document (`architecture/recon.md:46-48`) states:
> **npm**: Scanning package.json, package-lock.json, and yarn.lock files.
> **cargo**: Scanning Cargo.toml and Cargo.lock files.
> **go**: Scanning go.mod and go.sum files.

**Actual:**
The `find_manifests` function (`dependency_scan/mod.rs:137-187`) scans for 12 file types:
```rust
let targets = vec![
    "Cargo.toml", "Cargo.lock",           // cargo
    "package.json", "package-lock.json", "yarn.lock", "requirements.txt",  // npm (+ python)
    "go.mod", "go.sum",                    // go
    "Gemfile", "Gemfile.lock",             // ruby (NOT documented)
    "composer.json",                       // php (NOT documented)
    "pom.xml",                             // java/maven (NOT documented)
];
```

**Impact:** Documentation is incomplete. The module also handles Ruby (Gemfile), PHP (composer.json), and Java (pom.xml) dependencies.

**Recommendation:** Update documentation or remove unadvertised support.

---

### 3. `query_alexa` Is Stubbed
**Priority: Low**

**Location:** `subdomain.rs:141-145`

**Issue:**
```rust
#[allow(dead_code)]
/// Alexa ranking query - implementation incomplete, returns empty
async fn query_alexa(&self, _domain: &str) -> Result<FxHashSet<String>> {
    Ok(FxHashSet::default())
}
```

The `query_alexa` function is never called in `enumerate()`. If it were, it would always return empty results.

**Impact:** Minor - the function exists but is never used and always returns empty.

**Recommendation:** Either implement it or remove it.

---

## Improvement Opportunities

### 1. Consider Adding `secrets` to Pipeline
**Priority: Medium**

Currently `secrets` is a standalone module not in `FULL_RECON_PIPELINE_MODULES`. Since it uses regex scanning on response content, it could be valuable as part of the full pipeline.

---

### 2. Cache Persistence for CVE Mapper
**Priority: Medium**

As noted in Bugs section, the CVE cache doesn't persist across invocations. Consider using a module-level cache with `Arc<Mutex<>>` or a persistent cache file.

---

### 3. Cloud Module: `extract_target_from_url` Error Handling
**Priority: Low**

**Location:** `cloud/mod.rs:55`

```rust
let domain_name = extract_target_from_url(domain).unwrap_or_else(|| domain.to_string());
```

This silently falls back to the input if URL extraction fails. If a malformed URL is passed, it will use that directly. Consider logging a warning.

---

### 4. Geolocation Fallback Chain Could Be Documented
**Priority: Low**

The geolocation module tries 7 different providers in sequence (`geolocation.rs:245-278`). This fallback chain could be explicitly documented in the architecture.

---

### 5. Content Scanner Parallelism Could Be Optimized
**Priority: Low**

**Location:** `content.rs:48-116`

The `ContentScanner::scan()` creates a new task for each sensitive path (~80 paths). With `concurrency` semaphore limiting to `self.concurrency`, this works correctly but creates 80 futures regardless of concurrency limit.

Consider batching paths if concurrency is low.

---

## Priority Summary

| Finding | Priority | Type |
|---------|----------|------|
| CveMapper cache doesn't persist | High | Bug |
| FxHashMap count mismatch (55 vs actual) | Medium | Discrepancy |
| Secrets module not in pipeline | Medium | Improvement |
| Secrets regex count (24 vs 25+) | Low | Discrepancy |
| IAM pattern count (13 vs 12) | Low | Discrepancy |
| Dependency scan extra file types | Low | Discrepancy |
| query_alexa stubbed | Low | Bug |
| Cloud extract_target_from_url handling | Low | Improvement |
| Content scanner batching | Low | Improvement |

---

## Conclusion

The architecture document accurately describes the recon module's capabilities and design. The main issues are:

1. **CveMapper cache bug** - high priority, should be addressed
2. **Documentation count discrepancies** - low/medium priority, should be corrected for accuracy
3. **Missing features** - some documented features have additional undocumented capabilities (dependency scan handles Ruby/PHP/Java, IAM has 13 patterns not 12)

Overall the implementation matches the documented architecture well.
