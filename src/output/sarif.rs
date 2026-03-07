#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifReport {
    pub version: String,
    #[serde(rename = "$schema")]
    pub schema: String,
    pub runs: Vec<SarifRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifRun {
    pub tool: SarifTool,
    pub results: Vec<SarifResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invocations: Option<Vec<SarifInvocation>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifTool {
    pub driver: SarifDriver,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifDriver {
    pub name: String,
    pub version: String,
    pub information_uri: String,
    pub rules: Vec<SarifRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifRule {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_description: Option<SarifMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_description: Option<SarifMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_configuration: Option<SarifConfiguration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifMessage {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifConfiguration {
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifResult {
    pub rule_id: String,
    pub level: String,
    pub message: SarifMessage,
    pub locations: Vec<SarifLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifLocation {
    pub physical_location: SarifPhysicalLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifPhysicalLocation {
    pub artifact_location: SarifArtifactLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<SarifRegion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifArtifactLocation {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifRegion {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_column: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifInvocation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time_utc: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time_utc: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_successful: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_execution_notifications: Option<Vec<SarifNotification>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifNotification {
    pub level: String,
    pub message: SarifMessage,
}

pub struct SarifBuilder {
    tool_name: String,
    tool_version: String,
    rules: Vec<SarifRule>,
    results: Vec<SarifResult>,
    start_time: DateTime<Utc>,
}

impl SarifBuilder {
    pub fn new() -> Self {
        Self {
            tool_name: "Slapper".to_string(),
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            rules: Vec::new(),
            results: Vec::new(),
            start_time: Utc::now(),
        }
    }

    pub fn with_tool(mut self, name: String, version: String) -> Self {
        self.tool_name = name;
        self.tool_version = version;
        self
    }

    pub fn with_report(mut self, report: &crate::pipeline::PipelineReport) -> Self {
        for port in &report.open_ports {
            if port.status == "open" {
                let rule_id = format!("PORT-{}", port.port);
                self = self.add_rule(
                    &rule_id,
                    &format!("Open Port {}", port.port),
                    "note",
                    &format!("Open port {} detected on {}", port.port, report.target),
                );
                self = self.add_result(
                    &rule_id,
                    "note",
                    &format!("Port {} is open", port.port),
                    &format!("{}:{}", report.target, port.port),
                );
            }
        }

        for service in &report.services {
            let rule_id = format!("SERVICE-{}", service.service);
            self = self.add_rule(
                &rule_id,
                &service.service.clone(),
                "note",
                &format!(
                    "Detected {} service version {}",
                    service.service,
                    service.version.as_deref().unwrap_or("unknown")
                ),
            );
            self = self.add_result(
                &rule_id,
                "note",
                &format!(
                    "Service: {} {}",
                    service.service,
                    service.version.as_deref().unwrap_or("")
                ),
                &format!("{}:{}", report.target, service.port),
            );
        }

        for endpoint in &report.endpoints {
            let rule_id = "ENDPOINT".to_string();
            self = self.add_rule(
                &rule_id,
                "Discovered Endpoint",
                "note",
                &format!("Found endpoint: {}", endpoint.path),
            );
            self = self.add_result(
                &rule_id,
                "note",
                &format!(
                    "{} - {} ({})",
                    endpoint.path, endpoint.status_code, endpoint.status_text
                ),
                &endpoint.path,
            );
        }

        self
    }

    pub fn add_rule(mut self, id: &str, name: &str, level: &str, description: &str) -> Self {
        self.rules.push(SarifRule {
            id: id.to_string(),
            name: name.to_string(),
            short_description: Some(SarifMessage {
                text: description.to_string(),
            }),
            full_description: None,
            default_configuration: Some(SarifConfiguration {
                level: level.to_string(),
            }),
            help_uri: None,
        });
        self
    }

    pub fn add_result(
        mut self,
        rule_id: &str,
        level: &str,
        message: &str,
        location_uri: &str,
    ) -> Self {
        self.results.push(SarifResult {
            rule_id: rule_id.to_string(),
            level: level.to_string(),
            message: SarifMessage {
                text: message.to_string(),
            },
            locations: vec![SarifLocation {
                physical_location: SarifPhysicalLocation {
                    artifact_location: SarifArtifactLocation {
                        uri: location_uri.to_string(),
                    },
                    region: None,
                },
            }],
            properties: None,
        });
        self
    }

    pub fn add_result_with_properties(
        mut self,
        rule_id: &str,
        level: &str,
        message: &str,
        location_uri: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Self {
        self.results.push(SarifResult {
            rule_id: rule_id.to_string(),
            level: level.to_string(),
            message: SarifMessage {
                text: message.to_string(),
            },
            locations: vec![SarifLocation {
                physical_location: SarifPhysicalLocation {
                    artifact_location: SarifArtifactLocation {
                        uri: location_uri.to_string(),
                    },
                    region: None,
                },
            }],
            properties: Some(properties),
        });
        self
    }

    pub fn build(self) -> SarifReport {
        SarifReport {
            version: "2.1.0".to_string(),
            schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json".to_string(),
            runs: vec![SarifRun {
                tool: SarifTool {
                    driver: SarifDriver {
                        name: self.tool_name,
                        version: self.tool_version,
                        information_uri: "https://github.com/slapper-tool/slapper".to_string(),
                        rules: self.rules,
                    },
                },
                results: self.results,
                invocations: Some(vec![SarifInvocation {
                    start_time_utc: Some(self.start_time),
                    end_time_utc: Some(Utc::now()),
                    execution_successful: Some(true),
                    tool_execution_notifications: None,
                }]),
            }],
        }
    }
}

impl Default for SarifBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SarifReport {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sarif_builder() {
        let report = SarifBuilder::new()
            .add_rule(
                "SQLI001",
                "SQL Injection",
                "error",
                "SQL injection vulnerability detected",
            )
            .add_result(
                "SQLI001",
                "error",
                "Potential SQL injection in parameter 'id'",
                "https://example.com/api/users?id=1",
            )
            .build();

        assert_eq!(report.version, "2.1.0");
        assert_eq!(report.runs.len(), 1);
        assert_eq!(report.runs[0].results.len(), 1);
    }
}
