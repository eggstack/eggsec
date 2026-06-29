//! Optional MCP/Agent tool registration for web-proxy (Phase 4).
//! Only compiled when `web-proxy-mcp` feature is enabled.
//! Registers proxy control tools as MCP tools with appropriate policy gates.
//! Real runs via MCP require EnforcementContext + policy confirmation.

use serde::{Deserialize, Serialize};

/// MCP tool schema for web-proxy registration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebProxyToolSchema {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

impl Default for WebProxyToolSchema {
    fn default() -> Self {
        Self {
            name: "web-proxy".to_string(),
            description: "Interactive web proxy for HTTP/HTTPS traffic interception and inspection. Supports flow capture, manipulation, rule-based filtering, session recording, and HAR export. Dry-run is always safe. Real interception requires --allow-web-proxy + policy confirmation.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": [
                            "start", "stop", "status",
                            "list_flows", "inspect_flow", "forward_flow", "drop_flow", "replay_flow",
                            "add_rule", "list_rules", "remove_rule",
                            "export_session"
                        ],
                        "description": "Proxy control action"
                    },
                    "listen_addr": {
                        "type": "string",
                        "default": "127.0.0.1:8080",
                        "description": "Listen address for the proxy (start action)"
                    },
                    "upstream_addr": {
                        "type": "string",
                        "description": "Upstream proxy address (optional)"
                    },
                    "dry_run": {
                        "type": "boolean",
                        "default": true,
                        "description": "If true, produce synthetic reports without real network interception (always safe)"
                    },
                    "flow_index": {
                        "type": "integer",
                        "description": "Flow index for inspect/forward/drop/replay actions"
                    },
                    "rule_pattern": {
                        "type": "string",
                        "description": "URL pattern for intercept rules (e.g. '*.example.com/*')"
                    },
                    "rule_action": {
                        "type": "string",
                        "enum": ["allow", "block", "modify"],
                        "description": "Action for the intercept rule"
                    },
                    "rule_id": {
                        "type": "string",
                        "description": "Rule identifier for remove_rule action"
                    },
                    "export_format": {
                        "type": "string",
                        "enum": ["json", "har"],
                        "default": "json",
                        "description": "Export format for session data"
                    },
                    "max_flows": {
                        "type": "integer",
                        "default": 500,
                        "description": "Maximum number of flows to capture"
                    },
                    "max_duration": {
                        "type": "integer",
                        "default": 300,
                        "description": "Maximum session duration in seconds"
                    }
                },
                "required": ["action"]
            }),
        }
    }
}

/// MCP tool call parameters for web-proxy.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebProxyToolCall {
    pub action: String,
    #[serde(default = "default_listen_addr")]
    pub listen_addr: String,
    #[serde(default)]
    pub upstream_addr: Option<String>,
    #[serde(default = "default_true")]
    pub dry_run: bool,
    #[serde(default)]
    pub flow_index: Option<u64>,
    #[serde(default)]
    pub rule_pattern: Option<String>,
    #[serde(default)]
    pub rule_action: Option<String>,
    #[serde(default)]
    pub rule_id: Option<String>,
    #[serde(default = "default_export_format")]
    pub export_format: String,
    #[serde(default = "default_max_flows")]
    pub max_flows: u64,
    #[serde(default = "default_max_duration")]
    pub max_duration: u64,
}

fn default_listen_addr() -> String {
    "127.0.0.1:8080".to_string()
}

fn default_true() -> bool {
    true
}

fn default_export_format() -> String {
    "json".to_string()
}

fn default_max_flows() -> u64 {
    500
}

fn default_max_duration() -> u64 {
    300
}

/// Supported proxy actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyAction {
    Start,
    Stop,
    Status,
    ListFlows,
    InspectFlow,
    ForwardFlow,
    DropFlow,
    ReplayFlow,
    AddRule,
    ListRules,
    RemoveRule,
    ExportSession,
}

impl ProxyAction {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "start" => Some(Self::Start),
            "stop" => Some(Self::Stop),
            "status" => Some(Self::Status),
            "list_flows" => Some(Self::ListFlows),
            "inspect_flow" => Some(Self::InspectFlow),
            "forward_flow" => Some(Self::ForwardFlow),
            "drop_flow" => Some(Self::DropFlow),
            "replay_flow" => Some(Self::ReplayFlow),
            "add_rule" => Some(Self::AddRule),
            "list_rules" => Some(Self::ListRules),
            "remove_rule" => Some(Self::RemoveRule),
            "export_session" => Some(Self::ExportSession),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Status => "status",
            Self::ListFlows => "list_flows",
            Self::InspectFlow => "inspect_flow",
            Self::ForwardFlow => "forward_flow",
            Self::DropFlow => "drop_flow",
            Self::ReplayFlow => "replay_flow",
            Self::AddRule => "add_rule",
            Self::ListRules => "list_rules",
            Self::RemoveRule => "remove_rule",
            Self::ExportSession => "export_session",
        }
    }
}

impl WebProxyToolCall {
    /// Parse the action string into a typed [`ProxyAction`].
    pub fn parsed_action(&self) -> Option<ProxyAction> {
        ProxyAction::from_str(&self.action)
    }

    /// Validate the tool call parameters for the given action.
    pub fn validate(&self) -> Result<(), String> {
        let action = self.parsed_action().ok_or_else(|| {
            format!(
                "Invalid action '{}'. Supported: start, stop, status, list_flows, inspect_flow, \
                 forward_flow, drop_flow, replay_flow, add_rule, list_rules, remove_rule, export_session",
                self.action
            )
        })?;

        match action {
            ProxyAction::InspectFlow
            | ProxyAction::ForwardFlow
            | ProxyAction::DropFlow
            | ProxyAction::ReplayFlow => {
                if self.flow_index.is_none() {
                    return Err(format!(
                        "Action '{}' requires a 'flow_index' parameter",
                        action.as_str()
                    ));
                }
            }
            ProxyAction::RemoveRule => {
                if self.rule_id.is_none() {
                    return Err("Action 'remove_rule' requires a 'rule_id' parameter".to_string());
                }
            }
            ProxyAction::AddRule => {
                if self.rule_pattern.is_none() {
                    return Err("Action 'add_rule' requires a 'rule_pattern' parameter".to_string());
                }
                if let Some(ref act) = self.rule_action {
                    if !matches!(act.as_str(), "allow" | "block" | "modify") {
                        return Err(format!(
                            "Invalid rule_action '{}'. Supported: allow, block, modify",
                            act
                        ));
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Convert MCP tool call parameters to a summary for dry-run reporting.
    pub fn to_summary(&self) -> String {
        match self.parsed_action() {
            Some(action) => {
                let mut parts = vec![format!("action={}", action.as_str())];
                if action == ProxyAction::Start {
                    parts.push(format!("listen={}", self.listen_addr));
                    parts.push(format!("dry_run={}", self.dry_run));
                }
                if let Some(idx) = self.flow_index {
                    parts.push(format!("flow_index={}", idx));
                }
                if let Some(ref pat) = self.rule_pattern {
                    parts.push(format!("pattern={}", pat));
                }
                parts.join(", ")
            }
            None => format!("action=invalid({})", self.action),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_schema_has_name_and_description() {
        let schema = WebProxyToolSchema::default();
        assert_eq!(schema.name, "web-proxy");
        assert!(!schema.description.is_empty());
        assert!(schema.input_schema.is_object());
    }

    #[test]
    fn tool_call_defaults() {
        let call = serde_json::from_value::<WebProxyToolCall>(serde_json::json!({
            "action": "status"
        }))
        .unwrap();
        assert_eq!(call.action, "status");
        assert_eq!(call.listen_addr, "127.0.0.1:8080");
        assert!(call.dry_run);
        assert_eq!(call.export_format, "json");
        assert_eq!(call.max_flows, 500);
        assert_eq!(call.max_duration, 300);
    }

    #[test]
    fn tool_call_custom_params() {
        let call = serde_json::from_value::<WebProxyToolCall>(serde_json::json!({
            "action": "start",
            "listen_addr": "0.0.0.0:9090",
            "dry_run": false,
            "max_flows": 1000,
            "max_duration": 600
        }))
        .unwrap();
        assert_eq!(call.listen_addr, "0.0.0.0:9090");
        assert!(!call.dry_run);
        assert_eq!(call.max_flows, 1000);
        assert_eq!(call.max_duration, 600);
    }

    #[test]
    fn parsed_action_valid() {
        assert_eq!(
            WebProxyToolCall {
                action: "start".to_string(),
                ..Default::default()
            }
            .parsed_action(),
            Some(ProxyAction::Start)
        );
        assert_eq!(
            WebProxyToolCall {
                action: "list_flows".to_string(),
                ..Default::default()
            }
            .parsed_action(),
            Some(ProxyAction::ListFlows)
        );
    }

    #[test]
    fn parsed_action_invalid() {
        let call = WebProxyToolCall {
            action: "bogus".to_string(),
            ..Default::default()
        };
        assert_eq!(call.parsed_action(), None);
    }

    #[test]
    fn validate_requires_flow_index() {
        let call = WebProxyToolCall {
            action: "inspect_flow".to_string(),
            flow_index: None,
            ..Default::default()
        };
        assert!(call.validate().is_err());
        assert!(call.validate().unwrap_err().contains("flow_index"));
    }

    #[test]
    fn validate_inspect_flow_with_index() {
        let call = WebProxyToolCall {
            action: "inspect_flow".to_string(),
            flow_index: Some(5),
            ..Default::default()
        };
        assert!(call.validate().is_ok());
    }

    #[test]
    fn validate_remove_rule_requires_rule_id() {
        let call = WebProxyToolCall {
            action: "remove_rule".to_string(),
            rule_id: None,
            ..Default::default()
        };
        assert!(call.validate().is_err());
        assert!(call.validate().unwrap_err().contains("rule_id"));
    }

    #[test]
    fn validate_add_rule_requires_pattern() {
        let call = WebProxyToolCall {
            action: "add_rule".to_string(),
            rule_pattern: None,
            ..Default::default()
        };
        assert!(call.validate().is_err());
        assert!(call.validate().unwrap_err().contains("rule_pattern"));
    }

    #[test]
    fn validate_add_rule_invalid_action() {
        let call = WebProxyToolCall {
            action: "add_rule".to_string(),
            rule_pattern: Some("*.example.com/*".to_string()),
            rule_action: Some("bogus".to_string()),
            ..Default::default()
        };
        assert!(call.validate().is_err());
        assert!(call.validate().unwrap_err().contains("rule_action"));
    }

    #[test]
    fn validate_add_rule_valid() {
        let call = WebProxyToolCall {
            action: "add_rule".to_string(),
            rule_pattern: Some("*.example.com/*".to_string()),
            rule_action: Some("block".to_string()),
            ..Default::default()
        };
        assert!(call.validate().is_ok());
    }

    #[test]
    fn validate_invalid_action() {
        let call = WebProxyToolCall {
            action: "explode".to_string(),
            ..Default::default()
        };
        assert!(call.validate().is_err());
        assert!(call.validate().unwrap_err().contains("Invalid action"));
    }

    #[test]
    fn validate_forward_flow_requires_index() {
        let call = WebProxyToolCall {
            action: "forward_flow".to_string(),
            flow_index: None,
            ..Default::default()
        };
        assert!(call.validate().is_err());
    }

    #[test]
    fn validate_drop_flow_requires_index() {
        let call = WebProxyToolCall {
            action: "drop_flow".to_string(),
            flow_index: None,
            ..Default::default()
        };
        assert!(call.validate().is_err());
    }

    #[test]
    fn validate_replay_flow_requires_index() {
        let call = WebProxyToolCall {
            action: "replay_flow".to_string(),
            flow_index: None,
            ..Default::default()
        };
        assert!(call.validate().is_err());
    }

    #[test]
    fn to_summary_start() {
        let call = WebProxyToolCall {
            action: "start".to_string(),
            listen_addr: "0.0.0.0:8080".to_string(),
            dry_run: true,
            ..Default::default()
        };
        let s = call.to_summary();
        assert!(s.contains("action=start"));
        assert!(s.contains("listen=0.0.0.0:8080"));
        assert!(s.contains("dry_run=true"));
    }

    #[test]
    fn to_summary_inspect_flow() {
        let call = WebProxyToolCall {
            action: "inspect_flow".to_string(),
            flow_index: Some(3),
            ..Default::default()
        };
        let s = call.to_summary();
        assert!(s.contains("action=inspect_flow"));
        assert!(s.contains("flow_index=3"));
    }

    #[test]
    fn action_as_str_roundtrip() {
        for action in [
            ProxyAction::Start,
            ProxyAction::Stop,
            ProxyAction::Status,
            ProxyAction::ListFlows,
            ProxyAction::InspectFlow,
            ProxyAction::ForwardFlow,
            ProxyAction::DropFlow,
            ProxyAction::ReplayFlow,
            ProxyAction::AddRule,
            ProxyAction::ListRules,
            ProxyAction::RemoveRule,
            ProxyAction::ExportSession,
        ] {
            assert_eq!(ProxyAction::from_str(action.as_str()), Some(action));
        }
    }
}
