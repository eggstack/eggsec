# Supply Chain Module Architecture Review

**Document:** architecture/supply_chain.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 27

## Verified Claims
- [SupplyChainReport]: Verified at `crates/slapper/src/supply_chain/mod.rs:13`
- [SupplyChainFinding]: Verified at `crates/slapper/src/supply_chain/mod.rs:23`
- [SbomReport]: Verified at `crates/slapper/src/supply_chain/sbom.rs:7`
- [TyposquatReport]: Verified at `crates/slapper/src/supply_chain/typosquat.rs:6`
- [Files: mod.rs, sbom.rs, scanner.rs, typosquat.rs]: Verified
- [scanner.rs feature-gated behind sbom]: Verified at `crates/slapper/src/supply_chain/scanner.rs:67`

## Discrepancies
- None significant.

## Bugs Found
- None found.

## Improvement Opportunities
- [SBOM generation limited to 3 ecosystems]: The SBOM generator (`sbom.rs`) only supports Cargo, npm, and Python (requirements.txt). It doesn't support Go modules, Ruby gems, Java Maven/Gradle, .NET NuGet, or other package managers. Consider expanding or documenting this limitation (priority: medium)
- [No actual vulnerability lookup in SBOM]: `SbomReport` has a `vulnerabilities: Vec<SbomVulnerability>` field but `generate_from_cargo()`, `generate_from_npm()`, etc. all return empty `vulnerabilities: Vec::new()`. There's no actual CVE lookup implemented (priority: high)
- [TyposquatDetector has hardcoded package list]: `crates/slapper/src/supply_chain/typosquat.rs:42-86` has a static list of 45 "well known packages". This list will become stale and should be updated from an external source (priority: medium)

## Stale Items
- None.

## Code Interrogation Findings
- [sbom.rs:export_spdx() uses env!("CARGO_PKG_VERSION")]: Line 289-290 uses the slapper crate version as the SBOM creator tool version. This is correct but should be noted that SBOMs will differ based on what version of slapper generated them.
- [scanner.rs:scan_repo() uses walkdir]: The repo scanning uses `walkdir::WalkDir` which could be slow on large repositories. There's no filtering by file size or depth limit.
- [SupplyChainFinding in scanner.rs is different from supply_chain/mod.rs]: `scanner.rs:46` defines its own `SupplyChainFinding` struct which is different from `supply_chain/mod.rs:23`. This is a type duplication that could cause confusion.