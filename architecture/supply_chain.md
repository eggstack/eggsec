# Supply Chain Module

## Purpose

Supply chain security analysis including SBOM generation (CycloneDX, SPDX formats), typosquatting detection, manifest discovery (Cargo.toml, package.json, go.mod, etc.), and configuration analysis of Dockerfiles and GitHub Actions workflows for misconfigurations.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `SupplyChainReport` | `supply_chain/mod.rs` | Aggregated supply chain analysis results |
| `SupplyChainFinding` | `supply_chain/mod.rs` | Supply chain security finding (`file_path: Option<String>`, `line: Option<u32>`) |
| `ManifestType` | `supply_chain/scanner.rs` | Enum: CargoToml, CargoLock, PackageJson, PackageLockJson, YarnLock, PnpmLockYaml, GoMod, GoSum, Dockerfile, GitHubActions |
| `DiscoveredManifest` | `supply_chain/scanner.rs` | Discovered manifest file (path, manifest_type, dependency_count) |
| `SupplyChainScanResult` | `supply_chain/scanner.rs` | Supply chain scan result (repo_path, manifests, findings, totals) |
| `SbomReport` | `supply_chain/sbom.rs` | SBOM generation results |
| `TyposquatReport` | `supply_chain/typosquat.rs` | Typosquatting detection results |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `SupplyChainReport`, `SupplyChainFinding` |
| `sbom.rs` | SBOM generation in CycloneDX and SPDX formats. `SbomGenerator` methods: `generate_from_cargo()`, `generate_from_npm()`, `generate_from_requirements()`, `export_cyclonedx()`, `export_spdx()` |
| `scanner.rs` | Manifest discovery and configuration analysis (Dockerfile, GitHub Actions). Feature-gated behind `sbom`. Public function `collect_package_names()` parses Cargo.toml, package.json, and requirements.txt at top level. |
| `typosquat.rs` | Typosquatting detection for package names |

## Scope

SBOM vulnerability lookup (CVE matching against generated SBOMs) is explicitly out of scope. The module generates SBOMs in CycloneDX and SPDX formats but does not perform vulnerability database lookups against them.
