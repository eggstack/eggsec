//! Pure ICMP error parsing.
//!
//! Everything in this module is a function of bytes alone: no sockets, no
//! clock, no threads. That is what makes the scanner's decision logic testable
//! without privilege, without a network, and without flakiness.
//!
//! # Delivery shape differs by platform
//!
//! A raw IPv4 socket with `IPPROTO_ICMPV4` hands back the ICMP message
//! *without* its IP header. A datagram ICMP socket (macOS/BSD) hands back the
//! whole IPv4 packet, header included. [`parse_icmp_error`] accepts both: it
//! strips a leading IPv4 header when one is present and the remainder still
//! parses as an ICMP error, and otherwise treats the input as a bare ICMP
//! message. Getting this wrong silently classifies nothing, so the
//! disambiguation is asserted in both directions by tests.

use std::net::Ipv4Addr;

/// IANA ICMPv4 `type` values this crate interprets.
pub mod icmp_type {
    /// Destination Unreachable.
    pub const DEST_UNREACH: u8 = 3;
    /// Time Exceeded (includes TTL expired in transit).
    pub const TIME_EXCEEDED: u8 = 11;
    /// Parameter Problem.
    pub const PARAM_PROBLEM: u8 = 12;
}

/// IANA ICMPv4 `type=3` (Destination Unreachable) `code` values.
pub mod unreachable_code {
    /// Network unreachable — a gateway said the network is down, which is
    /// frequently a *dead host* rather than a filtered port.
    pub const NET_UNREACH: u8 = 0;
    /// Host unreachable — same caveat as [`NET_UNREACH`].
    pub const HOST_UNREACH: u8 = 1;
    /// Port unreachable — the definitive "this port is closed" signal.
    pub const PORT_UNREACH: u8 = 3;
    /// Administratively prohibited — a device refused; the port is filtered.
    pub const ADMIN_PROHIBITED: u8 = 9;
}

/// IP protocol number for UDP, as it appears in a quoted IP header.
const PROTO_UDP: u8 = 17;

/// A parsed ICMP error that provably refers to one of our UDP probes.
///
/// The correlation key is `(source_ip, source_port)` of the *original* datagram,
/// which is how an out-of-band error is tied back to the probe that caused it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IcmpError {
    pub icmp_type: u8,
    pub code: u8,
    /// Source address of the original datagram — our probe socket.
    pub probe_src: Ipv4Addr,
    /// Source port of the original datagram — our probe socket.
    pub probe_src_port: u16,
    /// Destination port of the original datagram — the port being probed.
    pub probe_dst_port: u16,
    /// Destination address of the original datagram.
    pub probe_dst: Ipv4Addr,
}

impl IcmpError {
    /// The correlation key: which in-flight probe this error belongs to.
    pub fn correlation_key(&self) -> (Ipv4Addr, u16) {
        (self.probe_src, self.probe_src_port)
    }
}

/// Why a buffer could not be read as an ICMP error referencing a UDP probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseFailure {
    /// Shorter than any ICMP error can be.
    TooShort,
    /// Looks like a bare ICMP header but the type/code pair is not an error we
    /// interpret (e.g. an echo reply, or a type we do not claim).
    NotAnErrorType,
    /// The quoted datagram is not UDP, so this error cannot be ours.
    NotUdp,
    /// RFC 792 guarantees the original IP header plus the first 8 bytes of
    /// payload; for UDP those 8 bytes are the whole UDP header. Anything
    /// shorter cannot be correlated.
    QuotedDatagramTruncated,
    /// A quoted header with an impossible IHL, or a length field shorter than
    /// the header it claims.
    MalformedQuotedHeader,
}

/// Parse an ICMP error that references a UDP datagram we sent.
///
/// Returns `Err(ParseFailure::NotAnErrorType)` for well-formed ICMP that is not
/// an error we interpret, so callers can count those separately from
/// corruption.
pub fn parse_icmp_error(bytes: &[u8]) -> Result<IcmpError, ParseFailure> {
    let icmp = split_icmp(bytes).ok_or(ParseFailure::TooShort)?;
    // type, code, checksum, unused x4 -- plus the quoted datagram after it.
    if icmp.len() < 8 {
        return Err(ParseFailure::TooShort);
    }
    let icmp_type = icmp[0];
    let code = icmp[1];
    // Only destination-unreachable and time-exceeded carry a quoted datagram.
    if icmp_type != icmp_type::DEST_UNREACH && icmp_type != icmp_type::TIME_EXCEEDED {
        return Err(ParseFailure::NotAnErrorType);
    }
    // An ICMP error is 8 bytes of header (type, code, checksum, unused x4)
    // followed by the quoted datagram.
    let quoted = icmp.get(8..).ok_or(ParseFailure::TooShort)?;
    parse_quoted_udp(quoted, icmp_type, code)
}

/// Strip a leading IPv4 header if present, returning the ICMP message.
///
/// Detection is deliberately conservative: the input is treated as
/// header-prefixed only when byte 0 has version 4, a plausible IHL, and the
/// protocol field says ICMP. A bare ICMP message can never satisfy all three
/// (its first byte is the ICMP type, and its 10th byte is arbitrary), so the
/// two shapes are reliably distinguishable.
fn split_icmp(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.len() >= 20 {
        let version = bytes[0] >> 4;
        let ihl = (bytes[0] & 0x0f) * 4;
        if version == 4 && ihl >= 20 && bytes[9] == icmp_proto() {
            return bytes.get(ihl as usize..);
        }
    }
    Some(bytes)
}

/// Protocol number for ICMP, as it appears in an IPv4 header.
const fn icmp_proto() -> u8 {
    1
}

/// Parse the quoted IP header + quoted UDP header of an ICMP error.
fn parse_quoted_udp(quoted: &[u8], icmp_type: u8, code: u8) -> Result<IcmpError, ParseFailure> {
    if quoted.len() < 20 {
        return Err(ParseFailure::QuotedDatagramTruncated);
    }
    if quoted[0] >> 4 != 4 {
        return Err(ParseFailure::MalformedQuotedHeader);
    }
    let ihl = usize::from(quoted[0] & 0x0f) * 4;
    if ihl < 20 {
        return Err(ParseFailure::MalformedQuotedHeader);
    }
    // A total length that cannot even cover its own header is corrupt.
    let total_len = u16::from_be_bytes([quoted[2], quoted[3]]);
    if total_len != 0 && (total_len as usize) < ihl {
        return Err(ParseFailure::MalformedQuotedHeader);
    }
    if quoted[9] != PROTO_UDP {
        return Err(ParseFailure::NotUdp);
    }
    // 8 bytes of quoted payload is exactly the UDP header for our purposes.
    let udp = quoted
        .get(ihl..)
        .ok_or(ParseFailure::QuotedDatagramTruncated)?;
    if udp.len() < 8 {
        return Err(ParseFailure::QuotedDatagramTruncated);
    }
    // UDP length covers header + payload and can never be below the header.
    let udp_len = u16::from_be_bytes([udp[4], udp[5]]);
    if udp_len != 0 && udp_len < 8 {
        return Err(ParseFailure::MalformedQuotedHeader);
    }
    Ok(IcmpError {
        icmp_type,
        code,
        probe_src: Ipv4Addr::new(quoted[12], quoted[13], quoted[14], quoted[15]),
        probe_src_port: u16::from_be_bytes([udp[0], udp[1]]),
        probe_dst: Ipv4Addr::new(quoted[16], quoted[17], quoted[18], quoted[19]),
        probe_dst_port: u16::from_be_bytes([udp[2], udp[3]]),
    })
}
