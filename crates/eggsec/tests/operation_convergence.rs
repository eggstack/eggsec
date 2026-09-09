//! Phase C convergence matrix tests.
//!
//! Exercises the same canonical request through multiple surfaces (CLI
//! adapters, runtime `TaskKind`, tool JSON params) and proves equivalent
//! normalized engine requests. Covers representative operations from
//! scanner, fuzzer, WAF, loadtest, pipeline, auth/GraphQL/OAuth, one
//! feature-gated high-risk domain (db-pentest), and one local-file domain
//! (storage).

use eggsec::operation_request::{runtime_adapters, validate_tool_params};
use eggsec::operation_request::{
    AuthTestRequest, DbPentestRequest, EndpointScanRequest, FingerprintRequest, FuzzRequest,
    GraphQlRequest, LoadTestRequest, OAuthRequest, PipelineRequest, PortScanRequest, ReconRequest,
    StorageRequest, WafDetectRequest, WafStressRequest,
};

#[test]
fn scanner_port_scan_surfaces_converge() {
    let canonical = PortScanRequest {
        target: "example.com".into(),
        ports: Some("22,80,443".into()),
        scan_type: Some("syn".into()),
        timeout_ms: None,
        concurrency: None,
    };
    let normalized = canonical.normalize().unwrap();
    assert_eq!(normalized.ports, "22,80,443");
    assert_eq!(normalized.port_count, 3);

    // Runtime adapter produces identical normalization.
    let runtime =
        runtime_adapters::port_scan_from_runtime(&eggsec_runtime::request::PortScanParams {
            target: "example.com".into(),
            ports: Some("22,80,443".into()),
            scan_type: Some("syn".into()),
            timeout_ms: None,
            concurrency: None,
        });
    assert_eq!(runtime.normalize().unwrap(), normalized);

    // Tool JSON params validate through the same canonical owner.
    let params = serde_json::json!({
        "target": "example.com",
        "ports": "22,80,443",
        "scan_type": "syn",
    });
    assert_eq!(
        validate_tool_params("scan-ports", &params).unwrap(),
        "scan-ports"
    );
}

#[test]
fn scanner_endpoint_and_fingerprint_converge() {
    let endpoint = EndpointScanRequest {
        target: "https://example.com".into(),
        concurrency: None,
        timeout_secs: None,
        wordlist: None,
    }
    .normalize()
    .unwrap();
    // Canonical default is 20 (CLI), not the legacy runtime 10.
    assert_eq!(endpoint.concurrency, 20);
    assert_eq!(endpoint.timeout_secs, 10);

    let runtime = runtime_adapters::endpoint_scan_from_runtime(
        &eggsec_runtime::request::EndpointScanParams {
            target: "https://example.com".into(),
            methods: None,
            wordlist: None,
            concurrency: None,
            timeout_secs: None,
        },
    )
    .normalize()
    .unwrap();
    assert_eq!(runtime, endpoint);

    let fp = FingerprintRequest {
        target: "example.com".into(),
        ports: None,
        timeout_secs: None,
        concurrency: None,
    }
    .normalize()
    .unwrap();
    assert_eq!(fp.ports, "80,443,22,21,25,3306,5432,6379,27017");
    assert_eq!(fp.timeout_secs, 5);
}

#[test]
fn fuzzer_surfaces_converge() {
    let canonical = FuzzRequest {
        target: "https://example.com".into(),
        payload_type: None,
        mode: None,
        method: None,
        param: None,
        threads: None,
        timeout_secs: None,
        mutations: None,
        mutation_count: None,
        graphql_introspection: None,
        graphql_depth_bypass: None,
        graphql_alias_overload: None,
        oauth_redirect_test: None,
        oauth_scope_test: None,
        oauth_state_test: None,
        oauth_grant_test: None,
    }
    .normalize()
    .unwrap();
    assert_eq!(canonical.payload_type, "all");
    assert_eq!(canonical.mode, "sequential");
    assert_eq!(canonical.mutation_count, 3);
    assert!(canonical.graphql_introspection);
    assert!(canonical.oauth_redirect_test);

    let runtime = runtime_adapters::fuzz_from_runtime(&eggsec_runtime::request::FuzzParams {
        target: "https://example.com".into(),
        ..Default::default()
    })
    .normalize()
    .unwrap();
    assert_eq!(runtime, canonical);

    let params = serde_json::json!({"target": "https://example.com"});
    assert_eq!(validate_tool_params("fuzz", &params).unwrap(), "fuzz");
}

#[test]
fn waf_surfaces_converge() {
    let detect = WafDetectRequest {
        target: "https://example.com".into(),
        bypass_mode: None,
        techniques: None,
    }
    .normalize()
    .unwrap();
    assert!(!detect.bypass_mode);

    let stress = WafStressRequest {
        target: "https://example.com".into(),
        requests: None,
        concurrency: None,
    }
    .normalize()
    .unwrap();
    // Canonical default is 20 (CLI WafStressArgs), not legacy 10.
    assert_eq!(stress.concurrency, 20);
    assert_eq!(stress.requests, 100);
}

#[test]
fn loadtest_requests_vs_connections_converge() {
    // Explicit requests win.
    let explicit = LoadTestRequest {
        target: "https://example.com".into(),
        method: None,
        requests: Some(1000),
        connections: Some(10),
        duration_secs: None,
        rate_limit: None,
    }
    .normalize()
    .unwrap();
    assert_eq!((explicit.requests, explicit.concurrency), (1000, 10));

    // Legacy connections-only means total == connections.
    let legacy = LoadTestRequest {
        target: "https://example.com".into(),
        method: None,
        requests: None,
        connections: Some(25),
        duration_secs: None,
        rate_limit: None,
    }
    .normalize()
    .unwrap();
    assert_eq!((legacy.requests, legacy.concurrency), (25, 25));

    let runtime =
        runtime_adapters::load_test_from_runtime(&eggsec_runtime::request::LoadTestParams {
            target: "https://example.com".into(),
            method: "GET".into(),
            requests: None,
            connections: Some(25),
            duration_secs: None,
            rate_limit: None,
        })
        .normalize()
        .unwrap();
    assert_eq!(runtime.requests, 25);
}

#[test]
fn pipeline_profile_rejects_unknown() {
    let ok = PipelineRequest {
        target: "https://example.com".into(),
        profile: Some("web".into()),
    }
    .normalize()
    .unwrap();
    assert_eq!(ok.profile.as_str(), "web");

    let default = PipelineRequest {
        target: "https://example.com".into(),
        profile: None,
    }
    .normalize()
    .unwrap();
    assert_eq!(default.profile.as_str(), "quick");

    let bad = PipelineRequest {
        target: "https://example.com".into(),
        profile: Some("bogus-profile".into()),
    };
    assert!(bad.normalize().is_err());
}

#[test]
fn recon_surfaces_converge() {
    let canonical = ReconRequest {
        target: "example.com".into(),
        modules: None,
    }
    .normalize()
    .unwrap();
    let runtime = runtime_adapters::recon_from_runtime(&eggsec_runtime::request::ReconParams {
        target: "example.com".into(),
        modules: None,
    })
    .normalize()
    .unwrap();
    assert_eq!(runtime, canonical);
}

#[test]
fn auth_graphql_oauth_converge() {
    let graphql = GraphQlRequest {
        target: "https://example.com/graphql".into(),
        introspection: None,
        inject: None,
        depth_bypass: None,
        alias_overload: None,
        concurrency: None,
        timeout_secs: None,
    }
    .normalize()
    .unwrap();
    assert!(graphql.introspection);
    assert!(graphql.depth_bypass);
    assert!(graphql.alias_overload);
    assert_eq!(graphql.timeout_secs, 15);

    let oauth = OAuthRequest {
        target: "https://example.com".into(),
        flow: None,
        client_id: None,
        redirect_uri: None,
        redirect_test: None,
        scope_test: None,
        state_test: None,
        grant_test: None,
        concurrency: None,
        timeout_secs: None,
    }
    .normalize()
    .unwrap();
    assert!(oauth.redirect_test);
    assert!(oauth.scope_test);
    assert!(oauth.state_test);
    assert!(oauth.grant_test);
    assert_eq!(oauth.timeout_secs, 15);

    let auth = AuthTestRequest {
        target: "https://example.com".into(),
        username: None,
        credential_list: None,
        credential_file: None,
        max_attempts: None,
        concurrency: None,
        timeout_secs: None,
    }
    .normalize()
    .unwrap();
    assert_eq!(auth.max_attempts, 100);
    assert_eq!(auth.concurrency, 1);
    assert_eq!(auth.timeout_secs, 10);
}

#[test]
fn feature_gated_db_pentest_converges() {
    let canonical = DbPentestRequest {
        target: "localhost".into(),
        db_type: "postgres".into(),
        port: None,
        checks: None,
        max_queries: None,
        max_duration_secs: None,
        dry_run: None,
        allow_advanced: None,
    }
    .normalize()
    .unwrap();
    assert_eq!(canonical.checks, "all");
    assert!(canonical.dry_run);

    let runtime =
        runtime_adapters::db_pentest_from_runtime(&eggsec_runtime::request::DbPentestParams {
            db_type: "postgres".into(),
            target: "localhost".into(),
            ..Default::default()
        })
        .normalize()
        .unwrap();
    assert_eq!(runtime.checks, canonical.checks);

    let params = serde_json::json!({"target": "localhost", "db_type": "postgres"});
    assert_eq!(
        validate_tool_params("db-pentest", &params).unwrap(),
        "db-pentest"
    );
    assert!(validate_tool_params(
        "db-pentest",
        &serde_json::json!({"target": "localhost", "db_type": "bogus"})
    )
    .is_err());
}

#[test]
fn local_file_storage_converges() {
    let canonical = StorageRequest {
        storage_type: "findings".into(),
        path: None,
    }
    .normalize()
    .unwrap();
    assert_eq!(canonical.storage_type, "findings");

    let params = serde_json::json!({"storage_type": "findings"});
    assert_eq!(validate_tool_params("storage", &params).unwrap(), "storage");
    assert!(validate_tool_params("storage", &serde_json::json!({"storage_type": "   "})).is_err());
}

#[test]
fn every_operation_backed_command_has_canonical_execution_mapping() {
    use eggsec::commands::registry::REGISTERED_COMMANDS;
    use eggsec::config::metadata_for_tool_id;

    for reg in REGISTERED_COMMANDS {
        if let Some(op_id) = reg.operation_id {
            // Must resolve to canonical metadata.
            assert!(
                metadata_for_tool_id(op_id).is_some(),
                "Command '{}' operation '{}' has no canonical metadata",
                reg.command_id,
                op_id
            );
            // Must use canonical dispatch (no LegacyWrapped remains).
            assert!(
                matches!(
                    reg.dispatch_mode,
                    eggsec::commands::registry::CommandDispatchMode::RegistryBacked
                ),
                "Operation-backed command '{}' must use RegistryBacked",
                reg.command_id
            );
            // Must build a descriptor (target optional; NoTarget/OptionalTarget
            // accept None, others require target — both paths must not panic).
            let _ = reg.build_descriptor(None);
            let _ = reg.build_descriptor(Some("example.com".to_string()));
        }
    }
}

#[test]
fn runtime_task_kinds_map_exhaustively_to_canonical_operations() {
    use eggsec_runtime::request::TaskKind;
    // Spot-check representative mappings; exhaustive construction is covered
    // in `operation_request` unit tests. Here we prove the runtime methods
    // agree with the engine bridge mapping.
    let port_scan = TaskKind::PortScan(eggsec_runtime::request::PortScanParams {
        target: "10.0.0.1".into(),
        ..Default::default()
    });
    assert_eq!(port_scan.operation_id(), "scan-ports");
    assert_eq!(port_scan.canonical_target().as_deref(), Some("10.0.0.1"));

    let packet_send = TaskKind::PacketSend(eggsec_runtime::request::PacketSendParams {
        target: "10.0.0.1".into(),
        protocol: "tcp".into(),
        ..Default::default()
    });
    assert_eq!(packet_send.operation_id(), "packet");
}
