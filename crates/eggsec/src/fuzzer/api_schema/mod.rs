use crate::error::Result;
use crate::fuzzer::payloads::{Payload, PayloadType};
use crate::types::Severity;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

const OVERSIZED_PAYLOAD_SIZES: [usize; 4] = [1_000, 10_000, 100_000, 1_000_000];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpoint {
    pub path: String,
    pub method: String,
    pub parameters: Vec<ApiParameter>,
    pub request_body: Option<RequestBody>,
    pub security: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiParameter {
    pub name: String,
    pub location: ParamLocation,
    pub required: bool,
    pub param_type: String,
    pub format: Option<String>,
    pub example: Option<String>,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
    pub pattern: Option<String>,
    pub enum_values: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ParamLocation {
    Path,
    Query,
    Header,
    Cookie,
    Body,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    pub content_type: String,
    pub schema: serde_json::Value,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaFuzzTarget {
    pub endpoint: ApiEndpoint,
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaFuzzResult {
    pub endpoint: String,
    pub method: String,
    pub test_type: String,
    pub status_code: Option<u16>,
    pub vulnerable: bool,
    pub details: String,
}

pub struct ApiSchemaFuzzer {
    client: reqwest::Client,
}

impl ApiSchemaFuzzer {
    pub fn new(timeout_secs: u64) -> Result<Self> {
        let client = crate::utils::create_insecure_http_client(timeout_secs)?;
        Ok(Self { client })
    }

    pub fn parse_openapi(content: &str) -> Result<Vec<ApiEndpoint>> {
        let json: serde_json::Value = if content.trim().starts_with('{') {
            serde_json::from_str(content)?
        } else {
            Self::yaml_to_json(content)?
        };

        let mut endpoints = Vec::new();

        if let Some(paths) = json.get("paths").and_then(|v| v.as_object()) {
            for (path, methods) in paths {
                if let Some(methods_obj) = methods.as_object() {
                    for (method, details) in methods_obj {
                        let method_upper = method.to_uppercase();
                        if !["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD"]
                            .contains(&method_upper.as_str())
                        {
                            continue;
                        }

                        let mut parameters = Vec::new();
                        let mut security = Vec::new();
                        let mut request_body = None;

                        if let Some(details_obj) = details.as_object() {
                            if let Some(params) =
                                details_obj.get("parameters").and_then(|v| v.as_array())
                            {
                                for param in params {
                                    if let Some(p) = Self::parse_parameter(param) {
                                        parameters.push(p);
                                    }
                                }
                            }

                            if let Some(rb) = details_obj.get("requestBody") {
                                request_body = Self::parse_request_body(rb);
                            }

                            if let Some(sec) =
                                details_obj.get("security").and_then(|v| v.as_array())
                            {
                                for s in sec {
                                    if let Some(s_obj) = s.as_object() {
                                        for key in s_obj.keys() {
                                            security.push(key.clone());
                                        }
                                    }
                                }
                            }
                        }

                        endpoints.push(ApiEndpoint {
                            path: path.clone(),
                            method: method_upper,
                            parameters,
                            request_body,
                            security,
                        });
                    }
                }
            }
        }

        Ok(endpoints)
    }

    pub fn generate_fuzz_payloads(
        &self,
        endpoints: &[ApiEndpoint],
        base_url: &str,
    ) -> Vec<SchemaFuzzTarget> {
        let mut targets = Vec::new();

        for endpoint in endpoints {
            targets.push(SchemaFuzzTarget {
                endpoint: endpoint.clone(),
                base_url: base_url.to_string(),
            });
        }

        targets
    }

    pub fn generate_type_aware_payloads(&self, param: &ApiParameter) -> Vec<Payload> {
        let mut payloads = Vec::new();

        match param.param_type.as_str() {
            "string" => {
                payloads.extend(Self::string_payload(param));
            }
            "integer" | "number" => {
                payloads.extend(Self::numeric_payload(param));
            }
            "boolean" => {
                payloads.extend(Self::boolean_payload(param));
            }
            "array" => {
                payloads.extend(Self::array_payload(param));
            }
            "object" => {
                payloads.extend(Self::object_payload(param));
            }
            _ => {
                payloads.push(Payload {
                    payload_type: PayloadType::Sqli,
                    payload: "' OR 1=1--".to_string(),
                    description: "Generic injection for unknown type".to_string(),
                    severity: Severity::High,
                    tags: vec![param.name.clone()],
                });
            }
        }

        payloads
    }

    pub fn generate_oversized_payloads(
        &self,
        endpoints: &[ApiEndpoint],
    ) -> Vec<(String, String, String)> {
        let mut payloads = Vec::new();

        for endpoint in endpoints {
            for param in &endpoint.parameters {
                if param.param_type == "string" {
                    for size in OVERSIZED_PAYLOAD_SIZES {
                        let oversized = "A".repeat(size);
                        payloads.push((endpoint.path.clone(), param.name.clone(), oversized));
                    }
                }
            }

            if let Some(ref body) = endpoint.request_body {
                if body.content_type.contains("json") {
                    for size in OVERSIZED_PAYLOAD_SIZES {
                        let oversized_body = format!("{{\"data\": \"{}\"}}", "A".repeat(size));
                        payloads.push((endpoint.path.clone(), "body".to_string(), oversized_body));
                    }
                }
            }
        }

        payloads
    }

    pub fn generate_auth_bypass_payloads(
        &self,
        endpoints: &[ApiEndpoint],
        base_url: &str,
    ) -> Vec<(String, String, FxHashMap<String, String>)> {
        let mut payloads = Vec::new();

        for endpoint in endpoints {
            if !endpoint.security.is_empty() {
                let auth_bypass_headers = vec![
                    ("X-Original-URL".to_string(), endpoint.path.clone()),
                    ("X-Override-URL".to_string(), endpoint.path.clone()),
                    ("X-Rewrite-URL".to_string(), endpoint.path.clone()),
                ];

                for (header, value) in auth_bypass_headers {
                    let mut headers = FxHashMap::default();
                    headers.insert(header, value);
                    payloads.push((
                        base_url.to_string() + &endpoint.path,
                        endpoint.method.clone(),
                        headers,
                    ));
                }

                let mut no_auth = FxHashMap::default();
                no_auth.insert("Authorization".to_string(), "".to_string());
                payloads.push((
                    base_url.to_string() + &endpoint.path,
                    endpoint.method.clone(),
                    no_auth,
                ));
            }
        }

        payloads
    }

    pub fn generate_required_omission_payloads(
        &self,
        endpoints: &[ApiEndpoint],
        base_url: &str,
    ) -> Vec<(String, String)> {
        let mut payloads = Vec::new();

        for endpoint in endpoints {
            let required_params: Vec<&ApiParameter> =
                endpoint.parameters.iter().filter(|p| p.required).collect();

            if !required_params.is_empty() {
                let mut url = format!("{}{}", base_url, endpoint.path);
                for param in &required_params {
                    url = url.replace(&format!("{{{}}}", param.name), "");
                }
                url = url.trim_end_matches('/').to_string();

                payloads.push((endpoint.method.clone(), url));
            }
        }

        payloads
    }

    pub async fn fuzz_endpoint(&self, target: &SchemaFuzzTarget) -> Result<Vec<SchemaFuzzResult>> {
        let mut results = Vec::new();
        let endpoint = &target.endpoint;

        for param in &endpoint.parameters {
            let injection_payloads = self.generate_type_aware_payloads(param);

            for payload in injection_payloads.iter().take(5) {
                let url = Self::build_url(&target.base_url, endpoint, param, &payload.payload);
                let method = &endpoint.method;

                let response = match self.send_request(method, &url, None).await {
                    Ok(r) => r,
                    Err(e) => {
                        results.push(SchemaFuzzResult {
                            endpoint: endpoint.path.clone(),
                            method: method.clone(),
                            test_type: format!("injection:{}", param.name),
                            status_code: None,
                            vulnerable: false,
                            details: format!("Request failed: {}", e),
                        });
                        continue;
                    }
                };

                let status = response.status().as_u16();
                let is_error = status >= 500;
                let body = match response.text().await {
                    Ok(text) => text,
                    Err(e) => {
                        tracing::debug!(
                            "Failed to read response body for {}: {}",
                            endpoint.path,
                            e
                        );
                        String::new()
                    }
                };
                let has_error = body.contains("SQL syntax")
                    || body.contains("stack trace")
                    || body.contains("exception");

                results.push(SchemaFuzzResult {
                    endpoint: endpoint.path.clone(),
                    method: method.clone(),
                    test_type: format!("injection:{}", param.name),
                    status_code: Some(status),
                    vulnerable: is_error || has_error,
                    details: format!("Status: {}, Error indicators: {}", status, has_error),
                });
            }
        }

        Ok(results)
    }

    async fn send_request(
        &self,
        method: &str,
        url: &str,
        body: Option<String>,
    ) -> Result<reqwest::Response> {
        let client = self.client.clone();
        let req = match method {
            "GET" => client.get(url),
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "PATCH" => client.patch(url),
            "DELETE" => client.delete(url),
            _ => client.get(url),
        };

        let req = if let Some(b) = body {
            req.header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(b)
        } else {
            req
        };

        Ok(req.send().await?)
    }

    fn build_url(
        base_url: &str,
        endpoint: &ApiEndpoint,
        param: &ApiParameter,
        value: &str,
    ) -> String {
        let mut url = format!("{}{}", base_url, endpoint.path);

        if param.location == ParamLocation::Path {
            url = url.replace(&format!("{{{}}}", param.name), value);
        } else if param.location == ParamLocation::Query {
            let separator = if url.contains('?') { '&' } else { '?' };
            url = format!("{}{}{}={}", url, separator, param.name, value);
        }

        url
    }

    fn parse_parameter(param: &serde_json::Value) -> Option<ApiParameter> {
        let name = param.get("name").and_then(|v| v.as_str())?.to_string();
        let location_str = param.get("in").and_then(|v| v.as_str()).unwrap_or("query");
        let location = match location_str {
            "path" => ParamLocation::Path,
            "query" => ParamLocation::Query,
            "header" => ParamLocation::Header,
            "cookie" => ParamLocation::Cookie,
            "body" => ParamLocation::Body,
            _ => ParamLocation::Query,
        };
        let required = param
            .get("required")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let schema = param.get("schema").unwrap_or(param);

        let param_type = schema
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("string")
            .to_string();
        let format = schema
            .get("format")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let example = schema
            .get("example")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let min_value = schema.get("minimum").map(|v| v.to_string());
        let max_value = schema.get("maximum").map(|v| v.to_string());
        let pattern = schema
            .get("pattern")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let enum_values = schema.get("enum").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        });

        Some(ApiParameter {
            name,
            location,
            required,
            param_type,
            format,
            example,
            min_value,
            max_value,
            pattern,
            enum_values,
        })
    }

    fn parse_request_body(body: &serde_json::Value) -> Option<RequestBody> {
        let content = body.get("content").and_then(|v| v.as_object())?;
        for (content_type, media) in content {
            if let Some(schema) = media.get("schema") {
                return Some(RequestBody {
                    content_type: content_type.clone(),
                    schema: schema.clone(),
                    required: body
                        .get("required")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                });
            }
        }
        None
    }

    fn yaml_to_json(content: &str) -> Result<serde_json::Value> {
        let yaml_value: serde_yaml_neo::Value = serde_yaml_neo::from_str(content)?;
        Ok(serde_json::to_value(yaml_value)?)
    }

    fn string_payload(param: &ApiParameter) -> Vec<Payload> {
        let mut payloads = Vec::new();

        payloads.push(Payload {
            payload_type: PayloadType::Sqli,
            payload: "' OR '1'='1".to_string(),
            description: "SQL injection in string parameter".to_string(),
            severity: Severity::High,
            tags: vec![param.name.clone()],
        });

        payloads.push(Payload {
            payload_type: PayloadType::Xss,
            payload: "<script>alert(1)</script>".to_string(),
            description: "XSS in string parameter".to_string(),
            severity: Severity::High,
            tags: vec![param.name.clone()],
        });

        payloads.push(Payload {
            payload_type: PayloadType::Ssti,
            payload: "{{7*7}}".to_string(),
            description: "SSTI in string parameter".to_string(),
            severity: Severity::High,
            tags: vec![param.name.clone()],
        });

        if let Some(ref pattern) = param.pattern {
            payloads.push(Payload {
                payload_type: PayloadType::Sqli,
                payload: format!("{}' OR 1=1--", pattern),
                description: format!("Pattern bypass attempt for {}", param.name),
                severity: Severity::Medium,
                tags: vec![param.name.clone()],
            });
        }

        payloads
    }

    fn numeric_payload(param: &ApiParameter) -> Vec<Payload> {
        vec![
            Payload {
                payload_type: PayloadType::Sqli,
                payload: "0 OR 1=1".to_string(),
                description: "SQL injection in numeric parameter".to_string(),
                severity: Severity::High,
                tags: vec![param.name.clone()],
            },
            Payload {
                payload_type: PayloadType::Sqli,
                payload: "99999999999999999999".to_string(),
                description: "Integer overflow attempt".to_string(),
                severity: Severity::Medium,
                tags: vec![param.name.clone()],
            },
            Payload {
                payload_type: PayloadType::Sqli,
                payload: "-1".to_string(),
                description: "Negative value injection".to_string(),
                severity: Severity::Medium,
                tags: vec![param.name.clone()],
            },
            Payload {
                payload_type: PayloadType::Sqli,
                payload: "1e308".to_string(),
                description: "Float overflow attempt".to_string(),
                severity: Severity::Medium,
                tags: vec![param.name.clone()],
            },
        ]
    }

    fn boolean_payload(param: &ApiParameter) -> Vec<Payload> {
        vec![
            Payload {
                payload_type: PayloadType::Headers,
                payload: "true".to_string(),
                description: "Boolean true injection".to_string(),
                severity: Severity::Low,
                tags: vec![param.name.clone()],
            },
            Payload {
                payload_type: PayloadType::Headers,
                payload: "false".to_string(),
                description: "Boolean false injection".to_string(),
                severity: Severity::Low,
                tags: vec![param.name.clone()],
            },
            Payload {
                payload_type: PayloadType::Headers,
                payload: "1".to_string(),
                description: "Numeric boolean injection".to_string(),
                severity: Severity::Low,
                tags: vec![param.name.clone()],
            },
        ]
    }

    fn array_payload(param: &ApiParameter) -> Vec<Payload> {
        vec![
            Payload {
                payload_type: PayloadType::Sqli,
                payload: "[]".to_string(),
                description: "Empty array injection".to_string(),
                severity: Severity::Medium,
                tags: vec![param.name.clone()],
            },
            Payload {
                payload_type: PayloadType::Sqli,
                payload: "[1,2,3]".to_string(),
                description: "Numeric array injection".to_string(),
                severity: Severity::Medium,
                tags: vec![param.name.clone()],
            },
            Payload {
                payload_type: PayloadType::Xss,
                payload: "['<script>alert(1)</script>']".to_string(),
                description: "XSS in array injection".to_string(),
                severity: Severity::High,
                tags: vec![param.name.clone()],
            },
        ]
    }

    fn object_payload(param: &ApiParameter) -> Vec<Payload> {
        vec![
            Payload {
                payload_type: PayloadType::Deser,
                payload: "{}".to_string(),
                description: "Empty object injection".to_string(),
                severity: Severity::Medium,
                tags: vec![param.name.clone()],
            },
            Payload {
                payload_type: PayloadType::Deser,
                payload: "{\"__proto__\": {\"isAdmin\": true}}".to_string(),
                description: "Prototype pollution attempt".to_string(),
                severity: Severity::Critical,
                tags: vec![param.name.clone()],
            },
            Payload {
                payload_type: PayloadType::Deser,
                payload: "{\"constructor\": {\"prototype\": {\"polluted\": true}}}".to_string(),
                description: "Constructor pollution attempt".to_string(),
                severity: Severity::Critical,
                tags: vec![param.name.clone()],
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_openapi3_simple() {
        let content = r#"{
            "openapi": "3.0.0",
            "paths": {
                "/users": {
                    "get": {
                        "parameters": [
                            {
                                "name": "id",
                                "in": "query",
                                "required": true,
                                "schema": {"type": "integer"}
                            }
                        ]
                    }
                }
            }
        }"#;
        let endpoints = ApiSchemaFuzzer::parse_openapi(content).unwrap();
        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].path, "/users");
        assert_eq!(endpoints[0].method, "GET");
        assert_eq!(endpoints[0].parameters.len(), 1);
    }

    #[test]
    fn test_parse_openapi_multiple_paths() {
        let content = r#"{
            "openapi": "3.0.0",
            "paths": {
                "/users": {"get": {}},
                "/posts": {"post": {}},
                "/items/{id}": {"delete": {}}
            }
        }"#;
        let endpoints = ApiSchemaFuzzer::parse_openapi(content).unwrap();
        assert_eq!(endpoints.len(), 3);
    }

    #[test]
    fn test_generate_type_aware_string_payloads() {
        let fuzzer = ApiSchemaFuzzer::new(10).unwrap();
        let param = ApiParameter {
            name: "username".to_string(),
            location: ParamLocation::Query,
            required: true,
            param_type: "string".to_string(),
            format: None,
            example: None,
            min_value: None,
            max_value: None,
            pattern: None,
            enum_values: None,
        };
        let payloads = fuzzer.generate_type_aware_payloads(&param);
        assert!(!payloads.is_empty());
        assert!(payloads.iter().any(|p| p.payload.contains("OR")));
    }

    #[test]
    fn test_generate_type_aware_numeric_payloads() {
        let fuzzer = ApiSchemaFuzzer::new(10).unwrap();
        let param = ApiParameter {
            name: "count".to_string(),
            location: ParamLocation::Query,
            required: true,
            param_type: "integer".to_string(),
            format: None,
            example: None,
            min_value: None,
            max_value: None,
            pattern: None,
            enum_values: None,
        };
        let payloads = fuzzer.generate_type_aware_payloads(&param);
        assert!(!payloads.is_empty());
        assert!(payloads.iter().any(|p| p.payload.contains("1e308")));
    }

    #[test]
    fn test_generate_oversized_payloads() {
        let fuzzer = ApiSchemaFuzzer::new(10).unwrap();
        let endpoints = vec![ApiEndpoint {
            path: "/api/data".to_string(),
            method: "POST".to_string(),
            parameters: vec![ApiParameter {
                name: "input".to_string(),
                location: ParamLocation::Query,
                required: true,
                param_type: "string".to_string(),
                format: None,
                example: None,
                min_value: None,
                max_value: None,
                pattern: None,
                enum_values: None,
            }],
            request_body: None,
            security: Vec::new(),
        }];
        let oversized = fuzzer.generate_oversized_payloads(&endpoints);
        assert!(!oversized.is_empty());
        assert_eq!(oversized.len(), 4);
    }

    #[test]
    fn test_build_url_query_param() {
        let endpoint = ApiEndpoint {
            path: "/search".to_string(),
            method: "GET".to_string(),
            parameters: vec![],
            request_body: None,
            security: Vec::new(),
        };
        let param = ApiParameter {
            name: "q".to_string(),
            location: ParamLocation::Query,
            required: true,
            param_type: "string".to_string(),
            format: None,
            example: None,
            min_value: None,
            max_value: None,
            pattern: None,
            enum_values: None,
        };
        let url = ApiSchemaFuzzer::build_url("https://example.com", &endpoint, &param, "test");
        assert!(url.contains("?q=test"));
    }

    #[test]
    fn test_build_url_path_param() {
        let endpoint = ApiEndpoint {
            path: "/users/{id}".to_string(),
            method: "GET".to_string(),
            parameters: vec![],
            request_body: None,
            security: Vec::new(),
        };
        let param = ApiParameter {
            name: "id".to_string(),
            location: ParamLocation::Path,
            required: true,
            param_type: "integer".to_string(),
            format: None,
            example: None,
            min_value: None,
            max_value: None,
            pattern: None,
            enum_values: None,
        };
        let url = ApiSchemaFuzzer::build_url("https://example.com", &endpoint, &param, "123");
        assert!(url.contains("/users/123"));
    }

    #[test]
    fn test_fuzzer_creation() {
        let fuzzer = ApiSchemaFuzzer::new(30);
        assert!(fuzzer.is_ok());
    }
}
