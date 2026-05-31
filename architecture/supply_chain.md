# Supply Chain Module

## Purpose

Supply chain security analysis including SBOM generation (CycloneDX, SPDX formats), typosquatting detection, and dependency vulnerability scanning.

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
| `scanner.rs` | Dependency vulnerability scanner |
| `typosquat.rs` | Typosquatting detection for package names |

## Implementation Status

Fully implemented. SBOM generation, dependency scanning, and typosquatting detection are all functional.
