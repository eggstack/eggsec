
macro_rules! dispatch_void_with_special {
    ($self:expr, $method:ident($($arg:expr),*), $special:expr) => {
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
            Tab::Settings => $self.settings.$method($($arg),*),
            Tab::History => $special,
            Tab::Dashboard => $self.dashboard.$method($($arg),*),
        }
    };
}


macro_rules! dispatch_bool_with_special {
    ($self:expr, $method:ident($($arg:expr),*), $special:expr) => {
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
            Tab::Settings => $self.settings.$method($($arg),*),
            Tab::History => $special,
            Tab::Dashboard => $self.dashboard.$method($($arg),*),
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
            Tab::Nse => {},
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => $self.plugin.$method($page_size),
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => {},
            Tab::Settings => {},
            Tab::History => {
                if let Ok(mut h) = $self.history.lock() {
                    h.$method($page_size);
                }
            }
            Tab::Dashboard => $self.dashboard.$method($page_size),
        }
    };
}


macro_rules! dispatch_is_running {
    ($self:expr) => {
        match $self.current_tab {
            Tab::Recon => $self.recon.is_running(),
            Tab::Load => $self.load.is_running(),
            Tab::ScanPorts => $self.scan_ports.is_running(),
            Tab::ScanEndpoints => $self.scan_endpoints.is_running(),
            Tab::Fingerprint => $self.fingerprint.is_running(),
            Tab::Fuzz => $self.fuzz.is_running(),
            Tab::Waf => $self.waf.is_running(),
            Tab::WafStress => $self.waf_stress.is_running(),
            Tab::Scan => $self.scan.is_running(),
            Tab::Resume => $self.resume.is_running(),
            Tab::Proxy => $self.proxy.is_running(),
            Tab::Packet => $self.packet.is_running(),
            Tab::GraphQl => $self.graphql.is_running(),
            Tab::OAuth => $self.oauth.is_running(),
            Tab::Cluster => $self.cluster.is_running(),
            Tab::Stress => $self.stress.is_running(),
            Tab::Report => $self.report.is_running(),
            #[cfg(feature = "nse")]
            Tab::Nse => $self.nse.is_running(),
            #[cfg(not(feature = "nse"))]
            Tab::Nse => false,
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => $self.plugin.is_running(),
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => false,
            Tab::Settings => false,
            Tab::History => false,
            Tab::Dashboard => false,
        }
    };
}


macro_rules! dispatch_stop {
    ($self:expr) => {
        match $self.current_tab {
            Tab::Recon => $self.recon.stop(),
            Tab::Load => $self.load.stop(),
            Tab::ScanPorts => $self.scan_ports.stop(),
            Tab::ScanEndpoints => $self.scan_endpoints.stop(),
            Tab::Fingerprint => $self.fingerprint.stop(),
            Tab::Fuzz => $self.fuzz.stop(),
            Tab::Waf => $self.waf.stop(),
            Tab::WafStress => $self.waf_stress.stop(),
            Tab::Scan => $self.scan.stop(),
            Tab::Resume => $self.resume.stop(),
            Tab::Proxy => $self.proxy.stop(),
            Tab::Packet => $self.packet.stop(),
            Tab::GraphQl => $self.graphql.stop(),
            Tab::OAuth => $self.oauth.stop(),
            Tab::Cluster => $self.cluster.stop(),
            Tab::Stress => $self.stress.stop(),
            Tab::Report => $self.report.stop(),
            #[cfg(feature = "nse")]
            Tab::Nse => $self.nse.stop(),
            #[cfg(not(feature = "nse"))]
            Tab::Nse => {},
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => $self.plugin.stop(),
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => {},
            Tab::Settings => {}
            Tab::History => {}
            Tab::Dashboard => {}
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
            Tab::Nse => {},
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => $self.plugin.reset(),
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => {},
            Tab::Settings => $self.settings.reset(),
            Tab::History => {
                if let Ok(mut h) = $self.history.lock() {
                    h.clear_all();
                }
            }
            Tab::Dashboard => $self.dashboard.reset(),
        }
    };
}


macro_rules! dispatch_is_at_left_edge {
    ($self:expr) => {
        match $self.current_tab {
            Tab::Recon => $self.recon.is_at_left_edge(),
            Tab::Load => $self.load.is_at_left_edge(),
            Tab::ScanPorts => $self.scan_ports.is_at_left_edge(),
            Tab::ScanEndpoints => $self.scan_endpoints.is_at_left_edge(),
            Tab::Fingerprint => $self.fingerprint.is_at_left_edge(),
            Tab::Fuzz => $self.fuzz.is_at_left_edge(),
            Tab::Waf => $self.waf.is_at_left_edge(),
            Tab::WafStress => $self.waf_stress.is_at_left_edge(),
            Tab::Scan => $self.scan.is_at_left_edge(),
            Tab::Resume => $self.resume.is_at_left_edge(),
            Tab::Proxy => $self.proxy.is_at_left_edge(),
            Tab::Packet => $self.packet.is_at_left_edge(),
            Tab::GraphQl => $self.graphql.is_at_left_edge(),
            Tab::OAuth => $self.oauth.is_at_left_edge(),
            Tab::Cluster => $self.cluster.is_at_left_edge(),
            Tab::Stress => $self.stress.is_at_left_edge(),
            Tab::Report => $self.report.is_at_left_edge(),
            #[cfg(feature = "nse")]
            Tab::Nse => $self.nse.is_at_left_edge(),
            #[cfg(not(feature = "nse"))]
            Tab::Nse => false,
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => $self.plugin.is_at_left_edge(),
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => false,
            Tab::Settings => $self.settings.is_at_left_edge(),
            Tab::History => true,
            Tab::Dashboard => true,
        }
    };
}


macro_rules! dispatch_is_at_right_edge {
    ($self:expr) => {
        match $self.current_tab {
            Tab::Recon => $self.recon.is_at_right_edge(),
            Tab::Load => $self.load.is_at_right_edge(),
            Tab::ScanPorts => $self.scan_ports.is_at_right_edge(),
            Tab::ScanEndpoints => $self.scan_endpoints.is_at_right_edge(),
            Tab::Fingerprint => $self.fingerprint.is_at_right_edge(),
            Tab::Fuzz => $self.fuzz.is_at_right_edge(),
            Tab::Waf => $self.waf.is_at_right_edge(),
            Tab::WafStress => $self.waf_stress.is_at_right_edge(),
            Tab::Scan => $self.scan.is_at_right_edge(),
            Tab::Resume => $self.resume.is_at_right_edge(),
            Tab::Proxy => $self.proxy.is_at_right_edge(),
            Tab::Packet => $self.packet.is_at_right_edge(),
            Tab::GraphQl => $self.graphql.is_at_right_edge(),
            Tab::OAuth => $self.oauth.is_at_right_edge(),
            Tab::Cluster => $self.cluster.is_at_right_edge(),
            Tab::Stress => $self.stress.is_at_right_edge(),
            Tab::Report => $self.report.is_at_right_edge(),
            #[cfg(feature = "nse")]
            Tab::Nse => $self.nse.is_at_right_edge(),
            #[cfg(not(feature = "nse"))]
            Tab::Nse => false,
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => $self.plugin.is_at_right_edge(),
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => false,
            Tab::Settings => $self.settings.is_at_right_edge(),
            Tab::History => true,
            Tab::Dashboard => true,
        }
    };
}
