//! Template models for nuclei-style vulnerability scanning
//!
//! Defines the structure for vulnerability templates including matchers,
//! conditions, and template metadata.

use crate::types::Severity;
use serde::{Deserialize, Serialize};
use rustc_hash::FxHashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilityTemplate {
    pub id: String,
    pub info: TemplateInfo,
    #[serde(default)]
    pub matchers: Vec<Matcher>,
    #[serde(default)]
    pub requests: Vec<TemplateRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    pub name: String,
    pub author: String,
    pub severity: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub references: Vec<String>,
    #[serde(default)]
    pub remediation: String,
}

impl VulnerabilityTemplate {
    pub fn severity(&self) -> Severity {
        match self.info.severity.to_lowercase().as_str() {
            "critical" => Severity::Critical,
            "high" => Severity::High,
            "medium" | "moderate" => Severity::Medium,
            "low" => Severity::Low,
            _ => Severity::Info,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Matcher {
    Http(HttpMatcher),
    Dns(DnsMatcher),
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpMatcher {
    pub path: Option<String>,
    pub method: Option<String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub search: Vec<SearchPattern>,
    #[serde(default)]
    pub status_codes: Vec<u16>,
    #[serde(default)]
    pub interactsh: Option<InteractshConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsMatcher {
    pub query_type: Option<String>,
    #[serde(default)]
    pub search: Vec<SearchPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPattern {
    pub pattern: String,
    #[serde(default)]
    pub mode: MatchMode,
    #[serde(default)]
    pub encoding: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchMode {
    #[default]
    Word,
    Regex,
    Binary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractshConfig {
    pub enabled: bool,
    #[serde(default)]
    pub authorization: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateRequest {
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub raw: Option<String>,
}

impl Default for TemplateRequest {
    fn default() -> Self {
        Self {
            method: "GET".to_string(),
            path: "/".to_string(),
            headers: HashMap::new(),
            body: None,
            raw: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_parsing() {
        let yaml = r#"
id: test-template
info:
  name: Test Template
  author: test
  severity: high
matchers:
  - type: http
    path: "/"
    search:
      - pattern: "vulnerable"
        mode: word
"#;
        let template: VulnerabilityTemplate = serde_yaml_neo::from_str(yaml).unwrap();
        assert_eq!(template.id, "test-template");
        assert_eq!(template.info.severity, "high");
        assert!(matches!(template.severity(), Severity::High));
    }

    #[test]
    fn test_severity_mapping() {
        let mut template = VulnerabilityTemplate {
            id: "test".to_string(),
            info: TemplateInfo {
                name: "Test".to_string(),
                author: "test".to_string(),
                severity: "critical".to_string(),
                description: String::new(),
                tags: vec![],
                references: vec![],
                remediation: String::new(),
            },
            matchers: vec![],
            requests: vec![],
        };

        assert!(matches!(template.severity(), Severity::Critical));

        template.info.severity = "low".to_string();
        assert!(matches!(template.severity(), Severity::Low));
    }
}
