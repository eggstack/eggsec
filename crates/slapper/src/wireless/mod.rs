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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
        let networks = self.parse_scan_output(&stdout);

        Ok(WirelessScanResult {
            interface: interface.clone(),
            networks,
            scan_duration_secs: duration_secs,
            recommendations: Vec::new(),
        })
    }

    #[cfg(not(feature = "wireless"))]
    pub async fn scan(&self, _duration_secs: u64) -> Result<WirelessScanResult> {
        Err(SlapperError::Config(
            "Wireless scanning requires the `wireless` feature".to_string(),
        ))
    }

    #[cfg(feature = "wireless")]
    fn parse_scan_output(&self, output: &str) -> Vec<WirelessNetwork> {
        let mut networks = Vec::new();
        let mut current_ssid = None;
        let mut current_bssid = None;
        let mut current_channel = 0u8;
        let mut current_security = SecurityType::Unknown;
        let mut current_signal = -100i32;

        for line in output.lines() {
            let line = line.trim();

            if line.starts_with("Cell ") {
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

    #[cfg(not(feature = "wireless"))]
    fn parse_scan_output(&self, _output: &str) -> Vec<WirelessNetwork> {
        Vec::new()
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
                _ => {}
            }
        }

        vulnerabilities
    }
}

impl Default for WirelessScanner {
    fn default() -> Self {
        Self::new().unwrap()
    }
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
