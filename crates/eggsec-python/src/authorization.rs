use pyo3::prelude::*;

use crate::config_model::PyEggsecConfig;

/// Python wrapper for Eggsec execution policy.
///
/// Controls which operations are allowed and under what conditions.
/// Loaded from the `[execution_policy]` section of the config file.
#[pyclass(frozen, name = "ExecutionPolicyPy")]
#[derive(Clone)]
pub struct ExecutionPolicyPy {
    pub(crate) inner: eggsec::config::ExecutionPolicy,
}

#[pymethods]
impl ExecutionPolicyPy {
    /// Create a default (restrictive) execution policy.
    #[new]
    fn new() -> Self {
        Self {
            inner: eggsec::config::ExecutionPolicy::default(),
        }
    }

    /// Create an execution policy from an EggsecConfig.
    ///
    /// Uses the execution_policy section of the config, or defaults if absent.
    #[staticmethod]
    fn from_config(config: &PyEggsecConfig) -> Self {
        Self {
            inner: config.to_inner().execution_policy,
        }
    }

    // --- Read-only getters for all boolean fields ---

    #[getter]
    fn require_explicit_scope(&self) -> bool {
        self.inner.require_explicit_scope
    }

    #[getter]
    fn allow_passive_fingerprint(&self) -> bool {
        true
    }

    #[getter]
    fn allow_active_scan(&self) -> bool {
        true
    }

    #[getter]
    fn allow_exploit(&self) -> bool {
        self.inner.allow_exploit_adjacent
    }

    #[getter]
    fn allow_dos(&self) -> bool {
        self.inner.allow_stress_testing
    }

    #[getter]
    fn allow_brute_force(&self) -> bool {
        self.inner.allow_credential_testing
    }

    #[getter]
    fn allow_credential_stuffing(&self) -> bool {
        self.inner.allow_credential_testing
    }

    #[getter]
    fn allow_web_fuzzing(&self) -> bool {
        self.inner.allow_intrusive_fuzzing
    }

    #[getter]
    fn allow_network_fuzzing(&self) -> bool {
        self.inner.allow_intrusive_fuzzing
    }

    #[getter]
    fn allow_mitm_proxy(&self) -> bool {
        self.inner.allow_traffic_interception
    }

    #[getter]
    fn allow_packet_capture(&self) -> bool {
        self.inner.allow_raw_packets
    }

    #[getter]
    fn allow_waf_bypass(&self) -> bool {
        self.inner.allow_intrusive_fuzzing
    }

    #[getter]
    fn allow_sql_injection(&self) -> bool {
        self.inner.allow_intrusive_fuzzing
    }

    #[getter]
    fn allow_xss_injection(&self) -> bool {
        self.inner.allow_intrusive_fuzzing
    }

    #[getter]
    fn allow_command_injection(&self) -> bool {
        self.inner.allow_intrusive_fuzzing
    }

    #[getter]
    fn allow_path_traversal(&self) -> bool {
        self.inner.allow_intrusive_fuzzing
    }

    #[getter]
    fn allow_ssrf(&self) -> bool {
        self.inner.allow_intrusive_fuzzing
    }

    #[getter]
    fn allow_mobile_dynamic(&self) -> bool {
        self.inner.allow_remote_execution
    }

    #[getter]
    fn allow_load_testing(&self) -> bool {
        self.inner.allow_load_testing
    }

    #[getter]
    fn allow_stress_testing(&self) -> bool {
        self.inner.allow_stress_testing
    }

    #[getter]
    fn allow_raw_packets(&self) -> bool {
        self.inner.allow_raw_packets
    }

    #[getter]
    fn allow_db_pentesting(&self) -> bool {
        self.inner.allow_db_pentesting
    }

    #[getter]
    fn allow_evasion_testing(&self) -> bool {
        self.inner.allow_evasion_testing
    }

    #[getter]
    fn allow_post_exploitation(&self) -> bool {
        self.inner.allow_post_exploitation
    }

    #[getter]
    fn allow_c2_operations(&self) -> bool {
        self.inner.allow_c2_operations
    }

    #[getter]
    fn allow_agent_autonomous(&self) -> bool {
        self.inner.allow_agent_autonomous
    }

    #[getter]
    fn allow_remote_execution(&self) -> bool {
        self.inner.allow_remote_execution
    }

    #[getter]
    fn max_risk_without_confirm(&self) -> String {
        self.inner.max_risk_without_confirm.to_string()
    }

    #[getter]
    fn allowed_capabilities(&self) -> Vec<String> {
        self.inner
            .allowed_capabilities
            .iter()
            .map(|c| c.to_string())
            .collect()
    }

    #[getter]
    fn denied_capabilities(&self) -> Vec<String> {
        self.inner
            .denied_capabilities
            .iter()
            .map(|c| c.to_string())
            .collect()
    }

    /// Validate the execution policy and return any errors.
    fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.inner.max_risk_without_confirm < eggsec::config::OperationRisk::SafeActive {
            errors.push("max_risk_without_confirm must be at least 'safe_active'".to_string());
        }
        // Check for conflicting capability rules
        for cap in &self.inner.denied_capabilities {
            if self.inner.allowed_capabilities.contains(cap) {
                errors.push(format!("capability '{}' is both allowed and denied", cap));
            }
        }
        errors
    }

    fn __repr__(&self) -> String {
        format!(
            "ExecutionPolicy(explicit_scope={}, intrusive_fuzz={}, stress={}, load_test={})",
            self.inner.require_explicit_scope,
            self.inner.allow_intrusive_fuzzing,
            self.inner.allow_stress_testing,
            self.inner.allow_load_testing,
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Python wrapper for Eggsec manual override flags.
///
/// These flags are honored only for `ManualPermissive` execution profiles.
/// Never used for MCP, agent, or CI surfaces.
#[pyclass(frozen, name = "ManualOverridePy")]
#[derive(Clone)]
pub struct ManualOverridePy {
    pub(crate) inner: eggsec::config::ManualOverride,
}

#[pymethods]
impl ManualOverridePy {
    /// Create a new manual override.
    ///
    /// Args:
    ///     reason: The reason for the override (for audit trail).
    #[new]
    #[pyo3(signature = (reason, *, assume_yes=false, allow_out_of_scope=false, allow_explicit_exclusion=false, allow_high_risk=false, allow_db_pentest=false, allow_web_proxy=false, allow_nonbaseline_capability=false, allow_private_resolution=false, allow_cross_host_redirect=false))]
    fn new(
        reason: &str,
        assume_yes: bool,
        allow_out_of_scope: bool,
        allow_explicit_exclusion: bool,
        allow_high_risk: bool,
        allow_db_pentest: bool,
        allow_web_proxy: bool,
        allow_nonbaseline_capability: bool,
        allow_private_resolution: bool,
        allow_cross_host_redirect: bool,
    ) -> Self {
        Self {
            inner: eggsec::config::ManualOverride {
                assume_yes,
                allow_out_of_scope,
                allow_explicit_exclusion,
                allow_high_risk,
                allow_db_pentest,
                allow_web_proxy,
                allow_nonbaseline_capability,
                allow_private_resolution,
                allow_cross_host_redirect,
                reason: Some(reason.to_string()),
            },
        }
    }

    #[getter]
    fn reason(&self) -> Option<String> {
        self.inner.reason.clone()
    }

    #[getter]
    fn assume_yes(&self) -> bool {
        self.inner.assume_yes
    }

    #[getter]
    fn allow_out_of_scope(&self) -> bool {
        self.inner.allow_out_of_scope
    }

    #[getter]
    fn allow_explicit_exclusion(&self) -> bool {
        self.inner.allow_explicit_exclusion
    }

    #[getter]
    fn allow_high_risk(&self) -> bool {
        self.inner.allow_high_risk
    }

    #[getter]
    fn allow_db_pentest(&self) -> bool {
        self.inner.allow_db_pentest
    }

    #[getter]
    fn allow_web_proxy(&self) -> bool {
        self.inner.allow_web_proxy
    }

    #[getter]
    fn allow_nonbaseline_capability(&self) -> bool {
        self.inner.allow_nonbaseline_capability
    }

    #[getter]
    fn allow_private_resolution(&self) -> bool {
        self.inner.allow_private_resolution
    }

    #[getter]
    fn allow_cross_host_redirect(&self) -> bool {
        self.inner.allow_cross_host_redirect
    }

    /// Check if this override permits a given confirmation class.
    ///
    /// Args:
    ///     class_name: The confirmation class name (e.g. "out-of-scope", "high-risk").
    fn permits(&self, class_name: &str) -> bool {
        let class = match class_name {
            "out-of-scope" => eggsec::config::ConfirmationClass::OutOfScope,
            "target-expansion" => eggsec::config::ConfirmationClass::TargetExpansion,
            "private-resolution" => eggsec::config::ConfirmationClass::PrivateResolution,
            "cross-host-redirect" => eggsec::config::ConfirmationClass::CrossHostRedirect,
            "explicit-exclusion" => eggsec::config::ConfirmationClass::ExplicitExclusion,
            "high-risk" => eggsec::config::ConfirmationClass::HighRisk,
            "traffic-interception" => eggsec::config::ConfirmationClass::TrafficInterception,
            "nonbaseline-capability" => eggsec::config::ConfirmationClass::NonBaselineCapability,
            _ => return false,
        };
        self.inner.permits(class)
    }

    fn __repr__(&self) -> String {
        format!(
            "ManualOverride(reason={:?}, assume_yes={}, allow_out_of_scope={}, allow_high_risk={})",
            self.inner.reason,
            self.inner.assume_yes,
            self.inner.allow_out_of_scope,
            self.inner.allow_high_risk,
        )
    }
}
