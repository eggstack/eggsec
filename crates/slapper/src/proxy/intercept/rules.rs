//! Request/response modification rules
//!
//! Defines rules for intercepting and modifying HTTP traffic.

use crate::error::Result;
use serde::{Deserialize, Serialize};
use rustc_hash::FxHashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    Allow,
    Block,
    Intercept,
    Monitor,
    Modify,
}

impl Default for RuleAction {
    fn default() -> Self {
        Self::Allow
    }
}

#[derive(Debug, Clone)]
pub struct InterceptRule {
    pub host_pattern: String,
    pub path_pattern: Option<String>,
    pub action: RuleAction,
    pub request_modifications: Vec<RequestModification>,
    pub response_modifications: Vec<ResponseModification>,
    pub priority: u32,
}

impl InterceptRule {
    pub fn new(host_pattern: String, path_pattern: Option<String>, action: RuleAction) -> Self {
        Self {
            host_pattern,
            path_pattern,
            action,
            request_modifications: Vec::new(),
            response_modifications: Vec::new(),
            priority: 0,
        }
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn add_request_modification(&mut self, mod_req: RequestModification) {
        self.request_modifications.push(mod_req);
    }

    pub fn add_response_modification(&mut self, mod_resp: ResponseModification) {
        self.response_modifications.push(mod_resp);
    }

    pub fn matches(&self, host: &str, path: &str) -> bool {
        if !self.host_matches(host) {
            return false;
        }

        if let Some(ref path_pattern) = self.path_pattern {
            self.path_matches(path, path_pattern)
        } else {
            true
        }
    }

    fn host_matches(&self, host: &str) -> bool {
        if self.host_pattern == "*" {
            return true;
        }

        if self.host_pattern.starts_with("*.") {
            let suffix = &self.host_pattern[2..];
            host.ends_with(suffix) || host == suffix
        } else {
            host == self.host_pattern
        }
    }

    fn path_matches(&self, path: &str, pattern: &str) -> bool {
        if pattern == "*" || pattern == "/*" {
            return true;
        }

        if pattern.ends_with("/*") {
            let prefix = &pattern[..pattern.len() - 2];
            path.starts_with(prefix)
        } else if pattern.ends_with("**") {
            let prefix = &pattern[..pattern.len() - 2];
            path.starts_with(prefix)
        } else {
            path == pattern
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RuleSet {
    rules: Vec<InterceptRule>,
}

impl RuleSet {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add(&mut self, rule: InterceptRule) {
        self.rules.push(rule);
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub fn evaluate(&self, host: &str, path: &str, _content: &str) -> RuleAction {
        for rule in &self.rules {
            if rule.matches(host, path) {
                return rule.action.clone();
            }
        }

        RuleAction::Allow
    }

    pub fn get_request_modifications(&self, host: &str, path: &str) -> Vec<RequestModification> {
        self.rules
            .iter()
            .filter(|r| r.matches(host, path))
            .flat_map(|r| r.request_modifications.clone())
            .collect()
    }

    pub fn get_response_modifications(&self, host: &str, path: &str) -> Vec<ResponseModification> {
        self.rules
            .iter()
            .filter(|r| r.matches(host, path))
            .flat_map(|r| r.response_modifications.clone())
            .collect()
    }

    pub fn remove(&mut self, host_pattern: &str, path_pattern: Option<&str>) {
        self.rules.retain(|r| {
            r.host_pattern != host_pattern
                || r.path_pattern.as_deref() != path_pattern
        });
    }

    pub fn clear(&mut self) {
        self.rules.clear();
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

impl Default for RuleSet {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct RequestModification {
    pub header_name: Option<String>,
    pub header_value: Option<String>,
    pub new_path: Option<String>,
    pub new_body: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResponseModification {
    pub header_name: Option<String>,
    pub header_value: Option<String>,
    pub new_body: Option<String>,
    pub new_status: Option<u16>,
}

pub fn parse_rule_from_yaml(yaml: &str) -> Result<InterceptRule> {
    #[derive(Deserialize)]
    struct YamlRule {
        host: String,
        path: Option<String>,
        action: String,
        priority: Option<u32>,
    }

    let parsed: YamlRule = serde_yaml_neo::from_str(yaml)
        .map_err(|e| crate::error::SlapperError::Config(format!("Invalid rule YAML: {}", e)))?;

    let action = match parsed.action.to_lowercase().as_str() {
        "allow" => RuleAction::Allow,
        "block" => RuleAction::Block,
        "intercept" => RuleAction::Intercept,
        "monitor" => RuleAction::Monitor,
        "modify" => RuleAction::Modify,
        _ => return Err(crate::error::SlapperError::Config(format!(
            "Unknown action: {}",
            parsed.action
        ))),
    };

    let mut rule = InterceptRule::new(parsed.host, parsed.path, action);

    if let Some(priority) = parsed.priority {
        rule = rule.with_priority(priority);
    }

    Ok(rule)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_matching() {
        let rule = InterceptRule::new(
            "example.com".to_string(),
            Some("/admin/*".to_string()),
            RuleAction::Block,
        );

        assert!(rule.matches("example.com", "/admin/panel"));
        assert!(!rule.matches("example.com", "/public/panel"));
        assert!(!rule.matches("other.com", "/admin/panel"));
    }

    fn test_wildcard_host_matching() {
        let rule = InterceptRule::new(
            "*.example.com".to_string(),
            None,
            RuleAction::Monitor,
        );

        assert!(rule.matches("api.example.com", "/any/path"));
        assert!(rule.matches("example.com", "/any/path"));
        assert!(!rule.matches("other.com", "/any/path"));
    }

    #[test]
    fn test_rule_set_evaluation() {
        let mut rules = RuleSet::new();

        rules.add(InterceptRule::new(
            "evil.com".to_string(),
            None,
            RuleAction::Block,
        ));

        rules.add(InterceptRule::new(
            "example.com".to_string(),
            Some("/admin/*".to_string()),
            RuleAction::Intercept,
        ));

        assert!(matches!(rules.evaluate("evil.com", "/any", ""), RuleAction::Block));
        assert!(matches!(rules.evaluate("example.com", "/admin/panel", ""), RuleAction::Intercept));
        assert!(matches!(rules.evaluate("example.com", "/public", ""), RuleAction::Allow));
    }

    #[test]
    fn test_parse_rule() {
        let yaml = r#"
host: example.com
path: /admin/*
action: intercept
priority: 100
"#;

        let rule = parse_rule_from_yaml(yaml).unwrap();
        assert_eq!(rule.host_pattern, "example.com");
        assert!(matches!(rule.action, RuleAction::Intercept));
        assert_eq!(rule.priority, 100);
    }
}
