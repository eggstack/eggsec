use crate::error::Result;
use crate::utils::create_insecure_http_client;
use serde::{Deserialize, Serialize};

const SCHEMA_PATHS: &[&str] = &[
    "/openapi.json",
    "/openapi.yaml",
    "/openapi.yml",
    "/swagger.json",
    "/swagger.yaml",
    "/swagger.yml",
    "/api-docs",
    "/api-docs.json",
    "/api-docs.yaml",
    "/v2/api-docs",
    "/v3/api-docs",
    "/api/swagger.json",
    "/api/swagger.yaml",
    "/docs/openapi.json",
    "/spec/openapi.json",
    "/swagger-ui/swagger.json",
    "/api/v1/openapi.json",
    "/api/v2/openapi.json",
    "/graphql",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDiscoveryResult {
    pub target: String,
    pub found: bool,
    pub url: Option<String>,
    pub schema_type: Option<SchemaType>,
    pub raw_content: Option<String>,
    pub endpoints_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SchemaType {
    OpenApi3,
    Swagger2,
    GraphQL,
    Unknown,
}

pub struct SchemaDiscovery {
    client: reqwest::Client,
}

impl SchemaDiscovery {
    pub fn new(timeout_secs: u64) -> Result<Self> {
        let client = create_insecure_http_client(timeout_secs)?;
        Ok(Self { client })
    }

    pub async fn discover(&self, base_url: &str) -> Result<Vec<SchemaDiscoveryResult>> {
        let base = base_url.trim_end_matches('/');
        let mut results = Vec::new();

        for path in SCHEMA_PATHS {
            let url = format!("{}{}", base, path);
            let result = self.check_endpoint(&url).await;
            if let Ok(r) = result {
                if r.found {
                    results.push(r);
                }
            }
        }

        Ok(results)
    }

    pub async fn discover_single(&self, base_url: &str, path: &str) -> Result<SchemaDiscoveryResult> {
        let base = base_url.trim_end_matches('/');
        let url = format!("{}{}", base, path.trim_start_matches('/'));
        self.check_endpoint(&url).await
    }

    async fn check_endpoint(&self, url: &str) -> Result<SchemaDiscoveryResult> {
        let response = match self.client.get(url).send().await {
            Ok(r) => r,
            Err(_) => {
                return Ok(SchemaDiscoveryResult {
                    target: url.to_string(),
                    found: false,
                    url: None,
                    schema_type: None,
                    raw_content: None,
                    endpoints_count: None,
                });
            }
        };

        if !response.status().is_success() {
            return Ok(SchemaDiscoveryResult {
                target: url.to_string(),
                found: false,
                url: None,
                schema_type: None,
                raw_content: None,
                endpoints_count: None,
            });
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let body = response.text().await.unwrap_or_default();
        let schema_type = Self::detect_schema_type(&body, &content_type);
        let endpoints_count = Self::count_endpoints(&body, &schema_type);

        Ok(SchemaDiscoveryResult {
            target: url.to_string(),
            found: true,
            url: Some(url.to_string()),
            schema_type: Some(schema_type),
            raw_content: Some(body),
            endpoints_count,
        })
    }

    fn detect_schema_type(body: &str, content_type: &str) -> SchemaType {
        if content_type.contains("graphql") || body.contains("\"__schema\"") || body.contains("IntrospectionQuery") {
            return SchemaType::GraphQL;
        }

        if body.contains("\"openapi\": \"3") || body.contains("'openapi': '3") {
            return SchemaType::OpenApi3;
        }

        if body.contains("\"swagger\": \"2") || body.contains("'swagger': '2") {
            return SchemaType::Swagger2;
        }

        if body.contains("openapi:") && body.contains("3.") {
            return SchemaType::OpenApi3;
        }

        if body.contains("swagger:") && body.contains("2.") {
            return SchemaType::Swagger2;
        }

        if content_type.contains("json") {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                if json.get("openapi").and_then(|v| v.as_str()).map(|s| s.starts_with('3')).unwrap_or(false) {
                    return SchemaType::OpenApi3;
                }
                if json.get("swagger").and_then(|v| v.as_str()).map(|s| s.starts_with('2')).unwrap_or(false) {
                    return SchemaType::Swagger2;
                }
            }
        }

        SchemaType::Unknown
    }

    fn count_endpoints(body: &str, schema_type: &SchemaType) -> Option<usize> {
        match schema_type {
            SchemaType::OpenApi3 | SchemaType::Swagger2 => {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                    if let Some(paths) = json.get("paths").and_then(|v| v.as_object()) {
                        return Some(paths.len());
                    }
                }
                let paths_count = body.matches("\"/").count();
                if paths_count > 0 {
                    return Some(paths_count);
                }
                let yaml_paths = body.matches("\n  /").count();
                if yaml_paths > 0 {
                    return Some(yaml_paths);
                }
                None
            }
            SchemaType::GraphQL => Some(1),
            SchemaType::Unknown => None,
        }
    }
}

pub async fn discover_schema(base_url: &str, timeout_secs: u64) -> Result<Vec<SchemaDiscoveryResult>> {
    let discovery = SchemaDiscovery::new(timeout_secs)?;
    discovery.discover(base_url).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_paths_not_empty() {
        assert!(!SCHEMA_PATHS.is_empty());
        assert!(SCHEMA_PATHS.contains(&"/openapi.json"));
        assert!(SCHEMA_PATHS.contains(&"/swagger.json"));
    }

    #[test]
    fn test_detect_openapi3_json() {
        let body = r#"{"openapi": "3.0.0", "info": {"title": "Test", "version": "1.0"}}"#;
        let schema_type = SchemaDiscovery::detect_schema_type(body, "application/json");
        assert_eq!(schema_type, SchemaType::OpenApi3);
    }

    #[test]
    fn test_detect_swagger2_json() {
        let body = r#"{"swagger": "2.0", "info": {"title": "Test", "version": "1.0"}}"#;
        let schema_type = SchemaDiscovery::detect_schema_type(body, "application/json");
        assert_eq!(schema_type, SchemaType::Swagger2);
    }

    #[test]
    fn test_detect_graphql() {
        let body = r#"{"__schema": {"types": []}}"#;
        let schema_type = SchemaDiscovery::detect_schema_type(body, "application/json");
        assert_eq!(schema_type, SchemaType::GraphQL);
    }

    #[test]
    fn test_detect_openapi3_yaml() {
        let body = "openapi: 3.0.0\ninfo:\n  title: Test";
        let schema_type = SchemaDiscovery::detect_schema_type(body, "text/yaml");
        assert_eq!(schema_type, SchemaType::OpenApi3);
    }

    #[test]
    fn test_detect_unknown() {
        let body = "<html><body>Hello</body></html>";
        let schema_type = SchemaDiscovery::detect_schema_type(body, "text/html");
        assert_eq!(schema_type, SchemaType::Unknown);
    }

    #[test]
    fn test_count_endpoints_openapi() {
        let body = r#"{"openapi": "3.0.0", "paths": {"/users": {}, "/posts": {}}}"#;
        let count = SchemaDiscovery::count_endpoints(body, &SchemaType::OpenApi3);
        assert_eq!(count, Some(2));
    }

    #[test]
    fn test_count_endpoints_graphql() {
        let count = SchemaDiscovery::count_endpoints("", &SchemaType::GraphQL);
        assert_eq!(count, Some(1));
    }

    #[test]
    fn test_discovery_creation() {
        let discovery = SchemaDiscovery::new(10);
        assert!(discovery.is_ok());
    }

    #[test]
    fn test_schema_discovery_result_default() {
        let result = SchemaDiscoveryResult {
            target: "http://example.com".to_string(),
            found: false,
            url: None,
            schema_type: None,
            raw_content: None,
            endpoints_count: None,
        };
        assert!(!result.found);
    }
}
