pub mod agent;
pub mod convert;
pub mod csv;
pub mod html;
pub mod junit;
pub mod markdown;
pub mod report;
pub mod sarif;
pub mod schedule;
pub mod session;
pub mod trend;

pub use agent::AttackSurface;
pub use agent::Severity as AgentSeverity;
pub use agent::{
    AgentFinding, Confidence, Evidence, FindingStatus, FindingSummary, Remediation,
    RemediationEffort,
};
pub use convert::{
    convert_to_csv, convert_to_html, convert_to_junit, convert_to_markdown, convert_to_sarif,
    load_scan_report, FindingData, PortData, ScanReportData, ServiceData,
};
pub use csv::{CsvExporter, EndpointCsv, ExportFormat, FindingCsv, PortCsv};
pub use junit::{JUnitBuilder, JUnitReport, JUnitTestResult};
pub use report::{Report, ReportMetadata, ReportTemplate, SeverityCounts};
pub use sarif::{SarifBuilder, SarifReport};
pub use schedule::{Priority, ScanOptions, ScanQueue, ScanType};
pub use session::{ScanSession, SessionInfo};
pub use trend::{
    ComparisonResult, Finding as TrendFinding, ResultComparator, ResultSummary, ScanResult,
    Severity as TrendSeverity, TrendAnalysis, TrendAnalyzer, TrendDirection,
};

#[allow(deprecated)]
pub use markdown::Finding as DeprecatedMarkdownFinding;
#[allow(deprecated)]
pub use trend::Finding as DeprecatedTrendFinding;
