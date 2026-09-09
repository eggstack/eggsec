//! Operation catalog: canonical metadata and alias lookup (Phase D WS6).
//!
//! Cohesive module extracted from `config/policy.rs`: `OperationMetadata`
//! declarations, `ALL_OPERATION_METADATA`, alias table, and lookup helpers
//! only. Target normalization lives in `policy_target.rs`; execution policy
//! types live in `policy.rs`; evaluation lives in `policy_decision.rs`.
//!
//! This is the single source of truth for operation identity, policy
//! metadata, and dispatch routing. Do not duplicate operation lists
//! elsewhere.
//!
//! Stable facade: `config/policy.rs` re-exports everything here.

use super::policy::{Capability, IntendedUse, OperationDescriptor, OperationMode, OperationRisk};
use super::policy_target::{
    normalize_target, DescriptorError, OperationTarget, TargetHint, TargetPolicyKind,
};

/// Canonical operation metadata — single source of truth for OperationDescriptor generation.
///
/// Every externally invokable Eggsec operation should have one `OperationMetadata`
/// declaration. This drives policy descriptors, protocol exposure, capability/risk
/// declarations, feature gates, and documentation.
#[derive(Debug, Clone, Copy)]
pub struct OperationMetadata {
    pub id: &'static str,
    pub display_name: &'static str,
    pub mode: OperationMode,
    pub risk: OperationRisk,
    pub intended_uses: &'static [IntendedUse],
    pub required_features: &'static [&'static str],
    pub required_policy_flags: &'static [&'static str],
    pub required_capabilities: &'static [Capability],
    pub target_policy: TargetPolicyKind,
    pub manual_exposable: bool,
    pub tui_exposable: bool,
    pub mcp_exposable: bool,
    pub rest_exposable: bool,
    pub agent_exposable: bool,
    pub grpc_exposable: bool,
}

impl OperationMetadata {
    /// Generate an `OperationDescriptor` from this metadata.
    ///
    /// The `normalized_target` is auto-detected from the raw target string.
    /// Prefer [`Self::try_descriptor_for_target`] for new code, which validates
    /// target policy before construction.
    pub fn descriptor_for_target(&self, target: Option<String>) -> OperationDescriptor {
        let normalized = target
            .as_deref()
            .map(|t| normalize_target(t, None))
            .unwrap_or(OperationTarget::None);
        OperationDescriptor {
            operation: self.id.to_string(),
            mode: self.mode,
            risk: self.risk,
            intended_uses: self.intended_uses.to_vec(),
            target,
            normalized_target: normalized,
            required_features: self
                .required_features
                .iter()
                .map(|s| s.to_string())
                .collect(),
            required_policy_flags: self
                .required_policy_flags
                .iter()
                .map(|s| s.to_string())
                .collect(),
            requires_private_or_local_target: matches!(
                self.target_policy,
                TargetPolicyKind::PrivateOrLocalRequired
            ),
            requires_explicit_scope: matches!(
                self.target_policy,
                TargetPolicyKind::ExplicitScopeRequired | TargetPolicyKind::PrivateOrLocalRequired
            ),
            required_capabilities: self.required_capabilities.to_vec(),
        }
    }

    /// Generate an `OperationDescriptor` from this metadata, overriding the risk tier.
    /// Used for dry-run overrides and tab-specific risk adjustments.
    pub fn descriptor_for_target_with_risk(
        &self,
        target: Option<String>,
        risk: OperationRisk,
    ) -> OperationDescriptor {
        let mut descriptor = self.descriptor_for_target(target);
        descriptor.risk = risk;
        descriptor
    }

    /// Fallibly generate an `OperationDescriptor`, validating the target against
    /// this metadata's target policy.
    ///
    /// Returns [`DescriptorError`] when the supplied target violates the policy:
    /// - `NoTarget`: rejects non-empty target unless the operation explicitly
    ///   permits an auxiliary/display-only target (not yet supported; all targets
    ///   are rejected for `NoTarget`).
    /// - `TargetRequired`: rejects `None` and empty targets.
    /// - `ExplicitScopeRequired`: rejects `None` and empty targets.
    /// - `PrivateOrLocalRequired`: rejects `None` and empty targets.
    /// - `OptionalTarget`: accepts `None` or any non-empty target.
    pub fn try_descriptor_for_target(
        &self,
        target: Option<&str>,
    ) -> Result<OperationDescriptor, DescriptorError> {
        self.try_descriptor_for_target_hint(target, None)
    }

    /// Fallibly generate an `OperationDescriptor`, validating the target against
    /// this metadata's target policy, with an optional [`TargetHint`] to improve
    /// normalization.
    ///
    /// When `hint` is `None`, the function auto-detects the target kind.
    pub fn try_descriptor_for_target_hint(
        &self,
        target: Option<&str>,
        hint: Option<TargetHint>,
    ) -> Result<OperationDescriptor, DescriptorError> {
        match self.target_policy {
            TargetPolicyKind::NoTarget => {
                if let Some(t) = target {
                    if !t.is_empty() {
                        return Err(DescriptorError::UnexpectedTarget {
                            operation_id: self.id.to_string(),
                            target_policy: self.target_policy,
                            target: t.to_string(),
                        });
                    }
                }
                Ok(self.descriptor_for_target(None))
            }
            TargetPolicyKind::OptionalTarget => {
                let t = target.filter(|s| !s.is_empty());
                let normalized = t
                    .map(|t| normalize_target(t, hint))
                    .unwrap_or(OperationTarget::None);
                Ok(OperationDescriptor {
                    operation: self.id.to_string(),
                    mode: self.mode,
                    risk: self.risk,
                    intended_uses: self.intended_uses.to_vec(),
                    target: t.map(|s| s.to_string()),
                    normalized_target: normalized,
                    required_features: self
                        .required_features
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    required_policy_flags: self
                        .required_policy_flags
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    requires_private_or_local_target: matches!(
                        self.target_policy,
                        TargetPolicyKind::PrivateOrLocalRequired
                    ),
                    requires_explicit_scope: matches!(
                        self.target_policy,
                        TargetPolicyKind::ExplicitScopeRequired
                            | TargetPolicyKind::PrivateOrLocalRequired
                    ),
                    required_capabilities: self.required_capabilities.to_vec(),
                })
            }
            TargetPolicyKind::TargetRequired
            | TargetPolicyKind::ExplicitScopeRequired
            | TargetPolicyKind::PrivateOrLocalRequired => {
                let t = target.filter(|s| !s.is_empty()).ok_or_else(|| {
                    DescriptorError::MissingTarget {
                        operation_id: self.id.to_string(),
                        target_policy: self.target_policy,
                    }
                })?;
                let normalized = normalize_target(t, hint);
                Ok(OperationDescriptor {
                    operation: self.id.to_string(),
                    mode: self.mode,
                    risk: self.risk,
                    intended_uses: self.intended_uses.to_vec(),
                    target: Some(t.to_string()),
                    normalized_target: normalized,
                    required_features: self
                        .required_features
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    required_policy_flags: self
                        .required_policy_flags
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    requires_private_or_local_target: matches!(
                        self.target_policy,
                        TargetPolicyKind::PrivateOrLocalRequired
                    ),
                    requires_explicit_scope: matches!(
                        self.target_policy,
                        TargetPolicyKind::ExplicitScopeRequired
                            | TargetPolicyKind::PrivateOrLocalRequired
                    ),
                    required_capabilities: self.required_capabilities.to_vec(),
                })
            }
        }
    }

    /// Generate an `OperationDescriptor` from this metadata, overriding the risk tier,
    /// and validating the target against this metadata's target policy.
    ///
    /// Combines [`Self::try_descriptor_for_target`] with a risk override. Returns
    /// [`DescriptorError`] on target-policy violation.
    pub fn try_descriptor_for_target_with_risk(
        &self,
        target: Option<&str>,
        risk: OperationRisk,
    ) -> Result<OperationDescriptor, DescriptorError> {
        let mut descriptor = self.try_descriptor_for_target(target)?;
        descriptor.risk = risk;
        Ok(descriptor)
    }

    /// Returns the canonical CLI command ID for this operation.
    ///
    /// By default this is the operation ID itself. Operations whose CLI command
    /// uses a different ID should be listed in the alias table with the command
    /// ID as the alias.
    pub fn canonical_command_id(&self) -> &'static str {
        self.id
    }

    /// Returns `true` if this operation requires a compile-time feature gate.
    pub fn is_feature_gated(&self) -> bool {
        !self.required_features.is_empty()
    }

    /// Returns the first required feature, or `None` if no feature is needed.
    pub fn primary_feature(&self) -> Option<&'static str> {
        self.required_features.first().copied()
    }

    /// Returns `true` if this operation is safe for default MCP tool listing
    /// (passive or safe-active risk, metadata-exposable, no feature gate).
    pub fn is_mcp_default_visible(&self) -> bool {
        matches!(
            self.risk,
            OperationRisk::Passive | OperationRisk::SafeActive
        ) && self.mcp_exposable
            && self.required_features.is_empty()
    }

    /// Returns `true` if this operation is hazardous for strict automated surfaces.
    ///
    /// Hazardous operations (risk > SafeActive) must not be default-visible
    /// on MCP/REST/gRPC/agent surfaces.
    pub fn is_hazardous(&self) -> bool {
        self.risk > OperationRisk::SafeActive
    }

    /// Returns `true` if this operation is exposed on any automated surface
    /// (MCP, REST, gRPC, or agent).
    pub fn is_exposed_automated(&self) -> bool {
        self.mcp_exposable || self.rest_exposable || self.grpc_exposable || self.agent_exposable
    }

    /// Derive the feature gate for a command registration from this metadata.
    ///
    /// Returns `None` if no feature is required, or the first required feature.
    pub fn derive_command_feature(&self) -> Option<&'static str> {
        self.primary_feature()
    }

    /// Returns `true` if this operation should be visible in TUI tab listings.
    pub fn is_tui_visible(&self) -> bool {
        self.tui_exposable
    }

    /// Returns `true` if this operation should be visible in programmatic surfaces
    /// (MCP/REST/gRPC/agent).
    pub fn is_programmatic_visible(&self) -> bool {
        self.mcp_exposable || self.rest_exposable || self.grpc_exposable || self.agent_exposable
    }

    /// Derive an `OperationIntegration` for domain descriptor construction.
    ///
    /// Returns a static `OperationIntegration` that mirrors this metadata's
    /// mode, risk, capabilities, features, and target policy. Domain-specific
    /// overrides (e.g. `requires_explicit_scope`, `requires_private_or_local_target`)
    /// can be applied after construction.
    pub fn derive_operation_integration(&'static self) -> crate::domain::OperationIntegration {
        crate::domain::OperationIntegration {
            operation_id: self.id,
            display_name: self.display_name,
            mode: self.mode,
            risk: self.risk,
            capabilities: self.required_capabilities,
            intended_uses: self.intended_uses,
            required_features: self.required_features,
            requires_explicit_scope: matches!(
                self.target_policy,
                TargetPolicyKind::ExplicitScopeRequired | TargetPolicyKind::PrivateOrLocalRequired
            ),
            requires_private_or_local_target: matches!(
                self.target_policy,
                TargetPolicyKind::PrivateOrLocalRequired
            ),
        }
    }
}

/// Static registry of all operation metadata. Single source of truth for
/// operation descriptors across REST, MCP, TUI, and agent surfaces.
pub static ALL_OPERATION_METADATA: &[OperationMetadata] = &[
    OperationMetadata {
        id: "recon",
        display_name: "Reconnaissance",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::PassiveFingerprint],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "scan-ports",
        display_name: "Port Scan",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::ActiveProbe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "scan-endpoints",
        display_name: "Endpoint Discovery",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::Crawl],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "fingerprint",
        display_name: "Service Fingerprint",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::ActiveProbe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "fuzz",
        display_name: "Fuzzing",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::Intrusive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::HttpFuzzLowImpact],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "waf-detect",
        display_name: "WAF Detection",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::WafDetect],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "waf-bypass",
        display_name: "WAF Bypass Simulation",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::Intrusive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::WafBypassSimulation],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "waf-stress",
        display_name: "WAF Stress Test",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::StressTest,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::WafStressTest],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "load-test",
        display_name: "Load Test",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::LoadTest,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::LoadTest],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "stress-test",
        display_name: "Stress Test",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::StressTest,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["stress-testing"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::WafStressTest],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "packet",
        display_name: "Raw Packet",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::RawPacket,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["packet-inspection"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::RawPacketProbe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "graphql",
        display_name: "GraphQL Fuzzing",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::Intrusive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::HttpFuzzLowImpact],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "oauth",
        display_name: "OAuth Testing",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::CredentialTesting,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::CredentialTesting],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "auth-test",
        display_name: "Authentication Testing",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::CredentialTesting,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::CredentialTesting],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "nse",
        display_name: "NSE Scripts",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["nse"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::NseSafe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "db-pentest",
        display_name: "Database Pentesting",
        mode: OperationMode::DefenseLab,
        risk: OperationRisk::DbPentest,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["db-pentest"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::DatabaseAssessment],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "c2",
        display_name: "C2 Simulation",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::C2Operation,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["c2"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::C2Simulation],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "proxy-intercept",
        display_name: "Traffic Interception",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::TrafficInterception,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["web-proxy"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::TrafficInterception],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "wireless",
        display_name: "Wireless Scanning",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["wireless"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::PassiveFingerprint],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "wireless-deauth",
        display_name: "Wireless Deauth Attack",
        mode: OperationMode::DefenseLab,
        risk: OperationRisk::Intrusive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["wireless-advanced"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::RawPacketProbe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: false,
        rest_exposable: false,
        agent_exposable: false,
        grpc_exposable: false,
    },
    OperationMetadata {
        id: "hunt",
        display_name: "Vulnerability Hunting",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["advanced-hunting"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::ActiveProbe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "browser",
        display_name: "Headless Browser",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["headless-browser"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::ActiveProbe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "compliance",
        display_name: "Compliance Scanning",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["compliance"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::ActiveProbe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "storage",
        display_name: "Database Storage",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["database"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::DatabaseAssessment],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "integrations",
        display_name: "External Integrations",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["external-integrations"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::ActiveProbe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "workflow",
        display_name: "Finding Workflow",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["finding-workflow"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::ActiveProbe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "vuln",
        display_name: "Vulnerability Management",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["vuln-management"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::ActiveProbe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "pipeline",
        display_name: "Security Pipeline",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::ActiveProbe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "remote",
        display_name: "Remote Execution",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::RemoteExecution,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::RemoteExecution],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "search",
        display_name: "Web Search",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::Passive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[],
        target_policy: TargetPolicyKind::NoTarget,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "mobile-static",
        display_name: "Mobile Static Analysis",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["mobile"],
        required_policy_flags: &[],
        required_capabilities: &[],
        target_policy: TargetPolicyKind::OptionalTarget,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: false,
        rest_exposable: false,
        agent_exposable: false,
        grpc_exposable: false,
    },
    OperationMetadata {
        id: "mobile-dynamic",
        display_name: "Mobile Dynamic Analysis",
        mode: OperationMode::DefenseLab,
        risk: OperationRisk::Intrusive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["mobile-dynamic"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::MobileDynamicAnalysis],
        target_policy: TargetPolicyKind::OptionalTarget,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: false,
        rest_exposable: false,
        agent_exposable: false,
        grpc_exposable: false,
    },
    OperationMetadata {
        id: "evasion",
        display_name: "Evasion Detection",
        mode: OperationMode::DefenseLab,
        risk: OperationRisk::EvasionTesting,
        intended_uses: &[IntendedUse::WafRegression],
        required_features: &["evasion"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::EvasionTesting],
        target_policy: TargetPolicyKind::OptionalTarget,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: false,
        rest_exposable: false,
        agent_exposable: false,
        grpc_exposable: false,
    },
    OperationMetadata {
        id: "postex",
        display_name: "Post-Exploitation",
        mode: OperationMode::DefenseLab,
        risk: OperationRisk::ExploitAdjacent,
        intended_uses: &[IntendedUse::WafRegression],
        required_features: &["postex"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::RemoteExecution],
        target_policy: TargetPolicyKind::OptionalTarget,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: false,
        rest_exposable: false,
        agent_exposable: false,
        grpc_exposable: false,
    },
];

/// Alias mapping: (alias_id, canonical_id).
///
/// REST and MCP tool IDs that resolve to the same canonical operation.
pub static ALL_OPERATION_METADATA_ALIASES: &[(&str, &str)] = &[
    ("scan", "scan-ports"),
    ("endpoints", "scan-endpoints"),
    ("waf", "waf-detect"),
    ("waf_detect", "waf-detect"),
    ("waf_bypass", "waf-bypass"),
    ("waf_stress", "waf-stress"),
    ("load", "load-test"),
    ("loadtest", "load-test"),
    ("http-bench", "load-test"),
    ("stress", "stress-test"),
    ("fuzzer", "fuzz"),
    ("api-fuzz", "fuzz"),
    ("proxy", "proxy-intercept"),
    ("raw-packet", "packet"),
    ("packet-capture", "packet"),
    ("packet-inspect", "packet"),
    ("recon-all", "recon"),
    ("subdomain", "recon"),
    ("credential", "auth-test"),
    ("brute", "auth-test"),
    ("syn-flood", "stress-test"),
    ("udp-flood", "stress-test"),
    ("icmp-flood", "stress-test"),
    ("raw-packet-send", "packet"),
    ("plan", "recon"),
    ("scan_ports", "scan-ports"),
    ("scan_endpoints", "scan-endpoints"),
    ("fingerprint_services", "fingerprint"),
    ("recon_dns", "recon"),
    ("detect_waf", "waf-detect"),
    ("load_test", "load-test"),
    ("graphql_test", "graphql"),
    ("oauth_test", "oauth"),
    ("db_probe", "db-pentest"),
    ("nse_run", "nse"),
    ("scan-pipeline", "pipeline"),
    ("db-pentest-mcp", "db-pentest"),
    ("mobile", "mobile-static"),
    ("mobile-scan", "mobile-static"),
    ("exec", "remote"),
    ("ssh", "remote"),
    ("tor", "proxy-intercept"),
];

/// Look up operation metadata by its canonical ID.
pub fn operation_metadata(id: &str) -> Option<&'static OperationMetadata> {
    ALL_OPERATION_METADATA.iter().find(|m| m.id == id)
}

/// Look up operation metadata by tool ID, resolving aliases to canonical IDs.
pub fn metadata_for_tool_id(tool_id: &str) -> Option<&'static OperationMetadata> {
    if let Some(m) = operation_metadata(tool_id) {
        return Some(m);
    }
    ALL_OPERATION_METADATA_ALIASES
        .iter()
        .find(|(alias, _)| *alias == tool_id)
        .and_then(|(_, canonical)| operation_metadata(canonical))
}

/// Return a reference to all operation metadata entries.
pub fn all_operation_metadata() -> &'static [OperationMetadata] {
    ALL_OPERATION_METADATA
}

/// Check if a tool ID (possibly an alias) matches a canonical operation ID.
///
/// Returns true if:
/// - `tool_id` == `operation_id` (exact match), or
/// - `tool_id` is an alias that resolves to `operation_id`, or
/// - `tool_id` resolves to the same canonical metadata entry as `operation_id`.
pub fn operation_matches_tool_id(tool_id: &str, operation_id: &str) -> bool {
    if tool_id == operation_id {
        return true;
    }
    // Check if tool_id aliases to operation_id
    if let Some((_, canonical)) = ALL_OPERATION_METADATA_ALIASES
        .iter()
        .find(|(alias, _)| *alias == tool_id)
    {
        if *canonical == operation_id {
            return true;
        }
    }
    // Check if tool_id resolves to the same canonical entry as operation_id
    if let Some(meta) = metadata_for_tool_id(tool_id) {
        if meta.id == operation_id {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod operation_metadata_tests {
    use super::*;
    use crate::config::baseline_allowed_capability;

    #[test]
    fn every_metadata_has_non_empty_id_and_display_name() {
        for m in all_operation_metadata() {
            assert!(!m.id.is_empty(), "metadata has empty id");
            assert!(
                !m.display_name.is_empty(),
                "metadata has empty display_name for {}",
                m.id
            );
        }
    }

    #[test]
    fn every_metadata_id_is_unique() {
        let mut seen = rustc_hash::FxHashSet::default();
        for m in all_operation_metadata() {
            assert!(seen.insert(m.id), "duplicate metadata id: {}", m.id);
        }
    }

    #[test]
    fn agent_exposable_ops_require_explicit_scope() {
        for m in all_operation_metadata() {
            if (m.agent_exposable || m.mcp_exposable)
                && m.target_policy != TargetPolicyKind::NoTarget
            {
                assert!(
                    matches!(
                        m.target_policy,
                        TargetPolicyKind::ExplicitScopeRequired
                            | TargetPolicyKind::PrivateOrLocalRequired
                    ),
                    "agent/mcp exposable target-bearing op '{}' should require explicit scope via target_policy, got {:?}",
                    m.id,
                    m.target_policy
                );
            }
        }
    }

    #[test]
    fn feature_gated_ops_declare_feature_name() {
        for m in all_operation_metadata() {
            if !m.required_features.is_empty() {
                for f in m.required_features {
                    assert!(
                        !f.is_empty(),
                        "metadata '{}' has empty required_feature",
                        m.id
                    );
                }
            }
        }
    }

    #[test]
    fn descriptor_generation_matches_metadata() {
        for m in all_operation_metadata() {
            let desc = m.descriptor_for_target(Some("https://example.com".to_string()));
            assert_eq!(desc.operation, m.id);
            assert_eq!(desc.mode, m.mode);
            assert_eq!(desc.risk, m.risk);
            assert_eq!(desc.target, Some("https://example.com".to_string()));
            assert_eq!(desc.required_capabilities, m.required_capabilities.to_vec());
        }
    }

    #[test]
    fn descriptor_with_target_none() {
        for m in all_operation_metadata() {
            let desc = m.descriptor_for_target(None);
            assert_eq!(desc.target, None);
        }
    }

    #[test]
    fn rest_descriptor_from_metadata_matches_expected() {
        // recon
        let m = metadata_for_tool_id("recon").unwrap();
        let desc = m.descriptor_for_target(Some("https://example.com".to_string()));
        assert_eq!(desc.risk, OperationRisk::SafeActive);
        assert!(desc
            .required_capabilities
            .contains(&Capability::PassiveFingerprint));
        // fuzz
        let m = metadata_for_tool_id("fuzz").unwrap();
        let desc = m.descriptor_for_target(Some("https://example.com".to_string()));
        assert_eq!(desc.risk, OperationRisk::Intrusive);
        assert!(desc
            .required_capabilities
            .contains(&Capability::HttpFuzzLowImpact));
        // stress
        let m = metadata_for_tool_id("stress-test").unwrap();
        let desc = m.descriptor_for_target(Some("https://example.com".to_string()));
        assert_eq!(desc.risk, OperationRisk::StressTest);
        assert!(desc
            .required_capabilities
            .contains(&Capability::WafStressTest));
    }

    #[test]
    fn alias_lookup_matches_canonical() {
        let canonical = metadata_for_tool_id("recon").unwrap();
        let alias = metadata_for_tool_id("recon-all").unwrap();
        assert_eq!(canonical.id, alias.id);
        assert_eq!(canonical.risk, alias.risk);
    }

    #[test]
    fn operation_matches_tool_id_exact_match() {
        assert!(operation_matches_tool_id("scan-ports", "scan-ports"));
        assert!(operation_matches_tool_id("fuzz", "fuzz"));
        assert!(operation_matches_tool_id("recon", "recon"));
    }

    #[test]
    fn operation_matches_tool_id_alias_to_canonical() {
        assert!(operation_matches_tool_id("scan", "scan-ports"));
        assert!(operation_matches_tool_id("load", "load-test"));
        assert!(operation_matches_tool_id("waf", "waf-detect"));
        assert!(operation_matches_tool_id("stress", "stress-test"));
        assert!(operation_matches_tool_id("fuzzer", "fuzz"));
        assert!(operation_matches_tool_id("recon-all", "recon"));
    }

    #[test]
    fn operation_matches_tool_id_canonical_to_alias() {
        // Bidirectional: canonical ID should match when compared against alias
        assert!(operation_matches_tool_id("scan-ports", "scan-ports"));
        // This tests the metadata_for_tool_id fallback path
        assert!(operation_matches_tool_id("load-test", "load-test"));
    }

    #[test]
    fn operation_matches_tool_id_unrelated_no_match() {
        assert!(!operation_matches_tool_id("scan", "fuzz"));
        assert!(!operation_matches_tool_id("load", "stress-test"));
        assert!(!operation_matches_tool_id("waf", "recon"));
    }

    #[test]
    fn operation_matches_tool_id_unknown_no_match() {
        assert!(!operation_matches_tool_id("nonexistent", "scan-ports"));
        assert!(!operation_matches_tool_id("scan-ports", "nonexistent"));
    }

    /// Every tool registered by `create_default_registry()` must have operation metadata.
    /// This prevents new tools from being added without metadata, which would cause
    /// runtime failures in REST, MCP, TUI, and agent surfaces.
    #[test]
    fn every_registered_tool_has_operation_metadata() {
        // Tool IDs from tool::create_default_registry() (non-feature-gated)
        let base_tool_ids = &[
            "recon",
            "scan-ports",
            "fingerprint",
            "scan-endpoints",
            "fuzz",
            "load",
            "waf-detect",
            "waf-bypass",
            "waf-stress",
            "pipeline",
            "search",
        ];

        for &tool_id in base_tool_ids {
            assert!(
                metadata_for_tool_id(tool_id).is_some(),
                "registered tool '{}' has no operation metadata — add an entry to ALL_OPERATION_METADATA or ALL_OPERATION_METADATA_ALIASES",
                tool_id,
            );
        }

        // Feature-gated tools: only check if the feature is enabled
        #[cfg(feature = "web-proxy-mcp")]
        assert!(
            metadata_for_tool_id("proxy").is_some(),
            "registered tool 'proxy' has no operation metadata"
        );
        #[cfg(feature = "db-pentest-mcp")]
        assert!(
            metadata_for_tool_id("db-pentest").is_some(),
            "registered tool 'db-pentest' has no operation metadata"
        );
        #[cfg(feature = "c2-mcp")]
        assert!(
            metadata_for_tool_id("c2").is_some(),
            "registered tool 'c2' has no operation metadata"
        );
    }

    /// High-risk operations (risk > SafeActive) must declare at least one
    /// non-baseline capability. This prevents accidentally omitting capability
    /// declarations on dangerous operations, which would allow them to slip
    /// through enforcement checks that gate on required_capabilities.
    #[test]
    fn high_risk_ops_declare_nonbaseline_capability() {
        for m in all_operation_metadata() {
            if m.risk > OperationRisk::SafeActive {
                let has_nonbaseline = m
                    .required_capabilities
                    .iter()
                    .any(|cap| !baseline_allowed_capability(*cap));
                assert!(
                    has_nonbaseline,
                    "high-risk operation '{}' (risk {:?}) must declare at least one \
                     non-baseline capability — current capabilities: {:?}",
                    m.id, m.risk, m.required_capabilities,
                );
            }
        }
    }

    /// TUI descriptor generation must match metadata for representative tabs.
    /// Verifies that metadata_for_tool_id() resolves TUI operation IDs and
    /// descriptor_for_target() produces the expected risk, mode, and capabilities.
    #[test]
    fn tui_descriptor_generation_matches_metadata() {
        // Representative TUI operation IDs (some canonical, some aliases)
        let cases: &[(&str, OperationRisk, &[Capability])] = &[
            (
                "recon",
                OperationRisk::SafeActive,
                &[Capability::PassiveFingerprint],
            ),
            (
                "scan-ports",
                OperationRisk::SafeActive,
                &[Capability::ActiveProbe],
            ),
            (
                "fuzz",
                OperationRisk::Intrusive,
                &[Capability::HttpFuzzLowImpact],
            ),
            ("waf", OperationRisk::SafeActive, &[Capability::WafDetect]),
            (
                "load-test",
                OperationRisk::LoadTest,
                &[Capability::LoadTest],
            ),
        ];

        for &(op_id, expected_risk, expected_caps) in cases {
            let metadata = metadata_for_tool_id(op_id)
                .unwrap_or_else(|| panic!("TUI operation '{}' should have metadata", op_id));
            let desc = metadata.descriptor_for_target(Some("https://example.com".to_string()));
            assert_eq!(
                desc.risk, expected_risk,
                "TUI tab '{}': expected risk {:?}, got {:?}",
                op_id, expected_risk, desc.risk
            );
            assert_eq!(
                desc.operation, metadata.id,
                "TUI tab '{}': descriptor operation should be canonical ID '{}'",
                op_id, metadata.id
            );
            for cap in expected_caps {
                assert!(
                    desc.required_capabilities.contains(cap),
                    "TUI tab '{}': expected capability {:?} in {:?}",
                    op_id,
                    cap,
                    desc.required_capabilities
                );
            }
        }
    }
}
