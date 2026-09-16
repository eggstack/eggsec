//! Output and report generation module
//!
//! Provides report generation, format conversion, trend analysis, and finding
//! management. Report/evidence data contracts live in `eggsec-report-model`
//! (Phase B); this crate owns rendering, conversion, and analysis over them.
//!
//! Ownership note (Phase A): scan scheduling/cron/queue behavior lives in
//! `eggsec-agent` (`cron` + generic `TaskScheduler`); autonomous-agent
//! scheduling is the only durable cron consumer. Legacy tab-state session
//! persistence (`ScanSession`) was removed as superseded by daemon/runtime
//! durable sessions plus frontend `AppState` — it had zero production
//! consumers.
//!
//! Ownership note (Phase B): `convert::{ScanReportData, FindingData,
//! PortData, ServiceData, WirelessNetworkReportData}`, the `envelope` report
//! envelope, `PolicySummary`, and `DiffSummary` are re-exports of
//! `eggsec-report-model`. Domain crates depend on the model directly; only
//! rendering/conversion/analysis behavior lives here.
//!
//! ## Key Components
//!
//! - [`convert`] - Format conversion (CSV, HTML, JUnit, Markdown, SARIF)
//! - [`dedup`] - Finding deduplication engine
//! - [`trend`] - Trend analysis across multiple scans
//! - [`baseline`] - Baseline comparison for regression detection
//! - [`ai_schema`] - AI-compatible output schema
//!
//! ## Supported Output Formats
//!
//! | Format | Module | Description |
//! |--------|--------|-------------|
//! | JSON | [`convert`] | Pretty-printed and compact JSON |
//! | CSV | [`csv`] | Tabular data export |
//! | HTML | [`html`] | Styled HTML reports |
//! | Markdown | [`markdown`] | Markdown-formatted reports |
//! | SARIF | [`sarif`] | Static Analysis Results Format |
//! | JUnit | [`junit`] | JUnit XML for CI/CD integration |

pub mod agent;
pub mod ai_schema;
pub mod audit_summary;
pub mod baseline;
pub mod convert;
pub mod csv;
pub mod dedup;
pub mod diff;
pub mod envelope;
pub mod escape;
pub mod html;
pub mod junit;
pub mod markdown;
pub mod policy_summary;
pub mod sarif;
pub mod trend;

pub use agent::AttackSurface;
pub use agent::Severity;
pub use agent::{
    AgentFinding, Confidence, Evidence, FindingStatus, FindingSummary, Remediation,
    RemediationEffort,
};
pub use audit_summary::AuditSummary;
pub use convert::{
    convert_to_csv, convert_to_html, convert_to_junit, convert_to_markdown, convert_to_sarif,
    load_scan_report, FindingData, PortData, ScanReportData, ServiceData,
    WirelessNetworkReportData,
};
pub use csv::{CsvExporter, EndpointCsv, FindingCsv, OutputFormat as ExportFormat, PortCsv};
pub use diff::DiffSummary;
pub use envelope::{
    BaselineSummary, EvidenceItem, EvidenceKind, EvidenceManifest, EvidenceSource, FindingRecord,
    RedactionState, ReportEnvelope, ToolMetadata,
};
pub use junit::{JUnitBuilder, JUnitReport, JUnitTestResult};
pub use policy_summary::PolicySummary;
pub use sarif::{SarifBuilder, SarifReport};
pub use trend::{
    ComparisonResult, Finding as TrendFinding, ResultComparator, ResultSummary, ScanResult,
    Severity as TrendSeverity, TrendAnalysis, TrendAnalyzer, TrendDirection,
};
