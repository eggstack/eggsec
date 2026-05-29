use serde::{Deserialize, Serialize};
use std::path::Path;

/// Dependency manifest type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ManifestType {
    CargoToml,
    CargoLock,
    PackageJson,
    PackageLockJson,
    YarnLock,
    PnpmLockYaml,
    GoMod,
    GoSum,
    Dockerfile,
    GitHubActions,
}

impl std::fmt::Display for ManifestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CargoToml => write!(f, "Cargo.toml"),
            Self::CargoLock => write!(f, "Cargo.lock"),
            Self::PackageJson => write!(f, "package.json"),
            Self::PackageLockJson => write!(f, "package-lock.json"),
            Self::YarnLock => write!(f, "yarn.lock"),
            Self::PnpmLockYaml => write!(f, "pnpm-lock.yaml"),
            Self::GoMod => write!(f, "go.mod"),
            Self::GoSum => write!(f, "go.sum"),
            Self::Dockerfile => write!(f, "Dockerfile"),
            Self::GitHubActions => write!(f, "GitHub Actions workflow"),
        }
    }
}

/// A discovered manifest file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredManifest {
    pub path: String,
    pub manifest_type: ManifestType,
    pub dependency_count: Option<usize>,
}

/// A supply chain finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyChainFinding {
    pub severity: String,
    pub category: String,
    pub title: String,
    pub description: String,
    pub file_path: Option<String>,
    pub line: Option<u32>,
}

/// Supply chain scan result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyChainScanResult {
    pub repo_path: String,
    pub manifests: Vec<DiscoveredManifest>,
    pub findings: Vec<SupplyChainFinding>,
    pub dockerfile_found: bool,
    pub github_actions_found: bool,
    pub total_dependencies: usize,
}

/// Scan a local repository for supply chain artifacts
#[cfg(feature = "sbom")]
pub fn scan_repo(repo_path: &Path) -> anyhow::Result<SupplyChainScanResult> {
    let mut manifests = Vec::new();
    let mut findings = Vec::new();
    let mut dockerfile_found = false;
    let mut github_actions_found = false;
    let mut total_dependencies = 0;

    // Walk the repo looking for known manifest files
    for entry in walkdir::WalkDir::new(repo_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        match file_name {
            "Cargo.toml" => {
                manifests.push(DiscoveredManifest {
                    path: path.display().to_string(),
                    manifest_type: ManifestType::CargoToml,
                    dependency_count: count_cargo_toml_deps(path).ok(),
                });
            }
            "Cargo.lock" => {
                manifests.push(DiscoveredManifest {
                    path: path.display().to_string(),
                    manifest_type: ManifestType::CargoLock,
                    dependency_count: None,
                });
            }
            "package.json" => {
                manifests.push(DiscoveredManifest {
                    path: path.display().to_string(),
                    manifest_type: ManifestType::PackageJson,
                    dependency_count: count_package_json_deps(path).ok(),
                });
            }
            "package-lock.json" => {
                manifests.push(DiscoveredManifest {
                    path: path.display().to_string(),
                    manifest_type: ManifestType::PackageLockJson,
                    dependency_count: None,
                });
            }
            "yarn.lock" => {
                manifests.push(DiscoveredManifest {
                    path: path.display().to_string(),
                    manifest_type: ManifestType::YarnLock,
                    dependency_count: None,
                });
            }
            "pnpm-lock.yaml" => {
                manifests.push(DiscoveredManifest {
                    path: path.display().to_string(),
                    manifest_type: ManifestType::PnpmLockYaml,
                    dependency_count: None,
                });
            }
            "go.mod" => {
                manifests.push(DiscoveredManifest {
                    path: path.display().to_string(),
                    manifest_type: ManifestType::GoMod,
                    dependency_count: count_go_mod_deps(path).ok(),
                });
            }
            "go.sum" => {
                manifests.push(DiscoveredManifest {
                    path: path.display().to_string(),
                    manifest_type: ManifestType::GoSum,
                    dependency_count: None,
                });
            }
            "Dockerfile" => {
                dockerfile_found = true;
                findings.extend(check_dockerfile(path));
            }
            name if (name.ends_with(".yml") || name.ends_with(".yaml"))
                && path.to_string_lossy().contains(".github/workflows") =>
            {
                github_actions_found = true;
                findings.extend(check_github_actions(path));
            }
            _ => {}
        }
    }

    // Calculate total dependencies
    for manifest in &manifests {
        if let Some(count) = manifest.dependency_count {
            total_dependencies += count;
        }
    }

    Ok(SupplyChainScanResult {
        repo_path: repo_path.display().to_string(),
        manifests,
        findings,
        dockerfile_found,
        github_actions_found,
        total_dependencies,
    })
}

fn count_cargo_toml_deps(path: &Path) -> anyhow::Result<usize> {
    let content = std::fs::read_to_string(path)?;
    let mut count = 0;
    let mut in_deps = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps = trimmed == "[dependencies]" || trimmed.starts_with("[dependencies.");
        } else if in_deps && !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with('[') {
            count += 1;
        }
    }

    Ok(count)
}

fn count_package_json_deps(path: &Path) -> anyhow::Result<usize> {
    let content = std::fs::read_to_string(path)?;
    let json: serde_json::Value = serde_json::from_str(&content)?;

    let mut count = 0;
    if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
        count += deps.len();
    }
    if let Some(dev_deps) = json.get("devDependencies").and_then(|d| d.as_object()) {
        count += dev_deps.len();
    }

    Ok(count)
}

fn count_go_mod_deps(path: &Path) -> anyhow::Result<usize> {
    let content = std::fs::read_to_string(path)?;
    Ok(content.lines().filter(|l| l.starts_with('\t')).count())
}

fn check_dockerfile(path: &Path) -> Vec<SupplyChainFinding> {
    let mut findings = Vec::new();

    if let Ok(content) = std::fs::read_to_string(path) {
        let lines: Vec<&str> = content.lines().collect();
        let has_user_instruction = lines.iter().any(|l| l.trim().starts_with("USER "));

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Check for ADD instead of COPY
            if trimmed.starts_with("ADD ") && !trimmed.contains("http") {
                findings.push(SupplyChainFinding {
                    severity: "low".to_string(),
                    category: "dockerfile".to_string(),
                    title: "ADD used instead of COPY".to_string(),
                    description: "Prefer COPY over ADD for local file copies to avoid unintended effects".to_string(),
                    file_path: Some(path.display().to_string()),
                    line: Some((i + 1) as u32),
                });
            }

            // Check for latest tag or untagged base image
            if trimmed.contains(":latest") || (trimmed.starts_with("FROM ") && !trimmed.contains(":")) {
                findings.push(SupplyChainFinding {
                    severity: "info".to_string(),
                    category: "dockerfile".to_string(),
                    title: "Using latest or untagged base image".to_string(),
                    description: "Pin base images to specific versions for reproducibility".to_string(),
                    file_path: Some(path.display().to_string()),
                    line: Some((i + 1) as u32),
                });
            }
        }

        // Check for running as root (no USER instruction)
        if !has_user_instruction && !lines.is_empty() {
            findings.push(SupplyChainFinding {
                severity: "medium".to_string(),
                category: "dockerfile".to_string(),
                title: "No USER instruction found".to_string(),
                description: "Container may run as root. Add a USER instruction to run as non-root".to_string(),
                file_path: Some(path.display().to_string()),
                line: None,
            });
        }
    }

    findings
}

fn is_pinned_action(line: &str) -> bool {
    // Check for @v (version tag) or @sha: (explicit SHA pin) or a 7+ char hex hash after @
    if line.contains("@v") || line.contains("@sha:") {
        return true;
    }
    // Check for SHA-pinned actions like actions/checkout@abc123def456
    if let Some(at_pos) = line.rfind('@') {
        let hash = &line[at_pos + 1..];
        return hash.len() >= 7 && hash.chars().all(|c| c.is_ascii_hexdigit());
    }
    false
}

fn check_github_actions(path: &Path) -> Vec<SupplyChainFinding> {
    let mut findings = Vec::new();

    if let Ok(content) = std::fs::read_to_string(path) {
        // Check for overly broad permissions
        if content.contains("permissions: write-all") || content.contains("permissions: read-all") {
            findings.push(SupplyChainFinding {
                severity: "medium".to_string(),
                category: "github_actions".to_string(),
                title: "Overly broad GitHub Actions permissions".to_string(),
                description: "Workflows with write-all or read-all permissions may be excessive".to_string(),
                file_path: Some(path.display().to_string()),
                line: None,
            });
        }

        // Check for unpinned actions
        for (i, line) in content.lines().enumerate() {
            if line.contains("uses:") && !is_pinned_action(line) {
                findings.push(SupplyChainFinding {
                    severity: "low".to_string(),
                    category: "github_actions".to_string(),
                    title: "Unpinned GitHub Action".to_string(),
                    description: "Pin actions to specific versions or SHA hashes".to_string(),
                    file_path: Some(path.display().to_string()),
                    line: Some((i + 1) as u32),
                });
            }
        }
    }

    findings
}

#[cfg(all(test, feature = "sbom"))]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn scan_empty_repo() {
        let dir = TempDir::new().unwrap();
        let result = scan_repo(dir.path()).unwrap();
        assert!(result.manifests.is_empty());
        assert!(result.findings.is_empty());
    }

    #[test]
    fn scan_finds_cargo_toml() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[dependencies]\nserde = \"1.0\"\n").unwrap();

        let result = scan_repo(dir.path()).unwrap();
        assert_eq!(result.manifests.len(), 1);
        assert_eq!(result.manifests[0].manifest_type, ManifestType::CargoToml);
        assert_eq!(result.manifests[0].dependency_count, Some(1));
    }

    #[test]
    fn scan_finds_package_json() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"dependencies": {"express": "^4.18"}, "devDependencies": {"jest": "^29"}}"#,
        )
        .unwrap();

        let result = scan_repo(dir.path()).unwrap();
        assert_eq!(result.manifests.len(), 1);
        assert_eq!(result.manifests[0].dependency_count, Some(2));
    }

    #[test]
    fn scan_detects_dockerfile_issues() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Dockerfile"), "FROM ubuntu\nADD file.txt /app/\n").unwrap();

        let result = scan_repo(dir.path()).unwrap();
        assert!(result.dockerfile_found);
        assert!(result.findings.iter().any(|f| f.title.contains("ADD")));
    }

    #[test]
    fn scan_detects_github_actions_issues() {
        let dir = TempDir::new().unwrap();
        let gh_dir = dir.path().join(".github/workflows");
        fs::create_dir_all(&gh_dir).unwrap();
        fs::write(
            gh_dir.join("ci.yml"),
            "name: CI\non: push\npermissions: write-all\njobs:\n  test:\n    runs-on: ubuntu-latest\n",
        )
        .unwrap();

        let result = scan_repo(dir.path()).unwrap();
        assert!(result.github_actions_found);
        assert!(result.findings.iter().any(|f| f.title.contains("permissions")));
    }

    #[test]
    fn count_cargo_toml_with_dev_deps() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[dependencies]\nserde = \"1.0\"\ntokio = { version = \"1\", features = [\"full\"] }\n\n[dev-dependencies]\nassert_cmd = \"2\"\n",
        )
        .unwrap();

        let result = scan_repo(dir.path()).unwrap();
        assert_eq!(result.manifests[0].dependency_count, Some(2));
    }

    #[test]
    fn count_package_json_deps_only() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"dependencies": {"express": "^4.18"}}"#,
        )
        .unwrap();

        let result = scan_repo(dir.path()).unwrap();
        assert_eq!(result.manifests[0].dependency_count, Some(1));
    }

    #[test]
    fn dockerfile_no_user_instruction() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Dockerfile"), "FROM ubuntu\nRUN apt-get update\n").unwrap();

        let result = scan_repo(dir.path()).unwrap();
        assert!(result
            .findings
            .iter()
            .any(|f| f.title.contains("No USER")));
    }

    #[test]
    fn dockerfile_with_user_instruction() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Dockerfile"),
            "FROM ubuntu\nRUN apt-get update\nUSER nobody\n",
        )
        .unwrap();

        let result = scan_repo(dir.path()).unwrap();
        assert!(!result
            .findings
            .iter()
            .any(|f| f.title.contains("No USER")));
    }

    #[test]
    fn dockerfile_latest_tag() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Dockerfile"),
            "FROM ubuntu:latest\nRUN echo hello\n",
        )
        .unwrap();

        let result = scan_repo(dir.path()).unwrap();
        assert!(result
            .findings
            .iter()
            .any(|f| f.title.contains("latest")));
    }

    #[test]
    fn github_actions_unpinned_action() {
        let dir = TempDir::new().unwrap();
        let gh_dir = dir.path().join(".github/workflows");
        fs::create_dir_all(&gh_dir).unwrap();
        fs::write(
            gh_dir.join("ci.yml"),
            "name: CI\non: push\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout\n",
        )
        .unwrap();

        let result = scan_repo(dir.path()).unwrap();
        assert!(result
            .findings
            .iter()
            .any(|f| f.title.contains("Unpinned")));
    }

    #[test]
    fn github_actions_pinned_action() {
        let dir = TempDir::new().unwrap();
        let gh_dir = dir.path().join(".github/workflows");
        fs::create_dir_all(&gh_dir).unwrap();
        fs::write(
            gh_dir.join("ci.yml"),
            "name: CI\non: push\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n",
        )
        .unwrap();

        let result = scan_repo(dir.path()).unwrap();
        assert!(!result
            .findings
            .iter()
            .any(|f| f.title.contains("Unpinned")));
    }

    #[test]
    fn github_actions_sha_pinned_action() {
        let dir = TempDir::new().unwrap();
        let gh_dir = dir.path().join(".github/workflows");
        fs::create_dir_all(&gh_dir).unwrap();
        fs::write(
            gh_dir.join("ci.yml"),
            "name: CI\non: push\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@abc123def456\n",
        )
        .unwrap();

        let result = scan_repo(dir.path()).unwrap();
        assert!(!result
            .findings
            .iter()
            .any(|f| f.title.contains("Unpinned")));
    }

    #[test]
    fn go_mod_dep_count() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("go.mod"),
            "module example.com/mymod\n\ngo 1.21\n\nrequire (\n\tgolang.org/x/text v0.14.0\n\tgithub.com/gin-gonic/gin v1.9.1\n)\n",
        )
        .unwrap();

        let result = scan_repo(dir.path()).unwrap();
        assert_eq!(result.manifests.len(), 1);
        assert_eq!(result.manifests[0].dependency_count, Some(2));
    }

    #[test]
    fn multiple_manifest_types() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[dependencies]\nserde = \"1.0\"\n").unwrap();
        fs::write(dir.path().join("go.mod"), "module m\n\nrequire (\n\tpkg v1\n)\n").unwrap();
        fs::write(dir.path().join("Dockerfile"), "FROM alpine\n").unwrap();

        let result = scan_repo(dir.path()).unwrap();
        assert_eq!(result.manifests.len(), 2);
        assert!(result.dockerfile_found);
    }
}
