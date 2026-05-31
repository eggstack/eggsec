# Findings Schema Architecture Review

**Document:** architecture/findings.md
**Reviewed:** 2026-05-31
**Accuracy:** Medium
**Lines Reviewed:** 33

## Verified Claims

- **Finding type at findings/mod.rs (line 11)**: Confirmed — `Finding` struct defined at `findings/mod.rs:251-291`
- **Confidence type at findings/mod.rs (line 12)**: Confirmed — `Confidence` enum at `findings/mod.rs:37-43`
- **Confidence variants: Confirmed, High, Medium, Low, Informational (line 12)**: Confirmed at `findings/mod.rs:38-42`
- **EvidenceKind type at findings/mod.rs (line 13)**: Confirmed — `EvidenceKind` enum at `findings/mod.rs:88-102`
- **EvidenceKind includes HTTP, Screenshot, Log (line 13)**: Confirmed — `HttpRequest`, `HttpResponse`, `Screenshot`, `LogLine` variants at `findings/mod.rs:89,90,99,101`
- **Evidence type at findings/mod.rs (line 14)**: Confirmed — `Evidence` struct at `findings/mod.rs:126-135`
- **AffectedAsset type at findings/mod.rs (line 15)**: Confirmed — `AffectedAsset` struct at `findings/mod.rs:161-172`
- **FindingLocation type at findings/mod.rs (line 16)**: Confirmed — `FindingLocation` struct at `findings/mod.rs:176-191`
- **Reproduction type at findings/mod.rs (line 17)**: Confirmed — `Reproduction` struct at `findings/mod.rs:194-202`
- **FindingType type at findings/mod.rs (line 18)**: Confirmed — `FindingType` enum at `findings/mod.rs:207-217`
- **FindingSource type at findings/mod.rs (line 19)**: Confirmed — `FindingSource` struct at `findings/mod.rs:237-244`
- **FindingStore type at findings/store.rs (line 20)**: Confirmed — `FindingStore` struct at `findings/store.rs:8-10`
- **FindingLifecycle at findings/lifecycle.rs (line 21)**: PARTIALLY INCORRECT — there is no type named `FindingLifecycle`. The file defines `FindingStatus` enum (line 6), `StoredFinding` struct (line 30), `StatusChange` struct (line 39), and `ScanRun` struct (line 73). (`crates/slapper/src/findings/lifecycle.rs`)
- **Confidence::score() method (line 27)**: Confirmed at `findings/mod.rs:47-55`
- **Confidence::from_ratio() method (line 27)**: Confirmed at `findings/mod.rs:58-70`
- **mod.rs: Module root with all canonical types (line 27)**: Confirmed — all types listed in the table are defined in `findings/mod.rs`
- **store.rs: FindingStore for storage (line 28)**: Confirmed at `findings/store.rs:8-10`
- **lifecycle.rs: Finding lifecycle state machine (line 29)**: Confirmed — `FindingStatus` enum with `change_status()` method on `StoredFinding` at `findings/lifecycle.rs:58-68`
- **Implementation Status: Fully implemented (line 33)**: Confirmed — all types have implementations with tests
- **Module notes existing types not yet migrated (line 33)**: Confirmed — `findings/mod.rs:8-11` states: "Existing module-specific types (e.g. `tool::finding::Finding`, `output::agent::AgentFinding`, `workflow::finding::Finding`) are NOT migrated yet"

## Discrepancies

- **"FindingLifecycle" type does not exist (line 21)**: The document lists `FindingLifecycle` at `findings/lifecycle.rs` with description "Finding status transitions". No type with this name exists in the codebase. The lifecycle module actually defines:
  - `FindingStatus` enum (`lifecycle.rs:6-13`) — the status states
  - `StoredFinding` struct (`lifecycle.rs:30-36`) — a finding with lifecycle metadata
  - `StatusChange` struct (`lifecycle.rs:39-44`) — a status transition record
  - `ScanRun` struct (`lifecycle.rs:73-81`) — a scan run record
  
  The intended reference is likely `FindingStatus` or `StoredFinding`. (`crates/slapper/src/findings/lifecycle.rs`)
- **FindingStore described as "In-memory" (line 20)**: The document says "In-memory finding storage with deduplication". `FindingStore` is actually **file-based JSONL storage** (`findings/store.rs:20-21`: `self.base_dir.join("findings.jsonl")`), not in-memory. While it loads findings into memory for querying, the primary persistence is on-disk. (`crates/slapper/src/findings/store.rs:20-21,34-45`)
- **Finding struct fields not enumerated**: The document describes `Finding` as "Canonical finding record with fingerprint, severity, confidence, evidence" but doesn't list the actual struct fields. The `Finding` struct has 18 fields (`findings/mod.rs:252-291`): `id`, `fingerprint`, `title`, `description`, `severity`, `confidence`, `finding_type`, `cwe`, `owasp`, `cve`, `affected_asset`, `location`, `evidence`, `reproduction`, `remediation`, `discovered_at`, `source`, `tags`, `metadata`. (`crates/slapper/src/findings/mod.rs:252-291`)
- **Confidence variants differ from output module**: The findings module defines `Confidence` with 5 variants (`Confirmed`, `High`, `Medium`, `Low`, `Informational`) while the output module's `agent.rs` defines a separate `Confidence` with 4 variants (`Confirmed`, `Likely`, `Possible`, `Unlikely`). The doc correctly describes the findings module version but doesn't note this divergence. (`crates/slapper/src/findings/mod.rs:37-43`, `crates/slapper/src/output/agent.rs:6-13`)

## Bugs Found

- **FindingLifecycle type name is wrong (line 21)**: The type `FindingLifecycle` does not exist. This will confuse developers looking for this type. Recommendation: Replace with `FindingStatus` or `StoredFinding` and update the description accordingly. (`architecture/findings.md:21`)

## Improvement Opportunities

- **Enumerate Finding struct fields (priority: high)**: The document should list all 18 fields of the canonical `Finding` struct to serve as a complete reference. Currently only 4 fields are mentioned in the description. (`architecture/findings.md:11`, `crates/slapper/src/findings/mod.rs:252-291`)
- **Document FindingStatus variants (priority: high)**: The lifecycle module's `FindingStatus` enum has 6 states (`New`, `Confirmed`, `AcceptedRisk`, `FalsePositive`, `Remediated`, `Reopened`) that should be documented. (`crates/slapper/src/findings/lifecycle.rs:6-13`)
- **Note Confidence divergence between modules (priority: medium)**: Add a note that the findings module's `Confidence` enum (5 variants) differs from the output module's `Confidence` enum (4 variants with different names). This is a known schema divergence that should be tracked for future unification. (`crates/slapper/src/findings/mod.rs:37-43`, `crates/slapper/src/output/agent.rs:6-13`)
- **Correct FindingStore description (priority: medium)**: Change "In-memory finding storage" to "JSONL-based persistent finding storage" or "File-based finding storage with in-memory query support" to accurately describe the implementation. (`crates/slapper/src/findings/store.rs:8-10`)
- **Document EvidenceKind variants (priority: low)**: The doc says "(HTTP, Screenshot, Log, etc.)" but the enum has 13 variants. List all for completeness. (`crates/slapper/src/findings/mod.rs:88-102`)
- **Document FindingType variants (priority: low)**: The doc describes FindingType as "High-level classification" but doesn't list the 9 variants (`Vulnerability`, `Misconfiguration`, `InformationLeak`, `PolicyViolation`, `AssetDiscovery`, `ServiceDetection`, `WafDetection`, `FuzzResult`, `ScanResult`). (`crates/slapper/src/findings/mod.rs:207-217`)
- **Document compute_fingerprint() algorithm (priority: low)**: The `Finding::compute_fingerprint()` method at `findings/mod.rs:299-326` generates stable fingerprints using a hash of asset type, identifier, finding type, path, parameter, CWE, and title. This is important for deduplication but undocumented. (`crates/slapper/src/findings/mod.rs:299-326`)

## Stale Items

- **"Fully implemented" claim (line 33)**: While the canonical schema is fully defined, the doc itself notes that "existing module-specific types are not yet migrated." This means the schema is defined but not yet adopted across the codebase. The "Fully implemented" status should be qualified to clarify that the schema definition is complete but cross-module migration is pending.
- **Missing migration tracking**: The document doesn't track which modules have been migrated to the canonical schema or provide a migration roadmap. Given that at least 3 module-specific types exist (`tool::finding::Finding`, `output::agent::AgentFinding`, `workflow::finding::Finding`), this would be valuable context.
