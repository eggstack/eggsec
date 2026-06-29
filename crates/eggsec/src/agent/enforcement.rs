use crate::agent::portfolio::ScanDepth;
use crate::config::{Capability, IntendedUse, OperationDescriptor, OperationMode, OperationRisk};

pub(crate) fn risk_for_agent_scan_depth(depth: ScanDepth, scan_type: &str) -> OperationRisk {
    let st = scan_type.to_ascii_lowercase();
    if st.contains("stress") || st.contains("syn") || st.contains("udp") || st.contains("icmp") {
        return OperationRisk::StressTest;
    }
    if st.contains("load") || st.contains("bench") {
        return OperationRisk::LoadTest;
    }
    if st.contains("packet") || st.contains("raw") {
        return OperationRisk::RawPacket;
    }
    if st.contains("credential") || st.contains("brute") || st.contains("auth") {
        return OperationRisk::CredentialTesting;
    }
    if st.contains("remote") || st.contains("exec") || st.contains("ssh") {
        return OperationRisk::RemoteExecution;
    }
    match depth {
        ScanDepth::Shallow => OperationRisk::SafeActive,
        ScanDepth::Deep => OperationRisk::Intrusive,
    }
}

pub(crate) fn capabilities_for_agent_scan(scan_type: &str, depth: ScanDepth) -> Vec<Capability> {
    let mut caps: Vec<Capability> = match depth {
        ScanDepth::Shallow => vec![Capability::ActiveProbe, Capability::Crawl],
        ScanDepth::Deep => vec![Capability::HttpFuzzLowImpact],
    };
    let st = scan_type.to_ascii_lowercase();
    if st.contains("stress") || st.contains("syn") || st.contains("udp") || st.contains("icmp") {
        caps.push(Capability::WafStressTest);
    }
    if st.contains("load") || st.contains("bench") {
        caps.push(Capability::LoadTest);
    }
    if st.contains("packet") || st.contains("raw") {
        caps.push(Capability::RawPacketProbe);
    }
    if st.contains("credential") || st.contains("brute") || st.contains("auth") {
        caps.push(Capability::CredentialTesting);
    }
    if st.contains("remote") || st.contains("exec") || st.contains("ssh") {
        caps.push(Capability::RemoteExecution);
    }
    if st.contains("fuzz") || st.contains("intrusive") {
        if !caps.contains(&Capability::HttpFuzzLowImpact)
            && !caps.contains(&Capability::IntrusiveFuzz)
        {
            caps.push(Capability::IntrusiveFuzz);
        }
    }
    caps
}

pub(crate) fn operation_descriptor_for_agent_scan(
    target: &str,
    scan_type: &str,
    depth: ScanDepth,
) -> OperationDescriptor {
    use crate::tool::metadata::metadata_for_tool_id;

    // Try to match scan_type to known metadata
    if let Some(metadata) = metadata_for_tool_id(scan_type) {
        let mut descriptor = metadata.descriptor_for_target(Some(target.to_string()));
        descriptor.requires_explicit_scope = true;
        return descriptor;
    }

    // Fallback: keyword-based classification for unknown scan types
    OperationDescriptor {
        operation: scan_type.to_string(),
        mode: OperationMode::StandardAssessment,
        risk: risk_for_agent_scan_depth(depth, scan_type),
        intended_uses: vec![IntendedUse::WebAssessment],
        target: Some(target.to_string()),
        required_features: Vec::new(),
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: true,
        required_capabilities: capabilities_for_agent_scan(scan_type, depth),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Capability, OperationRisk};

    #[test]
    fn risk_shallow_default_is_safe_active() {
        assert_eq!(
            risk_for_agent_scan_depth(ScanDepth::Shallow, "recon"),
            OperationRisk::SafeActive
        );
    }

    #[test]
    fn risk_deep_default_is_intrusive() {
        assert_eq!(
            risk_for_agent_scan_depth(ScanDepth::Deep, "pipeline"),
            OperationRisk::Intrusive
        );
    }

    #[test]
    fn risk_stress_keywords() {
        for kw in &["stress", "syn", "udp", "icmp"] {
            assert_eq!(
                risk_for_agent_scan_depth(ScanDepth::Shallow, kw),
                OperationRisk::StressTest,
                "keyword '{}' should map to StressTest",
                kw
            );
        }
    }

    #[test]
    fn risk_load_keywords() {
        for kw in &["load", "bench"] {
            assert_eq!(
                risk_for_agent_scan_depth(ScanDepth::Shallow, kw),
                OperationRisk::LoadTest,
                "keyword '{}' should map to LoadTest",
                kw
            );
        }
    }

    #[test]
    fn risk_packet_keywords() {
        for kw in &["packet", "raw"] {
            assert_eq!(
                risk_for_agent_scan_depth(ScanDepth::Shallow, kw),
                OperationRisk::RawPacket,
                "keyword '{}' should map to RawPacket",
                kw
            );
        }
    }

    #[test]
    fn risk_credential_keywords() {
        for kw in &["credential", "brute", "auth"] {
            assert_eq!(
                risk_for_agent_scan_depth(ScanDepth::Shallow, kw),
                OperationRisk::CredentialTesting,
                "keyword '{}' should map to CredentialTesting",
                kw
            );
        }
    }

    #[test]
    fn risk_remote_keywords() {
        for kw in &["remote", "exec", "ssh"] {
            assert_eq!(
                risk_for_agent_scan_depth(ScanDepth::Shallow, kw),
                OperationRisk::RemoteExecution,
                "keyword '{}' should map to RemoteExecution",
                kw
            );
        }
    }

    #[test]
    fn risk_keyword_takes_precedence_over_depth() {
        assert_eq!(
            risk_for_agent_scan_depth(ScanDepth::Deep, "stress_test"),
            OperationRisk::StressTest
        );
        assert_eq!(
            risk_for_agent_scan_depth(ScanDepth::Shallow, "load_test"),
            OperationRisk::LoadTest
        );
    }

    #[test]
    fn risk_case_insensitive() {
        assert_eq!(
            risk_for_agent_scan_depth(ScanDepth::Shallow, "STRESS"),
            OperationRisk::StressTest
        );
        assert_eq!(
            risk_for_agent_scan_depth(ScanDepth::Deep, "LoadBench"),
            OperationRisk::LoadTest
        );
    }

    #[test]
    fn capabilities_shallow_default() {
        let caps = capabilities_for_agent_scan("recon", ScanDepth::Shallow);
        assert!(caps.contains(&Capability::ActiveProbe));
        assert!(caps.contains(&Capability::Crawl));
        assert_eq!(caps.len(), 2);
    }

    #[test]
    fn capabilities_deep_default() {
        let caps = capabilities_for_agent_scan("pipeline", ScanDepth::Deep);
        assert!(caps.contains(&Capability::HttpFuzzLowImpact));
        assert_eq!(caps.len(), 1);
    }

    #[test]
    fn capabilities_stress_adds_waf_stress_test() {
        let caps = capabilities_for_agent_scan("syn_scan", ScanDepth::Shallow);
        assert!(caps.contains(&Capability::WafStressTest));
        assert!(caps.contains(&Capability::ActiveProbe));
    }

    #[test]
    fn capabilities_load_adds_load_test() {
        let caps = capabilities_for_agent_scan("load_test", ScanDepth::Shallow);
        assert!(caps.contains(&Capability::LoadTest));
    }

    #[test]
    fn capabilities_packet_adds_raw_packet_probe() {
        let caps = capabilities_for_agent_scan("packet_capture", ScanDepth::Shallow);
        assert!(caps.contains(&Capability::RawPacketProbe));
    }

    #[test]
    fn capabilities_credential_adds_credential_testing() {
        let caps = capabilities_for_agent_scan("brute_force", ScanDepth::Shallow);
        assert!(caps.contains(&Capability::CredentialTesting));
    }

    #[test]
    fn capabilities_remote_adds_remote_execution() {
        let caps = capabilities_for_agent_scan("ssh_exec", ScanDepth::Shallow);
        assert!(caps.contains(&Capability::RemoteExecution));
    }

    #[test]
    fn capabilities_fuzz_adds_intrusive_fuzz_when_not_deep() {
        let caps = capabilities_for_agent_scan("fuzz_test", ScanDepth::Shallow);
        assert!(caps.contains(&Capability::IntrusiveFuzz));
        assert!(caps.contains(&Capability::ActiveProbe));
    }

    #[test]
    fn capabilities_fuzz_does_not_duplicate_intrusive_fuzz_for_deep() {
        let caps = capabilities_for_agent_scan("fuzz_test", ScanDepth::Deep);
        assert!(caps.contains(&Capability::HttpFuzzLowImpact));
        // IntrusiveFuzz should NOT be added because HttpFuzzLowImpact is already present
        assert!(!caps.contains(&Capability::IntrusiveFuzz));
    }

    #[test]
    fn capabilities_intrusive_keyword_does_not_duplicate() {
        let caps = capabilities_for_agent_scan("intrusive_scan", ScanDepth::Deep);
        assert!(caps.contains(&Capability::HttpFuzzLowImpact));
        assert!(!caps.contains(&Capability::IntrusiveFuzz));
    }

    #[test]
    fn operation_descriptor_shallow_recon() {
        let desc =
            operation_descriptor_for_agent_scan("https://example.com", "recon", ScanDepth::Shallow);
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
    fn operation_descriptor_deep_stress() {
        let desc = operation_descriptor_for_agent_scan(
            "https://target.com",
            "syn_stress",
            ScanDepth::Deep,
        );
        assert_eq!(desc.risk, OperationRisk::StressTest);
        assert!(desc
            .required_capabilities
            .contains(&Capability::WafStressTest));
        assert!(desc
            .required_capabilities
            .contains(&Capability::HttpFuzzLowImpact));
    }

    #[test]
    fn operation_descriptor_multiple_keywords_first_match_wins() {
        let desc = operation_descriptor_for_agent_scan(
            "https://example.com",
            "stress_load",
            ScanDepth::Shallow,
        );
        // "stress" is checked before "load", so StressTest wins
        assert_eq!(desc.risk, OperationRisk::StressTest);
    }
}
