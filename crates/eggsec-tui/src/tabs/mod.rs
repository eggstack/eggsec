mod auth;
mod cluster;
#[cfg(feature = "compliance")]
pub mod compliance;
mod dashboard;
mod fingerprint;
mod fuzz;
pub mod graphql;
pub mod history;
#[cfg(feature = "advanced-hunting")]
pub mod hunt;
#[cfg(feature = "external-integrations")]
pub mod integrations;
mod load;
#[cfg(feature = "nse")]
pub mod nse;
pub mod oauth;
pub mod packet;
pub mod proxy;
pub mod recon;
mod report;
mod resume;
mod scan;
mod scan_endpoints;
mod scan_ports;
mod settings;
#[cfg(feature = "database")]
pub mod storage;
mod stress;
#[cfg(feature = "vuln-management")]
pub mod vuln;
mod waf;
mod waf_stress;
#[cfg(feature = "wireless")]
pub mod wireless;

mod spec;
pub(crate) use spec::{
    risk_from_group, spec_for, tab_specs, visible_tab_specs, TabCategory, TabRiskGroup, TabSpec,
};

pub use auth::AuthTab;
#[cfg(feature = "headless-browser")]
pub use browser::BrowserTab;
pub use cluster::ClusterTab;
#[cfg(feature = "compliance")]
pub use compliance::ComplianceTab;
pub use dashboard::DashboardTab;
pub use fingerprint::FingerprintTab;
pub use fuzz::FuzzTab;
pub use graphql::GraphQlTab;
pub use history::HistoryTab;
#[cfg(feature = "advanced-hunting")]
pub use hunt::HuntTab;
#[cfg(feature = "external-integrations")]
pub use integrations::IntegrationsTab;
pub use load::LoadTab;
#[cfg(feature = "nse")]
pub use nse::NseTab;
pub use oauth::OAuthTab;
pub use packet::PacketTab;
pub use proxy::ProxyTab;
pub use recon::ReconTab;
pub use report::ReportTab;
pub use resume::ResumeTab;
pub use scan::{ScanTab, StageStatus};
pub use scan_endpoints::ScanEndpointsTab;
pub use scan_ports::ScanPortsTab;
pub use settings::SettingsTab;
#[cfg(feature = "database")]
pub use storage::StorageTab;
pub use stress::StressTab;
#[cfg(feature = "vuln-management")]
pub use vuln::VulnTab;
pub use waf::WafTab;
pub use waf_stress::WafStressTab;
#[cfg(feature = "wireless")]
pub use wireless::WirelessTab;