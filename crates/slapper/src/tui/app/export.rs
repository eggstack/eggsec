use crate::output::ExportFormat;

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
        };

        let filename = format!("{}.{}", base_name, ext);

        match self.export_format {
            ExportFormat::Json => self.export_json(),
            ExportFormat::Csv => self.export_csv(&filename),
            ExportFormat::Html
            | ExportFormat::Markdown
            | ExportFormat::Sarif
            | ExportFormat::Junit => {
                self.export_json();
                self.export_converted(&filename);
            }
        }
    }

    pub(super) fn export_json(&mut self) {
        match self.current_tab {
            super::tabs::Tab::Recon => {
                if let Some(results) = self.recon.get_results() {
                    self.save_export(
                        "recon_results.json",
                        serde_json::to_string_pretty(results).unwrap_or_default(),
                    );
                }
            }
            super::tabs::Tab::Load => {
                if let Some(results) = self.load.get_results() {
                    self.save_export(
                        "load_results.json",
                        serde_json::to_string_pretty(results).unwrap_or_default(),
                    );
                }
            }
            super::tabs::Tab::ScanPorts => {
                if let Some(results) = self.scan_ports.get_results() {
                    self.save_export(
                        "port_scan_results.json",
                        serde_json::to_string_pretty(results).unwrap_or_default(),
                    );
                }
            }
            super::tabs::Tab::ScanEndpoints => {
                if let Some(results) = self.scan_endpoints.get_results() {
                    self.save_export(
                        "endpoint_scan_results.json",
                        serde_json::to_string_pretty(results).unwrap_or_default(),
                    );
                }
            }
            super::tabs::Tab::Fingerprint => {
                if let Some(results) = self.fingerprint.get_results() {
                    self.save_export(
                        "fingerprint_results.json",
                        serde_json::to_string_pretty(results).unwrap_or_default(),
                    );
                }
            }
            super::tabs::Tab::Fuzz => {
                if let Some(results) = self.fuzz.get_results() {
                    self.save_export(
                        "fuzz_results.json",
                        serde_json::to_string_pretty(results).unwrap_or_default(),
                    );
                }
            }
            super::tabs::Tab::Waf => {
                if let Some(results) = self.waf.get_detection_result() {
                    self.save_export(
                        "waf_detection_results.json",
                        serde_json::to_string_pretty(results).unwrap_or_default(),
                    );
                }
                if let Some(results) = self.waf.get_bypass_results() {
                    self.save_export(
                        "waf_bypass_results.json",
                        serde_json::to_string_pretty(results).unwrap_or_default(),
                    );
                }
            }
            super::tabs::Tab::WafStress => {
                if let Some(results) = self.waf_stress.get_results() {
                    self.save_export("waf_stress_results.json", results);
                }
            }
            super::tabs::Tab::Scan => {
                if let Some(report) = self.scan.get_report() {
                    self.save_export(
                        "pipeline_scan_report.json",
                        serde_json::to_string_pretty(report).unwrap_or_default(),
                    );
                }
            }
            super::tabs::Tab::Resume => {}
            super::tabs::Tab::GraphQl => {}
            super::tabs::Tab::OAuth => {}
            super::tabs::Tab::Cluster => {}
            super::tabs::Tab::Stress => {}
            super::tabs::Tab::Report => {}
            super::tabs::Tab::Nse => {}
            super::tabs::Tab::Plugin => {}
            super::tabs::Tab::Settings => {}
            super::tabs::Tab::History => {
                if let Ok(h) = self.history.lock() {
                    let history_data = h.export();
                    self.save_export("history.json", history_data);
                }
            }
            super::tabs::Tab::Dashboard => {}
            super::tabs::Tab::Proxy => {}
            super::tabs::Tab::Packet => {}
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
        let json_path = format!("./exports/{}", json_filename);

        if let Ok(report) = load_scan_report(&json_path) {
            let converted = match self.export_format {
                ExportFormat::Html => crate::output::convert::convert_to_html(&report),
                ExportFormat::Markdown => crate::output::convert::convert_to_markdown(&report),
                ExportFormat::Sarif => crate::output::convert::convert_to_sarif(&report),
                ExportFormat::Junit => crate::output::convert::convert_to_junit(&report),
                _ => return,
            };
            self.save_export(filename, converted);
        }
    }

    fn save_export(&self, filename: &str, data: String) {
        use std::io::Write;

        let path = format!("./exports/{}", filename);
        let dir = std::path::Path::new("./exports");
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
