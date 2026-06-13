use serde::{Deserialize, Serialize};

use crate::cli::ScanProfile;
use crate::probe::{ProbeIntent, ProbeRisk};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Stage {
    PortScan,
    Fingerprint,
    EndpointScan,
    Fuzz,
    LoadTest,
    Waf,
    Recon,
    Vuln,
    #[cfg(feature = "db-pentest")]
    DbPentest,
    #[cfg(feature = "web-proxy")]
    WebProxy,
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
            #[cfg(feature = "db-pentest")]
            Stage::DbPentest => write!(f, "DB Pentest"),
            #[cfg(feature = "web-proxy")]
            Stage::WebProxy => write!(f, "Web Proxy Intercept"),
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
                Stage::Vuln,
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
            // DbRegression is additive defense-lab / regression family.
            #[cfg(feature = "db-pentest")]
            ScanProfile::DbRegression => vec![Stage::DbPentest],
            #[cfg(not(feature = "db-pentest"))]
            ScanProfile::DbRegression => vec![
                Stage::PortScan,
                Stage::Fingerprint,
                Stage::EndpointScan,
                Stage::Waf,
                Stage::Fuzz,
            ],
            // WebProxy runs a single interception stage (like DbRegression).
            #[cfg(feature = "web-proxy")]
            ScanProfile::WebProxy => vec![Stage::WebProxy],
            #[cfg(not(feature = "web-proxy"))]
            ScanProfile::WebProxy => vec![
                Stage::PortScan,
                Stage::Fingerprint,
                Stage::EndpointScan,
                Stage::Waf,
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
            "vuln" | "vulnerability" | "vuln-assess" => Some(Stage::Vuln),
            "db" | "dbpentest" | "db-pentest" => {
                #[cfg(feature = "db-pentest")]
                { Some(Stage::DbPentest) }
                #[cfg(not(feature = "db-pentest"))]
                { None }
            }
            "proxy" | "webproxy" | "web-proxy" | "intercept" => {
                #[cfg(feature = "web-proxy")]
                { Some(Stage::WebProxy) }
                #[cfg(not(feature = "web-proxy"))]
                { None }
            }
            _ => None,
        }
    }

    /// Map this stage to its primary probe intent category.
    pub fn to_probe_intent(self) -> ProbeIntent {
        match self {
            Stage::PortScan => ProbeIntent::Discovery,
            Stage::Fingerprint => ProbeIntent::Fingerprint,
            Stage::EndpointScan => ProbeIntent::ServiceValidation,
            Stage::Fuzz => ProbeIntent::EvasionResistance,
            Stage::LoadTest => ProbeIntent::LoadBearing,
            Stage::Waf => ProbeIntent::WafEvaluation,
            Stage::Recon => ProbeIntent::Discovery,
            Stage::Vuln => ProbeIntent::ServiceValidation,
            #[cfg(feature = "db-pentest")]
            Stage::DbPentest => ProbeIntent::ServiceValidation,
            #[cfg(feature = "web-proxy")]
            Stage::WebProxy => ProbeIntent::WafEvaluation,
        }
    }

    /// Map this stage to its minimum required risk level.
    ///
    /// Stages that are inherently more intrusive require a higher risk
    /// budget to execute. The pipeline uses this to skip stages that
    /// exceed the profile's allowed risk budget.
    pub fn to_probe_risk(self) -> ProbeRisk {
        match self {
            Stage::PortScan => ProbeRisk::SafeActive,
            Stage::Fingerprint => ProbeRisk::Passive,
            Stage::EndpointScan => ProbeRisk::SafeActive,
            Stage::Fuzz => ProbeRisk::Intrusive,
            Stage::LoadTest => ProbeRisk::Stress,
            Stage::Waf => ProbeRisk::Intrusive,
            Stage::Recon => ProbeRisk::Passive,
            Stage::Vuln => ProbeRisk::SafeActive,
            #[cfg(feature = "db-pentest")]
            Stage::DbPentest => ProbeRisk::Intrusive,
            #[cfg(feature = "web-proxy")]
            Stage::WebProxy => ProbeRisk::Intrusive,
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
        "db-regression" | "db_regression" | "dbregression" => Some(crate::cli::ScanProfile::DbRegression),
        "web-proxy" | "webproxy" | "proxy" => Some(crate::cli::ScanProfile::WebProxy),
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

    #[test]
    fn test_defense_lab_profile() {
        let stages = Stage::from_profile(ScanProfile::DefenseLab);
        assert_eq!(stages.len(), 5);
        assert_eq!(stages[0], Stage::PortScan);
        assert_eq!(stages[1], Stage::Fingerprint);
        assert_eq!(stages[2], Stage::EndpointScan);
        assert_eq!(stages[3], Stage::Waf);
        assert_eq!(stages[4], Stage::Fuzz);
    }

    #[test]
    fn test_synvoid_local_profile() {
        let stages = Stage::from_profile(ScanProfile::SynvoidLocal);
        assert_eq!(stages.len(), 4);
        assert_eq!(stages[0], Stage::PortScan);
        assert_eq!(stages[1], Stage::Fingerprint);
        assert_eq!(stages[2], Stage::EndpointScan);
        assert_eq!(stages[3], Stage::Waf);
    }

    #[test]
    fn test_waf_regression_profile() {
        let stages = Stage::from_profile(ScanProfile::WafRegression);
        assert_eq!(stages.len(), 3);
        assert_eq!(stages[0], Stage::PortScan);
        assert_eq!(stages[1], Stage::Fingerprint);
        assert_eq!(stages[2], Stage::Waf);
    }

    #[test]
    fn test_protocol_edge_profile() {
        let stages = Stage::from_profile(ScanProfile::ProtocolEdge);
        assert_eq!(stages.len(), 2);
        assert_eq!(stages[0], Stage::PortScan);
        assert_eq!(stages[1], Stage::Fingerprint);
    }

    #[test]
    fn test_nse_safe_profile() {
        let stages = Stage::from_profile(ScanProfile::NseSafe);
        assert_eq!(stages.len(), 3);
        assert_eq!(stages[0], Stage::PortScan);
        assert_eq!(stages[1], Stage::Fingerprint);
        assert_eq!(stages[2], Stage::EndpointScan);
    }

    #[test]
    fn test_profile_from_str_defense_lab() {
        assert_eq!(
            profile_from_str("defense-lab"),
            Some(ScanProfile::DefenseLab)
        );
        assert_eq!(
            profile_from_str("synvoid-local"),
            Some(ScanProfile::SynvoidLocal)
        );
        assert_eq!(
            profile_from_str("waf-regression"),
            Some(ScanProfile::WafRegression)
        );
        assert_eq!(
            profile_from_str("protocol-edge"),
            Some(ScanProfile::ProtocolEdge)
        );
        assert_eq!(profile_from_str("nse-safe"), Some(ScanProfile::NseSafe));
        assert_eq!(profile_from_str("db-regression"), Some(ScanProfile::DbRegression));
    }

    #[test]
    fn test_profile_from_str_case_insensitive() {
        assert_eq!(
            profile_from_str("Defense-Lab"),
            Some(ScanProfile::DefenseLab)
        );
        assert_eq!(
            profile_from_str("SYNVOID-LOCAL"),
            Some(ScanProfile::SynvoidLocal)
        );
    }

    #[test]
    fn test_profile_from_str_invalid() {
        assert_eq!(profile_from_str("nonexistent"), None);
        assert_eq!(profile_from_str(""), None);
    }

    #[test]
    fn test_db_regression_profile() {
        let stages = Stage::from_profile(ScanProfile::DbRegression);
        #[cfg(feature = "db-pentest")]
        {
            assert_eq!(stages.len(), 1);
            assert_eq!(stages[0], Stage::DbPentest);
        }
        #[cfg(not(feature = "db-pentest"))]
        {
            assert!(!stages.is_empty());
        }
    }

    #[test]
    fn test_stage_to_probe_intent() {
        assert_eq!(Stage::PortScan.to_probe_intent(), ProbeIntent::Discovery);
        assert_eq!(
            Stage::Fingerprint.to_probe_intent(),
            ProbeIntent::Fingerprint
        );
        assert_eq!(
            Stage::EndpointScan.to_probe_intent(),
            ProbeIntent::ServiceValidation
        );
        assert_eq!(
            Stage::Fuzz.to_probe_intent(),
            ProbeIntent::EvasionResistance
        );
        assert_eq!(Stage::LoadTest.to_probe_intent(), ProbeIntent::LoadBearing);
        assert_eq!(Stage::Waf.to_probe_intent(), ProbeIntent::WafEvaluation);
        assert_eq!(Stage::Recon.to_probe_intent(), ProbeIntent::Discovery);
        assert_eq!(
            Stage::Vuln.to_probe_intent(),
            ProbeIntent::ServiceValidation
        );
    }

    #[test]
    fn test_stage_to_probe_risk() {
        assert_eq!(Stage::PortScan.to_probe_risk(), ProbeRisk::SafeActive);
        assert_eq!(Stage::Fingerprint.to_probe_risk(), ProbeRisk::Passive);
        assert_eq!(Stage::EndpointScan.to_probe_risk(), ProbeRisk::SafeActive);
        assert_eq!(Stage::Fuzz.to_probe_risk(), ProbeRisk::Intrusive);
        assert_eq!(Stage::LoadTest.to_probe_risk(), ProbeRisk::Stress);
        assert_eq!(Stage::Waf.to_probe_risk(), ProbeRisk::Intrusive);
        assert_eq!(Stage::Recon.to_probe_risk(), ProbeRisk::Passive);
        assert_eq!(Stage::Vuln.to_probe_risk(), ProbeRisk::SafeActive);
    }

    #[cfg(feature = "db-pentest")]
    #[test]
    fn test_stage_from_string_db_pentest() {
        assert_eq!(Stage::from_string("db"), Some(Stage::DbPentest));
        assert_eq!(Stage::from_string("db-pentest"), Some(Stage::DbPentest));
        assert_eq!(Stage::from_string("dbpentest"), Some(Stage::DbPentest));
    }
}
