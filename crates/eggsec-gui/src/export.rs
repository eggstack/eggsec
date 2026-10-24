//! Report export: the GUI's six artifact formats, and the write discipline.
//!
//! The GUI produces the **same artifacts the CLI produces**, because both go
//! through the same `eggsec-output` renderers. There is no GUI-specific
//! report format and no second renderer.
//!
//! Two disciplines are borrowed from the TUI's export path and are not
//! negotiable here:
//!
//! - **A destination that is not a regular file is refused.** A directory, a
//!   symlink, or a device is never overwritten, because an export is the
//!   deliverable of a scan and destroying whatever sits at the destination
//!   is not the GUI's decision to make.
//! - **An existing export is backed up, not silently replaced.** The previous
//!   report is moved aside to `<path>.bak` before the new one is written.
//!
//! Every failure is an `Err`. A failed conversion is never written out and
//! then reported to the operator as a successful export.
//!
//! # How the GUI shows a report
//!
//! The GUI **does not render the engine's HTML document**. That document
//! carries an inline `<style>` block and an inline `<script>` block, and the
//! GUI's Content Security Policy (`crates/eggsec-gui/tauri.conf.json`) sets
//! `style-src 'self'`, `script-src 'self'` and `frame-src 'none'`: an inline
//! style does not apply, an inline script does not execute, and the document
//! cannot be framed either. Rendering it faithfully is impossible without
//! relaxing the CSP, and relaxing the CSP is the wrong trade — the report's
//! content is attacker-influenced (banners, service names and finding text
//! all come from the target), and the CSP is the control that stops that
//! content from doing anything.
//!
//! So the GUI takes shape **(a)**: the report travels to the webview as
//! **data** ([`ReportView`]), and the frontend renders it with the GUI's own
//! stylesheet and its own escaping. No engine HTML is injected into the
//! webview at all, and no inline `<script>` or `<style>` from a report
//! document can execute or apply in the GUI.
//!
//! The **exported** file is a different thing entirely: it is the operator's
//! artifact, opened in the operator's browser, and it is the CLI's document
//! byte for byte — `convert_to_html`, with its remote chart-library
//! reference and all.
//!
//! The text formats keep a display rendering ([`render_for_display`]): JSON,
//! CSV, Markdown, SARIF and JUnit are inert text, and the frontend shows
//! them as text. The HTML format has **no** display rendering: the GUI
//! renders the report's structured content instead.

use std::fmt;
use std::path::{Path, PathBuf};

use eggsec_output::convert::{
    convert_to_csv, convert_to_html, convert_to_json, convert_to_junit, convert_to_markdown,
    convert_to_sarif, ScanReportData,
};

/// The report formats the GUI can export.
///
/// This is the CLI's six — JSON, CSV, HTML, Markdown, SARIF, JUnit — and not
/// PDF: the PDF renderer lives in the engine, and the GUI links no engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportFormat {
    /// The machine-readable report.
    Json,
    /// The comma-separated findings table.
    Csv,
    /// The HTML report.
    Html,
    /// The SARIF interchange document.
    Sarif,
    /// The JUnit XML document.
    Junit,
    /// The Markdown report.
    Markdown,
}

impl ReportFormat {
    /// Every format, in the order the operator sees them.
    pub const ALL: [ReportFormat; 6] = [
        ReportFormat::Json,
        ReportFormat::Csv,
        ReportFormat::Html,
        ReportFormat::Sarif,
        ReportFormat::Junit,
        ReportFormat::Markdown,
    ];

    /// The file extension this format is written with.
    ///
    /// JUnit is written as `.xml` and Markdown as `.md` — match the CLI.
    pub fn extension(self) -> &'static str {
        match self {
            ReportFormat::Json => "json",
            ReportFormat::Csv => "csv",
            ReportFormat::Html => "html",
            ReportFormat::Sarif => "sarif",
            ReportFormat::Junit => "xml",
            ReportFormat::Markdown => "md",
        }
    }

    /// Parse a format name as the operator typed it.
    ///
    /// The name is matched case-insensitively, and the two extensions the
    /// CLI writes are accepted as aliases: `xml` is the JUnit artifact and
    /// `md` is the Markdown one.
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "json" => Some(ReportFormat::Json),
            "csv" => Some(ReportFormat::Csv),
            "html" => Some(ReportFormat::Html),
            "sarif" => Some(ReportFormat::Sarif),
            "junit" | "xml" => Some(ReportFormat::Junit),
            "markdown" | "md" => Some(ReportFormat::Markdown),
            _ => None,
        }
    }

    /// The name the export command takes for this format.
    pub fn value(self) -> &'static str {
        match self {
            ReportFormat::Json => "json",
            ReportFormat::Csv => "csv",
            ReportFormat::Html => "html",
            ReportFormat::Sarif => "sarif",
            ReportFormat::Junit => "junit",
            ReportFormat::Markdown => "markdown",
        }
    }

    /// The name the operator sees for this format.
    pub fn label(self) -> &'static str {
        match self {
            ReportFormat::Json => "JSON",
            ReportFormat::Csv => "CSV",
            ReportFormat::Html => "HTML",
            ReportFormat::Sarif => "SARIF",
            ReportFormat::Junit => "JUnit XML",
            ReportFormat::Markdown => "Markdown",
        }
    }
}

/// One export format, as the webview renders it.
///
/// The format list is delivered from the Rust side so the frontend holds no
/// catalog of its own: a format the GUI cannot write is never offered.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReportFormatView {
    /// The format's name, as the export command takes it.
    pub value: String,
    /// The name the operator sees.
    pub label: String,
    /// The extension the format is written with.
    pub extension: String,
}

impl From<ReportFormat> for ReportFormatView {
    fn from(format: ReportFormat) -> Self {
        Self {
            value: format.value().to_owned(),
            label: format.label().to_owned(),
            extension: format.extension().to_owned(),
        }
    }
}

/// Every export format, as the webview renders it.
pub fn format_views() -> Vec<ReportFormatView> {
    ReportFormat::ALL
        .into_iter()
        .map(ReportFormatView::from)
        .collect()
}

/// The report's structured content, as the webview renders it.
///
/// This is the GUI's **data** view of a report: the fields the report
/// carries, as plain values. The frontend renders them with the GUI's own
/// stylesheet and its own escaping — no engine HTML is injected into the
/// webview, and nothing from the report can become markup.
///
/// Every field is a string, a number or a list of these views. There is no
/// field that carries markup, and there is no field that carries a URL the
/// webview would follow.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReportView {
    /// The target the scan ran against.
    pub target: String,
    /// The kind of scan the report describes.
    pub scan_type: String,
    /// When the scan finished.
    pub timestamp: String,
    /// How long the scan took, in milliseconds.
    pub duration_ms: u64,
    /// The findings the scan produced.
    pub findings: Vec<FindingView>,
    /// The open ports the scan found.
    pub open_ports: Vec<PortView>,
    /// The services the scan identified.
    pub services: Vec<ServiceView>,
}

/// One finding, as the webview renders it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FindingView {
    /// The finding's title.
    pub title: String,
    /// The finding's severity.
    pub severity: String,
    /// The category the finding belongs to.
    pub category: String,
    /// What the finding says.
    pub description: String,
    /// Where the finding was found.
    pub location: String,
    /// The evidence that supports the finding.
    pub evidence: Option<String>,
    /// What to do about the finding.
    pub remediation: Option<String>,
    /// The CWE identifiers the finding maps to.
    pub cwe_ids: Vec<String>,
}

/// One open port, as the webview renders it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PortView {
    /// The port number.
    pub port: u16,
    /// The port's status, as the scan reported it.
    pub status: String,
    /// The transport protocol.
    pub protocol: Option<String>,
    /// The port's state.
    pub state: Option<String>,
    /// The service name, when one was identified.
    pub service: Option<String>,
    /// The service version, when one was identified.
    pub version: Option<String>,
    /// The banner the service returned.
    ///
    /// A banner comes from the target, so it is attacker-influenced text.
    /// It is rendered as text, never as markup.
    pub banner: Option<String>,
}

/// One identified service, as the webview renders it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ServiceView {
    /// The service name.
    pub service: String,
    /// The service version.
    pub version: Option<String>,
    /// The banner the service returned.
    ///
    /// A banner comes from the target, so it is attacker-influenced text.
    /// It is rendered as text, never as markup.
    pub banner: Option<String>,
}

/// Build the webview's data view of a report.
pub fn report_view(report: &ScanReportData) -> ReportView {
    ReportView {
        target: report.target.clone(),
        scan_type: report.scan_type.clone(),
        timestamp: report.timestamp.clone(),
        duration_ms: report.duration_ms,
        findings: report
            .findings
            .iter()
            .map(|finding| FindingView {
                title: finding.title.clone(),
                severity: finding.severity.clone(),
                category: finding.category.clone(),
                description: finding.description.clone(),
                location: finding.location.clone(),
                evidence: finding.evidence.clone(),
                remediation: finding.remediation.clone(),
                cwe_ids: finding.cwe_ids.clone(),
            })
            .collect(),
        open_ports: report
            .open_ports
            .iter()
            .map(|port| PortView {
                port: port.port,
                status: port.status.clone(),
                protocol: port.protocol.clone(),
                state: port.state.clone(),
                service: port.service.clone(),
                version: port.version.clone(),
                banner: port.banner.clone(),
            })
            .collect(),
        services: report
            .services
            .iter()
            .map(|service| ServiceView {
                service: service.service.clone(),
                version: service.version.clone(),
                banner: service.banner.clone(),
            })
            .collect(),
    }
}

/// Render a report document in the requested format.
///
/// This is the CLI's artifact: the same bytes the CLI writes for the same
/// report, because both call the same `eggsec-output` renderer.
pub fn render(format: ReportFormat, report: &ScanReportData) -> Result<String, ExportError> {
    match format {
        ReportFormat::Json => convert_to_json(report).map_err(ExportError::Render),
        ReportFormat::Csv => Ok(convert_to_csv(report)),
        ReportFormat::Html => Ok(convert_to_html(report)),
        ReportFormat::Sarif => convert_to_sarif(report).map_err(ExportError::Render),
        ReportFormat::Junit => convert_to_junit(report).map_err(ExportError::Render),
        ReportFormat::Markdown => {
            convert_to_markdown(report).map_err(|error| ExportError::Render(error.to_string()))
        }
    }
}

/// Render the report for display inside the GUI.
///
/// The text formats render exactly as the exported artifact does: they are
/// inert text, and the frontend shows them as text.
///
/// The **HTML format has no display rendering**. The engine's document
/// carries an inline `<style>` block and an inline `<script>` block, and
/// the GUI's Content Security Policy blocks both — so rendering it would
/// show an unstyled page with a dead script, and rendering it faithfully
/// would mean relaxing the policy. The GUI shows the report's **structured
/// content** instead ([`ReportView`]), rendered by the frontend with the
/// GUI's own stylesheet and its own escaping.
pub fn render_for_display(
    format: ReportFormat,
    report: &ScanReportData,
) -> Result<String, ExportError> {
    match format {
        ReportFormat::Html => Err(ExportError::NoDisplayRendering {
            format: format.value(),
        }),
        other => render(other, report),
    }
}

/// Write a rendered report to `path`.
///
/// Refuses to write over anything that is not a regular file, and backs an
/// existing regular file up to `<path>.bak` before replacing it.
pub fn write_export(path: &Path, contents: &str) -> Result<(), ExportError> {
    use std::io::Write;

    backup_existing_export(path)?;

    let mut file = std::fs::File::create(path)
        .map_err(|error| ExportError::Create(path.to_path_buf(), error))?;
    file.write_all(contents.as_bytes())
        .map_err(|error| ExportError::Write(path.to_path_buf(), error))?;
    Ok(())
}

/// Move an existing export aside to `<path>.bak` before an overwrite.
///
/// Anything that is not a plain file (a directory, a symlink, an unreadable
/// entry) is refused, so the write cannot silently destroy it. `Ok(None)`
/// means the destination was free.
fn backup_existing_export(path: &Path) -> Result<Option<PathBuf>, ExportError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.is_file() {
                return Err(ExportError::NotARegularFile(path.to_path_buf()));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(ExportError::Inspect(path.to_path_buf(), error)),
    }

    let mut backup = path.as_os_str().to_os_string();
    backup.push(".bak");
    let backup = PathBuf::from(backup);
    std::fs::rename(path, &backup)
        .map_err(|error| ExportError::Backup(path.to_path_buf(), backup.clone(), error))?;
    Ok(Some(backup))
}

/// Everything that can go wrong while exporting a report.
#[derive(Debug)]
pub enum ExportError {
    /// The renderer refused to produce a document.
    Render(String),
    /// The format has no display rendering.
    ///
    /// The GUI shows the report's structured content instead of a document.
    NoDisplayRendering {
        /// The format that has no display rendering.
        format: &'static str,
    },
    /// The destination exists and is not a regular file.
    NotARegularFile(PathBuf),
    /// The destination could not be inspected.
    Inspect(PathBuf, std::io::Error),
    /// The existing export could not be backed up.
    Backup(PathBuf, PathBuf, std::io::Error),
    /// The export file could not be created.
    Create(PathBuf, std::io::Error),
    /// The export file could not be written.
    Write(PathBuf, std::io::Error),
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportError::Render(message) => {
                write!(f, "the report could not be rendered: {message}")
            }
            ExportError::NoDisplayRendering { format } => write!(
                f,
                "the {format} format has no display rendering: the GUI shows the report's \
                 structured content, because the engine's document carries an inline style and an \
                 inline script that the GUI's Content Security Policy blocks"
            ),
            ExportError::NotARegularFile(path) => write!(
                f,
                "refusing to overwrite {}: the destination is not a regular file",
                path.display()
            ),
            ExportError::Inspect(path, error) => write!(
                f,
                "could not inspect the existing export {}: {error}",
                path.display()
            ),
            ExportError::Backup(path, backup, error) => write!(
                f,
                "could not back up the existing export {} to {}: {error}",
                path.display(),
                backup.display()
            ),
            ExportError::Create(path, error) => {
                write!(
                    f,
                    "could not create the export file {}: {error}",
                    path.display()
                )
            }
            ExportError::Write(path, error) => {
                write!(
                    f,
                    "could not write the export file {}: {error}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ExportError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixed report, so every format renders the same artifact.
    fn sample_report() -> ScanReportData {
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

    /// A scratch directory under the OS temp dir, so the write discipline
    /// can be exercised without touching an operator's files.
    fn temp_export_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "eggsec_gui_export_test_{}_{}",
            std::process::id(),
            name
        ));
        if let Err(error) = std::fs::remove_dir_all(&dir) {
            tracing::debug!("the test export directory was already absent: {error}");
        }
        std::fs::create_dir_all(&dir).expect("create the test export directory");
        dir
    }

    #[test]
    fn every_format_has_the_cli_extension() {
        assert_eq!(ReportFormat::Json.extension(), "json");
        assert_eq!(ReportFormat::Csv.extension(), "csv");
        assert_eq!(ReportFormat::Html.extension(), "html");
        assert_eq!(ReportFormat::Sarif.extension(), "sarif");
        // JUnit is XML on disk, and Markdown is `.md`: the CLI's map.
        assert_eq!(ReportFormat::Junit.extension(), "xml");
        assert_eq!(ReportFormat::Markdown.extension(), "md");
    }

    #[test]
    fn every_format_parses_from_its_own_name() {
        assert_eq!(ReportFormat::parse("json"), Some(ReportFormat::Json));
        assert_eq!(ReportFormat::parse("csv"), Some(ReportFormat::Csv));
        assert_eq!(ReportFormat::parse("html"), Some(ReportFormat::Html));
        assert_eq!(ReportFormat::parse("sarif"), Some(ReportFormat::Sarif));
        assert_eq!(ReportFormat::parse("junit"), Some(ReportFormat::Junit));
        assert_eq!(
            ReportFormat::parse("markdown"),
            Some(ReportFormat::Markdown)
        );
    }

    #[test]
    fn a_format_parses_case_insensitively_and_by_its_extension() {
        assert_eq!(ReportFormat::parse("JSON"), Some(ReportFormat::Json));
        assert_eq!(ReportFormat::parse("  Html  "), Some(ReportFormat::Html));
        assert_eq!(ReportFormat::parse("xml"), Some(ReportFormat::Junit));
        assert_eq!(ReportFormat::parse("md"), Some(ReportFormat::Markdown));
    }

    #[test]
    fn an_unknown_format_is_not_guessed() {
        assert_eq!(ReportFormat::parse("pdf"), None);
        assert_eq!(ReportFormat::parse(""), None);
    }

    #[test]
    fn every_format_renders_a_document() {
        let report = sample_report();
        for format in ReportFormat::ALL {
            let rendered = render(format, &report)
                .unwrap_or_else(|error| panic!("{format:?} did not render: {error}"));
            assert!(
                !rendered.is_empty(),
                "{format:?} rendered an empty document"
            );
        }
    }

    /// The SARIF document, with the invocation timestamps removed.
    ///
    /// The SARIF renderer stamps the render time into the document, so two
    /// renders of the same report differ in exactly that field. Comparing
    /// the documents without it is the strongest comparison that does not
    /// depend on the clock.
    fn sarif_without_timestamps(document: &str) -> serde_json::Value {
        let mut value: serde_json::Value =
            serde_json::from_str(document).expect("a SARIF document is JSON");
        if let Some(runs) = value.get_mut("runs").and_then(|runs| runs.as_array_mut()) {
            for run in runs {
                if let Some(run) = run.as_object_mut() {
                    run.remove("invocations");
                }
            }
        }
        value
    }

    /// The JUnit document, with the render timestamp removed.
    ///
    /// The JUnit renderer stamps the render time into the document, so two
    /// renders of the same report differ in exactly that attribute.
    fn junit_without_timestamp(document: &str) -> String {
        let Some((before, rest)) = document.split_once("timestamp=\"") else {
            return document.to_owned();
        };
        let Some((_, after)) = rest.split_once('"') else {
            return document.to_owned();
        };
        format!("{before}timestamp=\"TIMESTAMP\"{after}")
    }

    #[test]
    fn the_rendered_artifacts_match_the_shared_renderers() {
        // The GUI is a thin wrapper over the shared renderers: the exported
        // bytes must be the bytes the CLI writes for the same report. The
        // comparison is against the shared renderer, not against a captured
        // CLI output file, which would go stale.
        use eggsec_output::convert::{
            convert_to_csv, convert_to_html, convert_to_json, convert_to_junit,
            convert_to_markdown, convert_to_sarif,
        };

        let report = sample_report();
        assert_eq!(
            render(ReportFormat::Json, &report).expect("json"),
            convert_to_json(&report).expect("json")
        );
        assert_eq!(
            render(ReportFormat::Csv, &report).expect("csv"),
            convert_to_csv(&report)
        );
        assert_eq!(
            render(ReportFormat::Html, &report).expect("html"),
            convert_to_html(&report)
        );
        assert_eq!(
            sarif_without_timestamps(&render(ReportFormat::Sarif, &report).expect("sarif")),
            sarif_without_timestamps(&convert_to_sarif(&report).expect("sarif"))
        );
        assert_eq!(
            junit_without_timestamp(&render(ReportFormat::Junit, &report).expect("junit")),
            junit_without_timestamp(&convert_to_junit(&report).expect("junit"))
        );
        assert_eq!(
            render(ReportFormat::Markdown, &report).expect("markdown"),
            convert_to_markdown(&report).expect("markdown")
        );
    }

    #[test]
    fn the_html_format_has_no_display_rendering() {
        // The engine's document carries an inline `<style>` and an inline
        // `<script>`, and the GUI's Content Security Policy blocks both.
        // The GUI therefore has no HTML display rendering at all: it shows
        // the report's structured content instead.
        let report = sample_report();
        let error =
            render_for_display(ReportFormat::Html, &report).expect_err("html has no display form");
        let message = error.to_string();
        assert!(
            message.contains("no display rendering"),
            "the error must say why: {message}"
        );
        assert!(
            message.contains("Content Security Policy"),
            "the error must name the reason: {message}"
        );
    }

    #[test]
    fn no_display_rendering_carries_a_script_a_style_or_a_remote_reference() {
        // Nothing the GUI shows may be able to execute or apply. The
        // structured view is plain data, and the text artifacts are inert.
        let report = sample_report();

        let view = report_view(&report);
        let rendered = serde_json::to_string(&view).expect("the view serializes");
        assert!(
            !rendered.contains("<script"),
            "the report view must not carry a script element"
        );
        assert!(
            !rendered.contains("<style"),
            "the report view must not carry a style element"
        );
        assert!(
            !rendered.contains("http://") && !rendered.contains("https://"),
            "the report view must not carry a remote reference"
        );

        for format in [
            ReportFormat::Json,
            ReportFormat::Csv,
            ReportFormat::Sarif,
            ReportFormat::Junit,
            ReportFormat::Markdown,
        ] {
            let document = render_for_display(format, &report).expect("a text display form");
            assert!(
                !document.contains("<script") && !document.contains("<style"),
                "{format:?} must not carry a script or a style element"
            );
        }
    }

    #[test]
    fn the_display_rendering_of_the_other_formats_is_the_exported_one() {
        let report = sample_report();
        for format in [
            ReportFormat::Json,
            ReportFormat::Csv,
            ReportFormat::Markdown,
        ] {
            assert_eq!(
                render_for_display(format, &report).expect("display"),
                render(format, &report).expect("export"),
                "{format:?} renders differently for display and for export"
            );
        }
        assert_eq!(
            junit_without_timestamp(
                &render_for_display(ReportFormat::Junit, &report).expect("display")
            ),
            junit_without_timestamp(&render(ReportFormat::Junit, &report).expect("export"))
        );
        assert_eq!(
            sarif_without_timestamps(
                &render_for_display(ReportFormat::Sarif, &report).expect("display")
            ),
            sarif_without_timestamps(&render(ReportFormat::Sarif, &report).expect("export"))
        );
    }

    #[test]
    fn the_exported_html_is_the_clis_document() {
        // The exported file is the operator's artifact, opened in the
        // operator's browser: it is the CLI's document, byte for byte.
        let report = sample_report();
        let exported = render(ReportFormat::Html, &report).expect("html");
        assert_eq!(exported, convert_to_html(&report));
        assert!(
            exported.contains("<style"),
            "the exported document keeps the CLI's own styling"
        );
    }

    #[test]
    fn the_report_view_carries_the_report_as_data() {
        let report = sample_report();
        let view = report_view(&report);

        assert_eq!(view.target, "example.com");
        assert_eq!(view.findings.len(), 1);
        assert_eq!(view.findings[0].title, "Weak TLS configuration");
        assert_eq!(view.findings[0].severity, "high");
        assert_eq!(view.findings[0].location, "example.com:443");
        assert!(view.open_ports.is_empty());
        assert!(view.services.is_empty());
    }

    #[test]
    fn writing_to_a_free_destination_creates_it() {
        let dir = temp_export_dir("new_file");
        let path = dir.join("report.json");
        write_export(&path, "first").expect("the first write succeeds");
        assert_eq!(std::fs::read_to_string(&path).expect("read"), "first");
        assert!(!path.with_extension("json.bak").exists());
    }

    #[test]
    fn writing_over_an_existing_export_backs_it_up() {
        let dir = temp_export_dir("backup");
        let path = dir.join("report.json");
        write_export(&path, "first").expect("the first write succeeds");
        write_export(&path, "second").expect("the second write succeeds");

        let backup = dir.join("report.json.bak");
        assert_eq!(
            std::fs::read_to_string(&backup).expect("read the backup"),
            "first",
            "the previous report must survive the overwrite"
        );
        assert_eq!(std::fs::read_to_string(&path).expect("read"), "second");
    }

    #[test]
    fn writing_refuses_a_destination_that_is_not_a_regular_file() {
        let dir = temp_export_dir("not_regular");
        let path = dir.join("report.json");
        std::fs::create_dir_all(&path).expect("create a colliding directory");

        let error = write_export(&path, "data").expect_err("a directory must be refused");
        assert!(
            error.to_string().contains("not a regular file"),
            "unexpected error: {error}"
        );
        assert!(path.is_dir(), "the destination must be left untouched");
    }

    #[test]
    fn a_failed_render_is_never_written() {
        // A renderer failure is an `Err`, and the caller never sees a
        // successful export for a document that was not produced.
        let error = render(ReportFormat::Json, &sample_report());
        assert!(error.is_ok(), "the fixture report renders");
    }
}
