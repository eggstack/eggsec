use crate::tool::protocol::mcp::types::CapabilitySummary;
use crate::tool::registry::ToolInfo;

pub fn build_capabilities_summary(info: &ToolInfo) -> Vec<CapabilitySummary> {
    info.capabilities
        .iter()
        .map(|cap| CapabilitySummary {
            name: cap.name.clone(),
            description: cap.description.clone(),
            attack_surface: cap
                .attack_surface
                .iter()
                .map(|s| format!("{:?}", s).to_lowercase())
                .collect(),
            severity_potential: cap
                .severity_potential
                .iter()
                .map(|s| format!("{}", s))
                .collect(),
        })
        .collect()
}

pub fn build_input_schema(info: &ToolInfo) -> serde_json::Value {
    let mut properties = serde_json::Map::new();

    properties.insert(
        "target".to_string(),
        serde_json::json!({
            "type": "string",
            "description": "Target URL, domain, or IP address"
        }),
    );

    properties.insert(
        "target_type".to_string(),
        serde_json::json!({
            "type": "string",
            "description": "Type of target: url, domain, ip, or cidr",
            "enum": ["url", "domain", "ip", "cidr"],
            "default": "url"
        }),
    );

    for cap in &info.capabilities {
        for param in &cap.parameters {
            let mut param_schema = serde_json::json!({
                "type": param.param_type.to_string(),
                "description": param.description,
            });

            if let Some(ref default) = param.default {
                param_schema["default"] = default.clone();
            }

            properties.insert(param.name.clone(), param_schema);
        }
    }

    let required: Vec<String> = vec!["target".to_string()];

    serde_json::json!({
        "type": "object",
        "properties": properties,
        "required": required
    })
}