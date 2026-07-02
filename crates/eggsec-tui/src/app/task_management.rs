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
                framework: self.selected_framework().map(|f| format!("{:?}", f)),
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
                target: self.target().unwrap_or_default(),
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
        Some(RunRequest {
            task_kind: TaskKind::Intercept(eggsec_runtime::request::InterceptParams {
                listen_port: self
                    .listen_addr()
                    .split(':')
                    .last()
                    .and_then(|p| p.parse().ok()),
                target: self.primary_target(),
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
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
        let db_type = self
            .core
            .inputs
            .fields
            .get(2)
            .map(|f| f.value.clone())
            .unwrap_or_else(|| "all".to_string());
        Some(RunRequest {
            task_kind: TaskKind::DbPentest(eggsec_runtime::request::DbPentestParams {
                db_type,
                target,
                port: None,
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        })
    }
}
