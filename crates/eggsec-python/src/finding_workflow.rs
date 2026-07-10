use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

/// Lifecycle state for a finding.
#[pyclass(frozen, name = "FindingState")]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FindingStatePy {
    New,
    Triaged,
    Confirmed,
    InProgress,
    AcceptedRisk,
    FalsePositive,
    Remediated,
    Reopened,
}

#[pymethods]
impl FindingStatePy {
    #[staticmethod]
    fn from_str(s: &str) -> PyResult<Self> {
        match s.to_lowercase().as_str() {
            "new" => Ok(Self::New),
            "triaged" => Ok(Self::Triaged),
            "confirmed" => Ok(Self::Confirmed),
            "in_progress" | "inprogress" => Ok(Self::InProgress),
            "accepted_risk" | "acceptedrisk" => Ok(Self::AcceptedRisk),
            "false_positive" | "falsepositive" => Ok(Self::FalsePositive),
            "remediated" => Ok(Self::Remediated),
            "reopened" => Ok(Self::Reopened),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid finding state: '{s}'. Must be one of: new, triaged, confirmed, in_progress, accepted_risk, false_positive, remediated, reopened"
            ))),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Triaged => "triaged",
            Self::Confirmed => "confirmed",
            Self::InProgress => "in_progress",
            Self::AcceptedRisk => "accepted_risk",
            Self::FalsePositive => "false_positive",
            Self::Remediated => "remediated",
            Self::Reopened => "reopened",
        }
    }

    fn __repr__(&self) -> String {
        format!("FindingState.{:?}", self)
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

/// A single state transition in the finding lifecycle.
#[pyclass(frozen, name = "WorkflowTransition")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTransitionPy {
    #[pyo3(get)]
    pub from_state: FindingStatePy,
    #[pyo3(get)]
    pub to_state: FindingStatePy,
    #[pyo3(get)]
    pub changed_at: String,
    #[pyo3(get)]
    pub changed_by: Option<String>,
    #[pyo3(get)]
    pub note: Option<String>,
}

#[pymethods]
impl WorkflowTransitionPy {
    #[new]
    #[pyo3(signature = (from_state, to_state, changed_at, *, changed_by=None, note=None))]
    fn new(
        from_state: FindingStatePy,
        to_state: FindingStatePy,
        changed_at: String,
        changed_by: Option<String>,
        note: Option<String>,
    ) -> Self {
        Self {
            from_state,
            to_state,
            changed_at,
            changed_by,
            note,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("from_state", self.from_state.as_str())?;
        dict.set_item("to_state", self.to_state.as_str())?;
        dict.set_item("changed_at", &self.changed_at)?;
        dict.set_item("changed_by", &self.changed_by)?;
        dict.set_item("note", &self.note)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "WorkflowTransition({} -> {})",
            self.from_state.as_str(),
            self.to_state.as_str()
        )
    }
}

/// A suppression record for hiding findings from reports.
#[pyclass(frozen, name = "Suppression")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuppressionPy {
    #[pyo3(get)]
    pub finding_id: String,
    #[pyo3(get)]
    pub reason: String,
    #[pyo3(get)]
    pub suppressed_by: Option<String>,
    #[pyo3(get)]
    pub suppressed_at: String,
    #[pyo3(get)]
    pub expires_at: Option<String>,
}

#[pymethods]
impl SuppressionPy {
    #[new]
    #[pyo3(signature = (finding_id, reason, suppressed_at, *, suppressed_by=None, expires_at=None))]
    fn new(
        finding_id: String,
        reason: String,
        suppressed_at: String,
        suppressed_by: Option<String>,
        expires_at: Option<String>,
    ) -> Self {
        Self {
            finding_id,
            reason,
            suppressed_by,
            suppressed_at,
            expires_at,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("finding_id", &self.finding_id)?;
        dict.set_item("reason", &self.reason)?;
        dict.set_item("suppressed_by", &self.suppressed_by)?;
        dict.set_item("suppressed_at", &self.suppressed_at)?;
        dict.set_item("expires_at", &self.expires_at)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "Suppression(finding_id={}, reason={})",
            self.finding_id, self.reason
        )
    }
}

/// Returns the valid target states for a given current state.
fn valid_targets(from: &FindingStatePy) -> Vec<FindingStatePy> {
    match from {
        FindingStatePy::New => vec![FindingStatePy::Triaged, FindingStatePy::FalsePositive],
        FindingStatePy::Triaged => vec![
            FindingStatePy::Confirmed,
            FindingStatePy::FalsePositive,
            FindingStatePy::New,
        ],
        FindingStatePy::Confirmed => vec![
            FindingStatePy::InProgress,
            FindingStatePy::AcceptedRisk,
            FindingStatePy::FalsePositive,
        ],
        FindingStatePy::InProgress => vec![FindingStatePy::Remediated, FindingStatePy::Confirmed],
        FindingStatePy::AcceptedRisk => vec![FindingStatePy::Reopened],
        FindingStatePy::FalsePositive => vec![FindingStatePy::Reopened],
        FindingStatePy::Remediated => vec![FindingStatePy::Reopened],
        FindingStatePy::Reopened => vec![FindingStatePy::Triaged, FindingStatePy::Confirmed],
    }
}

/// Manages finding lifecycle state, transitions, and suppressions.
#[pyclass(name = "FindingWorkflow")]
pub struct FindingWorkflowPy {
    states: RwLock<HashMap<String, FindingStatePy>>,
    transitions: RwLock<HashMap<String, Vec<WorkflowTransitionPy>>>,
    suppressions: RwLock<HashMap<String, SuppressionPy>>,
}

#[pymethods]
impl FindingWorkflowPy {
    #[new]
    fn new() -> Self {
        Self {
            states: RwLock::new(HashMap::new()),
            transitions: RwLock::new(HashMap::new()),
            suppressions: RwLock::new(HashMap::new()),
        }
    }

    /// Register a finding with initial state `New`.
    fn register_finding(&self, finding_id: &str) -> PyResult<()> {
        let mut states = self.states.write().unwrap();
        if states.contains_key(finding_id) {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Finding already registered: {finding_id}"
            )));
        }
        states.insert(finding_id.to_string(), FindingStatePy::New);
        self.transitions
            .write()
            .unwrap()
            .insert(finding_id.to_string(), Vec::new());
        Ok(())
    }

    /// Get the current state of a finding.
    fn get_state(&self, finding_id: &str) -> PyResult<FindingStatePy> {
        self.states
            .read()
            .unwrap()
            .get(finding_id)
            .cloned()
            .ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "Finding not registered: {finding_id}"
                ))
            })
    }

    /// Transition a finding to a new state.
    #[pyo3(signature = (finding_id, to_state, *, changed_by=None, note=None))]
    fn transition(
        &self,
        finding_id: &str,
        to_state: FindingStatePy,
        changed_by: Option<String>,
        note: Option<String>,
    ) -> PyResult<WorkflowTransitionPy> {
        let from_state = self.get_state(finding_id)?;

        if !valid_targets(&from_state).contains(&to_state) {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid transition: {} -> {} for finding {finding_id}",
                from_state.as_str(),
                to_state.as_str()
            )));
        }

        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        let transition = WorkflowTransitionPy {
            from_state: from_state.clone(),
            to_state: to_state.clone(),
            changed_at: now,
            changed_by,
            note,
        };

        self.states
            .write()
            .unwrap()
            .insert(finding_id.to_string(), to_state);
        self.transitions
            .write()
            .unwrap()
            .entry(finding_id.to_string())
            .or_default()
            .push(transition.clone());

        Ok(transition)
    }

    /// Check whether a transition is valid from the finding's current state.
    fn can_transition(&self, finding_id: &str, to_state: &FindingStatePy) -> bool {
        match self.get_state(finding_id) {
            Ok(from) => valid_targets(&from).contains(to_state),
            Err(_) => false,
        }
    }

    /// Return the list of valid target states from the current state.
    fn valid_transitions(&self, finding_id: &str) -> PyResult<Vec<FindingStatePy>> {
        let from = self.get_state(finding_id)?;
        Ok(valid_targets(&from))
    }

    /// Suppress a finding so it is excluded from reports.
    #[pyo3(signature = (finding_id, reason, *, suppressed_by=None, expires_at=None))]
    fn suppress(
        &self,
        finding_id: &str,
        reason: String,
        suppressed_by: Option<String>,
        expires_at: Option<String>,
    ) -> PyResult<SuppressionPy> {
        // Verify finding exists
        self.get_state(finding_id)?;

        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        let suppression = SuppressionPy {
            finding_id: finding_id.to_string(),
            reason,
            suppressed_by,
            suppressed_at: now,
            expires_at,
        };

        self.suppressions
            .write()
            .unwrap()
            .insert(finding_id.to_string(), suppression.clone());

        Ok(suppression)
    }

    /// Check whether a finding is currently suppressed (and not expired).
    fn is_suppressed(&self, finding_id: &str) -> bool {
        let suppressions = self.suppressions.read().unwrap();
        match suppressions.get(finding_id) {
            Some(sup) => match &sup.expires_at {
                Some(expires) => {
                    // If expiration is parseable, check if still valid
                    match chrono::DateTime::parse_from_rfc3339(expires) {
                        Ok(parsed) => chrono::Utc::now() < parsed,
                        Err(_) => true, // Unparseable = treat as non-expired
                    }
                }
                None => true, // No expiration = permanently suppressed
            },
            None => false,
        }
    }

    /// Remove suppression from a finding.
    fn unsuppress(&self, finding_id: &str) -> PyResult<()> {
        let removed = self
            .suppressions
            .write()
            .unwrap()
            .remove(finding_id)
            .is_some();
        if !removed {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Finding is not suppressed: {finding_id}"
            )));
        }
        Ok(())
    }

    /// Get the full transition history for a finding.
    fn get_history(&self, finding_id: &str) -> PyResult<Vec<WorkflowTransitionPy>> {
        self.transitions
            .read()
            .unwrap()
            .get(finding_id)
            .cloned()
            .ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "Finding not registered: {finding_id}"
                ))
            })
    }

    fn __repr__(&self) -> String {
        let len = self.states.read().unwrap().len();
        format!("FindingWorkflow(findings={len})")
    }
}
