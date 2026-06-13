use crate::workers;

pub trait TaskBuilder {
    fn build_task_config(&self) -> Option<workers::TaskConfig>;
}

impl TaskBuilder for super::tabs::ReconTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }

        Some(workers::TaskConfig::Recon {
            target: target.to_string(),
            concurrency: self.concurrency(),
            options: self.get_options(),
        })
    }
}

impl TaskBuilder for super::tabs::LoadTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }

        if self.is_stress_test() {
            Some(workers::TaskConfig::StressTest {
                target: target.to_string(),
                stress_type: self.stress_type().to_string(),
                rate: self.requests(),
                duration: 60,
                concurrency: self.concurrency(),
            })
        } else {
            Some(workers::TaskConfig::LoadTest {
                target: target.to_string(),
                requests: self.requests(),
                concurrency: self.concurrency(),
                timeout: std::time::Duration::from_secs(self.timeout()),
            })
        }
    }
}

impl TaskBuilder for super::tabs::ScanPortsTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }

        Some(workers::TaskConfig::PortScan {
            target: target.to_string(),
            ports: self.ports().to_string(),
            concurrency: self.concurrency(),
            timeout: std::time::Duration::from_secs(self.timeout()),
        })
    }
}

impl TaskBuilder for super::tabs::ScanEndpointsTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }

        Some(workers::TaskConfig::EndpointScan {
            target: target.to_string(),
            concurrency: self.concurrency(),
            timeout: std::time::Duration::from_secs(self.timeout()),
            wordlist: self.wordlist().map(|s| s.to_string()),
        })
    }
}

impl TaskBuilder for super::tabs::FingerprintTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }

        Some(workers::TaskConfig::Fingerprint {
            target: target.to_string(),
            ports: self.ports().to_string(),
            timeout: std::time::Duration::from_secs(self.timeout()),
        })
    }
}

impl TaskBuilder for super::tabs::FuzzTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }

        Some(workers::TaskConfig::Fuzz {
            target: target.to_string(),
            payload_type: self.payload_type_string(),
            mode: self.mode().to_string(),
            mutations: self.mutations_enabled(),
            mutation_count: self.mutation_count(),
            method: self.method().to_string(),
            param: self.param().map(|s| s.to_string()),
            concurrency: self.concurrency(),
            timeout: self.timeout(),
            graphql_introspection: self.graphql_introspection_enabled(),
            graphql_depth_bypass: self.graphql_depth_bypass_enabled(),
            graphql_alias_overload: self.graphql_alias_overload_enabled(),
            oauth_redirect_test: self.oauth_redirect_enabled(),
            oauth_scope_test: self.oauth_scope_enabled(),
            oauth_state_test: self.oauth_state_enabled(),
            oauth_grant_test: self.oauth_grant_enabled(),
        })
    }
}

impl TaskBuilder for super::tabs::WafTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }

        Some(workers::TaskConfig::Waf {
            target: target.to_string(),
            bypass_mode: self.is_bypass_mode(),
            techniques: self.enabled_techniques(),
        })
    }
}

impl TaskBuilder for super::tabs::WafStressTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }

        Some(workers::TaskConfig::WafStress {
            target: target.to_string(),
            concurrency: self.concurrency(),
            timeout: self.timeout(),
        })
    }
}

impl TaskBuilder for super::tabs::ScanTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }
        let profile = self.profile()?;

        Some(workers::TaskConfig::Pipeline {
            target: target.to_string(),
            profile,
            output_file: String::new(),
            output_format: "json".to_string(),
        })
    }
}

impl TaskBuilder for super::tabs::PacketTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        match self.current_view {
            super::tabs::packet::PacketView::Capture => {
                let interface = self.target();
                if interface.is_empty() {
                    return None;
                }

                Some(workers::TaskConfig::PacketCapture {
                    interface: interface.to_string(),
                    filter: self.filter().to_string(),
                    max_packets: self.max_packets(),
                    output_file: self.output_file().map(|s| s.to_string()),
                })
            }
            super::tabs::packet::PacketView::Traceroute => {
                let target = self.target();
                if target.is_empty() {
                    return None;
                }

                Some(workers::TaskConfig::PacketTraceroute {
                    target: target.to_string(),
                    max_hops: 30,
                })
            }
            super::tabs::packet::PacketView::Send => {
                let target = self.target();
                if target.is_empty() {
                    return None;
                }

                let port: u16 = match self.filter().parse() {
                    Ok(p) => p,
                    Err(_) => {
                        tracing::warn!("Invalid port specified: {:?}", self.filter());
                        return None;
                    }
                };
                let count = (self.max_packets() as u64).min(u32::MAX as u64) as u32;

                Some(workers::TaskConfig::PacketSend {
                    target: target.to_string(),
                    port,
                    count,
                    packet_size: 64,
                })
            }
            _ => None,
        }
    }
}

impl TaskBuilder for super::tabs::GraphQlTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }

        Some(workers::TaskConfig::GraphQl {
            url: target.to_string(),
            introspection: self.introspection_checkbox.checked,
            inject: self.inject_checkbox.checked,
            depth_bypass: self.depth_bypass_checkbox.checked,
            alias_overload: self.alias_overload_checkbox.checked,
            concurrency: self.concurrency(),
            timeout: self.timeout(),
        })
    }
}

impl TaskBuilder for super::tabs::OAuthTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }

        Some(workers::TaskConfig::OAuth {
            url: target.to_string(),
            client_id: self.client_id().map(|s| s.to_string()),
            redirect_uri: self.redirect_uri().map(|s| s.to_string()),
            redirect_test: self.redirect_test_checkbox.checked,
            scope_test: self.scope_test_checkbox.checked,
            state_test: self.state_test_checkbox.checked,
            grant_test: self.grant_test_checkbox.checked,
            concurrency: self.concurrency(),
            timeout: self.timeout(),
        })
    }
}

impl TaskBuilder for super::tabs::ClusterTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        None
    }
}

#[cfg(feature = "advanced-hunting")]
impl TaskBuilder for super::tabs::HuntTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }
        Some(workers::TaskConfig::Hunt {
            target: target.to_string(),
            config: self.get_config(),
        })
    }
}

#[cfg(feature = "headless-browser")]
impl TaskBuilder for super::tabs::BrowserTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }
        Some(workers::TaskConfig::Browser {
            target: target.to_string(),
            config: self.get_config(),
        })
    }
}

#[cfg(feature = "compliance")]
impl TaskBuilder for super::tabs::ComplianceTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        let target = self.target();
        if target.is_empty() {
            return None;
        }
        Some(workers::TaskConfig::Compliance {
            target: target.to_string(),
            framework: self.selected_framework(),
        })
    }
}

#[cfg(feature = "database")]
impl TaskBuilder for super::tabs::StorageTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        let config = self.get_config();
        let mode = self.get_mode();
        Some(workers::TaskConfig::Storage {
            config,
            mode: mode.to_string(),
            scan_id: Some(self.query_id().to_string()).filter(|s| !s.is_empty()),
            cve_id: None,
            severity_filter: self.severity_filter().map(String::from),
        })
    }
}

#[cfg(feature = "external-integrations")]
impl TaskBuilder for super::tabs::IntegrationsTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        let mode = self.get_mode();
        let (title, description) = self.get_issue_params();
        Some(workers::TaskConfig::Integrations {
            config: self.get_config(),
            mode: mode.to_string(),
            title: Some(title).filter(|s| !s.is_empty()),
            description: Some(description).filter(|s| !s.is_empty()),
            labels: self.issue_labels(),
            assignees: self.issue_assignees(),
            search_query: Some(self.search_query().to_string()).filter(|s| !s.is_empty()),
        })
    }
}

#[cfg(feature = "finding-workflow")]
impl TaskBuilder for super::tabs::WorkflowTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        Some(workers::TaskConfig::Workflow {
            mode: self.get_mode().to_string(),
            target: None,
            finding_ids: vec![],
        })
    }
}

#[cfg(feature = "vuln-management")]
impl TaskBuilder for super::tabs::VulnTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        Some(workers::TaskConfig::Vuln {
            mode: self.get_mode().to_string(),
            target: None,
            cve_id: Some(self.cve_id().to_string()).filter(|s| !s.is_empty()),
            title: Some(self.title().to_string()).filter(|s| !s.is_empty()),
            description: Some(self.description().to_string()).filter(|s| !s.is_empty()),
            cvss_vector: Some(self.cvss_vector().to_string()).filter(|s| !s.is_empty()),
            asset_type: Some(self.asset_type().to_string()).filter(|s| !s.is_empty()),
            severity: Some(self.severity_str().to_string()).filter(|s| !s.is_empty()),
        })
    }
}

#[cfg(feature = "wireless")]
impl TaskBuilder for super::tabs::WirelessTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        #[cfg(feature = "wireless-advanced")]
        {
            if self.active_mode {
                let (interface, attack_type, bssid, client, frame_count, rate_limit, dry_run) =
                    self.active_attack_config()?;
                return Some(workers::TaskConfig::WirelessActive {
                    interface,
                    attack_type,
                    bssid,
                    client,
                    frame_count,
                    rate_limit,
                    dry_run,
                });
            }
        }
        let interface = self.interface();
        if interface.is_empty() {
            None
        } else {
            Some(workers::TaskConfig::Wireless {
                interface: interface.to_string(),
            })
        }
    }
}

impl TaskBuilder for super::tabs::AuthTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        let target = self.target()?;
        if target.is_empty() {
            return None;
        }

        Some(workers::TaskConfig::Auth {
            target: target.to_string(),
            username: self.username().map(|s| s.to_string()),
            password_list: self.password_list().map(|s| s.to_string()),
            credential_file: None,
            max_attempts: 100,
            concurrency: 1,
            timeout: 30,
        })
    }
}

#[cfg(feature = "web-proxy")]
impl TaskBuilder for super::tabs::InterceptTab {
    fn build_task_config(&self) -> Option<workers::TaskConfig> {
        Some(workers::TaskConfig::Intercept {
            listen_addr: self.listen_addr(),
            dry_run: self.is_dry_run(),
            max_flows: self.max_flows(),
            target: self.primary_target(),
        })
    }
}
