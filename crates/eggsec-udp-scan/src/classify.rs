//! Pure classification: turning an ICMP error (or the absence of one) into an
//! honest port state.
//!
//! The governing fact is that **closed is provable and open is not**. A UDP
//! datagram to an open port that does not recognise the payload is simply
//! ignored, so silence is consistent with four different situations. Any
//! implementation that reports "open" from silence is producing false
//! positives; this one never does.

use crate::icmp::{icmp_type, unreachable_code, IcmpError};

/// The state of one probed UDP port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UdpPortState {
    /// Provably closed: the peer returned ICMP port-unreachable.
    Closed,
    /// Provably filtered: a device administratively refused, or the probe's
    /// TTL expired in transit so no reply could come back.
    Filtered,
    /// No decisive signal. This is the honest verdict for most ports and is
    /// the *maximum* claim an unprobed port supports.
    OpenFiltered,
    /// A protocol probe elicited a response. Only a protocol-specific probe
    /// can earn this, and this crate's range scan never does.
    Open,
}

/// Why the scanner believes a port is in its reported state.
///
/// Evidence travels with the verdict so a caller can tell a proven `Closed`
/// from a `Closed` that arrived after the host stopped responding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Evidence {
    /// ICMP port-unreachable (type 3 / code 3).
    PortUnreachable,
    /// ICMP administratively prohibited (type 3 / code 9).
    AdminProhibited,
    /// ICMP net/host unreachable (type 3 / code 0 or 1). Weak: a gateway
    /// saying the host is unreachable usually means the host is down.
    NetOrHostUnreachable,
    /// ICMP time exceeded (type 11): TTL ran out, so no reply could return.
    TimeExceeded,
    /// No ICMP attributable to this port within the window.
    Silence { sweeps: u32 },
    /// A protocol probe drew a response.
    ConfirmedResponse { bytes: usize },
}

impl Evidence {
    /// Whether this evidence *proves* its state, as opposed to permitting it.
    ///
    /// `Silence` never proves anything, and `NetOrHostUnreachable` proves very
    /// little — both are reported as unproven so a caller cannot mistake a
    /// permissive verdict for a measured one.
    pub fn is_proof(&self) -> bool {
        matches!(
            self,
            Evidence::PortUnreachable
                | Evidence::AdminProhibited
                | Evidence::TimeExceeded
                | Evidence::ConfirmedResponse { .. }
        )
    }
}

/// One port's verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortVerdict {
    pub port: u16,
    pub state: UdpPortState,
    pub evidence: Evidence,
}

/// Classify a parsed ICMP error.
///
/// `NetOrHostUnreachable` is deliberately reported as `Filtered` *with*
/// unproven evidence, and never as `OpenFiltered`: a gateway reporting an
/// unreachable host is a statement about the host, not about the port.
pub fn classify(error: &IcmpError) -> (UdpPortState, Evidence) {
    if error.icmp_type == icmp_type::TIME_EXCEEDED {
        return (UdpPortState::Filtered, Evidence::TimeExceeded);
    }
    match error.code {
        unreachable_code::PORT_UNREACH => (UdpPortState::Closed, Evidence::PortUnreachable),
        unreachable_code::ADMIN_PROHIBITED => (UdpPortState::Filtered, Evidence::AdminProhibited),
        unreachable_code::NET_UNREACH | unreachable_code::HOST_UNREACH => {
            (UdpPortState::Filtered, Evidence::NetOrHostUnreachable)
        }
        // An unhandled code proves the peer is reachable and refusing, but we
        // will not guess which refusal. Filtered is the safe reading.
        _ => (UdpPortState::Filtered, Evidence::AdminProhibited),
    }
}

/// Classify the absence of an ICMP error for one port.
///
/// `host_alive` is the honest input here: when the host produced no ICMP at
/// all across the whole scan, silence carries no information, and saying so
/// rather than implying 1024 open ports is the entire point of the host-state
/// verdict in [`crate::HostState`].
pub fn classify_silence(sweeps: u32) -> (UdpPortState, Evidence) {
    (UdpPortState::OpenFiltered, Evidence::Silence { sweeps })
}

/// Whether the host is in a state where per-port results mean anything.
///
/// This exists because a dead host and a fully-filtered host produce byte-for-
/// byte identical observations: silence on every port. Without a liveness
/// verdict, "everything is open|filtered" reads as a result when it is
/// actually a failure to reach the host at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostState {
    /// Proven alive: at least one ICMP error arrived from the target. Note this
    /// is self-validating — a port-unreachable from a known-closed port proves
    /// both liveness and that ICMP suppression is not total.
    Up,
    /// The scan window elapsed with zero ICMP attributable to the target.
    /// Consistent with host-down, total ICMP suppression, or a target that is
    /// genuinely silent on every probed port.
    Unresponsive,
    /// Inconclusive: errors arrived but none could be correlated to a probe.
    Indeterminate,
}

impl HostState {
    /// Whether per-port verdicts from a run with this host state are worth
    /// anything. A caller presenting `OpenFiltered` results from an
    /// `Unresponsive` host is over-claiming, and should say so.
    pub fn ports_are_meaningful(&self) -> bool {
        matches!(self, HostState::Up)
    }

    /// Derive the host verdict from observed evidence.
    ///
    /// `correlated` counts errors that matched an in-flight probe; `orphans`
    /// counts well-formed errors we could not attribute. Only a *correlated*
    /// error proves the host is up, because an orphan may be another process's
    /// traffic on a shared socket.
    pub fn derive(correlated: u64, orphans: u64) -> HostState {
        if correlated > 0 {
            HostState::Up
        } else if orphans > 0 {
            HostState::Indeterminate
        } else {
            HostState::Unresponsive
        }
    }
}
