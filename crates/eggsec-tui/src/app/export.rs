use super::{Notification, NotificationSeverity};
use eggsec::types::OutputFormat;

impl super::App {
    fn export_tab_json<T, F>(&mut self, get: F, filename: &str, tab_name: &str)
    where
        T: serde::Serialize,
        F: for<'a> FnOnce(&'a Self) -> Option<&'a T>,
    {
        let results = get(self);
        match results {
            Some(data) => match serde_json::to_string_pretty(data) {
                Ok(json) => self.save_export(&format!("{}.json", filename), json),
                Err(e) => {
                    let msg = format!("Failed to serialize {} results: {}", tab_name, e);
                    tracing::error!("{}", msg);
                    self.overlay.notification =
                        Some(Notification::new(msg, NotificationSeverity::Error));
                }
            },
            None => {
                let msg = format!("No exportable data for {} tab.", tab_name);
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
        }
    }

    pub(super) fn export_results(&mut self) {
        let ext = self.get_export_extension();
        let base_name = match self.current_tab {
            super::tabs::Tab::Recon => "recon_results",
            super::tabs::Tab::Load => "load_results",
            super::tabs::Tab::ScanPorts => "port_scan_results",
            super::tabs::Tab::ScanEndpoints => "endpoint_scan_results",
            super::tabs::Tab::Fingerprint => "fingerprint_results",
            super::tabs::Tab::Fuzz => "fuzz_results",
            super::tabs::Tab::Waf => "waf_results",
            super::tabs::Tab::WafStress => "waf_stress_results",
            super::tabs::Tab::Scan => "pipeline_scan_report",
            super::tabs::Tab::Resume => "resume_results",
            super::tabs::Tab::Proxy => "proxy_results",
            super::tabs::Tab::Packet => "packet_results",
            super::tabs::Tab::GraphQl => "graphql_results",
            super::tabs::Tab::OAuth => "oauth_results",
            super::tabs::Tab::Auth => "auth_results",
            super::tabs::Tab::C2 => "c2_results",
            super::tabs::Tab::DbPentest => "db_pentest_results",
            super::tabs::Tab::Cluster => "cluster_status",
            super::tabs::Tab::Stress => "stress_results",
            super::tabs::Tab::Report => "report_results",
            super::tabs::Tab::Nse => "nse_results",
            super::tabs::Tab::Settings => "settings",
            super::tabs::Tab::History => "history",
            super::tabs::Tab::Dashboard => "dashboard",
            super::tabs::Tab::Hunt => "hunt_results",
            super::tabs::Tab::Browser => "browser_results",
            super::tabs::Tab::Compliance => "compliance_results",
            super::tabs::Tab::Storage => "storage_results",
            super::tabs::Tab::Integrations => "integration_results",
            super::tabs::Tab::Workflow => "workflow_results",
            super::tabs::Tab::Vuln => "vuln_results",
            super::tabs::Tab::Wireless => "wireless_results",
            super::tabs::Tab::Intercept => "intercept_results",
        };

        let filename = format!("{}.{}", base_name, ext);

        match self.export_format {
            OutputFormat::Json => self.export_json(),
            OutputFormat::Csv => self.export_csv(&filename),
            OutputFormat::Html
            | OutputFormat::Markdown
            | OutputFormat::Sarif
            | OutputFormat::Junit => {
                self.export_json();
                self.export_converted(&filename);
            }
            _ => self.export_json(),
        }
    }

    pub(super) fn export_json(&mut self) {
        match self.current_tab {
            super::tabs::Tab::Recon => {
                self.export_tab_json(|s| s.tabs.recon.get_results(), "recon_results", "Recon")
            }
            super::tabs::Tab::Load => {
                self.export_tab_json(|s| s.tabs.load.get_results(), "load_results", "Load")
            }
            super::tabs::Tab::ScanPorts => self.export_tab_json(
                |s| s.tabs.scan_ports.get_results(),
                "port_scan_results",
                "Scan Ports",
            ),
            super::tabs::Tab::ScanEndpoints => self.export_tab_json(
                |s| s.tabs.scan_endpoints.get_results(),
                "endpoint_scan_results",
                "Scan Endpoints",
            ),
            super::tabs::Tab::Fingerprint => self.export_tab_json(
                |s| s.tabs.fingerprint.get_results(),
                "fingerprint_results",
                "Fingerprint",
            ),
            super::tabs::Tab::Fuzz => {
                self.export_tab_json(|s| s.tabs.fuzz.get_results(), "fuzz_results", "Fuzz")
            }
            super::tabs::Tab::Waf => self.export_waf_json(),
            super::tabs::Tab::WafStress => {
                if let Some(results) = self.tabs.waf_stress.get_results() {
                    self.save_export("waf_stress_results.json", results);
                } else {
                    let msg = "No exportable data for WAF Stress tab.".to_string();
                    self.overlay.notification =
                        Some(Notification::new(msg, NotificationSeverity::Warning));
                }
            }
            super::tabs::Tab::Scan => {
                self.export_tab_json(|s| s.tabs.scan.get_report(), "pipeline_scan_report", "Scan")
            }
            super::tabs::Tab::Resume => {
                let msg = "Resume tab: no exportable data (use original scan results)".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
            super::tabs::Tab::GraphQl => {
                let msg = "GraphQL tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
            super::tabs::Tab::OAuth => {
                let msg = "OAuth tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
            super::tabs::Tab::Auth => {
                let msg = "Auth tab: no exportable data available (defense-lab only)".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
            super::tabs::Tab::C2 => {
                let msg = "C2 tab: use CLI export (eggsec c2 --json -o report.json)".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
            super::tabs::Tab::Cluster => {
                let msg = "Cluster tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
            super::tabs::Tab::Stress => {
                let msg = "Stress tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
            super::tabs::Tab::Report => {
                let msg = "Report tab: use conversion endpoints (HTML/Markdown/SARIF) instead"
                    .to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
            super::tabs::Tab::Nse => {
                let msg = "NSE tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
            super::tabs::Tab::Settings => {
                let msg = "Settings tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
            super::tabs::Tab::History => {
                let history_data = {
                    let h = self.history.lock();
                    h.export()
                };
                self.save_export("history.json", history_data);
            }
            super::tabs::Tab::Dashboard => {
                let msg = "Dashboard tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
            super::tabs::Tab::Proxy => {
                let msg = "Proxy tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
            super::tabs::Tab::Packet => {
                let msg = "Packet tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
            #[cfg(feature = "advanced-hunting")]
            super::tabs::Tab::Hunt => {
                self.export_tab_json(|s| s.tabs.hunt.get_results(), "hunt_results", "Hunt")
            }
            #[cfg(not(feature = "advanced-hunting"))]
            super::tabs::Tab::Hunt => {
                let msg = "Hunt tab requires advanced-hunting feature".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
            super::tabs::Tab::Browser => {
                let msg = "Browser tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
            super::tabs::Tab::Compliance => {
                let msg = "Compliance tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
            super::tabs::Tab::Storage => {
                let msg = "Storage tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
            super::tabs::Tab::Integrations => {
                let msg = "Integrations tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
            super::tabs::Tab::Workflow => {
                let msg = "Workflow tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
            super::tabs::Tab::Vuln => {
                let msg = "Vuln tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
            super::tabs::Tab::Wireless => {
                let msg = "Wireless tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
            super::tabs::Tab::Intercept => {
                let msg = "Intercept tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
            }
            #[cfg(feature = "db-pentest")]
            super::tabs::Tab::DbPentest => {
                if let Some(ref r) = self.tabs.db_pentest.results {
                    match serde_json::to_string_pretty(r) {
                        Ok(json) => self.save_export("db_pentest_results.json", json),
                        Err(e) => {
                            tracing::error!("Failed to serialize Db Pentest results: {}", e);
                            self.overlay.notification = Some(Notification::new(
                                format!("Export failed: {}", e),
                                NotificationSeverity::Error,
                            ));
                            return;
                        }
                    }
                } else {
                    let msg = "Db Pentest tab: no exportable data available".to_string();
                    self.overlay.notification =
                        Some(Notification::new(msg, NotificationSeverity::Warning));
                }
            }
            #[cfg(not(feature = "db-pentest"))]
            super::tabs::Tab::DbPentest => {}
        }
    }

    fn export_waf_json(&mut self) {
        let detection_json = match self.tabs.waf.get_detection_result() {
            Some(r) => match serde_json::to_string_pretty(&r) {
                Ok(j) => Some(j),
                Err(e) => {
                    tracing::error!("Failed to serialize WAF detection results: {}", e);
                    self.overlay.notification = Some(Notification::new(
                        format!("Export failed: {}", e),
                        NotificationSeverity::Error,
                    ));
                    return;
                }
            },
            None => None,
        };
        let bypass_json = match self.tabs.waf.get_bypass_results() {
            Some(r) => match serde_json::to_string_pretty(&r) {
                Ok(j) => Some(j),
                Err(e) => {
                    tracing::error!("Failed to serialize WAF bypass results: {}", e);
                    self.overlay.notification = Some(Notification::new(
                        format!("Export failed: {}", e),
                        NotificationSeverity::Error,
                    ));
                    return;
                }
            },
            None => None,
        };

        if detection_json.is_none() && bypass_json.is_none() {
            let msg = "No exportable data for WAF tab.".to_string();
            self.overlay.notification = Some(Notification::new(msg, NotificationSeverity::Warning));
            return;
        }

        if let Some(json) = detection_json {
            self.save_export("waf_detection_results.json", json);
        }
        if let Some(json) = bypass_json {
            self.save_export("waf_bypass_results.json", json);
        }
    }

    fn export_csv(&mut self, filename: &str) {
        use eggsec::output::csv::{CsvExporter, EndpointCsv, PortCsv};

        match self.current_tab {
            super::tabs::Tab::ScanPorts => {
                if let Some(results) = self.tabs.scan_ports.get_results() {
                    let ports: Vec<PortCsv> = results
                        .open_ports
                        .iter()
                        .map(|p| PortCsv {
                            host: results.host.clone(),
                            port: p.port,
                            protocol: "tcp".to_string(),
                            service: Some(p.service.clone()),
                            version: None,
                            state: "open".to_string(),
                        })
                        .collect();
                    match CsvExporter::export_ports(&ports) {
                        Ok(csv) => self.save_export(filename, csv),
                        Err(e) => {
                            let msg = format!("Failed to export ports to CSV: {}", e);
                            tracing::error!("{}", msg);
                            self.overlay.notification =
                                Some(Notification::new(msg, NotificationSeverity::Error));
                        }
                    }
                }
            }
            super::tabs::Tab::ScanEndpoints => {
                if let Some(results) = self.tabs.scan_endpoints.get_results() {
                    let endpoints: Vec<EndpointCsv> = results
                        .results
                        .iter()
                        .map(|e| EndpointCsv {
                            url: format!("{}/{}", results.base_url, e.path),
                            method: "GET".to_string(),
                            status: e.status_code,
                            content_type: None,
                            content_length: e.content_length.unwrap_or(0),
                        })
                        .collect();
                    match CsvExporter::export_endpoints(&endpoints) {
                        Ok(csv) => self.save_export(filename, csv),
                        Err(e) => {
                            let msg = format!("Failed to export endpoints to CSV: {}", e);
                            tracing::error!("{}", msg);
                            self.overlay.notification =
                                Some(Notification::new(msg, NotificationSeverity::Error));
                        }
                    }
                }
            }
            _ => {
                self.export_json();
            }
        }
    }

    fn export_converted(&mut self, filename: &str) {
        use eggsec::output::convert::load_scan_report;

        let base_name = filename
            .trim_end_matches(".html")
            .trim_end_matches(".md")
            .trim_end_matches(".sarif")
            .trim_end_matches(".junit")
            .trim_end_matches(".json");

        let json_filename = format!("{}.json", base_name);
        let export_dir = self
            .tabs
            .settings
            .config
            .as_ref()
            .and_then(|c| c.paths.export_dir.as_deref())
            .unwrap_or(eggsec_core::constants::DEFAULT_EXPORT_DIR);

        let base_dir = std::path::Path::new(eggsec_core::constants::DEFAULT_EXPORT_DIR);
        if let Err(e) = eggsec::utils::validation::validate_path_string(base_dir, export_dir) {
            tracing::error!("Invalid export directory: {}", e);
            return;
        }

        let json_path = format!("{}/{}", export_dir, json_filename);

        match load_scan_report(&json_path) {
            Ok(report) => {
                let converted = match self.export_format {
                    OutputFormat::Html => eggsec::output::convert::convert_to_html(&report),
                    OutputFormat::Markdown => eggsec::output::convert::convert_to_markdown(&report)
                        .unwrap_or_else(|e| format!("Error: {}", e)),
                    OutputFormat::Sarif => eggsec::output::convert::convert_to_sarif(&report)
                        .unwrap_or_else(|e| format!("Error: {}", e)),
                    OutputFormat::Junit => eggsec::output::convert::convert_to_junit(&report)
                        .unwrap_or_else(|e| format!("Error: {}", e)),
                    _ => {
                        tracing::warn!("Unsupported export format: {:?}", self.export_format);
                        return;
                    }
                };
                self.save_export(filename, converted);
            }
            Err(e) => {
                tracing::warn!(
                    "Could not load JSON report for conversion ({}): {}",
                    json_path,
                    e
                );
            }
        }
    }

    fn save_export(&mut self, filename: &str, data: String) {
        use std::io::Write;

        let export_dir = self
            .tabs
            .settings
            .config
            .as_ref()
            .and_then(|c| c.paths.export_dir.as_deref())
            .unwrap_or(eggsec_core::constants::DEFAULT_EXPORT_DIR);

        let base_dir = std::path::Path::new(eggsec_core::constants::DEFAULT_EXPORT_DIR);
        if let Err(e) = eggsec::utils::validation::validate_path_string(base_dir, export_dir) {
            let msg = format!("Invalid export directory: {}", e);
            tracing::error!("{}", msg);
            self.overlay.notification = Some(Notification::new(msg, NotificationSeverity::Error));
            return;
        }

        let path = format!("{}/{}", export_dir, filename);
        let dir = std::path::Path::new(export_dir);
        if !dir.exists() {
            if let Err(e) = std::fs::create_dir_all(dir) {
                let msg = format!("Could not create export directory: {}", e);
                tracing::error!("{}", msg);
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Error));
                return;
            }
        }

        let mut file = match std::fs::File::create(&path) {
            Ok(file) => file,
            Err(e) => {
                let msg = format!("Could not create export file: {}", e);
                tracing::error!("{}", msg);
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Error));
                return;
            }
        };

        if let Err(e) = file.write_all(data.as_bytes()) {
            let msg = format!("Could not write to export file: {}", e);
            tracing::error!("{}", msg);
            self.overlay.notification = Some(Notification::new(msg, NotificationSeverity::Error));
        } else {
            let msg = format!("Exported to: {}", path);
            tracing::info!("{}", msg);
            self.overlay.notification = Some(Notification::new(msg, NotificationSeverity::Success));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::create_test_app;
    use crate::tabs::Tab;
    use eggsec::types::OutputFormat;

    #[test]
    fn test_get_export_extension_json() {
        let mut app = create_test_app();
        app.export_format = OutputFormat::Json;
        assert_eq!(app.get_export_extension(), "json");
    }

    #[test]
    fn test_get_export_extension_csv() {
        let mut app = create_test_app();
        app.export_format = OutputFormat::Csv;
        assert_eq!(app.get_export_extension(), "csv");
    }

    #[test]
    fn test_get_export_extension_html() {
        let mut app = create_test_app();
        app.export_format = OutputFormat::Html;
        assert_eq!(app.get_export_extension(), "html");
    }

    #[test]
    fn test_get_export_extension_sarif() {
        let mut app = create_test_app();
        app.export_format = OutputFormat::Sarif;
        assert_eq!(app.get_export_extension(), "sarif");
    }

    #[test]
    fn test_get_export_extension_junit() {
        let mut app = create_test_app();
        app.export_format = OutputFormat::Junit;
        assert_eq!(app.get_export_extension(), "xml");
    }

    #[test]
    fn test_get_export_extension_markdown() {
        let mut app = create_test_app();
        app.export_format = OutputFormat::Markdown;
        assert_eq!(app.get_export_extension(), "md");
    }

    #[test]
    fn test_get_export_extension_compact() {
        let mut app = create_test_app();
        app.export_format = OutputFormat::Compact;
        assert_eq!(app.get_export_extension(), "json");
    }

    #[test]
    fn test_get_export_extension_pretty() {
        let mut app = create_test_app();
        app.export_format = OutputFormat::Pretty;
        assert_eq!(app.get_export_extension(), "txt");
    }

    #[test]
    fn test_cycle_export_format_cycles_through_all_formats() {
        let mut app = create_test_app();

        app.export_format = OutputFormat::Pretty;
        app.cycle_export_format();
        assert_eq!(app.export_format, OutputFormat::Json);

        app.cycle_export_format();
        assert_eq!(app.export_format, OutputFormat::Compact);

        app.cycle_export_format();
        assert_eq!(app.export_format, OutputFormat::Csv);

        app.cycle_export_format();
        assert_eq!(app.export_format, OutputFormat::Html);

        app.cycle_export_format();
        assert_eq!(app.export_format, OutputFormat::Markdown);

        app.cycle_export_format();
        assert_eq!(app.export_format, OutputFormat::Sarif);

        app.cycle_export_format();
        assert_eq!(app.export_format, OutputFormat::Junit);

        app.cycle_export_format();
        assert_eq!(app.export_format, OutputFormat::Pretty);
    }

    #[test]
    fn test_export_results_does_not_panic() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        app.export_results();
    }

    #[test]
    fn test_export_results_does_not_panic_for_all_tabs() {
        let mut app = create_test_app();
        let tabs = Tab::all();
        for &tab in tabs {
            app.current_tab = tab;
            app.export_results();
        }
    }
}
