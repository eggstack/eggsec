use crate::tui::workers;

impl super::App {
    pub(crate) fn spawn_task(&mut self, config: Option<workers::TaskConfig>) {
        if let Some(config) = config {
            if self.task_handle.is_some() {
                tracing::warn!("A task is already running. Aborting previous task before starting new one.");
                if let Some(handle) = self.task_handle.take() {
                    handle.abort();
                }
                if let Some(rx) = self.progress_rx.take() {
                    drop(rx);
                }
                if let Some(rx) = self.result_rx.take() {
                    drop(rx);
                }
            }

            let (progress_tx, progress_rx) = tokio::sync::mpsc::channel(100);
            let (result_tx, result_rx) = tokio::sync::mpsc::channel(1);

            let runner = workers::TaskRunner::new(config, progress_tx, result_tx.clone());
            let error_tx = result_tx.clone();

            self.progress_rx = Some(progress_rx);
            self.result_rx = Some(result_rx);

            self.task_handle = Some(tokio::spawn(async move {
                match runner.run().await {
                    Ok(_) => {}
                    Err(e) => {
                        let friendly_error = super::make_friendly_error(&e);
                        tracing::error!("Task failed: {}", friendly_error);
                        let _ = error_tx.send(workers::TaskResult::Error(friendly_error)).await;
                    }
                }
            }));
        }
    }

    pub(crate) fn build_recon_task(&self) -> Option<workers::TaskConfig> {
        let target = self.recon.target();
        if target.is_empty() {
            return None;
        }

        Some(workers::TaskConfig::Recon {
            target: target.to_string(),
            concurrency: self.recon.concurrency(),
            options: self.recon.get_options(),
        })
    }

    pub(crate) fn build_load_task(&self) -> Option<workers::TaskConfig> {
        let target = self.load.target();
        if target.is_empty() {
            return None;
        }

        if self.load.is_stress_test() {
            Some(workers::TaskConfig::StressTest {
                target: target.to_string(),
                stress_type: self.load.stress_type().to_string(),
                rate: self.load.requests(),
                duration: 60,
                concurrency: self.load.concurrency(),
            })
        } else {
            Some(workers::TaskConfig::LoadTest {
                target: target.to_string(),
                requests: self.load.requests(),
                concurrency: self.load.concurrency(),
                timeout: std::time::Duration::from_secs(self.load.timeout()),
            })
        }
    }

    pub(crate) fn build_port_scan_task(&self) -> Option<workers::TaskConfig> {
        let target = self.scan_ports.target();
        if target.is_empty() {
            return None;
        }

        Some(workers::TaskConfig::PortScan {
            target: target.to_string(),
            ports: self.scan_ports.ports().to_string(),
            concurrency: self.scan_ports.concurrency(),
            timeout: std::time::Duration::from_secs(self.scan_ports.timeout()),
        })
    }

    pub(crate) fn build_endpoint_scan_task(&self) -> Option<workers::TaskConfig> {
        let target = self.scan_endpoints.target();
        if target.is_empty() {
            return None;
        }

        Some(workers::TaskConfig::EndpointScan {
            target: target.to_string(),
            concurrency: self.scan_endpoints.concurrency(),
            timeout: std::time::Duration::from_secs(self.scan_endpoints.timeout()),
            wordlist: self.scan_endpoints.wordlist().map(|s| s.to_string()),
        })
    }

    pub(crate) fn build_fingerprint_task(&self) -> Option<workers::TaskConfig> {
        let target = self.fingerprint.target();
        if target.is_empty() {
            return None;
        }

        Some(workers::TaskConfig::Fingerprint {
            target: target.to_string(),
            ports: self.fingerprint.ports().to_string(),
            timeout: std::time::Duration::from_secs(self.fingerprint.timeout()),
        })
    }

    pub(crate) fn build_fuzz_task(&self) -> Option<workers::TaskConfig> {
        let target = self.fuzz.target();
        if target.is_empty() {
            return None;
        }

        Some(workers::TaskConfig::Fuzz {
            target: target.to_string(),
            payload_type: self.fuzz.payload_type_string(),
            mode: self.fuzz.mode().to_string(),
            mutations: self.fuzz.mutations_enabled(),
            mutation_count: self.fuzz.mutation_count(),
            method: self.fuzz.method().to_string(),
            param: self.fuzz.param().map(|s| s.to_string()),
            concurrency: self.fuzz.concurrency(),
            timeout: self.fuzz.timeout(),
            graphql_introspection: self.fuzz.graphql_introspection_enabled(),
            graphql_depth_bypass: self.fuzz.graphql_depth_bypass_enabled(),
            graphql_alias_overload: self.fuzz.graphql_alias_overload_enabled(),
            oauth_redirect_test: self.fuzz.oauth_redirect_enabled(),
            oauth_scope_test: self.fuzz.oauth_scope_enabled(),
            oauth_state_test: self.fuzz.oauth_state_enabled(),
            oauth_grant_test: self.fuzz.oauth_grant_enabled(),
        })
    }

    pub(crate) fn build_waf_task(&self) -> Option<workers::TaskConfig> {
        let target = self.waf.target();
        if target.is_empty() {
            return None;
        }

        Some(workers::TaskConfig::Waf {
            target: target.to_string(),
            bypass_mode: self.waf.is_bypass_mode(),
            techniques: self.waf.enabled_techniques(),
        })
    }

    pub(crate) fn build_waf_stress_task(&self) -> Option<workers::TaskConfig> {
        let target = self.waf_stress.target();
        if target.is_empty() {
            return None;
        }

        Some(workers::TaskConfig::WafStress {
            target: target.to_string(),
            concurrency: self.waf_stress.concurrency(),
            timeout: self.waf_stress.timeout(),
        })
    }

    pub(crate) fn build_pipeline_task(&self) -> Option<workers::TaskConfig> {
        let target = self.scan.target();
        if target.is_empty() {
            return None;
        }
        let profile = self.scan.profile()?;

        Some(workers::TaskConfig::Pipeline {
            target: target.to_string(),
            profile,
            output_file: String::new(),
            output_format: "json".to_string(),
        })
    }

    pub(crate) fn build_packet_capture_task(&self) -> Option<workers::TaskConfig> {
        let interface = self.packet.target();
        if interface.is_empty() {
            return None;
        }

        Some(workers::TaskConfig::PacketCapture {
            interface: interface.to_string(),
            filter: self.packet.filter().to_string(),
            max_packets: self.packet.max_packets(),
            output_file: self.packet.output_file().map(|s| s.to_string()),
        })
    }

    pub(crate) fn build_packet_traceroute_task(&self) -> Option<workers::TaskConfig> {
        let target = self.packet.target();
        if target.is_empty() {
            return None;
        }

        Some(workers::TaskConfig::PacketTraceroute {
            target: target.to_string(),
            max_hops: 30,
        })
    }

    pub(crate) fn build_packet_send_task(&self) -> Option<workers::TaskConfig> {
        let target = self.packet.target();
        if target.is_empty() {
            return None;
        }

        let port: u16 = match self.packet.filter().parse() {
            Ok(p) => p,
            Err(_) => {
                tracing::warn!("Invalid port specified: {:?}", self.packet.filter());
                return None;
            }
        };
        let count = self.packet.max_packets() as u32;

        Some(workers::TaskConfig::PacketSend {
            target: target.to_string(),
            port,
            count,
            packet_size: 64,
        })
    }

    #[cfg(feature = "advanced-hunting")]
    pub(crate) fn build_hunt_task(&self) -> Option<workers::TaskConfig> {
        let target = self.hunt.target();
        if target.is_empty() {
            return None;
        }
        Some(workers::TaskConfig::Hunt {
            target: target.to_string(),
            config: self.hunt.get_config(),
        })
    }
    #[cfg(not(feature = "advanced-hunting"))]
    #[allow(dead_code)]
    pub(crate) fn build_hunt_task(&self) -> Option<workers::TaskConfig> {
        None
    }

    #[cfg(feature = "headless-browser")]
    pub(crate) fn build_browser_task(&self) -> Option<workers::TaskConfig> {
        let target = self.browser.target();
        if target.is_empty() {
            return None;
        }
        Some(workers::TaskConfig::Browser {
            target: target.to_string(),
            config: self.browser.get_config(),
        })
    }

    #[cfg(feature = "compliance")]
    pub(crate) fn build_compliance_task(&self) -> Option<workers::TaskConfig> {
        let target = self.compliance.target();
        if target.is_empty() {
            return None;
        }
        Some(workers::TaskConfig::Compliance {
            target: target.to_string(),
            framework: self.compliance.selected_framework(),
        })
    }
    #[cfg(not(feature = "compliance"))]
    #[allow(dead_code)]
    pub(crate) fn build_compliance_task(&self) -> Option<workers::TaskConfig> {
        None
    }

    #[cfg(feature = "database")]
    pub(crate) fn build_storage_task(&self) -> Option<workers::TaskConfig> {
        Some(workers::TaskConfig::Storage)
    }
    #[cfg(not(feature = "database"))]
    #[allow(dead_code)]
    pub(crate) fn build_storage_task(&self) -> Option<workers::TaskConfig> {
        None
    }

    #[cfg(feature = "external-integrations")]
    pub(crate) fn build_integrations_task(&self) -> Option<workers::TaskConfig> {
        Some(workers::TaskConfig::Integrations)
    }
    #[cfg(not(feature = "external-integrations"))]
    #[allow(dead_code)]
    pub(crate) fn build_integrations_task(&self) -> Option<workers::TaskConfig> {
        None
    }

    #[cfg(feature = "finding-workflow")]
    pub(crate) fn build_workflow_task(&self) -> Option<workers::TaskConfig> {
        Some(workers::TaskConfig::Workflow)
    }
    #[cfg(not(feature = "finding-workflow"))]
    #[allow(dead_code)]
    pub(crate) fn build_workflow_task(&self) -> Option<workers::TaskConfig> {
        None
    }

    #[cfg(feature = "vuln-management")]
    pub(crate) fn build_vuln_task(&self) -> Option<workers::TaskConfig> {
        Some(workers::TaskConfig::Vuln)
    }
    #[cfg(not(feature = "vuln-management"))]
    #[allow(dead_code)]
    pub(crate) fn build_vuln_task(&self) -> Option<workers::TaskConfig> {
        None
    }
}
