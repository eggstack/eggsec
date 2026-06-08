use crate::tui::app::tab_error::TabError;
#[cfg(any(feature = "database", feature = "external-integrations"))]
use crate::tui::tabs::AppState;
use crate::tui::workers::TaskResult;

impl super::App {
    fn with_history<F>(&self, f: F)
    where
        F: FnOnce(&mut crate::tui::tabs::HistoryTab),
    {
        let mut history = self.history.lock();
        f(&mut history);
    }

    pub(super) fn update(&mut self) {
        let mut dirty = false;

        if let Some(ref mut rx) = self.progress_rx {
            let mut pending_updates = Vec::new();
            while let Ok((completed, total)) = rx.try_recv() {
                pending_updates.push((completed, total));
            }
            if rx.is_closed() {
                self.progress_rx = None;
            }
            for (completed, total) in pending_updates {
                self.update_progress(completed, total);
                dirty = true;
            }
        }

        if let Some(ref mut rx) = self.result_rx {
            let mut pending_results = Vec::new();
            while let Ok(result) = rx.try_recv() {
                pending_results.push(result);
            }
            let is_closed = rx.is_closed();
            for result in pending_results {
                self.handle_result(result);
                dirty = true;
            }
            if is_closed {
                self.result_rx = None;
                self.task_tab = None;
                self.task_handle = None;
            }
        }

        if dirty {
            self.needs_redraw = true;
        }
    }

    fn update_progress(&mut self, completed: u64, total: u64) {
        // Use task_tab if set, otherwise fall back to current_tab (for backwards compatibility)
        let tab = self.task_tab.unwrap_or(self.current_tab);
        tab.update_progress_in_app(self, completed, total);
    }

    pub(super) fn handle_result(&mut self, result: TaskResult) {
        let result = match self.handle_security_result(result) {
            Some(r) => r,
            None => return,
        };
        let result = match self.handle_protocol_result(result) {
            Some(r) => r,
            None => return,
        };
        let result = match self.handle_feature_result(result) {
            Some(r) => r,
            None => return,
        };
        tracing::debug!("Unhandled TaskResult variant: {:?}", result);
    }

    fn handle_security_result(&mut self, result: TaskResult) -> Option<TaskResult> {
        match result {
            TaskResult::LoadTest(r) => {
                self.with_history(|h| {
                    h.add_load_test_result(
                        &r.target_url,
                        r.total_requests,
                        r.successful_requests,
                        r.failed_requests,
                        r.requests_per_second,
                        r.latency_mean_ms,
                    );
                });
                self.load.set_results(r);
                None
            }
            #[cfg(feature = "stress-testing")]
            TaskResult::StressTest { target, stats } => {
                let pps = if stats.duration_ms > 0 {
                    (stats.packets_sent * 1000) / stats.duration_ms
                } else {
                    0
                };
                self.with_history(|h| {
                    h.add_load_test_result(
                        "stress-test",
                        stats.packets_sent,
                        stats.packets_sent.saturating_sub(stats.errors),
                        stats.errors,
                        pps as f64,
                        0.0,
                    );
                });
                self.load.set_stress_results(target.clone(), stats);
                None
            }
            TaskResult::PortScan(r) => {
                self.with_history(|h| {
                    h.add_port_scan_result(
                        &r.host,
                        r.ports_scanned as usize,
                        r.open_ports.iter().map(|p| p.port).collect(),
                    );
                });
                self.scan_ports.set_results(r);
                None
            }
            TaskResult::EndpointScan(r) => {
                self.with_history(|h| {
                    h.add_endpoint_scan_result(
                        &r.base_url,
                        r.endpoints_found,
                        r.interesting_findings,
                    );
                });
                self.scan_endpoints.set_results(r);
                None
            }
            TaskResult::Fingerprint(r) => {
                self.with_history(|h| {
                    h.add_fingerprint_result(
                        &r.host,
                        r.services_identified,
                        r.results
                            .iter()
                            .map(|fp| format!("{}: {}", fp.port, fp.service))
                            .collect(),
                    );
                });
                self.fingerprint.set_results(r);
                None
            }
            TaskResult::WafDetection(r) => {
                let waf_name = r.waf_name.clone().unwrap_or_default();
                self.with_history(|h| {
                    h.add_waf_result("<target>", r.waf_name.is_some(), &waf_name, 0);
                });
                self.waf.set_results(r);
                None
            }
            TaskResult::WafBypass {
                detection,
                bypasses,
            } => {
                let success_count = bypasses.iter().filter(|b| b.success).count();
                let waf_name = detection.waf_name.clone().unwrap_or_default();
                self.with_history(|h| {
                    h.add_waf_result(
                        "<target>",
                        detection.waf_name.is_some(),
                        &waf_name,
                        success_count,
                    );
                });
                self.waf.set_results(detection);
                self.waf.set_bypass_results(bypasses);
                None
            }
            TaskResult::WafStress(bypasses) => {
                let success_count = bypasses.iter().filter(|b| b.success).count();
                self.with_history(|h| {
                    h.add_waf_result("<target>", true, "WAF Stress", success_count);
                });
                self.waf.set_bypass_results(bypasses);
                None
            }
            TaskResult::Pipeline(r) => {
                let completed = r.stage_results.iter().filter(|s| s.success).count();
                self.with_history(|h| {
                    h.add_pipeline_result(
                        &r.target,
                        completed,
                        r.stage_results.len(),
                        r.total_duration_ms,
                    );
                });
                self.scan.set_report(r);
                None
            }
            TaskResult::Fuzz(session) => {
                self.fuzz.set_results(session);
                None
            }
            TaskResult::Recon(r) => {
                self.with_history(|h| {
                    h.add_recon_result(
                        &r.target,
                        r.domain.as_deref().unwrap_or("").to_string(),
                        r.ip_address.as_deref().unwrap_or("").to_string(),
                    );
                });
                self.recon.set_results(r);
                None
            }
            _ => Some(result),
        }
    }

    fn handle_protocol_result(&mut self, result: TaskResult) -> Option<TaskResult> {
        match result {
            TaskResult::PacketCapture {
                packets_captured,
                output_file,
            } => {
                self.packet
                    .set_capture_results(packets_captured, output_file);
                None
            }
            TaskResult::PacketTraceroute { hops } => {
                self.packet.set_traceroute_results(hops);
                None
            }
            TaskResult::PacketSend {
                packets_sent,
                bytes_sent,
            } => {
                self.packet.set_send_results(packets_sent, bytes_sent);
                None
            }
            TaskResult::GraphQl(r) => {
                self.graphql.set_results(r);
                None
            }
            TaskResult::OAuth(r) => {
                self.oauth.set_results(r);
                None
            }
            #[cfg(feature = "nse")]
            TaskResult::Nse(r) => {
                self.nse.set_results(r);
                None
            }
            #[cfg(feature = "advanced-hunting")]
            TaskResult::Hunt(r) => {
                self.hunt.set_report(r);
                None
            }
            #[cfg(not(feature = "advanced-hunting"))]
            TaskResult::Hunt(_) => {
                tracing::warn!("TaskResult::Hunt received but feature \"advanced-hunting\" is disabled");
                None
            }
            #[cfg(feature = "headless-browser")]
            TaskResult::Browser(r) => {
                self.browser.set_report(r);
                None
            }
            #[cfg(feature = "compliance")]
            TaskResult::Compliance(r) => {
                self.compliance.set_report(r);
                None
            }
            #[cfg(not(feature = "compliance"))]
            TaskResult::Compliance(_) => {
                tracing::warn!("TaskResult::Compliance received but feature \"compliance\" is disabled");
                None
            }
            _ => Some(result),
        }
    }

    fn handle_feature_result(&mut self, result: TaskResult) -> Option<TaskResult> {
        match result {
            TaskResult::Storage => {
                #[cfg(feature = "database")]
                {
                    self.storage.state = AppState::Completed;
                    self.storage
                        .results_view
                        .add_line(ratatui::text::Line::from("Storage task completed"));
                }
                None
            }
            #[cfg(feature = "database")]
            TaskResult::StorageListScans { scans } => {
                self.storage.set_scans(scans.clone());
                self.storage.state = AppState::Completed;
                None
            }
            #[cfg(feature = "database")]
            TaskResult::StorageListFindings { findings } => {
                self.storage.set_findings(findings.clone());
                self.storage.state = AppState::Completed;
                None
            }
            #[cfg(not(feature = "database"))]
            TaskResult::StorageListScans { .. } => {
                tracing::warn!("TaskResult::StorageListScans received but feature \"database\" is disabled");
                None
            }
            #[cfg(not(feature = "database"))]
            TaskResult::StorageListFindings { .. } => {
                tracing::warn!("TaskResult::StorageListFindings received but feature \"database\" is disabled");
                None
            }
            TaskResult::Integrations => {
                #[cfg(feature = "external-integrations")]
                {
                    self.integrations.state = AppState::Completed;
                    self.integrations
                        .results_view
                        .add_line(ratatui::text::Line::from("Integrations task completed"));
                }
                None
            }
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
                None
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
                None
            }
            #[cfg(not(feature = "external-integrations"))]
            TaskResult::IntegrationsCreateIssue { .. } => {
                tracing::warn!("TaskResult::IntegrationsCreateIssue received but feature \"external-integrations\" is disabled");
                None
            }
            #[cfg(not(feature = "external-integrations"))]
            TaskResult::IntegrationsSearchIssues { .. } => {
                tracing::warn!("TaskResult::IntegrationsSearchIssues received but feature \"external-integrations\" is disabled");
                None
            }
            #[cfg(feature = "finding-workflow")]
            TaskResult::Workflow(ref report) => {
                self.workflow.report = Some(report.clone());
                self.workflow.state = AppState::Completed;
                None
            }
            #[cfg(not(feature = "finding-workflow"))]
            TaskResult::Workflow(_) => {
                tracing::warn!("TaskResult::Workflow received but feature \"finding-workflow\" is disabled");
                None
            }
            #[cfg(feature = "vuln-management")]
            TaskResult::Vuln(ref assessment) => {
                self.vuln.state = AppState::Completed;
                self.vuln.results_view.clear();
                
                // Display summary lines (backward compat)
                for line in &assessment.summary {
                    self.vuln
                        .results_view
                        .add_line(ratatui::text::Line::from(line.clone()));
                }
                
                // If no summary but structured data exists, display it
                if assessment.summary.is_empty() {
                    if let Some(ref cvss) = assessment.cvss_score {
                        self.vuln.display_cvss(&cvss.vector);
                    }
                    if let Some(ref info) = assessment.exploit_info {
                        self.vuln.display_exploit_info(&info.cve_id, info.clone());
                    }
                    if let Some(ref asset) = assessment.asset_criticality {
                        self.vuln.display_asset(asset.clone());
                    }
                    if !assessment.prioritized_findings.is_empty() {
                        self.vuln.display_prioritized(assessment.prioritized_findings.clone());
                    }
                    if let Some(ref result) = assessment.triage_results.first() {
                        self.vuln.display_triage(result.clone());
                    }
                    if let Some(ref rem) = assessment.remediation_plans.first() {
                        self.vuln.display_remediation(rem.clone());
                    }
                }
                None
            }
            #[cfg(not(feature = "vuln-management"))]
            TaskResult::Vuln(_) => {
                tracing::warn!("TaskResult::Vuln received but feature \"vuln-management\" is disabled");
                None
            }
            #[cfg(feature = "wireless")]
            TaskResult::Wireless(r) => {
                self.wireless.set_results(r);
                None
            }
            _ => Some(result),
        }
    }

    pub(super) fn set_error_for_current_tab(&mut self, error: TabError) {
        let mut tab = self.task_tab.unwrap_or(self.current_tab);
        if Self::is_error_unsupported_tab(tab) {
            self.log_unsupported_tab_error(tab, &error);
            return;
        }

        tab.as_tab_state_mut(self).set_error(error);
    }

    fn is_error_unsupported_tab(tab: super::tabs::Tab) -> bool {
        matches!(
            tab,
            super::tabs::Tab::Settings | super::tabs::Tab::History | super::tabs::Tab::Dashboard
        )
    }

    fn log_unsupported_tab_error(&self, tab: super::tabs::Tab, error: &TabError) {
        let tab_name = match tab {
            super::tabs::Tab::Settings => "Settings",
            super::tabs::Tab::History => "History",
            super::tabs::Tab::Dashboard => "Dashboard",
            _ => "Unknown",
        };
        tracing::error!("{} tab does not support error state: {}", tab_name, error);
    }
}

trait TabProgressUpdate {
    fn update_progress_in_app(&self, app: &mut super::App, completed: u64, total: u64);
}

impl TabProgressUpdate for super::tabs::Tab {
    fn update_progress_in_app(&self, app: &mut super::App, completed: u64, total: u64) {
        match self {
            super::tabs::Tab::Recon => app.recon.update_progress(completed, total),
            super::tabs::Tab::Load => app.load.update_progress(completed, total),
            super::tabs::Tab::ScanPorts => app.scan_ports.update_progress(completed, total),
            super::tabs::Tab::ScanEndpoints => app.scan_endpoints.update_progress(completed, total),
            super::tabs::Tab::Fingerprint => app.fingerprint.update_progress(completed, total),
            super::tabs::Tab::Fuzz => app.fuzz.update_progress(completed, total),
            super::tabs::Tab::Waf => app.waf.update_progress(completed, total),
            super::tabs::Tab::WafStress => app.waf_stress.update_progress(completed, total),
            super::tabs::Tab::Scan => {
                let total = app.scan.stages.len() as u64;
                if total == 0 {
                    return;
                }
                let completed = app
                    .scan
                    .stages
                    .iter()
                    .filter(|s| matches!(s.status, super::tabs::StageStatus::Completed))
                    .count() as u64;
                app.scan.update_progress(completed, total);
            }
            #[cfg(feature = "wireless")]
            super::tabs::Tab::Wireless => {
                app.wireless.update_progress(completed, total);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{create_shared_history, App};

    fn create_test_app() -> App {
        App::new_for_testing(create_shared_history())
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
