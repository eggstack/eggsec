//! Safe fixture strategy for wireless tests (Phase F).
//!
//! Three tiers, enforced by construction:
//!
//! 1. **Unit** (`wireless::` tests): pure parser/state, no hardware.
//! 2. **Passive fixture** (this module): canned `iwlist` output and network
//!    lists; no interface, no privilege, no RF.
//! 3. **Manual RF** (lab procedure only): real scans and frame injection are
//!    never part of routine CI. Active frame builders are tested dry-run only.
//!
//! Real RF transmission remains an explicitly documented maintainer procedure
//! and is not required for release readiness.

use super::{SecurityType, WirelessNetwork};

/// Deterministic canned `iwlist scan` output: one open, one WPA2, one
/// WPA2/WPA3 transition with WPS. No hardware required.
pub fn sample_iwlist_output() -> &'static str {
    "Cell 01 - Address: 00:11:22:33:44:55\n\
     ESSID:\"LabOpen\"\n\
     Channel:6\n\
     Signal level=-60 dBm\n\
     Encryption key:off\n\
     \n\
     Cell 02 - Address: 00:11:22:33:44:66\n\
     ESSID:\"LabWPA2\"\n\
     Channel:11\n\
     Signal level=-55 dBm\n\
     Encryption key:on\n\
     WPA2\n\
     \n\
     Cell 03 - Address: 00:11:22:33:44:77\n\
     ESSID:\"LabTransition\"\n\
     Channel:36\n\
     Signal level=-70 dBm\n\
     Encryption key:on\n\
     WPA2/WPA3 transition\n\
     WPS\n"
}

/// Deterministic canned networks: same SSID on two BSSIDs triggers the
/// passive rogue/Evil-Twin heuristic without any RF.
pub fn sample_networks_with_rogue_candidate() -> Vec<WirelessNetwork> {
    vec![
        WirelessNetwork {
            ssid: "LabCorp".to_string(),
            bssid: "00:11:22:33:44:55".to_string(),
            channel: 6,
            security_type: SecurityType::WPA2,
            signal_strength: -55,
            last_seen: "2026-01-01T00:00:00Z".to_string(),
            wps_enabled: false,
            is_hidden: false,
            transition_mode: false,
        },
        WirelessNetwork {
            ssid: "LabCorp".to_string(),
            bssid: "AA:BB:CC:DD:EE:FF".to_string(),
            channel: 6,
            security_type: SecurityType::WPA2,
            signal_strength: -60,
            last_seen: "2026-01-01T00:00:00Z".to_string(),
            wps_enabled: false,
            is_hidden: false,
            transition_mode: false,
        },
        WirelessNetwork {
            ssid: "LabGuest".to_string(),
            bssid: "11:22:33:44:55:66".to_string(),
            channel: 11,
            security_type: SecurityType::Open,
            signal_strength: -65,
            last_seen: "2026-01-01T00:00:00Z".to_string(),
            wps_enabled: false,
            is_hidden: false,
            transition_mode: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::super::{WirelessScanner, WirelessVulnerability};
    use super::*;

    #[test]
    fn fixture_parser_covers_open_wpa2_and_transition() {
        let scanner = WirelessScanner::new();
        let networks = scanner.parse_scan_output(sample_iwlist_output());
        assert_eq!(networks.len(), 3, "canned iwlist must yield 3 cells");
        assert_eq!(networks[0].security_type, SecurityType::Open);
        assert_eq!(networks[1].security_type, SecurityType::WPA2);
        assert!(
            networks[2].transition_mode,
            "third cell must flag transition"
        );
    }

    #[test]
    fn fixture_rogue_heuristic_triggers_on_canned_networks() {
        let networks = sample_networks_with_rogue_candidate();
        let vulns: Vec<WirelessVulnerability> = WirelessScanner::analyze_networks(&networks, None);
        assert!(
            vulns.iter().any(|v| v.vulnerability_type.contains("Rogue")),
            "same-SSID multi-BSSID fixture must raise a rogue candidate"
        );
    }

    #[test]
    fn fixture_known_good_suppresses_canned_rogue() {
        use std::collections::HashSet;
        let networks = sample_networks_with_rogue_candidate();
        let mut known_good = HashSet::new();
        known_good.insert("LabCorp".to_string());
        let vulns = WirelessScanner::analyze_networks(&networks, Some(&known_good));
        assert!(
            !vulns.iter().any(|v| v.vulnerability_type.contains("Rogue")),
            "known-good SSID must suppress the heuristic"
        );
    }

    #[test]
    fn fixture_passive_needs_no_advanced_feature_or_privilege() {
        // Passive analysis is pure: no socket, no interface, no privilege.
        let networks = sample_networks_with_rogue_candidate();
        let vulns = WirelessScanner::analyze_networks(&networks, None);
        assert!(!vulns.is_empty());
    }

    #[cfg(feature = "wireless-advanced")]
    #[test]
    fn fixture_active_frames_are_dry_run_only_and_bounded() {
        use super::super::active::attacks::deauth::{build_deauth_frame, build_disassoc_frame};
        let bssid = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
        let client = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        // Crafted fixture frames: byte-exact, never transmitted.
        let deauth = build_deauth_frame(&bssid, Some(&client), 7);
        let disassoc = build_disassoc_frame(&bssid, Some(&client), 7);
        assert_eq!(deauth.len(), 34);
        assert_eq!(disassoc.len(), 34);
        // Frame control distinguishes the two (0xC000 vs 0xA000, little-endian).
        assert_eq!(&deauth[8..10], &[0x00, 0xC0]);
        assert_eq!(&disassoc[8..10], &[0x00, 0xA0]);
    }

    #[cfg(feature = "wireless-advanced")]
    #[test]
    fn fixture_repeated_frame_builds_show_no_regression() {
        use super::super::active::attacks::deauth::build_deauth_frame;
        let bssid = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
        let client = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        for reason in 1u16..=9 {
            let frame = build_deauth_frame(&bssid, Some(&client), reason);
            assert_eq!(frame.len(), 34);
            assert_eq!(u16::from_le_bytes([frame[32], frame[33]]), reason);
        }
        for _ in 0..20 {
            let frame = build_deauth_frame(&bssid, Some(&client), 7);
            assert_eq!(frame.len(), 34);
        }
    }
}
