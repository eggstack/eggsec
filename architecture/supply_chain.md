# Supply Chain Module — Deep Dive

## Purpose

Supply chain security analysis including SBOM generation (CycloneDX and SPDX formats), typosquatting detection, manifest discovery across 10 ecosystem file types, and configuration analysis of Dockerfiles and GitHub Actions workflows for security misconfigurations.

## Location & Feature Gating

| Crate | Module path | Feature gate | lib.rs lines | Visibility |
|-------|-------------|-------------|--------------|------------|
| `eggsec` | `supply_chain/` | `sbom` | `lib.rs:146-148` | `pub mod` when enabled, `mod` (dead_code) when disabled |

The `sbom` feature is enabled via `sbom = ["cyclonedx-bom", "spdx", "walkdir"]`. (Note: it is *not* part of the `rest-api` feature set, and there is no `full-no-system` feature — see `crates/eggsec/Cargo.toml`.) The `scanner::scan_repo()` function is further gated behind `#[cfg(feature = "sbom")]` at `scanner.rs:82`.

## Files

| File | Lines | Purpose |
|------|-------|---------|
| `mod.rs` | 52 | Module root: `SupplyChainReport`, `SupplyChainFinding`, re-exports |
| `sbom.rs` | 790 | `SbomGenerator`: Cargo/npm/requirements.txt parsing, CycloneDX and SPDX exporters |
| `scanner.rs` | 767 | `ManifestType` enum, `scan_repo()` directory walker, Dockerfile/GitHub Actions analysis, `collect_package_names()` |
| `typosquat.rs` | 353 | `TyposquatDetector`: Levenshtein distance, 42-package known list, technique classification |

**Total: 4 files, 1,962 lines (including tests).**

## Architecture

### Type Inventory

| Type | File:line | Fields / Variants | Role |
|------|-----------|-------------------|------|
| `SupplyChainReport` | `mod.rs:12` | `project_path`, `sbom: Option<SbomReport>`, `typosquatting: Option<TyposquatReport>`, `total_packages`, `total_risks`, `findings` | Top-level aggregate |
| `SupplyChainFinding` | `mod.rs:22` | `category`, `severity`, `title`, `description`, `recommendation`, `file_path: Option<String>`, `line: Option<u32>` | Configuration finding |
| `SbomReport` | `sbom.rs:7` | `format: SbomFormat`, `project_name`, `version`, `generated_at`, `components`, `vulnerabilities` | SBOM output |
| `SbomFormat` | `sbom.rs:17` | `CycloneDx`, `Spdx` | Output format enum |
| `SbomComponent` | `sbom.rs:22` | `name`, `version`, `ecosystem`, `purl`, `licenses`, `is_direct` | Dependency entry |
| `SbomVulnerability` | `sbom.rs:32` | `component`, `cve_id`, `severity`, `description` | CVE reference (populated by external consumers) |
| `SbomGenerator` | `sbom.rs:40` | Unit struct (stateless) | SBOM generation facade |
| `ManifestType` | `scanner.rs:31` | 10 variants: `CargoToml`, `CargoLock`, `PackageJson`, `PackageLockJson`, `YarnLock`, `PnpmLockYaml`, `GoMod`, `GoSum`, `Dockerfile`, `GitHubActions` | File type classification |
| `DiscoveredManifest` | `scanner.rs:40` | `path`, `manifest_type`, `dependency_count: Option<usize>` | Discovery result |
| `SupplyChainScanResult` | `scanner.rs:48` | `repo_path`, `manifests`, `findings`, `dockerfile_found`, `github_actions_found`, `total_dependencies` | Scan output |
| `TyposquatReport` | `typosquat.rs:6` | `packages_checked`, `suspicious_packages`, `risk_level` | Detection output |
| `TyposquatFinding` | `typosquat.rs:12` | `package_name`, `suspected_target`, `similarity_score`, `techniques`, `severity`, `recommendation` | Single finding |
| `TyposquatTechnique` | `typosquat.rs:23` | 7 variants: `CharacterSwap`, `CharacterOmission`, `CharacterInsertion`, `CharacterReplacement`, `Hyphenation`, `Subdomain`, `Combosquatting` | Technique classification |
| `TyposquatRiskLevel` | `typosquat.rs:34` | `None`, `Low`, `Medium`, `High`, `Critical` | Aggregate risk |
| `TyposquatDetector` | `typosquat.rs:87` | `threshold: f64` | Detection engine |

### SBOM Generation

#### Source Methods

| Method | Input File | Ecosystem | Parses | File:line |
|--------|-----------|-----------|--------|-----------|
| `generate_from_cargo()` | `Cargo.toml` + `Cargo.lock` | cargo | Package name/version from `[package]`, deps from `[[package]]` blocks, direct deps from `[dependencies]` | `sbom.rs:77-` |
| `generate_from_npm()` | `package.json` + `package-lock.json` | npm | `dependencies` object, lockfile `packages` map | `sbom.rs:122-` |
| `generate_from_requirements()` | `requirements.txt` | pypi | Lines with `==`, `===`, `~=`, `!=`, `>=`, `<=`, `>`, `<` operators | `sbom.rs:209-` |

#### Cargo.toml Parsing (`sbom.rs:376-416`)

Custom line-by-line parser, **not** using a TOML library. Extracts `name` and `version` from the `[package]` section by matching `name = "..."` or `name="..."` patterns. Stops at the next `[section]` header. `[dependencies]` parsing (`sbom.rs:465-487`) similarly scans for `name = value` lines.

#### Cargo.lock Parsing (`sbom.rs:418-463`)

Iterates `[[package]]` blocks, extracting `name` and `version` fields. Each block becomes an `SbomComponent` with `is_direct: false`. Direct dependency marking is done in a second pass via `parse_cargo_toml_deps()`.

#### npm Parsing (`sbom.rs:98-183`)

Reads `dependencies` from `package.json`, strips `^`/`~` prefixes from versions. Merges `package-lock.json` `packages` map for transitive deps, deduplicating by name.

#### requirements.txt Parsing (`sbom.rs:185-279`)

Handles 8 version specifier patterns (`===`, `==`, `~=`, `!=`, `>=`, `<=`, `>`, `<`). Skips lines starting with `#` (comments) or `-` (flags like `-r`, `-e`). Falls back to `*` for version when no operator found.

#### CycloneDX Export (`sbom.rs:281-326`)

Outputs JSON conforming to CycloneDX 1.4 spec:
```json
{
  "bomFormat": "CycloneDX",
  "specVersion": "1.4",
  "metadata": { "component": { "name", "version" }, "timestamp" },
  "components": [{ "type": "library", "name", "version", "purl", "ecosystem", "licenses" }],
  "vulnerabilities": [{ "id", "source", "ratings", "description", "affects" }]
}
```
License data is rendered as `{"license": {"id": "..."}}` objects. Vulnerabilities are included only when non-empty.

#### SPDX Export (`sbom.rs:328-374`)

Outputs tag-value format (SPDX 2.3):
```
SPDXVersion: SPDX-2.3
DataLicense: CC0-1.0
SPDXID: SPDXRef-DOCUMENT
DocumentName: <name>
DocumentNamespace: https://spdx.org/spdxdocs/<name>-<version>
Creator: Tool: eggsec-<version>
Created: <timestamp>

## Package: <name>
SPDXID: SPDXRef-Package-<name>
PackageVersion: <version>
ExternalRef: PACKAGE-MANAGER purl <purl>
PackageLicenseDeclared: <licenses joined by " AND ">
```

#### Output Shape Differences

| Aspect | CycloneDX | SPDX |
|--------|-----------|------|
| Format | JSON | Tag-value text |
| Spec version | 1.4 | 2.3 |
| Component structure | Nested JSON objects | Flat `## Package:` blocks |
| License format | `{"license": {"id": "MIT"}}` array | `PackageLicenseDeclared: MIT AND Apache-2.0` |
| Vulnerabilities | Embedded `vulnerabilities` array | Not included |
| PURL | `purl` field in component | `ExternalRef: PACKAGE-MANAGER purl` |
| Creator metadata | `metadata.timestamp` | `Creator: Tool: eggsec-<version>` |

### Manifest Discovery (`scanner.rs:57-174`)

`scan_repo()` uses `walkdir` to traverse the repository, matching filenames against known manifest types:

| Filename | `ManifestType` | Dependency Count Source |
|----------|---------------|----------------------|
| `Cargo.toml` | `CargoToml` | Lines under `[dependencies]` section (`count_cargo_toml_deps`) |
| `Cargo.lock` | `CargoLock` | None |
| `package.json` | `PackageJson` | `dependencies` + `devDependencies` object lengths |
| `package-lock.json` | `PackageLockJson` | None |
| `yarn.lock` | `YarnLock` | None |
| `pnpm-lock.yaml` | `PnpmLockYaml` | None |
| `go.mod` | `GoMod` | Tab-indented non-comment lines (`count_go_mod_deps`) |
| `go.sum` | `GoSum` | None |
| `Dockerfile` | — | Triggers `check_dockerfile()` |
| `*.yml`/`*.yaml` in `.github/workflows/` | `GitHubActions` | Triggers `check_github_actions()` |

### Dockerfile Analysis (`scanner.rs:280-344`)

Three checks performed:

| Check | Severity | Logic | File:line |
|-------|----------|-------|-----------|
| `ADD` instead of `COPY` | Low | `ADD` used for non-URL, non-archive local files | `scanner.rs:291-308` |
| Latest/untagged base image | Info | `FROM` with `:latest` or no tag | `scanner.rs:311-325` |
| No `USER` instruction | Medium | No `USER` directive in any line | `scanner.rs:329-340` |

Archive detection (`scanner.rs:291-295`): ADD with `.tar`, `.gz`, `.zip`, `.xz`, `.bz2` is excluded from the ADD-finding (Docker archive auto-extract is intentional).

### GitHub Actions Analysis (`scanner.rs:359-399`)

| Check | Severity | Logic | File:line |
|-------|----------|-------|-----------|
| Overly broad permissions | Medium | `permissions: write-all`, `read-all`, or block form | `scanner.rs:364-379` |
| Unpinned action | Low | `uses:` line without `@v*`, `@sha:*`, or 7+ hex char after `@` | `scanner.rs:382-395` |

Pin detection (`scanner.rs:346-357`): Accepts `@v*` (version tag), `@sha:*` (explicit SHA), or `@<7+ hex chars>` (implicit SHA pin).

### `collect_package_names()` (`scanner.rs:180-232`)

Flat (no subdirectory walking) package name extraction from `Cargo.toml`, `package.json`, and `requirements.txt`. Used by the typosquat detector. Parses only top-level `[dependencies]` for Cargo, `dependencies` object for npm, and `==`/`>=` lines for Python.

### Typosquat Detection (`typosquat.rs`)

#### Algorithm (`typosquat.rs:96-158`)

For each input package name, iterates the 42-entry `WELL_KNOWN_PACKAGES` list (`typosquat.rs:42-85`) and computes:

1. **Levenshtein distance** (`typosquat.rs:160-191`): Classic O(n×m) dynamic programming, character-level.
2. **Similarity score**: `1.0 - (distance / max(len1, len2))` (`typosquat.rs:125-129`). Range [0.0, 1.0].
3. **Threshold check**: `similarity >= threshold && similarity < 1.0` (`typosquat.rs:131`). The `< 1.0` guard prevents flagging exact matches.

Default threshold is configurable via `TyposquatDetector::new(threshold)`.

#### Technique Detection (`typosquat.rs:193-230`)

| Condition | Technique | Logic |
|-----------|-----------|-------|
| Same length, 1 char differs | `CharacterReplacement` | `typosquat.rs:197-199` |
| Same length, 2 chars differ and are transposed | `CharacterSwap` | `typosquat.rs:200-206` |
| Same length, 2 chars differ (not transposed) | `CharacterReplacement` | `typosquat.rs:205` |
| Input 1 char longer than known | `CharacterInsertion` | `typosquat.rs:210-211` |
| Input 1 char shorter than known | `CharacterOmission` | `typosquat.rs:213-214` |
| Input contains `-`, known doesn't | `Hyphenation` | `typosquat.rs:217-218` |
| Input contains `.`, known doesn't | `Subdomain` | `typosquat.rs:221-222` |
| No other technique detected | `Combosquatting` | `typosquat.rs:225-227` |

#### Severity Mapping (`typosquat.rs:133-141`)

| Similarity Score | Severity |
|-----------------|----------|
| ≥ 0.9 | Critical |
| ≥ 0.8 | High |
| ≥ 0.7 | Medium |
| < 0.7 (but ≥ threshold) | Low |

#### Risk Level (`typosquat.rs:232-244`)

Aggregate risk is the highest severity among all findings: Critical > High > Medium > Low > None.

#### Known Package List (`typosquat.rs:42-85`)

42 packages spanning 6 ecosystems: Python (requests, flask, django, numpy, pandas, scipy, tensorflow, pytorch), JavaScript (express, lodash, react, angular, vue, axios, webpack, babel, moment, underscore, async, await, chalk), Rust (serde, tokio, actix, rocket, clap, rand, regex, reqwest), Ruby (rails, sinatra, devise, rspec, sidekiq, puma, unicorn), Java (spring-boot, hibernate, jackson, guava, lombok, log4j).

## Behavior / Flow

### SBOM Generation Flow (CLI)

```
handle_sbom()                          [handlers/sbom.rs:9]
  ├── SbomCommand::Generate
  │   ├── validate_project_path()      [handlers/sbom.rs:4]
  │   ├── SbomGenerator::new()         [sbom.rs:49]
  │   ├── Auto-detect ecosystem:
  │   │   ├── Cargo.toml exists → generate_from_cargo()
  │   │   ├── package.json exists → generate_from_npm()
  │   │   └── requirements.txt exists → generate_from_requirements()
  │   ├── Export:
  │   │   ├── "cyclonedx" → export_cyclonedx()
  │   │   ├── "spdx" → export_spdx()
  │   │   └── "json" → serde_json::to_string_pretty()
  │   └── Write to file or stdout
  └── SbomCommand::CheckTyposquat
      ├── collect_package_names()      [scanner.rs:180]
      ├── TyposquatDetector::new()     [typosquat.rs:92]
      ├── detector.check_packages()    [typosquat.rs:96]
      └── Print findings
```

### Supply Chain Scan Flow

```
scan_repo(repo_path)                  [scanner.rs:59]
  → walkdir traversal
  → Match filenames to ManifestType
  → count_*_deps() for dependency counts
  → check_dockerfile() → Vec<SupplyChainFinding>
  → check_github_actions() → Vec<SupplyChainFinding>
  → SupplyChainScanResult
```

## Public API

| Function / Method | Signature | Location |
|-------------------|-----------|----------|
| `SbomGenerator::new` | `fn() -> Self` | `sbom.rs:73` |
| `SbomGenerator::generate_from_cargo` | `fn(&self, path, format) -> Result<SbomReport>` | `sbom.rs:77` |
| `SbomGenerator::generate_from_npm` | `fn(&self, path, format) -> Result<SbomReport>` | `sbom.rs:122` |
| `SbomGenerator::generate_from_requirements` | `fn(&self, path, format) -> Result<SbomReport>` | `sbom.rs:209` |
| `SbomGenerator::export_cyclonedx` | `fn(&self, report) -> Result<String>` | `sbom.rs:305` |
| `SbomGenerator::export_spdx` | `fn(&self, report) -> Result<String>` | `sbom.rs:352` |
| `scan_repo` | `fn(repo_path: &Path) -> Result<SupplyChainScanResult>` | `scanner.rs:82` |
| `collect_package_names` | `fn(project_path: &Path) -> Result<Vec<String>>` | `scanner.rs:203` |
| `TyposquatDetector::new` | `fn(threshold: f64) -> Self` | `typosquat.rs:92` |
| `TyposquatDetector::check_packages` | `fn(&self, &[String]) -> Result<TyposquatReport>` | `typosquat.rs:96` |
| `TyposquatDetector::check_package` | `fn(&self, &str) -> Option<TyposquatFinding>` | `typosquat.rs:114` |

## Integration Points

### CLI (`cli/misc.rs:361-395`, `commands/handlers/sbom.rs`)

- **Commands**: `eggsec sbom generate <project> [--format cyclonedx|spdx|json] [--output <file>]`, `eggsec sbom check-typosquat <project> [--threshold <f64>]`
- **Dispatch**: `Commands::Sbom(args)` → `handle_sbom()` (`handlers/mod.rs:518`)
- **Output**: CycloneDX/SPDX JSON to stdout or file; typosquat results as plain-text table.

### Dispatch (`dispatch/mod.rs`)

- No `TaskKind::Sbom` — SBOM is a helper-only command, not a dispatched task. Listed in `cli_commands.md` as `HelperOnly` mode.

### Python Bindings (`crates/eggsec-python/src/sbom.rs`, `async_engine.rs:1974-2013`)

- **Stable operation**: `generate_sbom(project_path, ecosystem, format)` — part of the 22-operation stable core.
- **Async variant**: `async_generate_sbom()` for non-blocking generation.
- **Format mapping**: `SbomFormatPy::CycloneDx` / `SbomFormatPy::Spdx` → engine `SbomFormat` via `to_engine()`/`from_engine()`.
- **Feature gate**: `sbom` feature required; included in `full-no-system` profile.
- **No container relationship**: Container image scanning (`scan_docker_image`) is a separate operation, not integrated with SBOM generation.

### Pipeline

- No pipeline stage for SBOM. The `supply_chain` module is not part of the multi-stage pipeline — it operates as a standalone CLI/Python utility.

## Data Model

```
SupplyChainReport
├── sbom: Option<SbomReport>
│   ├── format: SbomFormat (CycloneDx | Spdx)
│   ├── components: Vec<SbomComponent>
│   │   ├── name: String
│   │   ├── version: String
│   │   ├── ecosystem: String ("cargo" | "npm" | "pypi")
│   │   ├── purl: String (pkg:cargo/serde@1.0)
│   │   ├── licenses: Vec<String>
│   │   └── is_direct: bool
│   └── vulnerabilities: Vec<SbomVulnerability> (empty by default)
├── typosquatting: Option<TyposquatReport>
│   ├── packages_checked: usize
│   ├── suspicious_packages: Vec<TyposquatFinding>
│   │   ├── similarity_score: f64 (0.0–1.0)
│   │   ├── techniques: Vec<TyposquatTechnique> (7 variants)
│   │   └── severity: Severity
│   └── risk_level: TyposquatRiskLevel
├── findings: Vec<SupplyChainFinding>
│   ├── category: String ("dockerfile" | "github_actions")
│   ├── file_path: Option<String>
│   └── line: Option<u32>
├── total_packages: usize
└── total_risks: usize
```

## Testing

| Module | Tests | Key assertions |
|--------|-------|----------------|
| `sbom.rs` | 14 tests | Package name/version extraction, Cargo.lock parsing, Cargo.toml deps, requirements.txt operators (8 patterns), SPDX format header, CycloneDX JSON structure, license rendering, missing manifest error |
| `scanner.rs` | 16 tests | Empty repo, Cargo.toml/package.json/go.mod detection, Dockerfile ADD/COPY/USER/latest, GitHub Actions permissions/pinning/SHA, `collect_package_names` (Cargo/npm/Python/empty) |
| `typosquat.rs` | 10 tests | Levenshtein distance (3 cases), character swap/omission/replacement detection, no false positive, multi-package check, risk level calculation, technique detection |
| `mod.rs` | 1 test | Finding creation |

Total: **41 unit tests** across all files.

## Invariants & Gotchas

1. **Crate-level exclusivity**: `supply_chain/` compiles as `pub mod` with `sbom` feature, `mod` (dead_code) without. The `#[cfg(feature = "sbom")]` guard on `scan_repo()` is a second gate within the module.
2. **No external vulnerability database**: `SbomReport.vulnerabilities` is always empty (`Vec::new()`). CVE matching against SBOMs is explicitly out of scope — see the Scope section in the original doc.
3. **Cargo.toml parsing is not TOML-compliant**: The custom line parser does not handle inline tables, multi-line values, or dotted keys. Edge cases with complex `Cargo.toml` formats may produce incorrect dependency counts.
4. **`requirements.txt` operator priority**: The parser checks `===` before `==` to avoid false splits. However, it does not handle environment markers (`; python_version >= "3.6"`), extras (`requests[security]`), or pip flags (`-r`, `-e`, `-c`).
5. **npm version prefix stripping**: `^` and `~` are stripped for display but the raw version from `package.json` (with prefix) is used for the purl `@version` field.
6. **Typosquat false positives**: The 42-package known list is static. Packages with names similar to each other (not just to well-known packages) are not cross-compared.
7. **Levenshtein is character-level**: Multi-byte Unicode characters are handled correctly (char iteration), but the similarity score normalizes by byte length of chars, not grapheme clusters.
8. **Dockerfile ADD-archive exclusion** is heuristic: it checks for `.tar`, `.gz`, `.zip`, `.xz`, `.bz2` substrings in the entire line, which could match non-archive filenames containing those strings.

## Bug Sweep

| Finding | File:line | Severity | Description |
|---------|-----------|----------|-------------|
| `max_len` division guard | `typosquat.rs:126` | None | `max_len == 0.0` check correctly prevents division by zero when both strings are empty (though `lower == known_lower` would skip first). |
| `unwrap_or` in requirements parsing | `sbom.rs:202,207,213,219,225,231,237,239` | Low | `trimmed.get(..pos).unwrap_or(trimmed)` and `trimmed.get(pos+N..).unwrap_or("*")` — safe; `.get()` returns `None` for out-of-bounds, falling back to full string or `"*"`. |
| `unwrap_or` in npm parsing | `sbom.rs:123,124,129,148` | Low | `.unwrap_or("*")` on `Option<&str>` from `.as_str()` — safe fallback. |
| `unwrap_or_else` in sbom.rs | `sbom.rs:67,70,112,114` | Low | Fallback to `"unknown"` / `"0.0.0"` — safe defaults. |
| No panicking `unwrap()`/`expect()` in non-test code | — | None | All `unwrap()` calls are in `#[cfg(test)]` blocks or behind `.ok()` / `.unwrap_or` patterns. |
| No silent error suppression | — | None | File I/O errors propagate via `?` operator; `walkdir` errors are logged with `tracing::warn!`/`tracing::debug!` (`scanner.rs:76,80`). |
| No division-by-zero in score normalization | `typosquat.rs:125-129` | None | `max_len` is `lower.len().max(known_lower.len()) as f64`. When both strings are empty, `max_len == 0.0` and the `continue` guard (`typosquat.rs:127`) skips the division. |

**Confirmed bugs: 0.** No division-by-zero, no panicking unwraps, no silent error suppression in production code.

## Repository Dependency Policy — Advisory Backlog Closure (Milestone 015)

This section is about the **repository's own dependency posture**, not about the
`eggsec::supply_chain` module described above. The module analyses *other*
projects' supply chains; the policy below governs *this* repository's. It is
recorded here so the supply-chain area of the architecture docs does not drift
from `deny.toml`, which remains the single canonical dependency-policy source.

**Milestone 015 (2026-10-24) closed the workspace's advisory backlog.** The
baseline was 23 advisory entries across 20 advisory IDs (14 `vulnerability`,
7 `unmaintained`, 2 `unsound`), measured with `cargo deny --workspace
--all-features check advisories`. They were disposed of as follows:

| Disposition | Count | What happened |
|-------------|-------|---------------|
| Cleared by a lockfile refresh | 9 | `cargo update` moved `ctutils` 0.4.2→0.4.5, `rustls` 0.23.45→0.23.46, `rustls-webpki` 0.103.15→0.103.16, `tokio` 1.53.1→1.54.1, `webpki-roots` 1.0.9→1.0.11, `webpki-root-certs` 1.0.9→1.0.11. No manifest change. |
| Cleared by a major bump | 3 | `zip` 2.4.2 → 9.x in `eggsec-mobile-lab` and `eggsec`. `zip` 9.1.0 declares `rust-version = "1.88"`, below the workspace floor, so the MSRV did not move. |
| Recorded as owned exceptions | 11 | Four clusters, listed below. Each carries an owner, a `2026-12-12` review-by date, and the exact clearing condition. |

**The four exception clusters** (all recorded in `deny.toml`'s `[advisories]`
`ignore` array and in `docs/DEPENDENCY_EXCEPTIONS.md`):

| Cluster | Advisory IDs | Owner | Why the upgrade is not adopted |
|---------|--------------|-------|--------------------------------|
| `glib` 0.18.5 / `proc-macro-error` 1.0.4 | RUSTSEC-2024-0429, RUSTSEC-2024-0370 | `eggsec-gui` | gtk-rs 0.20 is unreachable: no published `muda`/`tao`/`wry` uses gtk-rs 0.20+, and `tauri` 2.12.5 still requires `gtk ^0.18`. Upstream-blocked, not policy-blocked. |
| `rustls` 0.21.12 / `rustls-webpki` 0.101.7 | RUSTSEC-2026-0346, -0349, -0382 | `eggsec-db-lab` | `tiberius` 0.13.0 reaches `rustls` 0.23 only through `tokio-rustls` with default features, which enables the `aws-lc-rs` provider that `[bans] deny = ["aws-lc-rs"]` forbids (ring-only TLS). Policy-blocked. |
| `unsafe-libyaml` 0.2.11 | RUSTSEC-2024-0449 | `eggsec (config)` / `eggsec-web-proxy` | 0.2.11 is the latest published version and `serde_yaml_neo` 0.11.0 hard-depends on it. The advisory kind is `unmaintained`, not a vulnerability; the parser *is* exercised, on partly-downloaded template content. |
| `unic-*` 0.9.0 (×5) | RUSTSEC-2025-0075, -0080, -0081, -0098, -0100 | `eggsec-gui` | `urlpattern` 0.6.0 drops the set, but `tauri-utils` 2.10.x declares MSRV 1.90, above the workspace's 1.89 floor. MSRV-gated, not upgrade-less. |

**Guard Check 190** (`scripts/check-architecture-guards.sh`) makes "no
unowned exception" a mechanically enforced property rather than a review
convention: every `deny.toml` ignore must carry a non-empty reason naming
`docs/DEPENDENCY_EXCEPTIONS.md`, every ignored ID must have a record with a
non-empty `| Owner |` and a dated `| Review-by |`, and no `RESOLVED` record
may keep a live ignore. The check fails closed — an unparseable policy file
is a failure, not a pass.

**The tool that measures the policy is itself pinned (2026-10-25).** Two
properties are enforced, because the policy is only as trustworthy as the tool
that decides it:

- **One tool at one version** (guard Check 193): every `tool: cargo-deny` in
  `.github/workflows/*.yml` must name an exact `x.y.z`, every workflow's
  version must be the same, and it must be no older than **0.20.2** — the
  version the local contract runs. This is milestone 015's finding **F4** made
  enforceable: "a local green is not proof of a CI green" stops being true only
  when both sides run the same tool.
- **The policy is decided without the inclusion graph** (guard Check 194):
  every `cargo deny` invocation in the `Makefile` must pass
  `--hide-inclusion-graph`. cargo-deny prints an inclusion graph for every
  duplicated crate, and this workspace duplicates 98 of them; rendering those
  graphs is where the CI runner stalls — the deep-check job hangs inside
  `check bans`, mid-graph, with no output and no error. The stall reproduces
  on cargo-deny 0.19.0 and 0.20.2 alike, and a control branch proved a shallow
  checkout does not cause it, so the flag — not a version — is the fix. The
  flag removes only the rendering path: the same 98 duplicate warnings are
  printed and the same four verdicts are reached, so what the policy says is
  unchanged.

Neither check changes `deny.toml`. They change *how* the policy is decided,
which was the point: the measurement had stopped being reproducible, and now
it is.

Nothing in this milestone changed the ring-only TLS rule, the MSRV floor, the
`[advisories]` `unmaintained`/`unsound` settings, the `[licenses]` allow list,
or the `[bans]` deny list.

---

*Last verified against source: 2026-08-25; counts re-verified 2026-09-22 (systematic review); line cites refreshed 2026-09-25 (sbom 790 / scanner 767, ManifestType scanner.rs:31, known list 42 verified)*
