use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::config::ScopeRule;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConstraintContext {
    pub allowed_targets: Vec<ScopeRule>,
    pub disallowed_actions: Vec<String>,
    pub approval_required_actions: Vec<String>,
    pub max_concurrent_scans: Option<usize>,
    pub rate_limit_per_minute: Option<u32>,
}

impl McpConstraintContext {
    pub fn new() -> Self {
        Self {
            allowed_targets: Vec::new(),
            disallowed_actions: Vec::new(),
            approval_required_actions: Vec::new(),
            max_concurrent_scans: None,
            rate_limit_per_minute: None,
        }
    }

    pub fn with_allowed_targets(mut self, targets: Vec<ScopeRule>) -> Self {
        self.allowed_targets = targets;
        self
    }

    pub fn with_disallowed_actions(mut self, actions: Vec<String>) -> Self {
        self.disallowed_actions = actions;
        self
    }

    pub fn with_approval_required(mut self, actions: Vec<String>) -> Self {
        self.approval_required_actions = actions;
        self
    }

    pub fn with_rate_limits(mut self, max_scans: usize, rate_limit: u32) -> Self {
        self.max_concurrent_scans = Some(max_scans);
        self.rate_limit_per_minute = Some(rate_limit);
        self
    }

    pub fn is_action_allowed(&self, action: &str) -> bool {
        !self
            .disallowed_actions
            .iter()
            .any(|a| a.eq_ignore_ascii_case(action))
    }

    pub fn requires_approval(&self, action: &str) -> bool {
        self.approval_required_actions
            .iter()
            .any(|a| a.eq_ignore_ascii_case(action))
    }

    pub fn is_target_allowed(&self, target: &str) -> bool {
        if self.allowed_targets.is_empty() {
            return true;
        }
        let target_scope = crate::config::TargetScope::parse(target).unwrap_or_else(|_| {
            crate::config::TargetScope {
                host: target.to_string(),
                ip: None,
            }
        });
        self.allowed_targets
            .iter()
            .any(|rule| rule.matches(&target_scope))
    }

    pub fn get_allowed_tools(&self) -> HashSet<String> {
        let mut tools = HashSet::new();
        tools.insert("scan".to_string());
        tools.insert("fuzz".to_string());
        tools.insert("recon".to_string());
        tools.insert("waf".to_string());

        for action in &self.disallowed_actions {
            tools.remove(action);
        }
        tools
    }
}

impl Default for McpConstraintContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "rest-api")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ScopeRule;

    #[test]
    fn test_action_allowed() {
        let ctx = McpConstraintContext::new()
            .with_disallowed_actions(vec!["delete".to_string(), "drop".to_string()]);

        assert!(ctx.is_action_allowed("scan"));
        assert!(!ctx.is_action_allowed("delete"));
        assert!(!ctx.is_action_allowed("DROP"));
    }

    #[test]
    fn test_approval_required() {
        let ctx = McpConstraintContext::new().with_approval_required(vec!["exploit".to_string()]);

        assert!(!ctx.requires_approval("scan"));
        assert!(ctx.requires_approval("exploit"));
    }

    #[test]
    fn test_allowed_tools() {
        let ctx = McpConstraintContext::new().with_disallowed_actions(vec!["waf".to_string()]);

        let tools = ctx.get_allowed_tools();
        assert!(tools.contains("scan"));
        assert!(!tools.contains("waf"));
    }
}
