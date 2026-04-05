macro_rules! dispatch {
    ($self:expr, $method:ident($($arg:expr),*), $special:expr, $default:expr) => {
        match $self.current_tab {
            Tab::Recon => $self.recon.$method($($arg),*),
            Tab::Load => $self.load.$method($($arg),*),
            Tab::ScanPorts => $self.scan_ports.$method($($arg),*),
            Tab::ScanEndpoints => $self.scan_endpoints.$method($($arg),*),
            Tab::Fingerprint => $self.fingerprint.$method($($arg),*),
            Tab::Fuzz => $self.fuzz.$method($($arg),*),
            Tab::Waf => $self.waf.$method($($arg),*),
            Tab::WafStress => $self.waf_stress.$method($($arg),*),
            Tab::Scan => $self.scan.$method($($arg),*),
            Tab::Resume => $self.resume.$method($($arg),*),
            Tab::Proxy => $self.proxy.$method($($arg),*),
            Tab::Packet => $self.packet.$method($($arg),*),
            Tab::GraphQl => $self.graphql.$method($($arg),*),
            Tab::OAuth => $self.oauth.$method($($arg),*),
            Tab::Cluster => $self.cluster.$method($($arg),*),
            Tab::Stress => $self.stress.$method($($arg),*),
            Tab::Report => $self.report.$method($($arg),*),
            #[cfg(feature = "nse")]
            Tab::Nse => $self.nse.$method($($arg),*),
            #[cfg(not(feature = "nse"))]
            Tab::Nse => $default,
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => $self.plugin.$method($($arg),*),
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => $default,
            Tab::Settings => $self.settings.$method($($arg),*),
            Tab::History => $special,
            Tab::Dashboard => $self.dashboard.$method($($arg),*),
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => $self.hunt.$method($($arg),*),
            #[cfg(not(feature = "advanced-hunting"))]
            Tab::Hunt => $default,
            #[cfg(feature = "headless-browser")]
            Tab::Browser => $self.browser.$method($($arg),*),
            #[cfg(not(feature = "headless-browser"))]
            Tab::Browser => $default,
            #[cfg(feature = "compliance")]
            Tab::Compliance => $self.compliance.$method($($arg),*),
            #[cfg(not(feature = "compliance"))]
            Tab::Compliance => $default,
            #[cfg(feature = "database")]
            Tab::Storage => $self.storage.$method($($arg),*),
            #[cfg(not(feature = "database"))]
            Tab::Storage => $default,
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => $self.integrations.$method($($arg),*),
            #[cfg(not(feature = "external-integrations"))]
            Tab::Integrations => $default,
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => $self.workflow.$method($($arg),*),
            #[cfg(not(feature = "finding-workflow"))]
            Tab::Workflow => $default,
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => $self.vuln.$method($($arg),*),
            #[cfg(not(feature = "vuln-management"))]
            Tab::Vuln => $default,
        }
    };
}

macro_rules! dispatch_void {
    ($self:expr, $method:ident($($arg:expr),*)) => {
        match $self.current_tab {
            Tab::Recon => $self.recon.$method($($arg),*),
            Tab::Load => $self.load.$method($($arg),*),
            Tab::ScanPorts => $self.scan_ports.$method($($arg),*),
            Tab::ScanEndpoints => $self.scan_endpoints.$method($($arg),*),
            Tab::Fingerprint => $self.fingerprint.$method($($arg),*),
            Tab::Fuzz => $self.fuzz.$method($($arg),*),
            Tab::Waf => $self.waf.$method($($arg),*),
            Tab::WafStress => $self.waf_stress.$method($($arg),*),
            Tab::Scan => $self.scan.$method($($arg),*),
            Tab::Resume => $self.resume.$method($($arg),*),
            Tab::Proxy => $self.proxy.$method($($arg),*),
            Tab::Packet => $self.packet.$method($($arg),*),
            Tab::GraphQl => $self.graphql.$method($($arg),*),
            Tab::OAuth => $self.oauth.$method($($arg),*),
            Tab::Cluster => $self.cluster.$method($($arg),*),
            Tab::Stress => $self.stress.$method($($arg),*),
            Tab::Report => $self.report.$method($($arg),*),
            #[cfg(feature = "nse")]
            Tab::Nse => $self.nse.$method($($arg),*),
            #[cfg(not(feature = "nse"))]
            Tab::Nse => {},
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => $self.plugin.$method($($arg),*),
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => {},
            Tab::Settings => {},
            Tab::History => {},
            Tab::Dashboard => {},
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => $self.hunt.$method($($arg),*),
            #[cfg(not(feature = "advanced-hunting"))]
            Tab::Hunt => {},
            #[cfg(feature = "headless-browser")]
            Tab::Browser => $self.browser.$method($($arg),*),
            #[cfg(not(feature = "headless-browser"))]
            Tab::Browser => {},
            #[cfg(feature = "compliance")]
            Tab::Compliance => $self.compliance.$method($($arg),*),
            #[cfg(not(feature = "compliance"))]
            Tab::Compliance => {},
            #[cfg(feature = "database")]
            Tab::Storage => $self.storage.$method($($arg),*),
            #[cfg(not(feature = "database"))]
            Tab::Storage => {},
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => $self.integrations.$method($($arg),*),
            #[cfg(not(feature = "external-integrations"))]
            Tab::Integrations => {},
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => $self.workflow.$method($($arg),*),
            #[cfg(not(feature = "finding-workflow"))]
            Tab::Workflow => {},
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => $self.vuln.$method($($arg),*),
            #[cfg(not(feature = "vuln-management"))]
            Tab::Vuln => {},
        }
    };
}

macro_rules! dispatch_bool {
    ($self:expr, $method:ident($($arg:expr),*)) => {
        match $self.current_tab {
            Tab::Recon => $self.recon.$method($($arg),*),
            Tab::Load => $self.load.$method($($arg),*),
            Tab::ScanPorts => $self.scan_ports.$method($($arg),*),
            Tab::ScanEndpoints => $self.scan_endpoints.$method($($arg),*),
            Tab::Fingerprint => $self.fingerprint.$method($($arg),*),
            Tab::Fuzz => $self.fuzz.$method($($arg),*),
            Tab::Waf => $self.waf.$method($($arg),*),
            Tab::WafStress => $self.waf_stress.$method($($arg),*),
            Tab::Scan => $self.scan.$method($($arg),*),
            Tab::Resume => $self.resume.$method($($arg),*),
            Tab::Proxy => $self.proxy.$method($($arg),*),
            Tab::Packet => $self.packet.$method($($arg),*),
            Tab::GraphQl => $self.graphql.$method($($arg),*),
            Tab::OAuth => $self.oauth.$method($($arg),*),
            Tab::Cluster => $self.cluster.$method($($arg),*),
            Tab::Stress => $self.stress.$method($($arg),*),
            Tab::Report => $self.report.$method($($arg),*),
            #[cfg(feature = "nse")]
            Tab::Nse => $self.nse.$method($($arg),*),
            #[cfg(not(feature = "nse"))]
            Tab::Nse => false,
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => $self.plugin.$method($($arg),*),
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => false,
            Tab::Settings => false,
            Tab::History => false,
            Tab::Dashboard => false,
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => $self.hunt.$method($($arg),*),
            #[cfg(not(feature = "advanced-hunting"))]
            Tab::Hunt => false,
            #[cfg(feature = "headless-browser")]
            Tab::Browser => $self.browser.$method($($arg),*),
            #[cfg(not(feature = "headless-browser"))]
            Tab::Browser => false,
            #[cfg(feature = "compliance")]
            Tab::Compliance => $self.compliance.$method($($arg),*),
            #[cfg(not(feature = "compliance"))]
            Tab::Compliance => false,
            #[cfg(feature = "database")]
            Tab::Storage => $self.storage.$method($($arg),*),
            #[cfg(not(feature = "database"))]
            Tab::Storage => false,
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => $self.integrations.$method($($arg),*),
            #[cfg(not(feature = "external-integrations"))]
            Tab::Integrations => false,
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => $self.workflow.$method($($arg),*),
            #[cfg(not(feature = "finding-workflow"))]
            Tab::Workflow => false,
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => $self.vuln.$method($($arg),*),
            #[cfg(not(feature = "vuln-management"))]
            Tab::Vuln => false,
        }
    };
}

macro_rules! dispatch_page {
    ($self:expr, $method:ident, $page_size:expr) => {
        match $self.current_tab {
            Tab::Recon => $self.recon.$method($page_size),
            Tab::Load => $self.load.$method($page_size),
            Tab::ScanPorts => $self.scan_ports.$method($page_size),
            Tab::ScanEndpoints => $self.scan_endpoints.$method($page_size),
            Tab::Fingerprint => $self.fingerprint.$method($page_size),
            Tab::Fuzz => $self.fuzz.$method($page_size),
            Tab::Waf => $self.waf.$method($page_size),
            Tab::WafStress => $self.waf_stress.$method($page_size),
            Tab::Scan => $self.scan.$method($page_size),
            Tab::Resume => $self.resume.$method($page_size),
            Tab::Proxy => $self.proxy.$method($page_size),
            Tab::Packet => $self.packet.$method($page_size),
            Tab::GraphQl => $self.graphql.$method($page_size),
            Tab::OAuth => $self.oauth.$method($page_size),
            Tab::Cluster => $self.cluster.$method($page_size),
            Tab::Stress => $self.stress.$method($page_size),
            Tab::Report => $self.report.$method($page_size),
            #[cfg(feature = "nse")]
            Tab::Nse => $self.nse.$method($page_size),
            #[cfg(not(feature = "nse"))]
            Tab::Nse => {}
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => $self.plugin.$method($page_size),
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => {}
            Tab::Settings => {}
            Tab::History => {
                if let Ok(mut h) = $self.history.lock() {
                    h.$method($page_size);
                }
            }
            Tab::Dashboard => $self.dashboard.$method($page_size),
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => $self.hunt.$method($page_size),
            #[cfg(not(feature = "advanced-hunting"))]
            Tab::Hunt => {}
            #[cfg(feature = "headless-browser")]
            Tab::Browser => $self.browser.$method($page_size),
            #[cfg(not(feature = "headless-browser"))]
            Tab::Browser => {}
            #[cfg(feature = "compliance")]
            Tab::Compliance => $self.compliance.$method($page_size),
            #[cfg(not(feature = "compliance"))]
            Tab::Compliance => {}
            #[cfg(feature = "database")]
            Tab::Storage => $self.storage.$method($page_size),
            #[cfg(not(feature = "database"))]
            Tab::Storage => {}
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => $self.integrations.$method($page_size),
            #[cfg(not(feature = "external-integrations"))]
            Tab::Integrations => {}
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => $self.workflow.$method($page_size),
            #[cfg(not(feature = "finding-workflow"))]
            Tab::Workflow => {}
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => $self.vuln.$method($page_size),
            #[cfg(not(feature = "vuln-management"))]
            Tab::Vuln => {}
        }
    };
}

macro_rules! dispatch_is_at_edge {
    ($self:expr, $method:ident, $default:expr) => {
        match $self.current_tab {
            Tab::Recon => $self.recon.$method(),
            Tab::Load => $self.load.$method(),
            Tab::ScanPorts => $self.scan_ports.$method(),
            Tab::ScanEndpoints => $self.scan_endpoints.$method(),
            Tab::Fingerprint => $self.fingerprint.$method(),
            Tab::Fuzz => $self.fuzz.$method(),
            Tab::Waf => $self.waf.$method(),
            Tab::WafStress => $self.waf_stress.$method(),
            Tab::Scan => $self.scan.$method(),
            Tab::Resume => $self.resume.$method(),
            Tab::Proxy => $self.proxy.$method(),
            Tab::Packet => $self.packet.$method(),
            Tab::GraphQl => $self.graphql.$method(),
            Tab::OAuth => $self.oauth.$method(),
            Tab::Cluster => $self.cluster.$method(),
            Tab::Stress => $self.stress.$method(),
            Tab::Report => $self.report.$method(),
            #[cfg(feature = "nse")]
            Tab::Nse => $self.nse.$method(),
            #[cfg(not(feature = "nse"))]
            Tab::Nse => $default,
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => $self.plugin.$method(),
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => $default,
            Tab::Settings => $default,
            Tab::History => true,
            Tab::Dashboard => true,
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => $self.hunt.$method(),
            #[cfg(not(feature = "advanced-hunting"))]
            Tab::Hunt => $default,
            #[cfg(feature = "headless-browser")]
            Tab::Browser => $self.browser.$method(),
            #[cfg(not(feature = "headless-browser"))]
            Tab::Browser => $default,
            #[cfg(feature = "compliance")]
            Tab::Compliance => $self.compliance.$method(),
            #[cfg(not(feature = "compliance"))]
            Tab::Compliance => $default,
            #[cfg(feature = "database")]
            Tab::Storage => $self.storage.$method(),
            #[cfg(not(feature = "database"))]
            Tab::Storage => $default,
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => $self.integrations.$method(),
            #[cfg(not(feature = "external-integrations"))]
            Tab::Integrations => $default,
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => $self.workflow.$method(),
            #[cfg(not(feature = "finding-workflow"))]
            Tab::Workflow => $default,
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => $self.vuln.$method(),
            #[cfg(not(feature = "vuln-management"))]
            Tab::Vuln => $default,
        }
    };
}

macro_rules! dispatch_reset {
    ($self:expr) => {
        match $self.current_tab {
            Tab::Recon => $self.recon.reset(),
            Tab::Load => $self.load.reset(),
            Tab::ScanPorts => $self.scan_ports.reset(),
            Tab::ScanEndpoints => $self.scan_endpoints.reset(),
            Tab::Fingerprint => $self.fingerprint.reset(),
            Tab::Fuzz => $self.fuzz.reset(),
            Tab::Waf => $self.waf.reset(),
            Tab::WafStress => $self.waf_stress.reset(),
            Tab::Scan => $self.scan.reset(),
            Tab::Resume => $self.resume.reset(),
            Tab::Proxy => $self.proxy.reset(),
            Tab::Packet => $self.packet.reset(),
            Tab::GraphQl => $self.graphql.reset(),
            Tab::OAuth => $self.oauth.reset(),
            Tab::Cluster => $self.cluster.reset(),
            Tab::Stress => $self.stress.reset(),
            Tab::Report => $self.report.reset(),
            #[cfg(feature = "nse")]
            Tab::Nse => $self.nse.reset(),
            #[cfg(not(feature = "nse"))]
            Tab::Nse => {}
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => $self.plugin.reset(),
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => {}
            Tab::Settings => $self.settings.reset(),
            Tab::History => {
                if let Ok(mut h) = $self.history.lock() {
                    h.clear_all();
                }
            }
            Tab::Dashboard => $self.dashboard.reset(),
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => $self.hunt.reset(),
            #[cfg(not(feature = "advanced-hunting"))]
            Tab::Hunt => {}
            #[cfg(feature = "headless-browser")]
            Tab::Browser => $self.browser.reset(),
            #[cfg(not(feature = "headless-browser"))]
            Tab::Browser => {}
            #[cfg(feature = "compliance")]
            Tab::Compliance => $self.compliance.reset(),
            #[cfg(not(feature = "compliance"))]
            Tab::Compliance => {}
            #[cfg(feature = "database")]
            Tab::Storage => $self.storage.reset(),
            #[cfg(not(feature = "database"))]
            Tab::Storage => {}
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => $self.integrations.reset(),
            #[cfg(not(feature = "external-integrations"))]
            Tab::Integrations => {}
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => $self.workflow.reset(),
            #[cfg(not(feature = "finding-workflow"))]
            Tab::Workflow => {}
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => $self.vuln.reset(),
            #[cfg(not(feature = "vuln-management"))]
            Tab::Vuln => {}
        }
    };
}
