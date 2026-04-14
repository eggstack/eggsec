use crate::types::OutputFormat;

impl super::App {
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
            super::tabs::Tab::Cluster => "cluster_status",
            super::tabs::Tab::Stress => "stress_results",
            super::tabs::Tab::Report => "report_results",
            super::tabs::Tab::Nse => "nse_results",
            super::tabs::Tab::Plugin => "plugin_results",
            super::tabs::Tab::Settings => "settings",
            super::tabs::Tab::History => "history",
            super::tabs::Tab::Dashboard => "dashboard",
            #[cfg(feature = "advanced-hunting")]
            super::tabs::Tab::Hunt => "hunt_results",
            #[cfg(not(feature = "advanced-hunting"))]
            super::tabs::Tab::Hunt => "hunt_results",
            super::tabs::Tab::Browser => "browser_results",
            #[cfg(feature = "compliance")]
            super::tabs::Tab::Compliance => "compliance_results",
            #[cfg(not(feature = "compliance"))]
            super::tabs::Tab::Compliance => "compliance_results",
            #[cfg(feature = "database")]
            super::tabs::Tab::Storage => "storage_results",
            #[cfg(not(feature = "database"))]
            super::tabs::Tab::Storage => "storage_results",
            #[cfg(feature = "external-integrations")]
            super::tabs::Tab::Integrations => "integration_results",
            #[cfg(not(feature = "external-integrations"))]
            super::tabs::Tab::Integrations => "integration_results",
            #[cfg(feature = "finding-workflow")]
            super::tabs::Tab::Workflow => "workflow_results",
            #[cfg(not(feature = "finding-workflow"))]
            super::tabs::Tab::Workflow => "workflow_results",
            #[cfg(feature = "vuln-management")]
            super::tabs::Tab::Vuln => "vuln_results",
            #[cfg(not(feature = "vuln-management"))]
            super::tabs::Tab::Vuln => "vuln_results",
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
                if let Some(results) = self.recon.get_results() {
                    match serde_json::to_string_pretty(results) {
                        Ok(json) => self.save_export("recon_results.json", json),
                        Err(e) => tracing::error!("Failed to serialize recon results: {}", e),
                    }
                }
            }
            super::tabs::Tab::Load => {
                if let Some(results) = self.load.get_results() {
                    match serde_json::to_string_pretty(results) {
                        Ok(json) => self.save_export("load_results.json", json),
                        Err(e) => tracing::error!("Failed to serialize load results: {}", e),
                    }
                }
            }
            super::tabs::Tab::ScanPorts => {
                if let Some(results) = self.scan_ports.get_results() {
                    match serde_json::to_string_pretty(results) {
                        Ok(json) => self.save_export("port_scan_results.json", json),
                        Err(e) => tracing::error!("Failed to serialize port scan results: {}", e),
                    }
                }
            }
            super::tabs::Tab::ScanEndpoints => {
                if let Some(results) = self.scan_endpoints.get_results() {
                    match serde_json::to_string_pretty(results) {
                        Ok(json) => self.save_export("endpoint_scan_results.json", json),
                        Err(e) => {
                            tracing::error!("Failed to serialize endpoint scan results: {}", e)
                        }
                    }
                }
            }
            super::tabs::Tab::Fingerprint => {
                if let Some(results) = self.fingerprint.get_results() {
                    match serde_json::to_string_pretty(results) {
                        Ok(json) => self.save_export("fingerprint_results.json", json),
                        Err(e) => tracing::error!("Failed to serialize fingerprint results: {}", e),
                    }
                }
            }
            super::tabs::Tab::Fuzz => {
                if let Some(results) = self.fuzz.get_results() {
                    match serde_json::to_string_pretty(results) {
                        Ok(json) => self.save_export("fuzz_results.json", json),
                        Err(e) => tracing::error!("Failed to serialize fuzz results: {}", e),
                    }
                }
            }
            super::tabs::Tab::Waf => {
                if let Some(results) = self.waf.get_detection_result() {
                    match serde_json::to_string_pretty(results) {
                        Ok(json) => self.save_export("waf_detection_results.json", json),
                        Err(e) => {
                            tracing::error!("Failed to serialize WAF detection results: {}", e)
                        }
                    }
                }
                if let Some(results) = self.waf.get_bypass_results() {
                    match serde_json::to_string_pretty(results) {
                        Ok(json) => self.save_export("waf_bypass_results.json", json),
                        Err(e) => tracing::error!("Failed to serialize WAF bypass results: {}", e),
                    }
                }
            }
            super::tabs::Tab::WafStress => {
                if let Some(results) = self.waf_stress.get_results() {
                    self.save_export("waf_stress_results.json", results);
                }
            }
            super::tabs::Tab::Scan => {
                if let Some(report) = self.scan.get_report() {
                    match serde_json::to_string_pretty(report) {
                        Ok(json) => self.save_export("pipeline_scan_report.json", json),
                        Err(e) => {
                            tracing::error!("Failed to serialize pipeline scan report: {}", e)
                        }
                    }
                }
            }
            super::tabs::Tab::Resume => {
                tracing::warn!("Resume tab: no exportable data (use original scan results)");
            }
            super::tabs::Tab::GraphQl => {
                tracing::warn!("GraphQL tab: no exportable data available");
            }
            super::tabs::Tab::OAuth => {
                tracing::warn!("OAuth tab: no exportable data available");
            }
            super::tabs::Tab::Cluster => {
                tracing::warn!("Cluster tab: no exportable data available");
            }
            super::tabs::Tab::Stress => {
                tracing::warn!("Stress tab: no exportable data available");
            }
            super::tabs::Tab::Report => {
                tracing::warn!(
                    "Report tab: use conversion endpoints (HTML/Markdown/SARIF) instead"
                );
            }
            super::tabs::Tab::Nse => {
                tracing::warn!("NSE tab: no exportable data available");
            }
            super::tabs::Tab::Plugin => {
                tracing::warn!("Plugin tab: no exportable data available");
            }
            super::tabs::Tab::Settings => {
                tracing::warn!("Settings tab: no exportable data available");
            }
            super::tabs::Tab::History => {
                if let Ok(h) = self.history.lock() {
                    let history_data = h.export();
                    self.save_export("history.json", history_data);
                }
            }
            super::tabs::Tab::Dashboard => {
                tracing::warn!("Dashboard tab: no exportable data available");
            }
            super::tabs::Tab::Proxy => {
                tracing::warn!("Proxy tab: no exportable data available");
            }
            super::tabs::Tab::Packet => {
                tracing::warn!("Packet tab: no exportable data available");
            }
            super::tabs::Tab::Hunt => {
                tracing::warn!("Hunt tab: no exportable data available");
            }
            super::tabs::Tab::Browser => {
                tracing::warn!("Browser tab: no exportable data available");
            }
            super::tabs::Tab::Compliance => {
                tracing::warn!("Compliance tab: no exportable data available");
            }
            super::tabs::Tab::Storage => {
                tracing::warn!("Storage tab: no exportable data available");
            }
            super::tabs::Tab::Integrations => {
                tracing::warn!("Integrations tab: no exportable data available");
            }
            super::tabs::Tab::Workflow => {
                tracing::warn!("Workflow tab: no exportable data available");
            }
            super::tabs::Tab::Vuln => {
                tracing::warn!("Vuln tab: no exportable data available");
            }
        }
    }

    fn export_csv(&mut self, filename: &str) {
        use crate::output::csv::{CsvExporter, EndpointCsv, PortCsv};

        match self.current_tab {
            super::tabs::Tab::ScanPorts => {
                if let Some(results) = self.scan_ports.get_results() {
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
                    let csv = CsvExporter::export_ports(&ports);
                    self.save_export(filename, csv);
                }
            }
            super::tabs::Tab::ScanEndpoints => {
                if let Some(results) = self.scan_endpoints.get_results() {
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
                    let csv = CsvExporter::export_endpoints(&endpoints);
                    self.save_export(filename, csv);
                }
            }
            _ => {
                self.export_json();
            }
        }
    }

    fn export_converted(&mut self, filename: &str) {
        use crate::output::convert::load_scan_report;

        let base_name = filename
            .trim_end_matches(".html")
            .trim_end_matches(".md")
            .trim_end_matches(".sarif")
            .trim_end_matches(".junit")
            .trim_end_matches(".json");

        let json_filename = format!("{}.json", base_name);
        let export_dir = self
            .settings
            .config
            .as_ref()
            .and_then(|c| c.paths.export_dir.as_deref())
            .unwrap_or(crate::constants::DEFAULT_EXPORT_DIR);

        let base_dir = std::path::Path::new(crate::constants::DEFAULT_EXPORT_DIR);
        if let Err(e) = crate::utils::validation::validate_path_string(base_dir, export_dir) {
            tracing::error!("Invalid export directory: {}", e);
            return;
        }

        let json_path = format!("{}/{}", export_dir, json_filename);

        match load_scan_report(&json_path) {
            Ok(report) => {
                let converted = match self.export_format {
                    OutputFormat::Html => crate::output::convert::convert_to_html(&report),
                    OutputFormat::Markdown => crate::output::convert::convert_to_markdown(&report),
                    OutputFormat::Sarif => crate::output::convert::convert_to_sarif(&report),
                    OutputFormat::Junit => crate::output::convert::convert_to_junit(&report),
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

    fn save_export(&self, filename: &str, data: String) {
        use std::io::Write;

        let export_dir = self
            .settings
            .config
            .as_ref()
            .and_then(|c| c.paths.export_dir.as_deref())
            .unwrap_or(crate::constants::DEFAULT_EXPORT_DIR);

        let base_dir = std::path::Path::new(crate::constants::DEFAULT_EXPORT_DIR);
        if let Err(e) = crate::utils::validation::validate_path_string(base_dir, export_dir) {
            tracing::error!("Invalid export directory: {}", e);
            return;
        }

        let path = format!("{}/{}", export_dir, filename);
        let dir = std::path::Path::new(export_dir);
        if !dir.exists() {
            let _ = std::fs::create_dir_all(dir);
        }

        let mut file = match std::fs::File::create(&path) {
            Ok(file) => file,
            Err(e) => {
                tracing::error!("Could not create export file: {}", e);
                return;
            }
        };

        if let Err(e) = file.write_all(data.as_bytes()) {
            tracing::error!("Could not write to export file: {}", e);
        } else {
            tracing::info!("Exported results to: {}", path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{create_shared_history, App};
    use crate::tui::tabs::Tab;
    use crate::types::OutputFormat;

    fn create_test_app() -> App {
        App::new(create_shared_history())
    }

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
