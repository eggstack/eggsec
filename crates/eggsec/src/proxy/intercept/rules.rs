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
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    #[default]
    Allow,
    Block,
    Intercept,
    Monitor,
    Modify,
    InjectResponse,
    Delay,
    Tag,
}

/// A rule identifier for tracking and referencing rules.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuleId(pub String);

impl RuleId {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Condition for matching against rule context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum RuleCondition {
    HostMatches(String),
    PathMatches(String),
    MethodMatches(String),
    HeaderContains(String, String),
    BodyContains(String),
    ProtocolIs(String),
    WebSocketOpcodeIs(String),
    GrpcMethodIs(String),
    And(Vec<RuleCondition>),
    Or(Vec<RuleCondition>),
    Not(Box<RuleCondition>),
    BodySizeGt(u64),
    BodySizeLt(u64),
}

impl RuleCondition {
    pub fn evaluate(&self, ctx: &RuleContext) -> bool {
        match self {
            Self::HostMatches(pattern) => host_matches_pattern(&ctx.host, pattern),
            Self::PathMatches(pattern) => path_matches_pattern(&ctx.path, pattern),
            Self::MethodMatches(method) => ctx.method.eq_ignore_ascii_case(method),
            Self::HeaderContains(name, value) => ctx
                .headers
                .get(name)
                .map(|v| v.contains(value.as_str()))
                .unwrap_or(false),
            Self::BodyContains(text) => ctx
                .body
                .as_ref()
                .map(|b| b.contains(text.as_str()))
                .unwrap_or(false),
            Self::ProtocolIs(protocol) => ctx.protocol.eq_ignore_ascii_case(protocol),
            Self::WebSocketOpcodeIs(opcode) => ctx
                .ws_opcode
                .as_ref()
                .map(|o| o.eq_ignore_ascii_case(opcode))
                .unwrap_or(false),
            Self::GrpcMethodIs(method) => ctx
                .grpc_method
                .as_ref()
                .map(|m| m.eq_ignore_ascii_case(method))
                .unwrap_or(false),
            Self::And(conditions) => conditions.iter().all(|c| c.evaluate(ctx)),
            Self::Or(conditions) => conditions.iter().any(|c| c.evaluate(ctx)),
            Self::Not(inner) => !inner.evaluate(ctx),
            Self::BodySizeGt(size) => ctx.body_size.map(|s| s > *size).unwrap_or(false),
            Self::BodySizeLt(size) => ctx.body_size.map(|s| s < *size).unwrap_or(false),
        }
    }
}

/// Configuration for inject-response rule action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectResponseConfig {
    pub status: u16,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

/// Context provided to rules for evaluation.
#[derive(Debug, Clone, Default)]
pub struct RuleContext {
    pub host: String,
    pub path: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub body_size: Option<u64>,
    pub protocol: String,
    pub ws_opcode: Option<String>,
    pub grpc_method: Option<String>,
}

impl RuleContext {
    pub fn new(host: &str, path: &str, method: &str) -> Self {
        Self {
            host: host.to_string(),
            path: path.to_string(),
            method: method.to_string(),
            ..Default::default()
        }
    }
}

/// An enhanced rule with complex conditions, IDs, and additional actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedRule {
    pub id: RuleId,
    pub name: String,
    pub description: Option<String>,
    pub condition: RuleCondition,
    pub action: RuleAction,
    pub priority: u32,
    pub enabled: bool,
    #[serde(default)]
    pub request_modifications: Vec<RequestModification>,
    #[serde(default)]
    pub response_modifications: Vec<ResponseModification>,
    pub inject_response: Option<InjectResponseConfig>,
    pub delay_ms: Option<u64>,
    pub tag: Option<String>,
}

impl EnhancedRule {
    pub fn new(id: &str, name: &str, condition: RuleCondition, action: RuleAction) -> Self {
        Self {
            id: RuleId::new(id),
            name: name.to_string(),
            description: None,
            condition,
            action,
            priority: 0,
            enabled: true,
            request_modifications: Vec::new(),
            response_modifications: Vec::new(),
            inject_response: None,
            delay_ms: None,
            tag: None,
        }
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn with_inject_response(mut self, config: InjectResponseConfig) -> Self {
        self.inject_response = Some(config);
        self
    }

    pub fn with_delay(mut self, ms: u64) -> Self {
        self.delay_ms = Some(ms);
        self
    }

    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tag = Some(tag.to_string());
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn evaluate(&self, context: &RuleContext) -> bool {
        self.enabled && self.condition.evaluate(context)
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// A collection of enhanced rules with persistence and complex evaluation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnhancedRuleSet {
    rules: Vec<EnhancedRule>,
    /// Index for fast host prefix lookup: maps host prefix to rule indices.
    host_prefix_index: HashMap<String, Vec<usize>>,
    /// Index for fast path prefix lookup: maps path prefix to rule indices.
    path_prefix_index: HashMap<String, Vec<usize>>,
}

impl EnhancedRuleSet {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            host_prefix_index: HashMap::new(),
            path_prefix_index: HashMap::new(),
        }
    }

    pub fn add(&mut self, rule: EnhancedRule) {
        let _idx = self.rules.len();
        self.rules.push(rule);
        self.rules.sort_by_key(|r| std::cmp::Reverse(r.priority));
        self.rebuild_index();
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let len_before = self.rules.len();
        self.rules.retain(|r| r.id.as_str() != id);
        if self.rules.len() < len_before {
            self.rebuild_index();
            true
        } else {
            false
        }
    }

    /// Rebuild the host and path prefix indices from the current rule set.
    fn rebuild_index(&mut self) {
        self.host_prefix_index.clear();
        self.path_prefix_index.clear();

        for (idx, rule) in self.rules.iter().enumerate() {
            if !rule.enabled {
                continue;
            }
            extract_prefixes(
                &rule.condition,
                &mut self.host_prefix_index,
                &mut self.path_prefix_index,
                idx,
            );
        }
    }

    pub fn evaluate(&self, context: &RuleContext) -> Vec<&EnhancedRule> {
        self.rules
            .iter()
            .filter(|r| r.enabled && r.evaluate(context))
            .collect()
    }

    pub fn evaluate_first(&self, context: &RuleContext) -> Option<&EnhancedRule> {
        self.rules.iter().find(|r| r.enabled && r.evaluate(context))
    }

    /// Evaluate rules using the prefix index for fast candidate selection.
    ///
    /// Returns rules that match based on the host and path prefixes,
    /// then applies full condition evaluation to filter the candidates.
    pub fn evaluate_indexed(&self, context: &RuleContext) -> Vec<&EnhancedRule> {
        let mut candidate_indices = rustc_hash::FxHashSet::default();

        // Find candidates by host prefix
        for (prefix, indices) in &self.host_prefix_index {
            if context.host.starts_with(prefix.as_str()) || context.host.contains(prefix.as_str()) {
                for &idx in indices {
                    candidate_indices.insert(idx);
                }
            }
        }

        // Find candidates by path prefix
        for (prefix, indices) in &self.path_prefix_index {
            if context.path.starts_with(prefix.as_str()) || context.path.contains(prefix.as_str()) {
                for &idx in indices {
                    candidate_indices.insert(idx);
                }
            }
        }

        // If no prefix matches found, fall back to full scan
        if candidate_indices.is_empty() {
            return self.evaluate(context);
        }

        // Evaluate full conditions on candidates
        candidate_indices
            .iter()
            .filter_map(|&idx| {
                let rule = &self.rules[idx];
                if rule.enabled && rule.evaluate(context) {
                    Some(rule)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Evaluate rules asynchronously using spawn_blocking for CPU-intensive conditions.
    ///
    /// This offloads regex-heavy condition evaluation to a blocking thread pool,
    /// preventing async tasks from blocking the event loop.
    pub async fn evaluate_async(&self, context: RuleContext) -> Vec<EnhancedRule> {
        let rules: Vec<EnhancedRule> = self.rules.to_vec();

        tokio::task::spawn_blocking(move || {
            rules
                .iter()
                .filter(|r| r.enabled && r.evaluate(&context))
                .cloned()
                .collect()
        })
        .await
        .unwrap_or_default()
    }

    /// Evaluate rules asynchronously using indexed lookup for fast candidate selection.
    ///
    /// Combines prefix indexing with async evaluation for optimal performance
    /// on large rule sets with CPU-intensive conditions.
    pub async fn evaluate_indexed_async(&self, context: RuleContext) -> Vec<EnhancedRule> {
        let candidate_indices: Vec<usize> = {
            let mut indices = rustc_hash::FxHashSet::default();

            for (prefix, idxs) in &self.host_prefix_index {
                if context.host.starts_with(prefix.as_str())
                    || context.host.contains(prefix.as_str())
                {
                    for &idx in idxs {
                        indices.insert(idx);
                    }
                }
            }

            for (prefix, idxs) in &self.path_prefix_index {
                if context.path.starts_with(prefix.as_str())
                    || context.path.contains(prefix.as_str())
                {
                    for &idx in idxs {
                        indices.insert(idx);
                    }
                }
            }

            if indices.is_empty() {
                // Fall back to all rules
                (0..self.rules.len()).collect()
            } else {
                indices.into_iter().collect()
            }
        };

        let rules: Vec<EnhancedRule> = candidate_indices
            .iter()
            .map(|&idx| self.rules[idx].clone())
            .collect();

        tokio::task::spawn_blocking(move || {
            rules
                .iter()
                .filter(|r| r.enabled && r.evaluate(&context))
                .cloned()
                .collect()
        })
        .await
        .unwrap_or_default()
    }

    pub fn get_by_id(&self, id: &str) -> Option<&EnhancedRule> {
        self.rules.iter().find(|r| r.id.as_str() == id)
    }

    pub fn enable(&mut self, id: &str) -> bool {
        if let Some(rule) = self.rules.iter_mut().find(|r| r.id.as_str() == id) {
            rule.enabled = true;
            true
        } else {
            false
        }
    }

    pub fn disable(&mut self, id: &str) -> bool {
        if let Some(rule) = self.rules.iter_mut().find(|r| r.id.as_str() == id) {
            rule.enabled = false;
            true
        } else {
            false
        }
    }

    pub fn get_tags(&self) -> Vec<&str> {
        self.rules.iter().filter_map(|r| r.tag.as_deref()).collect()
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            crate::error::EggsecError::Proxy(format!("Failed to serialize rules: {}", e))
        })?;
        std::fs::write(path, json).map_err(|e| {
            crate::error::EggsecError::Proxy(format!("Failed to write rules file: {}", e))
        })?;
        Ok(())
    }

    pub fn load_from_file(path: &str) -> Result<Self> {
        let json = std::fs::read_to_string(path).map_err(|e| {
            crate::error::EggsecError::Proxy(format!("Failed to read rules file: {}", e))
        })?;
        serde_json::from_str(&json).map_err(|e| {
            crate::error::EggsecError::Proxy(format!("Failed to deserialize rules: {}", e))
        })
    }

    pub fn export_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| crate::error::EggsecError::Proxy(format!("Failed to export rules: {}", e)))
    }

    pub fn import_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| crate::error::EggsecError::Proxy(format!("Failed to import rules: {}", e)))
    }
}

/// Extract host and path prefixes from a rule condition for indexing.
///
/// This function recursively walks the condition tree and extracts
/// simple string prefixes that can be used for fast candidate selection.
fn extract_prefixes(
    condition: &RuleCondition,
    host_index: &mut HashMap<String, Vec<usize>>,
    path_index: &mut HashMap<String, Vec<usize>>,
    rule_idx: usize,
) {
    match condition {
        RuleCondition::HostMatches(pattern) => {
            // Extract the prefix before any wildcard
            let prefix = pattern.split('*').next().unwrap_or(pattern).to_string();
            if !prefix.is_empty() {
                host_index.entry(prefix).or_default().push(rule_idx);
            }
        }
        RuleCondition::PathMatches(pattern) => {
            // Extract the prefix before any wildcard
            let prefix = pattern.split('*').next().unwrap_or(pattern).to_string();
            if !prefix.is_empty() {
                path_index.entry(prefix).or_default().push(rule_idx);
            }
        }
        RuleCondition::And(conditions) | RuleCondition::Or(conditions) => {
            for c in conditions {
                extract_prefixes(c, host_index, path_index, rule_idx);
            }
        }
        RuleCondition::Not(inner) => {
            extract_prefixes(inner, host_index, path_index, rule_idx);
        }
        _ => {}
    }
}

// ==================== Legacy types (backward compatible) ====================

#[derive(Debug, Clone)]
pub struct InterceptRule {
    pub id: Option<String>,
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
            id: None,
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

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = Some(id.to_string());
        self
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
    pub fn matches_request(
        &self,
        host: &str,
        path: &str,
        method: &str,
        headers: &std::collections::HashMap<String, String>,
    ) -> bool {
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
        host_matches_pattern(host, &self.host_pattern)
    }

    fn path_matches(&self, path: &str, pattern: &str) -> bool {
        path_matches_pattern(path, pattern)
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RequestModification {
    pub header_name: Option<String>,
    pub header_value: Option<String>,
    pub new_path: Option<String>,
    pub new_body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ResponseModification {
    pub header_name: Option<String>,
    pub header_value: Option<String>,
    pub new_body: Option<String>,
    pub new_status: Option<u16>,
}

/// Shared host matching logic.
fn host_matches_pattern(host: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        host.ends_with(suffix) || host == suffix
    } else {
        host == pattern
    }
}

/// Shared path matching logic.
fn path_matches_pattern(path: &str, pattern: &str) -> bool {
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
        id: Option<String>,
    }

    let parsed: YamlRule = serde_yaml_neo::from_str(yaml)
        .map_err(|e| crate::error::EggsecError::Config(format!("Invalid rule YAML: {}", e)))?;

    let action = match parsed.action.to_lowercase().as_str() {
        "allow" => RuleAction::Allow,
        "block" => RuleAction::Block,
        "intercept" => RuleAction::Intercept,
        "monitor" => RuleAction::Monitor,
        "modify" => RuleAction::Modify,
        "inject_response" | "inject-response" => RuleAction::InjectResponse,
        "delay" => RuleAction::Delay,
        "tag" => RuleAction::Tag,
        _ => {
            return Err(crate::error::EggsecError::Config(format!(
                "Unknown action: {}",
                parsed.action
            )))
        }
    };

    let mut rule = InterceptRule::new(parsed.host, parsed.path, action);

    if let Some(id) = parsed.id {
        rule = rule.with_id(&id);
    }

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

    // ==================== Legacy tests ====================

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
        let rule = InterceptRule::new("example.com".to_string(), None, RuleAction::Intercept)
            .with_method("POST".to_string());

        let headers = std::collections::HashMap::new();
        assert!(rule.matches_request("example.com", "/", "POST", &headers));
        assert!(!rule.matches_request("example.com", "/", "GET", &headers));
        assert!(rule.matches("example.com", "/"));
    }

    #[test]
    fn test_header_matching() {
        let rule = InterceptRule::new("example.com".to_string(), None, RuleAction::Intercept)
            .with_header("Authorization".to_string(), Some("Bearer".to_string()));

        let mut headers = std::collections::HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token123".to_string());
        assert!(rule.matches_request("example.com", "/", "GET", &headers));

        let mut headers2 = std::collections::HashMap::new();
        headers2.insert("Authorization".to_string(), "Basic abc".to_string());
        assert!(!rule.matches_request("example.com", "/", "GET", &headers2));
    }

    #[test]
    fn test_intercept_rule_with_id() {
        let rule = InterceptRule::new("example.com".to_string(), None, RuleAction::Block)
            .with_id("block-evil");
        assert_eq!(rule.id.as_deref(), Some("block-evil"));
    }

    #[test]
    fn test_parse_rule_with_id() {
        let yaml = r#"
host: example.com
action: block
id: my-rule
priority: 50
"#;
        let rule = parse_rule_from_yaml(yaml).unwrap();
        assert_eq!(rule.id.as_deref(), Some("my-rule"));
    }

    #[test]
    fn test_parse_rule_inject_response_action() {
        let yaml = r#"
host: example.com
action: inject_response
"#;
        let rule = parse_rule_from_yaml(yaml).unwrap();
        assert!(matches!(rule.action, RuleAction::InjectResponse));
    }

    // ==================== RuleCondition tests ====================

    #[test]
    fn test_condition_host_matches() {
        let cond = RuleCondition::HostMatches("example.com".to_string());
        let ctx = RuleContext::new("example.com", "/", "GET");
        assert!(cond.evaluate(&ctx));

        let ctx2 = RuleContext::new("other.com", "/", "GET");
        assert!(!cond.evaluate(&ctx2));
    }

    #[test]
    fn test_condition_host_wildcard() {
        let cond = RuleCondition::HostMatches("*.example.com".to_string());
        let ctx = RuleContext::new("api.example.com", "/", "GET");
        assert!(cond.evaluate(&ctx));
    }

    #[test]
    fn test_condition_path_matches() {
        let cond = RuleCondition::PathMatches("/admin/*".to_string());
        let ctx = RuleContext::new("example.com", "/admin/panel", "GET");
        assert!(cond.evaluate(&ctx));

        let ctx2 = RuleContext::new("example.com", "/public/page", "GET");
        assert!(!cond.evaluate(&ctx2));
    }

    #[test]
    fn test_condition_method_matches() {
        let cond = RuleCondition::MethodMatches("POST".to_string());
        let ctx = RuleContext::new("example.com", "/", "POST");
        assert!(cond.evaluate(&ctx));

        let ctx2 = RuleContext::new("example.com", "/", "GET");
        assert!(!cond.evaluate(&ctx2));
    }

    #[test]
    fn test_condition_header_contains() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token123".to_string());
        let cond = RuleCondition::HeaderContains("Authorization".to_string(), "Bearer".to_string());
        let mut ctx = RuleContext::new("example.com", "/", "GET");
        ctx.headers = headers;
        assert!(cond.evaluate(&ctx));
    }

    #[test]
    fn test_condition_body_contains() {
        let cond = RuleCondition::BodyContains("password".to_string());
        let mut ctx = RuleContext::new("example.com", "/", "POST");
        ctx.body = Some("user=admin&password=secret".to_string());
        assert!(cond.evaluate(&ctx));

        let ctx2 = RuleContext::new("example.com", "/", "POST");
        assert!(!cond.evaluate(&ctx2));
    }

    #[test]
    fn test_condition_protocol_is() {
        let cond = RuleCondition::ProtocolIs("websocket".to_string());
        let mut ctx = RuleContext::new("example.com", "/", "GET");
        ctx.protocol = "websocket".to_string();
        assert!(cond.evaluate(&ctx));
    }

    #[test]
    fn test_condition_ws_opcode() {
        let cond = RuleCondition::WebSocketOpcodeIs("text".to_string());
        let mut ctx = RuleContext::new("example.com", "/ws", "GET");
        ctx.ws_opcode = Some("text".to_string());
        assert!(cond.evaluate(&ctx));

        let mut ctx2 = RuleContext::new("example.com", "/ws", "GET");
        ctx2.ws_opcode = Some("binary".to_string());
        assert!(!cond.evaluate(&ctx2));
    }

    #[test]
    fn test_condition_grpc_method() {
        let cond = RuleCondition::GrpcMethodIs("unary".to_string());
        let mut ctx = RuleContext::new("example.com", "/pkg.Svc/Method", "POST");
        ctx.grpc_method = Some("unary".to_string());
        assert!(cond.evaluate(&ctx));
    }

    #[test]
    fn test_condition_and() {
        let cond = RuleCondition::And(vec![
            RuleCondition::HostMatches("example.com".to_string()),
            RuleCondition::MethodMatches("POST".to_string()),
        ]);
        let ctx = RuleContext::new("example.com", "/", "POST");
        assert!(cond.evaluate(&ctx));

        let ctx2 = RuleContext::new("example.com", "/", "GET");
        assert!(!cond.evaluate(&ctx2));
    }

    #[test]
    fn test_condition_or() {
        let cond = RuleCondition::Or(vec![
            RuleCondition::HostMatches("example.com".to_string()),
            RuleCondition::HostMatches("other.com".to_string()),
        ]);
        let ctx = RuleContext::new("example.com", "/", "GET");
        assert!(cond.evaluate(&ctx));

        let ctx2 = RuleContext::new("other.com", "/", "GET");
        assert!(cond.evaluate(&ctx2));

        let ctx3 = RuleContext::new("third.com", "/", "GET");
        assert!(!cond.evaluate(&ctx3));
    }

    #[test]
    fn test_condition_not() {
        let cond = RuleCondition::Not(Box::new(RuleCondition::HostMatches("evil.com".to_string())));
        let ctx = RuleContext::new("good.com", "/", "GET");
        assert!(cond.evaluate(&ctx));

        let ctx2 = RuleContext::new("evil.com", "/", "GET");
        assert!(!cond.evaluate(&ctx2));
    }

    #[test]
    fn test_condition_body_size() {
        let gt = RuleCondition::BodySizeGt(100);
        let lt = RuleCondition::BodySizeLt(100);

        let mut ctx = RuleContext::new("example.com", "/", "POST");
        ctx.body_size = Some(200);
        assert!(gt.evaluate(&ctx));
        assert!(!lt.evaluate(&ctx));

        let mut ctx2 = RuleContext::new("example.com", "/", "POST");
        ctx2.body_size = Some(50);
        assert!(!gt.evaluate(&ctx2));
        assert!(lt.evaluate(&ctx2));

        // No body size should return false
        let ctx3 = RuleContext::new("example.com", "/", "POST");
        assert!(!gt.evaluate(&ctx3));
        assert!(!lt.evaluate(&ctx3));
    }

    // ==================== EnhancedRule tests ====================

    #[test]
    fn test_enhanced_rule_new() {
        let rule = EnhancedRule::new(
            "r1",
            "Block evil",
            RuleCondition::HostMatches("evil.com".to_string()),
            RuleAction::Block,
        );
        assert_eq!(rule.id.as_str(), "r1");
        assert_eq!(rule.name, "Block evil");
        assert!(rule.enabled);
        assert_eq!(rule.priority, 0);
    }

    #[test]
    fn test_enhanced_rule_builder() {
        let rule = EnhancedRule::new(
            "r2",
            "Tag API",
            RuleCondition::HostMatches("api.example.com".to_string()),
            RuleAction::Tag,
        )
        .with_priority(100)
        .with_description("Tags API traffic")
        .with_delay(500)
        .with_tag("api-monitor")
        .with_enabled(true);
        assert_eq!(rule.priority, 100);
        assert_eq!(rule.description.as_deref(), Some("Tags API traffic"));
        assert_eq!(rule.delay_ms, Some(500));
        assert_eq!(rule.tag.as_deref(), Some("api-monitor"));
    }

    #[test]
    fn test_enhanced_rule_evaluate() {
        let rule = EnhancedRule::new(
            "r3",
            "Block POST",
            RuleCondition::And(vec![
                RuleCondition::HostMatches("example.com".to_string()),
                RuleCondition::MethodMatches("POST".to_string()),
            ]),
            RuleAction::Block,
        );
        let ctx = RuleContext::new("example.com", "/", "POST");
        assert!(rule.evaluate(&ctx));

        let ctx2 = RuleContext::new("example.com", "/", "GET");
        assert!(!rule.evaluate(&ctx2));
    }

    #[test]
    fn test_enhanced_rule_disabled() {
        let rule = EnhancedRule::new(
            "r4",
            "Disabled rule",
            RuleCondition::HostMatches("example.com".to_string()),
            RuleAction::Block,
        )
        .with_enabled(false);
        let ctx = RuleContext::new("example.com", "/", "GET");
        assert!(!rule.evaluate(&ctx));
        assert!(!rule.is_enabled());
    }

    // ==================== EnhancedRuleSet tests ====================

    #[test]
    fn test_enhanced_rule_set_add_remove() {
        let mut rules = EnhancedRuleSet::new();
        rules.add(EnhancedRule::new(
            "r1",
            "Rule 1",
            RuleCondition::HostMatches("a.com".to_string()),
            RuleAction::Block,
        ));
        rules.add(EnhancedRule::new(
            "r2",
            "Rule 2",
            RuleCondition::HostMatches("b.com".to_string()),
            RuleAction::Intercept,
        ));
        assert_eq!(rules.len(), 2);

        assert!(rules.remove("r1"));
        assert_eq!(rules.len(), 1);
        assert!(!rules.remove("nonexistent"));
    }

    #[test]
    fn test_enhanced_rule_set_evaluate() {
        let mut rules = EnhancedRuleSet::new();
        rules.add(EnhancedRule::new(
            "r1",
            "Block a.com",
            RuleCondition::HostMatches("a.com".to_string()),
            RuleAction::Block,
        ));
        rules.add(EnhancedRule::new(
            "r2",
            "Intercept /admin",
            RuleCondition::And(vec![
                RuleCondition::HostMatches("b.com".to_string()),
                RuleCondition::PathMatches("/admin/*".to_string()),
            ]),
            RuleAction::Intercept,
        ));

        let ctx = RuleContext::new("a.com", "/", "GET");
        let matches = rules.evaluate(&ctx);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].id.as_str(), "r1");

        let ctx2 = RuleContext::new("b.com", "/admin/panel", "GET");
        let matches2 = rules.evaluate(&ctx2);
        assert_eq!(matches2.len(), 1);
        assert_eq!(matches2[0].id.as_str(), "r2");

        let ctx3 = RuleContext::new("c.com", "/", "GET");
        let matches3 = rules.evaluate(&ctx3);
        assert!(matches3.is_empty());
    }

    #[test]
    fn test_enhanced_rule_set_evaluate_first() {
        let mut rules = EnhancedRuleSet::new();
        rules.add(
            EnhancedRule::new(
                "r1",
                "Rule 1",
                RuleCondition::HostMatches("a.com".to_string()),
                RuleAction::Block,
            )
            .with_priority(100),
        );
        rules.add(
            EnhancedRule::new(
                "r2",
                "Rule 2",
                RuleCondition::HostMatches("a.com".to_string()),
                RuleAction::Intercept,
            )
            .with_priority(50),
        );

        let ctx = RuleContext::new("a.com", "/", "GET");
        let first = rules.evaluate_first(&ctx).unwrap();
        assert_eq!(first.id.as_str(), "r1"); // Higher priority first
    }

    #[test]
    fn test_enhanced_rule_set_enable_disable() {
        let mut rules = EnhancedRuleSet::new();
        rules.add(EnhancedRule::new(
            "r1",
            "Rule 1",
            RuleCondition::HostMatches("a.com".to_string()),
            RuleAction::Block,
        ));
        assert!(rules.disable("r1"));
        assert!(!rules.get_by_id("r1").unwrap().enabled);
        assert!(rules.enable("r1"));
        assert!(rules.get_by_id("r1").unwrap().enabled);
        assert!(!rules.enable("nonexistent"));
    }

    #[test]
    fn test_enhanced_rule_set_tags() {
        let mut rules = EnhancedRuleSet::new();
        rules.add(
            EnhancedRule::new(
                "r1",
                "Rule 1",
                RuleCondition::HostMatches("a.com".to_string()),
                RuleAction::Tag,
            )
            .with_tag("api"),
        );
        rules.add(EnhancedRule::new(
            "r2",
            "Rule 2",
            RuleCondition::HostMatches("b.com".to_string()),
            RuleAction::Block,
        ));
        rules.add(
            EnhancedRule::new(
                "r3",
                "Rule 3",
                RuleCondition::HostMatches("c.com".to_string()),
                RuleAction::Tag,
            )
            .with_tag("monitor"),
        );

        let tags = rules.get_tags();
        assert!(tags.contains(&"api"));
        assert!(tags.contains(&"monitor"));
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn test_enhanced_rule_set_persistence() {
        let mut rules = EnhancedRuleSet::new();
        rules.add(EnhancedRule::new(
            "r1",
            "Rule 1",
            RuleCondition::HostMatches("a.com".to_string()),
            RuleAction::Block,
        ));
        rules.add(EnhancedRule::new(
            "r2",
            "Rule 2",
            RuleCondition::PathMatches("/admin/*".to_string()),
            RuleAction::Intercept,
        ));

        let json = rules.export_json().unwrap();
        let loaded = EnhancedRuleSet::import_json(&json).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.get_by_id("r1").unwrap().name, "Rule 1");
        assert_eq!(loaded.get_by_id("r2").unwrap().name, "Rule 2");
    }

    #[test]
    fn test_enhanced_rule_set_file_persistence() {
        let mut rules = EnhancedRuleSet::new();
        rules.add(EnhancedRule::new(
            "r1",
            "Test Rule",
            RuleCondition::HostMatches("test.com".to_string()),
            RuleAction::Block,
        ));

        let dir = std::env::temp_dir().join("eggsec_rule_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("rules.json");
        let path_str = path.to_str().unwrap();

        rules.save_to_file(path_str).unwrap();
        let loaded = EnhancedRuleSet::load_from_file(path_str).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.get_by_id("r1").unwrap().name, "Test Rule");

        let _ = std::fs::remove_file(path_str);
        let _ = std::fs::remove_dir(&dir);
    }

    // ==================== InjectResponseConfig tests ====================

    #[test]
    fn test_inject_response_config() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        let config = InjectResponseConfig {
            status: 200,
            headers,
            body: Some(r#"{"injected": true}"#.to_string()),
        };

        let rule = EnhancedRule::new(
            "inject-1",
            "Inject mock response",
            RuleCondition::PathMatches("/api/mock/*".to_string()),
            RuleAction::InjectResponse,
        )
        .with_inject_response(config);

        assert_eq!(rule.inject_response.as_ref().unwrap().status, 200);
        assert_eq!(
            rule.inject_response.as_ref().unwrap().body.as_deref(),
            Some(r#"{"injected": true}"#)
        );
    }

    // ==================== Complex condition nesting ====================

    #[test]
    fn test_nested_conditions() {
        let cond = RuleCondition::And(vec![
            RuleCondition::Or(vec![
                RuleCondition::HostMatches("a.com".to_string()),
                RuleCondition::HostMatches("b.com".to_string()),
            ]),
            RuleCondition::Not(Box::new(RuleCondition::PathMatches(
                "/public/*".to_string(),
            ))),
        ]);

        let ctx1 = RuleContext::new("a.com", "/admin", "GET");
        assert!(cond.evaluate(&ctx1));

        let ctx2 = RuleContext::new("b.com", "/admin", "GET");
        assert!(cond.evaluate(&ctx2));

        let ctx3 = RuleContext::new("a.com", "/public/page", "GET");
        assert!(!cond.evaluate(&ctx3));

        let ctx4 = RuleContext::new("c.com", "/admin", "GET");
        assert!(!cond.evaluate(&ctx4));
    }

    // ==================== Performance/Benchmark tests ====================

    #[test]
    fn test_rule_evaluation_throughput_1000_rules() {
        let mut rules = EnhancedRuleSet::new();

        // Add 1000 rules with varying conditions
        for i in 0..1000 {
            let condition = if i % 3 == 0 {
                RuleCondition::HostMatches(format!("host-{}.example.com", i))
            } else if i % 3 == 1 {
                RuleCondition::PathMatches(format!("/api/v{}/", i % 10))
            } else {
                RuleCondition::And(vec![
                    RuleCondition::HostMatches("target.example.com".to_string()),
                    RuleCondition::PathMatches(format!("/path/{}", i)),
                ])
            };

            rules.add(EnhancedRule::new(
                &format!("rule-{}", i),
                &format!("Rule {}", i),
                condition,
                RuleAction::Intercept,
            ));
        }

        let ctx = RuleContext::new("target.example.com", "/path/100", "GET");

        // Benchmark: evaluate 1000 rules
        let start = std::time::Instant::now();
        let iterations = 1000;
        for _ in 0..iterations {
            let _ = rules.evaluate(&ctx);
        }
        let elapsed = start.elapsed();
        let per_eval = elapsed / iterations;

        // Should complete in <1ms per evaluation for 1000 rules
        assert!(
            per_eval.as_micros() < 1000,
            "Rule evaluation too slow: {:?} per eval for 1000 rules",
            per_eval
        );
    }

    #[test]
    fn test_indexed_evaluation_throughput_1000_rules() {
        let mut rules = EnhancedRuleSet::new();

        // Add 1000 rules with host prefixes for indexing
        for i in 0..1000 {
            let condition = RuleCondition::HostMatches(format!("host-{}.example.com", i));
            rules.add(EnhancedRule::new(
                &format!("rule-{}", i),
                &format!("Rule {}", i),
                condition,
                RuleAction::Intercept,
            ));
        }

        let ctx = RuleContext::new("host-500.example.com", "/", "GET");

        // Benchmark indexed evaluation
        let start = std::time::Instant::now();
        let iterations = 1000;
        for _ in 0..iterations {
            let _ = rules.evaluate_indexed(&ctx);
        }
        let elapsed = start.elapsed();
        let per_eval = elapsed / iterations;

        // Indexed evaluation should be faster than full scan
        assert!(
            per_eval.as_micros() < 500,
            "Indexed rule evaluation too slow: {:?} per eval",
            per_eval
        );
    }

    #[test]
    fn test_rule_set_with_complex_nested_conditions() {
        let mut rules = EnhancedRuleSet::new();

        // Add rules with complex nested conditions
        for i in 0..100 {
            let condition = RuleCondition::And(vec![
                RuleCondition::Or(vec![
                    RuleCondition::HostMatches("api.example.com".to_string()),
                    RuleCondition::HostMatches("cdn.example.com".to_string()),
                ]),
                RuleCondition::Not(Box::new(RuleCondition::PathMatches("/health".to_string()))),
                RuleCondition::Or(vec![
                    RuleCondition::MethodMatches("POST".to_string()),
                    RuleCondition::MethodMatches("PUT".to_string()),
                    RuleCondition::MethodMatches("DELETE".to_string()),
                ]),
            ]);

            rules.add(EnhancedRule::new(
                &format!("complex-rule-{}", i),
                &format!("Complex Rule {}", i),
                condition.clone(),
                RuleAction::Intercept,
            ));
        }

        let ctx = RuleContext::new("api.example.com", "/api/data", "POST");
        let matches = rules.evaluate(&ctx);
        assert!(!matches.is_empty(), "Complex rules should match");
    }
}
