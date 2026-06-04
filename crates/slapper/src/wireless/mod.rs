//! Wireless security testing module
//!
//! Provides wireless network security testing capabilities including
//! iwlist scan parsing and wireless security type analysis.
//! WPA/WPA2 handshake capture analysis is aspirational (not yet implemented).
//!
//! This module is feature-gated behind the `wireless` feature flag.

use crate::error::{Result, SlapperError};
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
    pub fn new() -> Result<Self> {
        Ok(Self { interface: None })
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
            .ok_or_else(|| SlapperError::Config("No wireless interface specified".to_string()))?;

        let output = Command::new("iwlist")
            .args([interface, "scan"])
            .output()
            .await
            .map_err(|e| SlapperError::Network(format!("iwlist scan failed: {}", e)))?;

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
            scan_duration_secs: duration_secs,
            recommendations,
        })
    }

    #[cfg(feature = "wireless")]
    fn parse_scan_output(&self, output: &str) -> Vec<WirelessNetwork> {
        let mut networks = Vec::new();
        let mut current_ssid = None;
        let mut current_bssid = None;
        let mut current_channel = 0u8;
        let mut current_security = SecurityType::Unknown;
        let mut current_signal = -100i32;
        let mut current_auth_suite: Option<String> = None;

        for line in output.lines() {
            let line = line.trim();

            if line.starts_with("Cell ") {
                current_auth_suite = None;
                if let (Some(ssid), Some(bssid)) = (current_ssid.take(), current_bssid.take()) {
                    networks.push(WirelessNetwork {
                        ssid,
                        bssid,
                        channel: current_channel,
                        security_type: current_security,
                        signal_strength: current_signal,
                        last_seen: String::new(),
                    });
                }
            }

            if line.contains("Address:") {
                current_bssid = Some(
                    line.split("Address:")
                        .nth(1)
                        .unwrap_or("")
                        .trim()
                        .to_string(),
                );
            }

            if line.starts_with("ESSID:") {
                current_ssid = Some(
                    line.split("ESSID:\"")
                        .nth(1)
                        .unwrap_or("")
                        .trim_matches('"')
                        .to_string(),
                );
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
                if let Some(level) = line.split("Signal level:").nth(1) {
                    let level_str = level.split_whitespace().next().unwrap_or("-100");
                    current_signal = level_str.parse().unwrap_or(-100);
                }
            }

            if line.contains("WPA2") || line.contains("WPA3") {
                current_security = if line.contains("WPA3") {
                    SecurityType::WPA3
                } else {
                    SecurityType::WPA2
                };
            } else if line.contains("WPA") {
                current_security = SecurityType::WPA;
            } else if line.contains("WEP") {
                current_security = SecurityType::WEP;
            } else if line.contains("Encryption key:") && line.contains("off") {
                current_security = SecurityType::Open;
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
            networks.push(WirelessNetwork {
                ssid,
                bssid,
                channel: current_channel,
                security_type: current_security,
                signal_strength: current_signal,
                last_seen: String::new(),
            });
        }

        networks
    }

    pub fn analyze_networks(&self, networks: &[WirelessNetwork]) -> Vec<WirelessVulnerability> {
        let mut vulnerabilities = Vec::new();

        for network in networks {
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
                _ => {}
            }
        }

        vulnerabilities
    }

    fn generate_recommendations(networks: &[WirelessNetwork]) -> Vec<String> {
        let mut recommendations = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for network in networks {
            if seen.insert(network.security_type) {
                match network.security_type {
                    SecurityType::WEP => {
                        recommendations.push("Upgrade from WEP to WPA2 or WPA3 immediately — WEP is trivially cracked".to_string());
                    }
                    SecurityType::Open => {
                        recommendations.push("Enable WPA2 or WPA3 encryption on open networks".to_string());
                    }
                    SecurityType::WPA => {
                        recommendations.push("Upgrade from WPA to WPA2 or WPA3 — WPA has known TKIP vulnerabilities".to_string());
                    }
                    SecurityType::Unknown => {
                        recommendations.push("Verify wireless security configuration — security type could not be determined".to_string());
                    }
                    _ => {}
                }
            }
        }

        if recommendations.is_empty() && !networks.is_empty() {
            recommendations.push("All detected networks use strong encryption (WPA2/WPA3/Enterprise)".to_string());
        }
        if networks.is_empty() {
            recommendations.push("No wireless networks were detected during the scan".to_string());
        }

        recommendations
    }
}

impl Default for WirelessScanner {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

pub fn to_scan_report_data(result: &WirelessScanResult) -> crate::output::convert::ScanReportData {
    use crate::output::convert::WirelessNetworkReportData;

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
        })
        .collect();

    crate::output::convert::ScanReportData {
        target: result.interface.clone(),
        scan_type: "wireless".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        findings: Vec::new(),
        open_ports: Vec::new(),
        services: Vec::new(),
        duration_ms: result.scan_duration_secs * 1000,
        wireless_networks,
    }
}

pub async fn run_cli(args: crate::cli::WirelessArgs, _config: &crate::config::SlapperConfig) -> Result<()> {
    let scanner = WirelessScanner::new()?.with_interface(args.interface.clone());

    if !args.quiet {
        eprintln!("Scanning wireless networks on {}...", args.interface);
    }

    let result = scanner.scan(10).await?;

    let output = if args.json {
        serde_json::to_string_pretty(&result)?
    } else {
        let mut buf = String::new();
        buf.push_str(&format!("Wireless Scan Results - Interface: {}\n", result.interface));
        buf.push_str(&format!("Networks found: {}\n\n", result.networks.len()));

        for (i, network) in result.networks.iter().enumerate() {
            buf.push_str(&format!("  {}. {}\n", i + 1, network.ssid));
            buf.push_str(&format!("     BSSID:    {}\n", network.bssid));
            buf.push_str(&format!("     Channel:  {}\n", network.channel));
            buf.push_str(&format!("     Security: {}\n", network.security_type.as_str()));
            buf.push_str(&format!("     Signal:   {} dBm\n", network.signal_strength));
            buf.push_str(&format!("     Last seen: {}\n", network.last_seen));
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
        let scanner = WirelessScanner::new();
        assert!(scanner.is_ok());
    }

    #[test]
    fn test_network_analysis() {
        let scanner = WirelessScanner::new().unwrap();
        let networks = vec![
            WirelessNetwork {
                ssid: "OpenNetwork".to_string(),
                bssid: "00:11:22:33:44:55".to_string(),
                channel: 6,
                security_type: SecurityType::Open,
                signal_strength: -50,
                last_seen: String::new(),
            },
            WirelessNetwork {
                ssid: "SecureNetwork".to_string(),
                bssid: "00:11:22:33:44:66".to_string(),
                channel: 11,
                security_type: SecurityType::WPA3,
                signal_strength: -60,
                last_seen: String::new(),
            },
        ];

        let vulns = scanner.analyze_networks(&networks);
        assert_eq!(vulns.len(), 1);
        assert_eq!(vulns[0].vulnerability_type, "Open Network");
    }
}
