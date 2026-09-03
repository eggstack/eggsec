use crate::agent::portfolio::ScanDepth;
use crate::config::OperationDescriptor;

pub(crate) fn operation_descriptor_for_agent_scan(
    target: &str,
    scan_type: &str,
    _depth: ScanDepth,
) -> Result<OperationDescriptor, crate::config::DescriptorError> {
    use crate::tool::metadata::metadata_for_tool_id;

    // Try to match scan_type to known metadata
    if let Some(metadata) = metadata_for_tool_id(scan_type) {
        // Fail closed: propagate target validation failures instead of
        // synthesizing a target-less descriptor that would lose binding.
        let mut descriptor = metadata.try_descriptor_for_target(Some(target))?;
        descriptor.requires_explicit_scope = true;
        return Ok(descriptor);
    }

    // Fail closed: unknown scan types have no registered OperationMetadata, and
    // OperationMetadata is the single source of truth for operation policy.
    // Synthesizing risk/capabilities from scan_type keywords would let crafted
    // strings bypass registry review, so unknown types are rejected here.
    // (ScanDepth no longer influences classification; it only fed the removed
    // keyword fallback. Callers already log this error with operation/target.)
    Err(crate::config::DescriptorError::UnknownOperation {
        operation_id: scan_type.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Capability, DescriptorError, OperationMode, OperationRisk};

    #[test]
    fn operation_descriptor_shallow_recon() {
        let desc =
            operation_descriptor_for_agent_scan("https://example.com", "recon", ScanDepth::Shallow)
                .expect("valid target should produce a descriptor");
        assert_eq!(desc.operation, "recon");
        assert_eq!(desc.risk, OperationRisk::SafeActive);
        assert!(desc.target.as_deref() == Some("https://example.com"));
        assert!(desc.requires_explicit_scope);
        assert_eq!(desc.mode, OperationMode::StandardAssessment);
        // Metadata is now the source of truth: recon uses PassiveFingerprint
        assert!(desc
            .required_capabilities
            .contains(&Capability::PassiveFingerprint));
    }

    #[test]
    fn operation_descriptor_unknown_scan_type_fails_closed() {
        // Unknown scan types have no registered metadata and must be rejected
        // instead of receiving keyword-synthesized risk/capabilities.
        for unknown in &["syn_stress", "stress_load", "totally-unknown-scan"] {
            for depth in &[ScanDepth::Shallow, ScanDepth::Deep] {
                let result =
                    operation_descriptor_for_agent_scan("https://target.com", unknown, *depth);
                match result {
                    Err(DescriptorError::UnknownOperation { operation_id }) => {
                        assert_eq!(&operation_id, unknown);
                    }
                    other => panic!(
                        "unknown scan type '{}' should fail closed with UnknownOperation, got {:?}",
                        unknown,
                        other.map(|d| d.operation.clone())
                    ),
                }
            }
        }
    }

    #[test]
    fn operation_descriptor_invalid_target_fails_closed() {
        // Empty target violates ExplicitScopeRequired for "recon": must Err
        // instead of falling back to a target-less descriptor.
        let result = operation_descriptor_for_agent_scan("", "recon", ScanDepth::Shallow);
        assert!(
            result.is_err(),
            "invalid target should fail closed, not synthesize a target-less descriptor"
        );
    }
}
