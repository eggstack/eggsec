use super::{Notification, NotificationSeverity};
use eggsec::types::OutputFormat;
use std::path::{Path, PathBuf};

/// Extensions that a converted-format request (HTML/Markdown/SARIF/JUnit) is
/// named with, plus `json` for symmetry. `xml` is the JUnit extension
/// (`get_export_extension` maps `OutputFormat::Junit` to "xml"), so it must be
/// listed here: without it a `report.xml` request looked for `report.xml.json`,
/// never loaded, and still reported a successful export.
const CONVERTED_SOURCE_EXTENSIONS: [&str; 6] = ["html", "md", "sarif", "junit", "xml", "json"];

/// Outcome of one export attempt.
///
/// The converted formats consume the JSON dump written immediately before them,
/// so the caller must be able to tell "written" from "skipped" / "failed";
/// otherwise a skipped or failed dump is silently replaced by a stale file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExportOutcome {
    /// The requested file was written (a previous one may have been backed up).
    Written,
    /// Nothing to export; a user-visible warning is already set.
    Skipped,
    /// The export failed; a user-visible error is already set.
    Failed,
}

/// A failure in any part of a multi-file export wins, so a caller never treats a
/// partial export as a complete one.
fn merge_export_outcomes(first: ExportOutcome, second: ExportOutcome) -> ExportOutcome {
    match (first, second) {
        (ExportOutcome::Failed, _) | (_, ExportOutcome::Failed) => ExportOutcome::Failed,
        (ExportOutcome::Written, _) | (_, ExportOutcome::Written) => ExportOutcome::Written,
        (ExportOutcome::Skipped, ExportOutcome::Skipped) => ExportOutcome::Skipped,
    }
}

/// Strips a known export extension so a converted-format request can find the
/// intermediate JSON dump it converts from.
fn strip_export_extension(filename: &str) -> &str {
    CONVERTED_SOURCE_EXTENSIONS
        .iter()
        .find_map(|ext| filename.strip_suffix(&format!(".{}", ext)))
        .unwrap_or(filename)
}

/// Intermediate JSON dump a converted-format request reads.
fn intermediate_json_filename(filename: &str) -> String {
    format!("{}.json", strip_export_extension(filename))
}

/// Renders a loaded report in the requested format.
///
/// Every failure mode is an `Err` so a broken conversion can never be written out
/// and then reported to the user as a successful export.
fn convert_report(
    format: OutputFormat,
    report: &eggsec::output::convert::ScanReportData,
) -> Result<String, String> {
    use eggsec::output::convert::{
        convert_to_html, convert_to_junit, convert_to_markdown, convert_to_sarif,
    };

    match format {
        OutputFormat::Html => Ok(convert_to_html(report)),
        OutputFormat::Markdown => convert_to_markdown(report).map_err(|e| e.to_string()),
        OutputFormat::Sarif => convert_to_sarif(report),
        OutputFormat::Junit => convert_to_junit(report),
        other => Err(format!("unsupported export format: {:?}", other)),
    }
}

/// Moves an existing export aside to `<name>.bak` before an overwrite.
///
/// Exports are the deliverable of a scan, so a re-export must not destroy the
/// previous report with no trace. Anything that is not a plain file (directory,
/// symlink, unreadable entry) is refused so the write cannot silently destroy
/// it. `Ok(None)` means the destination was free.
fn backup_existing_export(path: &Path) -> Result<Option<PathBuf>, String> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            if !meta.is_file() {
                return Err(format!(
                    "Refusing to overwrite {}: destination is not a regular file",
                    path.display()
                ));
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(format!(
                "Could not inspect existing export {}: {}",
                path.display(),
                e
            ));
        }
    }

    let mut backup = path.as_os_str().to_os_string();
    backup.push(".bak");
    let backup = PathBuf::from(backup);
    std::fs::rename(path, &backup).map_err(|e| {
        format!(
            "Could not back up existing export {} to {}: {}",
            path.display(),
            backup.display(),
            e
        )
    })?;
    Ok(Some(backup))
}

/// Backs up any existing report, then writes `data`.
///
/// Returns the backup path when a previous report was preserved, or a
/// user-facing message on failure (including a failed backup, which aborts the
/// write rather than losing the earlier report).
fn write_export_file(path: &Path, data: &str) -> Result<Option<PathBuf>, String> {
    use std::io::Write;

    let backup = backup_existing_export(path)?;

    let mut file =
        std::fs::File::create(path).map_err(|e| format!("Could not create export file: {}", e))?;
    file.write_all(data.as_bytes())
        .map_err(|e| format!("Could not write to export file: {}", e))?;
    Ok(backup)
}

impl super::App {
    fn export_tab_json<T, F>(&mut self, get: F, filename: &str, tab_name: &str) -> ExportOutcome
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
                    ExportOutcome::Failed
                }
            },
            None => {
                let msg = format!("No exportable data for {} tab.", tab_name);
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
                ExportOutcome::Skipped
            }
        }
    }

    pub(super) fn export_results(&mut self) -> ExportOutcome {
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
                // The JSON dump is an intermediate step of the conversion, not the
                // artifact the user asked for. Convert it only when it was really
                // written, so a skipped or failed dump is never replaced by a stale
                // file, and let `export_converted` own the single user-visible
                // outcome for the request.
                match self.export_json() {
                    ExportOutcome::Written => self.export_converted(&filename),
                    outcome => {
                        tracing::debug!(
                            "Skipping {} conversion: JSON export outcome {:?}",
                            filename,
                            outcome
                        );
                        outcome
                    }
                }
            }
            _ => self.export_json(),
        }
    }

    pub(super) fn export_json(&mut self) -> ExportOutcome {
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
                    self.save_export("waf_stress_results.json", results)
                } else {
                    let msg = "No exportable data for WAF Stress tab.".to_string();
                    self.overlay.notification =
                        Some(Notification::new(msg, NotificationSeverity::Warning));
                    ExportOutcome::Skipped
                }
            }
            super::tabs::Tab::Scan => {
                self.export_tab_json(|s| s.tabs.scan.get_report(), "pipeline_scan_report", "Scan")
            }
            super::tabs::Tab::Resume => {
                let msg = "Resume tab: no exportable data (use original scan results)".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
                ExportOutcome::Skipped
            }
            super::tabs::Tab::GraphQl => {
                let msg = "GraphQL tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
                ExportOutcome::Skipped
            }
            super::tabs::Tab::OAuth => {
                let msg = "OAuth tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
                ExportOutcome::Skipped
            }
            super::tabs::Tab::Auth => {
                let msg = "Auth tab: no exportable data available (defense-lab only)".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
                ExportOutcome::Skipped
            }
            super::tabs::Tab::C2 => {
                let msg = "C2 tab: use CLI export (eggsec c2 --json -o report.json)".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
                ExportOutcome::Skipped
            }
            super::tabs::Tab::Cluster => {
                let msg = "Cluster tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
                ExportOutcome::Skipped
            }
            super::tabs::Tab::Stress => {
                let msg = "Stress tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
                ExportOutcome::Skipped
            }
            super::tabs::Tab::Report => {
                let msg = "Report tab: use conversion endpoints (HTML/Markdown/SARIF) instead"
                    .to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
                ExportOutcome::Skipped
            }
            super::tabs::Tab::Nse => {
                let msg = "NSE tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
                ExportOutcome::Skipped
            }
            super::tabs::Tab::Settings => {
                let msg = "Settings tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
                ExportOutcome::Skipped
            }
            super::tabs::Tab::History => {
                let history_data = {
                    let h = self.history.lock();
                    h.export()
                };
                self.save_export("history.json", history_data)
            }
            super::tabs::Tab::Dashboard => {
                let msg = "Dashboard tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
                ExportOutcome::Skipped
            }
            super::tabs::Tab::Proxy => {
                let msg = "Proxy tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
                ExportOutcome::Skipped
            }
            super::tabs::Tab::Packet => {
                let msg = "Packet tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
                ExportOutcome::Skipped
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
                ExportOutcome::Skipped
            }
            super::tabs::Tab::Browser => {
                let msg = "Browser tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
                ExportOutcome::Skipped
            }
            super::tabs::Tab::Compliance => {
                let msg = "Compliance tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
                ExportOutcome::Skipped
            }
            super::tabs::Tab::Storage => {
                let msg = "Storage tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
                ExportOutcome::Skipped
            }
            super::tabs::Tab::Integrations => {
                let msg = "Integrations tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
                ExportOutcome::Skipped
            }
            super::tabs::Tab::Workflow => {
                let msg = "Workflow tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
                ExportOutcome::Skipped
            }
            super::tabs::Tab::Vuln => {
                let msg = "Vuln tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
                ExportOutcome::Skipped
            }
            super::tabs::Tab::Wireless => {
                let msg = "Wireless tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
                ExportOutcome::Skipped
            }
            super::tabs::Tab::Intercept => {
                let msg = "Intercept tab: no exportable data available".to_string();
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Warning));
                ExportOutcome::Skipped
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
                            ExportOutcome::Failed
                        }
                    }
                } else {
                    let msg = "Db Pentest tab: no exportable data available".to_string();
                    self.overlay.notification =
                        Some(Notification::new(msg, NotificationSeverity::Warning));
                    ExportOutcome::Skipped
                }
            }
            #[cfg(not(feature = "db-pentest"))]
            super::tabs::Tab::DbPentest => ExportOutcome::Skipped,
        }
    }

    fn export_waf_json(&mut self) -> ExportOutcome {
        let detection_json = match self.tabs.waf.get_detection_result() {
            Some(r) => match serde_json::to_string_pretty(&r) {
                Ok(j) => Some(j),
                Err(e) => {
                    tracing::error!("Failed to serialize WAF detection results: {}", e);
                    self.overlay.notification = Some(Notification::new(
                        format!("Export failed: {}", e),
                        NotificationSeverity::Error,
                    ));
                    return ExportOutcome::Failed;
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
                    return ExportOutcome::Failed;
                }
            },
            None => None,
        };

        if detection_json.is_none() && bypass_json.is_none() {
            let msg = "No exportable data for WAF tab.".to_string();
            self.overlay.notification = Some(Notification::new(msg, NotificationSeverity::Warning));
            return ExportOutcome::Skipped;
        }

        // Two files: a failure on either side is a failed export, so a partial
        // write is never reported as a complete one.
        let mut outcome = ExportOutcome::Skipped;
        if let Some(json) = detection_json {
            outcome = self.save_export("waf_detection_results.json", json);
        }
        if let Some(json) = bypass_json {
            outcome =
                merge_export_outcomes(outcome, self.save_export("waf_bypass_results.json", json));
        }
        outcome
    }

    fn export_csv(&mut self, filename: &str) -> ExportOutcome {
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
                            // From the record, not a literal: a UDP run would
                            // otherwise export every row as tcp/open.
                            protocol: p.protocol.as_str().to_string(),
                            service: Some(p.service.clone()),
                            version: None,
                            state: p.status.as_str().to_string(),
                        })
                        .collect();
                    match CsvExporter::export_ports(&ports) {
                        Ok(csv) => self.save_export(filename, csv),
                        Err(e) => {
                            let msg = format!("Failed to export ports to CSV: {}", e);
                            tracing::error!("{}", msg);
                            self.overlay.notification =
                                Some(Notification::new(msg, NotificationSeverity::Error));
                            ExportOutcome::Failed
                        }
                    }
                } else {
                    ExportOutcome::Skipped
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
                            ExportOutcome::Failed
                        }
                    }
                } else {
                    ExportOutcome::Skipped
                }
            }
            _ => self.export_json(),
        }
    }

    fn export_converted(&mut self, filename: &str) -> ExportOutcome {
        use eggsec::output::convert::load_scan_report;

        let json_filename = intermediate_json_filename(filename);
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
            return ExportOutcome::Failed;
        }

        let json_path = format!("{}/{}", export_dir, json_filename);
        let report = match load_scan_report(&json_path) {
            Ok(report) => report,
            Err(e) => {
                // The converted file is the artifact the user asked for: report the
                // failure instead of leaving the intermediate "Exported to" success
                // as the last thing on screen.
                let msg = format!(
                    "Export to {} failed: {} (source: {})",
                    filename, e, json_path
                );
                tracing::error!("{}", msg);
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Error));
                return ExportOutcome::Failed;
            }
        };

        let converted = match convert_report(self.export_format, &report) {
            Ok(converted) => converted,
            Err(e) => {
                let msg = format!("Export to {} failed: {}", filename, e);
                tracing::error!("{}", msg);
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Error));
                return ExportOutcome::Failed;
            }
        };

        self.save_export(filename, converted)
    }

    fn save_export(&mut self, filename: &str, data: String) -> ExportOutcome {
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
            return ExportOutcome::Failed;
        }

        let path = format!("{}/{}", export_dir, filename);
        let dir = std::path::Path::new(export_dir);
        if !dir.exists() {
            if let Err(e) = std::fs::create_dir_all(dir) {
                let msg = format!("Could not create export directory: {}", e);
                tracing::error!("{}", msg);
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Error));
                return ExportOutcome::Failed;
            }
        }

        // Never destroy a previous report silently: `write_export_file` moves it
        // to `<name>.bak` first and refuses the write when that cannot be done.
        match write_export_file(std::path::Path::new(&path), &data) {
            Ok(backup) => {
                let msg = match backup {
                    Some(backup) => format!(
                        "Exported to: {} (previous report kept at {})",
                        path,
                        backup.display()
                    ),
                    None => format!("Exported to: {}", path),
                };
                tracing::info!("{}", msg);
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Success));
                ExportOutcome::Written
            }
            Err(msg) => {
                tracing::error!("{}", msg);
                self.overlay.notification =
                    Some(Notification::new(msg, NotificationSeverity::Error));
                ExportOutcome::Failed
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::create_test_app;
    use super::{
        convert_report, intermediate_json_filename, merge_export_outcomes, strip_export_extension,
        write_export_file, ExportOutcome,
    };
    use crate::tabs::Tab;
    use eggsec::types::OutputFormat;
    use std::path::PathBuf;

    use super::super::NotificationSeverity;

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

    /// Fresh per-test scratch directory under the OS temp dir (mirrors
    /// `theme::install::tests`), so the export core can be exercised without
    /// touching the configured export directory.
    fn temp_export_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "eggsec_export_test_{}_{}",
            std::process::id(),
            name
        ));
        if let Err(e) = std::fs::remove_dir_all(&dir) {
            tracing::debug!("test export dir already absent: {}", e);
        }
        std::fs::create_dir_all(&dir).expect("create test export dir");
        dir
    }

    fn sample_report() -> eggsec::output::convert::ScanReportData {
        serde_json::from_str(
            r#"{
                "target": "example.com",
                "scan_type": "port-scan",
                "timestamp": "2026-01-01T00:00:00Z",
                "findings": [
                    {
                        "title": "Weak TLS configuration",
                        "severity": "high",
                        "category": "transport",
                        "description": "TLS 1.0 accepted",
                        "location": "example.com:443",
                        "evidence": null,
                        "remediation": null,
                        "cwe_ids": []
                    }
                ],
                "open_ports": [],
                "services": [],
                "duration_ms": 1
            }"#,
        )
        .expect("sample scan report")
    }

    #[test]
    fn test_intermediate_json_filename_strips_junit_xml_extension() {
        // Regression: a JUnit request is named `*.xml`; the conversion source must
        // be `*.json`, never `*.xml.json`.
        assert_eq!(
            intermediate_json_filename("port_scan_results.xml"),
            "port_scan_results.json"
        );
        assert_eq!(
            intermediate_json_filename("port_scan_results.html"),
            "port_scan_results.json"
        );
        assert_eq!(
            intermediate_json_filename("port_scan_results.md"),
            "port_scan_results.json"
        );
        assert_eq!(
            intermediate_json_filename("port_scan_results.sarif"),
            "port_scan_results.json"
        );
        assert_eq!(
            intermediate_json_filename("port_scan_results.junit"),
            "port_scan_results.json"
        );
        assert_eq!(
            intermediate_json_filename("port_scan_results.json"),
            "port_scan_results.json"
        );
    }

    #[test]
    fn test_strip_export_extension_leaves_unknown_extension_alone() {
        assert_eq!(strip_export_extension("report.txt"), "report.txt");
        assert_eq!(strip_export_extension("report"), "report");
    }

    #[test]
    fn test_convert_report_junit_emits_junit_xml() {
        let report = sample_report();
        let xml = convert_report(OutputFormat::Junit, &report).expect("junit conversion");
        assert!(
            xml.contains("<testsuites"),
            "JUnit export must emit a JUnit XML body, got: {}",
            xml
        );
        assert!(
            !xml.starts_with("Error:"),
            "a failed conversion must never be written as the artifact"
        );
    }

    #[test]
    fn test_convert_report_html_and_sarif_emit_their_own_format() {
        let report = sample_report();
        let html = convert_report(OutputFormat::Html, &report).expect("html conversion");
        assert!(html.contains("<html"), "HTML export should be HTML");
        let sarif = convert_report(OutputFormat::Sarif, &report).expect("sarif conversion");
        assert!(
            sarif.contains("\"runs\""),
            "SARIF export should be SARIF JSON, got: {}",
            sarif
        );
    }

    #[test]
    fn test_convert_report_rejects_unsupported_format() {
        let report = sample_report();
        let err = convert_report(OutputFormat::Pretty, &report)
            .expect_err("unsupported formats must be an error, not a silent return");
        assert!(err.contains("unsupported export format"), "got: {}", err);
    }

    #[test]
    fn test_write_export_file_reports_no_backup_for_new_file() {
        let dir = temp_export_dir("new_file");
        let path = dir.join("report.json");
        let backup = write_export_file(&path, "first").expect("first write");
        assert!(backup.is_none(), "a free destination needs no backup");
        assert_eq!(std::fs::read_to_string(&path).expect("read"), "first");
    }

    #[test]
    fn test_write_export_file_keeps_previous_report_as_backup() {
        let dir = temp_export_dir("backup");
        let path = dir.join("report.json");
        write_export_file(&path, "first").expect("first write");

        let backup = write_export_file(&path, "second")
            .expect("second write")
            .expect("previous report must be preserved");
        assert_eq!(backup, dir.join("report.json.bak"));
        assert_eq!(
            std::fs::read_to_string(&backup).expect("read backup"),
            "first",
            "the previous report must survive the overwrite"
        );
        assert_eq!(std::fs::read_to_string(&path).expect("read"), "second");
    }

    #[test]
    fn test_write_export_file_refuses_non_regular_destination() {
        let dir = temp_export_dir("non_regular");
        let path = dir.join("report.json");
        std::fs::create_dir_all(&path).expect("create colliding directory");

        let err = write_export_file(&path, "data")
            .expect_err("a directory destination must not be overwritten");
        assert!(
            err.contains("not a regular file"),
            "unexpected error message: {}",
            err
        );
        assert!(path.is_dir(), "the destination must be left untouched");
    }

    #[test]
    fn test_merge_export_outcomes_prefers_failure() {
        assert_eq!(
            merge_export_outcomes(ExportOutcome::Written, ExportOutcome::Failed),
            ExportOutcome::Failed
        );
        assert_eq!(
            merge_export_outcomes(ExportOutcome::Skipped, ExportOutcome::Written),
            ExportOutcome::Written
        );
        assert_eq!(
            merge_export_outcomes(ExportOutcome::Skipped, ExportOutcome::Skipped),
            ExportOutcome::Skipped
        );
    }

    #[test]
    fn test_export_json_without_data_is_skipped_with_warning() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        assert_eq!(app.export_json(), ExportOutcome::Skipped);
        let notif = app
            .overlay
            .notification
            .as_ref()
            .expect("a skipped export must tell the user");
        assert_eq!(notif.severity, NotificationSeverity::Warning);
    }

    #[test]
    fn test_export_results_junit_without_data_reports_no_success() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        app.export_format = OutputFormat::Junit;

        assert_eq!(app.export_results(), ExportOutcome::Skipped);
        let notif = app
            .overlay
            .notification
            .as_ref()
            .expect("a skipped export must tell the user");
        assert_ne!(
            notif.severity,
            NotificationSeverity::Success,
            "a JUnit request with no data must not report success"
        );
    }
}
