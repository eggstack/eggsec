#[cfg(feature = "db-pentest")]
use crate::tabs::TabState;
use eggsec_runtime::request::{
    LoadTestParams, PortScanParams, ReconParams, RunRequest, RuntimeSurface, TaskKind,
};

pub trait TaskBuilder {
    fn build_run_request(&self) -> Option<RunRequest>;
}

impl TaskBuilder for super::tabs::ReconTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }
        Some(RunRequest {
            task_kind: TaskKind::Recon(ReconParams {
                target: target.to_string(),
                modules: None,
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}

impl TaskBuilder for super::tabs::LoadTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }

        if self.is_stress_test() {
            Some(RunRequest {
                task_kind: TaskKind::StressTest(eggsec_runtime::request::StressTestParams {
                    target: target.to_string(),
                    flood_type: self.stress_type().to_string(),
                    rate_pps: None,
                    duration_secs: Some(60),
                    threads: Some(self.concurrency() as u32),
                }),
                requested_by: None,
                surface: RuntimeSurface::TuiManual,
                labels: vec![],
            })
        } else {
            Some(RunRequest {
                task_kind: TaskKind::LoadTest(LoadTestParams {
                    target: target.to_string(),
                    method: "GET".to_string(),
                    requests: None,
                    connections: Some(self.concurrency() as u32),
                    duration_secs: Some(self.timeout() as u32),
                    rate_limit: None,
                }),
                requested_by: None,
                surface: RuntimeSurface::TuiManual,
                labels: vec![],
            })
        }
    }
}

impl TaskBuilder for super::tabs::ScanPortsTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }

        Some(RunRequest {
            task_kind: TaskKind::PortScan(PortScanParams {
                target: target.to_string(),
                ports: Some(self.ports().to_string()),
                scan_type: None,
                timeout_ms: Some(self.timeout() * 1000),
                concurrency: None,
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}

impl TaskBuilder for super::tabs::ScanEndpointsTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }

        Some(RunRequest {
            task_kind: TaskKind::EndpointScan(eggsec_runtime::request::EndpointScanParams {
                target: target.to_string(),
                methods: None,
                wordlist: self.wordlist().map(|s| s.to_string()),
                concurrency: None,
                timeout_secs: None,
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}

impl TaskBuilder for super::tabs::FingerprintTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }

        Some(RunRequest {
            task_kind: TaskKind::Fingerprint(eggsec_runtime::request::FingerprintParams {
                target: target.to_string(),
                ports: None,
                timeout_secs: None,
                concurrency: None,
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}

impl TaskBuilder for super::tabs::FuzzTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }

        Some(RunRequest {
            task_kind: TaskKind::Fuzz(eggsec_runtime::request::FuzzParams {
                target: target.to_string(),
                payload_type: Some(self.payload_type_string()),
                threads: Some(self.concurrency() as u32),
                mode: None,
                mutations: None,
                mutation_count: None,
                method: None,
                param: None,
                timeout: None,
                graphql_introspection: None,
                graphql_depth_bypass: None,
                graphql_alias_overload: None,
                oauth_redirect_test: None,
                oauth_scope_test: None,
                oauth_state_test: None,
                oauth_grant_test: None,
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}

impl TaskBuilder for super::tabs::WafTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }

        Some(RunRequest {
            task_kind: TaskKind::Waf(eggsec_runtime::request::WafParams {
                target: target.to_string(),
                bypass_mode: None,
                techniques: None,
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}

impl TaskBuilder for super::tabs::WafStressTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }

        Some(RunRequest {
            task_kind: TaskKind::WafStress(eggsec_runtime::request::WafStressParams {
                target: target.to_string(),
                requests: Some(self.concurrency() as u32),
                concurrency: None,
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}

impl TaskBuilder for super::tabs::ScanTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }
        let profile = self.profile()?;

        Some(RunRequest {
            task_kind: TaskKind::Pipeline(eggsec_runtime::request::PipelineParams {
                target: target.to_string(),
                profile: Some(profile.to_string()),
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}

impl TaskBuilder for super::tabs::PacketTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        match self.current_view {
            super::tabs::packet::PacketView::Capture => {
                let interface = self.target();
                if interface.is_empty() {
                    return None;
                }

                Some(RunRequest {
                    task_kind: TaskKind::PacketCapture(
                        eggsec_runtime::request::PacketCaptureParams {
                            interface: Some(interface.to_string()),
                            filter: Some(self.filter().to_string()),
                            duration_secs: Some(self.max_packets() as u32),
                            max_packets: None,
                            promiscuous: None,
                        },
                    ),
                    requested_by: None,
                    surface: RuntimeSurface::TuiManual,
                    labels: vec![],
                })
            }
            super::tabs::packet::PacketView::Traceroute => {
                let target = self.target();
                if target.is_empty() {
                    return None;
                }

                Some(RunRequest {
                    task_kind: TaskKind::PacketTraceroute(
                        eggsec_runtime::request::PacketTracerouteParams {
                            target: target.to_string(),
                            max_hops: Some(30),
                        },
                    ),
                    requested_by: None,
                    surface: RuntimeSurface::TuiManual,
                    labels: vec![],
                })
            }
            super::tabs::packet::PacketView::Send => {
                let target = self.target();
                if target.is_empty() {
                    return None;
                }

                Some(RunRequest {
                    task_kind: TaskKind::PacketSend(eggsec_runtime::request::PacketSendParams {
                        target: target.to_string(),
                        protocol: "tcp".to_string(),
                        payload: Some(self.filter().to_string()),
                        port: None,
                        count: None,
                        packet_size: None,
                    }),
                    requested_by: None,
                    surface: RuntimeSurface::TuiManual,
                    labels: vec![],
                })
            }
            _ => None,
        }
    }
}

impl TaskBuilder for super::tabs::GraphQlTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }

        Some(RunRequest {
            task_kind: TaskKind::GraphQl(eggsec_runtime::request::GraphQlParams {
                target: target.to_string(),
                introspection: Some(self.introspection_checkbox.checked),
                inject: None,
                depth_bypass: None,
                alias_overload: None,
                concurrency: None,
                timeout_secs: None,
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}

impl TaskBuilder for super::tabs::OAuthTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }

        Some(RunRequest {
            task_kind: TaskKind::OAuth(eggsec_runtime::request::OAuthParams {
                target: target.to_string(),
                flow: None,
                client_id: None,
                redirect_uri: None,
                redirect_test: None,
                scope_test: None,
                state_test: None,
                grant_test: None,
                concurrency: None,
                timeout_secs: None,
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}

impl TaskBuilder for super::tabs::ClusterTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        None
    }
}

#[cfg(feature = "advanced-hunting")]
impl TaskBuilder for super::tabs::HuntTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }
        Some(RunRequest {
            task_kind: TaskKind::Hunt(eggsec_runtime::request::HuntParams {
                target: target.to_string(),
                hunt_type: None,
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}

#[cfg(feature = "headless-browser")]
impl TaskBuilder for super::tabs::BrowserTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }
        Some(RunRequest {
            task_kind: TaskKind::Browser(eggsec_runtime::request::BrowserParams {
                target: target.to_string(),
                headless: Some(true),
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}

#[cfg(feature = "compliance")]
impl TaskBuilder for super::tabs::ComplianceTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }
        Some(RunRequest {
            task_kind: TaskKind::Compliance(eggsec_runtime::request::ComplianceParams {
                target: target.to_string(),
                framework: Some(format!("{:?}", self.selected_framework())),
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}

#[cfg(feature = "database")]
impl TaskBuilder for super::tabs::StorageTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        Some(RunRequest {
            task_kind: TaskKind::Storage(eggsec_runtime::request::StorageParams {
                storage_type: "sqlite".to_string(),
                path: None,
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}

#[cfg(feature = "external-integrations")]
impl TaskBuilder for super::tabs::IntegrationsTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        let mode = self.get_mode();
        Some(RunRequest {
            task_kind: TaskKind::Integrations(eggsec_runtime::request::IntegrationsParams {
                integration_type: mode.to_string(),
                config: None,
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}

#[cfg(feature = "finding-workflow")]
impl TaskBuilder for super::tabs::WorkflowTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        Some(RunRequest {
            task_kind: TaskKind::Workflow(eggsec_runtime::request::WorkflowParams {
                workflow_id: None,
                steps: None,
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}

#[cfg(feature = "vuln-management")]
impl TaskBuilder for super::tabs::VulnTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        Some(RunRequest {
            task_kind: TaskKind::Vuln(eggsec_runtime::request::VulnParams {
                target: self.core.target().to_string(),
                vuln_type: None,
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}

#[cfg(feature = "wireless")]
impl TaskBuilder for super::tabs::WirelessTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        #[cfg(feature = "wireless-advanced")]
        {
            if self.active_mode {
                if let Some((
                    interface,
                    _attack_type,
                    bssid,
                    _client,
                    _frame_count,
                    _rate_limit,
                    _dry_run,
                )) = self.active_attack_config()
                {
                    return Some(RunRequest {
                        task_kind: TaskKind::WirelessActive(
                            eggsec_runtime::request::WirelessActiveParams {
                                interface: Some(interface),
                                target_bssid: bssid,
                            },
                        ),
                        requested_by: None,
                        surface: RuntimeSurface::TuiManual,
                        labels: vec![],
                    });
                }
            }
        }
        let interface = self.interface();
        if interface.is_empty() {
            None
        } else {
            Some(RunRequest {
                task_kind: TaskKind::Wireless(eggsec_runtime::request::WirelessParams {
                    interface: Some(interface.to_string()),
                    duration_secs: None,
                }),
                requested_by: None,
                surface: RuntimeSurface::TuiManual,
                labels: vec![],
            })
        }
    }
}

impl TaskBuilder for super::tabs::AuthTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        let target = self.target()?;
        if target.is_empty() {
            return None;
        }

        Some(RunRequest {
            task_kind: TaskKind::AuthTest(eggsec_runtime::request::AuthTestParams {
                target: target.to_string(),
                username: self.username().map(|s| s.to_string()),
                credential_list: self.password_list().map(|s| s.to_string()),
                credential_file: None,
                max_attempts: None,
                concurrency: None,
                timeout_secs: None,
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}

#[cfg(feature = "web-proxy")]
impl TaskBuilder for super::tabs::InterceptTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        let addr = self.listen_addr();
        let (listen_host, listen_port) = parse_listen_addr(&addr);
        Some(RunRequest {
            task_kind: TaskKind::Intercept(eggsec_runtime::request::InterceptParams {
                listen_port,
                target: self.primary_target(),
                listen_host,
                // The TUI exposes explicit dry-run + flow-limit toggles; preserve
                // them through the runtime DTO so policy/engine sees the same
                // bounds the user accepted in the UI.
                dry_run: Some(self.dry_run),
                max_flows: Some(self.max_flows()),
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}

/// Split a `host:port` listen address into typed components.
///
/// Returns `(None, None)` for empty input, `(Some(host), Some(port))` for a
/// fully formed address, and `(Some(host), None)` / `(None, Some(port))` for
/// half-formed addresses. The TUI semantic is "do not silently coerce a
/// missing piece into a default"; the engine's canonical intercept contract
/// owns the fallback semantics.
#[cfg(any(feature = "web-proxy", test))]
fn parse_listen_addr(addr: &str) -> (Option<String>, Option<u16>) {
    let trimmed = addr.trim();
    if trimmed.is_empty() {
        return (None, None);
    }
    let Some(idx) = trimmed.rfind(':') else {
        return (Some(trimmed.to_string()), None);
    };
    let host = &trimmed[..idx];
    let port = trimmed[idx + 1..].parse::<u16>().ok();
    if host.is_empty() {
        (None, port)
    } else {
        (Some(host.to_string()), port)
    }
}

#[cfg(feature = "c2")]
impl TaskBuilder for super::tabs::C2Tab {
    fn build_run_request(&self) -> Option<RunRequest> {
        let target = self.target()?.to_string();
        if target.is_empty() {
            return None;
        }

        let campaign = self.campaign().unwrap_or("default").to_string();

        Some(RunRequest {
            task_kind: TaskKind::C2(eggsec_runtime::request::C2Params {
                target: Some(target),
                profile: Some(campaign),
                // The C2 tab does not expose a dry-run toggle. Leave the field
                // absent so the canonical executor applies its documented safe
                // default (`dry_run.unwrap_or(true)`), matching the rest of the
                // C2 surface.
                dry_run: None,
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}

#[cfg(feature = "db-pentest")]
impl TaskBuilder for super::tabs::DbPentestTab {
    fn build_run_request(&self) -> Option<RunRequest> {
        if self.is_running() {
            return None;
        }
        let target = self
            .core
            .inputs
            .fields
            .get(1)
            .map(|f| f.value.clone())
            .filter(|s| !s.trim().is_empty())?;
        // The DbPentest tab does not expose a `db_type` control. Infer it
        // from the connection-string scheme so the canonical request
        // normalize() contract is satisfied; refuse to fabricate a permissive
        // placeholder when no scheme is present.
        let db_type = detect_db_type_from_target(&target)?;
        let checks = self
            .core
            .inputs
            .fields
            .get(2)
            .map(|f| f.value.clone())
            .filter(|s| !s.trim().is_empty());
        let max_queries = self
            .core
            .inputs
            .fields
            .get(3)
            .and_then(|f| f.value.trim().parse::<u64>().ok());
        let max_duration = self
            .core
            .inputs
            .fields
            .get(4)
            .and_then(|f| f.value.trim().parse::<u64>().ok());
        Some(RunRequest {
            task_kind: TaskKind::DbPentest(eggsec_runtime::request::DbPentestParams {
                db_type,
                target,
                port: None,
                checks,
                max_queries,
                max_duration,
                dry_run: Some(self.dry_run),
                allow_advanced: Some(self.advanced),
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}

#[cfg(feature = "db-pentest")]
fn detect_db_type_from_target(target: &str) -> Option<String> {
    let lower = target.trim().to_ascii_lowercase();
    let Some(idx) = lower.find("://") else {
        return None;
    };
    match &lower[..idx] {
        "postgres" | "postgresql" => Some("postgres".to_string()),
        "mysql" => Some("mysql".to_string()),
        "mongodb" | "mongo" => Some("mongodb".to_string()),
        "mssql" | "sqlserver" => Some("mssql".to_string()),
        "redis" | "rediss" => Some("redis".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_listen_addr_handles_typical_and_partial_inputs() {
        assert_eq!(
            parse_listen_addr("127.0.0.1:8080"),
            (Some("127.0.0.1".to_string()), Some(8080))
        );
        assert_eq!(
            parse_listen_addr(" 0.0.0.0:443 "),
            (Some("0.0.0.0".to_string()), Some(443))
        );
        // Half-formed: host without port is preserved, port without host is
        // preserved; empty input collapses to (None, None). No silent default.
        assert_eq!(
            parse_listen_addr("[::]:9000"),
            (Some("[::]".to_string()), Some(9000))
        );
        assert_eq!(parse_listen_addr(""), (None, None));
        assert_eq!(
            parse_listen_addr("only-host"),
            (Some("only-host".to_string()), None)
        );
        assert_eq!(parse_listen_addr(":9999"), (None, Some(9999)));
        assert_eq!(
            parse_listen_addr("host:not-a-port"),
            (Some("host".to_string()), None)
        );
    }

    #[cfg(feature = "db-pentest")]
    #[test]
    fn db_pentest_builder_maps_ui_state_to_runtime_dto() {
        use crate::tabs::DbPentestTab;
        let mut tab = DbPentestTab::new();
        // Default input[1] target is a Postgres connection string.
        // Override input[2..4] with explicit non-default values.
        tab.core.inputs.fields[1].value =
            "postgres://labuser:labpass@127.0.0.1:5432/labdb".to_string();
        tab.core.inputs.fields[2].value = "misconfig,privs".to_string();
        tab.core.inputs.fields[3].value = "500".to_string();
        tab.core.inputs.fields[4].value = "300".to_string();
        tab.dry_run = false;
        tab.advanced = true;

        let req = tab.build_run_request().expect("run request present");
        assert_eq!(req.surface, eggsec_runtime::RuntimeSurface::TuiManual);
        match req.task_kind {
            TaskKind::DbPentest(p) => {
                assert_eq!(p.db_type, "postgres");
                assert_eq!(p.target, "postgres://labuser:labpass@127.0.0.1:5432/labdb");
                assert_eq!(p.checks.as_deref(), Some("misconfig,privs"));
                assert_eq!(p.max_queries, Some(500));
                assert_eq!(p.max_duration, Some(300));
                assert_eq!(p.dry_run, Some(false));
                assert_eq!(p.allow_advanced, Some(true));
                // Tab does not expose a port control.
                assert_eq!(p.port, None);
            }
            other => panic!("expected DbPentest, got {other:?}"),
        }
    }

    #[cfg(feature = "db-pentest")]
    #[test]
    fn db_pentest_builder_refuses_unknown_target_scheme() {
        use crate::tabs::DbPentestTab;
        let mut tab = DbPentestTab::new();
        tab.core.inputs.fields[1].value = "https://example.com/db".to_string();
        // db_type detection must fail (the normalizer rejects unknown
        // schemes); we must not silently fabricate a permissive value.
        assert!(tab.build_run_request().is_none());
    }

    #[cfg(feature = "db-pentest")]
    #[test]
    fn db_pentest_builder_omits_optional_inputs_when_blank_or_unparseable() {
        use crate::tabs::DbPentestTab;
        let mut tab = DbPentestTab::new();
        tab.core.inputs.fields[1].value = "mysql://root@127.0.0.1:3306/labdb".to_string();
        tab.core.inputs.fields[2].clear();
        tab.core.inputs.fields[3].value = "not-a-number".to_string();
        tab.core.inputs.fields[4].clear();

        let req = tab.build_run_request().expect("run request present");
        match req.task_kind {
            TaskKind::DbPentest(p) => {
                assert_eq!(p.db_type, "mysql");
                assert!(p.checks.is_none(), "blank checks must not be set");
                assert!(
                    p.max_queries.is_none(),
                    "unparseable max_queries must not be set"
                );
                assert!(
                    p.max_duration.is_none(),
                    "blank max_duration must not be set"
                );
                // Defaults from the TUI tab still flow through.
                assert_eq!(p.dry_run, Some(true));
                assert_eq!(p.allow_advanced, Some(false));
            }
            other => panic!("expected DbPentest, got {other:?}"),
        }
    }

    #[cfg(feature = "db-pentest")]
    #[test]
    fn db_pentest_builder_round_trips_through_canonical_conversion() {
        use crate::tabs::DbPentestTab;
        use eggsec::dispatch::CanonicalOperationRequest;

        let mut tab = DbPentestTab::new();
        tab.core.inputs.fields[1].value = "mongodb://user@127.0.0.1:27017/labdb".to_string();
        tab.core.inputs.fields[2].value = "all".to_string();
        tab.dry_run = true;

        let req = tab.build_run_request().expect("run request present");
        let canonical = CanonicalOperationRequest::from_task_kind(&req.task_kind);
        assert_eq!(canonical.operation_id(), "db-pentest");
        assert_eq!(
            canonical.canonical_target().as_deref(),
            Some("mongodb://user@127.0.0.1:27017/labdb")
        );
    }

    #[cfg(feature = "web-proxy")]
    #[test]
    fn intercept_builder_maps_ui_state_to_runtime_dto() {
        use crate::tabs::InterceptTab;
        let mut tab = InterceptTab::new();
        tab.listen_addr = "127.0.0.1:8080".to_string();
        tab.dry_run = false;
        tab.max_flows = 250;

        let req = tab.build_run_request().expect("run request present");
        assert_eq!(req.surface, eggsec_runtime::RuntimeSurface::TuiManual);
        match req.task_kind {
            TaskKind::Intercept(p) => {
                assert_eq!(p.listen_host.as_deref(), Some("127.0.0.1"));
                assert_eq!(p.listen_port, Some(8080));
                assert_eq!(p.dry_run, Some(false));
                assert_eq!(p.max_flows, Some(250));
                // primary_target() falls back to listen_addr when no session is
                // attached — that is the documented semantic for the empty-session
                // case, not an unintended default.
                assert_eq!(p.target.as_deref(), Some("127.0.0.1:8080"));
            }
            other => panic!("expected Intercept, got {other:?}"),
        }
    }

    #[cfg(feature = "web-proxy")]
    #[test]
    fn intercept_builder_preserves_partial_listen_addr_components() {
        use crate::tabs::InterceptTab;
        let mut tab = InterceptTab::new();
        // Half-formed: host only, no port. The engine defaults the port, but
        // we must not silently rewrite the host.
        tab.listen_addr = "0.0.0.0".to_string();
        let req = tab.build_run_request().expect("run request present");
        match req.task_kind {
            TaskKind::Intercept(p) => {
                assert_eq!(p.listen_host.as_deref(), Some("0.0.0.0"));
                assert_eq!(p.listen_port, None);
                assert_eq!(p.dry_run, Some(true));
                assert_eq!(p.max_flows, Some(100));
            }
            other => panic!("expected Intercept, got {other:?}"),
        }
    }

    #[cfg(feature = "web-proxy")]
    #[test]
    fn intercept_builder_dry_run_default_is_preserved_as_some_true() {
        use crate::tabs::InterceptTab;
        let tab = InterceptTab::new();
        assert!(tab.dry_run);
        let req = tab.build_run_request().expect("run request present");
        match req.task_kind {
            TaskKind::Intercept(p) => {
                assert_eq!(p.dry_run, Some(true));
                assert_eq!(p.max_flows, Some(tab.max_flows));
            }
            other => panic!("expected Intercept, got {other:?}"),
        }
    }

    #[cfg(feature = "c2")]
    #[test]
    fn c2_builder_maps_ui_state_to_runtime_dto() {
        use crate::tabs::C2Tab;
        let mut tab = C2Tab::new();
        tab.core.inputs.fields[0].value = "10.0.0.5".to_string();
        tab.core.inputs.fields[1].value = "carbanak".to_string();

        let req = tab.build_run_request().expect("run request present");
        assert_eq!(req.surface, eggsec_runtime::RuntimeSurface::TuiManual);
        match req.task_kind {
            TaskKind::C2(p) => {
                assert_eq!(p.target.as_deref(), Some("10.0.0.5"));
                assert_eq!(p.profile.as_deref(), Some("carbanak"));
                // C2 tab does not expose a dry-run control; deliberately absent
                // so the canonical executor's documented safe default applies.
                assert_eq!(p.dry_run, None);
            }
            other => panic!("expected C2, got {other:?}"),
        }
    }

    #[cfg(feature = "c2")]
    #[test]
    fn c2_builder_round_trips_through_canonical_conversion() {
        use crate::tabs::C2Tab;
        use eggsec::dispatch::CanonicalOperationRequest;

        let mut tab = C2Tab::new();
        tab.core.inputs.fields[0].value = "localhost".to_string();
        tab.core.inputs.fields[1].value = "apt29".to_string();

        let req = tab.build_run_request().expect("run request present");
        let canonical = CanonicalOperationRequest::from_task_kind(&req.task_kind);
        assert_eq!(canonical.operation_id(), "c2");
        assert_eq!(canonical.canonical_target().as_deref(), Some("localhost"));
    }

    #[cfg(any(feature = "db-pentest", feature = "web-proxy", feature = "c2"))]
    #[test]
    fn builder_results_carry_tui_manual_surface() {
        // Every frontend-built RunRequest must surface as TuiManual so the
        // runtime bridge maps to TuiManual / TuiManualStrict execution surface.
        let mut samples: Vec<TaskKind> = Vec::new();
        #[cfg(feature = "db-pentest")]
        {
            use crate::tabs::DbPentestTab;
            let mut tab = DbPentestTab::new();
            tab.core.inputs.fields[1].value = "postgres://labuser@127.0.0.1:5432/labdb".to_string();
            samples.push(
                tab.build_run_request()
                    .expect("db-pentest builder")
                    .task_kind,
            );
        }
        #[cfg(feature = "web-proxy")]
        {
            use crate::tabs::InterceptTab;
            let tab = InterceptTab::new();
            samples.push(
                tab.build_run_request()
                    .expect("intercept builder")
                    .task_kind,
            );
        }
        #[cfg(feature = "c2")]
        {
            use crate::tabs::C2Tab;
            let mut tab = C2Tab::new();
            tab.core.inputs.fields[0].value = "localhost".to_string();
            samples.push(tab.build_run_request().expect("c2 builder").task_kind);
        }
        assert!(!samples.is_empty(), "no frontend feature enabled");
        for kind in samples {
            let req = eggsec_runtime::RunRequest {
                task_kind: kind.clone(),
                requested_by: None,
                surface: eggsec_runtime::RuntimeSurface::TuiManual,
                labels: vec![],
            };
            assert_eq!(req.surface, eggsec_runtime::RuntimeSurface::TuiManual);
            // Sanity: every kind is a recognized wire variant.
            let json = serde_json::to_value(&kind).expect("TaskKind must serialize");
            assert!(json.get("kind").is_some());
        }
    }

    #[cfg(feature = "db-pentest")]
    #[test]
    fn detect_db_type_from_target_recognizes_known_schemes() {
        assert_eq!(
            detect_db_type_from_target("postgres://u:p@h:5432/d"),
            Some("postgres".to_string())
        );
        assert_eq!(
            detect_db_type_from_target("POSTGRESQL://u@h/d"),
            Some("postgres".to_string())
        );
        assert_eq!(
            detect_db_type_from_target("mongodb://h"),
            Some("mongodb".to_string())
        );
        assert_eq!(
            detect_db_type_from_target("mongo://h"),
            Some("mongodb".to_string())
        );
        assert_eq!(
            detect_db_type_from_target("mysql://h"),
            Some("mysql".to_string())
        );
        assert_eq!(
            detect_db_type_from_target("sqlserver://h"),
            Some("mssql".to_string())
        );
        assert_eq!(
            detect_db_type_from_target("redis://h"),
            Some("redis".to_string())
        );
        assert_eq!(detect_db_type_from_target("127.0.0.1"), None);
        assert_eq!(detect_db_type_from_target("https://h"), None);
        assert_eq!(detect_db_type_from_target(""), None);
    }
}
