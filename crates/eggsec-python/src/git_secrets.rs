use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use crate::error::EggsecResultExt;
use crate::finding::Severity;
use crate::runtime_async;
use crate::runtime_sync;

/// Confidence level for a secret finding.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[pymethods]
impl Confidence {
    fn __repr__(&self) -> String {
        format!("Confidence.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl Confidence {
    fn as_str(&self) -> &str {
        match self {
            Confidence::High => "High",
            Confidence::Medium => "Medium",
            Confidence::Low => "Low",
        }
    }

    pub fn from_engine(engine: eggsec::recon::secrets::Confidence) -> Self {
        match engine {
            eggsec::recon::secrets::Confidence::High => Confidence::High,
            eggsec::recon::secrets::Confidence::Medium => Confidence::Medium,
            eggsec::recon::secrets::Confidence::Low => Confidence::Low,
        }
    }
}

/// Type of secret detected.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecretType {
    AwsAccessKey,
    AwsSecretKey,
    AwsSessionToken,
    AzureKey,
    GcpApiKey,
    GcpServiceAccount,
    GithubToken,
    GitlabToken,
    BitbucketToken,
    SlackToken,
    DiscordToken,
    SlackWebhook,
    GenericApiKey,
    PrivateKey,
    JwtToken,
    BasicAuth,
    BearerToken,
    OpenAiKey,
    StripeKey,
    TwilioKey,
    SendGridKey,
    MailchimpKey,
    PasswordInUrl,
    DatabaseConnectionString,
    NpmToken,
    PyPiToken,
    HerokuKey,
    NetlifyToken,
    DockerhubToken,
    KubernetesSecret,
}

#[pymethods]
impl SecretType {
    fn __repr__(&self) -> String {
        format!("SecretType.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl SecretType {
    fn as_str(&self) -> &str {
        match self {
            SecretType::AwsAccessKey => "AwsAccessKey",
            SecretType::AwsSecretKey => "AwsSecretKey",
            SecretType::AwsSessionToken => "AwsSessionToken",
            SecretType::AzureKey => "AzureKey",
            SecretType::GcpApiKey => "GcpApiKey",
            SecretType::GcpServiceAccount => "GcpServiceAccount",
            SecretType::GithubToken => "GithubToken",
            SecretType::GitlabToken => "GitlabToken",
            SecretType::BitbucketToken => "BitbucketToken",
            SecretType::SlackToken => "SlackToken",
            SecretType::DiscordToken => "DiscordToken",
            SecretType::SlackWebhook => "SlackWebhook",
            SecretType::GenericApiKey => "GenericApiKey",
            SecretType::PrivateKey => "PrivateKey",
            SecretType::JwtToken => "JwtToken",
            SecretType::BasicAuth => "BasicAuth",
            SecretType::BearerToken => "BearerToken",
            SecretType::OpenAiKey => "OpenAiKey",
            SecretType::StripeKey => "StripeKey",
            SecretType::TwilioKey => "TwilioKey",
            SecretType::SendGridKey => "SendGridKey",
            SecretType::MailchimpKey => "MailchimpKey",
            SecretType::PasswordInUrl => "PasswordInUrl",
            SecretType::DatabaseConnectionString => "DatabaseConnectionString",
            SecretType::NpmToken => "NpmToken",
            SecretType::PyPiToken => "PyPiToken",
            SecretType::HerokuKey => "HerokuKey",
            SecretType::NetlifyToken => "NetlifyToken",
            SecretType::DockerhubToken => "DockerhubToken",
            SecretType::KubernetesSecret => "KubernetesSecret",
        }
    }

    pub fn from_engine(engine: eggsec::recon::secrets::SecretType) -> Self {
        match engine {
            eggsec::recon::secrets::SecretType::AwsAccessKey => SecretType::AwsAccessKey,
            eggsec::recon::secrets::SecretType::AwsSecretKey => SecretType::AwsSecretKey,
            eggsec::recon::secrets::SecretType::AwsSessionToken => SecretType::AwsSessionToken,
            eggsec::recon::secrets::SecretType::AzureKey => SecretType::AzureKey,
            eggsec::recon::secrets::SecretType::GcpApiKey => SecretType::GcpApiKey,
            eggsec::recon::secrets::SecretType::GcpServiceAccount => SecretType::GcpServiceAccount,
            eggsec::recon::secrets::SecretType::GithubToken => SecretType::GithubToken,
            eggsec::recon::secrets::SecretType::GitlabToken => SecretType::GitlabToken,
            eggsec::recon::secrets::SecretType::BitbucketToken => SecretType::BitbucketToken,
            eggsec::recon::secrets::SecretType::SlackToken => SecretType::SlackToken,
            eggsec::recon::secrets::SecretType::DiscordToken => SecretType::DiscordToken,
            eggsec::recon::secrets::SecretType::SlackWebhook => SecretType::SlackWebhook,
            eggsec::recon::secrets::SecretType::GenericApiKey => SecretType::GenericApiKey,
            eggsec::recon::secrets::SecretType::PrivateKey => SecretType::PrivateKey,
            eggsec::recon::secrets::SecretType::JwtToken => SecretType::JwtToken,
            eggsec::recon::secrets::SecretType::BasicAuth => SecretType::BasicAuth,
            eggsec::recon::secrets::SecretType::BearerToken => SecretType::BearerToken,
            eggsec::recon::secrets::SecretType::OpenAiKey => SecretType::OpenAiKey,
            eggsec::recon::secrets::SecretType::StripeKey => SecretType::StripeKey,
            eggsec::recon::secrets::SecretType::TwilioKey => SecretType::TwilioKey,
            eggsec::recon::secrets::SecretType::SendGridKey => SecretType::SendGridKey,
            eggsec::recon::secrets::SecretType::MailchimpKey => SecretType::MailchimpKey,
            eggsec::recon::secrets::SecretType::PasswordInUrl => SecretType::PasswordInUrl,
            eggsec::recon::secrets::SecretType::DatabaseConnectionString => {
                SecretType::DatabaseConnectionString
            }
            eggsec::recon::secrets::SecretType::NpmToken => SecretType::NpmToken,
            eggsec::recon::secrets::SecretType::PyPiToken => SecretType::PyPiToken,
            eggsec::recon::secrets::SecretType::HerokuKey => SecretType::HerokuKey,
            eggsec::recon::secrets::SecretType::NetlifyToken => SecretType::NetlifyToken,
            eggsec::recon::secrets::SecretType::DockerhubToken => SecretType::DockerhubToken,
            eggsec::recon::secrets::SecretType::KubernetesSecret => SecretType::KubernetesSecret,
        }
    }
}

/// A single secret finding within a commit.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretFindingPy {
    #[pyo3(get)]
    pub secret_type: SecretType,
    #[pyo3(get)]
    pub value_preview: String,
    #[pyo3(get)]
    pub location: String,
    #[pyo3(get)]
    pub confidence: Confidence,
    #[pyo3(get)]
    pub severity: Severity,
    #[pyo3(get)]
    pub description: String,
}

impl SecretFindingPy {
    pub fn from_engine(engine: eggsec::recon::secrets::SecretFinding) -> Self {
        Self {
            secret_type: SecretType::from_engine(engine.secret_type),
            value_preview: engine.value_preview,
            location: engine.location,
            confidence: Confidence::from_engine(engine.confidence),
            severity: Severity::from_engine(engine.severity),
            description: engine.description,
        }
    }
}

#[pymethods]
impl SecretFindingPy {
    /// Convert to a Python dictionary.
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("secret_type", self.secret_type.as_str())?;
        dict.set_item("value_preview", &self.value_preview)?;
        dict.set_item("location", &self.location)?;
        dict.set_item("confidence", self.confidence.as_str())?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("description", &self.description)?;
        Ok(dict.into())
    }

    /// Convert to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "SecretFinding(type={}, confidence={}, severity={})",
            self.secret_type.as_str(),
            self.confidence.as_str(),
            self.severity.as_str()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "[{}] {} (confidence: {}, severity: {})",
            self.secret_type.as_str(),
            self.location,
            self.confidence.as_str(),
            self.severity.as_str()
        )
    }
}

/// A finding from git history analysis.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitSecretFindingPy {
    #[pyo3(get)]
    pub commit_hash: String,
    #[pyo3(get)]
    pub commit_message: String,
    #[pyo3(get)]
    pub author: String,
    #[pyo3(get)]
    pub date: String,
    #[pyo3(get)]
    pub file_path: String,
    #[pyo3(get)]
    pub secret_type: SecretType,
    #[pyo3(get)]
    pub value_preview: String,
    #[pyo3(get)]
    pub location: String,
    #[pyo3(get)]
    pub confidence: Confidence,
    #[pyo3(get)]
    pub severity: Severity,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub introduced_in: bool,
}

impl GitSecretFindingPy {
    pub fn from_engine(engine: eggsec::recon::git_secrets::GitSecretFinding) -> Self {
        Self {
            commit_hash: engine.commit_hash,
            commit_message: engine.commit_message,
            author: engine.author,
            date: engine.date,
            file_path: engine.file_path,
            secret_type: SecretType::from_engine(engine.secret.secret_type),
            value_preview: engine.secret.value_preview,
            location: engine.secret.location,
            confidence: Confidence::from_engine(engine.secret.confidence),
            severity: Severity::from_engine(engine.secret.severity),
            description: engine.secret.description,
            introduced_in: engine.introduced_in,
        }
    }
}

#[pymethods]
impl GitSecretFindingPy {
    /// Get the flattened secret detail as a SecretFindingPy.
    #[getter]
    fn secret(&self) -> SecretFindingPy {
        SecretFindingPy {
            secret_type: self.secret_type,
            value_preview: self.value_preview.clone(),
            location: self.location.clone(),
            confidence: self.confidence,
            severity: self.severity,
            description: self.description.clone(),
        }
    }

    /// Convert to a Python dictionary.
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("commit_hash", &self.commit_hash)?;
        dict.set_item("commit_message", &self.commit_message)?;
        dict.set_item("author", &self.author)?;
        dict.set_item("date", &self.date)?;
        dict.set_item("file_path", &self.file_path)?;
        dict.set_item("secret_type", self.secret_type.as_str())?;
        dict.set_item("value_preview", &self.value_preview)?;
        dict.set_item("location", &self.location)?;
        dict.set_item("confidence", self.confidence.as_str())?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("description", &self.description)?;
        dict.set_item("introduced_in", self.introduced_in)?;
        Ok(dict.into())
    }

    /// Convert to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "GitSecretFinding(commit={}, file={}, type={})",
            &self.commit_hash[..8.min(self.commit_hash.len())],
            self.file_path,
            self.secret_type.as_str()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "[{}] {} in {} @ {}",
            self.secret_type.as_str(),
            self.file_path,
            &self.commit_hash[..8.min(self.commit_hash.len())],
            self.date
        )
    }
}

/// Severity summary for a git secrets scan.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitSecretsSummaryPy {
    #[pyo3(get)]
    pub critical: usize,
    #[pyo3(get)]
    pub high: usize,
    #[pyo3(get)]
    pub medium: usize,
    #[pyo3(get)]
    pub low: usize,
    #[pyo3(get)]
    pub info: usize,
}

impl GitSecretsSummaryPy {
    pub fn from_engine(engine: eggsec::recon::git_secrets::GitSecretsSummary) -> Self {
        Self {
            critical: engine.critical,
            high: engine.high,
            medium: engine.medium,
            low: engine.low,
            info: engine.info,
        }
    }
}

#[pymethods]
impl GitSecretsSummaryPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("critical", self.critical)?;
        dict.set_item("high", self.high)?;
        dict.set_item("medium", self.medium)?;
        dict.set_item("low", self.low)?;
        dict.set_item("info", self.info)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "GitSecretsSummary(critical={}, high={}, medium={}, low={}, info={})",
            self.critical, self.high, self.medium, self.low, self.info
        )
    }

    fn __str__(&self) -> String {
        format!(
            "critical={} high={} medium={} low={} info={}",
            self.critical, self.high, self.medium, self.low, self.info
        )
    }
}

/// Full report from a git secrets scan.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitSecretsReportPy {
    #[pyo3(get)]
    pub repo_path: String,
    #[pyo3(get)]
    pub commits_scanned: usize,
    #[pyo3(get)]
    pub files_scanned: usize,
    findings: Vec<GitSecretFindingPy>,
    #[pyo3(get)]
    pub summary: GitSecretsSummaryPy,
}

#[pymethods]
impl GitSecretsReportPy {
    #[getter]
    fn findings(&self) -> Vec<GitSecretFindingPy> {
        self.findings.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("repo_path", &self.repo_path)?;
        dict.set_item("commits_scanned", self.commits_scanned)?;
        dict.set_item("files_scanned", self.files_scanned)?;

        let findings_list = PyList::empty_bound(py);
        for f in &self.findings {
            findings_list.append(f.to_dict(py)?)?;
        }
        dict.set_item("findings", findings_list)?;
        dict.set_item("summary", self.summary.to_dict(py)?)?;
        Ok(dict.into())
    }

    /// Convert to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "GitSecretsReport(repo={}, commits={}, files={}, findings={})",
            self.repo_path,
            self.commits_scanned,
            self.files_scanned,
            self.findings.len()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Git secrets scan of '{}': {} commits, {} files, {} findings",
            self.repo_path,
            self.commits_scanned,
            self.files_scanned,
            self.findings.len()
        )
    }
}

/// Scan a git repository for leaked secrets in commit history.
///
/// Args:
///     repo_path: Path to the git repository directory.
///     max_commits: Maximum number of commits to scan (0 = all).
///
/// Returns:
///     GitSecretsReportPy: Full report with findings and severity summary.
///
/// Raises:
///     ScanError: If the scan fails.
#[pyfunction]
#[pyo3(signature = (repo_path, *, max_commits=1000))]
pub fn scan_git_secrets(repo_path: &str, max_commits: usize) -> PyResult<GitSecretsReportPy> {
    let repo_path_owned = repo_path.to_string();
    Python::with_gil(|py| {
        let result = runtime_sync::block_on(py, async move {
            eggsec::recon::git_secrets::scan_git_secrets(&repo_path_owned, max_commits).map_pyerr()
        })?;

        Ok(GitSecretsReportPy {
            repo_path: result.repo_path,
            commits_scanned: result.commits_scanned,
            files_scanned: result.files_scanned,
            findings: result
                .findings
                .into_iter()
                .map(GitSecretFindingPy::from_engine)
                .collect(),
            summary: GitSecretsSummaryPy::from_engine(result.summary),
        })
    })
}

/// Perform async git secrets scan.
#[pyfunction]
#[pyo3(signature = (repo_path, *, max_commits=1000))]
pub fn async_scan_git_secrets(
    repo_path: &str,
    max_commits: usize,
) -> PyResult<crate::runtime_async::PyFuture> {
    let repo_path_owned = repo_path.to_string();

    runtime_async::spawn_async(async move {
        let result = eggsec::recon::git_secrets::scan_git_secrets(&repo_path_owned, max_commits)
            .map_pyerr()?;

        Ok(GitSecretsReportPy {
            repo_path: result.repo_path,
            commits_scanned: result.commits_scanned,
            files_scanned: result.files_scanned,
            findings: result
                .findings
                .into_iter()
                .map(GitSecretFindingPy::from_engine)
                .collect(),
            summary: GitSecretsSummaryPy::from_engine(result.summary),
        })
    })
}
