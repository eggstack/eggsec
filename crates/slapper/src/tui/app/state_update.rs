use crate::tui::tabs::TabState;
use crate::tui::workers::TaskResult;

impl super::App {
    pub(super) fn update(&mut self) {
        let mut dirty = false;

        if let Some(ref mut rx) = self.progress_rx {
            use tokio::sync::mpsc;
            match rx.try_recv() {
                Ok((completed, total)) => {
                    self.update_progress(completed, total);
                    dirty = true;
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.progress_rx = None;
                }
            }
        }

        if let Some(ref mut rx) = self.result_rx {
            use tokio::sync::mpsc;
            match rx.try_recv() {
                Ok(result) => {
                    self.handle_result(result);
                    dirty = true;
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.result_rx = None;
                }
            }
        }

        if dirty {
            self.needs_redraw = true;
        }
    }

    fn update_progress(&mut self, completed: u64, total: u64) {
        use super::tabs::Tab;
        match self.current_tab {
            Tab::Recon => self.recon.update_progress(completed, total),
            Tab::Load => self.load.update_progress(completed, total),
            Tab::ScanPorts => self.scan_ports.update_progress(completed, total),
            Tab::ScanEndpoints => self.scan_endpoints.update_progress(completed, total),
            Tab::Fingerprint => self.fingerprint.update_progress(completed, total),
            Tab::Fuzz => self.fuzz.update_progress(completed, total),
            Tab::Waf => self.waf.update_progress(completed, total),
            Tab::WafStress => self.waf_stress.update_progress(completed, total),
            Tab::Scan => self.scan.update_progress(
                self.scan
                    .stages
                    .iter()
                    .filter(|s| matches!(s.status, super::tabs::StageStatus::Completed))
                    .count() as u64,
                self.scan.stages.len() as u64,
            ),
            _ => {}
        }
    }

    pub(super) fn handle_result(&mut self, result: TaskResult) {
        match result {
            TaskResult::LoadTest(r) => {
                if let Ok(mut h) = self.history.lock() {
                    h.add_load_test_result(
                        &r.target_url,
                        r.total_requests,
                        r.successful_requests,
                        r.failed_requests,
                        r.requests_per_second,
                        r.latency_mean_ms,
                    );
                }
                self.load.set_results(r);
            }
            #[cfg(feature = "stress-testing")]
            TaskResult::StressTest { target, stats } => {
                let pps = if stats.duration_ms > 0 {
                    (stats.packets_sent * 1000) / stats.duration_ms
                } else {
                    0
                };
                if let Ok(mut h) = self.history.lock() {
                    h.add_load_test_result(
                        "stress-test",
                        stats.packets_sent,
                        stats.packets_sent.saturating_sub(stats.errors),
                        stats.errors,
                        pps as f64,
                        0.0,
                    );
                }
                self.load.set_stress_results(target.clone(), stats);
            }
            TaskResult::PortScan(r) => {
                if let Ok(mut h) = self.history.lock() {
                    h.add_port_scan_result(
                        &r.host,
                        r.ports_scanned as usize,
                        r.open_ports.iter().map(|p| p.port).collect(),
                    );
                }
                self.scan_ports.set_results(r);
            }
            TaskResult::EndpointScan(r) => {
                if let Ok(mut h) = self.history.lock() {
                    h.add_endpoint_scan_result(
                        &r.base_url,
                        r.endpoints_found,
                        r.interesting_findings,
                    );
                }
                self.scan_endpoints.set_results(r);
            }
            TaskResult::Fingerprint(r) => {
                if let Ok(mut h) = self.history.lock() {
                    h.add_fingerprint_result(
                        &r.host,
                        r.services_identified,
                        r.results
                            .iter()
                            .map(|fp| format!("{}: {}", fp.port, fp.service))
                            .collect(),
                    );
                }
                self.fingerprint.set_results(r);
            }
            TaskResult::WafDetection(r) => {
                let waf_name = r.waf_name.clone().unwrap_or_default();
                if let Ok(mut h) = self.history.lock() {
                    h.add_waf_result("<target>", r.waf_name.is_some(), &waf_name, 0);
                }
                self.waf.set_detection_result(r);
            }
            TaskResult::WafBypass {
                detection,
                bypasses,
            } => {
                let success_count = bypasses.iter().filter(|b| b.success).count();
                let waf_name = detection.waf_name.clone().unwrap_or_default();
                if let Ok(mut h) = self.history.lock() {
                    h.add_waf_result(
                        "<target>",
                        detection.waf_name.is_some(),
                        &waf_name,
                        success_count,
                    );
                }
                self.waf.set_detection_result(detection);
                self.waf.set_bypass_results(bypasses);
            }
            TaskResult::Pipeline(r) => {
                let completed = r.stage_results.iter().filter(|s| s.success).count();
                if let Ok(mut h) = self.history.lock() {
                    h.add_pipeline_result(
                        &r.target,
                        completed,
                        r.stage_results.len(),
                        r.total_duration_ms,
                    );
                }
                self.scan.set_report(r);
            }
            TaskResult::Fuzz(session) => {
                self.fuzz.set_results(session);
            }
            TaskResult::Recon(r) => {
                if let Ok(mut h) = self.history.lock() {
                    h.add_recon_result(
                        &r.target,
                        r.domain.clone().unwrap_or_default(),
                        r.ip_address.clone().unwrap_or_default(),
                    );
                }
                self.recon.set_results(r);
            }
            TaskResult::PacketCapture {
                packets_captured,
                output_file,
            } => {
                self.packet
                    .set_capture_results(packets_captured, output_file);
            }
            TaskResult::PacketTraceroute { hops } => {
                self.packet.set_traceroute_results(hops);
            }
            TaskResult::PacketSend {
                packets_sent,
                bytes_sent,
            } => {
                self.packet.set_send_results(packets_sent, bytes_sent);
            }
            TaskResult::GraphQl(r) => {
                self.graphql.set_results(r);
            }
            TaskResult::OAuth(r) => {
                self.oauth.set_results(r);
            }
            #[cfg(feature = "nse")]
            TaskResult::Nse(r) => {
                self.nse.set_results(r);
            }
            #[cfg(feature = "advanced-hunting")]
            TaskResult::Hunt(r) => {
                self.hunt.set_report(r);
            }
            #[cfg(not(feature = "advanced-hunting"))]
            TaskResult::Hunt(_) => {}
            #[cfg(feature = "headless-browser")]
            TaskResult::Browser(r) => {
                self.browser.set_report(r);
            }
            #[cfg(feature = "compliance")]
            TaskResult::Compliance(r) => {
                self.compliance.set_report(r);
            }
            #[cfg(not(feature = "compliance"))]
            TaskResult::Compliance(_) => {}
            TaskResult::Storage => {}
            #[cfg(feature = "database")]
            TaskResult::StorageListScans { scans } => {
                self.storage.scans = scans.clone();
                self.storage.state = AppState::Completed;
            }
            #[cfg(feature = "database")]
            TaskResult::StorageListFindings { findings } => {
                self.storage.findings = findings.clone();
                self.storage.state = AppState::Completed;
            }
            #[cfg(not(feature = "database"))]
            TaskResult::StorageListScans { .. } => {}
            #[cfg(not(feature = "database"))]
            TaskResult::StorageListFindings { .. } => {}
            TaskResult::Integrations => {}
            #[cfg(feature = "external-integrations")]
            TaskResult::IntegrationsCreateIssue { ref issue } => {
                self.integrations.state = AppState::Completed;
                self.integrations.results_view.clear();
                self.integrations
                    .results_view
                    .add_line(ratatui::text::Line::from(format!(
                        "Created issue: {} ({})",
                        issue.title,
                        issue.id.as_deref().unwrap_or("no-id")
                    )));
            }
            #[cfg(feature = "external-integrations")]
            TaskResult::IntegrationsSearchIssues { issues } => {
                self.integrations.state = AppState::Completed;
                self.integrations.results_view.clear();
                self.integrations
                    .results_view
                    .add_line(ratatui::text::Line::from(format!(
                        "Found {} issues",
                        issues.len()
                    )));
            }
            #[cfg(not(feature = "external-integrations"))]
            TaskResult::IntegrationsCreateIssue { .. } => {}
            #[cfg(not(feature = "external-integrations"))]
            TaskResult::IntegrationsSearchIssues { .. } => {}
            #[cfg(feature = "finding-workflow")]
            TaskResult::Workflow(ref report) => {
                self.workflow.report = Some(report.clone());
                self.workflow.state = AppState::Completed;
            }
            #[cfg(not(feature = "finding-workflow"))]
            TaskResult::Workflow(_) => {}
            #[cfg(feature = "vuln-management")]
            TaskResult::Vuln(ref assessment) => {
                self.vuln.state = AppState::Completed;
                self.vuln.results_view.clear();
                for line in &assessment.results {
                    self.vuln
                        .results_view
                        .add_line(ratatui::text::Line::from(line.clone()));
                }
            }
            #[cfg(not(feature = "vuln-management"))]
            TaskResult::Vuln(_) => {}
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            TaskResult::PluginsLoaded(plugins) => {
                self.plugin.plugin_list = plugins;
            }
            TaskResult::Error(msg) => {
                self.set_error_for_current_tab(msg);
            }
        }
    }

    fn set_error_for_current_tab(&mut self, msg: String) {
        use super::tabs::Tab;
        match self.current_tab {
            Tab::Recon => self.recon.set_error(msg),
            Tab::Load => self.load.set_error(msg),
            Tab::ScanPorts => self.scan_ports.set_error(msg),
            Tab::ScanEndpoints => self.scan_endpoints.set_error(msg),
            Tab::Fingerprint => self.fingerprint.set_error(msg),
            Tab::Fuzz => self.fuzz.set_error(msg),
            Tab::Waf => self.waf.set_error(msg),
            Tab::WafStress => self.waf_stress.set_error(msg),
            Tab::Scan => self.scan.set_error(msg),
            Tab::Resume => self.resume.set_error(msg),
            Tab::Proxy => self.proxy.set_error(msg),
            Tab::Packet => self.packet.set_error(msg),
            Tab::GraphQl => self.graphql.set_error(msg),
            Tab::OAuth => self.oauth.set_error(msg),
            Tab::Cluster => self.cluster.set_error(msg),
            Tab::Stress => self.stress.set_error(msg),
            Tab::Report => self.report.set_error(msg),
            #[cfg(feature = "nse")]
            Tab::Nse => self.nse.set_error(msg),
            #[cfg(not(feature = "nse"))]
            Tab::Nse => {
                tracing::error!("NSE tab is not available: {}", msg);
            }
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => self.plugin.set_error(msg),
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => {
                tracing::error!("Plugin tab is not available: {}", msg);
            }
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => self.hunt.set_error(msg),
            #[cfg(not(feature = "advanced-hunting"))]
            Tab::Hunt => {
                tracing::error!("Hunt feature not available: {}", msg);
            }
            #[cfg(feature = "headless-browser")]
            Tab::Browser => self.browser.set_error(msg),
            #[cfg(not(feature = "headless-browser"))]
            Tab::Browser => {}
            #[cfg(feature = "compliance")]
            Tab::Compliance => self.compliance.set_error(msg),
            #[cfg(not(feature = "compliance"))]
            Tab::Compliance => {
                tracing::error!("Compliance feature not available: {}", msg);
            }
            #[cfg(feature = "database")]
            Tab::Storage => self.storage.set_error(msg),
            #[cfg(not(feature = "database"))]
            Tab::Storage => {
                tracing::error!("Storage feature not available: {}", msg);
            }
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => self.integrations.set_error(msg),
            #[cfg(not(feature = "external-integrations"))]
            Tab::Integrations => {
                tracing::error!("Integrations feature not available: {}", msg);
            }
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => self.workflow.set_error(msg),
            #[cfg(not(feature = "finding-workflow"))]
            Tab::Workflow => {
                tracing::error!("Workflow feature not available: {}", msg);
            }
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => self.vuln.set_error(msg),
            #[cfg(not(feature = "vuln-management"))]
            Tab::Vuln => {
                tracing::error!("Vuln management not available: {}", msg);
            }
            Tab::Settings => {
                tracing::error!("Settings tab does not support error state: {}", msg);
            }
            Tab::History => {
                tracing::error!("History tab does not support error state: {}", msg);
            }
            Tab::Dashboard => {
                tracing::error!("Dashboard tab does not support error state: {}", msg);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{create_shared_history, App};

    fn create_test_app() -> App {
        App::new(create_shared_history())
    }

    #[test]
    fn test_update_does_not_panic_when_no_channels() {
        let mut app = create_test_app();
        app.progress_rx = None;
        app.result_rx = None;
        app.update();
    }

    #[test]
    fn test_update_with_disconnected_progress_rx_clears_channel() {
        let mut app = create_test_app();
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        drop(tx);
        app.progress_rx = Some(rx);
        app.result_rx = None;
        app.update();
        assert!(app.progress_rx.is_none());
    }

    #[test]
    fn test_update_with_disconnected_result_rx_clears_channel() {
        let mut app = create_test_app();
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        drop(tx);
        app.progress_rx = None;
        app.result_rx = Some(rx);
        app.update();
        assert!(app.result_rx.is_none());
    }
}
