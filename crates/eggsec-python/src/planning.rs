use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::async_engine::AsyncEngine;
use crate::engine::Engine;
use crate::requests::OperationRequest;

/// A single step in a scan plan.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct PlanStep {
    #[pyo3(get)]
    operation: String,
    request: OperationRequest,
    #[pyo3(get)]
    rationale: String,
    #[pyo3(get)]
    priority: u8,
    #[pyo3(get)]
    estimated_duration_ms: u64,
}

#[pymethods]
impl PlanStep {
    #[new]
    #[pyo3(signature = (operation, request, rationale, priority=1, estimated_duration_ms=5000))]
    fn new(
        operation: String,
        request: OperationRequest,
        rationale: String,
        priority: u8,
        estimated_duration_ms: u64,
    ) -> Self {
        Self {
            operation,
            request,
            rationale,
            priority,
            estimated_duration_ms,
        }
    }

    #[getter]
    fn request(&self) -> OperationRequest {
        self.request.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("operation", &self.operation)?;
        dict.set_item("request", self.request.to_dict(py)?)?;
        dict.set_item("rationale", &self.rationale)?;
        dict.set_item("priority", self.priority)?;
        dict.set_item("estimated_duration_ms", self.estimated_duration_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PlanStep(operation={}, priority={})",
            self.operation, self.priority
        )
    }
}

impl serde::Serialize for PlanStep {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("PlanStep", 5)?;
        s.serialize_field("operation", &self.operation)?;
        s.serialize_field("request", &self.request)?;
        s.serialize_field("rationale", &self.rationale)?;
        s.serialize_field("priority", &self.priority)?;
        s.serialize_field("estimated_duration_ms", &self.estimated_duration_ms)?;
        s.end()
    }
}

/// A scan plan suggesting operations for a target.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ScanPlan {
    #[pyo3(get)]
    target: String,
    steps: Vec<PlanStep>,
    #[pyo3(get)]
    estimated_total_ms: u64,
    #[pyo3(get)]
    scope_notes: Vec<String>,
}

#[pymethods]
impl ScanPlan {
    #[new]
    #[pyo3(signature = (target, steps=None, estimated_total_ms=0, scope_notes=None))]
    fn new(
        target: String,
        steps: Option<Vec<PlanStep>>,
        estimated_total_ms: u64,
        scope_notes: Option<Vec<String>>,
    ) -> Self {
        Self {
            target,
            steps: steps.unwrap_or_default(),
            estimated_total_ms,
            scope_notes: scope_notes.unwrap_or_default(),
        }
    }

    #[getter]
    fn steps(&self) -> Vec<PlanStep> {
        self.steps.clone()
    }

    fn steps_count(&self) -> usize {
        self.steps.len()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;

        let steps_list = PyList::empty_bound(py);
        for step in &self.steps {
            steps_list.append(step.to_dict(py)?)?;
        }
        dict.set_item("steps", steps_list)?;
        dict.set_item("estimated_total_ms", self.estimated_total_ms)?;
        dict.set_item("scope_notes", &self.scope_notes)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ScanPlan(target={}, steps={}, estimated_total_ms={})",
            self.target,
            self.steps.len(),
            self.estimated_total_ms
        )
    }

    fn __str__(&self) -> String {
        format!(
            "ScanPlan for '{}' with {} steps (~{}ms)",
            self.target,
            self.steps.len(),
            self.estimated_total_ms
        )
    }
}

impl serde::Serialize for ScanPlan {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("ScanPlan", 4)?;
        s.serialize_field("target", &self.target)?;
        s.serialize_field("steps", &self.steps)?;
        s.serialize_field("estimated_total_ms", &self.estimated_total_ms)?;
        s.serialize_field("scope_notes", &self.scope_notes)?;
        s.end()
    }
}

/// Classify a target as a URL, IP address, or domain.
fn classify_target(target: &str) -> &'static str {
    if target.starts_with("http://") || target.starts_with("https://") {
        "url"
    } else if target.parse::<std::net::IpAddr>().is_ok() {
        "ip"
    } else {
        "domain"
    }
}

/// Build a basic scan plan based on target heuristics.
pub(crate) fn build_scan_plan(target: &str) -> ScanPlan {
    let kind = classify_target(target);
    let mut steps = Vec::new();
    let mut scope_notes = Vec::new();
    let mut total_ms: u64 = 0;

    // Always start with port_scan
    let port_scan_req =
        OperationRequest::new("scan_ports".to_string(), target.to_string(), None, None);
    let port_scan_duration = 10000;
    steps.push(PlanStep {
        operation: "scan_ports".to_string(),
        request: port_scan_req,
        rationale: "Always start with port scanning to discover open services".to_string(),
        priority: 1,
        estimated_duration_ms: port_scan_duration,
    });
    total_ms += port_scan_duration;

    match kind {
        "url" => {
            let tech_detect_req =
                OperationRequest::new("tech_detect".to_string(), target.to_string(), None, None);
            let tech_duration = 5000;
            steps.push(PlanStep {
                operation: "tech_detect".to_string(),
                request: tech_detect_req,
                rationale: "Identify web technologies and frameworks".to_string(),
                priority: 2,
                estimated_duration_ms: tech_duration,
            });
            total_ms += tech_duration;

            let waf_detect_req =
                OperationRequest::new("waf_detect".to_string(), target.to_string(), None, None);
            let waf_duration = 3000;
            steps.push(PlanStep {
                operation: "waf_detect".to_string(),
                request: waf_detect_req,
                rationale: "Check for Web Application Firewall presence".to_string(),
                priority: 3,
                estimated_duration_ms: waf_duration,
            });
            total_ms += waf_duration;

            let endpoint_req =
                OperationRequest::new("scan_endpoints".to_string(), target.to_string(), None, None);
            let endpoint_duration = 8000;
            steps.push(PlanStep {
                operation: "scan_endpoints".to_string(),
                request: endpoint_req,
                rationale: "Discover web endpoints and paths".to_string(),
                priority: 4,
                estimated_duration_ms: endpoint_duration,
            });
            total_ms += endpoint_duration;

            scope_notes.push(
                "URL target: added tech detection, WAF detection, and endpoint scanning"
                    .to_string(),
            );
        }
        "ip" => {
            let fingerprint_req =
                OperationRequest::new("fingerprint".to_string(), target.to_string(), None, None);
            let fingerprint_duration = 6000;
            steps.push(PlanStep {
                operation: "fingerprint".to_string(),
                request: fingerprint_req,
                rationale: "Identify services running on open ports".to_string(),
                priority: 2,
                estimated_duration_ms: fingerprint_duration,
            });
            total_ms += fingerprint_duration;

            scope_notes.push("IP target: added service fingerprinting".to_string());
        }
        "domain" => {
            let dns_req =
                OperationRequest::new("recon_dns".to_string(), target.to_string(), None, None);
            let dns_duration = 3000;
            steps.push(PlanStep {
                operation: "recon_dns".to_string(),
                request: dns_req,
                rationale: "Enumerate DNS records for the domain".to_string(),
                priority: 2,
                estimated_duration_ms: dns_duration,
            });
            total_ms += dns_duration;

            let tls_req =
                OperationRequest::new("tls_inspect".to_string(), target.to_string(), None, None);
            let tls_duration = 4000;
            steps.push(PlanStep {
                operation: "tls_inspect".to_string(),
                request: tls_req,
                rationale: "Analyze TLS/SSL configuration and certificate".to_string(),
                priority: 3,
                estimated_duration_ms: tls_duration,
            });
            total_ms += tls_duration;

            scope_notes.push("Domain target: added DNS recon and TLS inspection".to_string());
        }
        _ => {
            scope_notes.push("Unknown target type: only port scanning suggested".to_string());
        }
    }

    ScanPlan {
        target: target.to_string(),
        steps,
        estimated_total_ms: total_ms,
        scope_notes,
    }
}

impl Engine {
    /// Create a scan plan for a target.
    pub(crate) fn plan_inner(&self, target: &str) -> PyResult<ScanPlan> {
        Ok(build_scan_plan(target))
    }
}

impl AsyncEngine {
    pub(crate) fn plan_inner(&self, target: &str) -> PyResult<ScanPlan> {
        Ok(build_scan_plan(target))
    }
}
