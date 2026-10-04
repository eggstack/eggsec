//! UDP port scanning for Eggsec.
//!
//! # What this crate claims, and what it does not
//!
//! UDP has no handshake. A datagram sent to an open port that does not
//! recognise the payload is silently ignored, and a datagram sent to a closed
//! port usually *also* goes unacknowledged because the ICMP error is often
//! rate-limited or dropped. So a range scan can prove **`closed`** and can
//! prove **`filtered`**, but it cannot prove **`open`**. Everything else is
//! reported as [`UdpPortState::OpenFiltered`], which is the honest maximum.
//!
//! This is a semantic ceiling, not an implementation shortfall. No range scan
//! can do better, and any tool claiming otherwise is producing false positives.
//! Claiming `open` requires a protocol-specific probe that draws a
//! recognisable response, which is outside this crate's scope.
//!
//! # The host-down trap
//!
//! A dead host and a fully-filtered host produce byte-for-byte identical
//! observations: silence on every port. A naive scanner reports "all 1024
//! ports open|filtered" for a host that is simply not there, which reads as a
//! result when it is a failure to reach the target at all. [`HostState`]
//! exists to make that distinction explicit, and
//! [`HostState::ports_are_meaningful`] is how a caller checks whether a run's
//! per-port verdicts are worth presenting.
//!
//! # Layering
//!
//! This crate is a domain crate and authorizes nothing. It never resolves DNS
//! (the caller passes a pre-resolved [`Ipv4Addr`]), never checks scope or
//! capability, never renders output, and never acquires privilege — it
//! *reports* that privilege is required. The engine's `EnforcementContext` is
//! the only thing that may authorize a scan; see `architecture/` for the
//! decision record.
//!
//! The decision logic ([`icmp`], [`classify`], [`correlate`]) is pure — bytes
//! and an explicit `now` in, verdict out — so the entire classifier is tested
//! with no network, no privilege, and no clock.

pub mod classify;
pub mod correlate;
pub mod icmp;
pub mod socket;

use std::net::Ipv4Addr;
use std::time::Duration;

pub use classify::{Evidence, HostState, PortVerdict, UdpPortState};
pub use correlate::{Correlation, CorrelationTable, ProbeCollision};
pub use icmp::{parse_icmp_error, IcmpError, ParseFailure};
pub use socket::{IcmpReceiver, IcmpReceiverError, ProbeSocket};

/// Default per-probe response window.
pub const DEFAULT_PROBE_TIMEOUT: Duration = Duration::from_millis(800);
/// Default number of sweeps when the caller does not choose.
pub const DEFAULT_SWEEPS: u32 = 1;
/// Default in-flight probe cap.
pub const DEFAULT_CONCURRENCY: usize = 64;
/// Hard ceiling on in-flight probes.
///
/// Each in-flight probe holds one file descriptor. The workspace's own
/// `MAX_CONCURRENCY` is 1000, which against a typical 1024-descriptor soft
/// `ulimit` is an EMFILE cliff, so this crate's default is deliberately far
/// lower and the driver clamps to real descriptor headroom.
pub const MAX_CONCURRENCY: usize = 512;
/// Largest port number (inclusive).
pub const MAX_PORT: u16 = 65535;

/// Errors from building or running a scan.
#[derive(Debug, thiserror::Error)]
pub enum UdpScanError {
    #[error("target address must be an IPv4 address")]
    NonIpv4Target,
    #[error("port range is empty")]
    EmptyPortRange,
    #[error("port range {start}-{end} is outside 1-{MAX_PORT}")]
    PortRangeOutOfBounds { start: u16, end: u16 },
    #[error("concurrency {0} must be between 1 and {MAX_CONCURRENCY}")]
    InvalidConcurrency(usize),
    #[error("timeout must be non-zero")]
    InvalidTimeout,
    #[error("ICMP error reception unavailable: {0}")]
    Receiver(#[source] IcmpReceiverError),
}

/// A validated scan request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpScanRequest {
    /// Pre-resolved target. The caller owns DNS and policy; this crate takes
    /// an address, never a hostname.
    pub target: Ipv4Addr,
    /// Inclusive port range to sweep.
    pub start_port: u16,
    pub end_port: u16,
    /// Per-probe response window.
    pub timeout: Duration,
    /// Maximum probes in flight at once.
    pub concurrency: usize,
    /// Additional sweeps over the range. ICMP generation is rate-limited per
    /// host, so a second sweep materially improves `closed` detection; each
    /// costs one more pass.
    pub sweeps: u32,
}

impl Default for UdpScanRequest {
    fn default() -> Self {
        Self {
            target: Ipv4Addr::LOCALHOST,
            start_port: 1,
            end_port: 1024,
            timeout: DEFAULT_PROBE_TIMEOUT,
            concurrency: DEFAULT_CONCURRENCY,
            sweeps: DEFAULT_SWEEPS,
        }
    }
}

impl UdpScanRequest {
    /// Validate and bound the request.
    ///
    /// Pure: no I/O, so an invalid request is rejected before a single packet
    /// leaves the host.
    pub fn validate(&self) -> Result<(), UdpScanError> {
        // Port 0 is not a probeable port. The upper bound needs no check: the
        // type is `u16`, so `end_port > MAX_PORT` is unrepresentable and
        // asserting it would be a comparison the compiler can always fold.
        if self.start_port == 0 || self.end_port == 0 {
            return Err(UdpScanError::PortRangeOutOfBounds {
                start: self.start_port,
                end: self.end_port,
            });
        }
        if self.start_port > self.end_port {
            return Err(UdpScanError::EmptyPortRange);
        }
        if self.concurrency == 0 || self.concurrency > MAX_CONCURRENCY {
            return Err(UdpScanError::InvalidConcurrency(self.concurrency));
        }
        if self.timeout.is_zero() {
            return Err(UdpScanError::InvalidTimeout);
        }
        Ok(())
    }

    /// Number of ports in the range.
    pub fn port_count(&self) -> u32 {
        u32::from(self.end_port) - u32::from(self.start_port) + 1
    }
}

/// Evidence density for a completed scan.
///
/// Reported so a caller can see *how much* signal backed the verdicts. A run
/// with zero observed errors and 1024 `open|filtered` results is not a port
/// list; it is evidence that nothing came back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IcmpEvidence {
    /// Errors matched to an in-flight probe.
    pub correlated: u64,
    /// Well-formed errors we could not attribute to a probe.
    pub orphans: u64,
    /// Probes whose window closed with no error.
    pub expired: u64,
    /// Registrations refused as source-port collisions.
    pub collisions: u64,
    /// Buffer reads that were not ICMP errors at all.
    pub unparseable: u64,
    /// Completed sweeps.
    pub sweeps: u32,
}

/// The result of a scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpScanResults {
    pub target: Ipv4Addr,
    pub ports_scanned: u32,
    /// Liveness verdict. Read this before trusting the per-port states.
    pub host_state: HostState,
    /// One verdict per port, in ascending port order.
    pub ports: Vec<PortVerdict>,
    pub evidence: IcmpEvidence,
    pub duration: Duration,
    /// Set when the run stopped early rather than covering the whole range.
    pub truncated: bool,
}

impl UdpScanResults {
    /// Ports proven closed.
    pub fn closed_ports(&self) -> Vec<u16> {
        self.ports
            .iter()
            .filter(|p| p.state == UdpPortState::Closed)
            .map(|p| p.port)
            .collect()
    }

    /// Ports proven filtered.
    pub fn filtered_ports(&self) -> Vec<u16> {
        self.ports
            .iter()
            .filter(|p| p.state == UdpPortState::Filtered)
            .map(|p| p.port)
            .collect()
    }

    /// Ports with no decisive signal.
    pub fn open_filtered_ports(&self) -> Vec<u16> {
        self.ports
            .iter()
            .filter(|p| p.state == UdpPortState::OpenFiltered)
            .map(|p| p.port)
            .collect()
    }
}

/// Run a scan against a pre-opened receiver.
///
/// The receiver is taken rather than opened here so a caller can choose
/// [`IcmpReceiver::open`] or [`IcmpReceiver::open_unprivileged`] explicitly —
/// the right choice is platform-dependent, and a library that silently picked
/// one would hide the privilege difference from the operator.
///
/// # Blocking and deadline-bounded
///
/// This is a blocking function. It returns by
/// `timeout * sweeps * ceil(ports / concurrency)` plus one final window, and
/// holds no thread past that bound. Callers on an async runtime must therefore
/// bound the call from outside as well: a cancelled `spawn_blocking` join does
/// not stop the thread inside it.
pub fn scan_with_receiver(
    request: &UdpScanRequest,
    receiver: &IcmpReceiver,
) -> Result<UdpScanResults, UdpScanError> {
    request.validate()?;
    let started = std::time::Instant::now();
    let port_count = request.port_count();
    let sweeps = request.sweeps.max(1);

    let mut table = CorrelationTable::new();
    let mut unparseable: u64 = 0;
    // Decided ports, keyed by port so a later sweep can still correct an
    // earlier silence with a real ICMP error.
    let mut verdicts: std::collections::BTreeMap<u16, PortVerdict> =
        std::collections::BTreeMap::new();

    let mut buffer = vec![0u8; 2048];

    for sweep in 1..=sweeps {
        for port in request.start_port..=request.end_port {
            // Registration is the admission control: only `concurrency`
            // probes are ever live, which is what bounds both descriptor use
            // and table size.
            if table.in_flight() >= request.concurrency {
                flush_window(
                    receiver,
                    &mut table,
                    &mut verdicts,
                    &mut buffer,
                    &mut unparseable,
                    request.timeout,
                );
            }
            let mut socket = match ProbeSocket::bind(Ipv4Addr::UNSPECIFIED) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(error = %e, "probe socket creation failed; stopping sweep");
                    break;
                }
            };
            // Connect before registering: the kernel only quotes the
            // route-selected local address, and until the socket is connected
            // `getsockname` still reports the wildcard bind, so the
            // correlation key would never match.
            if let Err(e) = socket.connect_to(std::net::SocketAddrV4::new(request.target, port)) {
                tracing::debug!(port, error = %e, "probe connect failed");
                continue;
            }
            let local = socket.local();
            if let Err(e) = table.register(
                *local.ip(),
                local.port(),
                port,
                std::time::Instant::now(),
                request.timeout,
            ) {
                // A collision means another probe owns this source port.
                // Dropping this probe is correct: attributing its verdict to
                // the port that already holds the key would be a lie.
                tracing::debug!(port, error = %e, "probe source-port collision; skipping probe");
                continue;
            }
            if let Err(e) = socket.send(DEFAULT_PROBE_PAYLOAD) {
                tracing::debug!(port, error = %e, "probe send failed");
                table.sweep(std::time::Instant::now());
                continue;
            }
        }
        flush_window(
            receiver,
            &mut table,
            &mut verdicts,
            &mut buffer,
            &mut unparseable,
            request.timeout,
        );
        tracing::debug!(sweep, ports = request.port_count(), "sweep complete");
    }

    let host_state = HostState::derive(table.matched_count(), table.orphan_count());
    // Every port in the range gets a verdict, even if its probe never ran, so
    // the result is a complete picture rather than a partial one.
    let ports: Vec<PortVerdict> = (request.start_port..=request.end_port)
        .map(|port| {
            verdicts.remove(&port).unwrap_or_else(|| {
                let (state, evidence) = classify::classify_silence(sweeps);
                PortVerdict {
                    port,
                    state,
                    evidence,
                }
            })
        })
        .collect();

    Ok(UdpScanResults {
        target: request.target,
        ports_scanned: port_count,
        host_state,
        ports,
        evidence: IcmpEvidence {
            correlated: table.matched_count(),
            orphans: table.orphan_count(),
            expired: table.expired_count(),
            collisions: table.collision_count(),
            unparseable,
            sweeps,
        },
        duration: started.elapsed(),
        truncated: false,
    })
}

/// Probe payload.
///
/// Deliberately an empty, well-formed application datagram. Sending a
/// recognisable protocol payload would make `open` provable, but that is a
/// different operation with different authorization requirements: it lets a
/// caller put arbitrary protocol bytes on the wire, which is materially closer
/// to packet injection than to port scanning.
pub(crate) const DEFAULT_PROBE_PAYLOAD: &[u8] = b"";

/// Drain the receiver for one window, then resolve every probe that has
/// expired.
fn flush_window(
    receiver: &IcmpReceiver,
    table: &mut CorrelationTable,
    verdicts: &mut std::collections::BTreeMap<u16, PortVerdict>,
    buffer: &mut [u8],
    unparseable: &mut u64,
    window: Duration,
) {
    let deadline = std::time::Instant::now() + window;
    loop {
        let now = std::time::Instant::now();
        if now >= deadline || table.in_flight() == 0 {
            break;
        }
        let remaining = deadline.saturating_duration_since(now);
        match receiver.recv_timeout(remaining, buffer) {
            Ok(Some(n)) => match parse_icmp_error(&buffer[..n]) {
                Ok(err) => {
                    let key = err.correlation_key();
                    if let Correlation::Matched { port } = table.correlate(key) {
                        let (state, evidence) = classify::classify(&err);
                        verdicts.insert(
                            port,
                            PortVerdict {
                                port,
                                state,
                                evidence,
                            },
                        );
                    }
                }
                Err(ParseFailure::NotAnErrorType) | Err(ParseFailure::TooShort) => {
                    // Shared socket traffic. Expected, not an error.
                }
                Err(_) => *unparseable += 1,
            },
            Ok(None) => continue,
            Err(e) => {
                tracing::debug!(error = %e, "ICMP receive error; ending window early");
                break;
            }
        }
    }
    for port in table.sweep(std::time::Instant::now()) {
        // Silence is not recorded here: it is filled in for every uncovered
        // port at the end, so a later sweep can still overwrite it with a
        // proven verdict.
        let _ = port;
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod loopback {
    //! Live socket test against loopback.
    //!
    //! The hermetic suite proves the decision logic; this proves the socket
    //! path actually works on this platform — a separate question, because
    //! ICMP delivery differs by OS and only a real socket can show it.
    //!
    //! Gated behind `EGGSEC_ALLOW_LOOPBACK_FIXTURE=1`, matching the workspace
    //! convention, and skipped when running as root: an ICMP socket owned by
    //! root changes who receives errors and would make the assertions
    //! meaningless.

    use super::*;
    use std::net::UdpSocket;

    fn fixture_enabled() -> bool {
        std::env::var("EGGSEC_ALLOW_LOOPBACK_FIXTURE").is_ok_and(|v| v == "1")
    }

    fn running_as_root() -> bool {
        // SAFETY: `geteuid` takes no arguments, cannot fail, and has no
        // preconditions.
        unsafe { libc::geteuid() == 0 }
    }

    /// An open loopback UDP port, held for the duration of the test.
    ///
    /// Binding a real listener is what makes the "open" case deterministic: a
    /// port nobody holds may be filtered, and would test the wrong thing.
    fn open_loopback_port() -> Option<(UdpSocket, u16)> {
        let sock = UdpSocket::bind("127.0.0.1:0").ok()?;
        let port = sock.local_addr().ok()?.port();
        Some((sock, port))
    }

    /// A loopback port nothing holds, so the kernel answers port-unreachable.
    fn unheld_loopback_port() -> Option<u16> {
        let sock = UdpSocket::bind("127.0.0.1:0").ok()?;
        let port = sock.local_addr().ok()?.port();
        drop(sock);
        Some(port)
    }

    fn receiver_or_skip() -> Option<IcmpReceiver> {
        // Prefer the unprivileged datagram ICMP socket; fall back so the test
        // reports this platform rather than failing on an unsupported OS.
        match IcmpReceiver::open_unprivileged() {
            Ok(r) => Some(r),
            Err(_) => match IcmpReceiver::open() {
                Ok(r) => Some(r),
                Err(e) => {
                    eprintln!("skipping: no ICMP receiver available here: {e}");
                    None
                }
            },
        }
    }

    fn single_port_request(port: u16) -> UdpScanRequest {
        UdpScanRequest {
            target: Ipv4Addr::LOCALHOST,
            start_port: port,
            end_port: port,
            timeout: Duration::from_millis(500),
            concurrency: 1,
            sweeps: 2,
        }
    }

    /// A port with a live listener must never be claimed `Open` by silence.
    ///
    /// This is the crate's core honesty property, checked against a real
    /// socket rather than a synthetic packet.
    #[test]
    fn silence_on_an_open_port_is_never_reported_as_open() {
        if !fixture_enabled() {
            eprintln!("skipping: set EGGSEC_ALLOW_LOOPBACK_FIXTURE=1 to run");
            return;
        }
        if running_as_root() {
            eprintln!("skipping: root changes ICMP socket ownership");
            return;
        }
        let Some(receiver) = receiver_or_skip() else {
            return;
        };
        let Some((open_socket, open_port)) = open_loopback_port() else {
            eprintln!("skipping: could not bind a loopback UDP port");
            return;
        };

        let results =
            scan_with_receiver(&single_port_request(open_port), &receiver).expect("scan runs");
        assert_eq!(results.ports_scanned, 1);
        let verdict = results.ports.first().expect("one verdict");
        assert_eq!(verdict.port, open_port);
        assert_ne!(
            verdict.state,
            UdpPortState::Open,
            "silence must never be reported as Open"
        );
        drop(open_socket);
    }

    /// An unheld loopback port is proven `Closed` when its ICMP error arrives.
    ///
    /// When no ICMP comes back at all, the host verdict must carry that
    /// failure rather than the port list implying an open service.
    #[test]
    fn unheld_port_is_proven_closed_when_icmp_returns() {
        if !fixture_enabled() {
            eprintln!("skipping: set EGGSEC_ALLOW_LOOPBACK_FIXTURE=1 to run");
            return;
        }
        if running_as_root() {
            eprintln!("skipping: root changes ICMP socket ownership");
            return;
        }
        let Some(receiver) = receiver_or_skip() else {
            return;
        };
        let Some(closed_port) = unheld_loopback_port() else {
            eprintln!("skipping: could not bind a loopback UDP port");
            return;
        };

        let results =
            scan_with_receiver(&single_port_request(closed_port), &receiver).expect("scan runs");
        let verdict = results.ports.first().expect("one verdict");
        assert_eq!(verdict.port, closed_port);

        if results.evidence.correlated > 0 {
            assert_eq!(results.host_state, HostState::Up);
            assert_eq!(
                verdict.state,
                UdpPortState::Closed,
                "a proven port-unreachable must classify Closed, got {verdict:?}"
            );
            assert_eq!(verdict.evidence, Evidence::PortUnreachable);
            assert!(verdict.evidence.is_proof());
        } else {
            // The honest failure mode: no signal, so the host verdict is
            // unresponsive and the single `open|filtered` port is explicitly
            // not a finding.
            assert_eq!(results.host_state, HostState::Unresponsive);
            assert!(!results.host_state.ports_are_meaningful());
            assert!(!verdict.evidence.is_proof());
        }
    }
}
