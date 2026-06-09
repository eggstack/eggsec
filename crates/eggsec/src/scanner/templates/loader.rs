//! Template loading and validation
//!
//! Handles loading vulnerability templates from YAML/JSON files,
//! directory scanning, and validation.

use super::models::VulnerabilityTemplate;
use crate::error::{Result, EggsecError};
use crate::utils::validation::validate_path;
use std::path::{Path, PathBuf};

pub struct TemplateLoader {
    template_dirs: Vec<PathBuf>,
}

impl TemplateLoader {
    pub fn new(dirs: Vec<PathBuf>) -> Self {
        Self {
            template_dirs: dirs,
        }
    }

    pub fn add_directory(&mut self, dir: PathBuf) {
        self.template_dirs.push(dir);
    }

    pub fn load_template(&self, path: &Path) -> Result<VulnerabilityTemplate> {
        let base = self
            .template_dirs
            .first()
            .map(|d| d.as_path())
            .unwrap_or(path.parent().unwrap_or(path));
        let validated = validate_path(base, path)?;
        let content = std::fs::read_to_string(&validated)
            .map_err(|e| EggsecError::Config(format!("Failed to read template: {}", e)))?;

        self.parse_template(&content)
    }

    pub fn parse_template(&self, content: &str) -> Result<VulnerabilityTemplate> {
        let template: VulnerabilityTemplate = serde_yaml_neo::from_str(content)
            .or_else(|_| serde_json::from_str(content))
            .map_err(|e| EggsecError::Config(format!("Invalid template format: {}", e)))?;

        self.validate_template(&template)?;
        Ok(template)
    }

    pub fn validate_template(&self, template: &VulnerabilityTemplate) -> Result<()> {
        if template.id.is_empty() {
            return Err(EggsecError::Config(
                "Template ID cannot be empty".to_string(),
            ));
        }

        if template.info.name.is_empty() {
            return Err(EggsecError::Config(
                "Template name cannot be empty".to_string(),
            ));
        }

        let valid_severity = ["critical", "high", "medium", "moderate", "low", "info"];
        if !valid_severity.contains(&template.info.severity.to_lowercase().as_str()) {
            return Err(EggsecError::Config(format!(
                "Invalid severity '{}'. Must be one of: {:?}",
                template.info.severity, valid_severity
            )));
        }

        for matcher in &template.matchers {
            self.validate_matcher(matcher)?;
        }

        Ok(())
    }

    fn validate_matcher(&self, matcher: &super::models::Matcher) -> Result<()> {
        use super::models::Matcher;

        match matcher {
            Matcher::Http(http) => {
                let has_status = !http.status_codes.is_empty();
                let has_header_match = !http.headers.is_empty();
                let has_search = !http.search.is_empty();
                let has_interactsh = http.interactsh.as_ref().map(|i| i.enabled).unwrap_or(false);

                if !(has_status || has_header_match || has_search || has_interactsh) {
                    return Err(EggsecError::Config(
                        "HTTP matcher must define at least one matching condition: \
                         status_codes, headers, search, or enabled interactsh"
                            .to_string(),
                    ));
                }
            }
            Matcher::Dns(dns) => {
                if dns.search.is_empty() {
                    return Err(EggsecError::Config(
                        "DNS matcher must have at least one search pattern".to_string(),
                    ));
                }
            }
            Matcher::Other => {}
        }

        Ok(())
    }

    pub fn load_all(&self) -> Result<Vec<VulnerabilityTemplate>> {
        let mut templates = Vec::new();

        for dir in &self.template_dirs {
            let loaded = self.load_from_directory(dir)?;
            templates.extend(loaded);
        }

        Ok(templates)
    }

    pub fn load_from_directory(&self, dir: &Path) -> Result<Vec<VulnerabilityTemplate>> {
        if !dir.exists() {
            return Err(EggsecError::Config(format!(
                "Template directory does not exist: {}",
                dir.display()
            )));
        }

        let canonical_dir = dir.canonicalize().map_err(|e| {
            EggsecError::Config(format!("Failed to canonicalize directory: {}", e))
        })?;

        let valid_dir = self
            .template_dirs
            .first()
            .map(|d| d.canonicalize())
            .transpose()
            .map_err(|e| {
                EggsecError::Config(format!("Failed to canonicalize base directory: {}", e))
            })?;

        if let Some(ref valid) = valid_dir {
            if !canonical_dir.starts_with(valid) {
                return Err(EggsecError::Config(format!(
                    "Directory {} is not within allowed template directories",
                    dir.display()
                )));
            }
        }

        let mut templates = Vec::new();

        let entries = std::fs::read_dir(dir)
            .map_err(|e| EggsecError::Config(format!("Failed to read directory: {}", e)))?;

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                let sub_templates = self.load_from_directory(&path)?;
                templates.extend(sub_templates);
            } else if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "yaml" || ext_str == "yml" || ext_str == "json" {
                    match self.load_template(&path) {
                        Ok(template) => templates.push(template),
                        Err(e) => {
                            tracing::warn!("Skipping invalid template {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }

        Ok(templates)
    }

    pub fn load_by_id(&self, id: &str) -> Result<Option<VulnerabilityTemplate>> {
        let all_templates = self.load_all()?;

        Ok(all_templates.into_iter().find(|t| t.id == id))
    }

    pub fn load_by_tag(&self, tag: &str) -> Result<Vec<VulnerabilityTemplate>> {
        let all_templates = self.load_all()?;
        let tag_lower = tag.to_lowercase();

        Ok(all_templates
            .into_iter()
            .filter(|t| {
                t.info
                    .tags
                    .iter()
                    .any(|t_tag| t_tag.to_lowercase().contains(&tag_lower))
            })
            .collect())
    }
}

impl Default for TemplateLoader {
    fn default() -> Self {
        Self::new(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::templates::models::TemplateInfo;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_temp_template(content: &str) -> TempDir {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("template.yaml");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        dir
    }

    #[test]
    fn test_load_valid_template() {
        let dir = create_temp_template(
            r#"
id: test-cve
info:
  name: Test Vulnerability
  author: tester
  severity: high
  description: A test vulnerability
  tags:
    - cve
    - test
matchers:
  - type: http
    path: "/"
    search:
      - pattern: "vulnerable"
        mode: word
"#,
        );

        let loader = TemplateLoader::new(vec![dir.path().to_path_buf()]);
        let templates = loader.load_all().unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].id, "test-cve");
    }

    #[test]
    fn test_invalid_severity() {
        let template = VulnerabilityTemplate {
            id: "test".to_string(),
            info: TemplateInfo {
                name: "Test".to_string(),
                author: "test".to_string(),
                severity: "invalid".to_string(),
                description: String::new(),
                tags: vec![],
                references: vec![],
                remediation: String::new(),
            },
            matchers: vec![],
            requests: vec![],
        };

        let loader = TemplateLoader::default();
        assert!(loader.validate_template(&template).is_err());
    }

    #[test]
    fn test_empty_id() {
        let template = VulnerabilityTemplate {
            id: "".to_string(),
            info: TemplateInfo {
                name: "Test".to_string(),
                author: "test".to_string(),
                severity: "high".to_string(),
                description: String::new(),
                tags: vec![],
                references: vec![],
                remediation: String::new(),
            },
            matchers: vec![],
            requests: vec![],
        };

        let loader = TemplateLoader::default();
        assert!(loader.validate_template(&template).is_err());
    }
}
