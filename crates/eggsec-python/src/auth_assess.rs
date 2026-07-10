use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use crate::finding::Severity;
use crate::runtime_async;
use crate::runtime_sync;

/// Authentication test type.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuthTestTypePy {
    BruteForce,
    CredentialStuffing,
    AccountLockout,
    RateLimitBypass,
    MfaBypass,
    SessionFixation,
    TimingAttack,
    PasswordPolicy,
}

#[pymethods]
impl AuthTestTypePy {
    fn __repr__(&self) -> String {
        format!("AuthTestType.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl AuthTestTypePy {
    fn as_str(&self) -> &str {
        match self {
            AuthTestTypePy::BruteForce => "BruteForce",
            AuthTestTypePy::CredentialStuffing => "CredentialStuffing",
            AuthTestTypePy::AccountLockout => "AccountLockout",
            AuthTestTypePy::RateLimitBypass => "RateLimitBypass",
            AuthTestTypePy::MfaBypass => "MfaBypass",
            AuthTestTypePy::SessionFixation => "SessionFixation",
            AuthTestTypePy::TimingAttack => "TimingAttack",
            AuthTestTypePy::PasswordPolicy => "PasswordPolicy",
        }
    }

    fn from_engine(engine: eggsec::auth::AuthTestType) -> Self {
        match engine {
            eggsec::auth::AuthTestType::BruteForce => AuthTestTypePy::BruteForce,
            eggsec::auth::AuthTestType::CredentialStuffing => AuthTestTypePy::CredentialStuffing,
            eggsec::auth::AuthTestType::AccountLockout => AuthTestTypePy::AccountLockout,
            eggsec::auth::AuthTestType::RateLimitBypass => AuthTestTypePy::RateLimitBypass,
            eggsec::auth::AuthTestType::MfaBypass => AuthTestTypePy::MfaBypass,
            eggsec::auth::AuthTestType::SessionFixation => AuthTestTypePy::SessionFixation,
            eggsec::auth::AuthTestType::TimingAttack => AuthTestTypePy::TimingAttack,
            eggsec::auth::AuthTestType::PasswordPolicy => AuthTestTypePy::PasswordPolicy,
        }
    }
}

/// A single authentication security finding.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthFindingPy {
    #[pyo3(get)]
    pub test_type: AuthTestTypePy,
    #[pyo3(get)]
    pub severity: Severity,
    #[pyo3(get)]
    pub title: String,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub recommendation: String,
}

impl AuthFindingPy {
    fn from_engine(engine: eggsec::auth::AuthFinding) -> Self {
        Self {
            test_type: AuthTestTypePy::from_engine(engine.test_type),
            severity: Severity::from_engine(engine.severity),
            title: engine.title,
            description: engine.description,
            recommendation: engine.recommendation,
        }
    }
}

#[pymethods]
impl AuthFindingPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("test_type", self.test_type.as_str())?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("title", &self.title)?;
        dict.set_item("description", &self.description)?;
        dict.set_item("recommendation", &self.recommendation)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "AuthFinding(type={}, severity={}, title={})",
            self.test_type.as_str(),
            self.severity.as_str(),
            self.title
        )
    }

    fn __str__(&self) -> String {
        format!(
            "[{}] {} - {}",
            self.severity.as_str(),
            self.test_type.as_str(),
            self.title
        )
    }
}

/// Configuration for authentication security testing.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct AuthTestConfigPy {
    #[pyo3(get)]
    pub max_attempts: usize,
    #[pyo3(get)]
    pub concurrency: usize,
    #[pyo3(get)]
    pub timeout_secs: u64,
    #[pyo3(get)]
    pub stop_on_lockout: bool,
    usernames: Option<Vec<String>>,
    passwords: Option<Vec<String>>,
}

#[pymethods]
impl AuthTestConfigPy {
    /// Create a new authentication test configuration.
    ///
    /// Args:
    ///     max_attempts: Maximum number of login attempts (default: 100).
    ///     concurrency: Number of concurrent requests (default: 10).
    ///     timeout_secs: Timeout per request in seconds (default: 30).
    ///     stop_on_lockout: Stop testing when account lockout is detected (default: true).
    ///     usernames: List of usernames to test with.
    ///     passwords: List of passwords to test with.
    #[new]
    #[pyo3(signature = (max_attempts=100, concurrency=10, timeout_secs=30, stop_on_lockout=true, usernames=None, passwords=None))]
    fn new(
        max_attempts: usize,
        concurrency: usize,
        timeout_secs: u64,
        stop_on_lockout: bool,
        usernames: Option<Vec<String>>,
        passwords: Option<Vec<String>>,
    ) -> PyResult<Self> {
        Ok(Self {
            max_attempts,
            concurrency,
            timeout_secs,
            stop_on_lockout,
            usernames,
            passwords,
        })
    }

    #[getter]
    fn usernames(&self) -> Option<Vec<String>> {
        self.usernames.clone()
    }

    #[getter]
    fn passwords(&self) -> Option<Vec<String>> {
        self.passwords.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "AuthTestConfig(max_attempts={}, concurrency={}, timeout_secs={})",
            self.max_attempts, self.concurrency, self.timeout_secs
        )
    }
}

/// Complete authentication test report.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTestReportPy {
    #[pyo3(get)]
    pub target: String,
    tests_run: Vec<AuthTestTypePy>,
    #[pyo3(get)]
    pub total_attempts: usize,
    findings: Vec<AuthFindingPy>,
    #[pyo3(get)]
    pub has_brute_force: bool,
    #[pyo3(get)]
    pub has_credential_stuffing: bool,
    #[pyo3(get)]
    pub has_lockout_detection: bool,
    #[pyo3(get)]
    pub has_rate_limit: bool,
    #[pyo3(get)]
    pub has_mfa: bool,
    #[pyo3(get)]
    pub has_session: bool,
    #[pyo3(get)]
    pub has_timing: bool,
    #[pyo3(get)]
    pub has_password_policy: bool,
}

impl AuthTestReportPy {
    fn from_engine(engine: eggsec::auth::AuthTestReport) -> Self {
        Self {
            target: engine.target,
            tests_run: engine
                .tests_run
                .into_iter()
                .map(AuthTestTypePy::from_engine)
                .collect(),
            total_attempts: engine.total_attempts,
            findings: engine
                .findings
                .into_iter()
                .map(AuthFindingPy::from_engine)
                .collect(),
            has_brute_force: engine.brute_force.is_some(),
            has_credential_stuffing: engine.credential_stuffing.is_some(),
            has_lockout_detection: engine.lockout_detection.is_some(),
            has_rate_limit: engine.rate_limit.is_some(),
            has_mfa: engine.mfa.is_some(),
            has_session: engine.session.is_some(),
            has_timing: engine.timing.is_some(),
            has_password_policy: engine.password_policy.is_some(),
        }
    }
}

#[pymethods]
impl AuthTestReportPy {
    #[getter]
    fn tests_run(&self) -> Vec<AuthTestTypePy> {
        self.tests_run.clone()
    }

    #[getter]
    fn findings(&self) -> Vec<AuthFindingPy> {
        self.findings.clone()
    }

    #[getter]
    fn finding_count(&self) -> usize {
        self.findings.len()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("total_attempts", self.total_attempts)?;
        dict.set_item("finding_count", self.findings.len())?;

        let tests_list = PyList::empty_bound(py);
        for t in &self.tests_run {
            tests_list.append(t.as_str())?;
        }
        dict.set_item("tests_run", tests_list)?;

        let findings_list = PyList::empty_bound(py);
        for f in &self.findings {
            findings_list.append(f.to_dict(py)?)?;
        }
        dict.set_item("findings", findings_list)?;

        dict.set_item("has_brute_force", self.has_brute_force)?;
        dict.set_item("has_credential_stuffing", self.has_credential_stuffing)?;
        dict.set_item("has_lockout_detection", self.has_lockout_detection)?;
        dict.set_item("has_rate_limit", self.has_rate_limit)?;
        dict.set_item("has_mfa", self.has_mfa)?;
        dict.set_item("has_session", self.has_session)?;
        dict.set_item("has_timing", self.has_timing)?;
        dict.set_item("has_password_policy", self.has_password_policy)?;

        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "AuthTestReport(target={}, attempts={}, findings={})",
            self.target,
            self.total_attempts,
            self.findings.len()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Auth test report for {}: {} attempts, {} findings",
            self.target,
            self.total_attempts,
            self.findings.len()
        )
    }
}

/// Run a comprehensive authentication security test suite.
///
/// Tests brute force, credential stuffing, lockout detection, rate limiting,
/// MFA bypass, session security, timing attacks, and password policy.
///
/// Args:
///     target: Target URL (e.g. "https://example.com/login").
///     config: Test configuration (optional).
///
/// Returns:
///     AuthTestReportPy: Full test report with findings.
///
/// Raises:
///     NetworkError: If the target is unreachable.
///     ConfigError: If the configuration is invalid.
#[pyfunction]
#[pyo3(signature = (target, config=None))]
pub fn auth_test(target: &str, config: Option<AuthTestConfigPy>) -> PyResult<AuthTestReportPy> {
    let cfg = config.unwrap_or_else(|| AuthTestConfigPy {
        max_attempts: 100,
        concurrency: 10,
        timeout_secs: 30,
        stop_on_lockout: true,
        usernames: None,
        passwords: None,
    });

    let target_owned = target.to_string();

    Python::with_gil(|py| {
        let result = runtime_sync::block_on(py, async move {
            let mut engine = eggsec::auth::AuthEngine::new(
                cfg.max_attempts,
                cfg.concurrency,
                cfg.timeout_secs,
                cfg.stop_on_lockout,
            )
            .map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Auth engine error: {}", e))
            })?;

            if let (Some(usernames), Some(passwords)) = (&cfg.usernames, &cfg.passwords) {
                engine.load_wordlists(usernames.clone(), passwords.clone());
            }

            engine.run_full_test(&target_owned).await.map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Auth test failed: {}", e))
            })
        })?;

        Ok(AuthTestReportPy::from_engine(result))
    })
}

/// Run an authentication security test suite (async).
///
/// Returns a PyFuture that can be awaited in Python.
#[pyfunction]
#[pyo3(signature = (target, config=None))]
pub fn async_auth_test(
    target: &str,
    config: Option<AuthTestConfigPy>,
) -> PyResult<crate::runtime_async::PyFuture> {
    let cfg = config.unwrap_or_else(|| AuthTestConfigPy {
        max_attempts: 100,
        concurrency: 10,
        timeout_secs: 30,
        stop_on_lockout: true,
        usernames: None,
        passwords: None,
    });

    let target_owned = target.to_string();

    crate::runtime_async::spawn_async(async move {
        let mut engine = eggsec::auth::AuthEngine::new(
            cfg.max_attempts,
            cfg.concurrency,
            cfg.timeout_secs,
            cfg.stop_on_lockout,
        )
        .map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Auth engine error: {}", e))
        })?;

        if let (Some(usernames), Some(passwords)) = (&cfg.usernames, &cfg.passwords) {
            engine.load_wordlists(usernames.clone(), passwords.clone());
        }

        let report = engine.run_full_test(&target_owned).await.map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Auth test failed: {}", e))
        })?;

        Ok(AuthTestReportPy::from_engine(report))
    })
}
