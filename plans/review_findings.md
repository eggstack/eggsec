# Findings Module Architecture Review

**Document:** architecture/findings.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 89

## Verified Claims

- Finding struct location at `findings/mod.rs`: Verified at `findings/mod.rs:252`
- Confidence enum 5 variants (Confirmed, High, Medium, Low, Informational): Verified at `findings/mod.rs:37-43`
- Confidence::score() mapping (1.0, 0.75, 0.5, 0.25, 0.0): Verified at `findings/mod.rs:47-55`
- Confidence::from_ratio() exists: Verified at `findings/mod.rs:58-70`
- EvidenceKind enum location at `findings/mod.rs`: Verified at `findings/mod.rs:88`
- Evidence struct: Verified at `findings/mod.rs:126-135`
- AffectedAsset struct: Verified at `findings/mod.rs:161-172`
- FindingLocation struct: Verified at `findings/mod.rs:176-191`
- Reproduction struct: Verified at `findings/mod.rs:195-202`
- FindingType enum 9 variants: Verified at `findings/mod.rs:207-217` (Vulnerability, Misconfiguration, InformationLeak, PolicyViolation, AssetDiscovery, ServiceDetection, WafDetection, FuzzResult, ScanResult)
- FindingSource struct: Verified at `findings/mod.rs:237-244`
- FindingStore JSONL-based storage: Verified at `findings/store.rs:8-16`
- FindingStatus 6 variants: Verified at `findings/lifecycle.rs:6-13` (New, Confirmed, AcceptedRisk, FalsePositive, Remediated, Reopened)
- StoredFinding struct: Verified at `findings/lifecycle.rs:29-36`
- StatusChange struct: Verified at `findings/lifecycle.rs:38-44`
- ScanRun struct: Verified at `findings/lifecycle.rs:72-81`
- Finding struct 19 fields at lines 252-291: Verified at `findings/mod.rs:253-291` (id, fingerprint, title, description, severity, confidence, finding_type, cwe, owasp, cve, affected_asset, location, evidence, reproduction, remediation, discovered_at, source, tags, metadata)
- Confidence divergence acknowledged: Document at lines 75-86 correctly identifies the three separate Confidence enums

## Discrepancies

- **EvidenceKind variant count**: Document at line 13 says "13 variants" but `findings/mod.rs:88-102` shows only 11 variants: HttpRequest, HttpResponse, Header, BodySnippet, Timing, Diff, Banner, DnsRecord, Certificate, PortState, Screenshot, FilePath, LogLine. Counting these: 13 names but I count 13 in the list... let me recount: 1.HttpRequest, 2.HttpResponse, 3.Header, 4.BodySnippet, 5.Timing, 6.Diff, 7.Banner, 8.DnsRecord, 9.Certificate, 10.PortState, 11.Screenshot, 12.FilePath, 13.LogLine. Actually it's 13 - the document is correct on count but the display name in the table at line 13 says "13 variants" which is accurate.

Wait, re-reading the document at line 13:
| `EvidenceKind` | `findings/mod.rs` | Category of evidence data (HTTP, Screenshot, Log, etc.) - 13 variants |

Let me count the actual enum variants at `findings/mod.rs:88-102`:
```rust
pub enum EvidenceKind {
    HttpRequest,
    HttpResponse,
    Header,
    BodySnippet,
    Timing,
    Diff,
    Banner,
    DnsRecord,
    Certificate,
    PortState,
    Screenshot,
    FilePath,
    LogLine,
}
```
That's 13 variants. The document is correct.

## Bugs Found

- **No bugs found**: The findings module implementation matches the documented schema. The canonical Finding struct is well-designed with all 19 documented fields.

## Improvement Opportunities

- **EvidenceKind display names incomplete**: The `EvidenceKind` enum implements `Display` at `findings/mod.rs:104-122` but the display names use underscores (e.g., "http_request" instead of "HTTP Request"). Consider adding human-readable display names if these are used in user-facing output.
- **Finding::compute_fingerprint() uses DefaultHasher**: At `findings/mod.rs:300-326`, the fingerprint computation uses `std::collections::hash_map::DefaultHasher`. For security-sensitive deduplication, consider using a cryptographically secure hash (e.g., SHA-256) instead of the default hasher which may be vulnerable to hash collision attacks.

## Stale Items

- **No stale items identified**: The document accurately reflects the current implementation state.

## Code Interrogation Findings

- **FindingStore lacks deduplication on store**: At `findings/store.rs:34-45`, `store_finding()` appends to the JSONL file without checking for duplicates. The document at line 20 says "FindingStore for JSONL-based persistent file storage (findings.jsonl) and deduplication" but the actual implementation does not deduplicate - it just appends. The `update_status()` method at line 71-94 uses fingerprint to find and update existing findings, but new findings are always appended. This may be intentional (keeping history) but conflicts with the "deduplication" claim in the table.
- **Fingerprint computation non-cryptographic**: The `compute_fingerprint()` method at `findings/mod.rs:299-326` uses `DefaultHasher` which is SipHash in Rust. For security findings where fingerprint stability across versions matters, a deterministic content hash (like SHA-256 of canonical JSON) would be more appropriate and would ensure fingerprints don't change if hasher implementation changes.
- **FindingStore update is case-sensitive**: At `findings/store.rs:81`, finding lookup uses exact string comparison on fingerprint. This is correct behavior but means fingerprints must be preserved exactly across scan runs for matching to work.
- **JSONL format limitations**: The FindingStore uses JSONL (JSON Lines) format which requires rewriting the entire file on updates (`findings/store.rs:138-145`). For large finding sets, this could be slow. Consider using an append-only log with a separate index file for production use cases.