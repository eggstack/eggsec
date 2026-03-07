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
            _ => None,
        }
    }
}

pub fn parse_stages(s: &str) -> Vec<Stage> {
    s.split(',')
        .filter_map(|part| Stage::from_string(part.trim()))
        .collect()
}
