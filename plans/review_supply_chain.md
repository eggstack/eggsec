# Supply Chain Architecture Review
**Document:** architecture/supply_chain.md
**Reviewed:** 2026-05-31
**Accuracy:** High
**Lines Reviewed:** 27

## Verified Claims
- `SupplyChainReport` struct: Verified at `crates/slapper/src/supply_chain/mod.rs:13` with fields `project_path`, `sbom`, `typosquatting`, `total_packages`, `total_risks`, `findings`
- `SupplyChainFinding` struct: Verified at `crates/slapper/src/supply_chain/mod.rs:23` with fields `category`, `severity`, `title`, `description`, `recommendation`
- `SbomReport` struct: Verified at `crates/slapper/src/supply_chain/sbom.rs:7` with fields `format`, `project_name`, `version`, `generated_at`, `components`, `vulnerabilities`
- `TyposquatReport` struct: Verified at `crates/slapper/src/supply_chain/typosquat.rs:6` with fields `packages_checked`, `suspicious_packages`, `risk_level`
- SBOM generation in CycloneDX and SPDX formats: Verified - `export_cyclonedx()` at `sbom.rs:237` and `export_spdx()` at `sbom.rs:278`
- Typosquatting detection: Verified at `typosquat.rs:88` with Levenshtein distance-based detection
- Dependency vulnerability scanner: Verified at `scanner.rs` - but feature-gated behind `sbom` feature flag (`scanner.rs:67`), not a general dependency vulnerability scanner. It's a manifest discovery and Dockerfile/GitHub Actions analysis tool.
- All files present: `mod.rs`, `sbom.rs`, `scanner.rs`, `typosquat.rs` - verified

## Discrepancies
- **Scanner feature-gate**: Documented as always available. Actual: `scan_repo()` in `scanner.rs` is gated behind `#[cfg(feature = "sbom")]` (`scanner.rs:67`), while `SbomGenerator` and `TyposquatDetector` are always available.
- **Scanner purpose mischaracterized**: Documented as "Dependency vulnerability scanner". Actual: `scanner.rs` discovers manifest files (Cargo.toml, package.json, go.mod, Dockerfile, GitHub Actions workflows) and checks for Dockerfile misconfigurations and GitHub Actions security issues. It does not scan dependencies for known vulnerabilities.
- **`SbomReport` location**: Documented as `supply_chain/sbom.rs`. Correct but worth noting the struct is at `sbom.rs:7` with `SbomGenerator` at line 40.
- **`TyposquatReport` location**: Documented as `supply_chain/typosquat.rs`. Correct but the actual detector type is `TyposquatDetector` at `typosquat.rs:88`.

## Bugs Found
- None

## Improvement Opportunities
- Clarify that `scanner.rs` is a manifest discovery and configuration analysis tool, not a dependency vulnerability scanner
- Document `SbomGenerator` (`sbom.rs:40`) and `TyposquatDetector` (`typosquat.rs:88`) as key types
- Document `SbomFormat` enum (`sbom.rs:17`), `SbomComponent` (`sbom.rs:23`), `SbomVulnerability` (`sbom.rs:33`), `TyposquatFinding` (`typosquat.rs:13`), `TyposquatTechnique` (`typosquat.rs:23`), `TyposquatRiskLevel` (`typosquat.rs:34`)
- Note that `scan_repo()` requires the `sbom` feature flag

## Stale Items
- "Dependency vulnerability scanner" description for `scanner.rs` is stale - it's a manifest discovery and configuration analysis tool
