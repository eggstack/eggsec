use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpoint {
    pub path: String,
    pub method: String,
    pub operation_id: Option<String>,
    pub summary: Option<String>,
    pub parameters: Vec<ApiParameter>,
    pub request_body: Option<ApiRequestBody>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiParameter {
    pub name: String,
    pub location: ParameterLocation,
    pub required: bool,
    pub schema: Option<serde_json::Value>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterLocation {
    Query,
    Header,
    Path,
    Cookie,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequestBody {
    pub content_type: Option<String>,
    pub schema: Option<serde_json::Value>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSchema {
    pub title: Option<String>,
    pub version: Option<String>,
    pub base_url: Option<String>,
    pub endpoints: Vec<ApiEndpoint>,
    pub security_schemes: Vec<SecurityScheme>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScheme {
    pub name: String,
    pub scheme_type: String,
    pub location: Option<String>,
}

pub fn parse_openapi(content: &str, is_yaml: bool) -> anyhow::Result<ApiSchema> {
    let value: serde_json::Value = if is_yaml {
        let yaml_value: serde_yaml_neo::Value = serde_yaml_neo::from_str(content)?;
        serde_json::to_value(yaml_value)?
    } else {
        serde_json::from_str(content)?
    };

    let info = value.get("info");
    let title = info
        .and_then(|i| i.get("title"))
        .and_then(|t| t.as_str())
        .map(String::from);
    let version = info
        .and_then(|i| i.get("version"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let servers = value.get("servers").and_then(|s| s.as_array());
    let base_url = servers
        .and_then(|s| s.first())
        .and_then(|s| s.get("url"))
        .and_then(|u| u.as_str())
        .map(String::from);

    let mut endpoints = Vec::new();
    let paths = value.get("paths").and_then(|p| p.as_object());

    if let Some(paths_obj) = paths {
        for (path, path_item) in paths_obj {
            for method in &["get", "post", "put", "delete", "patch", "options", "head"] {
                if let Some(operation) = path_item.get(*method) {
                    let endpoint = parse_operation(path, method, operation)?;
                    endpoints.push(endpoint);
                }
            }
        }
    }

    let security_schemes = parse_security_schemes(&value);

    Ok(ApiSchema {
        title,
        version,
        base_url,
        endpoints,
        security_schemes,
    })
}

fn parse_operation(
    path: &str,
    method: &str,
    operation: &serde_json::Value,
) -> anyhow::Result<ApiEndpoint> {
    let operation_id = operation
        .get("operationId")
        .and_then(|o| o.as_str())
        .map(String::from);
    let summary = operation
        .get("summary")
        .and_then(|s| s.as_str())
        .map(String::from);

    let tags = operation
        .get("tags")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let parameters = operation
        .get("parameters")
        .and_then(|p| p.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|p| match parse_parameter(p) {
                    Ok(param) => Some(param),
                    Err(e) => {
                        tracing::warn!("Malformed OpenAPI parameter: {}", e);
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    let request_body = operation
        .get("requestBody")
        .and_then(|rb| parse_request_body(rb).ok());

    Ok(ApiEndpoint {
        path: path.to_string(),
        method: method.to_uppercase(),
        operation_id,
        summary,
        parameters,
        request_body,
        tags,
    })
}

fn parse_parameter(param: &serde_json::Value) -> anyhow::Result<ApiParameter> {
    let name = param
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_string();
    let location = match param.get("in").and_then(|l| l.as_str()).unwrap_or("query") {
        "query" => ParameterLocation::Query,
        "header" => ParameterLocation::Header,
        "path" => ParameterLocation::Path,
        "cookie" => ParameterLocation::Cookie,
        _ => ParameterLocation::Query,
    };
    let required = param
        .get("required")
        .and_then(|r| r.as_bool())
        .unwrap_or(false);
    let schema = param.get("schema").cloned();
    let description = param
        .get("description")
        .and_then(|d| d.as_str())
        .map(String::from);

    Ok(ApiParameter {
        name,
        location,
        required,
        schema,
        description,
    })
}

fn parse_request_body(rb: &serde_json::Value) -> anyhow::Result<ApiRequestBody> {
    let required = rb
        .get("required")
        .and_then(|r| r.as_bool())
        .unwrap_or(false);
    let content = rb.get("content").and_then(|c| c.as_object());

    let (content_type, schema) = if let Some(content_obj) = content {
        let ct = content_obj.keys().next().map(String::from);
        let schema = content_obj
            .values()
            .next()
            .and_then(|v| v.get("schema"))
            .cloned();
        (ct, schema)
    } else {
        (None, None)
    };

    Ok(ApiRequestBody {
        content_type,
        schema,
        required,
    })
}

fn parse_security_schemes(value: &serde_json::Value) -> Vec<SecurityScheme> {
    let components = value
        .get("components")
        .and_then(|c| c.get("securitySchemes"));
    let schemes = components.and_then(|s| s.as_object());

    schemes
        .map(|obj| {
            obj.iter()
                .filter_map(|(name, scheme)| {
                    let scheme_type = scheme
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let location = match scheme_type.as_str() {
                        "apiKey" => scheme.get("in").and_then(|i| i.as_str()).map(String::from),
                        "http" => Some(
                            scheme
                                .get("scheme")
                                .and_then(|s| s.as_str())
                                .unwrap_or("bearer")
                                .to_string(),
                        ),
                        _ => None,
                    };
                    Some(SecurityScheme {
                        name: name.clone(),
                        scheme_type,
                        location,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn generate_fuzz_targets(schema: &ApiSchema) -> Vec<FuzzTarget> {
    let mut targets = Vec::new();

    for endpoint in &schema.endpoints {
        for param in &endpoint.parameters {
            targets.push(FuzzTarget {
                path: endpoint.path.clone(),
                method: endpoint.method.clone(),
                parameter: param.name.clone(),
                location: format!("{:?}", param.location),
                schema_hint: param.schema.as_ref().map(|s| s.to_string()),
            });
        }

        if let Some(ref body) = endpoint.request_body {
            targets.push(FuzzTarget {
                path: endpoint.path.clone(),
                method: endpoint.method.clone(),
                parameter: "request_body".to_string(),
                location: "body".to_string(),
                schema_hint: body.schema.as_ref().map(|s| s.to_string()),
            });
        }
    }

    targets
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzTarget {
    pub path: String,
    pub method: String,
    pub parameter: String,
    pub location: String,
    pub schema_hint: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_OPENAPI: &str = r#"{
        "openapi": "3.0.0",
        "info": {"title": "Test API", "version": "1.0.0"},
        "servers": [{"url": "https://api.example.com"}],
        "paths": {
            "/users": {
                "get": {
                    "operationId": "listUsers",
                    "summary": "List all users",
                    "tags": ["users"],
                    "parameters": [
                        {"name": "limit", "in": "query", "required": false, "schema": {"type": "integer"}},
                        {"name": "Authorization", "in": "header", "required": true}
                    ]
                },
                "post": {
                    "operationId": "createUser",
                    "summary": "Create a user",
                    "tags": ["users"],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {"type": "object", "properties": {"name": {"type": "string"}}}
                            }
                        }
                    }
                }
            }
        },
        "components": {
            "securitySchemes": {
                "bearerAuth": {"type": "http", "scheme": "bearer"}
            }
        }
    }"#;

    const SAMPLE_OPENAPI_YAML: &str = r#"openapi: "3.0.0"
info:
  title: "YAML API"
  version: "2.0.0"
servers:
  - url: "https://yaml.example.com"
paths:
  /items:
    get:
      operationId: listItems
      summary: List items
      tags: [items]
      parameters:
        - name: page
          in: query
          required: false
          schema:
            type: integer
    post:
      operationId: createItem
      summary: Create item
      tags: [items]
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              properties:
                title:
                  type: string
components:
  securitySchemes:
    apiKey:
      type: apiKey
      in: header
      name: X-API-Key
"#;

    #[test]
    fn parse_openapi_json() {
        let schema = parse_openapi(SAMPLE_OPENAPI, false).unwrap();
        assert_eq!(schema.title, Some("Test API".to_string()));
        assert_eq!(schema.version, Some("1.0.0".to_string()));
        assert_eq!(schema.base_url, Some("https://api.example.com".to_string()));
        assert_eq!(schema.endpoints.len(), 2);
        assert_eq!(schema.security_schemes.len(), 1);
    }

    #[test]
    fn parse_openapi_yaml() {
        let schema = parse_openapi(SAMPLE_OPENAPI_YAML, true).unwrap();
        assert_eq!(schema.title, Some("YAML API".to_string()));
        assert_eq!(schema.version, Some("2.0.0".to_string()));
        assert_eq!(
            schema.base_url,
            Some("https://yaml.example.com".to_string())
        );
        assert_eq!(schema.endpoints.len(), 2);
        assert_eq!(schema.security_schemes.len(), 1);
    }

    #[test]
    fn generate_fuzz_targets_from_schema() {
        let schema = parse_openapi(SAMPLE_OPENAPI, false).unwrap();
        let targets = generate_fuzz_targets(&schema);
        assert!(!targets.is_empty());
        assert!(targets.iter().any(|t| t.parameter == "limit"));
        assert!(targets.iter().any(|t| t.parameter == "request_body"));
    }

    #[test]
    fn parse_parameters_correctly() {
        let schema = parse_openapi(SAMPLE_OPENAPI, false).unwrap();
        let get_users = &schema.endpoints[0];
        assert_eq!(get_users.path, "/users");
        assert_eq!(get_users.method, "GET");
        assert_eq!(get_users.operation_id, Some("listUsers".to_string()));
        assert_eq!(get_users.summary, Some("List all users".to_string()));
        assert_eq!(get_users.tags, vec!["users".to_string()]);
        assert_eq!(get_users.parameters.len(), 2);

        let limit_param = &get_users.parameters[0];
        assert_eq!(limit_param.name, "limit");
        assert!(matches!(limit_param.location, ParameterLocation::Query));
        assert!(!limit_param.required);

        let auth_param = &get_users.parameters[1];
        assert_eq!(auth_param.name, "Authorization");
        assert!(matches!(auth_param.location, ParameterLocation::Header));
        assert!(auth_param.required);
    }

    #[test]
    fn parse_request_body_correctly() {
        let schema = parse_openapi(SAMPLE_OPENAPI, false).unwrap();
        let post_users = &schema.endpoints[1];
        assert_eq!(post_users.path, "/users");
        assert_eq!(post_users.method, "POST");
        let body = post_users.request_body.as_ref().unwrap();
        assert!(body.required);
        assert_eq!(body.content_type, Some("application/json".to_string()));
        assert!(body.schema.is_some());
    }

    #[test]
    fn parse_security_schemes_correctly() {
        let schema = parse_openapi(SAMPLE_OPENAPI, false).unwrap();
        let bearer = &schema.security_schemes[0];
        assert_eq!(bearer.name, "bearerAuth");
        assert_eq!(bearer.scheme_type, "http");
        assert_eq!(bearer.location, Some("bearer".to_string()));
    }

    #[test]
    fn parse_minimal_schema() {
        let minimal = r#"{"openapi": "3.0.0", "paths": {}}"#;
        let schema = parse_openapi(minimal, false).unwrap();
        assert!(schema.title.is_none());
        assert!(schema.version.is_none());
        assert!(schema.base_url.is_none());
        assert!(schema.endpoints.is_empty());
        assert!(schema.security_schemes.is_empty());
    }
}
