use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use crate::finding::Severity;
use crate::runtime_sync;

/// OAuth vulnerability type.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OAuthVulnerabilityPy {
    RedirectUriValidation,
    StateParameterMissing,
    ScopeEscalation,
    GrantTypeMixing,
    PKCEBypass,
    TokenLeakage,
}

#[pymethods]
impl OAuthVulnerabilityPy {
    fn __repr__(&self) -> String {
        format!("OAuthVulnerability.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl OAuthVulnerabilityPy {
    fn as_str(&self) -> &str {
        match self {
            OAuthVulnerabilityPy::RedirectUriValidation => "RedirectUriValidation",
            OAuthVulnerabilityPy::StateParameterMissing => "StateParameterMissing",
            OAuthVulnerabilityPy::ScopeEscalation => "ScopeEscalation",
            OAuthVulnerabilityPy::GrantTypeMixing => "GrantTypeMixing",
            OAuthVulnerabilityPy::PKCEBypass => "PKCEBypass",
            OAuthVulnerabilityPy::TokenLeakage => "TokenLeakage",
        }
    }

    fn from_engine(engine: eggsec::fuzzer::payloads::oauth::OAuthVulnerability) -> Self {
        match engine {
            eggsec::fuzzer::payloads::oauth::OAuthVulnerability::RedirectUriValidation => {
                OAuthVulnerabilityPy::RedirectUriValidation
            }
            eggsec::fuzzer::payloads::oauth::OAuthVulnerability::StateParameterMissing => {
                OAuthVulnerabilityPy::StateParameterMissing
            }
            eggsec::fuzzer::payloads::oauth::OAuthVulnerability::ScopeEscalation => {
                OAuthVulnerabilityPy::ScopeEscalation
            }
            eggsec::fuzzer::payloads::oauth::OAuthVulnerability::GrantTypeMixing => {
                OAuthVulnerabilityPy::GrantTypeMixing
            }
            eggsec::fuzzer::payloads::oauth::OAuthVulnerability::PKCEBypass => {
                OAuthVulnerabilityPy::PKCEBypass
            }
            eggsec::fuzzer::payloads::oauth::OAuthVulnerability::TokenLeakage => {
                OAuthVulnerabilityPy::TokenLeakage
            }
        }
    }
}

/// OAuth endpoint kind.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OAuthEndpointKindPy {
    OidcDiscovery,
    OAuthDiscovery,
    Authorize,
    Token,
    UserInfo,
    Jwks,
    Revoke,
}

#[pymethods]
impl OAuthEndpointKindPy {
    fn __repr__(&self) -> String {
        format!("OAuthEndpointKind.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl OAuthEndpointKindPy {
    fn as_str(&self) -> &str {
        match self {
            OAuthEndpointKindPy::OidcDiscovery => "OidcDiscovery",
            OAuthEndpointKindPy::OAuthDiscovery => "OAuthDiscovery",
            OAuthEndpointKindPy::Authorize => "Authorize",
            OAuthEndpointKindPy::Token => "Token",
            OAuthEndpointKindPy::UserInfo => "UserInfo",
            OAuthEndpointKindPy::Jwks => "Jwks",
            OAuthEndpointKindPy::Revoke => "Revoke",
        }
    }

    fn from_engine(engine: eggsec::fuzzer::payloads::oauth::EndpointKind) -> Self {
        match engine {
            eggsec::fuzzer::payloads::oauth::EndpointKind::OidcDiscovery => {
                OAuthEndpointKindPy::OidcDiscovery
            }
            eggsec::fuzzer::payloads::oauth::EndpointKind::OAuthDiscovery => {
                OAuthEndpointKindPy::OAuthDiscovery
            }
            eggsec::fuzzer::payloads::oauth::EndpointKind::Authorize => {
                OAuthEndpointKindPy::Authorize
            }
            eggsec::fuzzer::payloads::oauth::EndpointKind::Token => OAuthEndpointKindPy::Token,
            eggsec::fuzzer::payloads::oauth::EndpointKind::UserInfo => {
                OAuthEndpointKindPy::UserInfo
            }
            eggsec::fuzzer::payloads::oauth::EndpointKind::Jwks => OAuthEndpointKindPy::Jwks,
            eggsec::fuzzer::payloads::oauth::EndpointKind::Revoke => OAuthEndpointKindPy::Revoke,
        }
    }
}

/// An OAuth/OIDC endpoint.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthEndpointPy {
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub kind: OAuthEndpointKindPy,
}

impl OAuthEndpointPy {
    fn from_engine(engine: eggsec::fuzzer::payloads::oauth::OAuthEndpoint) -> Self {
        Self {
            url: engine.url,
            kind: OAuthEndpointKindPy::from_engine(engine.kind),
        }
    }
}

#[pymethods]
impl OAuthEndpointPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("url", &self.url)?;
        dict.set_item("kind", self.kind.as_str())?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "OAuthEndpoint(url={}, kind={})",
            self.url,
            self.kind.as_str()
        )
    }

    fn __str__(&self) -> String {
        format!("{} ({})", self.url, self.kind.as_str())
    }
}

/// A single OAuth/OIDC security test result.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTestResultPy {
    #[pyo3(get)]
    pub vulnerability: OAuthVulnerabilityPy,
    #[pyo3(get)]
    pub success: bool,
    #[pyo3(get)]
    pub endpoint: String,
    #[pyo3(get)]
    pub proof: String,
    #[pyo3(get)]
    pub severity: Severity,
    #[pyo3(get)]
    pub description: String,
}

impl OAuthTestResultPy {
    fn from_engine(engine: eggsec::fuzzer::payloads::oauth::OAuthTestResult) -> Self {
        Self {
            vulnerability: OAuthVulnerabilityPy::from_engine(engine.vulnerability),
            success: engine.success,
            endpoint: engine.endpoint,
            proof: engine.proof,
            severity: Severity::from_engine(engine.severity),
            description: engine.description,
        }
    }
}

#[pymethods]
impl OAuthTestResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("vulnerability", self.vulnerability.as_str())?;
        dict.set_item("success", self.success)?;
        dict.set_item("endpoint", &self.endpoint)?;
        dict.set_item("proof", &self.proof)?;
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
            "OAuthTestResult(vuln={}, success={})",
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

/// Configuration for OAuth/OIDC security testing.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct OAuthTestConfigPy {
    #[pyo3(get)]
    pub client_id: String,
    #[pyo3(get)]
    pub redirect_uri: String,
    #[pyo3(get)]
    pub client_secret: Option<String>,
    #[pyo3(get)]
    pub issuer_url: Option<String>,
    #[pyo3(get)]
    pub enable_redirect_test: bool,
    #[pyo3(get)]
    pub enable_scope_test: bool,
    #[pyo3(get)]
    pub enable_state_test: bool,
    #[pyo3(get)]
    pub enable_grant_test: bool,
    #[pyo3(get)]
    pub timeout_secs: u64,
}

#[pymethods]
impl OAuthTestConfigPy {
    /// Create a new OAuth test configuration.
    ///
    /// Args:
    ///     client_id: OAuth client ID.
    ///     redirect_uri: Registered redirect URI.
    ///     client_secret: OAuth client secret (optional).
    ///     issuer_url: OIDC issuer URL for discovery (optional).
    ///     enable_redirect_test: Test redirect_uri validation (default: true).
    ///     enable_scope_test: Test scope escalation (default: true).
    ///     enable_state_test: Test state parameter (default: true).
    ///     enable_grant_test: Test grant type mixing (default: true).
    ///     timeout_secs: Request timeout in seconds (default: 10).
    #[new]
    #[pyo3(signature = (client_id, redirect_uri, client_secret=None, issuer_url=None, *, enable_redirect_test=true, enable_scope_test=true, enable_state_test=true, enable_grant_test=true, timeout_secs=10))]
    fn new(
        client_id: String,
        redirect_uri: String,
        client_secret: Option<String>,
        issuer_url: Option<String>,
        enable_redirect_test: bool,
        enable_scope_test: bool,
        enable_state_test: bool,
        enable_grant_test: bool,
        timeout_secs: u64,
    ) -> PyResult<Self> {
        if client_id.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "client_id must not be empty",
            ));
        }
        Ok(Self {
            client_id,
            redirect_uri,
            client_secret,
            issuer_url,
            enable_redirect_test,
            enable_scope_test,
            enable_state_test,
            enable_grant_test,
            timeout_secs,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "OAuthTestConfig(client_id={}, redirect_uri={})",
            self.client_id, self.redirect_uri
        )
    }
}

/// Discover OAuth/OIDC endpoints from an issuer URL.
///
/// Args:
///     issuer: Issuer URL (e.g. "https://accounts.example.com").
///
/// Returns:
///     list[OAuthEndpointPy]: List of discovered endpoints.
#[pyfunction]
pub fn oauth_discover_endpoints(issuer: &str) -> Vec<OAuthEndpointPy> {
    let fuzzer = eggsec::fuzzer::payloads::oauth::OAuthFuzzer::new(
        "dummy".to_string(),
        "https://dummy/callback".to_string(),
    );
    fuzzer
        .discover_endpoints(issuer)
        .into_iter()
        .map(OAuthEndpointPy::from_engine)
        .collect()
}

/// Run OAuth/OIDC security tests against an authorization endpoint.
///
/// Tests redirect_uri validation, state parameter, scope escalation,
/// grant type mixing, and PKCE bypass.
///
/// Args:
///     config: OAuth test configuration.
///     auth_endpoint: Authorization endpoint URL.
///
/// Returns:
///     list[OAuthTestResultPy]: List of test results.
///
/// Raises:
///     NetworkError: If the endpoint is unreachable.
///     ConfigError: If the configuration is invalid.
#[pyfunction]
pub fn oauth_test(
    config: OAuthTestConfigPy,
    auth_endpoint: &str,
) -> PyResult<Vec<OAuthTestResultPy>> {
    let fuzzer = build_oauth_fuzzer(&config)?;

    let mut results = Vec::new();

    if config.enable_redirect_test {
        results.extend(fuzzer.test_redirect_uri(auth_endpoint));
    }

    if config.enable_state_test {
        results.extend(fuzzer.test_state_parameter(auth_endpoint));
    }

    if config.enable_scope_test {
        results.extend(fuzzer.test_scope_escalation(auth_endpoint));
    }

    Ok(results
        .into_iter()
        .map(OAuthTestResultPy::from_engine)
        .collect())
}

/// Run OAuth/OIDC security tests (async).
///
/// Returns a PyFuture that resolves to a list of test results.
#[pyfunction]
pub fn async_oauth_test(
    config: OAuthTestConfigPy,
    auth_endpoint: &str,
) -> PyResult<crate::runtime_async::PyFuture> {
    let auth_endpoint_owned = auth_endpoint.to_string();

    crate::runtime_async::spawn_async(async move {
        let fuzzer = build_oauth_fuzzer_async(&config)?;

        let mut results = Vec::new();

        if config.enable_redirect_test {
            results.extend(fuzzer.test_redirect_uri(&auth_endpoint_owned));
        }

        if config.enable_state_test {
            results.extend(fuzzer.test_state_parameter(&auth_endpoint_owned));
        }

        if config.enable_scope_test {
            results.extend(fuzzer.test_scope_escalation(&auth_endpoint_owned));
        }

        Ok::<Vec<OAuthTestResultPy>, PyErr>(
            results
                .into_iter()
                .map(OAuthTestResultPy::from_engine)
                .collect(),
        )
    })
}

fn build_oauth_fuzzer(
    config: &OAuthTestConfigPy,
) -> PyResult<eggsec::fuzzer::payloads::oauth::OAuthFuzzer> {
    let mut fuzzer = eggsec::fuzzer::payloads::oauth::OAuthFuzzer::new(
        config.client_id.clone(),
        config.redirect_uri.clone(),
    )
    .with_redirect_test(config.enable_redirect_test)
    .with_scope_test(config.enable_scope_test)
    .with_state_test(config.enable_state_test)
    .with_grant_test(config.enable_grant_test);

    if let Some(ref issuer) = config.issuer_url {
        fuzzer = fuzzer.with_issuer(issuer.clone());
    }

    if let Some(ref secret) = config.client_secret {
        fuzzer = fuzzer.with_client_secret(secret.clone());
    }

    Ok(fuzzer)
}

fn build_oauth_fuzzer_async(
    config: &OAuthTestConfigPy,
) -> PyResult<eggsec::fuzzer::payloads::oauth::OAuthFuzzer> {
    build_oauth_fuzzer(config)
}
