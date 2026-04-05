pub mod agent;
pub mod ai_schema;
#[cfg(feature = "advanced-hunting")]
pub mod attack_graph;
pub mod baseline;
pub mod convert;
pub mod csv;
pub mod dedup;
pub mod diff;
pub mod escape;
pub mod html;
pub mod junit;
pub mod markdown;
pub mod pdf;
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
#[cfg(feature = "advanced-hunting")]
pub use attack_graph::{
    AttackGraph, AttackGraphBuilder, EdgeType, GraphCluster, GraphEdge, GraphNode, NodeType,
};
pub use convert::{
    convert_to_csv, convert_to_html, convert_to_junit, convert_to_markdown, convert_to_sarif,
    load_scan_report, FindingData, PortData, ScanReportData, ServiceData,
};
pub use csv::{CsvExporter, EndpointCsv, FindingCsv, OutputFormat as ExportFormat, PortCsv};
pub use diff::{DiffEngine, DiffFinding, DiffResult, DiffSummary};
pub use junit::{JUnitBuilder, JUnitReport, JUnitTestResult};
pub use pdf::{PdfConfig, PdfGenerator};
pub use report::{Report, ReportMetadata, ReportTemplate, SeverityCounts};
pub use sarif::{SarifBuilder, SarifReport};
pub use schedule::{CronExpression, CronScheduler, Priority, ScanOptions, ScanQueue, ScanType};
pub use session::{ScanSession, SessionInfo};
pub use trend::{
    ComparisonResult, Finding as TrendFinding, ResultComparator, ResultSummary, ScanResult,
    Severity as TrendSeverity, TrendAnalysis, TrendAnalyzer, TrendDirection,
};

pub use markdown::Finding as DeprecatedMarkdownFinding;
pub use trend::Finding as DeprecatedTrendFinding;
