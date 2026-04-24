use crate::error::Result;
use crate::recon::secrets::{SecretFinding, SecretScanner};
use crate::utils::validate_git_repo_path;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitSecretsReport {
    pub repo_path: String,
    pub commits_scanned: usize,
    pub files_scanned: usize,
    pub findings: Vec<GitSecretFinding>,
    pub summary: GitSecretsSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitSecretsSummary {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitSecretFinding {
    pub commit_hash: String,
    pub commit_message: String,
    pub author: String,
    pub date: String,
    pub file_path: String,
    pub secret: SecretFinding,
    pub introduced_in: bool,
}

pub use crate::types::Severity;

pub struct GitSecretsScanner {
    secret_scanner: SecretScanner,
    max_commits: usize,
}

impl GitSecretsScanner {
    pub fn new(max_commits: usize) -> Self {
        Self {
            secret_scanner: SecretScanner::new(),
            max_commits,
        }
    }

    pub fn scan_directory(&self, repo_path: &str) -> Result<GitSecretsReport> {
        validate_git_repo_path(repo_path)?;

        let path = Path::new(repo_path);

        let canonical_path = path.canonicalize().map_err(|e| {
            crate::error::SlapperError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "Path does not exist or cannot be canonicalized: {} - {}",
                    repo_path, e
                ),
            ))
        })?;

        if !canonical_path.exists() {
            return Err(crate::error::SlapperError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Path does not exist: {}", repo_path),
            )));
        }

        let git_dir = canonical_path.join(".git");
        if !git_dir.exists() {
            return self.scan_directory_fallback(&canonical_path);
        }

        self.scan_with_git(repo_path)
    }

    fn scan_with_git(&self, repo_path: &str) -> Result<GitSecretsReport> {
        let output = std::process::Command::new("git")
            .args([
                "-C",
                repo_path,
                "log",
                "--all",
                "--format=%H|%s|%an|%ai",
                "-n",
                &self.max_commits.to_string(),
            ])
            .output();

        let commits = match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout
                    .lines()
                    .filter(|l| !l.is_empty())
                    .map(|line| {
                        let parts: Vec<&str> = line.splitn(4, '|').collect();
                        CommitInfo {
                            hash: parts.first().unwrap_or(&"").to_string(),
                            message: parts.get(1).unwrap_or(&"").to_string(),
                            author: parts.get(2).unwrap_or(&"").to_string(),
                            date: parts.get(3).unwrap_or(&"").to_string(),
                        }
                    })
                    .collect::<Vec<_>>()
            }
            _ => {
                return self.scan_directory_fallback(Path::new(repo_path));
            }
        };

        let mut findings = Vec::new();
        let mut files_scanned = 0;

        for commit in &commits {
            let diff_output = std::process::Command::new("git")
                .args([
                    "-C",
                    repo_path,
                    "diff",
                    "--unified=0",
                    &format!("{}^", commit.hash),
                    &commit.hash,
                ])
                .output();

            if let Ok(output) = diff_output {
                let diff = String::from_utf8_lossy(&output.stdout);
                let added_lines: Vec<String> = diff
                    .lines()
                    .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
                    .map(|l| l[1..].to_string())
                    .collect();

                if !added_lines.is_empty() {
                    let content = added_lines.join("\n");
                    let secrets = self.secret_scanner.scan(&content);

                    for secret in secrets {
                        let file_path = extract_file_from_diff(&diff, &secret.location);
                        findings.push(GitSecretFinding {
                            commit_hash: commit.hash.clone(),
                            commit_message: commit.message.clone(),
                            author: commit.author.clone(),
                            date: commit.date.clone(),
                            file_path,
                            secret,
                            introduced_in: true,
                        });
                    }
                }
            }

            let show_output = std::process::Command::new("git")
                .args(["-C", repo_path, "show", &commit.hash])
                .output();

            if let Ok(output) = show_output {
                let content = String::from_utf8_lossy(&output.stdout);
                let secrets = self.secret_scanner.scan(&content);
                files_scanned += 1;

                for secret in secrets {
                    findings.push(GitSecretFinding {
                        commit_hash: commit.hash.clone(),
                        commit_message: commit.message.clone(),
                        author: commit.author.clone(),
                        date: commit.date.clone(),
                        file_path: "unknown".to_string(),
                        secret,
                        introduced_in: false,
                    });
                }
            }
        }

        let summary = GitSecretsSummary {
            critical: findings
                .iter()
                .filter(|f| f.secret.severity == Severity::Critical)
                .count(),
            high: findings
                .iter()
                .filter(|f| f.secret.severity == Severity::High)
                .count(),
            medium: findings
                .iter()
                .filter(|f| f.secret.severity == Severity::Medium)
                .count(),
            low: findings
                .iter()
                .filter(|f| f.secret.severity == Severity::Low)
                .count(),
            info: findings
                .iter()
                .filter(|f| f.secret.severity == Severity::Info)
                .count(),
        };

        Ok(GitSecretsReport {
            repo_path: repo_path.to_string(),
            commits_scanned: commits.len(),
            files_scanned,
            findings,
            summary,
        })
    }

    fn scan_directory_fallback(&self, path: &Path) -> Result<GitSecretsReport> {
        let mut findings = Vec::new();
        let mut files_scanned = 0;

        let git_extensions = [
            ".rs", ".py", ".js", ".ts", ".go", ".java", ".rb", ".php", ".yml", ".yaml", ".toml",
            ".json", ".env", ".conf", ".cfg", ".ini", ".sh", ".bash", ".zsh", ".ps1", ".cs",
            ".cpp", ".c", ".h", ".hpp", ".swift", ".kt", ".scala", ".ex", ".exs",
        ];

        let skip_dirs = [
            ".git",
            "node_modules",
            "vendor",
            "target",
            "build",
            "dist",
            "__pycache__",
            ".venv",
            "venv",
            ".tox",
        ];

        self.walk_directory(
            path,
            &mut findings,
            &mut files_scanned,
            &git_extensions,
            &skip_dirs,
            path,
        )?;

        let summary = GitSecretsSummary {
            critical: findings
                .iter()
                .filter(|f| f.secret.severity == Severity::Critical)
                .count(),
            high: findings
                .iter()
                .filter(|f| f.secret.severity == Severity::High)
                .count(),
            medium: findings
                .iter()
                .filter(|f| f.secret.severity == Severity::Medium)
                .count(),
            low: findings
                .iter()
                .filter(|f| f.secret.severity == Severity::Low)
                .count(),
            info: findings
                .iter()
                .filter(|f| f.secret.severity == Severity::Info)
                .count(),
        };

        Ok(GitSecretsReport {
            repo_path: path.to_string_lossy().to_string(),
            commits_scanned: 0,
            files_scanned,
            findings,
            summary,
        })
    }

    fn walk_directory(
        &self,
        dir: &Path,
        findings: &mut Vec<GitSecretFinding>,
        files_scanned: &mut usize,
        git_extensions: &[&str],
        skip_dirs: &[&str],
        base_path: &Path,
    ) -> Result<()> {
        let entries = std::fs::read_dir(dir)?;
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if path.is_dir() {
                if skip_dirs.contains(&name_str.as_ref()) {
                    continue;
                }
                self.walk_directory(
                    &path,
                    findings,
                    files_scanned,
                    git_extensions,
                    skip_dirs,
                    base_path,
                )?;
            } else if path.is_file() {
                let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let is_relevant = git_extensions.contains(&extension)
                    || name_str == ".env"
                    || name_str == ".gitconfig"
                    || name_str == ".npmrc"
                    || name_str == ".pypirc";

                if !is_relevant {
                    continue;
                }

                if let Ok(content) = std::fs::read_to_string(&path) {
                    *files_scanned += 1;
                    let secrets = self.secret_scanner.scan(&content);

                    for secret in secrets {
                        let rel_path = path
                            .strip_prefix(base_path)
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .to_string();
                        findings.push(GitSecretFinding {
                            commit_hash: "working-tree".to_string(),
                            commit_message: "Current working tree".to_string(),
                            author: "unknown".to_string(),
                            date: "unknown".to_string(),
                            file_path: rel_path,
                            secret,
                            introduced_in: false,
                        });
                    }
                }
            }
        }
        Ok(())
    }

    pub fn scan_content_for_secrets(&self, content: &str) -> Vec<SecretFinding> {
        self.secret_scanner.scan(content)
    }
}

struct CommitInfo {
    hash: String,
    message: String,
    author: String,
    date: String,
}

fn extract_file_from_diff(diff: &str, _location: &str) -> String {
    for line in diff.lines() {
        if let Some(stripped) = line.strip_prefix("+++ b/") {
            return stripped.to_string();
        }
        if let Some(stripped) = line.strip_prefix("+++ ") {
            return stripped.to_string();
        }
    }
    "unknown".to_string()
}

pub fn scan_git_secrets(repo_path: &str, max_commits: usize) -> Result<GitSecretsReport> {
    let scanner = GitSecretsScanner::new(max_commits);
    scanner.scan_directory(repo_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_creation() {
        let scanner = GitSecretsScanner::new(100);
        assert_eq!(scanner.max_commits, 100);
    }

    #[test]
    fn test_scan_nonexistent_path() {
        let scanner = GitSecretsScanner::new(100);
        let result = scanner.scan_directory("/nonexistent/path/that/does/not/exist");
        assert!(result.is_err());
    }

    #[test]
    fn test_scan_content_for_secrets() {
        let scanner = GitSecretsScanner::new(100);
        let content = "AKIAIOSFODNN7EXAMPLE";
        let findings = scanner.scan_content_for_secrets(content);
        assert!(!findings.is_empty());
    }

    #[test]
    fn test_scan_current_directory() {
        let scanner = GitSecretsScanner::new(10);
        let result = scanner.scan_directory(".");
        assert!(result.is_ok(), "Git secrets scan failed: {:?}", result.err());
        let report = result.unwrap();
        assert!(report.commits_scanned <= 100,
            "Expected 0-100 commits, got {}", report.commits_scanned);
    }

    #[test]
    fn test_summary_creation() {
        let summary = GitSecretsSummary {
            critical: 2,
            high: 3,
            medium: 5,
            low: 1,
            info: 0,
        };
        assert_eq!(summary.critical, 2);
        assert_eq!(summary.high, 3);
    }

    #[test]
    fn test_git_finding_creation() {
        let finding = GitSecretFinding {
            commit_hash: "abc123".to_string(),
            commit_message: "test commit".to_string(),
            author: "test user".to_string(),
            date: "2024-01-01".to_string(),
            file_path: "config.py".to_string(),
            secret: SecretFinding {
                secret_type: crate::recon::secrets::SecretType::AwsAccessKey,
                value_preview: "AKIA...".to_string(),
                location: "position 0".to_string(),
                confidence: crate::recon::secrets::Confidence::High,
                severity: Severity::Critical,
                description: "AWS Access Key".to_string(),
            },
            introduced_in: true,
        };
        assert_eq!(finding.commit_hash, "abc123");
        assert!(finding.introduced_in);
    }

    #[test]
    fn test_extract_file_from_diff() {
        let diff = r#"diff --git a/config.py b/config.py
--- a/config.py
+++ b/config.py
@@ -1 +1 @@
-old_value
+new_value
"#;
        let file = extract_file_from_diff(diff, "position 0");
        assert_eq!(file, "config.py");
    }

    #[test]
    fn test_extract_file_from_diff_no_match() {
        let diff = "no diff header here";
        let file = extract_file_from_diff(diff, "position 0");
        assert_eq!(file, "unknown");
    }
}
