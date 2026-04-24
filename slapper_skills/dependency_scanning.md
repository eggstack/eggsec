---
name: dependency_scanning
description: "Dependency vulnerability scanning for multiple package ecosystems"
triggers:
  - dependency scan
  - supply chain
  - package vulnerability
  - dependency audit
  - sbom
metadata:
  category: recon
  tools: [recon, dependency]
  scope: local
---

## Overview

The dependency scanner identifies vulnerable third-party dependencies in projects by detecting manifest files from various package ecosystems and checking for known CVEs.

## Supported Ecosystems

| Ecosystem | Manifests | Lock Files |
|----------|----------|-----------|
| npm (Node.js) | package.json | package-lock.json, yarn.lock |
| Rust (Cargo) | Cargo.toml | Cargo.lock |
| Go | go.mod | go.sum |
| Python (pip) | requirements.txt | - |
| Ruby (RubyGems) | Gemfile | Gemfile.lock |

## Usage

### CLI

```bash
# Scan a project directory
slapper recon deps /path/to/project

# Scan for specific ecosystems
slapper recon deps /path/to/project --ecosystems npm,cargo

# With vulnerability details
slapper recon deps /path/to/project --verbose
```

### Programmatic

```rust
use slapper::recon::DependencyScanner;

let scanner = DependencyScanner::new()?;
let report = scanner.scan_project("/path/to/project").await?;

println!("Found {} dependencies", report.total_dependencies);
println!("Found {} vulnerabilities", report.total_vulnerabilities);

for ecosystem in &report.ecosystems {
    println!("\n# {}", ecosystem.name);
    for vuln in &ecosystem.vulnerabilities {
        println!("{} [{}] {}", vuln.cve_id, vuln.severity, vuln.title);
    }
}
```

## Output Format

```json
{
  "project_path": "/path/to/project",
  "ecosystems": [
    {
      "name": "npm",
      "manifest_file": "package.json",
      "dependencies": [...],
      "vulnerabilities": [...]
    }
  ],
  "total_dependencies": 42,
  "total_vulnerabilities": 3,
  "summary": {
    "critical": 1,
    "high": 2,
    "medium": 0,
    "low": 0
  }
}
```

## Integration

This scanner is used by:
- `slapper agent` for automated assessments
- Supply chain security workflows
- CI/CD pipeline security scanning
- SBOM generation

## Triggers

Keywords that activate this skill:
- "dependency"
- "supply chain"
- "package scan"
- "vulnerable dependencies"
- "cve check"