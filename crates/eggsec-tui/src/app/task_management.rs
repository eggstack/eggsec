#[cfg(feature = "db-pentest")]
use crate::tabs::TabState;
use eggsec_runtime::request::{
    LoadTestParams, PortScanParams, ReconParams, RunRequest, RuntimeSurface, TaskKind,
};

pub trait TaskBuilder {
    fn build_run_request(&self) -> Option<RunRequest>;
}

/// Environment variable the engine resolves the storage password from.
///
/// The password is deliberately not a wire field: `RunRequest` is persisted
/// verbatim into the daemon's SQLite snapshot store, so anything on the wire
/// ends up at rest in plaintext. The TUI publishes its password field into this
/// variable in the current process and sends only the name.
#[cfg(feature = "database")]
pub(crate) const EGGSEC_STORAGE_PASSWORD_ENV: &str = "EGSEC_STORAGE_PASSWORD";

/// Normalise an editable text input into a request value.
///
/// A blank or whitespace-only field is *absent* rather than an empty string:
/// the canonical normalizers read `None` as "use the documented default", so
/// `Some("")` would ship a literal empty value for a field the operator simply
/// never filled in. Surrounding whitespace is dropped.
fn non_blank(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
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
                    // The load tab exposes no rate control; absent lets the
                    // canonical executor apply its documented default.
                    rate_pps: None,
                    duration_secs: Some(self.timeout() as u32),
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
                    method: non_blank(self.method()).unwrap_or_else(|| "GET".to_string()),
                    requests: Some(self.requests()),
                    connections: Some(self.concurrency() as u32),
                    duration_secs: Some(self.timeout() as u32),
                    // The load tab exposes no rate-limit control.
                    rate_limit: None,
                    body: self.body().map(str::to_string),
                    headers: Some(self.headers()),
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
                ports: non_blank(self.ports()),
                // The tab exposes no scan-type control; the canonical default
                // (SYN) applies.
                scan_type: None,
                timeout_ms: Some(self.timeout() * 1000),
                concurrency: Some(self.concurrency()),
                // Sent explicitly rather than omitted: the checkbox is a
                // visible operator control, so an unticked box must be
                // distinguishable from a field that does not exist.
                udp: Some(self.udp()),
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
                // The tab exposes no method list control; the canonical default
                // method set applies.
                methods: None,
                wordlist: self.wordlist().and_then(non_blank),
                concurrency: Some(self.concurrency()),
                timeout_secs: Some(self.timeout()),
                // The tab's checkbox defaults to on, so an untouched tab keeps
                // 404 responses in the result set. Previously this hardcoded
                // false, so unchecking the box changed nothing.
                include_404: Some(self.include_404()),
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
                ports: non_blank(self.ports()),
                timeout_secs: Some(self.timeout()),
                // The tab exposes no concurrency control; the canonical default
                // applies.
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
                mode: non_blank(self.mode()),
                mutations: Some(self.mutations_enabled()),
                mutation_count: Some(self.mutation_count()),
                method: non_blank(self.method()),
                param: self.param().and_then(non_blank),
                timeout: Some(self.timeout()),
                graphql_introspection: Some(self.graphql_introspection_enabled()),
                graphql_depth_bypass: Some(self.graphql_depth_bypass_enabled()),
                graphql_alias_overload: Some(self.graphql_alias_overload_enabled()),
                oauth_redirect_test: Some(self.oauth_redirect_enabled()),
                oauth_scope_test: Some(self.oauth_scope_enabled()),
                oauth_state_test: Some(self.oauth_state_enabled()),
                oauth_grant_test: Some(self.oauth_grant_enabled()),
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
                bypass_mode: Some(self.is_bypass_mode()),
                techniques: Some(self.enabled_techniques()),
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
                // The tab exposes no request-count control; the canonical
                // default (100 requests) applies.
                requests: None,
                concurrency: Some(self.concurrency()),
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}

impl TaskBuilder for super::tabs::StressTab {
    /// `Tab::Stress` is a direct-launch tab whose `handle_enter` sets
    /// `AppState::Running`, but it had no `TaskBuilder` at all, so confirming
    /// the flood type could never dispatch anything. `StressTestParams`
    /// already carries every field the tab collects.
    fn build_run_request(&self) -> Option<RunRequest> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }

        let flood_type = match self.stress_type() {
            super::tabs::StressType::Http => "http",
            super::tabs::StressType::Syn => "syn",
            super::tabs::StressType::Udp => "udp",
            super::tabs::StressType::Tcp => "tcp",
            super::tabs::StressType::Icmp => "icmp",
        };

        Some(RunRequest {
            task_kind: TaskKind::StressTest(eggsec_runtime::request::StressTestParams {
                target: target.to_string(),
                flood_type: flood_type.to_string(),
                rate_pps: Some(self.rate()).filter(|n| *n > 0),
                duration_secs: u32::try_from(self.duration()).ok().filter(|n| *n > 0),
                threads: u32::try_from(self.concurrency()).ok().filter(|n| *n > 0),
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
                // Both are validated by `PipelineRequest::normalize()`; the
                // output path is resolved against the configured export
                // directory by the engine, never by the tab.
                output_format: non_blank(self.output_format()),
                output_file: non_blank(self.output_file()),
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
                inject: Some(self.inject_checkbox.checked),
                depth_bypass: Some(self.depth_bypass_checkbox.checked),
                alias_overload: Some(self.alias_overload_checkbox.checked),
                concurrency: Some(self.concurrency()),
                timeout_secs: Some(self.timeout()),
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
                // The tab exposes no flow selector; the canonical default flow
                // applies.
                flow: None,
                client_id: self.client_id().and_then(non_blank),
                redirect_uri: self.redirect_uri().and_then(non_blank),
                redirect_test: Some(self.redirect_test_checkbox.checked),
                scope_test: Some(self.scope_test_checkbox.checked),
                state_test: Some(self.state_test_checkbox.checked),
                grant_test: Some(self.grant_test_checkbox.checked),
                concurrency: Some(self.concurrency()),
                timeout_secs: Some(self.timeout()),
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
        let config = self.get_config();
        let mode = self.get_mode();
        // `query_id` is a scan id for `list_findings` and a CVE id for
        // `search_cve`; the engine validates the pairing.
        let query_id = non_blank(self.query_id());
        let (scan_id, cve_id) = if mode == "search_cve" {
            (None, query_id)
        } else {
            (query_id, None)
        };
        Some(RunRequest {
            task_kind: TaskKind::Storage(eggsec_runtime::request::StorageParams {
                storage_type: "postgres".to_string(),
                path: None,
                host: non_blank(&config.host),
                port: Some(config.port),
                database: non_blank(&config.database),
                username: non_blank(&config.username),
                max_connections: Some(config.max_connections),
                mode: Some(mode.to_string()),
                scan_id,
                cve_id,
                severity_filter: self.severity_filter().and_then(non_blank),
                // Never the password itself — only the name of the variable
                // the engine resolves it from. See `password_env`.
                password_env: non_blank(EGGSEC_STORAGE_PASSWORD_ENV),
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
                credential_file: self.credential_file().and_then(non_blank),
                // The tab's accessors already fall back to their shipped
                // defaults (50/5/30) when a field is blank or unparseable, so
                // the value on screen is the value dispatched. Zero would mean
                // "do nothing", so treat it as unset.
                max_attempts: Some(self.max_attempts()).filter(|n| *n > 0),
                concurrency: Some(self.concurrency()).filter(|n| *n > 0),
                timeout_secs: Some(self.timeout()).filter(|n| *n > 0),
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

    #[test]
    fn non_blank_treats_untouched_text_input_as_absent() {
        assert_eq!(non_blank(""), None);
        assert_eq!(non_blank("   "), None);
        assert_eq!(non_blank("\t\n"), None);
        assert_eq!(non_blank(" 80,443 "), Some("80,443".to_string()));
    }

    #[test]
    fn scan_ports_builder_maps_concurrency_and_ports_to_runtime_dto() {
        use crate::tabs::ScanPortsTab;
        let mut tab = ScanPortsTab::new();
        tab.core.inputs.fields[0].value = "10.0.0.7".to_string();
        tab.core.inputs.fields[1].value = "80,443".to_string();
        tab.core.inputs.fields[2].value = "250".to_string();
        tab.core.inputs.fields[3].value = "4".to_string();

        let req = tab.build_run_request().expect("run request present");
        assert_eq!(req.surface, eggsec_runtime::RuntimeSurface::TuiManual);
        match req.task_kind {
            TaskKind::PortScan(p) => {
                assert_eq!(p.target, "10.0.0.7");
                assert_eq!(p.ports.as_deref(), Some("80,443"));
                assert_eq!(p.timeout_ms, Some(4000));
                assert_eq!(p.concurrency, Some(250));
                // The tab exposes no scan-type control.
                assert_eq!(p.scan_type, None);
            }
            other => panic!("expected PortScan, got {other:?}"),
        }
    }

    #[test]
    fn scan_ports_builder_omits_blank_ports() {
        use crate::tabs::ScanPortsTab;
        let mut tab = ScanPortsTab::new();
        tab.core.inputs.fields[0].value = "10.0.0.7".to_string();
        tab.core.inputs.fields[1].value = "   ".to_string();

        let req = tab.build_run_request().expect("run request present");
        match req.task_kind {
            TaskKind::PortScan(p) => assert!(p.ports.is_none(), "blank ports must not be set"),
            other => panic!("expected PortScan, got {other:?}"),
        }
    }

    #[test]
    fn scan_endpoints_builder_maps_concurrency_and_timeout_to_runtime_dto() {
        use crate::tabs::ScanEndpointsTab;
        let mut tab = ScanEndpointsTab::new();
        tab.core.inputs.fields[0].value = "https://target.lab".to_string();
        tab.core.inputs.fields[1].value = "37".to_string();
        tab.core.inputs.fields[2].value = "9".to_string();
        tab.core.inputs.fields[3].value = "  endpoints/big.txt  ".to_string();

        let req = tab.build_run_request().expect("run request present");
        match req.task_kind {
            TaskKind::EndpointScan(p) => {
                assert_eq!(p.concurrency, Some(37));
                assert_eq!(p.timeout_secs, Some(9));
                assert_eq!(p.wordlist.as_deref(), Some("endpoints/big.txt"));
                // The tab exposes no method list control.
                assert_eq!(p.methods, None);
            }
            other => panic!("expected EndpointScan, got {other:?}"),
        }
    }

    #[test]
    fn fingerprint_builder_maps_ports_and_timeout_to_runtime_dto() {
        use crate::tabs::FingerprintTab;
        let mut tab = FingerprintTab::new();
        tab.core.inputs.fields[0].value = "10.0.0.9".to_string();
        tab.core.inputs.fields[1].value = "22,8080".to_string();
        tab.core.inputs.fields[2].value = "11".to_string();

        let req = tab.build_run_request().expect("run request present");
        match req.task_kind {
            TaskKind::Fingerprint(p) => {
                assert_eq!(p.ports.as_deref(), Some("22,8080"));
                assert_eq!(p.timeout_secs, Some(11));
                // The tab exposes no concurrency control.
                assert_eq!(p.concurrency, None);
            }
            other => panic!("expected Fingerprint, got {other:?}"),
        }
    }

    #[test]
    fn fuzz_builder_maps_ui_controls_to_runtime_dto() {
        use crate::tabs::FuzzTab;
        let mut tab = FuzzTab::new();
        tab.core.inputs.fields[0].value = "https://target.lab".to_string();
        tab.core.inputs.fields[1].value = "POST".to_string();
        tab.core.inputs.fields[2].value = "q".to_string();
        tab.core.inputs.fields[4].value = "7".to_string();
        tab.core.inputs.fields[5].value = "42".to_string();
        tab.core.inputs.fields[6].value = "33".to_string();
        tab.payload_selector.select_by_value("xss");
        tab.mode_selector.select_by_value("Burst");
        tab.mutation_checkbox.checked = true;
        tab.graphql_introspection.checked = false;
        tab.oauth_grant_test.checked = false;

        let req = tab.build_run_request().expect("run request present");
        assert_eq!(req.surface, eggsec_runtime::RuntimeSurface::TuiManual);
        match req.task_kind {
            TaskKind::Fuzz(p) => {
                assert_eq!(p.method.as_deref(), Some("POST"));
                assert_eq!(p.param.as_deref(), Some("q"));
                assert_eq!(p.payload_type.as_deref(), Some("xss"));
                assert_eq!(p.mode.as_deref(), Some("Burst"));
                assert_eq!(p.mutations, Some(true));
                assert_eq!(p.mutation_count, Some(7));
                assert_eq!(p.threads, Some(42));
                assert_eq!(p.timeout, Some(33));
                assert_eq!(p.graphql_introspection, Some(false));
                assert_eq!(p.graphql_depth_bypass, Some(true));
                assert_eq!(p.graphql_alias_overload, Some(true));
                assert_eq!(p.oauth_redirect_test, Some(true));
                assert_eq!(p.oauth_scope_test, Some(true));
                assert_eq!(p.oauth_state_test, Some(true));
                assert_eq!(p.oauth_grant_test, Some(false));
            }
            other => panic!("expected Fuzz, got {other:?}"),
        }
    }

    #[test]
    fn fuzz_builder_omits_blank_text_inputs() {
        use crate::tabs::FuzzTab;
        let mut tab = FuzzTab::new();
        tab.core.inputs.fields[0].value = "https://target.lab".to_string();
        tab.core.inputs.fields[1].value = "  ".to_string();
        tab.core.inputs.fields[2].value = "   ".to_string();

        let req = tab.build_run_request().expect("run request present");
        match req.task_kind {
            TaskKind::Fuzz(p) => {
                assert!(p.method.is_none(), "blank method must not be set");
                assert!(p.param.is_none(), "blank param must not be set");
                // Numeric controls keep their tab defaults.
                assert_eq!(p.threads, Some(10));
                assert_eq!(p.timeout, Some(10));
                assert_eq!(p.mutations, Some(false));
                assert_eq!(p.mutation_count, Some(3));
            }
            other => panic!("expected Fuzz, got {other:?}"),
        }
    }

    #[test]
    fn waf_builder_maps_bypass_mode_and_enabled_techniques_to_runtime_dto() {
        use crate::tabs::WafTab;
        let mut tab = WafTab::new();
        tab.core.inputs.fields[0].value = "https://target.lab".to_string();
        tab.mode_radio.select(1);
        for cb in &mut tab.technique_checkboxes {
            cb.checked = false;
        }
        if let Some(cb) = tab.technique_checkboxes.get_mut(5) {
            cb.checked = true;
        }

        let req = tab.build_run_request().expect("run request present");
        match req.task_kind {
            TaskKind::Waf(p) => {
                assert_eq!(p.bypass_mode, Some(true));
                // Engine tokens, not the operator-facing checkbox labels:
                // `run_waf` only matches header/evasion/smuggling/all, so
                // sending labels left every bypass flag false.
                assert_eq!(
                    p.techniques,
                    Some(vec!["smuggling".to_string()]),
                    "only the checked technique must reach the request, as an engine token"
                );
            }
            other => panic!("expected Waf, got {other:?}"),
        }
    }

    #[test]
    fn waf_builder_reports_detect_only_mode() {
        use crate::tabs::WafTab;
        let mut tab = WafTab::new();
        tab.core.inputs.fields[0].value = "https://target.lab".to_string();

        let req = tab.build_run_request().expect("run request present");
        match req.task_kind {
            TaskKind::Waf(p) => {
                assert_eq!(p.bypass_mode, Some(false));
                // The two techniques checked by default; both are evasion.
                assert_eq!(p.techniques, Some(vec!["evasion".to_string()]));
            }
            other => panic!("expected Waf, got {other:?}"),
        }
    }

    #[test]
    fn waf_stress_builder_sends_concurrency_field_as_concurrency() {
        use crate::tabs::WafStressTab;
        let mut tab = WafStressTab::new();
        tab.core.inputs.fields[0].value = "https://target.lab".to_string();
        tab.core.inputs.fields[1].value = "64".to_string();

        let req = tab.build_run_request().expect("run request present");
        match req.task_kind {
            TaskKind::WafStress(p) => {
                assert_eq!(p.concurrency, Some(64));
                // The tab has no request-count control.
                assert_eq!(p.requests, None);
            }
            other => panic!("expected WafStress, got {other:?}"),
        }
    }

    #[test]
    fn load_builder_maps_requests_connections_duration_and_method_to_runtime_dto() {
        use crate::tabs::LoadTab;
        let mut tab = LoadTab::new();
        tab.core.inputs.fields[0].value = "https://target.lab".to_string();
        tab.core.inputs.fields[1].value = "POST".to_string();
        tab.core.inputs.fields[2].value = "5000".to_string();
        tab.core.inputs.fields[3].value = "32".to_string();
        tab.core.inputs.fields[4].value = "45".to_string();

        let req = tab.build_run_request().expect("run request present");
        assert_eq!(req.surface, eggsec_runtime::RuntimeSurface::TuiManual);
        match req.task_kind {
            TaskKind::LoadTest(p) => {
                assert_eq!(p.method, "POST");
                assert_eq!(p.requests, Some(5000));
                assert_eq!(p.connections, Some(32));
                assert_eq!(p.duration_secs, Some(45));
                // The tab exposes no rate-limit control.
                assert_eq!(p.rate_limit, None);
                // Body/header inputs are empty on a fresh tab.
                assert_eq!(p.body, None);
                assert_eq!(p.headers, Some(vec![]));
            }
            other => panic!("expected LoadTest, got {other:?}"),
        }
    }

    #[test]
    fn load_builder_carries_body_and_headers() {
        use crate::tabs::LoadTab;
        let mut tab = LoadTab::new();
        tab.core.inputs.fields[0].value = "https://target.lab".to_string();
        tab.core.inputs.fields[1].value = "POST".to_string();
        tab.core.inputs.fields[5].value = "{\"probe\":1}".to_string();
        tab.core.inputs.fields[6].value = "X-Probe: eggsec, Accept: application/json".to_string();

        let req = tab.build_run_request().expect("run request present");
        match req.task_kind {
            TaskKind::LoadTest(p) => {
                assert_eq!(p.body.as_deref(), Some("{\"probe\":1}"));
                assert_eq!(
                    p.headers,
                    Some(vec![
                        "X-Probe: eggsec".to_string(),
                        "Accept: application/json".to_string(),
                    ])
                );
                // The request must survive canonical normalization intact.
                let normalized = eggsec::operation_request::LoadTestRequest {
                    target: p.target.clone(),
                    method: Some(p.method.clone()),
                    requests: p.requests,
                    connections: p.connections,
                    duration_secs: p.duration_secs,
                    rate_limit: p.rate_limit,
                    body: p.body.clone(),
                    headers: p.headers.clone().unwrap_or_default(),
                }
                .normalize()
                .expect("load-test request normalizes");
                assert_eq!(normalized.method, "POST");
                assert_eq!(normalized.body.as_deref(), Some("{\"probe\":1}"));
                assert_eq!(
                    normalized.headers,
                    vec![
                        "X-Probe:eggsec".to_string(),
                        "Accept:application/json".to_string()
                    ]
                );
            }
            other => panic!("expected LoadTest, got {other:?}"),
        }
    }

    #[test]
    fn load_builder_request_count_field_drives_canonical_request_total() {
        use crate::tabs::LoadTab;
        use eggsec::dispatch::CanonicalOperationRequest;

        let mut tab = LoadTab::new();
        tab.core.inputs.fields[0].value = "https://target.lab".to_string();
        tab.core.inputs.fields[2].value = "5000".to_string();
        tab.core.inputs.fields[3].value = "32".to_string();

        let req = tab.build_run_request().expect("run request present");
        let CanonicalOperationRequest::LoadTest(load) =
            CanonicalOperationRequest::from_task_kind(&req.task_kind)
        else {
            panic!("expected canonical LoadTest");
        };
        // The engine's canonical rule: explicit `requests` wins, `connections`
        // is concurrency only. Before wiring, the tab's "Total Requests" was
        // discarded and `connections` doubled as the total.
        let normalized = load.normalize().expect("load test normalizes");
        assert_eq!(normalized.requests, 5000);
        assert_eq!(normalized.concurrency, 32);
    }

    #[test]
    fn load_builder_falls_back_to_get_when_method_is_blank() {
        use crate::tabs::LoadTab;
        let mut tab = LoadTab::new();
        tab.core.inputs.fields[0].value = "https://target.lab".to_string();
        tab.core.inputs.fields[1].value = "   ".to_string();

        let req = tab.build_run_request().expect("run request present");
        match req.task_kind {
            TaskKind::LoadTest(p) => assert_eq!(p.method, "GET"),
            other => panic!("expected LoadTest, got {other:?}"),
        }
    }

    // The stress types only exist when the stress-testing feature builds the
    // multi-entry Test Type selector.
    #[cfg(feature = "stress-testing")]
    #[test]
    fn load_builder_stress_branch_maps_duration_and_concurrency_fields() {
        use crate::tabs::LoadTab;
        let mut tab = LoadTab::new();
        tab.core.inputs.fields[0].value = "10.0.0.5".to_string();
        tab.core.inputs.fields[3].value = "24".to_string();
        tab.core.inputs.fields[4].value = "90".to_string();
        tab.test_type_selector.select(1);

        let req = tab.build_run_request().expect("run request present");
        match req.task_kind {
            TaskKind::StressTest(p) => {
                assert_eq!(p.flood_type, "syn");
                assert_eq!(p.duration_secs, Some(90));
                assert_eq!(p.threads, Some(24));
                // The tab exposes no rate control.
                assert_eq!(p.rate_pps, None);
            }
            other => panic!("expected StressTest, got {other:?}"),
        }
    }

    #[test]
    fn graphql_builder_maps_checkboxes_and_fields_to_runtime_dto() {
        use crate::tabs::GraphQlTab;
        let mut tab = GraphQlTab::new();
        tab.core.inputs.fields[0].value = "https://target.lab/graphql".to_string();
        tab.core.inputs.fields[1].value = "24".to_string();
        tab.core.inputs.fields[2].value = "8".to_string();
        tab.inject_checkbox.checked = false;

        let req = tab.build_run_request().expect("run request present");
        assert_eq!(req.surface, eggsec_runtime::RuntimeSurface::TuiManual);
        match req.task_kind {
            TaskKind::GraphQl(p) => {
                assert_eq!(p.introspection, Some(true));
                assert_eq!(p.inject, Some(false));
                assert_eq!(p.depth_bypass, Some(true));
                assert_eq!(p.alias_overload, Some(true));
                assert_eq!(p.concurrency, Some(24));
                assert_eq!(p.timeout_secs, Some(8));
            }
            other => panic!("expected GraphQl, got {other:?}"),
        }
    }

    #[test]
    fn oauth_builder_maps_fields_and_checkboxes_to_runtime_dto() {
        use crate::tabs::OAuthTab;
        let mut tab = OAuthTab::new();
        tab.core.inputs.fields[0].value = "https://target.lab/oauth".to_string();
        tab.core.inputs.fields[1].value = "  client-abc  ".to_string();
        tab.core.inputs.fields[2].value = "https://app.lab/cb".to_string();
        tab.core.inputs.fields[3].value = "17".to_string();
        tab.core.inputs.fields[4].value = "6".to_string();
        tab.state_test_checkbox.checked = false;

        let req = tab.build_run_request().expect("run request present");
        assert_eq!(req.surface, eggsec_runtime::RuntimeSurface::TuiManual);
        match req.task_kind {
            TaskKind::OAuth(p) => {
                assert_eq!(p.client_id.as_deref(), Some("client-abc"));
                assert_eq!(p.redirect_uri.as_deref(), Some("https://app.lab/cb"));
                assert_eq!(p.redirect_test, Some(true));
                assert_eq!(p.scope_test, Some(true));
                assert_eq!(p.state_test, Some(false));
                assert_eq!(p.grant_test, Some(true));
                assert_eq!(p.concurrency, Some(17));
                assert_eq!(p.timeout_secs, Some(6));
                // The tab exposes no flow selector.
                assert_eq!(p.flow, None);
            }
            other => panic!("expected OAuth, got {other:?}"),
        }
    }

    #[test]
    fn oauth_builder_omits_blank_optional_text_inputs() {
        use crate::tabs::OAuthTab;
        let mut tab = OAuthTab::new();
        tab.core.inputs.fields[0].value = "https://target.lab/oauth".to_string();
        tab.core.inputs.fields[1].value = "   ".to_string();
        tab.core.inputs.fields[2].value = "".to_string();

        let req = tab.build_run_request().expect("run request present");
        match req.task_kind {
            TaskKind::OAuth(p) => {
                assert!(p.client_id.is_none(), "blank client_id must not be set");
                assert!(
                    p.redirect_uri.is_none(),
                    "blank redirect_uri must not be set"
                );
            }
            other => panic!("expected OAuth, got {other:?}"),
        }
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

    /// Regression: `Tab::Stress` sets `AppState::Running` on Enter but had no
    /// `TaskBuilder`, so confirming the flood type could never dispatch and
    /// the tab hung on a spinner.
    #[test]
    fn stress_builder_dispatches_the_confirmed_flood_type() {
        use crate::tabs::StressTab;
        let mut tab = StressTab::new();
        if let Some(field) = tab.core.inputs.fields.get_mut(0) {
            field.value = "10.0.0.1".to_string();
        }
        tab.type_selector.select(1); // "syn"

        let req = tab.build_run_request().expect("stress run request");
        match req.task_kind {
            TaskKind::StressTest(p) => {
                assert_eq!(p.target, "10.0.0.1");
                assert_eq!(p.flood_type, "syn");
                assert_eq!(p.rate_pps, Some(100));
                assert_eq!(p.duration_secs, Some(30));
                assert_eq!(p.threads, Some(10));
            }
            other => panic!("expected StressTest, got {other:?}"),
        }
    }

    #[test]
    fn stress_builder_requires_a_target() {
        use crate::tabs::StressTab;
        let mut tab = StressTab::new();
        if let Some(field) = tab.core.inputs.fields.get_mut(0) {
            field.value = String::new();
        }
        assert!(tab.build_run_request().is_none());
    }

    /// Regression: the "Check for 404s" checkbox had an accessor with no
    /// caller, so unticking it changed nothing. Both the checked and unchecked
    /// states must reach the request.
    #[test]
    fn endpoint_builder_carries_include_404() {
        use crate::tabs::ScanEndpointsTab;

        let mut tab = ScanEndpointsTab::new();
        if let Some(field) = tab.core.inputs.fields.get_mut(0) {
            field.value = "https://example.com".to_string();
        }
        // The tab ships with the box ticked, so the default is to include.
        assert!(tab.include_404());
        let req = tab.build_run_request().expect("endpoint run request");
        match req.task_kind {
            TaskKind::EndpointScan(p) => {
                assert_eq!(p.include_404, Some(true));
            }
            other => panic!("expected EndpointScan, got {other:?}"),
        }

        tab.include_404_checkbox.checked = false;
        let req = tab.build_run_request().expect("endpoint run request");
        match req.task_kind {
            TaskKind::EndpointScan(p) => {
                assert_eq!(p.include_404, Some(false));
            }
            other => panic!("expected EndpointScan, got {other:?}"),
        }
    }

    /// The scan tab's output-file field and output-format selector must reach
    /// the request.
    ///
    /// Regression guard: both controls have been on the tab for the whole life
    /// of the tab, and neither was ever sent — `PipelineParams` had no fields
    /// for them, so every TUI pipeline run discarded the operator's chosen
    /// destination.
    #[test]
    fn scan_builder_carries_output_format_and_file() {
        use crate::tabs::ScanTab;

        let mut tab = ScanTab::new();
        if let Some(field) = tab.inputs.fields.get_mut(0) {
            field.value = "https://example.com".to_string();
        }
        if let Some(field) = tab.inputs.fields.get_mut(1) {
            field.value = "  reports/scan.sarif  ".to_string();
        }
        // The selector starts on its documented default.
        assert_eq!(tab.output_format(), "json");

        let req = tab.build_run_request().expect("pipeline run request");
        match req.task_kind {
            TaskKind::Pipeline(p) => {
                assert_eq!(p.output_format.as_deref(), Some("json"));
                // Whitespace is stripped by `non_blank` so a padded value does
                // not become a path with trailing spaces.
                assert_eq!(p.output_file.as_deref(), Some("reports/scan.sarif"));
            }
            other => panic!("expected Pipeline, got {other:?}"),
        }

        // The pair must survive canonical normalization.
        let canonical = eggsec::operation_request::PipelineRequest {
            target: "https://example.com".into(),
            profile: Some("quick".into()),
            output_format: Some("json".into()),
            output_file: Some("reports/scan.sarif".into()),
        }
        .normalize()
        .expect("pipeline request normalizes");
        assert_eq!(
            canonical.output_format,
            eggsec::operation_request::PipelineOutputFormat::Json
        );
        assert_eq!(canonical.output_file.as_deref(), Some("reports/scan.sarif"));
    }

    /// The storage tab's connection fields must reach the request, and its
    /// password field must not.
    ///
    /// Regression guard: the tab hardcoded `storage_type: "sqlite"` with no
    /// host, mode or query, so every storage run dialled the engine default
    /// host. And because `RunRequest` is persisted verbatim into the daemon's
    /// SQLite snapshot store, a password field on the wire would sit at rest
    /// in plaintext — only the variable name may cross.
    #[cfg(feature = "database")]
    #[test]
    fn storage_builder_carries_config_but_never_the_password() {
        use crate::tabs::StorageTab;

        let mut tab = StorageTab::new();
        if let Some(f) = tab.config_inputs.fields.first_mut() {
            f.value = "db.internal".to_string();
        }
        if let Some(f) = tab.config_inputs.fields.get_mut(1) {
            f.value = "6543".to_string();
        }
        if let Some(f) = tab.config_inputs.fields.get_mut(2) {
            f.value = "findings_db".to_string();
        }
        if let Some(f) = tab.config_inputs.fields.get_mut(3) {
            f.value = "analyst".to_string();
        }
        if let Some(f) = tab.config_inputs.fields.get_mut(4) {
            f.value = "hunter2-plaintext".to_string();
        }
        if let Some(f) = tab.query_inputs.fields.first_mut() {
            f.value = "scan-7".to_string();
        }

        let req = tab.build_run_request().expect("storage run request");
        // The serialized wire form is the real assertion: this is exactly what
        // the daemon persists to disk.
        let wire = serde_json::to_string(&req).expect("serialize request");
        assert!(
            !wire.contains("hunter2-plaintext"),
            "password leaked onto the wire: {wire}"
        );
        assert!(wire.contains("password_env"), "no password_env in {wire}");

        match req.task_kind {
            TaskKind::Storage(p) => {
                assert_eq!(p.host.as_deref(), Some("db.internal"));
                assert_eq!(p.port, Some(6543));
                assert_eq!(p.database.as_deref(), Some("findings_db"));
                assert_eq!(p.username.as_deref(), Some("analyst"));
                assert_eq!(p.mode.as_deref(), Some(tab.get_mode()));
                assert_eq!(
                    p.password_env.as_deref(),
                    Some(crate::app::task_management::EGGSEC_STORAGE_PASSWORD_ENV)
                );
            }
            other => panic!("expected Storage, got {other:?}"),
        }
    }

    /// A traversal destination typed into the scan tab is rejected by
    /// normalization rather than written.
    #[test]
    fn scan_builder_rejects_traversal_output_path() {
        let canonical = eggsec::operation_request::PipelineRequest {
            target: "https://example.com".into(),
            profile: Some("quick".into()),
            output_format: Some("json".into()),
            output_file: Some("../../../etc/cron.d/pwn".into()),
        };
        assert!(canonical.normalize().is_err());
    }

    /// The UDP checkbox must reach the request in both states.
    ///
    /// The tab has shipped this checkbox for its whole life with no wire
    /// field behind it, so ticking it changed nothing. Both states are
    /// asserted because an unticked box has to be distinguishable from a
    /// field that does not exist.
    #[test]
    fn port_scan_builder_carries_udp_checkbox() {
        use crate::tabs::ScanPortsTab;

        let mut tab = ScanPortsTab::new();
        if let Some(f) = tab.core.inputs.fields.get_mut(0) {
            f.value = "127.0.0.1".to_string();
        }
        assert!(!tab.udp(), "the box ships unticked");

        let req = tab.build_run_request().expect("port scan run request");
        match req.task_kind {
            TaskKind::PortScan(p) => assert_eq!(p.udp, Some(false)),
            other => panic!("expected PortScan, got {other:?}"),
        }

        tab.udp_checkbox.checked = true;
        let req = tab.build_run_request().expect("port scan run request");
        match req.task_kind {
            TaskKind::PortScan(p) => assert_eq!(p.udp, Some(true)),
            other => panic!("expected PortScan, got {other:?}"),
        }
    }
}
