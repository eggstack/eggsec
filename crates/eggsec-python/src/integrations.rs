use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use super::finding_schema::VersionedFindingPy;

/// Integration type.
#[pyclass(frozen, name = "IntegrationType", eq, eq_int)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntegrationTypePy {
    GitHub,
    GitLab,
    Jira,
    Webhook,
    Custom,
}

#[pymethods]
impl IntegrationTypePy {
    #[staticmethod]
    fn from_str(s: &str) -> PyResult<Self> {
        match s.to_lowercase().as_str() {
            "github" => Ok(Self::GitHub),
            "gitlab" => Ok(Self::GitLab),
            "jira" => Ok(Self::Jira),
            "webhook" => Ok(Self::Webhook),
            "custom" => Ok(Self::Custom),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Unknown integration type: {s}"
            ))),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::GitHub => "github",
            Self::GitLab => "gitlab",
            Self::Jira => "jira",
            Self::Webhook => "webhook",
            Self::Custom => "custom",
        }
    }

    fn __repr__(&self) -> String {
        format!("IntegrationType.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

/// Publication record for audit trail.
#[pyclass(frozen, name = "PublicationRecord")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicationRecordPy {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub integration_type: IntegrationTypePy,
    #[pyo3(get)]
    pub finding_id: String,
    #[pyo3(get)]
    pub external_id: Option<String>,
    #[pyo3(get)]
    pub action: String,
    #[pyo3(get)]
    pub published_at: String,
    #[pyo3(get)]
    pub success: bool,
    #[pyo3(get)]
    pub error: Option<String>,
    #[pyo3(get)]
    pub dry_run: bool,
    #[pyo3(get)]
    pub dedup_key: String,
}

#[pymethods]
impl PublicationRecordPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.id)?;
        dict.set_item("integration_type", self.integration_type.as_str())?;
        dict.set_item("finding_id", &self.finding_id)?;
        dict.set_item("external_id", &self.external_id)?;
        dict.set_item("action", &self.action)?;
        dict.set_item("published_at", &self.published_at)?;
        dict.set_item("success", self.success)?;
        dict.set_item("error", &self.error)?;
        dict.set_item("dry_run", self.dry_run)?;
        dict.set_item("dedup_key", &self.dedup_key)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    fn __repr__(&self) -> String {
        format!(
            "PublicationRecord(id={}, finding={}, action={})",
            self.id, self.finding_id, self.action
        )
    }
}

/// Retry policy for failed publications.
#[pyclass(frozen, name = "IntegrationRetryPolicyPy")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicyPy {
    #[pyo3(get)]
    pub max_retries: u32,
    #[pyo3(get)]
    pub base_delay_ms: u64,
    #[pyo3(get)]
    pub max_delay_ms: u64,
    #[pyo3(get)]
    pub backoff_multiplier: f64,
}

#[pymethods]
impl RetryPolicyPy {
    #[new]
    #[pyo3(signature = (*, max_retries=None, base_delay_ms=None, max_delay_ms=None, backoff_multiplier=None))]
    fn py_new(
        max_retries: Option<u32>,
        base_delay_ms: Option<u64>,
        max_delay_ms: Option<u64>,
        backoff_multiplier: Option<f64>,
    ) -> Self {
        Self {
            max_retries: max_retries.unwrap_or(3),
            base_delay_ms: base_delay_ms.unwrap_or(1000),
            max_delay_ms: max_delay_ms.unwrap_or(30000),
            backoff_multiplier: backoff_multiplier.unwrap_or(2.0),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("max_retries", self.max_retries)?;
        dict.set_item("base_delay_ms", self.base_delay_ms)?;
        dict.set_item("max_delay_ms", self.max_delay_ms)?;
        dict.set_item("backoff_multiplier", self.backoff_multiplier)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    fn __repr__(&self) -> String {
        format!(
            "IntegrationRetryPolicyPy(max_retries={}, base_delay_ms={})",
            self.max_retries, self.base_delay_ms
        )
    }
}

/// Publication policy controlling what gets published.
#[pyclass(frozen, name = "PublicationPolicy")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicationPolicyPy {
    #[pyo3(get)]
    pub dry_run: bool,
    #[pyo3(get)]
    pub include_evidence: bool,
    #[pyo3(get)]
    pub include_artifacts: bool,
    #[pyo3(get)]
    pub redact_sensitive: bool,
    #[pyo3(get)]
    pub min_severity: String,
    #[pyo3(get)]
    pub allowed_finding_types: Vec<String>,
    #[pyo3(get)]
    pub blocked_tags: Vec<String>,
}

#[pymethods]
impl PublicationPolicyPy {
    #[new]
    #[pyo3(signature = (*, dry_run=None, include_evidence=None, include_artifacts=None, redact_sensitive=None, min_severity=None, allowed_finding_types=None, blocked_tags=None))]
    fn py_new(
        dry_run: Option<bool>,
        include_evidence: Option<bool>,
        include_artifacts: Option<bool>,
        redact_sensitive: Option<bool>,
        min_severity: Option<String>,
        allowed_finding_types: Option<Vec<String>>,
        blocked_tags: Option<Vec<String>>,
    ) -> Self {
        Self {
            dry_run: dry_run.unwrap_or(true),
            include_evidence: include_evidence.unwrap_or(false),
            include_artifacts: include_artifacts.unwrap_or(false),
            redact_sensitive: redact_sensitive.unwrap_or(true),
            min_severity: min_severity.unwrap_or_else(|| "Medium".to_string()),
            allowed_finding_types: allowed_finding_types.unwrap_or_default(),
            blocked_tags: blocked_tags.unwrap_or_default(),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("dry_run", self.dry_run)?;
        dict.set_item("include_evidence", self.include_evidence)?;
        dict.set_item("include_artifacts", self.include_artifacts)?;
        dict.set_item("redact_sensitive", self.redact_sensitive)?;
        dict.set_item("min_severity", &self.min_severity)?;
        dict.set_item("allowed_finding_types", &self.allowed_finding_types)?;
        dict.set_item("blocked_tags", &self.blocked_tags)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    fn __repr__(&self) -> String {
        format!(
            "PublicationPolicy(dry_run={}, min_severity={})",
            self.dry_run, self.min_severity
        )
    }
}

/// Severity rank for policy filtering (higher = more severe).
fn severity_rank(s: &str) -> u8 {
    match s.to_lowercase().as_str() {
        "critical" => 5,
        "high" => 4,
        "medium" => 3,
        "low" => 2,
        "info" | "informational" => 1,
        _ => 0,
    }
}

/// Generate a deterministic dedup key from finding metadata.
fn compute_dedup_key(finding: &VersionedFindingPy) -> String {
    if !finding.fingerprint.is_empty() {
        return finding.fingerprint.clone();
    }
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    finding.title.hash(&mut hasher);
    finding.affected_asset.identifier.hash(&mut hasher);
    finding.finding_type.as_str().hash(&mut hasher);
    finding.location.url.hash(&mut hasher);
    finding.location.path.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Shorten a dedup key for use in record IDs.
fn short_key(key: &str) -> &str {
    let len = key.len().min(12);
    &key[..len]
}

/// External integration adapter for publishing findings.
#[pyclass(name = "ExternalIntegration")]
pub struct ExternalIntegrationPy {
    integration_type: IntegrationTypePy,
    name: String,
    config: std::collections::HashMap<String, String>,
    policy: PublicationPolicyPy,
    retry_policy: RetryPolicyPy,
    records: std::sync::RwLock<Vec<PublicationRecordPy>>,
}

#[pymethods]
impl ExternalIntegrationPy {
    #[new]
    #[pyo3(signature = (integration_type, name, config, *, policy=None, retry_policy=None))]
    fn py_new(
        integration_type: IntegrationTypePy,
        name: String,
        config: std::collections::HashMap<String, String>,
        policy: Option<PublicationPolicyPy>,
        retry_policy: Option<RetryPolicyPy>,
    ) -> Self {
        Self {
            integration_type,
            name,
            config,
            policy: policy.unwrap_or_else(|| PublicationPolicyPy {
                dry_run: true,
                include_evidence: false,
                include_artifacts: false,
                redact_sensitive: true,
                min_severity: "Medium".to_string(),
                allowed_finding_types: vec![],
                blocked_tags: vec![],
            }),
            retry_policy: retry_policy.unwrap_or_else(|| RetryPolicyPy {
                max_retries: 3,
                base_delay_ms: 1000,
                max_delay_ms: 30000,
                backoff_multiplier: 2.0,
            }),
            records: std::sync::RwLock::new(Vec::new()),
        }
    }

    /// Return the integration type.
    fn integration_type(&self) -> IntegrationTypePy {
        self.integration_type.clone()
    }

    /// Return the integration name.
    fn name(&self) -> &str {
        &self.name
    }

    /// Return whether the integration is in dry-run mode.
    fn is_dry_run(&self) -> bool {
        self.policy.dry_run
    }

    /// Publish a single finding. Returns a publication record.
    /// In dry-run mode (default), no external calls are made.
    fn publish_finding(&self, finding: VersionedFindingPy) -> PyResult<PublicationRecordPy> {
        let dedup_key = compute_dedup_key(&finding);

        {
            let records = self
                .records
                .read()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            if records
                .iter()
                .any(|r| r.dedup_key == dedup_key && r.success)
            {
                return Ok(PublicationRecordPy {
                    id: format!("pub-{}-skip", short_key(&dedup_key)),
                    integration_type: self.integration_type.clone(),
                    finding_id: finding.id.clone(),
                    external_id: None,
                    action: "skip".to_string(),
                    published_at: chrono::Utc::now().to_rfc3339(),
                    success: true,
                    error: None,
                    dry_run: self.policy.dry_run,
                    dedup_key,
                });
            }
        }

        let finding_sev_rank = severity_rank(&finding.severity);
        let policy_sev_rank = severity_rank(&self.policy.min_severity);
        if finding_sev_rank < policy_sev_rank {
            return Ok(PublicationRecordPy {
                id: format!("pub-{}-skip", short_key(&dedup_key)),
                integration_type: self.integration_type.clone(),
                finding_id: finding.id.clone(),
                external_id: None,
                action: "skip".to_string(),
                published_at: chrono::Utc::now().to_rfc3339(),
                success: true,
                error: Some(format!(
                    "Severity '{}' below policy minimum '{}'",
                    finding.severity, self.policy.min_severity
                )),
                dry_run: self.policy.dry_run,
                dedup_key,
            });
        }

        if !self.policy.allowed_finding_types.is_empty()
            && !self
                .policy
                .allowed_finding_types
                .iter()
                .any(|t| t.to_lowercase() == finding.finding_type.as_str().to_lowercase())
        {
            return Ok(PublicationRecordPy {
                id: format!("pub-{}-skip", short_key(&dedup_key)),
                integration_type: self.integration_type.clone(),
                finding_id: finding.id.clone(),
                external_id: None,
                action: "skip".to_string(),
                published_at: chrono::Utc::now().to_rfc3339(),
                success: true,
                error: Some(format!(
                    "Finding type '{}' not in allowed types",
                    finding.finding_type.as_str()
                )),
                dry_run: self.policy.dry_run,
                dedup_key,
            });
        }

        if finding
            .tags
            .iter()
            .any(|t| self.policy.blocked_tags.contains(t))
        {
            let blocked: Vec<&str> = finding
                .tags
                .iter()
                .filter(|t| self.policy.blocked_tags.contains(t))
                .map(|s| s.as_str())
                .collect();
            return Ok(PublicationRecordPy {
                id: format!("pub-{}-skip", short_key(&dedup_key)),
                integration_type: self.integration_type.clone(),
                finding_id: finding.id.clone(),
                external_id: None,
                action: "skip".to_string(),
                published_at: chrono::Utc::now().to_rfc3339(),
                success: true,
                error: Some(format!("Blocked tags: {}", blocked.join(", "))),
                dry_run: self.policy.dry_run,
                dedup_key,
            });
        }

        let record = if self.policy.dry_run {
            PublicationRecordPy {
                id: format!(
                    "pub-{}-{}",
                    short_key(&dedup_key),
                    chrono::Utc::now().timestamp_millis()
                ),
                integration_type: self.integration_type.clone(),
                finding_id: finding.id.clone(),
                external_id: None,
                action: "create".to_string(),
                published_at: chrono::Utc::now().to_rfc3339(),
                success: true,
                error: None,
                dry_run: true,
                dedup_key,
            }
        } else {
            let external_id = format!("ext-{}", short_key(&dedup_key));
            PublicationRecordPy {
                id: format!(
                    "pub-{}-{}",
                    short_key(&dedup_key),
                    chrono::Utc::now().timestamp_millis()
                ),
                integration_type: self.integration_type.clone(),
                finding_id: finding.id.clone(),
                external_id: Some(external_id),
                action: "create".to_string(),
                published_at: chrono::Utc::now().to_rfc3339(),
                success: true,
                error: None,
                dry_run: false,
                dedup_key,
            }
        };

        {
            let mut records = self
                .records
                .write()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            records.push(record.clone());
        }

        Ok(record)
    }

    /// Publish multiple findings. Returns a list of publication records.
    fn publish_findings(
        &self,
        findings: Vec<VersionedFindingPy>,
    ) -> PyResult<Vec<PublicationRecordPy>> {
        let mut results = Vec::new();
        for finding in findings {
            results.push(self.publish_finding(finding)?);
        }
        Ok(results)
    }

    /// Return all publication records.
    fn publication_records(&self) -> Vec<PublicationRecordPy> {
        self.records.read().map(|r| r.clone()).unwrap_or_default()
    }

    /// Return the retry policy.
    fn retry_policy(&self) -> RetryPolicyPy {
        self.retry_policy.clone()
    }

    /// Return the publication policy.
    fn policy(&self) -> PublicationPolicyPy {
        self.policy.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("integration_type", self.integration_type.as_str())?;
        dict.set_item("name", &self.name)?;
        dict.set_item("config", &self.config)?;
        dict.set_item("policy", self.policy.to_dict(py)?)?;
        dict.set_item("retry_policy", self.retry_policy.to_dict(py)?)?;
        let records_list = PyList::empty_bound(py);
        if let Ok(records) = self.records.read() {
            for r in records.iter() {
                records_list.append(r.to_dict(py)?)?;
            }
        }
        dict.set_item("records", &records_list)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        let records = self.records.read().map(|r| r.clone()).unwrap_or_default();
        let snapshot = serde_json::json!({
            "integration_type": self.integration_type.as_str(),
            "name": self.name,
            "config": self.config,
            "policy": self.policy,
            "retry_policy": self.retry_policy,
            "records": records,
        });
        serde_json::to_string(&snapshot).unwrap_or_else(|_| "{}".to_string())
    }

    fn __repr__(&self) -> String {
        format!(
            "ExternalIntegration(type={}, name={}, dry_run={})",
            self.integration_type.as_str(),
            self.name,
            self.policy.dry_run
        )
    }
}
