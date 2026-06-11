//! Wireless security testing module
//!
//! Provides wireless network security testing capabilities including
//! iwlist scan parsing and wireless security type analysis.
//! WPA/WPA2 handshake capture analysis is aspirational (not yet implemented).
//!
//! This module is feature-gated behind the `wireless` feature flag.

use crate::error::{EggsecError, Result};
use crate::types::Severity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WirelessNetwork {
    pub ssid: String,
    pub bssid: String,
    pub channel: u8,
    pub security_type: SecurityType,
    pub signal_strength: i32,
    pub last_seen: String,
    pub wps_enabled: bool,
    pub is_hidden: bool,
    pub transition_mode: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecurityType {
    Open,
    WEP,
    WPA,
    WPA2,
    WPA3,
    Enterprise,
    Unknown,
}

impl SecurityType {
    pub fn as_str(&self) -> &str {
        match self {
            SecurityType::Open => "Open",
            SecurityType::WEP => "WEP",
            SecurityType::WPA => "WPA",
            SecurityType::WPA2 => "WPA2",
            SecurityType::WPA3 => "WPA3",
            SecurityType::Enterprise => "Enterprise",
            SecurityType::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WirelessScanResult {
    pub interface: String,
    pub networks: Vec<WirelessNetwork>,
    pub scan_duration_secs: u64,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WirelessVulnerability {
    pub ssid: String,
    pub bssid: String,
    pub vulnerability_type: String,
    pub severity: Severity,
    pub description: String,
    pub recommendation: String,
}

pub struct WirelessScanner {
    interface: Option<String>,
}

impl WirelessScanner {
    pub fn new() -> Self {
        Self { interface: None }
    }

    pub fn with_interface(mut self, interface: String) -> Self {
        self.interface = Some(interface);
        self
    }

    #[cfg(feature = "wireless")]
    pub async fn scan(&self, duration_secs: u64) -> Result<WirelessScanResult> {
        use tokio::process::Command;

        let interface = self
            .interface
            .as_ref()
            .ok_or_else(|| EggsecError::Config("No wireless interface specified".to_string()))?;

        let start = std::time::Instant::now();

        let output = Command::new("iwlist")
            .args([interface, "scan"])
            .output()
            .await
            .map_err(|e| EggsecError::Network(format!("iwlist scan failed: {}", e)))?;

        let elapsed_secs = start.elapsed().as_secs();

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let msg = if stderr.trim().is_empty() {
                format!(
                    "iwlist scan failed with exit code {}",
                    output.status.code().unwrap_or(-1)
                )
            } else {
                format!("iwlist scan failed: {}", stderr.trim())
            };
            return Err(EggsecError::Network(msg));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut networks = self.parse_scan_output(&stdout);
        let scan_time = chrono::Utc::now().to_rfc3339();
        for network in &mut networks {
            network.last_seen = scan_time.clone();
        }

        let recommendations = Self::generate_recommendations(&networks);

        Ok(WirelessScanResult {
            interface: interface.clone(),
            networks,
            scan_duration_secs: elapsed_secs.max(duration_secs),
            recommendations,
        })
    }

    #[cfg(feature = "wireless")]
    fn parse_scan_output(&self, output: &str) -> Vec<WirelessNetwork> {
        let mut networks = Vec::new();
        let mut current_ssid: Option<String> = None;
        let mut current_bssid: Option<String> = None;
        let mut current_channel = 0u8;
        let mut current_security = SecurityType::Unknown;
        let mut current_signal = -100i32;
        let mut current_auth_suite: Option<String> = None;
        let mut current_wps = false;
        let mut current_is_hidden = false;
        let mut current_transition = false;
        let mut saw_wpa2 = false;
        let mut saw_wpa3 = false;

        for line in output.lines() {
            let line = line.trim();

            if line.starts_with("Cell ") {
                if let (Some(ssid), Some(bssid)) = (current_ssid.take(), current_bssid.take()) {
                    let mut final_ssid = ssid.clone();
                    if ssid.is_empty() || ssid == "<hidden>" || ssid == "\"\"" {
                        final_ssid = "<hidden>".to_string();
                        current_is_hidden = true;
                    }
                    networks.push(WirelessNetwork {
                        ssid: final_ssid,
                        bssid,
                        channel: current_channel,
                        security_type: current_security,
                        signal_strength: current_signal,
                        last_seen: String::new(),
                        wps_enabled: current_wps,
                        is_hidden: current_is_hidden,
                        transition_mode: current_transition || (saw_wpa2 && saw_wpa3),
                    });
                }
                current_auth_suite = None;
                current_channel = 0u8;
                current_security = SecurityType::Unknown;
                current_signal = -100i32;
                current_wps = false;
                current_is_hidden = false;
                current_transition = false;
                saw_wpa2 = false;
                saw_wpa3 = false;
            }

            if line.contains("Address:") {
                if let Some(addr) = line.split("Address:").nth(1) {
                    let bssid = addr.trim().to_string();
                    if !bssid.is_empty() && bssid.contains(':') {
                        current_bssid = Some(bssid);
                    }
                }
            }

            if line.starts_with("ESSID:") {
                let essid = line.split("ESSID:\"")
                    .nth(1)
                    .unwrap_or("")
                    .trim_matches('"')
                    .to_string();
                current_ssid = Some(essid.clone());
                if essid.is_empty() || essid == "<hidden>" || essid == "\"\"" {
                    current_is_hidden = true;
                }
            }

            if line.contains("Channel:") {
                current_channel = line
                    .split("Channel:")
                    .nth(1)
                    .unwrap_or("1")
                    .trim()
                    .parse()
                    .unwrap_or(1);
            }

            if line.contains("Signal level") {
                let level_str = if let Some(rest) = line.split("Signal level=").nth(1) {
                    rest.split_whitespace().next().unwrap_or("-100")
                } else if let Some(rest) = line.split("Signal level:").nth(1) {
                    rest.split_whitespace().next().unwrap_or("-100")
                } else {
                    "-100"
                };
                current_signal = level_str.parse().unwrap_or(-100);
            }

            let lower = line.to_lowercase();
            if lower.contains("wps") || lower.contains("wi-fi protected setup") {
                current_wps = true;
            }

            if lower.contains("wpa2/wpa3") || lower.contains("transition") {
                current_transition = true;
            }

            if line.contains("Encryption key:") && line.contains("off") {
                current_security = SecurityType::Open;
            } else if line.contains("WPA3") {
                current_security = SecurityType::WPA3;
                saw_wpa3 = true;
            } else if line.contains("WPA2") {
                current_security = SecurityType::WPA2;
                saw_wpa2 = true;
            } else if line.contains("WPA") {
                current_security = SecurityType::WPA;
            } else if line.contains("WEP") {
                current_security = SecurityType::WEP;
            }

            if line.contains("Authentication Suites") {
                if let Some(suite) = line.split(':').nth(1) {
                    current_auth_suite = Some(suite.trim().to_string());
                }
            }

            if current_auth_suite.as_deref() == Some("802.1X") {
                current_security = SecurityType::Enterprise;
            }
        }

        if let (Some(ssid), Some(bssid)) = (current_ssid, current_bssid) {
            let mut final_ssid = ssid.clone();
            if ssid.is_empty() || ssid == "<hidden>" || ssid == "\"\"" {
                final_ssid = "<hidden>".to_string();
                current_is_hidden = true;
            }
            networks.push(WirelessNetwork {
                ssid: final_ssid,
                bssid,
                channel: current_channel,
                security_type: current_security,
                signal_strength: current_signal,
                last_seen: String::new(),
                wps_enabled: current_wps,
                is_hidden: current_is_hidden,
                transition_mode: current_transition || (saw_wpa2 && saw_wpa3),
            });
        }

        networks
    }

    pub fn analyze_networks(networks: &[WirelessNetwork]) -> Vec<WirelessVulnerability> {
        let mut vulnerabilities = Vec::new();

        for network in networks {
            if network.signal_strength <= -80 {
                vulnerabilities.push(WirelessVulnerability {
                    ssid: network.ssid.clone(),
                    bssid: network.bssid.clone(),
                    vulnerability_type: "Weak Signal Strength".to_string(),
                    severity: if network.signal_strength <= -90 { Severity::Low } else { Severity::Medium },
                    description: format!("Network {} has weak signal ({} dBm)", network.ssid, network.signal_strength),
                    recommendation: "Reposition AP or investigate interference".to_string(),
                });
            }

            if network.wps_enabled {
                vulnerabilities.push(WirelessVulnerability {
                    ssid: network.ssid.clone(),
                    bssid: network.bssid.clone(),
                    vulnerability_type: "WPS Enabled".to_string(),
                    severity: Severity::Medium,
                    description: format!("Network {} has WPS enabled (PIN brute-force risk)", network.ssid),
                    recommendation: "Disable WPS or use PIN-less mode with rate limiting".to_string(),
                });
            }

            if network.is_hidden {
                vulnerabilities.push(WirelessVulnerability {
                    ssid: network.ssid.clone(),
                    bssid: network.bssid.clone(),
                    vulnerability_type: "Hidden SSID".to_string(),
                    severity: Severity::Low,
                    description: format!("Network {} uses hidden SSID (provides minimal security benefit)", network.ssid),
                    recommendation: "Consider broadcasting SSID for better client compatibility".to_string(),
                });
            }

            if network.transition_mode {
                vulnerabilities.push(WirelessVulnerability {
                    ssid: network.ssid.clone(),
                    bssid: network.bssid.clone(),
                    vulnerability_type: "WPA2/WPA3 Transition Mode".to_string(),
                    severity: Severity::Low,
                    description: format!("Network {} is in WPA2/WPA3 transition mode (downgrade risk)", network.ssid),
                    recommendation: "Enforce WPA3-only when all clients support it".to_string(),
                });
            }

            match network.security_type {
                SecurityType::Open => {
                    vulnerabilities.push(WirelessVulnerability {
                        ssid: network.ssid.clone(),
                        bssid: network.bssid.clone(),
                        vulnerability_type: "Open Network".to_string(),
                        severity: Severity::Medium,
                        description: format!("Network {} has no encryption", network.ssid),
                        recommendation: "Enable WPA2 or WPA3 encryption".to_string(),
                    });
                }
                SecurityType::WEP => {
                    vulnerabilities.push(WirelessVulnerability {
                        ssid: network.ssid.clone(),
                        bssid: network.bssid.clone(),
                        vulnerability_type: "WEP Encryption".to_string(),
                        severity: Severity::High,
                        description: "WEP encryption is easily cracked".to_string(),
                        recommendation: "Upgrade to WPA2 or WPA3 immediately".to_string(),
                    });
                }
                SecurityType::WPA => {
                    vulnerabilities.push(WirelessVulnerability {
                        ssid: network.ssid.clone(),
                        bssid: network.bssid.clone(),
                        vulnerability_type: "WPA Encryption".to_string(),
                        severity: Severity::Medium,
                        description: "WPA encryption has known vulnerabilities".to_string(),
                        recommendation: "Upgrade to WPA2 or WPA3".to_string(),
                    });
                }
                SecurityType::Enterprise => {
                    vulnerabilities.push(WirelessVulnerability {
                        ssid: network.ssid.clone(),
                        bssid: network.bssid.clone(),
                        vulnerability_type: "Enterprise Authentication".to_string(),
                        severity: Severity::Low,
                        description: "Enterprise (802.1X) network detected - verify RADIUS server configuration and certificate validation".to_string(),
                        recommendation: "Ensure proper RADIUS server configuration, certificate validation, and EAP method security".to_string(),
                    });
                }
                SecurityType::Unknown => {
                    vulnerabilities.push(WirelessVulnerability {
                        ssid: network.ssid.clone(),
                        bssid: network.bssid.clone(),
                        vulnerability_type: "Unknown Security".to_string(),
                        severity: Severity::Medium,
                        description: "Wireless security type could not be determined - manual verification recommended".to_string(),
                        recommendation: "Verify wireless security configuration manually".to_string(),
                    });
                }
                _ => {}
            }
        }

        let mut ssid_groups: std::collections::HashMap<String, Vec<&WirelessNetwork>> = std::collections::HashMap::new();
        for network in networks {
            ssid_groups.entry(network.ssid.clone()).or_default().push(network);
        }
        for (ssid, nets) in &ssid_groups {
            if nets.len() >= 2 {
                let distinct_bssids: std::collections::HashSet<_> = nets.iter().map(|n| &n.bssid).collect();
                let distinct_secs: std::collections::HashSet<_> = nets.iter().map(|n| n.security_type).collect();
                if distinct_bssids.len() >= 2 || distinct_secs.len() >= 2 {
                    let bssid_list: Vec<_> = nets.iter().map(|n| format!("{} (ch{}, {})", n.bssid, n.channel, n.security_type.as_str())).collect();
                    vulnerabilities.push(WirelessVulnerability {
                        ssid: ssid.clone(),
                        bssid: nets[0].bssid.clone(),
                        vulnerability_type: "Possible Rogue AP / Evil Twin (passive heuristic)".to_string(),
                        severity: Severity::Low,
                        description: format!("Multiple BSSIDs or security configs for SSID {}: {}", ssid, bssid_list.join("; ")),
                        recommendation: "Verify authorized APs only; investigate in lab".to_string(),
                    });
                }
            }
        }

        vulnerabilities
    }

    fn generate_recommendations(networks: &[WirelessNetwork]) -> Vec<String> {
        let mut recommendations = Vec::new();
        let mut seen: rustc_hash::FxHashSet<String> = rustc_hash::FxHashSet::default();

        for network in networks {
            let sec_key = format!("sec:{}", network.security_type.as_str());
            if seen.insert(sec_key) {
                match network.security_type {
                    SecurityType::WEP => {
                        recommendations.push("Upgrade from WEP to WPA2 or WPA3 immediately — WEP is trivially cracked".to_string());
                    }
                    SecurityType::Open => {
                        recommendations
                            .push("Enable WPA2 or WPA3 encryption on open networks".to_string());
                    }
                    SecurityType::WPA => {
                        recommendations.push(
                            "Upgrade from WPA to WPA2 or WPA3 — WPA has known TKIP vulnerabilities"
                                .to_string(),
                        );
                    }
                    SecurityType::Unknown => {
                        recommendations.push("Verify wireless security configuration — security type could not be determined".to_string());
                    }
                    _ => {}
                }
            }
            if network.wps_enabled && seen.insert(format!("wps:{}", network.bssid)) {
                recommendations.push("Disable WPS on networks where it is enabled (PIN brute-force risk)".to_string());
            }
            if network.transition_mode && seen.insert(format!("transition:{}", network.bssid)) {
                recommendations.push("Consider enforcing WPA3-only mode on transition networks to avoid downgrade attacks".to_string());
            }
            if network.is_hidden && seen.insert(format!("hidden:{}", network.bssid)) {
                recommendations.push("Hidden SSIDs offer little security benefit; consider broadcasting for client compatibility".to_string());
            }
            if network.signal_strength <= -80 && seen.insert(format!("weak:{}", network.bssid)) {
                recommendations.push("Investigate weak signal networks for interference or reposition APs".to_string());
            }
        }

        if recommendations.is_empty() && !networks.is_empty() {
            recommendations.push(
                "All detected networks use strong encryption (WPA2/WPA3/Enterprise)".to_string(),
            );
        }
        if networks.is_empty() {
            recommendations.push("No wireless networks were detected during the scan".to_string());
        }

        recommendations.push("Run repeated scans to observe changes over time for rogue detection.".to_string());

        recommendations
    }
}

impl Default for WirelessScanner {
    fn default() -> Self {
        Self::new()
    }
}

pub fn to_scan_report_data(result: &WirelessScanResult) -> crate::output::convert::ScanReportData {
    use crate::output::convert::{FindingData, WirelessNetworkReportData};

    let findings: Vec<FindingData> = WirelessScanner::analyze_networks(&result.networks)
        .iter()
        .map(|v| FindingData {
            title: v.vulnerability_type.clone(),
            severity: v.severity.as_str().to_string(),
            category: "wireless".to_string(),
            description: v.description.clone(),
            location: format!("{} ({})", v.ssid, v.bssid),
            evidence: None,
            remediation: Some(v.recommendation.clone()),
            cwe_ids: Vec::new(),
        })
        .collect();

    let wireless_networks = result
        .networks
        .iter()
        .map(|n| WirelessNetworkReportData {
            ssid: n.ssid.clone(),
            bssid: n.bssid.clone(),
            channel: n.channel,
            security_type: n.security_type.as_str().to_string(),
            signal_strength: n.signal_strength,
            last_seen: n.last_seen.clone(),
            wps_enabled: n.wps_enabled,
            is_hidden: n.is_hidden,
            transition_mode: n.transition_mode,
        })
        .collect();

    crate::output::convert::ScanReportData {
        target: result.interface.clone(),
        scan_type: "wireless".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        findings,
        open_ports: Vec::new(),
        services: Vec::new(),
        duration_ms: result.scan_duration_secs * 1000,
        wireless_networks,
        policy_summary: None,
    }
}

pub async fn run_cli(
    args: crate::cli::WirelessArgs,
    _config: &crate::config::EggsecConfig,
) -> Result<()> {
    let scanner = WirelessScanner::new().with_interface(args.interface.clone());

    if !args.quiet {
        eprintln!("WARNING: Requires root (or CAP_NET_ADMIN) and 'iwlist' (wireless-tools). Interface must be in managed mode and up. Use only on authorized networks in lab/defense-validation contexts. This is passive reconnaissance.");
        if args.repeat == 1 {
            eprintln!("Scanning on {} for ~{}s...", args.interface, args.duration);
        } else {
            eprintln!("Performing {} repeated scans on {} ({}s each, ~2s delay between)...", args.repeat, args.interface, args.duration);
        }
    }

    let mut last_result = None;
    for i in 1..=args.repeat {
        if !args.quiet && args.repeat > 1 {
            eprintln!("Scan {}/{} ...", i, args.repeat);
        }
        let result = scanner.scan(args.duration).await?;
        last_result = Some(result);
        if i < args.repeat {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }
    let result = last_result.expect("at least one scan performed");

    let output = if args.json {
        serde_json::to_string_pretty(&result)?
    } else {
        let mut buf = String::new();
        buf.push_str(&format!(
            "Wireless Scan Results - Interface: {}\n",
            result.interface
        ));
        buf.push_str(&format!("Networks found: {}\n\n", result.networks.len()));

        for (i, network) in result.networks.iter().enumerate() {
            buf.push_str(&format!("  {}. {}\n", i + 1, network.ssid));
            buf.push_str(&format!("     BSSID:    {}\n", network.bssid));
            buf.push_str(&format!("     Channel:  {}\n", network.channel));
            buf.push_str(&format!(
                "     Security: {}{}\n",
                network.security_type.as_str(),
                if network.wps_enabled { " (WPS)" } else { "" }
            ));
            buf.push_str(&format!("     Signal:   {} dBm{}\n", network.signal_strength, if network.is_hidden { " [hidden]" } else { "" }));
            buf.push_str(&format!("     Last seen: {}\n", network.last_seen));
            if network.transition_mode {
                buf.push_str("     Note: WPA2/WPA3 transition mode\n");
            }
            buf.push('\n');
        }

        let vulns = WirelessScanner::analyze_networks(&result.networks);
        if !vulns.is_empty() {
            buf.push_str("Findings / Vulnerabilities:\n");
            for v in &vulns {
                buf.push_str(&format!(
                    "  [{}] {} ({}): {}\n     Rec: {}\n",
                    v.severity.as_str(),
                    v.vulnerability_type,
                    v.ssid,
                    v.description,
                    v.recommendation
                ));
            }
            buf.push('\n');
        }

        if !result.recommendations.is_empty() {
            buf.push_str("Recommendations:\n");
            for rec in &result.recommendations {
                buf.push_str(&format!("  - {}\n", rec));
            }
        }

        buf
    };

    if let Some(ref output_file) = args.output {
        tokio::fs::write(output_file, &output).await?;
        if !args.quiet {
            eprintln!("Results written to {}", output_file);
        }
    } else {
        println!("{}", output);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_type_str() {
        assert_eq!(SecurityType::WPA2.as_str(), "WPA2");
        assert_eq!(SecurityType::Open.as_str(), "Open");
    }

    #[test]
    fn test_wireless_scanner_creation() {
        let _scanner = WirelessScanner::new();
    }

    #[test]
    fn test_network_analysis() {
        let networks = vec![
            WirelessNetwork {
                ssid: "OpenNetwork".to_string(),
                bssid: "00:11:22:33:44:55".to_string(),
                channel: 6,
                security_type: SecurityType::Open,
                signal_strength: -50,
                last_seen: String::new(),
                wps_enabled: false,
                is_hidden: false,
                transition_mode: false,
            },
            WirelessNetwork {
                ssid: "SecureNetwork".to_string(),
                bssid: "00:11:22:33:44:66".to_string(),
                channel: 11,
                security_type: SecurityType::WPA3,
                signal_strength: -60,
                last_seen: String::new(),
                wps_enabled: false,
                is_hidden: false,
                transition_mode: false,
            },
        ];

        let vulns = WirelessScanner::analyze_networks(&networks);
        assert_eq!(vulns.len(), 1);
        assert_eq!(vulns[0].vulnerability_type, "Open Network");
    }

    #[test]
    fn test_parse_scan_output_resets_state_per_cell() {
        let scanner = WirelessScanner::new();
        let iwlist_output = "Cell 01 - Address: 00:11:22:33:44:55\nESSID:\"Network1\"\nChannel:6\nSignal level=-50 dBi\nEncryption key:on\nWPA2\n\nCell 02 - Address: 00:11:22:33:44:66\nESSID:\"Network2\"\nChannel:11\nSignal level=-70 dBi\nEncryption key:on\nWPA3\n";
        let networks = scanner.parse_scan_output(iwlist_output);
        assert_eq!(networks.len(), 2);
        assert_eq!(networks[0].ssid, "Network1");
        assert_eq!(networks[0].channel, 6);
        assert_eq!(networks[0].signal_strength, -50);
        assert_eq!(networks[0].security_type, SecurityType::WPA2);
        assert_eq!(networks[1].ssid, "Network2");
        assert_eq!(networks[1].channel, 11);
        assert_eq!(networks[1].signal_strength, -70);
        assert_eq!(networks[1].security_type, SecurityType::WPA3);
    }

    #[test]
    fn test_parse_scan_output_open_network() {
        let scanner = WirelessScanner::new();
        let iwlist_output = "Cell 01 - Address: 00:11:22:33:44:55\nESSID:\"OpenNet\"\nChannel:1\nSignal level=-60 dBi\nEncryption key:off\n";
        let networks = scanner.parse_scan_output(iwlist_output);
        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0].ssid, "OpenNet");
        assert_eq!(networks[0].security_type, SecurityType::Open);
    }

    #[test]
    fn test_parse_scan_output_enterprise_network() {
        let scanner = WirelessScanner::new();
        let iwlist_output = "Cell 01 - Address: 00:11:22:33:44:55\nESSID:\"EnterpriseNet\"\nChannel:36\nSignal level=-45 dBi\nEncryption key:on\nWPA2\nAuthentication Suites (1): 802.1X\n";
        let networks = scanner.parse_scan_output(iwlist_output);
        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0].ssid, "EnterpriseNet");
        assert_eq!(networks[0].security_type, SecurityType::Enterprise);
    }

    #[test]
    fn test_parse_scan_output_wps() {
        let scanner = WirelessScanner::new();
        let iwlist_output = "Cell 01 - Address: 00:11:22:33:44:55\nESSID:\"WPSNet\"\nChannel:6\nSignal level=-55 dBi\nEncryption key:on\nWPA2\nWPS\n";
        let networks = scanner.parse_scan_output(iwlist_output);
        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0].ssid, "WPSNet");
        assert!(networks[0].wps_enabled);
    }

    #[test]
    fn test_parse_scan_output_hidden() {
        let scanner = WirelessScanner::new();
        let iwlist_output = "Cell 01 - Address: 00:11:22:33:44:55\nESSID:\"\" \nChannel:1\nSignal level=-65 dBi\nEncryption key:on\nWPA2\n";
        let networks = scanner.parse_scan_output(iwlist_output);
        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0].ssid, "<hidden>");
        assert!(networks[0].is_hidden);
    }

    #[test]
    fn test_parse_scan_output_transition() {
        let scanner = WirelessScanner::new();
        let iwlist_output = "Cell 01 - Address: 00:11:22:33:44:55\nESSID:\"TransitionNet\"\nChannel:36\nSignal level=-50 dBi\nEncryption key:on\nWPA2\nWPA3\n";
        let networks = scanner.parse_scan_output(iwlist_output);
        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0].ssid, "TransitionNet");
        assert!(networks[0].transition_mode);
    }

    #[test]
    fn test_analyze_weak_signal() {
        let networks = vec![WirelessNetwork {
            ssid: "WeakNet".to_string(),
            bssid: "00:11:22:33:44:55".to_string(),
            channel: 6,
            security_type: SecurityType::WPA2,
            signal_strength: -85,
            last_seen: String::new(),
            wps_enabled: false,
            is_hidden: false,
            transition_mode: false,
        }];
        let vulns = WirelessScanner::analyze_networks(&networks);
        assert!(vulns.iter().any(|v| v.vulnerability_type == "Weak Signal Strength"));
    }

    #[test]
    fn test_analyze_rogue_candidate() {
        let networks = vec![
            WirelessNetwork {
                ssid: "CorpNet".to_string(),
                bssid: "00:11:22:33:44:55".to_string(),
                channel: 6,
                security_type: SecurityType::WPA2,
                signal_strength: -50,
                last_seen: String::new(),
                wps_enabled: false,
                is_hidden: false,
                transition_mode: false,
            },
            WirelessNetwork {
                ssid: "CorpNet".to_string(),
                bssid: "aa:bb:cc:dd:ee:ff".to_string(),
                channel: 11,
                security_type: SecurityType::Open,
                signal_strength: -55,
                last_seen: String::new(),
                wps_enabled: false,
                is_hidden: false,
                transition_mode: false,
            },
        ];
        let vulns = WirelessScanner::analyze_networks(&networks);
        assert!(vulns.iter().any(|v| v.vulnerability_type.contains("Rogue AP / Evil Twin")));
    }

    #[test]
    fn test_parse_scan_output_wpa2_wpa3_mixed() {
        let scanner = WirelessScanner::new();
        let iwlist_output = "Cell 01 - Address: 00:11:22:33:44:55\nESSID:\"MixedNet\"\nChannel:36\nSignal level=-45 dBi\nEncryption key:on\nWPA2\nWPA3\n";
        let networks = scanner.parse_scan_output(iwlist_output);
        assert_eq!(networks.len(), 1);
        assert!(networks[0].transition_mode);
    }
}
