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

    Ok(metadata.descriptor_for_target(target))
}

/// Resolve the canonical operation ID and optional target from a [`TaskKind`].
///
/// Returns `(operation_id, target)` where `target` is `None` for operations
/// with `NoTarget` policy.
fn resolve_operation_and_target(
    task_kind: &TaskKind,
) -> Result<(&'static str, Option<String>), RuntimeBridgeError> {
    match task_kind {
        TaskKind::PortScan(p) => Ok(("scan-ports", Some(p.target.clone()))),
        TaskKind::EndpointScan(p) => Ok(("scan-endpoints", Some(p.target.clone()))),
        TaskKind::Fingerprint(p) => Ok(("fingerprint", Some(p.target.clone()))),
        TaskKind::Waf(p) => Ok(("waf-detect", Some(p.target.clone()))),
        TaskKind::WafStress(p) => Ok(("waf-stress", Some(p.target.clone()))),
        TaskKind::Pipeline(p) => Ok(("pipeline", Some(p.target.clone()))),
        TaskKind::Recon(p) => Ok(("recon", Some(p.target.clone()))),
        TaskKind::LoadTest(p) => Ok(("load-test", Some(p.target.clone()))),
        TaskKind::Fuzz(p) => Ok(("fuzz", Some(p.target.clone()))),
        TaskKind::StressTest(p) => Ok(("stress-test", Some(p.target.clone()))),
        TaskKind::PacketCapture(_) => Ok(("packet", None)),
        TaskKind::GraphQl(p) => Ok(("graphql", Some(p.target.clone()))),
        TaskKind::OAuth(p) => Ok(("oauth", Some(p.target.clone()))),
        TaskKind::AuthTest(p) => Ok(("auth-test", Some(p.target.clone()))),
        TaskKind::Nse(p) => Ok(("nse", Some(p.target.clone()))),
        TaskKind::Hunt(p) => Ok(("hunt", Some(p.target.clone()))),
        TaskKind::Browser(p) => Ok(("browser", Some(p.target.clone()))),
        TaskKind::Compliance(p) => Ok(("compliance", Some(p.target.clone()))),
        TaskKind::Storage(_) => Ok(("storage", None)),
        TaskKind::Integrations(_) => Ok(("integrations", None)),
        TaskKind::Workflow(_) => Ok(("workflow", None)),
        TaskKind::Vuln(p) => Ok(("vuln", Some(p.target.clone()))),
        TaskKind::Wireless(_) => Ok(("wireless", None)),
        TaskKind::WirelessActive(_) => Ok(("wireless", None)),
        TaskKind::DbPentest(p) => Ok(("db-pentest", Some(p.target.clone()))),
        TaskKind::Intercept(_) => Ok(("proxy-intercept", None)),
        TaskKind::C2(_) => Ok(("c2", None)),
        TaskKind::PacketTraceroute(p) => Err(RuntimeBridgeError::UnsupportedTaskKind {
            kind: format!("PacketTraceroute (target: {})", p.target),
        }),
        TaskKind::PacketSend(p) => Err(RuntimeBridgeError::UnsupportedTaskKind {
            kind: format!("PacketSend (target: {})", p.target),
        }),
    }
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
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "scan-endpoints");
        assert_eq!(desc.target, Some("https://example.com".to_string()));
    }

    #[test]
    fn fingerprint_descriptor() {
        let req = make_request(TaskKind::Fingerprint(FingerprintParams {
            target: "10.0.0.1".into(),
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "fingerprint");
        assert_eq!(desc.target, Some("10.0.0.1".to_string()));
    }

    #[test]
    fn waf_descriptor() {
        let req = make_request(TaskKind::Waf(WafParams {
            target: "https://example.com".into(),
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "waf-detect");
    }

    #[test]
    fn waf_stress_descriptor() {
        let req = make_request(TaskKind::WafStress(WafStressParams {
            target: "https://example.com".into(),
            requests: None,
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
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "packet");
        assert_eq!(desc.target, None);
    }

    #[test]
    fn graphql_descriptor() {
        let req = make_request(TaskKind::GraphQl(GraphQlParams {
            target: "https://example.com/graphql".into(),
            introspection: None,
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "graphql");
    }

    #[test]
    fn oauth_descriptor() {
        let req = make_request(TaskKind::OAuth(OAuthParams {
            target: "https://example.com".into(),
            flow: None,
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
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "storage");
        assert_eq!(desc.target, None);
    }

    #[test]
    fn integrations_descriptor() {
        let req = make_request(TaskKind::Integrations(IntegrationsParams {
            integration_type: "jira".into(),
            config: None,
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "integrations");
    }

    #[test]
    fn workflow_descriptor() {
        let req = make_request(TaskKind::Workflow(WorkflowParams {
            workflow_id: None,
            steps: None,
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "workflow");
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
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "wireless");
    }

    #[test]
    fn wireless_active_descriptor() {
        let req = make_request(TaskKind::WirelessActive(WirelessActiveParams {
            interface: None,
            target_bssid: None,
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "wireless");
    }

    #[test]
    fn db_pentest_descriptor() {
        let req = make_request(TaskKind::DbPentest(DbPentestParams {
            db_type: "postgres".into(),
            target: "localhost".into(),
            port: None,
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "db-pentest");
    }

    #[test]
    fn intercept_descriptor() {
        let req = make_request(TaskKind::Intercept(InterceptParams {
            listen_port: None,
            target: None,
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "proxy-intercept");
    }

    #[test]
    fn c2_descriptor() {
        let req = make_request(TaskKind::C2(C2Params {
            profile: None,
            target: None,
        }));
        let desc = descriptor_for_run_request(&req).unwrap();
        assert_eq!(desc.operation, "c2");
    }

    #[test]
    fn packet_traceroute_unsupported() {
        let req = make_request(TaskKind::PacketTraceroute(PacketTracerouteParams {
            target: "10.0.0.1".into(),
            max_hops: None,
        }));
        let result = descriptor_for_run_request(&req);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RuntimeBridgeError::UnsupportedTaskKind { .. }
        ));
    }

    #[test]
    fn packet_send_unsupported() {
        let req = make_request(TaskKind::PacketSend(PacketSendParams {
            target: "10.0.0.1".into(),
            protocol: "tcp".into(),
            payload: None,
        }));
        let result = descriptor_for_run_request(&req);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RuntimeBridgeError::UnsupportedTaskKind { .. }
        ));
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
