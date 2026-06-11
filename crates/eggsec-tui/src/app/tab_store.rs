use crate::tabs;

pub struct TabStore {
    pub recon: tabs::ReconTab,
    pub load: tabs::LoadTab,
    pub scan_ports: tabs::ScanPortsTab,
    pub scan_endpoints: tabs::ScanEndpointsTab,
    pub fingerprint: tabs::FingerprintTab,
    pub fuzz: tabs::FuzzTab,
    pub waf: tabs::WafTab,
    pub waf_stress: tabs::WafStressTab,
    pub scan: tabs::ScanTab,
    pub resume: tabs::ResumeTab,
    pub proxy: tabs::ProxyTab,
    pub packet: tabs::PacketTab,
    pub graphql: tabs::GraphQlTab,
    pub oauth: tabs::OAuthTab,
    pub cluster: tabs::ClusterTab,
    pub stress: tabs::StressTab,
    pub report: tabs::ReportTab,
    pub settings: tabs::SettingsTab,
    pub dashboard: tabs::DashboardTab,
    pub auth: tabs::AuthTab,
    #[cfg(feature = "nse")]
    pub nse: tabs::NseTab,
    #[cfg(feature = "advanced-hunting")]
    pub hunt: tabs::HuntTab,
    #[cfg(feature = "headless-browser")]
    pub browser: tabs::BrowserTab,
    #[cfg(feature = "compliance")]
    pub compliance: tabs::ComplianceTab,
    #[cfg(feature = "database")]
    pub storage: tabs::StorageTab,
    #[cfg(feature = "external-integrations")]
    pub integrations: tabs::IntegrationsTab,
    #[cfg(feature = "finding-workflow")]
    pub workflow: tabs::WorkflowTab,
    #[cfg(feature = "vuln-management")]
    pub vuln: tabs::VulnTab,
    #[cfg(feature = "wireless")]
    pub wireless: tabs::WirelessTab,
}

impl TabStore {
    pub fn new() -> Self {
        Self {
            recon: tabs::ReconTab::new(),
            load: tabs::LoadTab::new(),
            scan_ports: tabs::ScanPortsTab::new(),
            scan_endpoints: tabs::ScanEndpointsTab::new(),
            fingerprint: tabs::FingerprintTab::new(),
            fuzz: tabs::FuzzTab::new(),
            waf: tabs::WafTab::new(),
            waf_stress: tabs::WafStressTab::new(),
            scan: tabs::ScanTab::new(),
            resume: tabs::ResumeTab::new(),
            proxy: tabs::ProxyTab::new(),
            packet: tabs::PacketTab::new(),
            graphql: tabs::GraphQlTab::new(),
            oauth: tabs::OAuthTab::new(),
            cluster: tabs::ClusterTab::new(),
            stress: tabs::StressTab::new(),
            report: tabs::ReportTab::new(),
            settings: tabs::SettingsTab::new(),
            dashboard: tabs::DashboardTab::new(),
            auth: tabs::AuthTab::new(),
            #[cfg(feature = "nse")]
            nse: tabs::NseTab::new(),
            #[cfg(feature = "advanced-hunting")]
            hunt: tabs::HuntTab::new(),
            #[cfg(feature = "headless-browser")]
            browser: tabs::BrowserTab::new(),
            #[cfg(feature = "compliance")]
            compliance: tabs::ComplianceTab::new(),
            #[cfg(feature = "database")]
            storage: tabs::StorageTab::new(),
            #[cfg(feature = "external-integrations")]
            integrations: tabs::IntegrationsTab::new(),
            #[cfg(feature = "finding-workflow")]
            workflow: tabs::WorkflowTab::new(),
            #[cfg(feature = "vuln-management")]
            vuln: tabs::VulnTab::new(),
            #[cfg(feature = "wireless")]
            wireless: tabs::WirelessTab::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_store_new_initializes_all_tabs() {
        let store = TabStore::new();
        let _ = &store.recon;
        let _ = &store.load;
        let _ = &store.settings;
        let _ = &store.dashboard;
        let _ = &store.fuzz;
    }
}
