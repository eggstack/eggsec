use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde_json::json;

use crate::operation_registry::StableOperation;

/// Map a stable tool ID to its corresponding engine metadata ID.
///
/// The engine metadata uses dash-separated IDs (e.g. "scan-ports") while
/// the Python stable-core surface uses underscored IDs (e.g. "scan_ports").
fn metadata_id_for_tool(tool_id: &str) -> Option<&'static str> {
    match tool_id {
        "scan_ports" => Some("scan-ports"),
        "scan_endpoints" => Some("scan-endpoints"),
        "fingerprint_services" => Some("fingerprint"),
        "recon_dns" => Some("recon"),
        "inspect_tls" => None,
        "detect_technology" => None,
        "detect_waf" => Some("waf-detect"),
        "validate_waf" => Some("waf-detect"),
        "fuzz_http" => Some("fuzz"),
        "load_test" => Some("load-test"),
        "scan_git_secrets" => None,
        "generate_sbom" => None,
        "run_consolidated_recon" => Some("recon"),
        "graphql_test" => Some("graphql"),
        "oauth_test" => Some("oauth"),
        "auth_test" => Some("auth-test"),
        "db_probe" => Some("db-pentest"),
        "nse_run" => Some("nse"),
        "scan_docker_image" => None,
        "scan_kubernetes" => None,
        "analyze_apk" => Some("mobile-static"),
        "analyze_ipa" => Some("mobile-static"),
        _ => None,
    }
}

/// Classify a tool ID into a human-readable category.
fn category_for_tool(tool_id: &str) -> &'static str {
    match tool_id {
        "recon_dns" | "run_consolidated_recon" | "inspect_tls" => "recon",
        "scan_ports" | "scan_endpoints" => "scanning",
        "fingerprint_services" => "fingerprinting",
        "detect_waf" | "validate_waf" => "waf",
        "fuzz_http" => "fuzzing",
        "load_test" => "load_testing",
        "detect_technology" | "graphql_test" | "oauth_test" | "auth_test" => "assessment",
        "analyze_apk" | "analyze_ipa" => "mobile",
        "scan_docker_image" | "scan_kubernetes" => "container",
        "db_probe" => "database",
        "nse_run" => "nse",
        "scan_git_secrets" | "generate_sbom" => "assessment",
        _ => "other",
    }
}

/// Map a stable operation risk to a short risk string.
fn risk_string_for_stable(op: StableOperation) -> &'static str {
    match op {
        StableOperation::ScanPorts
        | StableOperation::ScanEndpoints
        | StableOperation::FingerprintServices
        | StableOperation::ReconDns
        | StableOperation::InspectTls
        | StableOperation::DetectTechnology
        | StableOperation::DetectWaf
        | StableOperation::ValidateWaf
        | StableOperation::RunConsolidatedRecon
        | StableOperation::GraphqlTest
        | StableOperation::OauthTest
        | StableOperation::AuthTest
        | StableOperation::ScanGitSecrets
        | StableOperation::GenerateSbom
        | StableOperation::ScanDockerImage
        | StableOperation::ScanKubernetes
        | StableOperation::AnalyzeApk
        | StableOperation::AnalyzeIpa => "safe_active",
        StableOperation::FuzzHttp | StableOperation::NseRun => "intrusive",
        StableOperation::LoadTest => "load_test",
        StableOperation::DbProbe => "db_pentest",
    }
}

/// Determine confirmation behavior for a stable operation.
fn confirmation_required_for(op: StableOperation) -> bool {
    matches!(
        op,
        StableOperation::NseRun
            | StableOperation::DbProbe
            | StableOperation::FuzzHttp
            | StableOperation::LoadTest
    )
}

fn confirmation_message_for(op: StableOperation) -> Option<&'static str> {
    if confirmation_required_for(op) {
        Some("This operation may interact with external systems. Confirm to proceed.")
    } else {
        None
    }
}

/// Compute supported surfaces from engine metadata exposable flags.
fn compute_surfaces(
    manual: bool,
    tui: bool,
    mcp: bool,
    rest: bool,
    agent: bool,
    grpc: bool,
) -> Vec<String> {
    let mut surfaces = Vec::new();
    if manual {
        surfaces.push("cli".to_string());
    }
    if tui {
        surfaces.push("tui".to_string());
    }
    if mcp {
        surfaces.push("mcp".to_string());
    }
    if rest {
        surfaces.push("rest".to_string());
    }
    if agent {
        surfaces.push("agent".to_string());
    }
    if grpc {
        surfaces.push("grpc".to_string());
    }
    surfaces
}

/// Compute supported surfaces for a stable operation from metadata.
fn surfaces_for_stable(tool_id: &str) -> Vec<String> {
    if let Some(meta_id) = metadata_id_for_tool(tool_id) {
        if let Some(meta) = eggsec::config::operation_metadata(meta_id) {
            return compute_surfaces(
                meta.manual_exposable,
                meta.tui_exposable,
                meta.mcp_exposable,
                meta.rest_exposable,
                meta.agent_exposable,
                meta.grpc_exposable,
            );
        }
    }
    // Fallback: stable operations with no metadata are available on core surfaces
    vec!["cli".to_string(), "tui".to_string(), "mcp".to_string()]
}

/// Compute intended uses for a stable operation from metadata.
fn intended_uses_for_stable(tool_id: &str) -> Vec<String> {
    if let Some(meta_id) = metadata_id_for_tool(tool_id) {
        if let Some(meta) = eggsec::config::operation_metadata(meta_id) {
            return meta
                .intended_uses
                .iter()
                .map(|u| format!("{}", u))
                .collect();
        }
    }
    vec!["web-assessment".to_string()]
}

/// Determine whether a stable operation is locally available (CLI/TUI/MCP).
fn local_available_for_stable(tool_id: &str) -> bool {
    if let Some(meta_id) = metadata_id_for_tool(tool_id) {
        if let Some(meta) = eggsec::config::operation_metadata(meta_id) {
            return meta.manual_exposable || meta.tui_exposable || meta.mcp_exposable;
        }
    }
    true
}

/// Determine whether a stable operation is daemon/agent available.
fn daemon_available_for_stable(tool_id: &str) -> bool {
    if let Some(meta_id) = metadata_id_for_tool(tool_id) {
        if let Some(meta) = eggsec::config::operation_metadata(meta_id) {
            return meta.agent_exposable;
        }
    }
    false
}

/// Map a risk string to the target policy for display.
fn target_policy_for_stable(tool_id: &str) -> &'static str {
    if let Some(meta_id) = metadata_id_for_tool(tool_id) {
        if let Some(meta) = eggsec::config::operation_metadata(meta_id) {
            return match meta.target_policy {
                eggsec::config::TargetPolicyKind::NoTarget => "no-target",
                eggsec::config::TargetPolicyKind::OptionalTarget => "optional-target",
                eggsec::config::TargetPolicyKind::TargetRequired => "target-required",
                eggsec::config::TargetPolicyKind::ExplicitScopeRequired => {
                    "explicit-scope-required"
                }
                eggsec::config::TargetPolicyKind::PrivateOrLocalRequired => {
                    "private-or-local-required"
                }
            };
        }
    }
    "explicit-scope-required"
}

/// Framework-neutral tool descriptor.
///
/// Bridges the engine `OperationMetadata` system to a tool-descriptor model
/// suitable for registry, schema generation, and invocation metadata.
#[pyclass(frozen, name = "ToolDescriptor")]
#[derive(Clone, Debug)]
pub struct ToolDescriptorPy {
    /// Unique tool identifier using underscores (e.g. "scan_ports").
    #[pyo3(get)]
    pub tool_id: String,
    /// Canonical operation identifier (e.g. "scan-ports" from engine metadata).
    #[pyo3(get)]
    pub operation_id: String,
    /// Human-readable title.
    #[pyo3(get)]
    pub title: String,
    /// Detailed description of what the tool does.
    #[pyo3(get)]
    pub description: String,
    /// Tool version string.
    #[pyo3(get)]
    pub version: String,
    /// Category grouping (e.g. "scanning", "recon", "waf").
    #[pyo3(get)]
    pub category: String,
    /// Risk classification string (e.g. "safe_active", "intrusive").
    #[pyo3(get)]
    pub risk: String,
    /// Feature flag required to use this tool, or None if always available.
    #[pyo3(get)]
    pub feature_required: Option<String>,
    /// Maturity level (e.g. "stable", "provisional", "experimental").
    #[pyo3(get)]
    pub maturity: String,
    /// Whether explicit user confirmation is required before execution.
    #[pyo3(get)]
    pub confirmation_required: bool,
    /// Confirmation message shown to the user, if applicable.
    #[pyo3(get)]
    pub confirmation_message: Option<String>,
    /// Target policy requirement string.
    #[pyo3(get)]
    pub target_policy: String,
    /// JSON Schema string for the tool's input request type, if available.
    #[pyo3(get)]
    pub input_schema: Option<String>,
    /// JSON Schema string for the tool's output result type, if available.
    #[pyo3(get)]
    pub output_schema: Option<String>,
    /// Whether the tool supports streaming progress events.
    #[pyo3(get)]
    pub supports_streaming: bool,
    /// Whether the tool supports mid-execution cancellation.
    #[pyo3(get)]
    pub supports_cancellation: bool,
    /// Whether the tool supports configurable timeouts.
    #[pyo3(get)]
    pub supports_timeout: bool,
    /// Whether the tool is available on local surfaces (CLI, TUI, MCP).
    #[pyo3(get)]
    pub local_available: bool,
    /// Whether the tool is available via daemon/agent surfaces.
    #[pyo3(get)]
    pub daemon_available: bool,
    /// Intended use cases for this tool.
    #[pyo3(get)]
    pub intended_uses: Vec<String>,
    /// Execution surfaces that support this tool.
    #[pyo3(get)]
    pub supported_surfaces: Vec<String>,
}

impl ToolDescriptorPy {
    /// Build a tool descriptor from a stable operation identity.
    ///
    /// Uses the canonical `StableOperation` enum and engine metadata to
    /// construct a fully-populated descriptor.
    pub fn from_stable(operation: StableOperation) -> Self {
        let tool_id = operation.id().to_string();
        let feature_required = operation.feature_required().map(str::to_string);
        let risk = risk_string_for_stable(operation).to_string();
        let confirmation_required = confirmation_required_for(operation);
        let confirmation_message = confirmation_message_for(operation).map(str::to_string);
        let category = category_for_tool(operation.id()).to_string();
        let target_policy = target_policy_for_stable(operation.id()).to_string();
        let supported_surfaces = surfaces_for_stable(operation.id());
        let intended_uses = intended_uses_for_stable(operation.id());
        let local_available = local_available_for_stable(operation.id());
        let daemon_available = daemon_available_for_stable(operation.id());

        let maturity = "stable".to_string();
        let description = format!(
            "{} security assessment operation",
            operation.name().to_lowercase()
        );

        let input_schema = SchemaGeneratorPy::generate_input_schema_for_tool(operation.id())
            .map(|v| serde_json::to_string_pretty(&v).unwrap_or_default());
        let output_schema = SchemaGeneratorPy::generate_output_schema_for_tool(operation.id())
            .map(|v| serde_json::to_string_pretty(&v).unwrap_or_default());

        Self {
            tool_id,
            operation_id: operation.id().to_string(),
            title: operation.name().to_string(),
            description,
            version: env!("CARGO_PKG_VERSION").to_string(),
            category,
            risk,
            feature_required,
            maturity,
            confirmation_required,
            confirmation_message,
            target_policy,
            input_schema,
            output_schema,
            supports_streaming: true,
            supports_cancellation: true,
            supports_timeout: true,
            local_available,
            daemon_available,
            intended_uses,
            supported_surfaces,
        }
    }

    /// Build a tool descriptor from engine `OperationMetadata`.
    ///
    /// Maps metadata fields to the tool-descriptor model. The `operation_id`
    /// preserves the original dash-separated metadata ID, and the `tool_id`
    /// replaces dashes with underscores.
    pub fn from_operation(meta: &eggsec::config::OperationMetadata) -> Self {
        let operation_id = meta.id.to_string();
        let tool_id = meta.id.replace('-', "_");
        let title = meta.display_name.to_string();
        let risk = format!("{}", meta.risk);
        let feature_required = if meta.required_features.is_empty() {
            None
        } else {
            Some(meta.required_features[0].to_string())
        };
        let target_policy = match meta.target_policy {
            eggsec::config::TargetPolicyKind::NoTarget => "no-target",
            eggsec::config::TargetPolicyKind::OptionalTarget => "optional-target",
            eggsec::config::TargetPolicyKind::TargetRequired => "target-required",
            eggsec::config::TargetPolicyKind::ExplicitScopeRequired => "explicit-scope-required",
            eggsec::config::TargetPolicyKind::PrivateOrLocalRequired => "private-or-local-required",
        }
        .to_string();
        let supported_surfaces = compute_surfaces(
            meta.manual_exposable,
            meta.tui_exposable,
            meta.mcp_exposable,
            meta.rest_exposable,
            meta.agent_exposable,
            meta.grpc_exposable,
        );
        let local_available = meta.manual_exposable || meta.tui_exposable || meta.mcp_exposable;
        let daemon_available = meta.agent_exposable;
        let intended_uses: Vec<String> = meta
            .intended_uses
            .iter()
            .map(|u| format!("{}", u))
            .collect();
        let category = category_for_tool(&tool_id).to_string();

        // Attempt to match to a stable operation for confirmation behavior
        let stable = StableOperation::parse(&tool_id);
        let (confirmation_required, confirmation_message) = if let Some(op) = stable {
            (
                confirmation_required_for(op),
                confirmation_message_for(op).map(str::to_string),
            )
        } else {
            (false, None)
        };

        let description = format!("{} security assessment operation", title.to_lowercase());

        Self {
            tool_id,
            operation_id,
            title,
            description,
            version: env!("CARGO_PKG_VERSION").to_string(),
            category,
            risk,
            feature_required,
            maturity: "stable".to_string(),
            confirmation_required,
            confirmation_message,
            target_policy,
            input_schema: None,
            output_schema: None,
            supports_streaming: true,
            supports_cancellation: true,
            supports_timeout: true,
            local_available,
            daemon_available,
            intended_uses,
            supported_surfaces,
        }
    }
}

#[pymethods]
impl ToolDescriptorPy {
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new_bound(py);
        dict.set_item("tool_id", &self.tool_id)?;
        dict.set_item("operation_id", &self.operation_id)?;
        dict.set_item("title", &self.title)?;
        dict.set_item("description", &self.description)?;
        dict.set_item("version", &self.version)?;
        dict.set_item("category", &self.category)?;
        dict.set_item("risk", &self.risk)?;
        dict.set_item("feature_required", &self.feature_required)?;
        dict.set_item("maturity", &self.maturity)?;
        dict.set_item("confirmation_required", self.confirmation_required)?;
        dict.set_item("confirmation_message", &self.confirmation_message)?;
        dict.set_item("target_policy", &self.target_policy)?;
        dict.set_item("input_schema", &self.input_schema)?;
        dict.set_item("output_schema", &self.output_schema)?;
        dict.set_item("supports_streaming", self.supports_streaming)?;
        dict.set_item("supports_cancellation", self.supports_cancellation)?;
        dict.set_item("supports_timeout", self.supports_timeout)?;
        dict.set_item("local_available", self.local_available)?;
        dict.set_item("daemon_available", self.daemon_available)?;
        dict.set_item("intended_uses", &self.intended_uses)?;
        dict.set_item("supported_surfaces", &self.supported_surfaces)?;
        Ok(dict.unbind())
    }

    /// Serialize to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Serialization error: {}", e))
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "ToolDescriptor(tool_id={:?}, title={:?}, risk={:?})",
            self.tool_id, self.title, self.risk
        )
    }

    fn __str__(&self) -> String {
        format!("{} ({})", self.title, self.tool_id)
    }
}

impl serde::Serialize for ToolDescriptorPy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(21))?;
        map.serialize_entry("tool_id", &self.tool_id)?;
        map.serialize_entry("operation_id", &self.operation_id)?;
        map.serialize_entry("title", &self.title)?;
        map.serialize_entry("description", &self.description)?;
        map.serialize_entry("version", &self.version)?;
        map.serialize_entry("category", &self.category)?;
        map.serialize_entry("risk", &self.risk)?;
        map.serialize_entry("feature_required", &self.feature_required)?;
        map.serialize_entry("maturity", &self.maturity)?;
        map.serialize_entry("confirmation_required", &self.confirmation_required)?;
        map.serialize_entry("confirmation_message", &self.confirmation_message)?;
        map.serialize_entry("target_policy", &self.target_policy)?;
        map.serialize_entry("input_schema", &self.input_schema)?;
        map.serialize_entry("output_schema", &self.output_schema)?;
        map.serialize_entry("supports_streaming", &self.supports_streaming)?;
        map.serialize_entry("supports_cancellation", &self.supports_cancellation)?;
        map.serialize_entry("supports_timeout", &self.supports_timeout)?;
        map.serialize_entry("local_available", &self.local_available)?;
        map.serialize_entry("daemon_available", &self.daemon_available)?;
        map.serialize_entry("intended_uses", &self.intended_uses)?;
        map.serialize_entry("supported_surfaces", &self.supported_surfaces)?;
        map.end()
    }
}

/// Registry of all tool descriptors.
///
/// Provides static methods to query the canonical operation metadata registry
/// through a tool-descriptor lens.
#[pyclass(name = "ToolRegistry")]
pub struct ToolRegistryPy;

#[pymethods]
impl ToolRegistryPy {
    /// List all operations as tool descriptors.
    ///
    /// Returns:
    ///     list[ToolDescriptor]: All 22 stable-core tool descriptors.
    #[staticmethod]
    fn list() -> Vec<ToolDescriptorPy> {
        StableOperation::ALL
            .iter()
            .map(|&op| ToolDescriptorPy::from_stable(op))
            .collect()
    }

    /// Find a tool descriptor by tool ID or operation ID.
    ///
    /// Searches by tool ID first (underscore format), then by operation ID
    /// (dash format), resolving aliases.
    ///
    /// Args:
    ///     tool_id: Tool identifier (e.g. "scan_ports" or "scan-ports").
    ///
    /// Returns:
    ///     ToolDescriptor | None: The matching descriptor, or None if not found.
    #[staticmethod]
    fn get(tool_id: &str) -> Option<ToolDescriptorPy> {
        // Try stable operation parse first (handles both underscore and alias forms)
        if let Some(op) = StableOperation::parse(tool_id) {
            return Some(ToolDescriptorPy::from_stable(op));
        }
        // Try direct metadata lookup (dash-separated)
        if let Some(meta) = eggsec::config::metadata_for_tool_id(tool_id) {
            return Some(ToolDescriptorPy::from_operation(meta));
        }
        // Try converting underscores to dashes for metadata lookup
        let dash_id = tool_id.replace('_', "-");
        if let Some(meta) = eggsec::config::operation_metadata(&dash_id) {
            return Some(ToolDescriptorPy::from_operation(meta));
        }
        None
    }

    /// Get the JSON Schema for an operation's request type.
    ///
    /// Args:
    ///     tool_id: Tool identifier.
    ///
    /// Returns:
    ///     str | None: JSON Schema string, or None if not available.
    #[staticmethod]
    fn schema(tool_id: &str) -> Option<String> {
        Self::get(tool_id).and_then(|d| d.input_schema)
    }

    /// Validate a payload dict against the request schema for an operation.
    ///
    /// Args:
    ///     tool_id: Tool identifier.
    ///     payload: Payload dictionary to validate.
    ///
    /// Returns:
    ///     ValidationReport: Validation result with errors and warnings.
    #[staticmethod]
    fn validate(tool_id: &str, payload: Bound<'_, PyDict>) -> ValidationReportPy {
        let payload_json = match pydict_to_json(&payload) {
            Ok(v) => v,
            Err(e) => {
                return ValidationReportPy {
                    valid: false,
                    errors: vec![format!("Failed to serialize payload: {}", e)],
                    warnings: vec![],
                };
            }
        };

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate known required fields based on operation type
        if let Some(op) = StableOperation::parse(tool_id) {
            let required_fields = required_fields_for_operation(op);
            for field in &required_fields {
                if !payload_json.get(*field).is_some() {
                    errors.push(format!("Missing required field: '{}'", field));
                }
            }

            // Warn on unexpected fields
            let known = known_fields_for_operation(op);
            if let Some(obj) = payload_json.as_object() {
                for key in obj.keys() {
                    if !known.contains(&key.as_str()) {
                        warnings.push(format!("Unexpected field: '{}'", key));
                    }
                }
            }
        } else {
            warnings.push(format!(
                "Unknown operation '{}'; skipping field validation",
                tool_id
            ));
        }

        ValidationReportPy {
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Total number of registered tool descriptors.
    ///
    /// Returns:
    ///     int: Count of all stable-core operations.
    #[staticmethod]
    fn count() -> usize {
        StableOperation::ALL.len()
    }

    /// Find all operations requiring a specific feature flag.
    ///
    /// Args:
    ///     feature: Feature flag name (e.g. "db-pentest", "mobile").
    ///
    /// Returns:
    ///     list[ToolDescriptor]: Operations gated by the given feature.
    #[staticmethod]
    fn operations_for_feature(feature: &str) -> Vec<ToolDescriptorPy> {
        StableOperation::ALL
            .iter()
            .filter(|op| op.feature_required() == Some(feature))
            .map(|&op| ToolDescriptorPy::from_stable(op))
            .collect()
    }

    /// Find all operations in a given category.
    ///
    /// Args:
    ///     category: Category name (e.g. "scanning", "recon", "waf").
    ///
    /// Returns:
    ///     list[ToolDescriptor]: Operations in the given category.
    #[staticmethod]
    fn operations_for_category(category: &str) -> Vec<ToolDescriptorPy> {
        StableOperation::ALL
            .iter()
            .filter(|op| category_for_tool(op.id()) == category)
            .map(|&op| ToolDescriptorPy::from_stable(op))
            .collect()
    }

    /// Generate tool descriptors from the operation registry.
    ///
    /// This ensures tool descriptors stay in sync with operation metadata.
    /// Equivalent to `list()` but named to emphasize registry derivation.
    #[staticmethod]
    fn from_registry() -> Vec<ToolDescriptorPy> {
        StableOperation::ALL
            .iter()
            .map(|&op| ToolDescriptorPy::from_stable(op))
            .collect()
    }

    fn __repr__(&self) -> String {
        "ToolRegistry".to_string()
    }
}

/// View of an operation as a tool, including request/result type metadata.
///
/// Created by `operation_as_tool()` and provides invocation guidance
/// alongside the core tool descriptor.
#[pyclass(frozen, name = "OperationToolView")]
#[derive(Clone, Debug)]
pub struct OperationToolViewPy {
    /// The underlying tool descriptor.
    #[pyo3(get)]
    pub descriptor: ToolDescriptorPy,
    /// Name of the request type (e.g. "PortScanRequest").
    #[pyo3(get)]
    pub request_type_name: String,
    /// Name of the result type (e.g. "PortScanResult").
    #[pyo3(get)]
    pub result_type_name: String,
    /// Example request as a JSON string, if available.
    #[pyo3(get)]
    pub example_request: Option<String>,
    /// Example result as a JSON string, if available.
    #[pyo3(get)]
    pub example_result: Option<String>,
}

#[pymethods]
impl OperationToolViewPy {
    /// Generate a human-readable invocation guide.
    ///
    /// Returns:
    ///     str: Multi-line guide showing how to invoke this operation.
    fn invoke_description(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("=== {} ===", self.descriptor.title));
        lines.push(format!("Tool ID: {}", self.descriptor.tool_id));
        lines.push(format!("Category: {}", self.descriptor.category));
        lines.push(format!("Risk: {}", self.descriptor.risk));
        lines.push(format!(
            "Confirmation required: {}",
            self.descriptor.confirmation_required
        ));

        if let Some(ref feature) = self.descriptor.feature_required {
            lines.push(format!("Required feature: {}", feature));
        }

        lines.push(format!("Target policy: {}", self.descriptor.target_policy));
        lines.push(format!(
            "Surfaces: {}",
            self.descriptor.supported_surfaces.join(", ")
        ));
        lines.push(format!("Request type: {}", self.request_type_name));
        lines.push(format!("Result type: {}", self.result_type_name));

        if let Some(ref schema) = self.descriptor.input_schema {
            lines.push("Input schema:".to_string());
            lines.push(schema.clone());
        }

        if let Some(ref example) = self.example_request {
            lines.push("Example request:".to_string());
            lines.push(example.clone());
        }

        lines.join("\n")
    }

    /// Serialize all fields to a Python dict.
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new_bound(py);
        dict.set_item("descriptor", &self.descriptor.to_dict(py)?)?;
        dict.set_item("request_type_name", &self.request_type_name)?;
        dict.set_item("result_type_name", &self.result_type_name)?;
        dict.set_item("example_request", &self.example_request)?;
        dict.set_item("example_result", &self.example_result)?;
        Ok(dict.unbind())
    }

    /// Serialize to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Serialization error: {}", e))
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "OperationToolView(tool_id={:?}, request={}, result={})",
            self.descriptor.tool_id, self.request_type_name, self.result_type_name
        )
    }
}

impl serde::Serialize for OperationToolViewPy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("descriptor", &self.descriptor)?;
        map.serialize_entry("request_type_name", &self.request_type_name)?;
        map.serialize_entry("result_type_name", &self.result_type_name)?;
        map.serialize_entry("example_request", &self.example_request)?;
        map.serialize_entry("example_result", &self.example_result)?;
        map.end()
    }
}

/// Create a tool-oriented view of a stable operation.
///
/// Args:
///     operation_id: The stable operation ID (e.g. "scan_ports").
///
/// Returns:
///     OperationToolView | None: The view, or None if the operation is unknown.
#[pyfunction]
pub fn operation_as_tool(operation_id: &str) -> Option<OperationToolViewPy> {
    let op = StableOperation::parse(operation_id)?;
    let descriptor = ToolDescriptorPy::from_stable(op);
    let (request_type, result_type) = request_result_type_names(op);
    let example_request = example_request_json(op);
    let example_result = example_result_json(op);

    Some(OperationToolViewPy {
        descriptor,
        request_type_name: request_type,
        result_type_name: result_type,
        example_request,
        example_result,
    })
}

/// Result of payload validation against a tool's request schema.
///
/// Reports errors for missing required fields and warnings for
/// unexpected fields.
#[pyclass(frozen, name = "ValidationReport")]
#[derive(Clone, Debug)]
pub struct ValidationReportPy {
    /// Whether the payload is valid (no errors).
    #[pyo3(get)]
    pub valid: bool,
    /// List of validation error messages.
    #[pyo3(get)]
    pub errors: Vec<String>,
    /// List of validation warning messages.
    #[pyo3(get)]
    pub warnings: Vec<String>,
}

#[pymethods]
impl ValidationReportPy {
    /// Serialize all fields to a Python dict.
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new_bound(py);
        dict.set_item("valid", self.valid)?;
        dict.set_item("errors", &self.errors)?;
        dict.set_item("warnings", &self.warnings)?;
        Ok(dict.unbind())
    }

    /// Serialize to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Serialization error: {}", e))
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "ValidationReport(valid={}, errors={}, warnings={})",
            self.valid,
            self.errors.len(),
            self.warnings.len()
        )
    }

    /// Boolean truth: returns `valid`.
    fn __bool__(&self) -> bool {
        self.valid
    }
}

impl serde::Serialize for ValidationReportPy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("valid", &self.valid)?;
        map.serialize_entry("errors", &self.errors)?;
        map.serialize_entry("warnings", &self.warnings)?;
        map.end()
    }
}

/// JSON Schema generation for tool inputs and outputs.
///
/// Generates JSON Schema Draft 2020-12 schemas for tool request and
/// result types.
#[pyclass(name = "SchemaGenerator")]
pub struct SchemaGeneratorPy;

#[pymethods]
impl SchemaGeneratorPy {
    /// Generate a JSON Schema for the tool's input request type.
    ///
    /// Args:
    ///     tool_id: Tool identifier (e.g. "scan_ports").
    ///
    /// Returns:
    ///     str | None: JSON Schema string in Draft 2020-12 format, or None.
    #[staticmethod]
    fn generate_input_schema(tool_id: &str) -> Option<String> {
        Self::generate_input_schema_for_tool(tool_id)
            .map(|v| serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".to_string()))
    }

    /// Generate a JSON Schema for the tool's output result type.
    ///
    /// Args:
    ///     tool_id: Tool identifier (e.g. "scan_ports").
    ///
    /// Returns:
    ///     str | None: JSON Schema string in Draft 2020-12 format, or None.
    #[staticmethod]
    fn generate_output_schema(tool_id: &str) -> Option<String> {
        Self::generate_output_schema_for_tool(tool_id)
            .map(|v| serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".to_string()))
    }

    /// Generate all schemas for all registered tools.
    ///
    /// Returns:
    ///     dict[str, dict]: Map of tool_id to {input_schema, output_schema}.
    #[staticmethod]
    fn all_schemas(py: Python<'_>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new_bound(py);
        for &op in StableOperation::ALL {
            let tool_id = op.id();
            let entry = PyDict::new_bound(py);

            if let Some(input) = Self::generate_input_schema_for_tool(tool_id) {
                let input_str = serde_json::to_string_pretty(&input).unwrap_or_default();
                entry.set_item("input_schema", input_str)?;
            }

            if let Some(output) = Self::generate_output_schema_for_tool(tool_id) {
                let output_str = serde_json::to_string_pretty(&output).unwrap_or_default();
                entry.set_item("output_schema", output_str)?;
            }

            dict.set_item(tool_id, entry)?;
        }
        Ok(dict.unbind())
    }

    fn __repr__(&self) -> String {
        "SchemaGenerator".to_string()
    }
}

impl SchemaGeneratorPy {
    /// Internal: generate input schema JSON value for a tool.
    pub fn generate_input_schema_for_tool(tool_id: &str) -> Option<serde_json::Value> {
        let op = StableOperation::parse(tool_id)?;
        let risk = risk_string_for_stable(op);
        let maturity = "stable";

        match op {
            StableOperation::ScanPorts => Some(json_schema(
                tool_id,
                "Port scan request",
                maturity,
                risk,
                json!({
                    "target": { "type": "string", "description": "Target hostname, IP, or CIDR range" },
                    "ports": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Port ranges to scan (e.g. '1-1024', '80,443')"
                    },
                    "timeout_ms": { "type": "integer", "description": "Scan timeout in milliseconds" }
                }),
                vec!["target"],
            )),
            StableOperation::ScanEndpoints => Some(json_schema(
                tool_id,
                "Endpoint scan request",
                maturity,
                risk,
                json!({
                    "target": { "type": "string", "description": "Target URL or host" },
                    "paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Paths to probe"
                    },
                    "timeout_ms": { "type": "integer", "description": "Timeout in milliseconds" }
                }),
                vec!["target"],
            )),
            StableOperation::FingerprintServices => Some(json_schema(
                tool_id,
                "Service fingerprint request",
                maturity,
                risk,
                json!({
                    "target": { "type": "string", "description": "Target host or IP" },
                    "ports": {
                        "type": "array",
                        "items": { "type": "integer" },
                        "description": "Ports to fingerprint"
                    },
                    "timeout_ms": { "type": "integer", "description": "Timeout in milliseconds" }
                }),
                vec!["target"],
            )),
            StableOperation::ReconDns => Some(json_schema(
                tool_id,
                "DNS reconnaissance request",
                maturity,
                risk,
                json!({
                    "target": { "type": "string", "description": "Target domain" },
                    "timeout_ms": { "type": "integer", "description": "Timeout in milliseconds" }
                }),
                vec!["target"],
            )),
            StableOperation::InspectTls => Some(json_schema(
                tool_id,
                "TLS inspection request",
                maturity,
                risk,
                json!({
                    "target": { "type": "string", "description": "Target host:port" },
                    "timeout_ms": { "type": "integer", "description": "Timeout in milliseconds" }
                }),
                vec!["target"],
            )),
            StableOperation::DetectTechnology => Some(json_schema(
                tool_id,
                "Technology detection request",
                maturity,
                risk,
                json!({
                    "target": { "type": "string", "description": "Target URL" },
                    "timeout_ms": { "type": "integer", "description": "Timeout in milliseconds" }
                }),
                vec!["target"],
            )),
            StableOperation::DetectWaf => Some(json_schema(
                tool_id,
                "WAF detection request",
                maturity,
                risk,
                json!({
                    "target": { "type": "string", "description": "Target URL" },
                    "timeout_ms": { "type": "integer", "description": "Timeout in milliseconds" }
                }),
                vec!["target"],
            )),
            StableOperation::ValidateWaf => Some(json_schema(
                tool_id,
                "WAF validation request",
                maturity,
                risk,
                json!({
                    "target": { "type": "string", "description": "Target URL" },
                    "scope": {
                        "type": "string",
                        "description": "Validation scope",
                        "enum": ["passive", "active", "bypass"]
                    }
                }),
                vec!["target"],
            )),
            StableOperation::FuzzHttp => Some(json_schema(
                tool_id,
                "HTTP fuzzing request",
                maturity,
                risk,
                json!({
                    "target": { "type": "string", "description": "Target URL" },
                    "scope": {
                        "type": "string",
                        "description": "Fuzz scope",
                        "enum": ["auto", "paths", "params", "headers", "body"]
                    },
                    "concurrency": {
                        "type": "integer",
                        "description": "Concurrent fuzzing workers",
                        "minimum": 1,
                        "maximum": 100
                    }
                }),
                vec!["target"],
            )),
            StableOperation::LoadTest => Some(json_schema(
                tool_id,
                "Load test request",
                maturity,
                risk,
                json!({
                    "target": { "type": "string", "description": "Target URL" },
                    "total_requests": {
                        "type": "integer",
                        "description": "Total number of requests to send",
                        "minimum": 1
                    },
                    "concurrency": {
                        "type": "integer",
                        "description": "Concurrent workers",
                        "minimum": 1
                    }
                }),
                vec!["target", "total_requests", "concurrency"],
            )),
            StableOperation::ScanGitSecrets => Some(json_schema(
                tool_id,
                "Git secrets scan request",
                maturity,
                risk,
                json!({
                    "path": { "type": "string", "description": "Repository path" },
                    "depth": { "type": "integer", "description": "Scan depth (commit history)" }
                }),
                vec!["path"],
            )),
            StableOperation::GenerateSbom => Some(json_schema(
                tool_id,
                "SBOM generation request",
                maturity,
                risk,
                json!({
                    "path": { "type": "string", "description": "Project root path" },
                    "format": {
                        "type": "string",
                        "description": "Output format",
                        "enum": ["spdx", "cyclonedx"]
                    }
                }),
                vec!["path"],
            )),
            StableOperation::RunConsolidatedRecon => Some(json_schema(
                tool_id,
                "Consolidated recon request",
                maturity,
                risk,
                json!({
                    "target": { "type": "string", "description": "Target URL or domain" },
                    "timeout_ms": { "type": "integer", "description": "Timeout in milliseconds" }
                }),
                vec!["target"],
            )),
            StableOperation::GraphqlTest => Some(json_schema(
                tool_id,
                "GraphQL security test request",
                maturity,
                risk,
                json!({
                    "target": { "type": "string", "description": "GraphQL endpoint URL" },
                    "timeout_ms": { "type": "integer", "description": "Timeout in milliseconds" }
                }),
                vec!["target"],
            )),
            StableOperation::OauthTest => Some(json_schema(
                tool_id,
                "OAuth security test request",
                maturity,
                risk,
                json!({
                    "target": { "type": "string", "description": "Target URL" },
                    "timeout_ms": { "type": "integer", "description": "Timeout in milliseconds" }
                }),
                vec!["target"],
            )),
            StableOperation::AuthTest => Some(json_schema(
                tool_id,
                "Authentication test request",
                maturity,
                risk,
                json!({
                    "target": { "type": "string", "description": "Target URL" },
                    "timeout_ms": { "type": "integer", "description": "Timeout in milliseconds" }
                }),
                vec!["target"],
            )),
            StableOperation::DbProbe => Some(json_schema(
                tool_id,
                "Database probe request",
                maturity,
                risk,
                json!({
                    "connection": {
                        "type": "string",
                        "description": "Database connection string"
                    },
                    "queries": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Queries to execute"
                    }
                }),
                vec!["connection"],
            )),
            StableOperation::NseRun => Some(json_schema(
                tool_id,
                "NSE script execution request",
                maturity,
                risk,
                json!({
                    "target": { "type": "string", "description": "Target host or IP" },
                    "scripts": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "NSE script names to run"
                    }
                }),
                vec!["target", "scripts"],
            )),
            StableOperation::ScanDockerImage => Some(json_schema(
                tool_id,
                "Docker image scan request",
                maturity,
                risk,
                json!({
                    "image": { "type": "string", "description": "Docker image name or ID" }
                }),
                vec!["image"],
            )),
            StableOperation::ScanKubernetes => Some(json_schema(
                tool_id,
                "Kubernetes scan request",
                maturity,
                risk,
                json!({
                    "kubeconfig": {
                        "type": "string",
                        "description": "Path to kubeconfig file"
                    }
                }),
                vec!["kubeconfig"],
            )),
            StableOperation::AnalyzeApk => Some(json_schema(
                tool_id,
                "APK analysis request",
                maturity,
                risk,
                json!({
                    "path": { "type": "string", "description": "Path to APK file" }
                }),
                vec!["path"],
            )),
            StableOperation::AnalyzeIpa => Some(json_schema(
                tool_id,
                "IPA analysis request",
                maturity,
                risk,
                json!({
                    "path": { "type": "string", "description": "Path to IPA file" }
                }),
                vec!["path"],
            )),
        }
    }

    /// Internal: generate output schema JSON value for a tool.
    pub fn generate_output_schema_for_tool(tool_id: &str) -> Option<serde_json::Value> {
        let op = StableOperation::parse(tool_id)?;
        let risk = risk_string_for_stable(op);
        let maturity = "stable";

        match op {
            StableOperation::ScanPorts => Some(json_schema(
                tool_id,
                "Port scan result",
                maturity,
                risk,
                json!({
                    "open_ports": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "port": { "type": "integer" },
                                "state": { "type": "string", "enum": ["open", "closed", "filtered"] },
                                "service": { "type": "string" }
                            }
                        }
                    },
                    "stats": {
                        "type": "object",
                        "properties": {
                            "total_scanned": { "type": "integer" },
                            "open_count": { "type": "integer" },
                            "duration_ms": { "type": "integer" }
                        }
                    }
                }),
                vec![],
            )),
            StableOperation::ScanEndpoints => Some(json_schema(
                tool_id,
                "Endpoint scan result",
                maturity,
                risk,
                json!({
                    "findings": { "type": "array", "items": { "$ref": "#/$defs/Finding" } },
                    "stats": {
                        "type": "object",
                        "properties": {
                            "total_probed": { "type": "integer" },
                            "endpoints_found": { "type": "integer" }
                        }
                    }
                }),
                vec![],
            )),
            StableOperation::FingerprintServices => Some(json_schema(
                tool_id,
                "Service fingerprint result",
                maturity,
                risk,
                json!({
                    "services": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "port": { "type": "integer" },
                                "service": { "type": "string" },
                                "version": { "type": "string" },
                                "confidence": { "type": "string" }
                            }
                        }
                    },
                    "stats": { "type": "object" }
                }),
                vec![],
            )),
            StableOperation::ReconDns => Some(json_schema(
                tool_id,
                "DNS recon result",
                maturity,
                risk,
                json!({
                    "a": { "type": "array", "items": { "type": "string" } },
                    "aaaa": { "type": "array", "items": { "type": "string" } },
                    "mx": { "type": "array", "items": { "type": "string" } },
                    "ns": { "type": "array", "items": { "type": "string" } },
                    "txt": { "type": "array", "items": { "type": "string" } },
                    "soa": { "type": "object" }
                }),
                vec![],
            )),
            StableOperation::InspectTls => Some(json_schema(
                tool_id,
                "TLS inspection result",
                maturity,
                risk,
                json!({
                    "certificate": { "type": "object" },
                    "issues": { "type": "array", "items": { "type": "string" } }
                }),
                vec![],
            )),
            StableOperation::DetectTechnology => Some(json_schema(
                tool_id,
                "Technology detection result",
                maturity,
                risk,
                json!({
                    "technologies": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" },
                                "version": { "type": "string" },
                                "category": { "type": "string" }
                            }
                        }
                    }
                }),
                vec![],
            )),
            StableOperation::DetectWaf => Some(json_schema(
                tool_id,
                "WAF detection result",
                maturity,
                risk,
                json!({
                    "detected": { "type": "boolean" },
                    "vendor": { "type": "string" },
                    "confidence": { "type": "string" }
                }),
                vec![],
            )),
            StableOperation::ValidateWaf => Some(json_schema(
                tool_id,
                "WAF validation result",
                maturity,
                risk,
                json!({
                    "bypasses": { "type": "array", "items": { "type": "string" } },
                    "blocked": { "type": "array", "items": { "type": "string" } }
                }),
                vec![],
            )),
            StableOperation::FuzzHttp => Some(json_schema(
                tool_id,
                "HTTP fuzzing result",
                maturity,
                risk,
                json!({
                    "findings": { "type": "array", "items": { "$ref": "#/$defs/Finding" } },
                    "stats": {
                        "type": "object",
                        "properties": {
                            "total_payloads": { "type": "integer" },
                            "findings_count": { "type": "integer" }
                        }
                    }
                }),
                vec![],
            )),
            StableOperation::LoadTest => Some(json_schema(
                tool_id,
                "Load test result",
                maturity,
                risk,
                json!({
                    "latency": { "type": "object" },
                    "throughput": { "type": "number" },
                    "errors": { "type": "integer" }
                }),
                vec![],
            )),
            StableOperation::ScanGitSecrets => Some(json_schema(
                tool_id,
                "Git secrets scan result",
                maturity,
                risk,
                json!({
                    "findings": { "type": "array", "items": { "$ref": "#/$defs/Finding" } },
                    "summary": { "type": "object" }
                }),
                vec![],
            )),
            StableOperation::GenerateSbom => Some(json_schema(
                tool_id,
                "SBOM generation result",
                maturity,
                risk,
                json!({
                    "components": { "type": "array", "items": { "type": "object" } },
                    "vulnerabilities": { "type": "array", "items": { "type": "object" } }
                }),
                vec![],
            )),
            StableOperation::RunConsolidatedRecon => Some(json_schema(
                tool_id,
                "Consolidated recon result",
                maturity,
                risk,
                json!({
                    "modules": { "type": "array", "items": { "type": "object" } },
                    "summary": { "type": "object" }
                }),
                vec![],
            )),
            StableOperation::GraphqlTest => Some(json_schema(
                tool_id,
                "GraphQL test result",
                maturity,
                risk,
                json!({
                    "vulnerabilities": { "type": "array", "items": { "type": "object" } },
                    "schema": { "type": "object" }
                }),
                vec![],
            )),
            StableOperation::OauthTest => Some(json_schema(
                tool_id,
                "OAuth test result",
                maturity,
                risk,
                json!({
                    "vulnerabilities": { "type": "array", "items": { "type": "object" } },
                    "endpoints": { "type": "array", "items": { "type": "object" } }
                }),
                vec![],
            )),
            StableOperation::AuthTest => Some(json_schema(
                tool_id,
                "Auth test result",
                maturity,
                risk,
                json!({
                    "findings": { "type": "array", "items": { "$ref": "#/$defs/Finding" } },
                    "summary": { "type": "object" }
                }),
                vec![],
            )),
            StableOperation::DbProbe => Some(json_schema(
                tool_id,
                "Database probe result",
                maturity,
                risk,
                json!({
                    "findings": { "type": "array", "items": { "$ref": "#/$defs/Finding" } },
                    "stats": { "type": "object" }
                }),
                vec![],
            )),
            StableOperation::NseRun => Some(json_schema(
                tool_id,
                "NSE run result",
                maturity,
                risk,
                json!({
                    "scripts": { "type": "array", "items": { "type": "object" } },
                    "findings": { "type": "array", "items": { "$ref": "#/$defs/Finding" } }
                }),
                vec![],
            )),
            StableOperation::ScanDockerImage => Some(json_schema(
                tool_id,
                "Docker image scan result",
                maturity,
                risk,
                json!({
                    "misconfigs": { "type": "array", "items": { "type": "object" } },
                    "layers": { "type": "array", "items": { "type": "object" } }
                }),
                vec![],
            )),
            StableOperation::ScanKubernetes => Some(json_schema(
                tool_id,
                "Kubernetes scan result",
                maturity,
                risk,
                json!({
                    "findings": { "type": "array", "items": { "$ref": "#/$defs/Finding" } }
                }),
                vec![],
            )),
            StableOperation::AnalyzeApk => Some(json_schema(
                tool_id,
                "APK analysis result",
                maturity,
                risk,
                json!({
                    "findings": { "type": "array", "items": { "$ref": "#/$defs/Finding" } },
                    "platform": { "type": "string", "const": "android" }
                }),
                vec![],
            )),
            StableOperation::AnalyzeIpa => Some(json_schema(
                tool_id,
                "IPA analysis result",
                maturity,
                risk,
                json!({
                    "findings": { "type": "array", "items": { "$ref": "#/$defs/Finding" } },
                    "platform": { "type": "string", "const": "ios" }
                }),
                vec![],
            )),
        }
    }
}

/// Build a JSON Schema Draft 2020-12 object with standard eggsec extensions.
fn json_schema(
    tool_id: &str,
    title: &str,
    maturity: &str,
    risk: &str,
    properties: serde_json::Value,
    required: Vec<&str>,
) -> serde_json::Value {
    let mut schema = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": format!("urn:eggsec:tool:{}:input", tool_id),
        "title": title,
        "description": format!("{} for the {} tool", title, tool_id),
        "type": "object",
        "properties": properties,
        "x-eggsec-risk": risk,
        "x-eggsec-maturity": maturity,
    });

    if !required.is_empty() {
        schema["required"] = serde_json::json!(required);
    }

    schema
}

// ---------------------------------------------------------------------------
// Type-name and example mappings
// ---------------------------------------------------------------------------

/// Map a stable operation to its request and result type names.
fn request_result_type_names(op: StableOperation) -> (String, String) {
    let (req, res) = match op {
        StableOperation::ScanPorts => ("PortScanRequest", "PortScanResult"),
        StableOperation::ScanEndpoints => ("EndpointScanRequest", "EndpointScanResult"),
        StableOperation::FingerprintServices => ("FingerprintRequest", "FingerprintScanResult"),
        StableOperation::ReconDns => ("ReconDnsRequest", "DnsRecordSet"),
        StableOperation::InspectTls => ("TlsInspectRequest", "TlsInspectionResult"),
        StableOperation::DetectTechnology => ("TechDetectRequest", "TechDetectionResult"),
        StableOperation::DetectWaf => ("WafDetectRequest", "WafDetectionResult"),
        StableOperation::ValidateWaf => ("WafValidateRequest", "WafScanResult"),
        StableOperation::FuzzHttp => ("FuzzRequest", "FuzzResult"),
        StableOperation::LoadTest => ("LoadTestRequest", "LoadTestResult"),
        StableOperation::ScanGitSecrets => ("GitSecretsScanRequest", "GitSecretsReport"),
        StableOperation::GenerateSbom => ("SbomRequest", "SbomReport"),
        StableOperation::RunConsolidatedRecon => {
            ("ConsolidatedReconRequest", "ConsolidatedReconReport")
        }
        StableOperation::GraphqlTest => ("GraphqlTestRequest", "GraphQLTestResult"),
        StableOperation::OauthTest => ("OauthTestRequest", "OAuthTestResult"),
        StableOperation::AuthTest => ("AuthTestRequest", "AuthTestReport"),
        StableOperation::DbProbe => ("DbProbeRequest", "DbPentestReport"),
        StableOperation::NseRun => ("NseRunRequest", "NseReport"),
        StableOperation::ScanDockerImage => ("DockerImageScanRequest", "DockerScanResult"),
        StableOperation::ScanKubernetes => ("KubernetesScanRequest", "KubernetesScanResult"),
        StableOperation::AnalyzeApk => ("ApkAnalysisRequest", "MobileScanReport"),
        StableOperation::AnalyzeIpa => ("IpaAnalysisRequest", "MobileScanReport"),
    };
    (req.to_string(), res.to_string())
}

/// Return an example request JSON string for a stable operation.
fn example_request_json(op: StableOperation) -> Option<String> {
    let json = match op {
        StableOperation::ScanPorts => serde_json::json!({
            "target": "192.168.1.1",
            "ports": ["22", "80", "443", "8080"]
        }),
        StableOperation::ScanEndpoints => serde_json::json!({
            "target": "https://example.com",
            "paths": ["/api", "/admin", "/login"]
        }),
        StableOperation::FingerprintServices => serde_json::json!({
            "target": "192.168.1.1",
            "ports": [80, 443, 8080]
        }),
        StableOperation::ReconDns => serde_json::json!({
            "target": "example.com"
        }),
        StableOperation::InspectTls => serde_json::json!({
            "target": "example.com:443"
        }),
        StableOperation::DetectTechnology => serde_json::json!({
            "target": "https://example.com"
        }),
        StableOperation::DetectWaf => serde_json::json!({
            "target": "https://example.com"
        }),
        StableOperation::ValidateWaf => serde_json::json!({
            "target": "https://example.com",
            "scope": "passive"
        }),
        StableOperation::FuzzHttp => serde_json::json!({
            "target": "https://example.com/api",
            "scope": "auto",
            "concurrency": 10
        }),
        StableOperation::LoadTest => serde_json::json!({
            "target": "https://example.com",
            "total_requests": 1000,
            "concurrency": 50
        }),
        StableOperation::ScanGitSecrets => serde_json::json!({
            "path": "/path/to/repo"
        }),
        StableOperation::GenerateSbom => serde_json::json!({
            "path": "/path/to/project",
            "format": "cyclonedx"
        }),
        StableOperation::RunConsolidatedRecon => serde_json::json!({
            "target": "https://example.com"
        }),
        StableOperation::GraphqlTest => serde_json::json!({
            "target": "https://example.com/graphql"
        }),
        StableOperation::OauthTest => serde_json::json!({
            "target": "https://example.com"
        }),
        StableOperation::AuthTest => serde_json::json!({
            "target": "https://example.com/login"
        }),
        StableOperation::DbProbe => serde_json::json!({
            "connection": "postgresql://user:pass@localhost:5432/mydb"
        }),
        StableOperation::NseRun => serde_json::json!({
            "target": "192.168.1.1",
            "scripts": ["http-title", "ssl-cert"]
        }),
        StableOperation::ScanDockerImage => serde_json::json!({
            "image": "nginx:latest"
        }),
        StableOperation::ScanKubernetes => serde_json::json!({
            "kubeconfig": "~/.kube/config"
        }),
        StableOperation::AnalyzeApk => serde_json::json!({
            "path": "/path/to/app.apk"
        }),
        StableOperation::AnalyzeIpa => serde_json::json!({
            "path": "/path/to/app.ipa"
        }),
    };
    Some(serde_json::to_string_pretty(&json).unwrap_or_default())
}

/// Return an example result JSON string for a stable operation.
fn example_result_json(op: StableOperation) -> Option<String> {
    let json = match op {
        StableOperation::ScanPorts => serde_json::json!({
            "open_ports": [
                { "port": 80, "state": "open", "service": "http" },
                { "port": 443, "state": "open", "service": "https" }
            ],
            "stats": { "total_scanned": 100, "open_count": 2, "duration_ms": 1234 }
        }),
        StableOperation::ReconDns => serde_json::json!({
            "a": ["93.184.216.34"],
            "aaaa": ["2606:2800:220:1:248:1893:25c8:1946"],
            "mx": ["mx1.example.com"],
            "ns": ["ns1.example.com"],
            "txt": ["v=spf1 include:_spf.example.com ~all"]
        }),
        StableOperation::DetectWaf => serde_json::json!({
            "detected": true,
            "vendor": "Cloudflare",
            "confidence": "high"
        }),
        _ => serde_json::json!({}),
    };
    Some(serde_json::to_string_pretty(&json).unwrap_or_default())
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

/// Required fields for each stable operation's request type.
fn required_fields_for_operation(op: StableOperation) -> Vec<&'static str> {
    match op {
        StableOperation::ScanPorts => vec!["target"],
        StableOperation::ScanEndpoints => vec!["target"],
        StableOperation::FingerprintServices => vec!["target"],
        StableOperation::ReconDns => vec!["target"],
        StableOperation::InspectTls => vec!["target"],
        StableOperation::DetectTechnology => vec!["target"],
        StableOperation::DetectWaf => vec!["target"],
        StableOperation::ValidateWaf => vec!["target"],
        StableOperation::FuzzHttp => vec!["target"],
        StableOperation::LoadTest => vec!["target", "total_requests", "concurrency"],
        StableOperation::ScanGitSecrets => vec!["path"],
        StableOperation::GenerateSbom => vec!["path"],
        StableOperation::RunConsolidatedRecon => vec!["target"],
        StableOperation::GraphqlTest => vec!["target"],
        StableOperation::OauthTest => vec!["target"],
        StableOperation::AuthTest => vec!["target"],
        StableOperation::DbProbe => vec!["connection"],
        StableOperation::NseRun => vec!["target", "scripts"],
        StableOperation::ScanDockerImage => vec!["image"],
        StableOperation::ScanKubernetes => vec!["kubeconfig"],
        StableOperation::AnalyzeApk => vec!["path"],
        StableOperation::AnalyzeIpa => vec!["path"],
    }
}

/// Known fields for each stable operation (used for unexpected-field warnings).
fn known_fields_for_operation(op: StableOperation) -> Vec<&'static str> {
    match op {
        StableOperation::ScanPorts => vec!["target", "ports", "timeout_ms"],
        StableOperation::ScanEndpoints => vec!["target", "paths", "timeout_ms"],
        StableOperation::FingerprintServices => vec!["target", "ports", "timeout_ms"],
        StableOperation::ReconDns => vec!["target", "timeout_ms"],
        StableOperation::InspectTls => vec!["target", "timeout_ms"],
        StableOperation::DetectTechnology => vec!["target", "timeout_ms"],
        StableOperation::DetectWaf => vec!["target", "timeout_ms"],
        StableOperation::ValidateWaf => vec!["target", "scope"],
        StableOperation::FuzzHttp => vec!["target", "scope", "concurrency"],
        StableOperation::LoadTest => vec!["target", "total_requests", "concurrency"],
        StableOperation::ScanGitSecrets => vec!["path", "depth"],
        StableOperation::GenerateSbom => vec!["path", "format"],
        StableOperation::RunConsolidatedRecon => vec!["target", "timeout_ms"],
        StableOperation::GraphqlTest => vec!["target", "timeout_ms"],
        StableOperation::OauthTest => vec!["target", "timeout_ms"],
        StableOperation::AuthTest => vec!["target", "timeout_ms"],
        StableOperation::DbProbe => vec!["connection", "queries"],
        StableOperation::NseRun => vec!["target", "scripts"],
        StableOperation::ScanDockerImage => vec!["image"],
        StableOperation::ScanKubernetes => vec!["kubeconfig"],
        StableOperation::AnalyzeApk => vec!["path"],
        StableOperation::AnalyzeIpa => vec!["path"],
    }
}

// ---------------------------------------------------------------------------
// PyDict to serde_json conversion
// ---------------------------------------------------------------------------

/// Recursively convert a Python dict to a `serde_json::Value`.
fn pydict_to_json(dict: &Bound<'_, PyDict>) -> PyResult<serde_json::Value> {
    let mut map = serde_json::Map::new();
    for (key, value) in dict.iter() {
        if let Ok(key_str) = key.extract::<String>() {
            let json_val = pyobject_to_json(&value)?;
            map.insert(key_str, json_val);
        }
    }
    Ok(serde_json::Value::Object(map))
}

/// Recursively convert a Python object to a `serde_json::Value`.
fn pyobject_to_json(obj: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    if let Ok(v) = obj.extract::<bool>() {
        return Ok(serde_json::Value::Bool(v));
    }
    if let Ok(v) = obj.extract::<i64>() {
        return Ok(serde_json::json!(v));
    }
    if let Ok(v) = obj.extract::<f64>() {
        return Ok(serde_json::json!(v));
    }
    if let Ok(v) = obj.extract::<String>() {
        return Ok(serde_json::Value::String(v));
    }
    if let Ok(list) = obj.downcast::<pyo3::types::PyList>() {
        let mut arr = Vec::new();
        for item in list.iter() {
            arr.push(pyobject_to_json(&item)?);
        }
        return Ok(serde_json::Value::Array(arr));
    }
    if let Ok(dict) = obj.downcast::<PyDict>() {
        return pydict_to_json(dict);
    }
    if obj.is_none() {
        return Ok(serde_json::Value::Null);
    }
    // Fallback: convert to string representation
    Ok(serde_json::Value::String(format!("{}", obj)))
}

// ---------------------------------------------------------------------------
// OpenAPI derived adapter
// ---------------------------------------------------------------------------

/// Convert a JSON Schema object to an OpenAPI 3.0 parameter schema.
fn json_schema_to_openapi_param(schema: &serde_json::Value) -> serde_json::Value {
    let mut param = serde_json::json!({});
    if let Some(desc) = schema.get("description") {
        param["description"] = desc.clone();
    }
    if let Some(t) = schema.get("type") {
        param["schema"] = serde_json::json!({ "type": t.clone() });
        // Map JSON Schema types to OpenAPI parameter locations
        match t.as_str() {
            Some("string") => param["schema"]["type"] = serde_json::json!("string"),
            Some("integer") => param["schema"]["type"] = serde_json::json!("integer"),
            Some("number") => param["schema"]["type"] = serde_json::json!("number"),
            Some("boolean") => param["schema"]["type"] = serde_json::json!("boolean"),
            Some("array") => {
                param["schema"]["type"] = serde_json::json!("array");
                if let Some(items) = schema.get("items") {
                    param["schema"]["items"] = items.clone();
                }
            }
            _ => {}
        }
    }
    if let Some(enums) = schema.get("enum") {
        param["schema"]["enum"] = enums.clone();
    }
    if let Some(min) = schema.get("minimum") {
        param["schema"]["minimum"] = min.clone();
    }
    if let Some(max) = schema.get("maximum") {
        param["schema"]["maximum"] = max.clone();
    }
    param
}

/// Convert a tool descriptor's input schema to an OpenAPI 3.0 operation object.
///
/// This is a derived adapter: JSON Schema remains the canonical tool contract.
/// The OpenAPI output is a convenience for frameworks that require OpenAPI
/// specifications (e.g., REST API generators, Swagger UI).
#[pyclass(name = "OpenApiAdapter")]
pub struct OpenApiAdapterPy;

#[pymethods]
impl OpenApiAdapterPy {
    /// Convert a tool descriptor to an OpenAPI 3.0 path item.
    ///
    /// Args:
    ///     tool_id: Tool identifier (e.g. "scan_ports").
    ///
    /// Returns:
    ///     dict | None: OpenAPI 3.0 path item, or None if schema unavailable.
    #[staticmethod]
    fn tool_to_openapi(tool_id: &str) -> Option<Py<PyDict>> {
        let descriptor = ToolRegistryPy::get(tool_id)?;
        let input_schema_str = descriptor.input_schema.as_deref()?;
        let input_schema: serde_json::Value = serde_json::from_str(input_schema_str).ok()?;

        Python::with_gil(|py| {
            let path_item = PyDict::new_bound(py);

            // Build parameters from schema properties
            let mut parameters = Vec::new();
            if let Some(props) = input_schema.get("properties") {
                if let Some(obj) = props.as_object() {
                    let required_fields: Vec<String> = input_schema
                        .get("required")
                        .and_then(|r| r.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();

                    for (name, schema) in obj {
                        let mut param = json_schema_to_openapi_param(schema);
                        param["name"] = serde_json::json!(name);
                        param["in"] = serde_json::json!("query");
                        param["required"] = serde_json::json!(required_fields.contains(name));
                        parameters.push(param);
                    }
                }
            }

            // Build the operation object as JSON, then convert to Python
            let operation_json = serde_json::json!({
                "operationId": descriptor.tool_id,
                "summary": descriptor.title,
                "description": descriptor.description,
                "tags": [descriptor.category],
                "parameters": parameters,
                "responses": {
                    "200": {
                        "description": "Successful operation",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "status": { "type": "string" },
                                        "findings": { "type": "array" },
                                        "metadata": { "type": "object" }
                                    }
                                }
                            }
                        }
                    }
                },
                "x-eggsec-risk": descriptor.risk,
                "x-eggsec-maturity": descriptor.maturity,
                "x-eggsec-confirmation-required": descriptor.confirmation_required,
            });

            let json_str = serde_json::to_string(&operation_json).ok()?;
            let json_mod = py.import_bound("json").ok()?;
            let op_obj = json_mod.call_method1("loads", (json_str,)).ok()?;

            path_item.set_item("post", op_obj).ok()?;
            Some(path_item.unbind())
        })
    }

    /// Generate a full OpenAPI 3.0 spec for all registered tools.
    ///
    /// Returns:
    ///     dict: OpenAPI 3.0 document with all tool paths.
    #[staticmethod]
    fn full_openapi_spec(py: Python<'_>) -> PyResult<PyObject> {
        let mut paths = serde_json::json!({});

        for desc in ToolRegistryPy::list() {
            if let Some(openapi_op) = Self::tool_to_openapi_inner(&desc) {
                paths[format!("/tools/{}", desc.tool_id)] = openapi_op;
            }
        }

        let spec = serde_json::json!({
            "openapi": "3.0.3",
            "info": {
                "title": "Eggsec Tool API",
                "version": env!("CARGO_PKG_VERSION"),
                "description": "Auto-generated from eggsec tool descriptors"
            },
            "paths": paths,
        });

        // Convert to Python dict
        let json_str = serde_json::to_string(&spec)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let json_mod = py.import_bound("json")?;
        let obj = json_mod.call_method1("loads", (json_str,))?;
        Ok(obj.into())
    }

    fn __repr__(&self) -> String {
        "OpenApiAdapter".to_string()
    }
}

impl OpenApiAdapterPy {
    /// Internal: generate OpenAPI path item from descriptor (no Python GIL needed).
    fn tool_to_openapi_inner(descriptor: &ToolDescriptorPy) -> Option<serde_json::Value> {
        let input_schema_str = descriptor.input_schema.as_deref()?;
        let input_schema: serde_json::Value = serde_json::from_str(input_schema_str).ok()?;

        let mut parameters = Vec::new();
        if let Some(props) = input_schema.get("properties") {
            if let Some(obj) = props.as_object() {
                let required_fields: Vec<String> = input_schema
                    .get("required")
                    .and_then(|r| r.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                for (name, schema) in obj {
                    let mut param = json_schema_to_openapi_param(schema);
                    param["name"] = serde_json::json!(name);
                    param["in"] = serde_json::json!("query");
                    param["required"] = serde_json::json!(required_fields.contains(name));
                    parameters.push(param);
                }
            }
        }

        Some(serde_json::json!({
            "post": {
                "operationId": descriptor.tool_id,
                "summary": descriptor.title,
                "description": descriptor.description,
                "tags": [descriptor.category],
                "parameters": parameters,
                "responses": {
                    "200": {
                        "description": "Successful operation",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "status": { "type": "string" },
                                        "findings": { "type": "array" },
                                        "metadata": { "type": "object" }
                                    }
                                }
                            }
                        }
                    }
                },
                "x-eggsec-risk": descriptor.risk,
                "x-eggsec-maturity": descriptor.maturity,
                "x-eggsec-confirmation-required": descriptor.confirmation_required,
            }
        }))
    }
}
