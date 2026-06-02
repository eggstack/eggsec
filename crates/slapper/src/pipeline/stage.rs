use serde::{Deserialize, Serialize};

use crate::cli::ScanProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stage {
    PortScan,
    Fingerprint,
    EndpointScan,
    Fuzz,
    LoadTest,
    Waf,
    Recon,
    Vuln,
}

impl std::fmt::Display for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Stage::PortScan => write!(f, "Port Scan"),
            Stage::Fingerprint => write!(f, "Fingerprint"),
            Stage::EndpointScan => write!(f, "Endpoint Scan"),
            Stage::Fuzz => write!(f, "Fuzzing"),
            Stage::LoadTest => write!(f, "Load Test"),
            Stage::Waf => write!(f, "WAF Test"),
            Stage::Recon => write!(f, "Recon"),
            Stage::Vuln => write!(f, "Vulnerability Assessment"),
        }
    }
}

impl Stage {
    pub fn from_profile(profile: ScanProfile) -> Vec<Self> {
        match profile {
            ScanProfile::Quick => vec![Stage::PortScan, Stage::Fingerprint],
            ScanProfile::Endpoint => vec![Stage::PortScan, Stage::Fingerprint, Stage::EndpointScan],
            ScanProfile::Web => vec![
                Stage::PortScan,
                Stage::Fingerprint,
                Stage::EndpointScan,
                Stage::Fuzz,
            ],
            ScanProfile::Waf => vec![
                Stage::PortScan,
                Stage::Fingerprint,
                Stage::EndpointScan,
                Stage::Waf,
            ],
            ScanProfile::Full => vec![
                Stage::PortScan,
                Stage::Fingerprint,
                Stage::EndpointScan,
                Stage::Fuzz,
                Stage::LoadTest,
            ],
            ScanProfile::Api => vec![
                Stage::PortScan,
                Stage::Fingerprint,
                Stage::EndpointScan,
                Stage::Fuzz,
            ],
            ScanProfile::Recon => vec![
                Stage::PortScan,
                Stage::Fingerprint,
                Stage::EndpointScan,
                Stage::Recon,
                Stage::Fuzz,
            ],
            ScanProfile::Stealth => vec![
                Stage::PortScan,
                Stage::Fingerprint,
                Stage::EndpointScan,
                Stage::Fuzz,
            ],
            ScanProfile::Deep => vec![
                Stage::PortScan,
                Stage::Fingerprint,
                Stage::EndpointScan,
                Stage::Fuzz,
            ],
            ScanProfile::Vuln => vec![
                Stage::PortScan,
                Stage::Fingerprint,
                Stage::Vuln,
                Stage::EndpointScan,
                Stage::Recon,
                Stage::Fuzz,
            ],
            ScanProfile::Auth => vec![
                Stage::PortScan,
                Stage::Fingerprint,
                Stage::EndpointScan,
                Stage::Fuzz,
            ],
            ScanProfile::DefenseLab => vec![
                Stage::PortScan,
                Stage::Fingerprint,
                Stage::EndpointScan,
                Stage::Waf,
                Stage::Fuzz,
            ],
            ScanProfile::SynvoidLocal => vec![
                Stage::PortScan,
                Stage::Fingerprint,
                Stage::EndpointScan,
                Stage::Waf,
            ],
            ScanProfile::WafRegression => vec![Stage::PortScan, Stage::Fingerprint, Stage::Waf],
            ScanProfile::ProtocolEdge => vec![Stage::PortScan, Stage::Fingerprint],
            ScanProfile::NseSafe => vec![Stage::PortScan, Stage::Fingerprint, Stage::EndpointScan],
        }
    }

    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "port" | "portscan" | "port-scan" => Some(Stage::PortScan),
            "fingerprint" | "fp" => Some(Stage::Fingerprint),
            "endpoint" | "endpoints" | "endpoint-scan" => Some(Stage::EndpointScan),
            "fuzz" | "fuzzer" | "fuzzing" => Some(Stage::Fuzz),
            "load" | "loadtest" | "load-test" => Some(Stage::LoadTest),
            "graphql" => Some(Stage::Fuzz),
            "oauth" => Some(Stage::Fuzz),
            "jwt" => Some(Stage::Fuzz),
            "waf" => Some(Stage::Waf),
            "recon" => Some(Stage::Recon),
            "vuln" | "vulnerability" | "vuln-assess" => Some(Stage::Vuln),
            _ => None,
        }
    }
}

pub fn parse_stages(s: &str) -> Vec<Stage> {
    s.split(',')
        .filter_map(|part| Stage::from_string(part.trim()))
        .collect()
}

pub const DEFAULT_SCAN_PORTS: &str = "80,443";

pub const EXTENDED_SCAN_PORTS: &str = "21,22,23,25,53,80,110,143,443,445,993,995,1433,1521,3306,3389,5432,5900,6379,8080,8443,27017,9092,9200,5672,2181,2375,2376,6443,10250,3000,5000,8000,9000,4200,5601,9090";

pub fn profile_from_str(s: &str) -> Option<crate::cli::ScanProfile> {
    match s.to_lowercase().as_str() {
        "quick" => Some(crate::cli::ScanProfile::Quick),
        "endpoint" => Some(crate::cli::ScanProfile::Endpoint),
        "web" => Some(crate::cli::ScanProfile::Web),
        "waf" => Some(crate::cli::ScanProfile::Waf),
        "full" => Some(crate::cli::ScanProfile::Full),
        "api" => Some(crate::cli::ScanProfile::Api),
        "recon" => Some(crate::cli::ScanProfile::Recon),
        "stealth" => Some(crate::cli::ScanProfile::Stealth),
        "deep" => Some(crate::cli::ScanProfile::Deep),
        "vuln" => Some(crate::cli::ScanProfile::Vuln),
        "auth" => Some(crate::cli::ScanProfile::Auth),
        "defense-lab" => Some(crate::cli::ScanProfile::DefenseLab),
        "synvoid-local" => Some(crate::cli::ScanProfile::SynvoidLocal),
        "waf-regression" => Some(crate::cli::ScanProfile::WafRegression),
        "protocol-edge" => Some(crate::cli::ScanProfile::ProtocolEdge),
        "nse-safe" => Some(crate::cli::ScanProfile::NseSafe),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_stages() {
        let stages = parse_stages("port,fingerprint,fuzz");
        assert_eq!(stages.len(), 3);
        assert_eq!(stages[0], Stage::PortScan);
        assert_eq!(stages[1], Stage::Fingerprint);
        assert_eq!(stages[2], Stage::Fuzz);
    }

    #[test]
    fn test_parse_stages_aliases() {
        let stages = parse_stages("portscan,fp,endpoint-scan");
        assert_eq!(stages.len(), 3);
    }

    #[test]
    fn test_parse_stages_unknown_ignored() {
        let stages = parse_stages("port,unknown,fuzz");
        assert_eq!(stages.len(), 2);
    }

    #[test]
    fn test_quick_profile() {
        let stages = Stage::from_profile(ScanProfile::Quick);
        assert_eq!(stages.len(), 2);
        assert_eq!(stages[0], Stage::PortScan);
        assert_eq!(stages[1], Stage::Fingerprint);
    }

    #[test]
    fn test_full_profile() {
        let stages = Stage::from_profile(ScanProfile::Full);
        assert_eq!(stages.len(), 5);
        assert_eq!(stages[4], Stage::LoadTest);
    }

    #[test]
    fn test_waf_profile() {
        let stages = Stage::from_profile(ScanProfile::Waf);
        assert!(stages.contains(&Stage::Waf));
    }

    #[test]
    fn test_recon_profile() {
        let stages = Stage::from_profile(ScanProfile::Recon);
        assert!(stages.contains(&Stage::Recon));
    }
}
