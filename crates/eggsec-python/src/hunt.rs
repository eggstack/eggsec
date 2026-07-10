use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use crate::finding::Severity;
use crate::runtime_sync;

/// Attack chain type.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChainTypePy {
    PrivilegeEscalation,
    DataExfiltration,
    RemoteCodeExecution,
    LateralMovement,
    Persistence,
    DenialOfService,
}

#[pymethods]
impl ChainTypePy {
    fn __repr__(&self) -> String {
        format!("ChainType.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl ChainTypePy {
    fn as_str(&self) -> &str {
        match self {
            ChainTypePy::PrivilegeEscalation => "PrivilegeEscalation",
            ChainTypePy::DataExfiltration => "DataExfiltration",
            ChainTypePy::RemoteCodeExecution => "RemoteCodeExecution",
            ChainTypePy::LateralMovement => "LateralMovement",
            ChainTypePy::Persistence => "Persistence",
            ChainTypePy::DenialOfService => "DenialOfService",
        }
    }

    fn from_engine(engine: eggsec::hunt::chain::ChainType) -> Self {
        match engine {
            eggsec::hunt::chain::ChainType::PrivilegeEscalation => ChainTypePy::PrivilegeEscalation,
            eggsec::hunt::chain::ChainType::DataExfiltration => ChainTypePy::DataExfiltration,
            eggsec::hunt::chain::ChainType::RemoteCodeExecution => ChainTypePy::RemoteCodeExecution,
            eggsec::hunt::chain::ChainType::LateralMovement => ChainTypePy::LateralMovement,
            eggsec::hunt::chain::ChainType::Persistence => ChainTypePy::Persistence,
            eggsec::hunt::chain::ChainType::DenialOfService => ChainTypePy::DenialOfService,
        }
    }
}

/// A step in an attack chain.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStepPy {
    #[pyo3(get)]
    pub step_number: usize,
    #[pyo3(get)]
    pub vulnerability: String,
    #[pyo3(get)]
    pub prerequisite: String,
    #[pyo3(get)]
    pub impact: String,
    #[pyo3(get)]
    pub evidence: String,
    #[pyo3(get)]
    pub severity: Severity,
}

impl ChainStepPy {
    fn from_engine(engine: eggsec::hunt::chain::ChainStep) -> Self {
        Self {
            step_number: engine.step_number,
            vulnerability: engine.vulnerability,
            prerequisite: engine.prerequisite,
            impact: engine.impact,
            evidence: engine.evidence,
            severity: Severity::from_engine(engine.severity),
        }
    }
}

#[pymethods]
impl ChainStepPy {
    fn __repr__(&self) -> String {
        format!(
            "ChainStep(step={}, vuln={})",
            self.step_number, self.vulnerability
        )
    }
}

/// An attack chain combining multiple vulnerabilities.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackChainPy {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub chain_type: ChainTypePy,
    steps: Vec<ChainStepPy>,
    #[pyo3(get)]
    pub severity: Severity,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub remediation: String,
    #[pyo3(get)]
    pub cvss_score: Option<f32>,
}

impl AttackChainPy {
    fn from_engine(engine: eggsec::hunt::chain::AttackChain) -> Self {
        Self {
            id: engine.id,
            name: engine.name,
            chain_type: ChainTypePy::from_engine(engine.chain_type),
            steps: engine
                .steps
                .into_iter()
                .map(ChainStepPy::from_engine)
                .collect(),
            severity: Severity::from_engine(engine.severity),
            description: engine.description,
            remediation: engine.remediation,
            cvss_score: engine.cvss_score,
        }
    }
}

#[pymethods]
impl AttackChainPy {
    #[getter]
    fn steps(&self) -> Vec<ChainStepPy> {
        self.steps.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.id)?;
        dict.set_item("name", &self.name)?;
        dict.set_item("chain_type", self.chain_type.as_str())?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("description", &self.description)?;
        dict.set_item("remediation", &self.remediation)?;
        dict.set_item("cvss_score", self.cvss_score)?;
        dict.set_item("step_count", self.steps.len())?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "AttackChain(id={}, name={}, type={})",
            self.id,
            self.name,
            self.chain_type.as_str()
        )
    }
}

/// Business logic flaw type.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FlawTypePy {
    PriceManipulation,
    PrivilegeEscalation,
    RateLimitBypass,
    CartManipulation,
    CreditOverflow,
    WorkflowBypass,
    InsufficientValidation,
    TrustBoundaryViolation,
    TimeTravel,
    IntegerOverflow,
}

#[pymethods]
impl FlawTypePy {
    fn __repr__(&self) -> String {
        format!("FlawType.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl FlawTypePy {
    fn as_str(&self) -> &str {
        match self {
            FlawTypePy::PriceManipulation => "PriceManipulation",
            FlawTypePy::PrivilegeEscalation => "PrivilegeEscalation",
            FlawTypePy::RateLimitBypass => "RateLimitBypass",
            FlawTypePy::CartManipulation => "CartManipulation",
            FlawTypePy::CreditOverflow => "CreditOverflow",
            FlawTypePy::WorkflowBypass => "WorkflowBypass",
            FlawTypePy::InsufficientValidation => "InsufficientValidation",
            FlawTypePy::TrustBoundaryViolation => "TrustBoundaryViolation",
            FlawTypePy::TimeTravel => "TimeTravel",
            FlawTypePy::IntegerOverflow => "IntegerOverflow",
        }
    }

    fn from_engine(engine: eggsec::hunt::business::FlawType) -> Self {
        match engine {
            eggsec::hunt::business::FlawType::PriceManipulation => FlawTypePy::PriceManipulation,
            eggsec::hunt::business::FlawType::PrivilegeEscalation => {
                FlawTypePy::PrivilegeEscalation
            }
            eggsec::hunt::business::FlawType::RateLimitBypass => FlawTypePy::RateLimitBypass,
            eggsec::hunt::business::FlawType::CartManipulation => FlawTypePy::CartManipulation,
            eggsec::hunt::business::FlawType::CreditOverflow => FlawTypePy::CreditOverflow,
            eggsec::hunt::business::FlawType::WorkflowBypass => FlawTypePy::WorkflowBypass,
            eggsec::hunt::business::FlawType::InsufficientValidation => {
                FlawTypePy::InsufficientValidation
            }
            eggsec::hunt::business::FlawType::TrustBoundaryViolation => {
                FlawTypePy::TrustBoundaryViolation
            }
            eggsec::hunt::business::FlawType::TimeTravel => FlawTypePy::TimeTravel,
            eggsec::hunt::business::FlawType::IntegerOverflow => FlawTypePy::IntegerOverflow,
        }
    }
}

/// A business logic flaw.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessLogicFlawPy {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub flaw_type: FlawTypePy,
    #[pyo3(get)]
    pub severity: Severity,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub location: String,
    #[pyo3(get)]
    pub evidence: String,
    #[pyo3(get)]
    pub remediation: String,
    #[pyo3(get)]
    pub cvss_score: Option<f32>,
}

impl BusinessLogicFlawPy {
    fn from_engine(engine: eggsec::hunt::business::BusinessLogicFlaw) -> Self {
        Self {
            id: engine.id,
            flaw_type: FlawTypePy::from_engine(engine.flaw_type),
            severity: Severity::from_engine(engine.severity),
            description: engine.description,
            location: engine.location,
            evidence: engine.evidence,
            remediation: engine.remediation,
            cvss_score: engine.cvss_score,
        }
    }
}

#[pymethods]
impl BusinessLogicFlawPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.id)?;
        dict.set_item("flaw_type", self.flaw_type.as_str())?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("description", &self.description)?;
        dict.set_item("location", &self.location)?;
        dict.set_item("evidence", &self.evidence)?;
        dict.set_item("remediation", &self.remediation)?;
        dict.set_item("cvss_score", self.cvss_score)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "BusinessLogicFlaw(id={}, type={})",
            self.id,
            self.flaw_type.as_str()
        )
    }
}

/// Race condition type.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RaceTypePy {
    TimeOfCheckTimeOfUse,
    ConcurrentFundsTransfer,
    InventoryOverSale,
    SessionRace,
    CouponReuse,
    CommentRace,
    ResponseInconsistency,
    TimingAnomaly,
}

#[pymethods]
impl RaceTypePy {
    fn __repr__(&self) -> String {
        format!("RaceType.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl RaceTypePy {
    fn as_str(&self) -> &str {
        match self {
            RaceTypePy::TimeOfCheckTimeOfUse => "TimeOfCheckTimeOfUse",
            RaceTypePy::ConcurrentFundsTransfer => "ConcurrentFundsTransfer",
            RaceTypePy::InventoryOverSale => "InventoryOverSale",
            RaceTypePy::SessionRace => "SessionRace",
            RaceTypePy::CouponReuse => "CouponReuse",
            RaceTypePy::CommentRace => "CommentRace",
            RaceTypePy::ResponseInconsistency => "ResponseInconsistency",
            RaceTypePy::TimingAnomaly => "TimingAnomaly",
        }
    }

    fn from_engine(engine: eggsec::hunt::race::RaceType) -> Self {
        match engine {
            eggsec::hunt::race::RaceType::TimeOfCheckTimeOfUse => RaceTypePy::TimeOfCheckTimeOfUse,
            eggsec::hunt::race::RaceType::ConcurrentFundsTransfer => {
                RaceTypePy::ConcurrentFundsTransfer
            }
            eggsec::hunt::race::RaceType::InventoryOverSale => RaceTypePy::InventoryOverSale,
            eggsec::hunt::race::RaceType::SessionRace => RaceTypePy::SessionRace,
            eggsec::hunt::race::RaceType::CouponReuse => RaceTypePy::CouponReuse,
            eggsec::hunt::race::RaceType::CommentRace => RaceTypePy::CommentRace,
            eggsec::hunt::race::RaceType::ResponseInconsistency => {
                RaceTypePy::ResponseInconsistency
            }
            eggsec::hunt::race::RaceType::TimingAnomaly => RaceTypePy::TimingAnomaly,
        }
    }
}

/// A race condition finding.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceConditionPy {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub race_type: RaceTypePy,
    #[pyo3(get)]
    pub severity: Severity,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub endpoint: String,
    #[pyo3(get)]
    pub evidence: String,
    #[pyo3(get)]
    pub remediation: String,
    #[pyo3(get)]
    pub cvss_score: Option<f32>,
}

impl RaceConditionPy {
    fn from_engine(engine: eggsec::hunt::race::RaceCondition) -> Self {
        Self {
            id: engine.id,
            race_type: RaceTypePy::from_engine(engine.race_type),
            severity: Severity::from_engine(engine.severity),
            description: engine.description,
            endpoint: engine.endpoint,
            evidence: engine.evidence,
            remediation: String::new(),
            cvss_score: None,
        }
    }
}

#[pymethods]
impl RaceConditionPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.id)?;
        dict.set_item("race_type", self.race_type.as_str())?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("description", &self.description)?;
        dict.set_item("endpoint", &self.endpoint)?;
        dict.set_item("evidence", &self.evidence)?;
        dict.set_item("remediation", &self.remediation)?;
        dict.set_item("cvss_score", self.cvss_score)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "RaceCondition(id={}, type={})",
            self.id,
            self.race_type.as_str()
        )
    }
}

/// Authorization bypass type.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BypassTypePy {
    Idor,
    MissingAuthorization,
    PrivilegeEscalation,
    ForceBrowsing,
    APIKeyLeak,
    JWTBypass,
    RoleManipulation,
}

#[pymethods]
impl BypassTypePy {
    fn __repr__(&self) -> String {
        format!("BypassType.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl BypassTypePy {
    fn as_str(&self) -> &str {
        match self {
            BypassTypePy::Idor => "Idor",
            BypassTypePy::MissingAuthorization => "MissingAuthorization",
            BypassTypePy::PrivilegeEscalation => "PrivilegeEscalation",
            BypassTypePy::ForceBrowsing => "ForceBrowsing",
            BypassTypePy::APIKeyLeak => "APIKeyLeak",
            BypassTypePy::JWTBypass => "JWTBypass",
            BypassTypePy::RoleManipulation => "RoleManipulation",
        }
    }

    fn from_engine(engine: eggsec::hunt::authz::BypassType) -> Self {
        match engine {
            eggsec::hunt::authz::BypassType::Idor => BypassTypePy::Idor,
            eggsec::hunt::authz::BypassType::MissingAuthorization => {
                BypassTypePy::MissingAuthorization
            }
            eggsec::hunt::authz::BypassType::PrivilegeEscalation => {
                BypassTypePy::PrivilegeEscalation
            }
            eggsec::hunt::authz::BypassType::ForceBrowsing => BypassTypePy::ForceBrowsing,
            eggsec::hunt::authz::BypassType::APIKeyLeak => BypassTypePy::APIKeyLeak,
            eggsec::hunt::authz::BypassType::JWTBypass => BypassTypePy::JWTBypass,
            eggsec::hunt::authz::BypassType::RoleManipulation => BypassTypePy::RoleManipulation,
        }
    }
}

/// An authorization bypass finding.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthzBypassPy {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub bypass_type: BypassTypePy,
    #[pyo3(get)]
    pub severity: Severity,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub endpoint: String,
    #[pyo3(get)]
    pub evidence: String,
    #[pyo3(get)]
    pub remediation: String,
    #[pyo3(get)]
    pub cvss_score: Option<f32>,
}

impl AuthzBypassPy {
    fn from_engine(engine: eggsec::hunt::authz::AuthzBypass) -> Self {
        Self {
            id: engine.id,
            bypass_type: BypassTypePy::from_engine(engine.bypass_type),
            severity: Severity::from_engine(engine.severity),
            description: engine.description,
            endpoint: engine.endpoint,
            evidence: engine.evidence,
            remediation: engine.remediation,
            cvss_score: engine.cvss_score,
        }
    }
}

#[pymethods]
impl AuthzBypassPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.id)?;
        dict.set_item("bypass_type", self.bypass_type.as_str())?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("description", &self.description)?;
        dict.set_item("endpoint", &self.endpoint)?;
        dict.set_item("evidence", &self.evidence)?;
        dict.set_item("remediation", &self.remediation)?;
        dict.set_item("cvss_score", self.cvss_score)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "AuthzBypass(id={}, type={})",
            self.id,
            self.bypass_type.as_str()
        )
    }
}

/// Session issue type.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionIssueTypePy {
    SessionFixation,
    SessionTimeout,
    TokenPrediction,
    InsufficientEntropy,
    MissingHttpOnly,
    MissingSecure,
    MissingSameSite,
    Csrf,
    ConcurrentSessions,
}

#[pymethods]
impl SessionIssueTypePy {
    fn __repr__(&self) -> String {
        format!("SessionIssueType.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl SessionIssueTypePy {
    fn as_str(&self) -> &str {
        match self {
            SessionIssueTypePy::SessionFixation => "SessionFixation",
            SessionIssueTypePy::SessionTimeout => "SessionTimeout",
            SessionIssueTypePy::TokenPrediction => "TokenPrediction",
            SessionIssueTypePy::InsufficientEntropy => "InsufficientEntropy",
            SessionIssueTypePy::MissingHttpOnly => "MissingHttpOnly",
            SessionIssueTypePy::MissingSecure => "MissingSecure",
            SessionIssueTypePy::MissingSameSite => "MissingSameSite",
            SessionIssueTypePy::Csrf => "Csrf",
            SessionIssueTypePy::ConcurrentSessions => "ConcurrentSessions",
        }
    }

    fn from_engine(engine: eggsec::hunt::session::SessionIssueType) -> Self {
        match engine {
            eggsec::hunt::session::SessionIssueType::SessionFixation => {
                SessionIssueTypePy::SessionFixation
            }
            eggsec::hunt::session::SessionIssueType::SessionTimeout => {
                SessionIssueTypePy::SessionTimeout
            }
            eggsec::hunt::session::SessionIssueType::TokenPrediction => {
                SessionIssueTypePy::TokenPrediction
            }
            eggsec::hunt::session::SessionIssueType::InsufficientEntropy => {
                SessionIssueTypePy::InsufficientEntropy
            }
            eggsec::hunt::session::SessionIssueType::MissingHttpOnly => {
                SessionIssueTypePy::MissingHttpOnly
            }
            eggsec::hunt::session::SessionIssueType::MissingSecure => {
                SessionIssueTypePy::MissingSecure
            }
            eggsec::hunt::session::SessionIssueType::MissingSameSite => {
                SessionIssueTypePy::MissingSameSite
            }
            eggsec::hunt::session::SessionIssueType::Csrf => SessionIssueTypePy::Csrf,
            eggsec::hunt::session::SessionIssueType::ConcurrentSessions => {
                SessionIssueTypePy::ConcurrentSessions
            }
        }
    }
}

/// A session security issue.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionIssuePy {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub issue_type: SessionIssueTypePy,
    #[pyo3(get)]
    pub severity: Severity,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub evidence: String,
    #[pyo3(get)]
    pub remediation: String,
    #[pyo3(get)]
    pub cvss_score: Option<f32>,
}

impl SessionIssuePy {
    fn from_engine(engine: eggsec::hunt::session::SessionIssue) -> Self {
        Self {
            id: engine.id,
            issue_type: SessionIssueTypePy::from_engine(engine.issue_type),
            severity: Severity::from_engine(engine.severity),
            description: engine.description,
            evidence: engine.evidence,
            remediation: engine.remediation,
            cvss_score: engine.cvss_score,
        }
    }
}

#[pymethods]
impl SessionIssuePy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.id)?;
        dict.set_item("issue_type", self.issue_type.as_str())?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("description", &self.description)?;
        dict.set_item("evidence", &self.evidence)?;
        dict.set_item("remediation", &self.remediation)?;
        dict.set_item("cvss_score", self.cvss_score)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "SessionIssue(id={}, type={})",
            self.id,
            self.issue_type.as_str()
        )
    }
}

/// Configuration for advanced vulnerability hunting.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct HuntTestConfigPy {
    #[pyo3(get)]
    pub check_attack_chains: bool,
    #[pyo3(get)]
    pub check_business_logic: bool,
    #[pyo3(get)]
    pub check_race_conditions: bool,
    #[pyo3(get)]
    pub check_authz_bypass: bool,
    #[pyo3(get)]
    pub check_session: bool,
    #[pyo3(get)]
    pub concurrency: usize,
    #[pyo3(get)]
    pub timeout_ms: u64,
}

impl Default for HuntTestConfigPy {
    fn default() -> Self {
        Self {
            check_attack_chains: true,
            check_business_logic: true,
            check_race_conditions: true,
            check_authz_bypass: true,
            check_session: true,
            concurrency: 10,
            timeout_ms: 10000,
        }
    }
}

#[pymethods]
impl HuntTestConfigPy {
    /// Create a new hunt test configuration.
    ///
    /// Args:
    ///     check_attack_chains: Detect attack chains (default: true).
    ///     check_business_logic: Detect business logic flaws (default: true).
    ///     check_race_conditions: Test for race conditions (default: true).
    ///     check_authz_bypass: Test for authorization bypass (default: true).
    ///     check_session: Test session security (default: true).
    ///     concurrency: Number of concurrent requests (default: 10).
    ///     timeout_ms: Request timeout in milliseconds (default: 10000).
    #[new]
    #[pyo3(signature = (*, check_attack_chains=true, check_business_logic=true, check_race_conditions=true, check_authz_bypass=true, check_session=true, concurrency=10, timeout_ms=10000))]
    fn new(
        check_attack_chains: bool,
        check_business_logic: bool,
        check_race_conditions: bool,
        check_authz_bypass: bool,
        check_session: bool,
        concurrency: usize,
        timeout_ms: u64,
    ) -> PyResult<Self> {
        Ok(Self {
            check_attack_chains,
            check_business_logic,
            check_race_conditions,
            check_authz_bypass,
            check_session,
            concurrency,
            timeout_ms,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "HuntTestConfig(chains={}, business={}, race={}, authz={}, session={})",
            self.check_attack_chains,
            self.check_business_logic,
            self.check_race_conditions,
            self.check_authz_bypass,
            self.check_session
        )
    }
}

/// Complete vulnerability hunt report.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuntReportPy {
    #[pyo3(get)]
    pub target: String,
    attack_chains: Vec<AttackChainPy>,
    business_logic: Vec<BusinessLogicFlawPy>,
    race_conditions: Vec<RaceConditionPy>,
    authz_bypasses: Vec<AuthzBypassPy>,
    session_issues: Vec<SessionIssuePy>,
    #[pyo3(get)]
    pub total_findings: usize,
}

impl HuntReportPy {
    fn from_engine(engine: eggsec::hunt::HuntReport) -> Self {
        Self {
            target: engine.target,
            attack_chains: engine
                .attack_chains
                .into_iter()
                .map(AttackChainPy::from_engine)
                .collect(),
            business_logic: engine
                .business_logic
                .into_iter()
                .map(BusinessLogicFlawPy::from_engine)
                .collect(),
            race_conditions: engine
                .race_conditions
                .into_iter()
                .map(RaceConditionPy::from_engine)
                .collect(),
            authz_bypasses: engine
                .authz_bypasses
                .into_iter()
                .map(AuthzBypassPy::from_engine)
                .collect(),
            session_issues: engine
                .session_issues
                .into_iter()
                .map(SessionIssuePy::from_engine)
                .collect(),
            total_findings: engine.total_findings,
        }
    }
}

#[pymethods]
impl HuntReportPy {
    #[getter]
    fn attack_chains(&self) -> Vec<AttackChainPy> {
        self.attack_chains.clone()
    }

    #[getter]
    fn business_logic(&self) -> Vec<BusinessLogicFlawPy> {
        self.business_logic.clone()
    }

    #[getter]
    fn race_conditions(&self) -> Vec<RaceConditionPy> {
        self.race_conditions.clone()
    }

    #[getter]
    fn authz_bypasses(&self) -> Vec<AuthzBypassPy> {
        self.authz_bypasses.clone()
    }

    #[getter]
    fn session_issues(&self) -> Vec<SessionIssuePy> {
        self.session_issues.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("total_findings", self.total_findings)?;

        let chains_list = PyList::empty_bound(py);
        for c in &self.attack_chains {
            chains_list.append(c.to_dict(py)?)?;
        }
        dict.set_item("attack_chains", chains_list)?;

        let business_list = PyList::empty_bound(py);
        for b in &self.business_logic {
            business_list.append(b.to_dict(py)?)?;
        }
        dict.set_item("business_logic", business_list)?;

        let race_list = PyList::empty_bound(py);
        for r in &self.race_conditions {
            race_list.append(r.to_dict(py)?)?;
        }
        dict.set_item("race_conditions", race_list)?;

        let authz_list = PyList::empty_bound(py);
        for a in &self.authz_bypasses {
            authz_list.append(a.to_dict(py)?)?;
        }
        dict.set_item("authz_bypasses", authz_list)?;

        let session_list = PyList::empty_bound(py);
        for s in &self.session_issues {
            session_list.append(s.to_dict(py)?)?;
        }
        dict.set_item("session_issues", session_list)?;

        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "HuntReport(target={}, findings={})",
            self.target, self.total_findings
        )
    }
}

/// Run advanced vulnerability hunting against a target.
///
/// Performs attack chain detection, business logic flaw detection,
/// race condition testing, authorization bypass testing, and session
/// security analysis.
///
/// Args:
///     target: Target URL (e.g. "https://example.com").
///     config: Hunt test configuration (optional).
///
/// Returns:
///     HuntReportPy: Full hunt report with findings.
///
/// Raises:
///     FeatureUnavailableError: If advanced-hunting feature is not enabled.
///     NetworkError: If the target is unreachable.
///     ConfigError: If the target URL is invalid.
#[pyfunction]
#[pyo3(signature = (target, config=None))]
pub fn hunt_test(target: &str, config: Option<HuntTestConfigPy>) -> PyResult<HuntReportPy> {
    let cfg = config.unwrap_or_default();
    let target_owned = target.to_string();

    Python::with_gil(|py| {
        let result = runtime_sync::block_on(py, async move {
            let hunt_config = eggsec::hunt::HuntConfig {
                check_attack_chains: cfg.check_attack_chains,
                check_business_logic: cfg.check_business_logic,
                check_race_conditions: cfg.check_race_conditions,
                check_authz_bypass: cfg.check_authz_bypass,
                check_session: cfg.check_session,
                concurrency: cfg.concurrency,
                timeout_ms: cfg.timeout_ms,
            };
            eggsec::hunt::run_hunt(&target_owned, hunt_config)
                .await
                .map_err(|e| {
                    pyo3::exceptions::PyRuntimeError::new_err(format!("Hunt failed: {}", e))
                })
        })?;

        Ok(HuntReportPy::from_engine(result))
    })
}

/// Run advanced vulnerability hunting (async).
///
/// Returns a PyFuture that resolves to a HuntReportPy.
#[pyfunction]
#[pyo3(signature = (target, config=None))]
pub fn async_hunt_test(
    target: &str,
    config: Option<HuntTestConfigPy>,
) -> PyResult<crate::runtime_async::PyFuture> {
    let cfg = config.unwrap_or_default();
    let target_owned = target.to_string();

    crate::runtime_async::spawn_async(async move {
        let hunt_config = eggsec::hunt::HuntConfig {
            check_attack_chains: cfg.check_attack_chains,
            check_business_logic: cfg.check_business_logic,
            check_race_conditions: cfg.check_race_conditions,
            check_authz_bypass: cfg.check_authz_bypass,
            check_session: cfg.check_session,
            concurrency: cfg.concurrency,
            timeout_ms: cfg.timeout_ms,
        };
        let report = eggsec::hunt::run_hunt(&target_owned, hunt_config)
            .await
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Hunt failed: {}", e))
            })?;
        Ok(HuntReportPy::from_engine(report))
    })
}
