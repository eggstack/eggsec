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
    GraphQlRequest, LoadTestRequest, OAuthRequest, PipelineOutputFormat, PipelineRequest,
    PortScanRequest, ReconRequest, StorageRequest, WafDetectRequest, WafStressRequest,
};

#[test]
fn scanner_port_scan_surfaces_converge() {
    let canonical = PortScanRequest {
        target: "example.com".into(),
        ports: Some("22,80,443".into()),
        scan_type: Some("syn".into()),
        timeout_ms: None,
        concurrency: None,
        udp: None,
    };
    let normalized = canonical.normalize().unwrap();
    assert_eq!(normalized.ports, "22,80,443");
    assert_eq!(normalized.port_count, 3);
    // Absent means TCP, so every existing caller keeps its behaviour.
    assert!(!normalized.udp);

    // Runtime adapter produces identical normalization.
    let runtime =
        runtime_adapters::port_scan_from_runtime(&eggsec_runtime::request::PortScanParams {
            target: "example.com".into(),
            ports: Some("22,80,443".into()),
            scan_type: Some("syn".into()),
            timeout_ms: None,
            concurrency: None,
            udp: None,
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
        include_404: Some(true),
    }
    .normalize()
    .unwrap();
    // Canonical default is 20 (CLI), not the legacy runtime 10.
    assert_eq!(endpoint.concurrency, 20);
    assert_eq!(endpoint.timeout_secs, 10);
    // The flag used to be hardcoded false by the canonical dispatch path, so
    // both the CLI's `--include-404` and the TUI's checkbox were ignored.
    assert!(endpoint.include_404);

    let runtime = runtime_adapters::endpoint_scan_from_runtime(
        &eggsec_runtime::request::EndpointScanParams {
            target: "https://example.com".into(),
            methods: None,
            wordlist: None,
            concurrency: None,
            timeout_secs: None,
            include_404: Some(true),
        },
    )
    .normalize()
    .unwrap();
    assert_eq!(runtime, endpoint);

    // Absent must normalize to the conservative default (exclude 404s), which
    // is what an omitted CLI `--include-404` implies.
    let without_flag = EndpointScanRequest {
        target: "https://example.com".into(),
        concurrency: None,
        timeout_secs: None,
        wordlist: None,
        include_404: None,
    }
    .normalize()
    .unwrap();
    assert!(!without_flag.include_404);

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
        body: None,
        headers: vec![],
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
        body: None,
        headers: vec![],
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
            body: None,
            headers: None,
        })
        .normalize()
        .unwrap();
    assert_eq!(runtime.requests, 25);
}

/// `method`/`body`/`headers` must survive the runtime wire DTO unchanged.
///
/// Regression guard: all three were accepted by the wire DTO and silently
/// dropped before reaching the runner, so a load test issued over the daemon,
/// agent, REST or MCP surface ran as a bodyless GET regardless of the request.
#[test]
fn loadtest_request_shape_converges_across_wire_dto() {
    let canonical = LoadTestRequest {
        target: "https://example.com".into(),
        method: Some("post".into()),
        requests: Some(10),
        connections: Some(2),
        duration_secs: None,
        rate_limit: None,
        body: Some("{\"probe\":1}".into()),
        headers: vec!["X-Probe: eggsec".into()],
    };
    let normalized = canonical.normalize().unwrap();
    // Method is canonicalized to uppercase; headers to `Name:Value`.
    assert_eq!(normalized.method, "POST");
    assert_eq!(normalized.body.as_deref(), Some("{\"probe\":1}"));
    assert_eq!(normalized.headers, vec!["X-Probe:eggsec".to_string()]);

    let runtime =
        runtime_adapters::load_test_from_runtime(&eggsec_runtime::request::LoadTestParams {
            target: "https://example.com".into(),
            method: "post".into(),
            requests: Some(10),
            connections: Some(2),
            duration_secs: None,
            rate_limit: None,
            body: Some("{\"probe\":1}".into()),
            headers: Some(vec!["X-Probe: eggsec".into()]),
        });
    assert_eq!(runtime.normalize().unwrap(), normalized);
}

#[test]
fn pipeline_profile_rejects_unknown() {
    let ok = PipelineRequest {
        target: "https://example.com".into(),
        profile: Some("web".into()),
        output_format: None,
        output_file: None,
        session_path: None,
    }
    .normalize()
    .unwrap();
    assert_eq!(ok.profile.as_str(), "web");

    let default = PipelineRequest {
        target: "https://example.com".into(),
        profile: None,
        output_format: None,
        output_file: None,
        session_path: None,
    }
    .normalize()
    .unwrap();
    assert_eq!(default.profile.as_str(), "quick");

    let bad = PipelineRequest {
        target: "https://example.com".into(),
        profile: Some("bogus-profile".into()),
        output_format: None,
        output_file: None,
        session_path: None,
    };
    assert!(bad.normalize().is_err());
}

/// `output_format`/`output_file` must survive the wire DTO and normalize to a
/// concrete format.
///
/// Regression guard: the scan tab has shown an output-file field and an output
/// format selector for the whole life of the tab, and neither ever reached the
/// engine — the fields did not exist on the wire DTO.
#[test]
fn pipeline_output_converges_across_wire_dto() {
    let canonical = PipelineRequest {
        target: "https://example.com".into(),
        profile: Some("web".into()),
        output_format: Some("SARIF".into()),
        output_file: Some("  reports/scan.sarif  ".into()),
        session_path: None,
    };
    let normalized = canonical.normalize().unwrap();
    assert_eq!(normalized.output_format, PipelineOutputFormat::Sarif);
    // Surrounding whitespace is dropped, not shipped.
    assert_eq!(
        normalized.output_file.as_deref(),
        Some("reports/scan.sarif")
    );

    let runtime =
        runtime_adapters::pipeline_from_runtime(&eggsec_runtime::request::PipelineParams {
            target: "https://example.com".into(),
            profile: Some("web".into()),
            output_format: Some("SARIF".into()),
            output_file: Some("  reports/scan.sarif  ".into()),
            session_path: None,
        });
    assert_eq!(runtime.normalize().unwrap(), normalized);
}

#[test]
fn pipeline_output_format_rejects_unknown_and_defaults_to_html() {
    let base = |format: Option<&str>| PipelineRequest {
        target: "https://example.com".into(),
        profile: None,
        output_format: format.map(str::to_string),
        output_file: Some("report.out".into()),
        session_path: None,
    };
    assert_eq!(
        base(None).normalize().unwrap().output_format,
        PipelineOutputFormat::Html
    );
    assert_eq!(
        base(Some("markdown")).normalize().unwrap().output_format,
        PipelineOutputFormat::Markdown
    );
    // Alias for markdown.
    assert_eq!(
        base(Some("MD")).normalize().unwrap().output_format,
        PipelineOutputFormat::Markdown
    );
    assert!(base(Some("pdf")).normalize().is_err());
}

#[test]
fn pipeline_output_path_rejects_traversal_and_unsafe_characters() {
    let base = |path: &str| PipelineRequest {
        target: "https://example.com".into(),
        profile: None,
        output_format: Some("json".into()),
        output_file: Some(path.to_string()),
        session_path: None,
    };
    // Plain relative paths and nested directories are fine.
    assert!(base("report.json").normalize().is_ok());
    assert!(base("nested/dir/report.json").normalize().is_ok());

    // Traversal is the arbitrary-write primitive and must never survive to
    // the filesystem, at any position in the path.
    for path in [
        "../escape.json",
        "..",
        "a/../../escape.json",
        "nested/../../../escape.json",
    ] {
        let err = base(path).normalize().unwrap_err();
        assert!(
            err.to_string().contains(".."),
            "traversal {path:?} was not rejected: {err}"
        );
    }
    // NUL truncation and empty destinations.
    assert!(base("report\0.json").normalize().is_err());
    assert!(base("   ").normalize().is_err());
    assert!(base(&"x".repeat(5000)).normalize().is_err());
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
        host: None,
        port: None,
        database: None,
        username: None,
        max_connections: None,
        mode: None,
        scan_id: None,
        cve_id: None,
        severity_filter: None,
        password_env: None,
    }
    .normalize()
    .unwrap();
    assert_eq!(canonical.storage_type, "findings");

    let params = serde_json::json!({"storage_type": "findings"});
    assert_eq!(validate_tool_params("storage", &params).unwrap(), "storage");
    assert!(validate_tool_params("storage", &serde_json::json!({"storage_type": "   "})).is_err());
}

/// Storage connection settings and mode must survive the wire DTO.
///
/// Regression guard: every one of these was accepted by the wire DTO and
/// dropped before reaching the executor, which ran with
/// `StorageConfig::default()` and the mode string `"read"` — a mode the
/// executor does not implement, so the call always ended in
/// "Unknown storage mode: read" after dialling the default host.
#[test]
fn storage_config_and_mode_converge_across_wire_dto() {
    let canonical = StorageRequest {
        storage_type: "postgres".into(),
        path: None,
        host: Some("db.internal".into()),
        port: Some(6543),
        database: Some("findings_db".into()),
        username: Some("analyst".into()),
        max_connections: Some(20),
        mode: Some("LIST_FINDINGS".into()),
        scan_id: Some("scan-7".into()),
        cve_id: None,
        severity_filter: Some("High".into()),
        password_env: Some("EGSEC_PG_PASSWORD".into()),
    };
    let normalized = canonical.normalize().unwrap();
    assert_eq!(normalized.host, "db.internal");
    assert_eq!(normalized.port, 6543);
    assert_eq!(normalized.database, "findings_db");
    assert_eq!(normalized.username, "analyst");
    assert_eq!(normalized.max_connections, 20);
    assert_eq!(normalized.mode, "list_findings");
    assert_eq!(normalized.scan_id.as_deref(), Some("scan-7"));
    assert_eq!(normalized.severity_filter.as_deref(), Some("high"));
    // Only the variable *name* is carried; the secret never is.
    assert_eq!(
        normalized.password_env.as_deref(),
        Some("EGSEC_PG_PASSWORD")
    );

    let runtime = runtime_adapters::storage_from_runtime(&eggsec_runtime::request::StorageParams {
        storage_type: "postgres".into(),
        path: None,
        host: Some("db.internal".into()),
        port: Some(6543),
        database: Some("findings_db".into()),
        username: Some("analyst".into()),
        max_connections: Some(20),
        mode: Some("LIST_FINDINGS".into()),
        scan_id: Some("scan-7".into()),
        cve_id: None,
        severity_filter: Some("High".into()),
        password_env: Some("EGSEC_PG_PASSWORD".into()),
    });
    assert_eq!(runtime.normalize().unwrap(), normalized);
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

/// The UDP transport flag must survive the wire DTO.
///
/// `udp` is a sibling of `scan_type`, not a value of it: `scan_type` is a TCP
/// *technique*, and folding UDP in there would let a scan type bypass technique
/// validation and be swallowed by the engine's `_ => Syn` fallback.
#[test]
fn scanner_udp_transport_converges_across_wire_dto() {
    let canonical = PortScanRequest {
        target: "127.0.0.1".into(),
        ports: Some("53".into()),
        scan_type: Some("syn".into()),
        timeout_ms: None,
        concurrency: None,
        udp: Some(true),
    };
    let normalized = canonical.normalize().unwrap();
    assert!(normalized.udp);
    // A TCP technique is still required and still validated: `udp` does not
    // make an invalid technique acceptable.
    assert_eq!(normalized.scan_type.as_str(), "syn");

    let runtime =
        runtime_adapters::port_scan_from_runtime(&eggsec_runtime::request::PortScanParams {
            target: "127.0.0.1".into(),
            ports: Some("53".into()),
            scan_type: Some("syn".into()),
            timeout_ms: None,
            concurrency: None,
            udp: Some(true),
        });
    assert_eq!(runtime.normalize().unwrap(), normalized);

    // `udp` is `#[serde(default)]`, so a payload written before it existed
    // still deserializes and means TCP.
    let legacy = serde_json::json!({"target": "127.0.0.1"});
    let req: PortScanRequest = serde_json::from_value(legacy).expect("legacy payload parses");
    assert!(!req.normalize().unwrap().udp);
}
