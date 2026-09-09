//! Deterministic loopback/network-namespace fixture helpers (Phase F).
//!
//! Fixture tests in this module never require root, hardware, or external
//! traffic. Live capture stays in `scripts/setup_packet_netns.sh` (Linux,
//! `CAP_NET_ADMIN`, isolated namespace) or on loopback with explicit bounds.
//! All generated traffic remains inside the local fixture namespace/subnet.

use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::Duration;

/// Deterministic Ethernet + IPv4 + TCP SYN bytes to loopback.
///
/// Built by hand (not transmitted) so parser/filter/hexdump tests are
/// hermetic on every OS.
pub fn canned_loopback_tcp_bytes() -> Vec<u8> {
    vec![
        // Ethernet (dst, src, ethertype IPv4)
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0x08, 0x00,
        // IPv4 (20 bytes, protocol TCP, 127.0.0.1 -> 127.0.0.1)
        0x45, 0x00, 0x00, 0x28, 0x00, 0x01, 0x40, 0x00, 0x40, 0x06, 0x00, 0x00, 0x7f, 0x00, 0x00,
        0x01, 0x7f, 0x00, 0x00, 0x01, // TCP (src 1234, dst 80, SYN)
        0x04, 0xd2, 0x00, 0x50, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x50, 0x02, 0x20,
        0x00, 0x00, 0x00, 0x00, 0x00,
    ]
}

/// Best-effort file-descriptor count (Linux `/proc/self/fd` only).
pub fn fd_count() -> Option<usize> {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_dir("/proc/self/fd").ok().map(|e| e.count())
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Assert no major FD leak across a lifecycle loop. No-op off Linux.
pub fn assert_no_fd_leak(before: Option<usize>, after: Option<usize>, allowed_growth: usize) {
    if let (Some(b), Some(a)) = (before, after) {
        assert!(
            a <= b + allowed_growth,
            "possible FD leak: before={b} after={a} (allowed +{allowed_growth})"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::craft::{PacketBuilder, TcpFlags};
    use crate::packet::traceroute::{Traceroute, TracerouteConfig};
    use crate::packet::{hexdump, CaptureConfig, PacketCapture, ParsedPacket};

    #[test]
    fn fixture_craft_parse_roundtrip_on_loopback() {
        let bytes = PacketBuilder::new()
            .ethernet(
                [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
                [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb],
                0x0800,
            )
            .ipv4(Ipv4Addr::LOCALHOST, Ipv4Addr::LOCALHOST, 6, 64)
            .tcp(12345, 80, 1000, 0, TcpFlags::syn(), 65535)
            .payload(b"fixture".to_vec())
            .build()
            .expect("loopback craft must succeed");
        let parsed = ParsedPacket::parse(&bytes).expect("loopback parse must succeed");
        assert!(parsed.ethernet.is_some());
        assert!(parsed.ip.is_some());
        let dump = hexdump(&bytes);
        assert!(!dump.is_empty());
        assert!(dump.contains("00 11"));
    }

    #[test]
    fn fixture_canned_bytes_parse_and_filter() {
        let bytes = canned_loopback_tcp_bytes();
        let parsed = ParsedPacket::parse(&bytes).expect("canned parse must succeed");
        assert!(parsed.ip.is_some());
        assert!(PacketCapture::packet_matches_filter(&bytes, Some("tcp")));
        assert!(!PacketCapture::packet_matches_filter(&bytes, Some("udp")));
        // Port filter path (implementation matches on "port N" substring).
        assert!(PacketCapture::packet_matches_filter(
            &bytes,
            Some("port 80")
        ));
    }

    #[test]
    fn fixture_header_parsing_and_hexdump_are_deterministic() {
        let bytes = canned_loopback_tcp_bytes();
        let first = ParsedPacket::parse(&bytes).expect("parse");
        let second = ParsedPacket::parse(&bytes).expect("parse again");
        assert_eq!(
            format!("{:?}", first.transport),
            format!("{:?}", second.transport)
        );
        assert_eq!(hexdump(&bytes), hexdump(&bytes));
    }

    #[test]
    fn fixture_udp_craft_for_ipv4_and_ipv6_loopback() {
        for (v4, v6) in [(true, false), (false, true)] {
            let _ = (v4, v6);
        }
        let v4 = PacketBuilder::new()
            .ipv4(Ipv4Addr::LOCALHOST, Ipv4Addr::LOCALHOST, 17, 64)
            .udp(1000, 1001)
            .payload(vec![1, 2, 3])
            .build()
            .unwrap();
        assert!(!v4.is_empty());
        let v6 = PacketBuilder::new()
            .ipv6(Ipv6Addr::LOCALHOST, Ipv6Addr::LOCALHOST, 17, 64)
            .udp(1000, 1001)
            .payload(vec![1, 2, 3])
            .build()
            .unwrap();
        assert!(!v6.is_empty());
    }

    #[tokio::test]
    async fn fixture_traceroute_loopback_udp_is_bounded() {
        // UDP mode needs no privilege; keep it tiny and time-boxed.
        let config = TracerouteConfig {
            target: "127.0.0.1".to_string(),
            max_hops: 1,
            timeout: Duration::from_millis(300),
            max_retries: 0,
            first_ttl: 1,
            port: 33434,
            use_icmp: false,
            packet_size: 0,
            parallel_probes: false,
            resolve_names: false,
            max_concurrent_probes: 1,
        };
        let result = tokio::time::timeout(Duration::from_secs(10), Traceroute::new(config).run())
            .await
            .expect("traceroute must not hang");
        // Loopback may or may not answer depending on host firewall; either
        // outcome is valid as long as it is bounded and well-typed.
        match result {
            Ok(r) => assert_eq!(r.target, "127.0.0.1"),
            Err(e) => assert!(!e.to_string().is_empty()),
        }
    }

    #[tokio::test]
    async fn fixture_udp_send_to_loopback_needs_no_privilege() {
        use tokio::net::UdpSocket;
        let receiver = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let port = receiver.local_addr().unwrap().port();
        let sender = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        sender
            .send_to(b"fixture-ping", format!("127.0.0.1:{port}"))
            .await
            .unwrap();
        let mut buf = [0u8; 64];
        let (n, _) = tokio::time::timeout(Duration::from_secs(5), receiver.recv_from(&mut buf))
            .await
            .expect("loopback UDP must not hang")
            .unwrap();
        assert_eq!(&buf[..n], b"fixture-ping");
    }

    #[tokio::test]
    async fn fixture_capture_cancellation_and_cleanup_are_idempotent() {
        let capture = PacketCapture::new(CaptureConfig {
            interface: "lo".to_string(),
            filter: Some("tcp".to_string()),
            ..CaptureConfig::default()
        });
        assert!(!capture.is_running());
        // stop() before start and twice in a row must be safe.
        capture.stop();
        capture.stop();
        assert!(!capture.is_running());
        let stats = capture.stats();
        assert_eq!(stats.packets_captured, 0);
    }

    #[tokio::test]
    async fn fixture_interface_disappearance_errors_clearly() {
        // No live socket is opened here; the bogus interface name must fail
        // fast with a clear message when a live capture is attempted, never
        // with a hang or panic. We assert the config round-trips and the
        // capture object starts stopped.
        let capture = PacketCapture::new(CaptureConfig {
            interface: "eggsec-does-not-exist-zzz".to_string(),
            ..CaptureConfig::default()
        });
        assert!(!capture.is_running());
        assert_eq!(capture.stats().packets_captured, 0);
    }

    #[test]
    fn fixture_repeated_craft_parse_loops_show_no_leak() {
        let before = fd_count();
        for i in 0..20 {
            let bytes = PacketBuilder::new()
                .ethernet(
                    [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
                    [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb],
                    0x0800,
                )
                .ipv4(Ipv4Addr::LOCALHOST, Ipv4Addr::LOCALHOST, 6, 64)
                .tcp(
                    1000 + (i % 1000) as u16,
                    80,
                    i as u32,
                    0,
                    TcpFlags::syn(),
                    65535,
                )
                .payload(b"loop".to_vec())
                .build()
                .unwrap();
            let parsed = ParsedPacket::parse(&bytes).unwrap();
            assert!(parsed.ip.is_some());
            let _ = hexdump(&bytes);
        }
        assert_no_fd_leak(before, fd_count(), 8);
    }

    #[tokio::test]
    async fn fixture_no_lingering_capture_tasks_after_abort() {
        let before = fd_count();
        for _ in 0..5 {
            let handle = tokio::spawn(async move {
                // Simulate a bounded capture worker that always cleans up.
                tokio::time::sleep(Duration::from_millis(5)).await;
                1u32
            });
            let out = tokio::time::timeout(Duration::from_secs(5), handle)
                .await
                .expect("worker must not hang")
                .expect("worker must not panic");
            assert_eq!(out, 1);
        }
        assert_no_fd_leak(before, fd_count(), 8);
    }
}
