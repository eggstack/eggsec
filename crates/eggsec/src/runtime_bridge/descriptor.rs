use crate::config::{operation_metadata, OperationDescriptor};
use eggsec_runtime::request::TaskKind;
use eggsec_runtime::RunRequest;

use super::surface::RuntimeBridgeError;

/// Convert a [`RunRequest`] into an [`OperationDescriptor`].
///
/// This extracts the canonical operation ID and target from the frontend-neutral
/// task payload, then resolves the descriptor via `ALL_OPERATION_METADATA`.
/// It does not decide authorization — that is the enforcement layer's job.
///
/// Unsupported task kinds return a typed error rather than silently downgrading
/// to a generic operation.
pub fn descriptor_for_run_request(
    request: &RunRequest,
) -> Result<OperationDescriptor, RuntimeBridgeError> {
    let (operation_id, target) = resolve_operation_and_target(&request.task_kind)?;

    let metadata =
        operation_metadata(operation_id).ok_or_else(|| RuntimeBridgeError::UnknownOperationId {
            operation_id: operation_id.to_string(),
        })?;

    metadata
        .try_descriptor_for_target(target.as_deref())
        .map_err(|e| RuntimeBridgeError::InvalidTarget {
            operation_id: operation_id.to_string(),
            reason: e.to_string(),
        })
}

/// Resolve the canonical operation ID and optional target from a [`TaskKind`].
///
/// Phase 3 closure: delegates to the single wire-side exhaustive mapping
/// (`TaskKind::operation_id` / `TaskKind::canonical_target`) so runtime
/// semantics cannot silently diverge from the canonical path. Returns
/// `(operation_id, target)` where `target` is `None` for operations with
/// `NoTarget` policy or interface-bound tasks.
fn resolve_operation_and_target(
    task_kind: &TaskKind,
) -> Result<(&'static str, Option<String>), RuntimeBridgeError> {
    // Exhaustive via TaskKind methods; no wildcard fallback. Packet
    // traceroute/send map explicitly to the `packet` family with their target
    // (they no longer fail as `UnsupportedTaskKind`).
    Ok((task_kind.operation_id(), task_kind.canonical_target()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ALL_OPERATION_METADATA;
    use eggsec_runtime::request::*;

    fn make_request(task_kind: TaskKind) -> RunRequest {
        RunRequest {
            task_kind,
            requested_by: None,
            surface: RuntimeSurface::CliManual,
            labels: vec![],
        }
    }

    #[test]
    fn port_scan_descriptor_matches_metadata() {
        let req = make_request(TaskKind::PortScan(PortScanParams {
            target: "10.0.0.1".into(),
            ports: Some("80,443".into()),
            scan_type: None,
            timeout_ms: None,
            ..Default::default()
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        let meta = operation_metadata("scan-ports").unwrap();
        assert_eq!(desc.operation, meta.id);
        assert_eq!(desc.risk, meta.risk);
        assert_eq!(desc.mode, meta.mode);
        assert_eq!(desc.target, Some("10.0.0.1".to_string()));
        assert_eq!(
            desc.required_capabilities,
            meta.required_capabilities.to_vec()
        );
    }

    #[test]
    fn endpoint_scan_descriptor() {
        let req = make_request(TaskKind::EndpointScan(EndpointScanParams {
            target: "https://example.com".into(),
            methods: None,
            wordlist: None,
            ..Default::default()
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "scan-endpoints");
        assert_eq!(desc.target, Some("https://example.com".to_string()));
    }

    #[test]
    fn fingerprint_descriptor() {
        let req = make_request(TaskKind::Fingerprint(FingerprintParams {
            target: "10.0.0.1".into(),
            ..Default::default()
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "fingerprint");
        assert_eq!(desc.target, Some("10.0.0.1".to_string()));
    }

    #[test]
    fn waf_descriptor() {
        let req = make_request(TaskKind::Waf(WafParams {
            target: "https://example.com".into(),
            ..Default::default()
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "waf-detect");
    }

    #[test]
    fn waf_stress_descriptor() {
        let req = make_request(TaskKind::WafStress(WafStressParams {
            target: "https://example.com".into(),
            requests: None,
            ..Default::default()
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "waf-stress");
    }

    #[test]
    fn pipeline_descriptor() {
        let req = make_request(TaskKind::Pipeline(PipelineParams {
            target: "https://example.com".into(),
            profile: None,
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "pipeline");
    }

    #[test]
    fn recon_descriptor() {
        let req = make_request(TaskKind::Recon(ReconParams {
            target: "example.com".into(),
            modules: None,
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "recon");
    }

    #[test]
    fn load_test_descriptor() {
        let req = make_request(TaskKind::LoadTest(LoadTestParams {
            target: "https://example.com".into(),
            method: "GET".into(),
            connections: None,
            duration_secs: None,
            rate_limit: None,
            ..Default::default()
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "load-test");
    }

    #[test]
    fn fuzz_descriptor() {
        let req = make_request(TaskKind::Fuzz(FuzzParams {
            target: "https://example.com".into(),
            payload_type: None,
            threads: None,
            ..Default::default()
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "fuzz");
    }

    #[test]
    fn stress_test_descriptor() {
        let req = make_request(TaskKind::StressTest(StressTestParams {
            target: "https://example.com".into(),
            flood_type: "syn".into(),
            duration_secs: None,
            threads: None,
            ..Default::default()
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "stress-test");
    }

    #[test]
    fn packet_capture_descriptor() {
        let req = make_request(TaskKind::PacketCapture(PacketCaptureParams {
            interface: None,
            filter: None,
            duration_secs: None,
            ..Default::default()
        }));
        let result = descriptor_for_run_request(&req);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RuntimeBridgeError::InvalidTarget { .. }
        ));
    }

    #[test]
    fn graphql_descriptor() {
        let req = make_request(TaskKind::GraphQl(GraphQlParams {
            target: "https://example.com/graphql".into(),
            introspection: None,
            ..Default::default()
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "graphql");
    }

    #[test]
    fn oauth_descriptor() {
        let req = make_request(TaskKind::OAuth(OAuthParams {
            target: "https://example.com".into(),
            flow: None,
            ..Default::default()
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "oauth");
    }

    #[test]
    fn auth_test_descriptor() {
        let req = make_request(TaskKind::AuthTest(AuthTestParams {
            target: "https://example.com".into(),
            username: None,
            credential_list: None,
            ..Default::default()
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "auth-test");
    }

    #[test]
    fn nse_descriptor() {
        let req = make_request(TaskKind::Nse(NseParams {
            target: "10.0.0.1".into(),
            script: "http-enum".into(),
            args: None,
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "nse");
    }

    #[test]
    fn hunt_descriptor() {
        let req = make_request(TaskKind::Hunt(HuntParams {
            target: "https://example.com".into(),
            hunt_type: None,
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "hunt");
    }

    #[test]
    fn browser_descriptor() {
        let req = make_request(TaskKind::Browser(BrowserParams {
            target: "https://example.com".into(),
            headless: None,
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "browser");
    }

    #[test]
    fn compliance_descriptor() {
        let req = make_request(TaskKind::Compliance(ComplianceParams {
            target: "https://example.com".into(),
            framework: None,
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "compliance");
    }

    #[test]
    fn storage_descriptor() {
        let req = make_request(TaskKind::Storage(StorageParams {
            storage_type: "findings".into(),
            path: None,
        }));
        let result = descriptor_for_run_request(&req);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RuntimeBridgeError::InvalidTarget { .. }
        ));
    }

    #[test]
    fn integrations_descriptor() {
        let req = make_request(TaskKind::Integrations(IntegrationsParams {
            integration_type: "jira".into(),
            config: None,
        }));
        let result = descriptor_for_run_request(&req);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RuntimeBridgeError::InvalidTarget { .. }
        ));
    }

    #[test]
    fn workflow_descriptor() {
        let req = make_request(TaskKind::Workflow(WorkflowParams {
            workflow_id: None,
            steps: None,
        }));
        let result = descriptor_for_run_request(&req);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RuntimeBridgeError::InvalidTarget { .. }
        ));
    }

    #[test]
    fn vuln_descriptor() {
        let req = make_request(TaskKind::Vuln(VulnParams {
            target: "https://example.com".into(),
            vuln_type: None,
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "vuln");
    }

    #[test]
    fn wireless_descriptor() {
        let req = make_request(TaskKind::Wireless(WirelessParams {
            interface: None,
            duration_secs: None,
        }));
        let result = descriptor_for_run_request(&req);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RuntimeBridgeError::InvalidTarget { .. }
        ));
    }

    #[test]
    fn wireless_active_descriptor() {
        let req = make_request(TaskKind::WirelessActive(WirelessActiveParams {
            interface: None,
            target_bssid: None,
        }));
        let result = descriptor_for_run_request(&req);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RuntimeBridgeError::InvalidTarget { .. }
        ));
    }

    #[test]
    fn db_pentest_descriptor() {
        let req = make_request(TaskKind::DbPentest(DbPentestParams {
            db_type: "postgres".into(),
            target: "localhost".into(),
            port: None,
            ..Default::default()
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "db-pentest");
    }

    #[test]
    fn intercept_descriptor() {
        let req = make_request(TaskKind::Intercept(InterceptParams {
            listen_port: None,
            target: None,
            ..Default::default()
        }));
        let result = descriptor_for_run_request(&req);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RuntimeBridgeError::InvalidTarget { .. }
        ));
    }

    #[test]
    fn c2_descriptor() {
        let req = make_request(TaskKind::C2(C2Params {
            profile: None,
            target: None,
            ..Default::default()
        }));
        let result = descriptor_for_run_request(&req);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RuntimeBridgeError::InvalidTarget { .. }
        ));
    }

    #[test]
    fn packet_traceroute_maps_to_packet_operation() {
        // Phase C convergence: packet traceroute/send are explicit `packet`
        // family mappings with their target (no silent alias fallback, no
        // `UnsupportedTaskKind`).
        let req = make_request(TaskKind::PacketTraceroute(PacketTracerouteParams {
            target: "10.0.0.1".into(),
            max_hops: None,
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "packet");
        assert_eq!(desc.target, Some("10.0.0.1".to_string()));
    }

    #[test]
    fn packet_send_maps_to_packet_operation() {
        let req = make_request(TaskKind::PacketSend(PacketSendParams {
            target: "10.0.0.1".into(),
            protocol: "tcp".into(),
            payload: None,
            ..Default::default()
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "packet");
        assert_eq!(desc.target, Some("10.0.0.1".to_string()));
    }

    #[test]
    fn descriptor_requires_explicit_scope_for_agent_exposable_ops() {
        for meta in ALL_OPERATION_METADATA {
            if meta.agent_exposable
                && meta.target_policy != crate::config::TargetPolicyKind::NoTarget
            {
                let desc = meta.descriptor_for_target(Some("https://example.com".into()));
                assert!(
                    desc.requires_explicit_scope,
                    "agent-exposable op '{}' with target should require explicit scope",
                    meta.id
                );
            }
        }
    }
}
