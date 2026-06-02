# Supply Chain Module

## Purpose

Supply chain security analysis including SBOM generation (CycloneDX, SPDX formats), typosquatting detection, manifest discovery (Cargo.toml, package.json, go.mod, etc.), and configuration analysis of Dockerfiles and GitHub Actions workflows for misconfigurations.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `SupplyChainReport` | `supply_chain/mod.rs` | Aggregated supply chain analysis results |
| `SupplyChainFinding` | `supply_chain/mod.rs` | Supply chain security finding |
| `SbomReport` | `supply_chain/sbom.rs` | SBOM generation results |
| `TyposquatReport` | `supply_chain/typosquat.rs` | Typosquatting detection results |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `SupplyChainReport`, `SupplyChainFinding` |
| `sbom.rs` | SBOM generation in CycloneDX and SPDX formats |
| `scanner.rs` | Manifest discovery and configuration analysis (Dockerfile, GitHub Actions). Feature-gated behind `sbom`. |
| `typosquat.rs` | Typosquatting detection for package names |

## Scope

SBOM vulnerability lookup (CVE matching against generated SBOMs) is explicitly out of scope. The module generates SBOMs in CycloneDX and SPDX formats but does not perform vulnerability database lookups against them.
