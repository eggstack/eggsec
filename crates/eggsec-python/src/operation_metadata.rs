use pyo3::prelude::*;

/// Risk tier for operations. Higher variants are more dangerous.
///
/// Used by execution policy to control which operations are permitted
/// without explicit user confirmation.
#[pyclass(frozen, name = "OperationRisk")]
#[derive(Clone)]
pub struct OperationRiskPy {
    inner: eggsec::config::OperationRisk,
}

#[pymethods]
impl OperationRiskPy {
    #[staticmethod]
    fn passive() -> Self {
        Self {
            inner: eggsec::config::OperationRisk::Passive,
        }
    }

    #[staticmethod]
    fn safe_active() -> Self {
        Self {
            inner: eggsec::config::OperationRisk::SafeActive,
        }
    }

    #[staticmethod]
    fn intrusive() -> Self {
        Self {
            inner: eggsec::config::OperationRisk::Intrusive,
        }
    }

    #[staticmethod]
    fn load_test() -> Self {
        Self {
            inner: eggsec::config::OperationRisk::LoadTest,
        }
    }

    #[staticmethod]
    fn stress_test() -> Self {
        Self {
            inner: eggsec::config::OperationRisk::StressTest,
        }
    }

    #[staticmethod]
    fn raw_packet() -> Self {
        Self {
            inner: eggsec::config::OperationRisk::RawPacket,
        }
    }

    #[staticmethod]
    fn credential_testing() -> Self {
        Self {
            inner: eggsec::config::OperationRisk::CredentialTesting,
        }
    }

    #[staticmethod]
    fn db_pentest() -> Self {
        Self {
            inner: eggsec::config::OperationRisk::DbPentest,
        }
    }

    #[staticmethod]
    fn traffic_interception() -> Self {
        Self {
            inner: eggsec::config::OperationRisk::TrafficInterception,
        }
    }

    #[staticmethod]
    fn exploit_adjacent() -> Self {
        Self {
            inner: eggsec::config::OperationRisk::ExploitAdjacent,
        }
    }

    #[staticmethod]
    fn evasion_testing() -> Self {
        Self {
            inner: eggsec::config::OperationRisk::EvasionTesting,
        }
    }

    #[staticmethod]
    fn post_exploitation() -> Self {
        Self {
            inner: eggsec::config::OperationRisk::PostExploitation,
        }
    }

    #[staticmethod]
    fn c2_operation() -> Self {
        Self {
            inner: eggsec::config::OperationRisk::C2Operation,
        }
    }

    #[staticmethod]
    fn remote_execution() -> Self {
        Self {
            inner: eggsec::config::OperationRisk::RemoteExecution,
        }
    }

    #[staticmethod]
    fn agent_autonomous() -> Self {
        Self {
            inner: eggsec::config::OperationRisk::AgentAutonomous,
        }
    }

    /// Human-readable name of this risk level.
    #[getter]
    fn name(&self) -> String {
        format!("{}", self.inner)
    }

    /// Numeric ordering level (0-based, higher = more dangerous).
    #[getter]
    fn level(&self) -> u8 {
        match self.inner {
            eggsec::config::OperationRisk::Passive => 0,
            eggsec::config::OperationRisk::SafeActive => 1,
            eggsec::config::OperationRisk::Intrusive => 2,
            eggsec::config::OperationRisk::LoadTest => 3,
            eggsec::config::OperationRisk::StressTest => 4,
            eggsec::config::OperationRisk::RawPacket => 5,
            eggsec::config::OperationRisk::CredentialTesting => 6,
            eggsec::config::OperationRisk::DbPentest => 7,
            eggsec::config::OperationRisk::TrafficInterception => 8,
            eggsec::config::OperationRisk::ExploitAdjacent => 9,
            eggsec::config::OperationRisk::EvasionTesting => 10,
            eggsec::config::OperationRisk::PostExploitation => 11,
            eggsec::config::OperationRisk::C2Operation => 12,
            eggsec::config::OperationRisk::RemoteExecution => 13,
            eggsec::config::OperationRisk::AgentAutonomous => 14,
        }
    }

    fn __repr__(&self) -> String {
        format!("OperationRisk({})", self.name())
    }

    fn __str__(&self) -> String {
        self.name()
    }
}

/// Operating mode for an eggsec session.
///
/// Determines the safety boundary and allowed operation surface.
#[pyclass(frozen, name = "OperationMode")]
#[derive(Clone)]
pub struct OperationModePy {
    inner: eggsec::config::OperationMode,
}

#[pymethods]
impl OperationModePy {
    #[staticmethod]
    fn standard_assessment() -> Self {
        Self {
            inner: eggsec::config::OperationMode::StandardAssessment,
        }
    }

    #[staticmethod]
    fn defense_lab() -> Self {
        Self {
            inner: eggsec::config::OperationMode::DefenseLab,
        }
    }

    #[staticmethod]
    fn hazardous_lab() -> Self {
        Self {
            inner: eggsec::config::OperationMode::HazardousLab,
        }
    }

    /// Human-readable name of this mode.
    #[getter]
    fn name(&self) -> &'static str {
        self.inner.label()
    }

    fn __repr__(&self) -> String {
        format!("OperationMode({})", self.name())
    }

    fn __str__(&self) -> String {
        self.name().to_string()
    }
}

/// Intended use case for an operation or profile.
#[pyclass(frozen, name = "IntendedUse")]
#[derive(Clone)]
pub struct IntendedUsePy {
    inner: eggsec::config::IntendedUse,
}

#[pymethods]
impl IntendedUsePy {
    #[staticmethod]
    fn web_assessment() -> Self {
        Self {
            inner: eggsec::config::IntendedUse::WebAssessment,
        }
    }

    #[staticmethod]
    fn api_assessment() -> Self {
        Self {
            inner: eggsec::config::IntendedUse::ApiAssessment,
        }
    }

    #[staticmethod]
    fn waf_regression() -> Self {
        Self {
            inner: eggsec::config::IntendedUse::WafRegression,
        }
    }

    #[staticmethod]
    fn synvoid_regression() -> Self {
        Self {
            inner: eggsec::config::IntendedUse::SynvoidRegression,
        }
    }

    #[staticmethod]
    fn distributed_system_stress() -> Self {
        Self {
            inner: eggsec::config::IntendedUse::DistributedSystemStress,
        }
    }

    #[staticmethod]
    fn protocol_edge_validation() -> Self {
        Self {
            inner: eggsec::config::IntendedUse::ProtocolEdgeValidation,
        }
    }

    #[staticmethod]
    fn ci_regression() -> Self {
        Self {
            inner: eggsec::config::IntendedUse::CiRegression,
        }
    }

    #[staticmethod]
    fn coding_agent_verification() -> Self {
        Self {
            inner: eggsec::config::IntendedUse::CodingAgentVerification,
        }
    }

    /// Human-readable name of this intended use.
    #[getter]
    fn name(&self) -> &'static str {
        self.inner.label()
    }

    fn __repr__(&self) -> String {
        format!("IntendedUse({})", self.name())
    }

    fn __str__(&self) -> String {
        self.name().to_string()
    }
}

/// Operation capability declaration.
///
/// Used by [`OperationDescriptor`] to declare what a tool needs, and by
/// enforcement to check whether the caller profile permits that capability.
#[pyclass(frozen, name = "Capability")]
#[derive(Clone)]
pub struct CapabilityPy {
    inner: eggsec::config::Capability,
}

#[pymethods]
impl CapabilityPy {
    #[staticmethod]
    fn passive_fingerprint() -> Self {
        Self {
            inner: eggsec::config::Capability::PassiveFingerprint,
        }
    }

    #[staticmethod]
    fn active_probe() -> Self {
        Self {
            inner: eggsec::config::Capability::ActiveProbe,
        }
    }

    #[staticmethod]
    fn crawl() -> Self {
        Self {
            inner: eggsec::config::Capability::Crawl,
        }
    }

    #[staticmethod]
    fn http_fuzz_low_impact() -> Self {
        Self {
            inner: eggsec::config::Capability::HttpFuzzLowImpact,
        }
    }

    #[staticmethod]
    fn intrusive_fuzz() -> Self {
        Self {
            inner: eggsec::config::Capability::IntrusiveFuzz,
        }
    }

    #[staticmethod]
    fn waf_detect() -> Self {
        Self {
            inner: eggsec::config::Capability::WafDetect,
        }
    }

    #[staticmethod]
    fn waf_bypass_simulation() -> Self {
        Self {
            inner: eggsec::config::Capability::WafBypassSimulation,
        }
    }

    #[staticmethod]
    fn waf_stress_test() -> Self {
        Self {
            inner: eggsec::config::Capability::WafStressTest,
        }
    }

    #[staticmethod]
    fn load_test() -> Self {
        Self {
            inner: eggsec::config::Capability::LoadTest,
        }
    }

    #[staticmethod]
    fn raw_packet_probe() -> Self {
        Self {
            inner: eggsec::config::Capability::RawPacketProbe,
        }
    }

    #[staticmethod]
    fn credential_testing() -> Self {
        Self {
            inner: eggsec::config::Capability::CredentialTesting,
        }
    }

    #[staticmethod]
    fn remote_execution() -> Self {
        Self {
            inner: eggsec::config::Capability::RemoteExecution,
        }
    }

    #[staticmethod]
    fn nse_safe() -> Self {
        Self {
            inner: eggsec::config::Capability::NseSafe,
        }
    }

    #[staticmethod]
    fn nse_intrusive() -> Self {
        Self {
            inner: eggsec::config::Capability::NseIntrusive,
        }
    }

    #[staticmethod]
    fn traffic_interception() -> Self {
        Self {
            inner: eggsec::config::Capability::TrafficInterception,
        }
    }

    #[staticmethod]
    fn evasion_testing() -> Self {
        Self {
            inner: eggsec::config::Capability::EvasionTesting,
        }
    }

    #[staticmethod]
    fn database_assessment() -> Self {
        Self {
            inner: eggsec::config::Capability::DatabaseAssessment,
        }
    }

    #[staticmethod]
    fn c2_simulation() -> Self {
        Self {
            inner: eggsec::config::Capability::C2Simulation,
        }
    }

    #[staticmethod]
    fn mobile_dynamic_analysis() -> Self {
        Self {
            inner: eggsec::config::Capability::MobileDynamicAnalysis,
        }
    }

    /// Human-readable name of this capability.
    #[getter]
    fn name(&self) -> String {
        format!("{}", self.inner)
    }

    fn __repr__(&self) -> String {
        format!("Capability({})", self.name())
    }

    fn __str__(&self) -> String {
        self.name()
    }
}

/// Classification of why an operation was denied.
///
/// Used by enforcement to determine whether a denial can be downgraded
/// to a warning in permissive profiles.
#[pyclass(frozen, name = "DenialClass")]
#[derive(Clone)]
pub struct DenialClassPy {
    inner: eggsec::config::DenialClass,
}

#[pymethods]
impl DenialClassPy {
    #[staticmethod]
    fn scope_missing() -> Self {
        Self {
            inner: eggsec::config::DenialClass::ScopeMissing,
        }
    }

    #[staticmethod]
    fn target_out_of_scope() -> Self {
        Self {
            inner: eggsec::config::DenialClass::TargetOutOfScope,
        }
    }

    #[staticmethod]
    fn explicit_exclusion() -> Self {
        Self {
            inner: eggsec::config::DenialClass::ExplicitExclusion,
        }
    }

    #[staticmethod]
    fn feature_missing() -> Self {
        Self {
            inner: eggsec::config::DenialClass::FeatureMissing,
        }
    }

    #[staticmethod]
    fn risk_policy_denied() -> Self {
        Self {
            inner: eggsec::config::DenialClass::RiskPolicyDenied,
        }
    }

    #[staticmethod]
    fn capability_denied() -> Self {
        Self {
            inner: eggsec::config::DenialClass::CapabilityDenied,
        }
    }

    #[staticmethod]
    fn invalid_target() -> Self {
        Self {
            inner: eggsec::config::DenialClass::InvalidTarget,
        }
    }

    #[staticmethod]
    fn unknown() -> Self {
        Self {
            inner: eggsec::config::DenialClass::Unknown,
        }
    }

    /// Human-readable name of this denial class.
    #[getter]
    fn name(&self) -> String {
        format!("{}", self.inner)
    }

    fn __repr__(&self) -> String {
        format!("DenialClass({})", self.name())
    }

    fn __str__(&self) -> String {
        self.name()
    }
}

/// Target policy requirement for operation metadata.
#[pyclass(frozen, name = "TargetPolicyKind")]
#[derive(Clone)]
pub struct TargetPolicyKindPy {
    inner: eggsec::config::TargetPolicyKind,
}

#[pymethods]
impl TargetPolicyKindPy {
    #[staticmethod]
    fn no_target() -> Self {
        Self {
            inner: eggsec::config::TargetPolicyKind::NoTarget,
        }
    }

    #[staticmethod]
    fn optional_target() -> Self {
        Self {
            inner: eggsec::config::TargetPolicyKind::OptionalTarget,
        }
    }

    #[staticmethod]
    fn target_required() -> Self {
        Self {
            inner: eggsec::config::TargetPolicyKind::TargetRequired,
        }
    }

    #[staticmethod]
    fn explicit_scope_required() -> Self {
        Self {
            inner: eggsec::config::TargetPolicyKind::ExplicitScopeRequired,
        }
    }

    #[staticmethod]
    fn private_or_local_required() -> Self {
        Self {
            inner: eggsec::config::TargetPolicyKind::PrivateOrLocalRequired,
        }
    }

    /// Human-readable name of this target policy.
    #[getter]
    fn name(&self) -> String {
        match self.inner {
            eggsec::config::TargetPolicyKind::NoTarget => "no-target",
            eggsec::config::TargetPolicyKind::OptionalTarget => "optional-target",
            eggsec::config::TargetPolicyKind::TargetRequired => "target-required",
            eggsec::config::TargetPolicyKind::ExplicitScopeRequired => "explicit-scope-required",
            eggsec::config::TargetPolicyKind::PrivateOrLocalRequired => "private-or-local-required",
        }
        .to_string()
    }

    fn __repr__(&self) -> String {
        format!("TargetPolicyKind({})", self.name())
    }

    fn __str__(&self) -> String {
        self.name()
    }
}

/// Descriptor for an operation that can be evaluated against policy and scope.
///
/// Bundles the metadata needed to produce a policy decision.
#[pyclass(frozen, name = "OperationDescriptor")]
#[derive(Clone)]
pub struct OperationDescriptorPy {
    inner: eggsec::config::OperationDescriptor,
}

#[pymethods]
impl OperationDescriptorPy {
    /// Unique operation identifier (e.g. "scan-ports", "fuzz").
    #[getter]
    fn operation_id(&self) -> String {
        self.inner.operation.clone()
    }

    /// Human-readable operation name.
    #[getter]
    fn operation_name(&self) -> String {
        self.inner.operation.clone()
    }

    /// Operating mode.
    #[getter]
    fn mode(&self) -> OperationModePy {
        OperationModePy {
            inner: self.inner.mode,
        }
    }

    /// Risk tier.
    #[getter]
    fn risk(&self) -> OperationRiskPy {
        OperationRiskPy {
            inner: self.inner.risk,
        }
    }

    /// Intended use cases.
    #[getter]
    fn intended_uses(&self) -> Vec<IntendedUsePy> {
        self.inner
            .intended_uses
            .iter()
            .map(|u| IntendedUsePy { inner: *u })
            .collect()
    }

    /// Original target string (hostname, URL, or IP).
    #[getter]
    fn target(&self) -> Option<String> {
        self.inner.target.clone()
    }

    /// Feature flags required to execute this operation.
    #[getter]
    fn required_features(&self) -> Vec<String> {
        self.inner.required_features.clone()
    }

    /// Capabilities required by this operation.
    #[getter]
    fn required_capabilities(&self) -> Vec<CapabilityPy> {
        self.inner
            .required_capabilities
            .iter()
            .map(|c| CapabilityPy { inner: *c })
            .collect()
    }

    /// Capabilities denied by this operation.
    #[getter]
    fn denied_capabilities(&self) -> Vec<CapabilityPy> {
        Vec::new()
    }

    /// Target policy kind.
    #[getter]
    fn target_policy(&self) -> TargetPolicyKindPy {
        let inner = if self.inner.requires_private_or_local_target {
            eggsec::config::TargetPolicyKind::PrivateOrLocalRequired
        } else if self.inner.requires_explicit_scope {
            eggsec::config::TargetPolicyKind::ExplicitScopeRequired
        } else if self.inner.target.is_some() {
            eggsec::config::TargetPolicyKind::TargetRequired
        } else {
            eggsec::config::TargetPolicyKind::NoTarget
        };
        TargetPolicyKindPy { inner }
    }

    fn __repr__(&self) -> String {
        format!(
            "OperationDescriptor(id={}, risk={}, target={:?})",
            self.inner.operation, self.inner.risk, self.inner.target
        )
    }
}

/// View into a static operation metadata entry.
///
/// Read-only wrapper over [`OperationMetadata`] that exposes fields as
/// Python properties and provides a method to generate descriptors.
#[pyclass(frozen, name = "OperationMetadataView")]
#[derive(Clone)]
pub struct OperationMetadataViewPy {
    operation_id: String,
    operation_name: String,
    default_risk: OperationRiskPy,
    default_mode: OperationModePy,
    target_policy: TargetPolicyKindPy,
}

#[pymethods]
impl OperationMetadataViewPy {
    /// Unique operation identifier.
    #[getter]
    fn operation_id(&self) -> String {
        self.operation_id.clone()
    }

    /// Human-readable operation name.
    #[getter]
    fn operation_name(&self) -> String {
        self.operation_name.clone()
    }

    /// Default risk tier for this operation.
    #[getter]
    fn default_risk(&self) -> OperationRiskPy {
        self.default_risk.clone()
    }

    /// Default operating mode for this operation.
    #[getter]
    fn default_mode(&self) -> OperationModePy {
        self.default_mode.clone()
    }

    /// Target policy requirement.
    #[getter]
    fn target_policy(&self) -> TargetPolicyKindPy {
        self.target_policy.clone()
    }

    /// Generate an [`OperationDescriptor`] for a specific target.
    ///
    /// Args:
    ///     target: Optional target string (hostname, URL, or IP).
    ///
    /// Returns:
    ///     OperationDescriptor: The generated descriptor.
    #[pyo3(signature = (target=None))]
    fn descriptor_for_target(&self, target: Option<&str>) -> PyResult<OperationDescriptorPy> {
        let meta = eggsec::config::operation_metadata(&self.operation_id).ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Operation '{}' not found in metadata registry",
                self.operation_id
            ))
        })?;
        let descriptor = meta.descriptor_for_target(target.map(|s| s.to_string()));
        Ok(OperationDescriptorPy { inner: descriptor })
    }

    fn __repr__(&self) -> String {
        format!(
            "OperationMetadataView(id={}, name={:?})",
            self.operation_id, self.operation_name
        )
    }
}

/// Registry of all operation metadata.
///
/// Provides static methods to query the canonical operation metadata registry.
#[pyclass]
pub struct OperationRegistry;

#[pymethods]
impl OperationRegistry {
    /// Get metadata for all registered operations.
    ///
    /// Returns:
    ///     list[OperationMetadataView]: All operation metadata entries.
    #[staticmethod]
    fn all_operations() -> Vec<OperationMetadataViewPy> {
        eggsec::config::all_operation_metadata()
            .iter()
            .map(|m| OperationMetadataViewPy {
                operation_id: m.id.to_string(),
                operation_name: m.display_name.to_string(),
                default_risk: OperationRiskPy { inner: m.risk },
                default_mode: OperationModePy { inner: m.mode },
                target_policy: TargetPolicyKindPy {
                    inner: m.target_policy,
                },
            })
            .collect()
    }

    /// Find metadata for a specific operation by ID.
    ///
    /// Args:
    ///     operation_id: The canonical operation ID (e.g. "scan-ports", "fuzz").
    ///
    /// Returns:
    ///     OperationMetadataView | None: The metadata entry, or None if not found.
    #[staticmethod]
    fn find(operation_id: &str) -> Option<OperationMetadataViewPy> {
        let m = eggsec::config::operation_metadata(operation_id)?;
        Some(OperationMetadataViewPy {
            operation_id: m.id.to_string(),
            operation_name: m.display_name.to_string(),
            default_risk: OperationRiskPy { inner: m.risk },
            default_mode: OperationModePy { inner: m.mode },
            target_policy: TargetPolicyKindPy {
                inner: m.target_policy,
            },
        })
    }

    /// Find metadata by tool ID, resolving aliases to canonical IDs.
    ///
    /// Args:
    ///     tool_id: Tool ID which may be an alias (e.g. "scan", "waf").
    ///
    /// Returns:
    ///     OperationMetadataView | None: The metadata entry, or None if not found.
    #[staticmethod]
    fn find_by_tool_id(tool_id: &str) -> Option<OperationMetadataViewPy> {
        let m = eggsec::config::metadata_for_tool_id(tool_id)?;
        Some(OperationMetadataViewPy {
            operation_id: m.id.to_string(),
            operation_name: m.display_name.to_string(),
            default_risk: OperationRiskPy { inner: m.risk },
            default_mode: OperationModePy { inner: m.mode },
            target_policy: TargetPolicyKindPy {
                inner: m.target_policy,
            },
        })
    }

    fn __repr__(&self) -> String {
        "OperationRegistry".to_string()
    }
}
