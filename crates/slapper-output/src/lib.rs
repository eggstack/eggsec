//! Output and report generation module
//!
//! Provides report generation, format conversion, trend analysis, and scan session management.
//!
//! ## Key Components
//!
//! - [`convert`] - Format conversion (CSV, HTML, JUnit, Markdown, SARIF)
//! - [`dedup`] - Finding deduplication engine
//! - [`trend`] - Trend analysis across multiple scans
//! - [`baseline`] - Baseline comparison for regression detection
//! - [`session`] - Scan session persistence
//! - [`schedule`] - Scheduled scan management
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
pub mod baseline;
pub mod convert;
pub mod csv;
pub mod dedup;
pub mod diff;
pub mod escape;
pub mod html;
pub mod junit;
pub mod markdown;
pub mod sarif;
pub mod schedule;
pub mod session;
pub mod trend;

pub use agent::AttackSurface;
pub use agent::Severity;
#[deprecated(since = "0.1.0", note = "Use Severity directly from this module")]
pub use agent::Severity as AgentSeverity;
pub use agent::{
    AgentFinding, Confidence, Evidence, FindingStatus, FindingSummary, Remediation,
    RemediationEffort,
};
pub use convert::{
    convert_to_csv, convert_to_html, convert_to_junit, convert_to_markdown, convert_to_sarif,
    load_scan_report, FindingData, PortData, ScanReportData, ServiceData,
    WirelessNetworkReportData,
};
pub use csv::{CsvExporter, EndpointCsv, FindingCsv, OutputFormat as ExportFormat, PortCsv};
pub use diff::DiffSummary;
pub use junit::{JUnitBuilder, JUnitReport, JUnitTestResult};
pub use sarif::{SarifBuilder, SarifReport};
pub use schedule::{CronExpression, CronScheduler, Priority, ScanOptions, ScanQueue, ScanType};
pub use session::{ScanSession, SessionInfo};
pub use trend::{
    ComparisonResult, Finding as TrendFinding, ResultComparator, ResultSummary, ScanResult,
    Severity as TrendSeverity, TrendAnalysis, TrendAnalyzer, TrendDirection,
};
