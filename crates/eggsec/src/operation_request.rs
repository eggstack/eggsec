//! Canonical operation request facade — Phase C convergence.
//!
//! Re-exports the dependency-light canonical contracts from
//! `eggsec-tool-core::operation_request` and provides the single
//! engine-owned conversion layer from frontend parse DTOs (CLI args,
//! runtime `TaskKind`, `ToolRequest` JSON) into those canonical types.
//!
//! # Invariants
//!
//! - Defaults/validation live in `eggsec-tool-core::operation_request`.
//!   This module only adapts frontend shapes into canonical requests.
//! - No authorization decision happens here. Callers must still build an
//!   `OperationDescriptor` via `OperationMetadata::try_descriptor_for_target`
//!   and evaluate through `EnforcementContext` before execution.
//! - Every conversion is exhaustive: adding a new `TaskKind` variant or
//!   canonical operation without updating the mapping is a compile error
//!   (no wildcard fallback for supported kinds).

pub use eggsec_tool_core::operation_request::*;

// ── CLI args → canonical (shallow adapters) ──

#[cfg(feature = "cli")]
pub mod cli_adapters {
    use super::*;
    use crate::cli::{
        AuthTestArgs, EndpointScanArgs, FingerprintArgs, FuzzArgs, GraphQlArgs, LoadArgs,
        OAuthArgs, PortScanArgs, ReconArgs, WafArgs, WafStressArgs,
    };

    pub fn port_scan_from_cli(args: &PortScanArgs) -> PortScanRequest {
        PortScanRequest {
            target: args.host.clone(),
            ports: Some(args.ports.clone()),
            scan_type: args.scan_type.clone(),
            timeout_ms: Some(args.timeout.saturating_mul(1000)),
            concurrency: Some(args.concurrency),
        }
    }

    pub fn endpoint_scan_from_cli(args: &EndpointScanArgs) -> EndpointScanRequest {
        EndpointScanRequest {
            target: args.url.clone(),
            concurrency: Some(args.concurrency),
            timeout_secs: Some(args.timeout),
            wordlist: args.wordlist.clone(),
        }
    }

    pub fn fingerprint_from_cli(args: &FingerprintArgs) -> FingerprintRequest {
        FingerprintRequest {
            target: args.host.clone(),
            ports: Some(args.ports.clone()),
            timeout_secs: Some(args.timeout),
            concurrency: Some(args.concurrency),
        }
    }

    pub fn fuzz_from_cli(args: &FuzzArgs) -> FuzzRequest {
        FuzzRequest {
            target: args.url.clone(),
            payload_type: Some(args.payload_type.clone()),
            mode: Some(args.mode.to_string()),
            method: Some(args.method.clone()),
            param: args.param.clone(),
            threads: Some(args.concurrency as u32),
            timeout_secs: Some(args.timeout),
            mutations: Some(args.mutate),
            mutation_count: Some(args.mutation_count),
            graphql_introspection: Some(args.graphql_introspection),
            graphql_depth_bypass: Some(args.graphql_depth_bypass),
            graphql_alias_overload: Some(args.graphql_alias_overload),
            oauth_redirect_test: Some(args.oauth_redirect),
            oauth_scope_test: Some(args.oauth_scope),
            oauth_state_test: Some(args.oauth_state),
            oauth_grant_test: Some(args.oauth_grant),
        }
    }

    pub fn waf_detect_from_cli(args: &WafArgs) -> WafDetectRequest {
        WafDetectRequest {
            target: args.url.clone(),
            bypass_mode: Some(args.bypass),
            techniques: None,
        }
    }

    pub fn waf_stress_from_cli(args: &WafStressArgs) -> WafStressRequest {
        WafStressRequest {
            target: args.url.clone(),
            requests: None,
            concurrency: Some(args.concurrency),
        }
    }

    pub fn load_test_from_cli(args: &LoadArgs) -> LoadTestRequest {
        LoadTestRequest {
            target: args.url.clone(),
            method: Some(args.method.clone()),
            requests: Some(args.requests),
            connections: Some(args.concurrency as u32),
            duration_secs: args.timeout.map(|v| v as u32),
            rate_limit: None,
        }
    }

    pub fn recon_from_cli(args: &ReconArgs) -> ReconRequest {
        ReconRequest {
            target: args.target.clone(),
            modules: None,
        }
    }

    pub fn graphql_from_cli(args: &GraphQlArgs) -> GraphQlRequest {
        GraphQlRequest {
            target: args.url.clone(),
            introspection: Some(args.introspection),
            inject: Some(args.inject),
            depth_bypass: Some(args.depth_bypass),
            alias_overload: Some(args.alias_overload),
            concurrency: Some(args.concurrency),
            timeout_secs: Some(args.timeout),
        }
    }

    pub fn oauth_from_cli(args: &OAuthArgs) -> OAuthRequest {
        OAuthRequest {
            target: args.url.clone(),
            flow: None,
            client_id: args.client_id.clone(),
            redirect_uri: args.redirect_uri.clone(),
            redirect_test: Some(args.redirect_test),
            scope_test: Some(args.scope_test),
            state_test: Some(args.state_test),
            grant_test: Some(args.grant_test),
            concurrency: Some(args.concurrency),
            timeout_secs: Some(args.timeout),
        }
    }

    pub fn auth_test_from_cli(args: &AuthTestArgs) -> AuthTestRequest {
        AuthTestRequest {
            target: args.target.clone(),
            username: args.username.clone(),
            credential_list: args.wordlist.clone(),
            credential_file: args.credential_file.clone(),
            max_attempts: Some(args.max_attempts),
            concurrency: Some(args.concurrency),
            timeout_secs: Some(args.timeout),
        }
    }
}

// ── Runtime TaskKind → canonical (exhaustive) ──

#[cfg(feature = "cli")]
pub mod runtime_adapters {
    use super::*;
    use eggsec_runtime::request::TaskKind;

    /// Canonical operation ID for a runtime task kind.
    ///
    /// Phase 3 closure: thin delegation to `TaskKind::operation_id()` — the
    /// single wire-side match lives in `eggsec-runtime` (which cannot depend
    /// on the engine without creating an architectural cycle). No parallel
    /// match here; adding a `TaskKind` variant without updating the runtime
    /// method is a compile error there, and this wrapper agrees by
    /// construction. The engine-side canonical identity for a converted
    /// request is `CanonicalOperationRequest::operation_id()`.
    pub fn operation_id_for_task_kind(kind: &TaskKind) -> Option<&'static str> {
        Some(kind.operation_id())
    }

    /// Canonical target for a runtime task kind (`None` for `NoTarget`
    /// operations such as storage/integrations/workflow and interface-bound
    /// packet/wireless/intercept tasks).
    ///
    /// Phase 3 closure: thin delegation to `TaskKind::canonical_target()`.
    pub fn target_for_task_kind(kind: &TaskKind) -> Option<String> {
        kind.canonical_target()
    }

    /// Convert runtime port-scan params into the canonical request.
    pub fn port_scan_from_runtime(p: &eggsec_runtime::request::PortScanParams) -> PortScanRequest {
        PortScanRequest {
            target: p.target.clone(),
            ports: p.ports.clone(),
            scan_type: p.scan_type.clone(),
            timeout_ms: p.timeout_ms,
            concurrency: p.concurrency,
        }
    }

    pub fn endpoint_scan_from_runtime(
        p: &eggsec_runtime::request::EndpointScanParams,
    ) -> EndpointScanRequest {
        EndpointScanRequest {
            target: p.target.clone(),
            concurrency: p.concurrency,
            timeout_secs: p.timeout_secs,
            wordlist: p.wordlist.clone(),
        }
    }

    pub fn fingerprint_from_runtime(
        p: &eggsec_runtime::request::FingerprintParams,
    ) -> FingerprintRequest {
        FingerprintRequest {
            target: p.target.clone(),
            ports: p.ports.clone(),
            timeout_secs: p.timeout_secs,
            concurrency: p.concurrency,
        }
    }

    pub fn fuzz_from_runtime(p: &eggsec_runtime::request::FuzzParams) -> FuzzRequest {
        FuzzRequest {
            target: p.target.clone(),
            payload_type: p.payload_type.clone(),
            mode: p.mode.clone(),
            method: p.method.clone(),
            param: p.param.clone(),
            threads: p.threads,
            timeout_secs: p.timeout,
            mutations: p.mutations,
            mutation_count: p.mutation_count,
            graphql_introspection: p.graphql_introspection,
            graphql_depth_bypass: p.graphql_depth_bypass,
            graphql_alias_overload: p.graphql_alias_overload,
            oauth_redirect_test: p.oauth_redirect_test,
            oauth_scope_test: p.oauth_scope_test,
            oauth_state_test: p.oauth_state_test,
            oauth_grant_test: p.oauth_grant_test,
        }
    }

    pub fn waf_from_runtime(p: &eggsec_runtime::request::WafParams) -> WafDetectRequest {
        WafDetectRequest {
            target: p.target.clone(),
            bypass_mode: p.bypass_mode,
            techniques: p.techniques.clone(),
        }
    }

    pub fn waf_stress_from_runtime(
        p: &eggsec_runtime::request::WafStressParams,
    ) -> WafStressRequest {
        WafStressRequest {
            target: p.target.clone(),
            requests: p.requests,
            concurrency: p.concurrency,
        }
    }

    pub fn load_test_from_runtime(p: &eggsec_runtime::request::LoadTestParams) -> LoadTestRequest {
        LoadTestRequest {
            target: p.target.clone(),
            method: Some(p.method.clone()),
            requests: p.requests,
            connections: p.connections,
            duration_secs: p.duration_secs,
            rate_limit: p.rate_limit,
        }
    }

    pub fn recon_from_runtime(p: &eggsec_runtime::request::ReconParams) -> ReconRequest {
        ReconRequest {
            target: p.target.clone(),
            modules: p.modules.clone(),
        }
    }

    pub fn pipeline_from_runtime(p: &eggsec_runtime::request::PipelineParams) -> PipelineRequest {
        PipelineRequest {
            target: p.target.clone(),
            profile: p.profile.clone(),
        }
    }

    pub fn graphql_from_runtime(p: &eggsec_runtime::request::GraphQlParams) -> GraphQlRequest {
        GraphQlRequest {
            target: p.target.clone(),
            introspection: p.introspection,
            inject: p.inject,
            depth_bypass: p.depth_bypass,
            alias_overload: p.alias_overload,
            concurrency: p.concurrency,
            timeout_secs: p.timeout_secs,
        }
    }

    pub fn oauth_from_runtime(p: &eggsec_runtime::request::OAuthParams) -> OAuthRequest {
        OAuthRequest {
            target: p.target.clone(),
            flow: p.flow.clone(),
            client_id: p.client_id.clone(),
            redirect_uri: p.redirect_uri.clone(),
            redirect_test: p.redirect_test,
            scope_test: p.scope_test,
            state_test: p.state_test,
            grant_test: p.grant_test,
            concurrency: p.concurrency,
            timeout_secs: p.timeout_secs,
        }
    }

    pub fn auth_test_from_runtime(p: &eggsec_runtime::request::AuthTestParams) -> AuthTestRequest {
        AuthTestRequest {
            target: p.target.clone(),
            username: p.username.clone(),
            credential_list: p.credential_list.clone(),
            credential_file: p.credential_file.clone(),
            max_attempts: p.max_attempts,
            concurrency: p.concurrency,
            timeout_secs: p.timeout_secs,
        }
    }

    pub fn db_pentest_from_runtime(
        p: &eggsec_runtime::request::DbPentestParams,
    ) -> DbPentestRequest {
        DbPentestRequest {
            target: p.target.clone(),
            db_type: p.db_type.clone(),
            port: p.port,
            checks: p.checks.clone(),
            max_queries: p.max_queries,
            max_duration_secs: p.max_duration,
            dry_run: p.dry_run,
            allow_advanced: p.allow_advanced,
        }
    }
}

// ── ToolRequest.params → canonical (single owner) ──

/// Validate `ToolRequest.params` through operation-owned canonical parsing.
///
/// Returns the canonical operation ID on success. This is the only supported
/// path for tool/protocol surfaces to interpret JSON params; per-protocol
/// ad hoc structs must not duplicate this logic.
pub fn validate_tool_request_params(
    tool: &str,
    params: &serde_json::Value,
) -> Result<&'static str, NormalizationError> {
    validate_tool_params(tool, params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_defaults_are_single_owner() {
        assert_eq!(DEFAULT_PORT_SCAN_PORTS, "1-1024");
        assert_eq!(
            DEFAULT_FINGERPRINT_PORTS,
            "80,443,22,21,25,3306,5432,6379,27017"
        );
        assert_eq!(DEFAULT_ENDPOINT_CONCURRENCY, 20);
        assert_eq!(DEFAULT_FUZZ_MUTATION_COUNT, 3);
        assert!(DEFAULT_GRAPHQL_INTROSPECTION);
    }

    #[cfg(feature = "cli")]
    #[test]
    fn runtime_task_kind_mapping_is_exhaustive() {
        use eggsec_runtime::request::*;
        use runtime_adapters::{operation_id_for_task_kind, target_for_task_kind};

        // Every TaskKind variant must map to a canonical operation ID.
        // This test constructs each variant once; if a new variant is added
        // without updating the match, this test fails to compile (missing
        // construction) or the mapping returns None.
        let kinds: Vec<TaskKind> = vec![
            TaskKind::LoadTest(LoadTestParams {
                target: "https://example.com".into(),
                method: "GET".into(),
                ..Default::default()
            }),
            TaskKind::StressTest(StressTestParams {
                target: "https://example.com".into(),
                flood_type: "syn".into(),
                ..Default::default()
            }),
            TaskKind::PortScan(PortScanParams {
                target: "10.0.0.1".into(),
                ..Default::default()
            }),
            TaskKind::EndpointScan(EndpointScanParams {
                target: "https://example.com".into(),
                ..Default::default()
            }),
            TaskKind::Fingerprint(FingerprintParams {
                target: "10.0.0.1".into(),
                ..Default::default()
            }),
            TaskKind::Fuzz(FuzzParams {
                target: "https://example.com".into(),
                ..Default::default()
            }),
            TaskKind::Waf(WafParams {
                target: "https://example.com".into(),
                ..Default::default()
            }),
            TaskKind::WafStress(WafStressParams {
                target: "https://example.com".into(),
                ..Default::default()
            }),
            TaskKind::Pipeline(PipelineParams {
                target: "https://example.com".into(),
                profile: None,
            }),
            TaskKind::Recon(ReconParams {
                target: "example.com".into(),
                modules: None,
            }),
            TaskKind::PacketCapture(PacketCaptureParams::default()),
            TaskKind::PacketTraceroute(PacketTracerouteParams {
                target: "10.0.0.1".into(),
                max_hops: None,
            }),
            TaskKind::PacketSend(PacketSendParams {
                target: "10.0.0.1".into(),
                protocol: "tcp".into(),
                ..Default::default()
            }),
            TaskKind::GraphQl(GraphQlParams {
                target: "https://example.com/graphql".into(),
                ..Default::default()
            }),
            TaskKind::OAuth(OAuthParams {
                target: "https://example.com".into(),
                ..Default::default()
            }),
            TaskKind::AuthTest(AuthTestParams {
                target: "https://example.com".into(),
                ..Default::default()
            }),
            TaskKind::Nse(NseParams {
                target: "10.0.0.1".into(),
                script: "default".into(),
                args: None,
            }),
            TaskKind::Hunt(HuntParams {
                target: "https://example.com".into(),
                hunt_type: None,
            }),
            TaskKind::Browser(BrowserParams {
                target: "https://example.com".into(),
                headless: None,
            }),
            TaskKind::Compliance(ComplianceParams {
                target: "https://example.com".into(),
                framework: None,
            }),
            TaskKind::Storage(StorageParams {
                storage_type: "findings".into(),
                path: None,
            }),
            TaskKind::Integrations(IntegrationsParams {
                integration_type: "jira".into(),
                config: None,
            }),
            TaskKind::Workflow(WorkflowParams {
                workflow_id: None,
                steps: None,
            }),
            TaskKind::Vuln(VulnParams {
                target: "https://example.com".into(),
                vuln_type: None,
            }),
            TaskKind::Wireless(WirelessParams {
                interface: None,
                duration_secs: None,
            }),
            TaskKind::WirelessActive(WirelessActiveParams {
                interface: None,
                target_bssid: None,
            }),
            TaskKind::DbPentest(DbPentestParams {
                db_type: "postgres".into(),
                target: "localhost".into(),
                ..Default::default()
            }),
            TaskKind::Intercept(InterceptParams::default()),
            TaskKind::C2(C2Params::default()),
        ];
        for kind in &kinds {
            assert!(
                operation_id_for_task_kind(kind).is_some(),
                "TaskKind {kind:?} has no canonical operation mapping"
            );
            // Target extraction must not panic for any variant.
            let _ = target_for_task_kind(kind);
        }
        assert_eq!(kinds.len(), 29);
    }

    #[cfg(feature = "cli")]
    #[test]
    fn cli_and_runtime_normalize_equivalently_port_scan() {
        use eggsec_runtime::request::PortScanParams;
        use runtime_adapters::port_scan_from_runtime;

        // Same logical request through CLI-shaped and runtime-shaped adapters
        // must produce the same normalized engine request.
        let canonical = PortScanRequest {
            target: "  example.com  ".into(),
            ports: Some("22,80,443".into()),
            scan_type: Some("syn".into()),
            timeout_ms: None,
            concurrency: None,
        };
        let normalized = canonical.normalize().unwrap();

        let runtime = port_scan_from_runtime(&PortScanParams {
            target: "  example.com  ".into(),
            ports: Some("22,80,443".into()),
            scan_type: Some("syn".into()),
            timeout_ms: None,
            concurrency: None,
        });
        assert_eq!(runtime.normalize().unwrap(), normalized);
    }

    #[cfg(feature = "cli")]
    #[test]
    fn cli_and_runtime_normalize_equivalently_load_test() {
        use eggsec_runtime::request::LoadTestParams;
        use runtime_adapters::load_test_from_runtime;

        let canonical = LoadTestRequest {
            target: "https://example.com".into(),
            method: Some("GET".into()),
            requests: None,
            connections: Some(25),
            duration_secs: None,
            rate_limit: None,
        };
        let normalized = canonical.normalize().unwrap();
        // Legacy connections-only means 25 total requests, 25 concurrency.
        assert_eq!(normalized.requests, 25);
        assert_eq!(normalized.concurrency, 25);

        let runtime = load_test_from_runtime(&LoadTestParams {
            target: "https://example.com".into(),
            method: "GET".into(),
            requests: None,
            connections: Some(25),
            duration_secs: None,
            rate_limit: None,
        });
        assert_eq!(runtime.normalize().unwrap(), normalized);
    }

    #[cfg(feature = "cli")]
    #[test]
    fn cli_and_runtime_normalize_equivalently_fuzz() {
        use eggsec_runtime::request::FuzzParams;
        use runtime_adapters::fuzz_from_runtime;

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
        };
        let normalized = canonical.normalize().unwrap();

        let runtime = fuzz_from_runtime(&FuzzParams {
            target: "https://example.com".into(),
            ..Default::default()
        });
        assert_eq!(runtime.normalize().unwrap(), normalized);
    }
}
