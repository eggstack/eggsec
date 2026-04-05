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

#[allow(unused_imports)]
pub use asset::AssetCriticality;
#[allow(unused_imports)]
pub use cvss::CvssScore;
#[allow(unused_imports)]
pub use exploit::ExploitInfo;
#[allow(unused_imports)]
pub use prioritizer::{PrioritizedFinding, PriorityLevel, RiskScore};
#[allow(unused_imports)]
pub use remediation::Remediation;
#[allow(unused_imports)]
pub use triage::{TriageResult, TriageStatus};
