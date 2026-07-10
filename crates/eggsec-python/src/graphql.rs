use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use crate::finding::Severity;
use crate::runtime_sync;

/// GraphQL vulnerability type.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GraphQLVulnerabilityPy {
    Introspection,
    QueryInjection,
    DepthLimitBypass,
    AliasBypass,
    BatchBypass,
    DirectiveInjection,
    FieldSuggestion,
    AliasOverload,
}

#[pymethods]
impl GraphQLVulnerabilityPy {
    fn __repr__(&self) -> String {
        format!("GraphQLVulnerability.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl GraphQLVulnerabilityPy {
    fn as_str(&self) -> &str {
        match self {
            GraphQLVulnerabilityPy::Introspection => "Introspection",
            GraphQLVulnerabilityPy::QueryInjection => "QueryInjection",
            GraphQLVulnerabilityPy::DepthLimitBypass => "DepthLimitBypass",
            GraphQLVulnerabilityPy::AliasBypass => "AliasBypass",
            GraphQLVulnerabilityPy::BatchBypass => "BatchBypass",
            GraphQLVulnerabilityPy::DirectiveInjection => "DirectiveInjection",
            GraphQLVulnerabilityPy::FieldSuggestion => "FieldSuggestion",
            GraphQLVulnerabilityPy::AliasOverload => "AliasOverload",
        }
    }

    fn from_engine(engine: eggsec::fuzzer::payloads::graphql::GraphQLVulnerability) -> Self {
        match engine {
            eggsec::fuzzer::payloads::graphql::GraphQLVulnerability::Introspection => {
                GraphQLVulnerabilityPy::Introspection
            }
            eggsec::fuzzer::payloads::graphql::GraphQLVulnerability::QueryInjection => {
                GraphQLVulnerabilityPy::QueryInjection
            }
            eggsec::fuzzer::payloads::graphql::GraphQLVulnerability::DepthLimitBypass => {
                GraphQLVulnerabilityPy::DepthLimitBypass
            }
            eggsec::fuzzer::payloads::graphql::GraphQLVulnerability::AliasBypass => {
                GraphQLVulnerabilityPy::AliasBypass
            }
            eggsec::fuzzer::payloads::graphql::GraphQLVulnerability::BatchBypass => {
                GraphQLVulnerabilityPy::BatchBypass
            }
            eggsec::fuzzer::payloads::graphql::GraphQLVulnerability::DirectiveInjection => {
                GraphQLVulnerabilityPy::DirectiveInjection
            }
            eggsec::fuzzer::payloads::graphql::GraphQLVulnerability::FieldSuggestion => {
                GraphQLVulnerabilityPy::FieldSuggestion
            }
            eggsec::fuzzer::payloads::graphql::GraphQLVulnerability::AliasOverload => {
                GraphQLVulnerabilityPy::AliasOverload
            }
        }
    }
}

/// A single GraphQL security test result.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLTestResultPy {
    #[pyo3(get)]
    pub vulnerability: GraphQLVulnerabilityPy,
    #[pyo3(get)]
    pub success: bool,
    #[pyo3(get)]
    pub query: String,
    #[pyo3(get)]
    pub response_snippet: String,
    #[pyo3(get)]
    pub severity: Severity,
    #[pyo3(get)]
    pub description: String,
}

impl GraphQLTestResultPy {
    fn from_engine(engine: eggsec::fuzzer::payloads::graphql::GraphQLTestResult) -> Self {
        Self {
            vulnerability: GraphQLVulnerabilityPy::from_engine(engine.vulnerability),
            success: engine.success,
            query: engine.query,
            response_snippet: engine.response_snippet,
            severity: Severity::from_engine(engine.severity),
            description: engine.description,
        }
    }
}

#[pymethods]
impl GraphQLTestResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("vulnerability", self.vulnerability.as_str())?;
        dict.set_item("success", self.success)?;
        dict.set_item("query", &self.query)?;
        dict.set_item("response_snippet", &self.response_snippet)?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("description", &self.description)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "GraphQLTestResult(vuln={}, success={})",
            self.vulnerability.as_str(),
            self.success
        )
    }

    fn __str__(&self) -> String {
        format!(
            "[{}] {} - success={}",
            self.severity.as_str(),
            self.vulnerability.as_str(),
            self.success
        )
    }
}

/// GraphQL schema type information.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLTypePy {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub kind: String,
    fields: Vec<GraphQLFieldPy>,
    input_fields: Option<Vec<GraphQLInputFieldPy>>,
    interfaces: Vec<String>,
    enum_values: Option<Vec<String>>,
}

impl GraphQLTypePy {
    fn from_engine(engine: eggsec::fuzzer::payloads::graphql::GraphQLType) -> Self {
        Self {
            name: engine.name,
            kind: engine.kind,
            fields: engine
                .fields
                .into_iter()
                .map(GraphQLFieldPy::from_engine)
                .collect(),
            input_fields: engine.input_fields.map(|v| {
                v.into_iter()
                    .map(GraphQLInputFieldPy::from_engine)
                    .collect()
            }),
            interfaces: engine.interfaces,
            enum_values: engine.enum_values,
        }
    }
}

#[pymethods]
impl GraphQLTypePy {
    #[getter]
    fn fields(&self) -> Vec<GraphQLFieldPy> {
        self.fields.clone()
    }

    #[getter]
    fn input_fields(&self) -> Option<Vec<GraphQLInputFieldPy>> {
        self.input_fields.clone()
    }

    #[getter]
    fn interfaces(&self) -> Vec<String> {
        self.interfaces.clone()
    }

    #[getter]
    fn enum_values(&self) -> Option<Vec<String>> {
        self.enum_values.clone()
    }

    fn __repr__(&self) -> String {
        format!("GraphQLType(name={}, kind={})", self.name, self.kind)
    }
}

/// GraphQL field information.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLFieldPy {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub r#type: String,
    args: Vec<GraphQLArgPy>,
}

impl GraphQLFieldPy {
    fn from_engine(engine: eggsec::fuzzer::payloads::graphql::GraphQLField) -> Self {
        Self {
            name: engine.name,
            r#type: engine.r#type,
            args: engine
                .args
                .into_iter()
                .map(GraphQLArgPy::from_engine)
                .collect(),
        }
    }
}

#[pymethods]
impl GraphQLFieldPy {
    #[getter]
    fn args(&self) -> Vec<GraphQLArgPy> {
        self.args.clone()
    }

    fn __repr__(&self) -> String {
        format!("GraphQLField(name={}, type={})", self.name, self.r#type)
    }
}

/// GraphQL argument information.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLArgPy {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub r#type: String,
    #[pyo3(get)]
    pub default_value: Option<String>,
}

impl GraphQLArgPy {
    fn from_engine(engine: eggsec::fuzzer::payloads::graphql::GraphQLArg) -> Self {
        Self {
            name: engine.name,
            r#type: engine.r#type,
            default_value: engine.default_value,
        }
    }
}

#[pymethods]
impl GraphQLArgPy {
    fn __repr__(&self) -> String {
        format!("GraphQLArg(name={}, type={})", self.name, self.r#type)
    }
}

/// GraphQL input field information.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLInputFieldPy {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub r#type: String,
}

impl GraphQLInputFieldPy {
    fn from_engine(engine: eggsec::fuzzer::payloads::graphql::GraphQLInputField) -> Self {
        Self {
            name: engine.name,
            r#type: engine.r#type,
        }
    }
}

#[pymethods]
impl GraphQLInputFieldPy {
    fn __repr__(&self) -> String {
        format!(
            "GraphQLInputField(name={}, type={})",
            self.name, self.r#type
        )
    }
}

/// GraphQL schema introspection result.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLSchemaPy {
    #[pyo3(get)]
    pub query_type: Option<String>,
    #[pyo3(get)]
    pub mutation_type: Option<String>,
    #[pyo3(get)]
    pub subscription_type: Option<String>,
    types: Vec<GraphQLTypePy>,
    directives: Vec<String>,
}

impl GraphQLSchemaPy {
    fn from_engine(engine: eggsec::fuzzer::payloads::graphql::GraphQLSchema) -> Self {
        Self {
            query_type: engine.query_type,
            mutation_type: engine.mutation_type,
            subscription_type: engine.subscription_type,
            types: engine
                .types
                .into_iter()
                .map(GraphQLTypePy::from_engine)
                .collect(),
            directives: engine.directives,
        }
    }
}

#[pymethods]
impl GraphQLSchemaPy {
    #[getter]
    fn types(&self) -> Vec<GraphQLTypePy> {
        self.types.clone()
    }

    #[getter]
    fn directives(&self) -> Vec<String> {
        self.directives.clone()
    }

    #[getter]
    fn type_count(&self) -> usize {
        self.types.len()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("query_type", &self.query_type)?;
        dict.set_item("mutation_type", &self.mutation_type)?;
        dict.set_item("subscription_type", &self.subscription_type)?;
        dict.set_item("type_count", self.types.len())?;
        dict.set_item("directives", &self.directives)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "GraphQLSchema(query_type={:?}, types={})",
            self.query_type,
            self.types.len()
        )
    }
}

/// Configuration for GraphQL security testing.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct GraphQLTestConfigPy {
    #[pyo3(get)]
    pub endpoint: String,
    #[pyo3(get)]
    pub enable_introspection: bool,
    #[pyo3(get)]
    pub enable_depth_bypass: bool,
    #[pyo3(get)]
    pub enable_alias_overload: bool,
    #[pyo3(get)]
    pub timeout_secs: u64,
}

#[pymethods]
impl GraphQLTestConfigPy {
    /// Create a new GraphQL test configuration.
    ///
    /// Args:
    ///     endpoint: GraphQL endpoint URL (e.g. "https://example.com/graphql").
    ///     enable_introspection: Test for introspection exposure (default: true).
    ///     enable_depth_bypass: Test for depth limit bypass (default: true).
    ///     enable_alias_overload: Test for alias overload DoS (default: true).
    ///     timeout_secs: Request timeout in seconds (default: 10).
    #[new]
    #[pyo3(signature = (endpoint, *, enable_introspection=true, enable_depth_bypass=true, enable_alias_overload=true, timeout_secs=10))]
    fn new(
        endpoint: String,
        enable_introspection: bool,
        enable_depth_bypass: bool,
        enable_alias_overload: bool,
        timeout_secs: u64,
    ) -> PyResult<Self> {
        if endpoint.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "endpoint must not be empty",
            ));
        }
        Ok(Self {
            endpoint,
            enable_introspection,
            enable_depth_bypass,
            enable_alias_overload,
            timeout_secs,
        })
    }

    fn __repr__(&self) -> String {
        format!("GraphQLTestConfig(endpoint={})", self.endpoint)
    }
}

/// Run GraphQL security tests against a target endpoint.
///
/// Tests introspection exposure, query injection, depth limit bypass,
/// alias overload, batch query bypass, and directive injection.
///
/// Args:
///     config: GraphQL test configuration.
///
/// Returns:
///     list[GraphQLTestResultPy]: List of test results.
///
/// Raises:
///     NetworkError: If the endpoint is unreachable.
///     ConfigError: If the configuration is invalid.
#[pyfunction]
pub fn graphql_test(config: GraphQLTestConfigPy) -> PyResult<Vec<GraphQLTestResultPy>> {
    Python::with_gil(|py| {
        let result = runtime_sync::block_on(py, async move {
            let mut fuzzer =
                eggsec::fuzzer::payloads::graphql::GraphQLFuzzer::new(config.endpoint.clone())
                    .with_introspection(config.enable_introspection)
                    .with_depth_bypass(config.enable_depth_bypass)
                    .with_alias_overload(config.enable_alias_overload);

            let mut results = Vec::new();

            // Static introspection tests
            results.extend(fuzzer.test_introspection_enabled());

            // Injection tests
            results.extend(fuzzer.generate_injection_queries(
                config.enable_depth_bypass,
                config.enable_alias_overload,
            ));

            // Batch tests
            results.extend(fuzzer.generate_batch_queries(config.enable_alias_overload));

            Ok::<_, PyErr>(results)
        })?;

        Ok(result
            .into_iter()
            .map(GraphQLTestResultPy::from_engine)
            .collect())
    })
}

/// Run GraphQL security tests (async).
///
/// Returns a PyFuture that resolves to a list of test results.
#[pyfunction]
pub fn async_graphql_test(config: GraphQLTestConfigPy) -> PyResult<crate::runtime_async::PyFuture> {
    crate::runtime_async::spawn_async(async move {
        let mut fuzzer =
            eggsec::fuzzer::payloads::graphql::GraphQLFuzzer::new(config.endpoint.clone())
                .with_introspection(config.enable_introspection)
                .with_depth_bypass(config.enable_depth_bypass)
                .with_alias_overload(config.enable_alias_overload);

        let mut results = Vec::new();
        results.extend(fuzzer.test_introspection_enabled());
        results.extend(
            fuzzer.generate_injection_queries(
                config.enable_depth_bypass,
                config.enable_alias_overload,
            ),
        );
        results.extend(fuzzer.generate_batch_queries(config.enable_alias_overload));

        Ok::<Vec<GraphQLTestResultPy>, PyErr>(
            results
                .into_iter()
                .map(GraphQLTestResultPy::from_engine)
                .collect(),
        )
    })
}
