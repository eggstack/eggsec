//! Active wireless attack primitives for lab-only defense validation.
//!
//! **This module implements active attack capabilities** (deauthentication,
//! disassociation, etc.) that transmit 802.11 management frames. These
//! operations are:
//!
//! - Gated behind the `wireless-advanced` feature flag
//! - Classified as high-risk operations requiring explicit authorization
//! - Restricted to lab/defense-validation environments
//! - Subject to packet budgets and rate limits
//!
//! # Safety Requirements
//!
//! - Root or CAP_NET_ADMIN privileges required
//! - Monitor-mode capable wireless interface required
//! - Explicit `--allow-active-wireless` flag required (or policy confirmation)
//! - Use **only on networks you own or have explicit written authorization to test**
//!
//! See `docs/WIRELESS.md` and `docs/SAFETY.md` for full safety guidance.

pub mod attacks;

use serde::{Deserialize, Serialize};

use crate::types::Severity;

/// Result of an active wireless attack operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveWirelessAttackResult {
    /// Wireless interface used for the attack
    pub interface: String,
    /// Type of attack performed (e.g., "deauth", "disassoc")
    pub attack_type: String,
    /// Target BSSID (AP MAC address) if specified
    pub target_bssid: Option<String>,
    /// Target client MAC address if specified
    pub target_client: Option<String>,
    /// Total frames transmitted
    pub frames_sent: u64,
    /// Duration of the attack in seconds
    pub duration_secs: u64,
    /// Whether this was a dry run (no frames actually sent)
    pub dry_run: bool,
    /// Findings generated from the attack
    pub findings: Vec<ActiveWirelessFinding>,
    /// Raw output summary (e.g., hexdump of frames, capture file path)
    pub raw_output: Option<String>,
    /// Security recommendations
    pub recommendations: Vec<String>,
}

/// A finding produced by an active wireless attack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveWirelessFinding {
    /// Attack type that produced this finding (e.g., "deauth")
    pub attack_type: String,
    /// Severity of the finding
    pub severity: Severity,
    /// Human-readable description
    pub description: String,
    /// Evidence (e.g., "Sent 47 deauth frames to BSSID AA:BB:CC:DD:EE:FF")
    pub evidence: String,
    /// Recommended remediation
    pub remediation: String,
}

/// Configuration for an active wireless attack.
#[derive(Debug, Clone)]
pub struct ActiveAttackConfig {
    /// Wireless interface in monitor mode
    pub interface: String,
    /// Target BSSID (AP MAC)
    pub bssid: Option<[u8; 6]>,
    /// Target client MAC
    pub client: Option<[u8; 6]>,
    /// 802.11 reason code
    pub reason_code: u16,
    /// Maximum frames to send (budget)
    pub max_frames: u64,
    /// Rate limit (frames per second)
    pub frames_per_second: u64,
    /// Dry run mode
    pub dry_run: bool,
}

impl ActiveAttackConfig {
    /// Parse a MAC address string "AA:BB:CC:DD:EE:FF" into bytes.
    pub fn parse_mac(s: &str) -> Option<[u8; 6]> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 6 {
            return None;
        }
        let mut bytes = [0u8; 6];
        for (i, part) in parts.iter().enumerate() {
            bytes[i] = u8::from_str_radix(part, 16).ok()?;
        }
        Some(bytes)
    }

    /// Format a MAC address bytes to string.
    pub fn format_mac(mac: &[u8; 6]) -> String {
        mac.iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(":")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mac_valid() {
        let mac = ActiveAttackConfig::parse_mac("AA:BB:CC:DD:EE:FF");
        assert_eq!(mac, Some([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]));
    }

    #[test]
    fn test_parse_mac_lowercase() {
        let mac = ActiveAttackConfig::parse_mac("aa:bb:cc:dd:ee:ff");
        assert_eq!(mac, Some([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]));
    }

    #[test]
    fn test_parse_mac_invalid() {
        assert!(ActiveAttackConfig::parse_mac("not-a-mac").is_none());
        assert!(ActiveAttackConfig::parse_mac("AA:BB:CC:DD:EE").is_none());
        assert!(ActiveAttackConfig::parse_mac("AA:BB:CC:DD:EE:FF:00").is_none());
    }

    #[test]
    fn test_format_mac() {
        let mac = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        assert_eq!(ActiveAttackConfig::format_mac(&mac), "AA:BB:CC:DD:EE:FF");
    }

    #[test]
    fn test_attack_result_serde_roundtrip() {
        let result = ActiveWirelessAttackResult {
            interface: "wlan0mon".to_string(),
            attack_type: "deauth".to_string(),
            target_bssid: Some("AA:BB:CC:DD:EE:FF".to_string()),
            target_client: Some("11:22:33:44:55:66".to_string()),
            frames_sent: 50,
            duration_secs: 5,
            dry_run: true,
            findings: vec![ActiveWirelessFinding {
                attack_type: "deauth".to_string(),
                severity: Severity::High,
                description: "Test deauth frames sent".to_string(),
                evidence: "Sent 50 deauth frames".to_string(),
                remediation: "Verify WIPS logged event".to_string(),
            }],
            raw_output: None,
            recommendations: vec!["Check WIPS logs".to_string()],
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ActiveWirelessAttackResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.attack_type, "deauth");
        assert_eq!(deserialized.frames_sent, 50);
        assert!(deserialized.dry_run);
        assert_eq!(deserialized.findings.len(), 1);
    }

    #[test]
    fn test_finding_serde_roundtrip() {
        let finding = ActiveWirelessFinding {
            attack_type: "deauth".to_string(),
            severity: Severity::High,
            description: "Deauth flood detected".to_string(),
            evidence: "100 frames to BSSID AA:BB:CC:DD:EE:FF".to_string(),
            remediation: "Enable 802.11w PMF".to_string(),
        };
        let json = serde_json::to_string(&finding).unwrap();
        let deserialized: ActiveWirelessFinding = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.severity, Severity::High);
    }
}
