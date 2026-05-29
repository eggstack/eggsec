use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::tool::ToolRegistry;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: Info,
    pub servers: Vec<Server>,
    pub paths: FxHashMap<String, PathItem>,
    pub components: Components,
    pub tags: Vec<Tag>,
}

impl OpenApiSpec {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self)
            .inspect_err(|e| {
                tracing::warn!(error = %e, "Failed to serialize OpenAPI spec to JSON");
            })
            .unwrap_or_default()
    }

    pub fn to_yaml(&self) -> String {
        serde_yaml_neo::to_string(self).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    pub title: String,
    pub description: String,
    pub version: String,
    pub contact: Option<Contact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub url: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathItem {
    pub get: Option<Operation>,
    pub post: Option<Operation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub tags: Vec<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub operation_id: String,
    pub parameters: Vec<Parameter>,
    pub request_body: Option<RequestBody>,
    pub responses: FxHashMap<String, Response>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "in")]
    pub location: String,
    pub description: Option<String>,
    pub required: bool,
    pub schema_: Schema,
}

impl Parameter {
    pub fn new(name: &str, location: &str, required: bool, schema: Schema) -> Self {
        Self {
            name: name.to_string(),
            location: location.to_string(),
            description: None,
            required,
            schema_: schema,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Schema {
    Type(String),
    Object(SchemaObject),
    Ref(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaObject {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: Option<FxHashMap<String, Box<Schema>>>,
    pub required: Option<Vec<String>>,
}

impl Schema {
    pub fn string() -> Self {
        Schema::Type("string".to_string())
    }

    pub fn integer() -> Self {
        Schema::Type("integer".to_string())
    }

    pub fn boolean() -> Self {
        Schema::Type("boolean".to_string())
    }

    pub fn number() -> Self {
        Schema::Type("number".to_string())
    }

    pub fn array(items: Schema) -> Self {
        Schema::Object(SchemaObject {
            schema_type: "array".to_string(),
            properties: Some(
                [("items".to_string(), Box::new(items))]
                    .into_iter()
                    .collect(),
            ),
            required: None,
        })
    }

    pub fn object(properties: Vec<(&str, Schema)>, required: Vec<&str>) -> Self {
        let props: FxHashMap<String, Box<Schema>> = properties
            .into_iter()
            .map(|(k, v)| (k.to_string(), Box::new(v)))
            .collect();
        Schema::Object(SchemaObject {
            schema_type: "object".to_string(),
            properties: Some(props),
            required: Some(required.into_iter().map(|s| s.to_string()).collect()),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    pub description: Option<String>,
    pub content: FxHashMap<String, MediaType>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaType {
    pub schema_: Schema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub description: String,
    pub content: Option<FxHashMap<String, MediaType>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Components {
    pub schemas: FxHashMap<String, Schema>,
    pub security_schemes: Option<FxHashMap<String, SecurityScheme>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScheme {
    #[serde(rename = "type")]
    pub scheme_type: String,
    pub name: Option<String>,
    #[serde(rename = "in")]
    pub location: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub description: String,
}

#[derive(Serialize)]
pub struct OpenApiGenerator {
    base_url: String,
    version: String,
}

impl OpenApiGenerator {
    pub fn new(base_url: &str, version: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            version: version.to_string(),
        }
    }

    pub fn generate(&self, registry: &ToolRegistry) -> OpenApiSpec {
        let tools = registry.list();

        let mut paths = FxHashMap::default();
        let mut schemas = FxHashMap::default();
        let mut tags: Vec<Tag> = Vec::new();

        let mut categories: std::collections::HashSet<String> = std::collections::HashSet::new();

        for tool in &tools {
            categories.insert(format!("{:?}", tool.category));
        }

        for category in categories {
            tags.push(Tag {
                name: category.clone(),
                description: format!("{} tools", category),
            });
        }

        for tool in &tools {
            let category = format!("{:?}", tool.category);

            let mut properties: FxHashMap<String, Box<Schema>> = FxHashMap::default();
            properties.insert("target".to_string(), Box::new(Schema::string()));
            properties.insert("target_type".to_string(), Box::new(Schema::string()));
            properties.insert("api_key".to_string(), Box::new(Schema::string()));
            properties.insert("timeout_ms".to_string(), Box::new(Schema::integer()));
            properties.insert("concurrency".to_string(), Box::new(Schema::integer()));

            for cap in &tool.capabilities {
                for param in &cap.parameters {
                    let schema = match param.param_type {
                        crate::tool::traits::ParameterType::String => Schema::string(),
                        crate::tool::traits::ParameterType::Integer => Schema::integer(),
                        crate::tool::traits::ParameterType::Boolean => Schema::boolean(),
                        crate::tool::traits::ParameterType::Float => Schema::number(),
                        crate::tool::traits::ParameterType::Array => {
                            Schema::array(Schema::string())
                        }
                        crate::tool::traits::ParameterType::Url => Schema::string(),
                        crate::tool::traits::ParameterType::Ip => Schema::string(),
                        crate::tool::traits::ParameterType::Domain => Schema::string(),
                        _ => Schema::string(),
                    };
                    properties.insert(param.name.clone(), Box::new(schema));
                }
            }

            let schema_name = format!("{}Request", tool.id);
            schemas.insert(
                schema_name.clone(),
                Schema::Object(SchemaObject {
                    schema_type: "object".to_string(),
                    properties: Some(properties.clone()),
                    required: Some(vec!["target".to_string()]),
                }),
            );

            schemas.insert(
                format!("{}Response", tool.id),
                Schema::Object(SchemaObject {
                    schema_type: "object".to_string(),
                    properties: Some(
                        [
                            ("request_id".to_string(), Box::new(Schema::string())),
                            ("tool_id".to_string(), Box::new(Schema::string())),
                            ("status".to_string(), Box::new(Schema::string())),
                            (
                                "results".to_string(),
                                Box::new(Schema::Object(SchemaObject {
                                    schema_type: "object".to_string(),
                                    properties: None,
                                    required: None,
                                })),
                            ),
                            (
                                "metadata".to_string(),
                                Box::new(Schema::Object(SchemaObject {
                                    schema_type: "object".to_string(),
                                    properties: None,
                                    required: None,
                                })),
                            ),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                    required: Some(vec![
                        "request_id".to_string(),
                        "tool_id".to_string(),
                        "status".to_string(),
                    ]),
                }),
            );

            let tool_params: Vec<Parameter> = vec![
                Parameter::new("name", "query", true, Schema::string()),
                Parameter::new(
                    "arguments",
                    "body",
                    false,
                    Schema::Object(SchemaObject {
                        schema_type: "object".to_string(),
                        properties: Some(properties),
                        required: None,
                    }),
                ),
                Parameter::new("api_key", "query", false, Schema::string()),
            ];

            let responses = [
                (
                    "200".to_string(),
                    Response {
                        description: "Successful response".to_string(),
                        content: Some(
                            [(
                                "application/json".to_string(),
                                MediaType {
                                    schema_: Schema::Ref(format!(
                                        "#/components/schemas/{}Response",
                                        tool.id
                                    )),
                                },
                            )]
                            .into_iter()
                            .collect(),
                        ),
                    },
                ),
                (
                    "400".to_string(),
                    Response {
                        description: "Bad request".to_string(),
                        content: None,
                    },
                ),
                (
                    "401".to_string(),
                    Response {
                        description: "Unauthorized".to_string(),
                        content: None,
                    },
                ),
                (
                    "429".to_string(),
                    Response {
                        description: "Rate limit exceeded".to_string(),
                        content: None,
                    },
                ),
            ]
            .into_iter()
            .collect();

            paths.insert(
                format!("/mcp/{}", tool.id),
                PathItem {
                    post: Some(Operation {
                        tags: vec![category.clone()],
                        summary: Some(tool.name.to_string()),
                        description: Some(tool.description.to_string()),
                        operation_id: format!("execute_{}", tool.id),
                        parameters: vec![],
                        request_body: Some(RequestBody {
                            description: Some("Tool execution parameters".to_string()),
                            content: [(
                                "application/json".to_string(),
                                MediaType {
                                    schema_: Schema::Object(SchemaObject {
                                        schema_type: "object".to_string(),
                                        properties: [
                                            ("name".to_string(), Box::new(Schema::string())),
                                            (
                                                "arguments".to_string(),
                                                Box::new(Schema::Ref(format!(
                                                    "#/components/schemas/{}",
                                                    schema_name
                                                ))),
                                            ),
                                            ("api_key".to_string(), Box::new(Schema::string())),
                                        ]
                                        .into_iter()
                                        .collect(),
                                        required: Some(vec!["name".to_string()]),
                                    }),
                                },
                            )]
                            .into_iter()
                            .collect(),
                            required: true,
                        }),
                        responses,
                    }),
                    get: None,
                },
            );

            paths.insert(
                "/mcp".to_string(),
                PathItem {
                    get: Some(Operation {
                        tags: vec![category.clone()],
                        summary: Some("List tools".to_string()),
                        description: Some("List all available tools".to_string()),
                        operation_id: "list_tools".to_string(),
                        parameters: vec![],
                        request_body: None,
                        responses: [(
                            "200".to_string(),
                            Response {
                                description: "List of tools".to_string(),
                                content: None,
                            },
                        )]
                        .into_iter()
                        .collect(),
                    }),
                    post: Some(Operation {
                        tags: vec![category],
                        summary: Some("Execute tool".to_string()),
                        description: Some("Execute a tool with the given parameters".to_string()),
                        operation_id: "execute_tool".to_string(),
                        parameters: tool_params,
                        request_body: None,
                        responses: [(
                            "200".to_string(),
                            Response {
                                description: "Tool execution result".to_string(),
                                content: None,
                            },
                        )]
                        .into_iter()
                        .collect(),
                    }),
                },
            );
        }

        let mut security_schemes = FxHashMap::default();
        security_schemes.insert(
            "ApiKeyAuth".to_string(),
            SecurityScheme {
                scheme_type: "apiKey".to_string(),
                name: Some("api_key".to_string()),
                location: Some("query".to_string()),
                description: Some("API key authentication".to_string()),
            },
        );

        paths.insert(
            "/health".to_string(),
            PathItem {
                get: Some(Operation {
                    tags: vec!["System".to_string()],
                    summary: Some("Health check".to_string()),
                    description: Some("Check if the service is healthy".to_string()),
                    operation_id: "health_check".to_string(),
                    parameters: vec![],
                    request_body: None,
                    responses: [(
                        "200".to_string(),
                        Response {
                            description: "Service is healthy".to_string(),
                            content: None,
                        },
                    )]
                    .into_iter()
                    .collect(),
                }),
                post: None,
            },
        );

        paths.insert(
            "/mcp/stream/{request_id}".to_string(),
            PathItem {
                get: Some(Operation {
                    tags: vec!["Streaming".to_string()],
                    summary: Some("SSE stream".to_string()),
                    description: Some(
                        "Subscribe to real-time updates via Server-Sent Events".to_string(),
                    ),
                    operation_id: "stream_events".to_string(),
                    parameters: vec![Parameter::new("request_id", "path", true, Schema::string())],
                    request_body: None,
                    responses: [(
                        "200".to_string(),
                        Response {
                            description: "SSE stream".to_string(),
                            content: Some(
                                [(
                                    "text/event-stream".to_string(),
                                    MediaType {
                                        schema_: Schema::Object(SchemaObject {
                                            schema_type: "object".to_string(),
                                            properties: None,
                                            required: None,
                                        }),
                                    },
                                )]
                                .into_iter()
                                .collect(),
                            ),
                        },
                    )]
                    .into_iter()
                    .collect(),
                }),
                post: None,
            },
        );

        OpenApiSpec {
            openapi: "3.1.0".to_string(),
            info: Info {
                title: "Slapper Security Toolkit API".to_string(),
                description: "High-performance security testing toolkit for AI agents".to_string(),
                version: self.version.clone(),
                contact: Some(Contact {
                    name: "Slapper Team".to_string(),
                    url: "https://github.com/dbowm91/slapper".to_string(),
                }),
            },
            servers: vec![Server {
                url: self.base_url.clone(),
                description: Some("Production server".to_string()),
            }],
            paths,
            components: Components {
                schemas,
                security_schemes: Some(security_schemes),
            },
            tags,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self)
            .inspect_err(|e| {
                tracing::warn!(error = %e, "Failed to serialize OpenApiGenerator to JSON");
            })
            .unwrap_or_default()
    }

    pub fn to_yaml(&self) -> String {
        serde_yaml_neo::to_string(&self).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::ToolRegistry;

    #[test]
    fn test_generate_openapi() {
        let registry = ToolRegistry::new();
        let generator = OpenApiGenerator::new("http://localhost:8080", "0.1.0");
        let spec = generator.generate(&registry);

        assert_eq!(spec.openapi, "3.1.0");
        assert_eq!(spec.info.title, "Slapper Security Toolkit API");
        assert!(!spec.paths.is_empty());
        assert!(spec.paths.contains_key("/health"));
    }

    #[test]
    fn test_openapi_json_output() {
        let registry = ToolRegistry::new();
        let generator = OpenApiGenerator::new("http://localhost:8080", "0.1.0");
        let spec = generator.generate(&registry);
        let json = spec.to_json();

        assert!(json.contains("openapi"));
        assert!(json.contains("Slapper"));
    }
}
