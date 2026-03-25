use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;

use crate::tool::{
    RequestOptions, Target, ToolDispatcher, ToolRegistry, ToolRequest,
};

#[derive(Clone)]
pub struct GrpcService {
    registry: ToolRegistry,
    dispatcher: ToolDispatcher,
    api_key: Option<String>,
}

impl GrpcService {
    pub fn new(registry: ToolRegistry, api_key: Option<String>) -> Self {
        let dispatcher = ToolDispatcher::new(registry.clone());
        Self {
            registry,
            dispatcher,
            api_key,
        }
    }

    pub fn validate_api_key(&self, key: &str) -> Result<(), String> {
        if let Some(ref expected) = self.api_key {
            if expected.as_bytes().ct_eq(key.as_bytes()).unwrap_u8() != 1 {
                return Err("Invalid or missing API key".to_string());
            }
        }
        Ok(())
    }

    pub fn get_registry(&self) -> &ToolRegistry {
        &self.registry
    }

    pub fn get_dispatcher(&self) -> &ToolDispatcher {
        &self.dispatcher
    }

    pub async fn list_tools(&self, category: Option<String>) -> Result<ListToolsResponse, String> {
        let tools = if let Some(cat) = category {
            let cat_lower = cat.to_lowercase();
            let tool_cat = match cat_lower.as_str() {
                "recon" => crate::tool::ToolCategory::Recon,
                "scanning" => crate::tool::ToolCategory::Scanning,
                "fuzzing" => crate::tool::ToolCategory::Fuzzing,
                "waf" => crate::tool::ToolCategory::Waf,
                "loadtest" | "load_test" => crate::tool::ToolCategory::LoadTest,
                "stress" => crate::tool::ToolCategory::Stress,
                "pipeline" => crate::tool::ToolCategory::Pipeline,
                _ => crate::tool::ToolCategory::Recon,
            };
            self.registry.list_by_category(tool_cat)
        } else {
            self.registry.list()
        };

        let tool_infos: Vec<GrpcToolInfo> = tools
            .into_iter()
            .map(|t| GrpcToolInfo {
                id: t.id,
                name: t.name,
                category: t.category.to_string(),
                description: t.description,
                protocols: t.protocols,
                capabilities: t
                    .capabilities
                    .into_iter()
                    .map(|c| GrpcToolCapability {
                        name: c.name,
                        description: c.description,
                        parameters: c
                            .parameters
                            .into_iter()
                            .map(|p| GrpcParameterDef {
                                name: p.name,
                                param_type: p.param_type.to_string(),
                                required: p.required,
                                default: p.default,
                                description: p.description,
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect();

        let categories: Vec<String> = self
            .registry
            .categories()
            .iter()
            .map(|c| c.to_string())
            .collect();

        Ok(ListToolsResponse {
            tools: tool_infos,
            categories,
        })
    }

    pub async fn get_tool(&self, tool_id: &str) -> Result<GrpcToolInfo, String> {
        let tools = self.registry.list();
        tools
            .into_iter()
            .find(|t| t.id == tool_id)
            .map(|t| GrpcToolInfo {
                id: t.id,
                name: t.name,
                category: t.category.to_string(),
                description: t.description,
                protocols: t.protocols,
                capabilities: t
                    .capabilities
                    .into_iter()
                    .map(|c| GrpcToolCapability {
                        name: c.name,
                        description: c.description,
                        parameters: c
                            .parameters
                            .into_iter()
                            .map(|p| GrpcParameterDef {
                                name: p.name,
                                param_type: p.param_type.to_string(),
                                required: p.required,
                                default: p.default,
                                description: p.description,
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .ok_or_else(|| format!("Tool '{}' not found", tool_id))
    }

    pub async fn execute_tool(
        &self,
        req: ExecuteToolRequest,
    ) -> Result<ExecuteToolResponse, String> {
        let tool_id = req.tool_id.clone();

        let target = req.target.ok_or_else(|| "Target is required".to_string())?;
        let target = match target.target_type.as_str() {
            "url" | "1" => Target::url(target.value),
            "domain" | "2" => Target::domain(target.value),
            "ip" | "3" => Target::ip(target.value),
            "cidr" | "4" => Target::cidr(target.value),
            _ => Target::url(target.value),
        };

        let options = req
            .options
            .map(|o| RequestOptions {
                timeout_ms: o.timeout_ms,
                concurrency: o.concurrency.map(|c| c as usize),
                rate_limit: o.rate_limit,
                proxy: o.proxy,
                headers: o.headers,
                auth: None,
                stealth: o.stealth.unwrap_or(false),
                follow_redirects: o.follow_redirects.unwrap_or(true),
                verify_ssl: o.verify_ssl.unwrap_or(true),
            })
            .unwrap_or_default();

        let request = ToolRequest::new(tool_id.clone(), target)
            .with_params(req.params.unwrap_or_default())
            .with_options(options);

        let response = self
            .dispatcher
            .dispatch(request)
            .await
            .map_err(|e| e.to_string())?;

        Ok(ExecuteToolResponse {
            request_id: response.request_id,
            tool_id: response.tool_id,
            status: response.status.to_string(),
            results: Some(response.results),
            metadata: GrpcResponseMetadata {
                started_at: response.metadata.started_at.to_rfc3339(),
                completed_at: response.metadata.completed_at.to_rfc3339(),
                duration_ms: response.metadata.duration_ms,
                targets_scanned: response.metadata.targets_scanned as u32,
                findings_count: response.metadata.findings_count as u32,
            },
            errors: response
                .errors
                .into_iter()
                .map(|e| GrpcToolError {
                    code: e.code,
                    message: e.message,
                    details: e.details,
                    target: e.target,
                })
                .collect(),
            findings: response
                .findings
                .into_iter()
                .map(|f| GrpcFinding {
                    id: f.id,
                    finding_type: f.finding_type.to_string(),
                    severity: f.severity.to_string(),
                    title: f.title,
                    description: f.description,
                    location: f.location,
                    evidence: f.evidence,
                    cve_ids: f.cve_ids,
                    remediation: f.remediation,
                    references: f.references,
                })
                .collect(),
        })
    }

    pub async fn get_capabilities(
        &self,
        tool_id: Option<String>,
    ) -> Result<CapabilitiesResponse, String> {
        if let Some(id) = tool_id {
            let tool = self.get_tool(&id).await?;
            Ok(CapabilitiesResponse {
                capabilities: tool.capabilities,
            })
        } else {
            let tools = self.registry.list();
            let caps: Vec<GrpcToolCapability> = tools
                .into_iter()
                .flat_map(|t| {
                    t.capabilities.into_iter().map(|c| GrpcToolCapability {
                        name: c.name,
                        description: c.description,
                        parameters: c
                            .parameters
                            .into_iter()
                            .map(|p| GrpcParameterDef {
                                name: p.name,
                                param_type: p.param_type.to_string(),
                                required: p.required,
                                default: p.default,
                                description: p.description,
                            })
                            .collect(),
                    })
                })
                .collect();
            Ok(CapabilitiesResponse { capabilities: caps })
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GrpcToolInfo {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub protocols: Vec<String>,
    pub capabilities: Vec<GrpcToolCapability>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GrpcToolCapability {
    pub name: String,
    pub description: String,
    pub parameters: Vec<GrpcParameterDef>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GrpcParameterDef {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub default: Option<serde_json::Value>,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GrpcResponseMetadata {
    pub started_at: String,
    pub completed_at: String,
    pub duration_ms: u64,
    pub targets_scanned: u32,
    pub findings_count: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GrpcToolError {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub target: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GrpcFinding {
    pub id: String,
    pub finding_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub location: String,
    pub evidence: Option<String>,
    pub cve_ids: Vec<String>,
    pub remediation: Option<String>,
    pub references: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListToolsRequest {
    pub category: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListToolsResponse {
    pub tools: Vec<GrpcToolInfo>,
    pub categories: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetToolRequest {
    pub tool_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetToolResponse {
    pub tool: Option<GrpcToolInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecuteToolRequest {
    pub tool_id: String,
    pub target: Option<GrpcTarget>,
    pub params: Option<serde_json::Value>,
    pub options: Option<GrpcRequestOptions>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecuteToolResponse {
    pub request_id: String,
    pub tool_id: String,
    pub status: String,
    pub results: Option<serde_json::Value>,
    pub metadata: GrpcResponseMetadata,
    pub errors: Vec<GrpcToolError>,
    pub findings: Vec<GrpcFinding>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GrpcTarget {
    pub target_type: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GrpcRequestOptions {
    pub timeout_ms: Option<u64>,
    pub concurrency: Option<u32>,
    pub rate_limit: Option<f64>,
    pub proxy: Option<String>,
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub stealth: Option<bool>,
    pub follow_redirects: Option<bool>,
    pub verify_ssl: Option<bool>,
    pub auth: Option<GrpcAuth>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GrpcAuth {
    pub auth_type: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CapabilitiesRequest {
    pub tool_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CapabilitiesResponse {
    pub capabilities: Vec<GrpcToolCapability>,
}
