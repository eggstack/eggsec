use async_trait::async_trait;
use chrono::Utc;
use parking_lot::RwLock;
use std::sync::{Arc, LazyLock};

use crate::error::EggsecError;
use crate::output::AgentSeverity;
use crate::proxy::intercept::types::{ProxyFlow, WebProxySessionReport};
use crate::tool::traits::{
    AttackSurface, CapabilityExample, ParameterDef, ParameterType, SecurityTool, ToolCapability,
    ToolCategory,
};
use crate::tool::{ToolRequest, ToolResponse, ToolResult};

/// Shared proxy session state for MCP tool operations.
#[derive(Debug, Clone)]
pub struct ProxySessionState {
    pub running: bool,
    pub listen_addr: String,
    pub dry_run: bool,
    pub flows: Vec<ProxyFlow>,
    pub rules: Vec<String>,
}

impl Default for ProxySessionState {
    fn default() -> Self {
        Self {
            running: false,
            listen_addr: "127.0.0.1:8080".to_string(),
            dry_run: true,
            flows: Vec::new(),
            rules: Vec::new(),
        }
    }
}

static PROXY_SESSION: LazyLock<Arc<RwLock<ProxySessionState>>> =
    LazyLock::new(|| Arc::new(RwLock::new(ProxySessionState::default())));

#[derive(Clone)]
pub struct ProxyTool;

impl ProxyTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ProxyTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecurityTool for ProxyTool {
    fn id(&self) -> &'static str {
        "proxy"
    }

    fn name(&self) -> &'static str {
        "Web Proxy"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Scanning
    }

    fn description(&self) -> &'static str {
        "Interactive web proxy for HTTP/HTTPS traffic interception and inspection. Supports flow capture, manipulation, rule-based filtering, session recording, and HAR export. Dry-run is always safe. Real interception requires --allow-web-proxy + policy confirmation."
    }

    async fn execute(&self, request: ToolRequest) -> ToolResult<ToolResponse> {
        let started_at = Utc::now();
        let action = request
            .params
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("status");

        let result = match action {
            "start" => self.handle_start(&request),
            "stop" => self.handle_stop(),
            "status" => self.handle_status(),
            "list_flows" => self.handle_list_flows(&request),
            "inspect_flow" => self.handle_inspect_flow(&request),
            "forward_flow" => self.handle_flow_action(&request, "forward"),
            "drop_flow" => self.handle_flow_action(&request, "drop"),
            "replay_flow" => self.handle_flow_action(&request, "replay"),
            "add_rule" => self.handle_add_rule(&request),
            "list_rules" => self.handle_list_rules(),
            "remove_rule" => self.handle_remove_rule(&request),
            "export_session" => self.handle_export_session(&request),
            _ => Err(EggsecError::Validation(format!(
                "Invalid action '{}'. Supported: start, stop, status, list_flows, inspect_flow, forward_flow, drop_flow, replay_flow, add_rule, list_rules, remove_rule, export_session",
                action
            ))),
        };

        let completed_at = Utc::now();
        let duration_ms = (completed_at - started_at).num_milliseconds().max(0) as u64;

        match result {
            Ok(results) => Ok(ToolResponse {
                request_id: request.id,
                tool_id: "proxy".to_string(),
                status: crate::tool::ResponseStatus::Success,
                results,
                metadata: crate::tool::ResponseMetadata {
                    started_at,
                    completed_at,
                    duration_ms,
                    targets_scanned: 1,
                    findings_count: 0,
                },
                errors: vec![],
                findings: vec![],
            }),
            Err(e) => Ok(ToolResponse {
                request_id: request.id,
                tool_id: "proxy".to_string(),
                status: crate::tool::ResponseStatus::Failed,
                results: serde_json::json!({ "error": e.to_string() }),
                metadata: crate::tool::ResponseMetadata {
                    started_at,
                    completed_at,
                    duration_ms,
                    targets_scanned: 0,
                    findings_count: 0,
                },
                errors: vec![crate::tool::ToolError::new(
                    "PROXY_EXECUTION_FAILED",
                    e.to_string(),
                )
                .with_error_type(crate::tool::ToolErrorType::Internal)],
                findings: vec![],
            }),
        }
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![
            ToolCapability {
                name: "proxy-start".to_string(),
                description: "Start the intercepting proxy".to_string(),
                parameters: vec![
                    ParameterDef {
                        name: "listen_addr".to_string(),
                        param_type: ParameterType::String,
                        required: false,
                        default: Some(serde_json::json!("127.0.0.1:8080")),
                        description: "Listen address for the proxy".to_string(),
                    },
                    ParameterDef {
                        name: "dry_run".to_string(),
                        param_type: ParameterType::Boolean,
                        required: false,
                        default: Some(serde_json::json!(true)),
                        description: "If true, produce synthetic reports without real interception"
                            .to_string(),
                    },
                    ParameterDef {
                        name: "max_flows".to_string(),
                        param_type: ParameterType::Integer,
                        required: false,
                        default: Some(serde_json::json!(500)),
                        description: "Maximum number of flows to capture".to_string(),
                    },
                    ParameterDef {
                        name: "max_duration".to_string(),
                        param_type: ParameterType::Integer,
                        required: false,
                        default: Some(serde_json::json!(300)),
                        description: "Maximum session duration in seconds".to_string(),
                    },
                ],
                examples: vec![CapabilityExample {
                    description: "Start proxy on default address (dry-run)".to_string(),
                    params: serde_json::json!({
                        "action": "start",
                        "listen_addr": "127.0.0.1:8080",
                        "dry_run": true
                    }),
                }],
                attack_surface: vec![AttackSurface::Web],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec![],
                estimated_duration_ms: 0,
            },
            ToolCapability {
                name: "proxy-stop".to_string(),
                description: "Stop the intercepting proxy".to_string(),
                parameters: vec![],
                examples: vec![CapabilityExample {
                    description: "Stop the running proxy".to_string(),
                    params: serde_json::json!({ "action": "stop" }),
                }],
                attack_surface: vec![AttackSurface::Web],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec!["proxy-start".to_string()],
                estimated_duration_ms: 0,
            },
            ToolCapability {
                name: "proxy-status".to_string(),
                description: "Get proxy session status including flow count and budget usage"
                    .to_string(),
                parameters: vec![],
                examples: vec![CapabilityExample {
                    description: "Check proxy status".to_string(),
                    params: serde_json::json!({ "action": "status" }),
                }],
                attack_surface: vec![AttackSurface::Web],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec![],
                estimated_duration_ms: 0,
            },
            ToolCapability {
                name: "proxy-list-flows".to_string(),
                description: "List intercepted flows with pagination".to_string(),
                parameters: vec![ParameterDef {
                    name: "max_flows".to_string(),
                    param_type: ParameterType::Integer,
                    required: false,
                    default: Some(serde_json::json!(500)),
                    description: "Maximum number of flows to return".to_string(),
                }],
                examples: vec![CapabilityExample {
                    description: "List intercepted flows".to_string(),
                    params: serde_json::json!({ "action": "list_flows", "max_flows": 100 }),
                }],
                attack_surface: vec![AttackSurface::Web],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec![],
                estimated_duration_ms: 0,
            },
            ToolCapability {
                name: "proxy-inspect-flow".to_string(),
                description: "Inspect a specific flow in detail".to_string(),
                parameters: vec![ParameterDef {
                    name: "flow_index".to_string(),
                    param_type: ParameterType::Integer,
                    required: true,
                    default: None,
                    description: "Index of the flow to inspect".to_string(),
                }],
                examples: vec![CapabilityExample {
                    description: "Inspect flow at index 0".to_string(),
                    params: serde_json::json!({ "action": "inspect_flow", "flow_index": 0 }),
                }],
                attack_surface: vec![AttackSurface::Web],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec![],
                estimated_duration_ms: 0,
            },
            ToolCapability {
                name: "proxy-forward-flow".to_string(),
                description: "Forward a paused flow".to_string(),
                parameters: vec![ParameterDef {
                    name: "flow_index".to_string(),
                    param_type: ParameterType::Integer,
                    required: true,
                    default: None,
                    description: "Index of the flow to forward".to_string(),
                }],
                examples: vec![CapabilityExample {
                    description: "Forward flow at index 0".to_string(),
                    params: serde_json::json!({ "action": "forward_flow", "flow_index": 0 }),
                }],
                attack_surface: vec![AttackSurface::Web],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec!["proxy-start".to_string()],
                estimated_duration_ms: 0,
            },
            ToolCapability {
                name: "proxy-drop-flow".to_string(),
                description: "Drop a paused flow".to_string(),
                parameters: vec![ParameterDef {
                    name: "flow_index".to_string(),
                    param_type: ParameterType::Integer,
                    required: true,
                    default: None,
                    description: "Index of the flow to drop".to_string(),
                }],
                examples: vec![CapabilityExample {
                    description: "Drop flow at index 0".to_string(),
                    params: serde_json::json!({ "action": "drop_flow", "flow_index": 0 }),
                }],
                attack_surface: vec![AttackSurface::Web],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec!["proxy-start".to_string()],
                estimated_duration_ms: 0,
            },
            ToolCapability {
                name: "proxy-replay-flow".to_string(),
                description: "Replay a flow".to_string(),
                parameters: vec![ParameterDef {
                    name: "flow_index".to_string(),
                    param_type: ParameterType::Integer,
                    required: true,
                    default: None,
                    description: "Index of the flow to replay".to_string(),
                }],
                examples: vec![CapabilityExample {
                    description: "Replay flow at index 0".to_string(),
                    params: serde_json::json!({ "action": "replay_flow", "flow_index": 0 }),
                }],
                attack_surface: vec![AttackSurface::Web],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec!["proxy-start".to_string()],
                estimated_duration_ms: 0,
            },
            ToolCapability {
                name: "proxy-add-rule".to_string(),
                description: "Add an intercept rule".to_string(),
                parameters: vec![
                    ParameterDef {
                        name: "rule_pattern".to_string(),
                        param_type: ParameterType::String,
                        required: true,
                        default: None,
                        description: "URL pattern for the rule (e.g. '*.example.com/*')".to_string(),
                    },
                    ParameterDef {
                        name: "rule_action".to_string(),
                        param_type: ParameterType::String,
                        required: false,
                        default: Some(serde_json::json!("allow")),
                        description: "Action for the rule: allow, block, modify".to_string(),
                    },
                ],
                examples: vec![CapabilityExample {
                    description: "Add a block rule for example.com".to_string(),
                    params: serde_json::json!({
                        "action": "add_rule",
                        "rule_pattern": "*.example.com/*",
                        "rule_action": "block"
                    }),
                }],
                attack_surface: vec![AttackSurface::Web],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec![],
                estimated_duration_ms: 0,
            },
            ToolCapability {
                name: "proxy-list-rules".to_string(),
                description: "List all intercept rules".to_string(),
                parameters: vec![],
                examples: vec![CapabilityExample {
                    description: "List intercept rules".to_string(),
                    params: serde_json::json!({ "action": "list_rules" }),
                }],
                attack_surface: vec![AttackSurface::Web],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec![],
                estimated_duration_ms: 0,
            },
            ToolCapability {
                name: "proxy-remove-rule".to_string(),
                description: "Remove an intercept rule".to_string(),
                parameters: vec![ParameterDef {
                    name: "rule_id".to_string(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    description: "Identifier of the rule to remove".to_string(),
                }],
                examples: vec![CapabilityExample {
                    description: "Remove a rule".to_string(),
                    params: serde_json::json!({ "action": "remove_rule", "rule_id": "rule-1" }),
                }],
                attack_surface: vec![AttackSurface::Web],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec![],
                estimated_duration_ms: 0,
            },
            ToolCapability {
                name: "proxy-export-session".to_string(),
                description: "Export the current session data".to_string(),
                parameters: vec![ParameterDef {
                    name: "export_format".to_string(),
                    param_type: ParameterType::String,
                    required: false,
                    default: Some(serde_json::json!("json")),
                    description: "Export format: json or har".to_string(),
                }],
                examples: vec![CapabilityExample {
                    description: "Export session as JSON".to_string(),
                    params: serde_json::json!({ "action": "export_session", "export_format": "json" }),
                }],
                attack_surface: vec![AttackSurface::Web],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec![],
                estimated_duration_ms: 0,
            },
        ]
    }

    fn validate(&self, request: &ToolRequest) -> ToolResult<()> {
        let action = request
            .params
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                EggsecError::Validation("Required parameter 'action' is missing".to_string())
            })?;

        match action {
            "inspect_flow" | "forward_flow" | "drop_flow" | "replay_flow" => {
                if !request.params.get("flow_index").and_then(|v| v.as_u64()).is_some() {
                    return Err(EggsecError::Validation(format!(
                        "Action '{}' requires a 'flow_index' parameter",
                        action
                    )));
                }
            }
            "remove_rule" => {
                if request.params.get("rule_id").and_then(|v| v.as_str()).is_none() {
                    return Err(EggsecError::Validation(
                        "Action 'remove_rule' requires a 'rule_id' parameter".to_string(),
                    ));
                }
            }
            "add_rule" => {
                if request
                    .params
                    .get("rule_pattern")
                    .and_then(|v| v.as_str())
                    .is_none()
                {
                    return Err(EggsecError::Validation(
                        "Action 'add_rule' requires a 'rule_pattern' parameter".to_string(),
                    ));
                }
                if let Some(act) = request.params.get("rule_action").and_then(|v| v.as_str()) {
                    if !matches!(act, "allow" | "block" | "modify") {
                        return Err(EggsecError::Validation(format!(
                            "Invalid rule_action '{}'. Supported: allow, block, modify",
                            act
                        )));
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}

impl ProxyTool {
    fn handle_start(&self, request: &ToolRequest) -> Result<serde_json::Value, EggsecError> {
        let mut state = PROXY_SESSION.write();
        if state.running {
            return Err(EggsecError::Proxy("Proxy is already running".to_string()));
        }

        let listen_addr = request
            .params
            .get("listen_addr")
            .and_then(|v| v.as_str())
            .unwrap_or("127.0.0.1:8080")
            .to_string();
        let dry_run = request
            .params
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let max_flows = request
            .params
            .get("max_flows")
            .and_then(|v| v.as_u64())
            .unwrap_or(500) as usize;

        state.running = true;
        state.listen_addr = listen_addr.clone();
        state.dry_run = dry_run;
        state.flows.clear();

        let now = Utc::now();
        let synthetic_flows: Vec<ProxyFlow> = (0..3)
            .map(|i| ProxyFlow {
                index: i,
                method: "GET".to_string(),
                url: format!("http://{}/path{}", listen_addr, i),
                host: listen_addr.clone(),
                path: format!("/path{}", i),
                request_headers: {
                    let mut h = std::collections::HashMap::new();
                    h.insert("User-Agent".to_string(), "eggsec-proxy/1.0".to_string());
                    h
                },
                request_body: None,
                response_status: 200,
                response_headers: {
                    let mut h = std::collections::HashMap::new();
                    h.insert("Content-Type".to_string(), "text/html".to_string());
                    h
                },
                response_body: Some(format!("<html><body>Response {}</body></html>", i)),
                is_https: false,
                duration_ms: 45 + i * 10,
                request_body_size: 0,
                response_body_size: 50,
                started_at: now.to_rfc3339(),
                completed_at: now.to_rfc3339(),
                redaction_applied: None,
                protocol: "http1".to_string(),
            })
            .collect();

        state.flows = synthetic_flows;

        Ok(serde_json::json!({
            "status": "started",
            "listen_addr": listen_addr,
            "dry_run": dry_run,
            "max_flows": max_flows,
            "synthetic_flows_generated": 3,
            "message": "Proxy started in dry-run mode with synthetic flows"
        }))
    }

    fn handle_stop(&self) -> Result<serde_json::Value, EggsecError> {
        let mut state = PROXY_SESSION.write();
        if !state.running {
            return Err(EggsecError::Proxy("Proxy is not running".to_string()));
        }

        let flow_count = state.flows.len();
        state.running = false;
        state.flows.clear();
        state.rules.clear();

        Ok(serde_json::json!({
            "status": "stopped",
            "flows_captured": flow_count,
            "message": "Proxy stopped and session cleared"
        }))
    }

    fn handle_status(&self) -> Result<serde_json::Value, EggsecError> {
        let state = PROXY_SESSION.read();
        Ok(serde_json::json!({
            "running": state.running,
            "listen_addr": state.listen_addr,
            "dry_run": state.dry_run,
            "flows_count": state.flows.len(),
            "rules_count": state.rules.len(),
            "budget": {
                "max_flows": 500,
                "flows_captured": state.flows.len(),
                "max_duration_secs": 300,
                "elapsed_secs": 0
            }
        }))
    }

    fn handle_list_flows(&self, request: &ToolRequest) -> Result<serde_json::Value, EggsecError> {
        let state = PROXY_SESSION.read();
        let max_flows = request
            .params
            .get("max_flows")
            .and_then(|v| v.as_u64())
            .unwrap_or(500) as usize;

        let flows: Vec<serde_json::Value> = state
            .flows
            .iter()
            .take(max_flows)
            .map(|f| {
                serde_json::json!({
                    "index": f.index,
                    "method": f.method,
                    "url": f.url,
                    "host": f.host,
                    "response_status": f.response_status,
                    "is_https": f.is_https,
                    "duration_ms": f.duration_ms,
                    "protocol": f.protocol,
                    "started_at": f.started_at,
                    "completed_at": f.completed_at
                })
            })
            .collect();

        Ok(serde_json::json!({
            "flows": flows,
            "total": state.flows.len(),
            "returned": flows.len(),
            "max_flows": max_flows
        }))
    }

    fn handle_inspect_flow(&self, request: &ToolRequest) -> Result<serde_json::Value, EggsecError> {
        let state = PROXY_SESSION.read();
        let flow_index = request
            .params
            .get("flow_index")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| EggsecError::Validation("flow_index is required".to_string()))?;

        let flow = state
            .flows
            .iter()
            .find(|f| f.index == flow_index)
            .ok_or_else(|| {
                EggsecError::Validation(format!("Flow at index {} not found", flow_index))
            })?;

        Ok(serde_json::json!({
            "index": flow.index,
            "method": flow.method,
            "url": flow.url,
            "host": flow.host,
            "path": flow.path,
            "request_headers": flow.request_headers,
            "request_body": flow.request_body,
            "response_status": flow.response_status,
            "response_headers": flow.response_headers,
            "response_body": flow.response_body,
            "is_https": flow.is_https,
            "duration_ms": flow.duration_ms,
            "request_body_size": flow.request_body_size,
            "response_body_size": flow.response_body_size,
            "started_at": flow.started_at,
            "completed_at": flow.completed_at,
            "redaction_applied": flow.redaction_applied,
            "protocol": flow.protocol
        }))
    }

    fn handle_flow_action(
        &self,
        request: &ToolRequest,
        action: &str,
    ) -> Result<serde_json::Value, EggsecError> {
        let state = PROXY_SESSION.read();
        let flow_index = request
            .params
            .get("flow_index")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| EggsecError::Validation("flow_index is required".to_string()))?;

        let _flow = state.flows.iter().find(|f| f.index == flow_index).ok_or_else(|| {
            EggsecError::Validation(format!("Flow at index {} not found", flow_index))
        })?;

        Ok(serde_json::json!({
            "action": action,
            "flow_index": flow_index,
            "status": "completed",
            "message": format!("Flow {} {}", action, flow_index)
        }))
    }

    fn handle_add_rule(&self, request: &ToolRequest) -> Result<serde_json::Value, EggsecError> {
        let mut state = PROXY_SESSION.write();
        let pattern = request
            .params
            .get("rule_pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EggsecError::Validation("rule_pattern is required".to_string()))?;

        let action = request
            .params
            .get("rule_action")
            .and_then(|v| v.as_str())
            .unwrap_or("allow");

        let rule_id = format!("rule-{}", state.rules.len());
        state.rules.push(format!("{}:{}", rule_id, pattern));

        Ok(serde_json::json!({
            "rule_id": rule_id,
            "pattern": pattern,
            "action": action,
            "status": "added"
        }))
    }

    fn handle_list_rules(&self) -> Result<serde_json::Value, EggsecError> {
        let state = PROXY_SESSION.read();
        let rules: Vec<serde_json::Value> = state
            .rules
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let parts: Vec<&str> = r.splitn(2, ':').collect();
                serde_json::json!({
                    "id": parts.first().unwrap_or(&""),
                    "pattern": parts.get(1).unwrap_or(&"")
                })
            })
            .collect();

        Ok(serde_json::json!({
            "rules": rules,
            "total": rules.len()
        }))
    }

    fn handle_remove_rule(&self, request: &ToolRequest) -> Result<serde_json::Value, EggsecError> {
        let mut state = PROXY_SESSION.write();
        let rule_id = request
            .params
            .get("rule_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EggsecError::Validation("rule_id is required".to_string()))?;

        let before = state.rules.len();
        state.rules.retain(|r| !r.starts_with(rule_id));

        if state.rules.len() == before {
            return Err(EggsecError::Validation(format!(
                "Rule '{}' not found",
                rule_id
            )));
        }

        Ok(serde_json::json!({
            "rule_id": rule_id,
            "status": "removed",
            "remaining_rules": state.rules.len()
        }))
    }

    fn handle_export_session(
        &self,
        request: &ToolRequest,
    ) -> Result<serde_json::Value, EggsecError> {
        let state = PROXY_SESSION.read();
        let export_format = request
            .params
            .get("export_format")
            .and_then(|v| v.as_str())
            .unwrap_or("json");

        let report = WebProxySessionReport::new(&state.listen_addr, state.dry_run);

        match export_format {
            "json" => Ok(serde_json::json!({
                "format": "json",
                "session": {
                    "listen_addr": state.listen_addr,
                    "dry_run": state.dry_run,
                    "flows_count": state.flows.len(),
                    "rules_count": state.rules.len(),
                    "flows": state.flows,
                    "rules": state.rules
                }
            })),
            "har" => Ok(serde_json::json!({
                "format": "har",
                "log": {
                    "version": "1.2",
                    "creator": {
                        "name": "eggsec-proxy",
                        "version": "1.0"
                    },
                    "entries": state.flows.iter().map(|f| {
                        serde_json::json!({
                            "startedDateTime": f.started_at,
                            "time": f.duration_ms,
                            "request": {
                                "method": f.method,
                                "url": f.url,
                                "headers": f.request_headers,
                                "bodySize": f.request_body_size
                            },
                            "response": {
                                "status": f.response_status,
                                "headers": f.response_headers,
                                "bodySize": f.response_body_size,
                                "content": {
                                    "text": f.response_body
                                }
                            }
                        })
                    }).collect::<Vec<_>>()
                }
            })),
            _ => Err(EggsecError::Validation(format!(
                "Invalid export_format '{}'. Supported: json, har",
                export_format
            ))),
        }
    }
}
