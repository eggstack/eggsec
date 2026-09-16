//! Stable serializable report/evidence data contracts for Eggsec.
//!
//! `eggsec-report-model` owns the pure data DTOs that domain crates exchange
//! (`eggsec-db-lab`, `eggsec-mobile-lab`, `eggsec-web-proxy`, `eggsec-nse`)
//! without paying for or coupling to the renderer crate. Rendering, format
//! conversion, filesystem loading, trend/baseline analysis, and presentation
//! behavior stay in `eggsec-output`, which depends on this crate (never the
//! reverse).
//!
//! ## Layering
//!
//! ```text
//! eggsec-report-model  <- data only (this crate)
//!         ^
//!         |
//! eggsec-output        <- rendering/conversion/analysis
//!         ^
//!         |
//! process hosts / engine
//! ```
//!
//! ## Allowed
//!
//! Serializable report/finding/evidence DTOs, policy/audit/baseline/diff
//! summary DTOs when data-only, and `Display`/builder helpers intrinsic to the
//! data type that do not pull presentation dependencies.
//!
//! ## Forbidden
//!
//! Filesystem I/O, JSON file loading, HTML/Markdown/CSV/JUnit/SARIF
//! generation, async I/O/Tokio, scheduling/cron/queues, terminal output,
//! hostname/environment discovery, LRU/cache behavior, engine policy
//! evaluation, network access, and domain execution.

pub mod envelope;
pub mod report;
pub mod summary;

pub use envelope::{
    BaselineSummary, EvidenceItem, EvidenceKind, EvidenceManifest, EvidenceSource, FindingRecord,
    RedactionPolicy, RedactionState, ReportEnvelope, ToolMetadata,
};
pub use report::{FindingData, PortData, ScanReportData, ServiceData, WirelessNetworkReportData};
pub use summary::{DiffSummary, PolicySummary};
