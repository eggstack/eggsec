//! Hermetic tests for the whole decision path.
//!
//! Every test here runs with no network, no privilege, and no clock. The
//! scanner's correctness lives entirely in parsing bytes and bookkeeping
//! against an explicit `now`, so none of it needs a real network to verify.
//!
//! Packets here are hand-built rather than captured, so a test failure names
//! the exact structure that broke rather than "the fixture changed".

use super::*;
use std::net::Ipv4Addr;

// ── packet construction ──

/// Build a bare ICMP error: 8-byte ICMP header + quoted IP + quoted UDP.
fn bare_icmp(
    icmp_type: u8,
    code: u8,
    src: Ipv4Addr,
    src_port: u16,
    dst: Ipv4Addr,
    dst_port: u16,
) -> Vec<u8> {
    let mut out = vec![icmp_type, code, 0, 0, 0, 0, 0, 0];
    out.extend(quoted_ip_udp(src, src_port, dst, dst_port));
    out
}

/// Wrap an ICMP message in the IPv4 header that macOS/BSD datagram ICMP sockets
/// deliver (a Linux raw socket with IPPROTO_ICMPV4 omits it).
fn with_ipv4_header(icmp: &[u8]) -> Vec<u8> {
    // ver/ihl, tos, totlen, id, flags/frag, ttl, proto, checksum(2) = 12 bytes
    let mut out = vec![
        0x45, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x01, 0x00, 0x00,
    ];
    out.extend_from_slice(&[127, 0, 0, 1]); // src
    out.extend_from_slice(&[127, 0, 0, 1]); // dst
    out.extend_from_slice(icmp);
    let total = u16::try_from(out.len()).unwrap_or(0);
    out[2..4].copy_from_slice(&total.to_be_bytes());
    out
}

fn quoted_ip_udp(src: Ipv4Addr, src_port: u16, dst: Ipv4Addr, dst_port: u16) -> Vec<u8> {
    let mut qip = vec![
        0x45, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 17, 0x00, 0x00,
    ];
    qip.extend_from_slice(&src.octets());
    qip.extend_from_slice(&dst.octets());
    let mut udp = Vec::new();
    udp.extend_from_slice(&src_port.to_be_bytes());
    udp.extend_from_slice(&dst_port.to_be_bytes());
    udp.extend_from_slice(&8u16.to_be_bytes()); // length = header only
    udp.extend_from_slice(&0u16.to_be_bytes()); // checksum placeholder
    qip.extend_from_slice(&udp);
    qip
}

// ── ICMP parsing ──

#[test]
fn parses_port_unreachable_and_recovers_the_correlation_key() {
    let pkt = bare_icmp(
        3,
        3,
        Ipv4Addr::new(10, 0, 0, 5),
        57115,
        Ipv4Addr::new(10, 0, 0, 9),
        53,
    );
    let err = parse_icmp_error(&pkt).expect("port-unreachable parses");
    assert_eq!(err.icmp_type, 3);
    assert_eq!(err.code, 3);
    assert_eq!(err.probe_src, Ipv4Addr::new(10, 0, 0, 5));
    assert_eq!(err.probe_src_port, 57115);
    assert_eq!(err.probe_dst, Ipv4Addr::new(10, 0, 0, 9));
    assert_eq!(err.probe_dst_port, 53);
    assert_eq!(err.correlation_key(), (Ipv4Addr::new(10, 0, 0, 5), 57115));
}

#[test]
fn parses_both_delivery_shapes_identically() {
    // The same error with and without the IPv4 header macOS adds. If the
    // disambiguation in `split_icmp` regresses, one of these two fails.
    let bare = bare_icmp(
        3,
        3,
        Ipv4Addr::new(10, 0, 0, 5),
        40000,
        Ipv4Addr::new(10, 0, 0, 9),
        161,
    );
    let wrapped = with_ipv4_header(&bare);
    assert_ne!(bare, wrapped, "wrapper must actually change the bytes");
    assert_eq!(
        parse_icmp_error(&bare).expect("bare parses"),
        parse_icmp_error(&wrapped).expect("wrapped parses"),
    );
}

#[test]
fn quoted_ipv4_options_are_measured_from_ihl_not_assumed() {
    // IHL=6 means a 24-byte header. Hardcoding 20 would read the IHL nibble
    // and the TOS byte as the quoted UDP ports, producing a wrong key that
    // still looks plausible.
    let mut quoted = quoted_ip_udp(
        Ipv4Addr::new(10, 0, 0, 5),
        40000,
        Ipv4Addr::new(10, 0, 0, 9),
        161,
    );
    // Options fill the space between the 20-byte fixed header and IHL*4, so
    // they are inserted at offset 20 -- splicing them earlier would shift the
    // protocol, address and UDP-header bytes out from under the parser.
    quoted[0] = 0x46; // version 4, IHL 6 => 24-byte header
    quoted.splice(20..20, [0u8; 4]);
    let mut pkt = vec![3u8, 3, 0, 0, 0, 0, 0, 0];
    pkt.extend_from_slice(&quoted);

    let err = parse_icmp_error(&pkt).expect("options-bearing quote parses");
    assert_eq!(
        err.probe_src_port, 40000,
        "sport must come from the real UDP header"
    );
    assert_eq!(err.probe_dst_port, 161);
}

#[test]
fn rejects_non_udp_quotes() {
    // A TCP probe's error must never be attributed to a UDP port.
    let mut quoted = quoted_ip_udp(
        Ipv4Addr::new(10, 0, 0, 5),
        40000,
        Ipv4Addr::new(10, 0, 0, 9),
        161,
    );
    quoted[9] = 6; // TCP
    let mut pkt = vec![3u8, 3, 0, 0, 0, 0, 0, 0];
    pkt.extend_from_slice(&quoted);
    assert_eq!(parse_icmp_error(&pkt), Err(ParseFailure::NotUdp));
}

#[test]
fn rejects_truncated_quoted_datagram() {
    // RFC 792 guarantees 8 bytes of original payload; for UDP that is the
    // whole header. Fewer cannot be correlated.
    let mut pkt = vec![3u8, 3, 0, 0, 0, 0, 0, 0];
    pkt.extend_from_slice(&quoted_ip_udp(
        Ipv4Addr::new(10, 0, 0, 5),
        40000,
        Ipv4Addr::new(10, 0, 0, 9),
        161,
    ));
    pkt.truncate(pkt.len() - 6);
    assert_eq!(
        parse_icmp_error(&pkt),
        Err(ParseFailure::QuotedDatagramTruncated)
    );
}

#[test]
fn rejects_non_error_icmp_types() {
    // An echo reply is well-formed ICMP that is not an error about our probe.
    let mut pkt = vec![0u8, 0, 0, 0, 0, 0, 0, 0];
    pkt.extend_from_slice(&quoted_ip_udp(
        Ipv4Addr::new(10, 0, 0, 5),
        40000,
        Ipv4Addr::new(10, 0, 0, 9),
        161,
    ));
    assert_eq!(parse_icmp_error(&pkt), Err(ParseFailure::NotAnErrorType));
}

#[test]
fn rejects_empty_and_undersized_input() {
    assert_eq!(parse_icmp_error(&[]), Err(ParseFailure::TooShort));
    assert_eq!(parse_icmp_error(&[3, 3]), Err(ParseFailure::TooShort));
}

#[test]
fn rejects_quoted_header_whose_length_cannot_cover_itself() {
    let mut quoted = quoted_ip_udp(
        Ipv4Addr::new(10, 0, 0, 5),
        40000,
        Ipv4Addr::new(10, 0, 0, 9),
        161,
    );
    quoted[2..4].copy_from_slice(&8u16.to_be_bytes()); // total_len < ihl
    let mut pkt = vec![3u8, 3, 0, 0, 0, 0, 0, 0];
    pkt.extend_from_slice(&quoted);
    assert_eq!(
        parse_icmp_error(&pkt),
        Err(ParseFailure::MalformedQuotedHeader)
    );
}

// ── classification ──

#[test]
fn classification_lattice() {
    let mk = |t, c| IcmpError {
        icmp_type: t,
        code: c,
        probe_src: Ipv4Addr::LOCALHOST,
        probe_src_port: 1,
        probe_dst: Ipv4Addr::LOCALHOST,
        probe_dst_port: 2,
    };
    assert_eq!(
        classify::classify(&mk(3, 3)),
        (UdpPortState::Closed, Evidence::PortUnreachable)
    );
    assert_eq!(
        classify::classify(&mk(3, 9)),
        (UdpPortState::Filtered, Evidence::AdminProhibited)
    );
    assert_eq!(
        classify::classify(&mk(11, 0)),
        (UdpPortState::Filtered, Evidence::TimeExceeded)
    );
    // Net/host unreachable is a statement about the host, and is reported as
    // unproven rather than as a clean per-port verdict.
    for code in [0, 1] {
        let (state, ev) = classify::classify(&mk(3, code));
        assert_eq!(state, UdpPortState::Filtered);
        assert_eq!(ev, Evidence::NetOrHostUnreachable);
        assert!(
            !ev.is_proof(),
            "net/host unreachable must not count as proof"
        );
    }
}

#[test]
fn silence_is_never_reported_as_open() {
    // The single most important property of this crate.
    for sweeps in [1, 2, 5] {
        let (state, ev) = classify::classify_silence(sweeps);
        assert_eq!(state, UdpPortState::OpenFiltered);
        assert_eq!(ev, Evidence::Silence { sweeps });
        assert!(!ev.is_proof(), "silence must never be proof");
    }
}

#[test]
fn only_positive_evidence_counts_as_proof() {
    assert!(Evidence::PortUnreachable.is_proof());
    assert!(Evidence::AdminProhibited.is_proof());
    assert!(Evidence::TimeExceeded.is_proof());
    assert!(Evidence::ConfirmedResponse { bytes: 1 }.is_proof());
    assert!(!Evidence::Silence { sweeps: 1 }.is_proof());
    assert!(!Evidence::NetOrHostUnreachable.is_proof());
}

// ── the host-down trap ──

#[test]
fn host_state_requires_correlated_evidence_to_claim_liveness() {
    // An orphan may be another process's traffic, so it cannot prove the host
    // is up — only that the socket saw something.
    assert_eq!(HostState::derive(0, 0), HostState::Unresponsive);
    assert_eq!(HostState::derive(0, 5), HostState::Indeterminate);
    assert_eq!(HostState::derive(1, 0), HostState::Up);
    assert_eq!(HostState::derive(5, 5), HostState::Up);
}

#[test]
fn unresponsive_host_makes_per_port_results_meaningless() {
    // This is the trap: a dead host reads exactly like an all-filtered host,
    // so a caller must be able to detect that the port list is not a finding.
    assert!(!HostState::Unresponsive.ports_are_meaningful());
    assert!(!HostState::Indeterminate.ports_are_meaningful());
    assert!(HostState::Up.ports_are_meaningful());
}

// ── correlation ──

#[test]
fn correlation_matches_the_right_port() {
    let t0 = std::time::Instant::now();
    let mut table = CorrelationTable::new();
    let a = Ipv4Addr::new(10, 0, 0, 5);
    let b = Ipv4Addr::new(10, 0, 0, 6);
    table
        .register(a, 40001, 53, t0, std::time::Duration::from_secs(1))
        .unwrap();
    table
        .register(a, 40002, 161, t0, std::time::Duration::from_secs(1))
        .unwrap();
    table
        .register(b, 40001, 445, t0, std::time::Duration::from_secs(1))
        .unwrap();

    // Same source port, different address: the address is part of the key.
    assert_eq!(
        table.correlate((a, 40001)),
        Correlation::Matched { port: 53 }
    );
    assert_eq!(
        table.correlate((a, 40002)),
        Correlation::Matched { port: 161 }
    );
    assert_eq!(
        table.correlate((b, 40001)),
        Correlation::Matched { port: 445 }
    );
    assert_eq!(table.in_flight(), 0);
    assert_eq!(table.matched_count(), 3);
    assert_eq!(table.orphan_count(), 0);
}

#[test]
fn duplicate_errors_are_idempotent() {
    // Consuming the entry is what makes a duplicated ICMP error safe.
    let t0 = std::time::Instant::now();
    let mut table = CorrelationTable::new();
    let a = Ipv4Addr::new(10, 0, 0, 5);
    table
        .register(a, 40001, 53, t0, std::time::Duration::from_secs(1))
        .unwrap();
    assert_eq!(
        table.correlate((a, 40001)),
        Correlation::Matched { port: 53 }
    );
    assert_eq!(table.correlate((a, 40001)), Correlation::Orphan);
    assert_eq!(table.matched_count(), 1, "one verdict per port, not two");
    assert_eq!(table.orphan_count(), 1);
}

#[test]
fn late_error_never_misattributes_to_a_reused_source_port() {
    // The dangerous sequence: a probe expires, the kernel hands its source
    // port to a new probe, then the stale error finally arrives. Re-inserting
    // or re-correlating here would report the old port's answer for the new
    // one.
    let t0 = std::time::Instant::now();
    let mut table = CorrelationTable::new();
    let a = Ipv4Addr::new(10, 0, 0, 5);
    table
        .register(a, 40001, 53, t0, std::time::Duration::from_millis(10))
        .unwrap();

    let expired = table.sweep(t0 + std::time::Duration::from_millis(50));
    assert_eq!(expired, vec![53]);
    assert_eq!(table.in_flight(), 0);
    assert_eq!(table.expired_count(), 1);

    // The stale error for 53 arrives after the port is gone.
    assert_eq!(table.correlate((a, 40001)), Correlation::Orphan);
    assert_eq!(
        table.matched_count(),
        0,
        "stale error must not produce a verdict"
    );

    // A new probe legitimately claims the same source port.
    table
        .register(a, 40001, 443, t0, std::time::Duration::from_secs(1))
        .unwrap();
    assert_eq!(
        table.correlate((a, 40001)),
        Correlation::Matched { port: 443 }
    );
    assert_eq!(table.matched_count(), 1);
    assert_eq!(table.orphan_count(), 1);
}

#[test]
fn registration_refuses_a_live_key_collision() {
    // Overwriting a live entry would let one port's verdict be reported for
    // another, so the collision is surfaced instead.
    let t0 = std::time::Instant::now();
    let mut table = CorrelationTable::new();
    let a = Ipv4Addr::new(10, 0, 0, 5);
    table
        .register(a, 40001, 53, t0, std::time::Duration::from_secs(1))
        .unwrap();
    let err = table
        .register(a, 40001, 161, t0, std::time::Duration::from_secs(1))
        .expect_err("collision must be refused");
    assert_eq!(err.probe_src_port, 40001);
    assert_eq!(table.collision_count(), 1);
    // The original probe still owns the key.
    assert_eq!(
        table.correlate((a, 40001)),
        Correlation::Matched { port: 53 }
    );
}

#[test]
fn sweep_returns_exactly_the_expired_ports_and_frees_them() {
    let t0 = std::time::Instant::now();
    let mut table = CorrelationTable::new();
    let a = Ipv4Addr::new(10, 0, 0, 5);
    for (i, port) in [53u16, 80, 443].iter().enumerate() {
        let window = std::time::Duration::from_millis((i as u64 + 1) * 100);
        table
            .register(a, 40000 + i as u16, *port, t0, window)
            .unwrap();
    }
    assert_eq!(table.in_flight(), 3);

    // Windows are 100ms / 200ms / 300ms, so a sweep at 250ms closes the first two.
    let expired = table.sweep(t0 + std::time::Duration::from_millis(250));
    let mut expired = expired;
    expired.sort_unstable();
    assert_eq!(expired, vec![53, 80], "only the two windows that closed");
    assert_eq!(table.in_flight(), 1, "the long-window probe survives");
    assert_eq!(table.expired_count(), 2);

    let expired = table.sweep(t0 + std::time::Duration::from_secs(10));
    assert_eq!(expired, vec![443]);
    assert_eq!(table.in_flight(), 0);
}

#[test]
fn table_memory_is_bounded_by_in_flight_not_by_port_count() {
    // A 65 535-port sweep must not hold 65 535 entries; the driver registers
    // and drains continuously, so the table only ever holds live probes.
    let t0 = std::time::Instant::now();
    let mut table = CorrelationTable::new();
    let a = Ipv4Addr::new(10, 0, 0, 5);
    let window = std::time::Duration::from_millis(1);
    for port in 1..=65_535u16 {
        table.register(a, port, port, t0, window).unwrap();
        if table.in_flight() > 64 {
            table.sweep(t0 + std::time::Duration::from_millis(10));
        }
    }
    assert!(
        table.in_flight() <= 64,
        "table grew to {} entries; it must stay O(concurrency)",
        table.in_flight()
    );
}

// ── request validation ──

#[test]
fn request_validation_rejects_bad_input_before_any_io() {
    let base = UdpScanRequest::default();
    base.validate().expect("defaults are valid");
    assert_eq!(base.port_count(), 1024);

    let bad_port_zero = UdpScanRequest {
        start_port: 0,
        ..base.clone()
    };
    assert!(matches!(
        bad_port_zero.validate(),
        Err(UdpScanError::PortRangeOutOfBounds { .. })
    ));

    let inverted = UdpScanRequest {
        start_port: 200,
        end_port: 100,
        ..base.clone()
    };
    assert!(matches!(
        inverted.validate(),
        Err(UdpScanError::EmptyPortRange)
    ));

    let zero_conc = UdpScanRequest {
        concurrency: 0,
        ..base.clone()
    };
    assert!(matches!(
        zero_conc.validate(),
        Err(UdpScanError::InvalidConcurrency(0))
    ));

    let huge_conc = UdpScanRequest {
        concurrency: MAX_CONCURRENCY + 1,
        ..base.clone()
    };
    assert!(matches!(
        huge_conc.validate(),
        Err(UdpScanError::InvalidConcurrency(_))
    ));

    let zero_timeout = UdpScanRequest {
        timeout: Duration::ZERO,
        ..base.clone()
    };
    assert!(matches!(
        zero_timeout.validate(),
        Err(UdpScanError::InvalidTimeout)
    ));

    let single = UdpScanRequest {
        start_port: 53,
        end_port: 53,
        ..base.clone()
    };
    single.validate().expect("single-port range is valid");
    assert_eq!(single.port_count(), 1);
}

#[test]
fn result_accessors_partition_ports() {
    let results = UdpScanResults {
        target: Ipv4Addr::LOCALHOST,
        ports_scanned: 3,
        host_state: HostState::Up,
        ports: vec![
            PortVerdict {
                port: 53,
                state: UdpPortState::Closed,
                evidence: Evidence::PortUnreachable,
            },
            PortVerdict {
                port: 80,
                state: UdpPortState::Filtered,
                evidence: Evidence::AdminProhibited,
            },
            PortVerdict {
                port: 443,
                state: UdpPortState::OpenFiltered,
                evidence: Evidence::Silence { sweeps: 1 },
            },
        ],
        evidence: IcmpEvidence::default(),
        duration: Duration::from_millis(5),
        truncated: false,
    };
    assert_eq!(results.closed_ports(), vec![53]);
    assert_eq!(results.filtered_ports(), vec![80]);
    assert_eq!(results.open_filtered_ports(), vec![443]);
}
