//! Request/response modification rules
//!
//! Defines rules for intercepting and modifying HTTP traffic.
//!
//! ## Modification Types
//!
//! This module defines rule-based [`RequestModification`] and [`ResponseModification`]
//! types used in YAML configuration for declarative rule modifications. These differ
//! from the runtime modification types in [`super::interceptor`] which use `FxHashMap`
//! for in-memory modifications during request/response processing.

use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    #[default]
    Allow,
    Block,
    Intercept,
    Monitor,
    Modify,
}

#[derive(Debug, Clone)]
pub struct InterceptRule {
    pub host_pattern: String,
    pub path_pattern: Option<String>,
    pub method_pattern: Option<String>,
    pub header_name: Option<String>,
    pub header_value_pattern: Option<String>,
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
            method_pattern: None,
            header_name: None,
            header_value_pattern: None,
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

    pub fn with_method(mut self, method: String) -> Self {
        self.method_pattern = Some(method);
        self
    }

    pub fn with_header(mut self, name: String, value_pattern: Option<String>) -> Self {
        self.header_name = Some(name);
        self.header_value_pattern = value_pattern;
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
            if !self.path_matches(path, path_pattern) {
                return false;
            }
        }

        true
    }

    /// Check if this rule matches a full request context.
    pub fn matches_request(&self, host: &str, path: &str, method: &str, headers: &std::collections::HashMap<String, String>) -> bool {
        if !self.matches(host, path) {
            return false;
        }

        if let Some(ref method_pattern) = self.method_pattern {
            if !method.eq_ignore_ascii_case(method_pattern) {
                return false;
            }
        }

        if let Some(ref header_name) = self.header_name {
            let header_value = headers.get(header_name).map(|v| v.as_str()).unwrap_or("");
            if let Some(ref value_pattern) = self.header_value_pattern {
                if !header_value.contains(value_pattern.as_str()) {
                    return false;
                }
            }
        }

        true
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

        if let Some(prefix) = pattern
            .strip_suffix("/*")
            .or_else(|| pattern.strip_suffix("**"))
        {
            path.starts_with(prefix)
        } else {
            path == pattern
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuleSet {
    rules: Vec<InterceptRule>,
}

impl RuleSet {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add(&mut self, rule: InterceptRule) {
        self.rules.push(rule);
        self.rules.sort_by_key(|b| std::cmp::Reverse(b.priority));
    }

    pub fn evaluate(&self, host: &str, path: &str) -> RuleAction {
        for rule in &self.rules {
            if rule.matches(host, path) {
                return rule.action;
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
            r.host_pattern != host_pattern || r.path_pattern.as_deref() != path_pattern
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
#[non_exhaustive]
pub struct RequestModification {
    pub header_name: Option<String>,
    pub header_value: Option<String>,
    pub new_path: Option<String>,
    pub new_body: Option<String>,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ResponseModification {
    pub header_name: Option<String>,
    pub header_value: Option<String>,
    pub new_body: Option<String>,
    pub new_status: Option<u16>,
}

#[allow(dead_code)]
pub fn parse_rule_from_yaml(yaml: &str) -> Result<InterceptRule> {
    #[derive(Deserialize)]
    struct YamlRule {
        host: String,
        path: Option<String>,
        method: Option<String>,
        header_name: Option<String>,
        header_value: Option<String>,
        action: String,
        priority: Option<u32>,
    }

    let parsed: YamlRule = serde_yaml_neo::from_str(yaml)
        .map_err(|e| crate::error::EggsecError::Config(format!("Invalid rule YAML: {}", e)))?;

    let action = match parsed.action.to_lowercase().as_str() {
        "allow" => RuleAction::Allow,
        "block" => RuleAction::Block,
        "intercept" => RuleAction::Intercept,
        "monitor" => RuleAction::Monitor,
        "modify" => RuleAction::Modify,
        _ => {
            return Err(crate::error::EggsecError::Config(format!(
                "Unknown action: {}",
                parsed.action
            )))
        }
    };

    let mut rule = InterceptRule::new(parsed.host, parsed.path, action);

    if let Some(priority) = parsed.priority {
        rule = rule.with_priority(priority);
    }

    if let Some(method) = parsed.method {
        rule = rule.with_method(method);
    }
    if let Some(header_name) = parsed.header_name {
        rule = rule.with_header(header_name, parsed.header_value);
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

    #[test]
    fn test_wildcard_host_matching() {
        let rule = InterceptRule::new("*.example.com".to_string(), None, RuleAction::Monitor);

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

        assert!(matches!(
            rules.evaluate("evil.com", "/any"),
            RuleAction::Block
        ));
        assert!(matches!(
            rules.evaluate("example.com", "/admin/panel"),
            RuleAction::Intercept
        ));
        assert!(matches!(
            rules.evaluate("example.com", "/public"),
            RuleAction::Allow
        ));
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

    #[test]
    fn test_method_matching() {
        let rule = InterceptRule::new(
            "example.com".to_string(),
            None,
            RuleAction::Intercept,
        ).with_method("POST".to_string());

        let headers = std::collections::HashMap::new();
        assert!(rule.matches_request("example.com", "/", "POST", &headers));
        assert!(!rule.matches_request("example.com", "/", "GET", &headers));
        assert!(rule.matches("example.com", "/")); // base match still works
    }

    #[test]
    fn test_header_matching() {
        let rule = InterceptRule::new(
            "example.com".to_string(),
            None,
            RuleAction::Intercept,
        ).with_header("Authorization".to_string(), Some("Bearer".to_string()));

        let mut headers = std::collections::HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token123".to_string());
        assert!(rule.matches_request("example.com", "/", "GET", &headers));

        let mut headers2 = std::collections::HashMap::new();
        headers2.insert("Authorization".to_string(), "Basic abc".to_string());
        assert!(!rule.matches_request("example.com", "/", "GET", &headers2));
    }
}
