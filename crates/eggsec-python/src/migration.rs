use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::{Deserialize, Serialize};

use super::finding_schema::{
    AffectedAssetPy, ConfidencePy, EvidenceKindPy, FindingTypePy, VersionedEvidencePy,
    VersionedFindingPy, FINDING_SCHEMA_VERSION,
};

/// Schema version metadata.
#[pyclass(frozen, name = "SchemaVersion")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaVersionPy {
    #[pyo3(get)]
    pub version: String,
    #[pyo3(get)]
    pub created_at: String,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub compatible_with: Vec<String>,
}

#[pymethods]
impl SchemaVersionPy {
    #[new]
    #[pyo3(signature = (version, description, *, compatible_with=None))]
    fn py_new(version: String, description: String, compatible_with: Option<Vec<String>>) -> Self {
        Self {
            version,
            created_at: chrono::Utc::now().to_rfc3339(),
            description,
            compatible_with: compatible_with.unwrap_or_default(),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("version", &self.version)?;
        dict.set_item("created_at", &self.created_at)?;
        dict.set_item("description", &self.description)?;
        dict.set_item("compatible_with", &self.compatible_with)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    fn __repr__(&self) -> String {
        format!(
            "SchemaVersion(version={}, description={})",
            self.version, self.description
        )
    }
}

/// Migration result.
#[pyclass(frozen, name = "MigrationResult")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResultPy {
    #[pyo3(get)]
    pub success: bool,
    #[pyo3(get)]
    pub source_version: String,
    #[pyo3(get)]
    pub target_version: String,
    #[pyo3(get)]
    pub items_migrated: u32,
    #[pyo3(get)]
    pub warnings: Vec<String>,
    #[pyo3(get)]
    pub errors: Vec<String>,
}

#[pymethods]
impl MigrationResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("success", self.success)?;
        dict.set_item("source_version", &self.source_version)?;
        dict.set_item("target_version", &self.target_version)?;
        dict.set_item("items_migrated", self.items_migrated)?;
        dict.set_item("warnings", &self.warnings)?;
        dict.set_item("errors", &self.errors)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    fn __repr__(&self) -> String {
        format!(
            "MigrationResult(success={}, migrated={})",
            self.success, self.items_migrated
        )
    }
}

/// Map a legacy category string to a FindingTypePy.
fn map_legacy_category(category: &str) -> FindingTypePy {
    match category.to_lowercase().as_str() {
        "vulnerability" | "vuln" | "xss" | "sqli" | "csrf" | "ssrf" | "cmdi" | "lfi" | "rfi"
        | "idor" | "xxe" | "deserialization" | "race_condition" => FindingTypePy::Vulnerability,
        "misconfiguration" | "config" | "configuration" | "security_header" | "ssl_tls"
        | "open_redirect" => FindingTypePy::Misconfiguration,
        "information_disclosure" | "info_disclosure" | "info_leak" => {
            FindingTypePy::InformationLeak
        }
        "policy_violation" | "policy" => FindingTypePy::PolicyViolation,
        "service_detection" | "service" => FindingTypePy::ServiceDetection,
        "waf_detection" | "waf" => FindingTypePy::WafDetection,
        "fuzz_result" | "fuzz" => FindingTypePy::FuzzResult,
        "port_scan" | "port" | "endpoint" | "endpoint_discovery" | "scan" => {
            FindingTypePy::ScanResult
        }
        _ => FindingTypePy::ScanResult,
    }
}

/// Normalize a severity string to canonical lowercase form.
fn normalize_severity(severity: &str) -> String {
    match severity.to_lowercase().as_str() {
        "critical" | "crit" => "critical".to_string(),
        "high" => "high".to_string(),
        "medium" | "med" | "moderate" => "medium".to_string(),
        "low" => "low".to_string(),
        "info" | "informational" | "information" => "info".to_string(),
        _ => severity.to_lowercase(),
    }
}

/// Map a string to an EvidenceKindPy.
fn map_evidence_kind(s: &str) -> EvidenceKindPy {
    match s {
        "HttpRequest" | "http_request" | "httprequest" => EvidenceKindPy::HttpRequest,
        "HttpResponse" | "http_response" | "httpresponse" => EvidenceKindPy::HttpResponse,
        "Header" | "header" => EvidenceKindPy::Header,
        "BodySnippet" | "body_snippet" | "bodysnippet" => EvidenceKindPy::BodySnippet,
        "Timing" | "timing" => EvidenceKindPy::Timing,
        "Diff" | "diff" => EvidenceKindPy::Diff,
        "Banner" | "banner" => EvidenceKindPy::Banner,
        "DnsRecord" | "dns_record" | "dnsrecord" => EvidenceKindPy::DnsRecord,
        "Certificate" | "certificate" => EvidenceKindPy::Certificate,
        "PortState" | "port_state" | "portstate" => EvidenceKindPy::PortState,
        "Screenshot" | "screenshot" => EvidenceKindPy::Screenshot,
        "FilePath" | "file_path" | "filepath" => EvidenceKindPy::FilePath,
        "LogLine" | "log_line" | "logline" => EvidenceKindPy::LogLine,
        _ => EvidenceKindPy::BodySnippet,
    }
}

/// Build a VersionedFindingPy from legacy Finding fields.
fn build_legacy_finding(
    id: String,
    title: String,
    severity: String,
    target: String,
    category: String,
    description: String,
    recommendation: Option<String>,
    evidence_str: Option<String>,
    metadata: Option<std::collections::HashMap<String, String>>,
) -> VersionedFindingPy {
    let finding_type = map_legacy_category(&category);
    let severity_normalized = normalize_severity(&severity);

    let mut tags = Vec::new();
    if !category.is_empty() {
        tags.push(category.to_lowercase());
    }
    if let Some(ref meta) = metadata {
        if let Some(t) = meta.get("tags") {
            for tag in t.split(',') {
                let trimmed = tag.trim().to_string();
                if !trimmed.is_empty() && !tags.contains(&trimmed) {
                    tags.push(trimmed);
                }
            }
        }
    }

    let evidence_items: Vec<VersionedEvidencePy> = evidence_str
        .map(|e| {
            vec![VersionedEvidencePy {
                kind: EvidenceKindPy::BodySnippet,
                redacted: false,
                summary: e,
                data: String::new(),
            }]
        })
        .unwrap_or_default();

    let mut details = std::collections::HashMap::new();
    if let Some(meta) = metadata {
        for (k, v) in meta {
            if k != "tags" {
                details.insert(k, v);
            }
        }
    }
    details.insert("legacy_category".to_string(), category);
    details.insert("migration_source".to_string(), "legacy_finding".to_string());

    let metadata_json = serde_json::to_string(&details).unwrap_or_else(|_| "{}".to_string());

    let remediation = recommendation.or_else(|| details.get("recommendation").cloned());

    VersionedFindingPy {
        schema_version: FINDING_SCHEMA_VERSION.to_string(),
        id,
        fingerprint: String::new(),
        title,
        description,
        severity: severity_normalized,
        confidence: ConfidencePy::Medium,
        finding_type,
        cwe: None,
        owasp: None,
        cve: None,
        affected_asset: AffectedAssetPy {
            asset_type: "host".to_string(),
            identifier: target,
            host: None,
            port: None,
            protocol: None,
        },
        location: super::finding_schema::FindingLocationPy::default(),
        evidence: evidence_items,
        remediation,
        tags,
        discovered_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        source_tool: "legacy_migration".to_string(),
        source_module: "finding_migration".to_string(),
        metadata: metadata_json,
    }
}

/// Build a VersionedFindingPy from engine Finding JSON.
fn build_engine_finding(finding_json: &str) -> PyResult<VersionedFindingPy> {
    let parsed: serde_json::Value = serde_json::from_str(finding_json)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    let id = parsed
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let title = parsed
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let severity_str = parsed
        .get("severity")
        .and_then(|v| v.as_str())
        .unwrap_or("info")
        .to_string();
    let target = parsed
        .get("target")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let category = parsed
        .get("category")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let description = parsed
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let recommendation = parsed
        .get("recommendation")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let evidence_items: Vec<VersionedEvidencePy> = parsed
        .get("evidence")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|ev| {
                    let kind_str = ev
                        .get("kind")
                        .and_then(|v| v.as_str())
                        .unwrap_or("BodySnippet");
                    let kind = map_evidence_kind(kind_str);
                    let summary = ev
                        .get("value")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let source = ev
                        .get("source")
                        .and_then(|v| v.as_str())
                        .unwrap_or("engine")
                        .to_string();
                    let redacted = ev
                        .get("redacted")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    Some(VersionedEvidencePy {
                        kind,
                        redacted,
                        summary: format!("[{source}] {summary}"),
                        data: String::new(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let finding_type = map_legacy_category(&category);
    let severity_normalized = normalize_severity(&severity_str);

    let mut details = std::collections::HashMap::new();
    details.insert("migration_source".to_string(), "engine_finding".to_string());
    details.insert("legacy_category".to_string(), category);

    let mut tags = Vec::new();
    if let Some(t) = parsed.get("tags").and_then(|v| v.as_array()) {
        for tag in t {
            if let Some(s) = tag.as_str() {
                tags.push(s.to_string());
            }
        }
    }

    let metadata_json = serde_json::to_string(&details).unwrap_or_else(|_| "{}".to_string());

    Ok(VersionedFindingPy {
        schema_version: FINDING_SCHEMA_VERSION.to_string(),
        id,
        fingerprint: String::new(),
        title,
        description,
        severity: severity_normalized,
        confidence: ConfidencePy::Medium,
        finding_type,
        cwe: parsed
            .get("cwe")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        owasp: None,
        cve: parsed
            .get("cve")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        affected_asset: AffectedAssetPy {
            asset_type: "host".to_string(),
            identifier: target,
            host: None,
            port: None,
            protocol: None,
        },
        location: super::finding_schema::FindingLocationPy::default(),
        evidence: evidence_items,
        remediation: recommendation,
        tags,
        discovered_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        source_tool: "engine_migration".to_string(),
        source_module: "finding_migration".to_string(),
        metadata: metadata_json,
    })
}

/// Migration adapter for converting old finding formats.
#[pyclass(name = "FindingMigration")]
pub struct FindingMigrationPy;

#[pymethods]
impl FindingMigrationPy {
    #[new]
    fn new() -> Self {
        Self
    }

    /// Migrate from legacy Python Finding fields to VersionedFinding.
    fn migrate_legacy_finding(
        &self,
        id: String,
        title: String,
        severity: String,
        target: String,
        category: String,
        description: String,
        recommendation: Option<String>,
        evidence: Option<String>,
        metadata: Option<std::collections::HashMap<String, String>>,
    ) -> VersionedFindingPy {
        build_legacy_finding(
            id,
            title,
            severity,
            target,
            category,
            description,
            recommendation,
            evidence,
            metadata,
        )
    }

    /// Migrate from engine Finding JSON to VersionedFinding.
    fn migrate_engine_finding(&self, finding_json: &str) -> PyResult<VersionedFindingPy> {
        build_engine_finding(finding_json)
    }

    /// Batch migrate findings from a list of legacy dicts.
    fn migrate_batch(&self, legacy_findings: Vec<PyObject>) -> MigrationResultPy {
        let mut warnings = Vec::new();
        let mut errors = Vec::<String>::new();
        let mut migrated = 0u32;

        Python::with_gil(|py| {
            for (i, finding_ref) in legacy_findings.iter().enumerate() {
                let finding = finding_ref.bind(py);
                let get_str = |key: &str| -> String {
                    finding
                        .get_item(key)
                        .ok()
                        .and_then(|v| v.extract::<String>().ok())
                        .unwrap_or_default()
                };
                let get_opt_str = |key: &str| -> Option<String> {
                    finding
                        .get_item(key)
                        .ok()
                        .and_then(|v| v.extract::<String>().ok())
                };

                let id = get_str("id");
                let title = get_str("title");
                let severity = get_str("severity");
                let target = get_str("target");
                let category = get_str("category");
                let description = get_str("description");
                let recommendation = get_opt_str("recommendation");
                let evidence = get_opt_str("evidence");

                if id.is_empty() {
                    warnings.push(format!(
                        "Finding at index {i} has empty ID, using generated ID"
                    ));
                }

                let _result = build_legacy_finding(
                    if id.is_empty() {
                        format!("migrated-{i}")
                    } else {
                        id
                    },
                    title,
                    severity,
                    target,
                    category,
                    description,
                    recommendation,
                    evidence,
                    None,
                );
                migrated += 1;
            }
        });

        MigrationResultPy {
            success: errors.is_empty(),
            source_version: "legacy".to_string(),
            target_version: FINDING_SCHEMA_VERSION.to_string(),
            items_migrated: migrated,
            warnings,
            errors,
        }
    }

    /// Check if a finding JSON string needs migration (lacks schema_version).
    fn needs_migration(&self, finding_json: &str) -> bool {
        match serde_json::from_str::<serde_json::Value>(finding_json) {
            Ok(val) => val.get("schema_version").is_none(),
            Err(_) => true,
        }
    }

    /// Get supported source versions.
    fn supported_versions(&self) -> Vec<String> {
        vec![
            "0.1".to_string(),
            "0.2".to_string(),
            "legacy".to_string(),
            "engine".to_string(),
        ]
    }

    /// Get the target schema version.
    fn target_version(&self) -> String {
        FINDING_SCHEMA_VERSION.to_string()
    }
}
