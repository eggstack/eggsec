//! Vulnerability prioritization module
//!
//! Provides CVSS scoring, exploitability assessment, asset criticality, and risk prioritization.
//!
//! ## Modules
//!
//! - [`cvss`] - CVSS 3.1 score calculation
//! - [`exploit`] - Exploitability assessment
//! - [`asset`] - Asset criticality scoring
//! - [`prioritizer`] - Combined risk prioritization
//! - [`triage`] - Finding triage
//! - [`remediation`] - Remediation guidance

pub mod asset;
pub mod cvss;
pub mod exploit;
pub mod prioritizer;
pub mod remediation;
pub mod triage;

pub use asset::AssetCriticality;
pub use cvss::CvssScore;
pub use exploit::ExploitInfo;
pub use prioritizer::{PrioritizedFinding, PriorityLevel, RiskScore};
pub use remediation::Remediation;
pub use triage::{TriageResult, TriageStatus};
