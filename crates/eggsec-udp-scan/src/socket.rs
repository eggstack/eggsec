//! Platform ICMP receive socket.
//!
//! # Why not `socket2`
//!
//! The obvious choice is a `socket2::Socket`, but it cannot express this job on
//! any platform:
//!
//! * `IP_RECVERR` (the Linux trick of having the kernel cache per-socket ICMP
//!   errors) has no accessor in `socket2` 0.5, and the type is not defined on
//!   macOS/BSD at all.
//! * `Socket::as_raw()` is `pub(crate)`, so even a `libc::setsockopt` shim is
//!   impossible against a `socket2` socket.
//!
//! So the socket is built with `libc` directly. `libc` is already in the
//! workspace graph, so this adds no new supply-chain surface.
//!
//! # Why `SOCK_DGRAM` ICMP on macOS/BSD and `SOCK_RAW` on Linux
//!
//! Measured on this repository's development platform (darwin/arm64, non-root):
//! `socket(AF_INET, SOCK_DGRAM, IPPROTO_ICMPV4)` succeeds unprivileged and
//! delivers port-unreachable errors for UDP probes sent from another socket,
//! with the originating IPv4 header still attached. `SOCK_RAW` for the same
//! protocol returns `EPERM`.
//!
//! Linux is the mirror image: a datagram ICMP socket is confined to echo
//! replies (`net.ipv4.ping_group_range`), so unsolicited errors need
//! `SOCK_RAW` and therefore `CAP_NET_RAW`.
//!
//! The practical consequence is that this crate's unprivileged tier is
//! available on macOS/BSD and *not* on Linux, which is the opposite of the
//! assumption the usual "unprivileged tier" designs start from. Callers get
//! [`IcmpReceiver::is_unprivileged`] to branch on it rather than guessing.

use std::io;
use std::net::{Ipv4Addr, SocketAddrV4};

/// Why an ICMP receive socket could not be opened.
#[derive(Debug, thiserror::Error)]
pub enum IcmpReceiverError {
    #[error("UDP port scanning is not supported on this platform")]
    UnsupportedPlatform,
    #[error("failed to create ICMP receive socket: {0}")]
    Create(#[source] io::Error),
    #[error("ICMP receive requires elevated privileges (CAP_NET_RAW or root) on this platform")]
    PrivilegesRequired,
    #[error("failed to configure ICMP receive socket: {0}")]
    Configure(#[source] io::Error),
}

/// A blocking ICMP error receiver.
#[derive(Debug)]
pub struct IcmpReceiver {
    fd: libc::c_int,
    /// Whether this socket receives errors without elevated privileges here.
    unprivileged: bool,
}

// The fd is owned exclusively by this struct and closed exactly once.
impl Drop for IcmpReceiver {
    fn drop(&mut self) {
        // SAFETY: `fd` was created by `socket()` in `open()` and is closed
        // exactly once here. Nothing else holds a duplicate, so there is no
        // use-after-close.
        unsafe { libc::close(self.fd) };
    }
}

impl IcmpReceiver {
    /// Open a socket that receives ICMP errors for the whole host.
    ///
    /// The socket is shared by every scanner on the machine, so
    /// [`CorrelationTable`](crate::CorrelationTable) is not optional: errors
    /// belonging to other processes arrive here too and must be discarded
    /// rather than misattributed.
    pub fn open() -> Result<Self, IcmpReceiverError> {
        #[cfg(not(all(unix, target_os = "linux")))]
        {
            let _ = IcmpReceiverError::UnsupportedPlatform;
            Err(IcmpReceiverError::UnsupportedPlatform)
        }
        #[cfg(all(unix, target_os = "linux"))]
        {
            // Linux: datagram ICMP is echo-only, so raw is the only path that
            // sees unsolicited errors — and it needs CAP_NET_RAW.
            // SAFETY: `socket` with a constant AF_INET/SOCK_RAW/IPPROTO_ICMPV4
            // is a plain syscall with no pointer arguments.
            let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_RAW, 1) };
            if fd < 0 {
                let err = io::Error::last_os_error();
                return Err(if err.kind() == io::ErrorKind::PermissionDenied {
                    IcmpReceiverError::PrivilegesRequired
                } else {
                    IcmpReceiverError::Create(err)
                });
            }
            // SAFETY: `fd` is a live socket we own.
            let receiver = IcmpReceiver {
                fd,
                unprivileged: false,
            };
            receiver.configure()?;
            Ok(receiver)
        }
    }

    /// Open a datagram ICMP socket where the platform allows it unprivileged.
    pub fn open_unprivileged() -> Result<Self, IcmpReceiverError> {
        #[cfg(not(all(unix, not(target_os = "linux"))))]
        {
            let _ = IcmpReceiverError::UnsupportedPlatform;
            Err(IcmpReceiverError::UnsupportedPlatform)
        }
        #[cfg(all(unix, not(target_os = "linux")))]
        {
            // SAFETY: constant AF_INET/SOCK_DGRAM/IPPROTO_ICMPV4, no pointers.
            let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 1) };
            if fd < 0 {
                let err = io::Error::last_os_error();
                return Err(if err.kind() == io::ErrorKind::PermissionDenied {
                    IcmpReceiverError::PrivilegesRequired
                } else {
                    IcmpReceiverError::Create(err)
                });
            }
            // The datagram ICMP socket defaults to dual-stack, which delivers
            // IPv4 errors in IPv6 encapsulation. Clearing V6ONLY yields the
            // native IPv4 form this crate parses.
            let off: libc::c_int = 0;
            // SAFETY: `fd` is live and owned; the option/value pair is a valid
            // pointer for the duration of the call.
            let rc = unsafe {
                libc::setsockopt(
                    fd,
                    libc::IPPROTO_IPV6,
                    libc::IPV6_V6ONLY,
                    std::ptr::addr_of!(off).cast(),
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                )
            };
            if rc < 0 {
                tracing::debug!(
                    error = %io::Error::last_os_error(),
                    "could not clear IPV6_V6ONLY on ICMP socket; continuing"
                );
            }
            let receiver = IcmpReceiver {
                fd,
                unprivileged: true,
            };
            receiver.configure()?;
            Ok(receiver)
        }
    }

    /// Whether this socket receives unsolicited errors without root here.
    pub fn is_unprivileged(&self) -> bool {
        self.unprivileged
    }

    /// Put the socket in non-blocking mode and arm a receive deadline.
    fn configure(&self) -> Result<(), IcmpReceiverError> {
        // SAFETY: `self.fd` is live and owned by this struct.
        unsafe {
            let flags = libc::fcntl(self.fd, libc::F_GETFL, 0);
            if flags < 0 {
                return Err(IcmpReceiverError::Configure(io::Error::last_os_error()));
            }
            if libc::fcntl(self.fd, libc::F_SETFL, flags | libc::O_NONBLOCK) < 0 {
                return Err(IcmpReceiverError::Configure(io::Error::last_os_error()));
            }
        }
        Ok(())
    }

    /// Read one datagram, waiting up to `window` for it.
    ///
    /// Returns `Ok(None)` on timeout. The caller is expected to sweep expired
    /// probes at the same boundary, which keeps the timeout as the single
    /// clock the driver follows.
    pub fn recv_timeout(
        &self,
        window: std::time::Duration,
        out: &mut [u8],
    ) -> Result<Option<usize>, IcmpReceiverError> {
        // SAFETY: `self.fd` is live and owned. `poll` writes at most one
        // `pollfd` to `fds`, which is a local, correctly sized, valid pointer.
        let ready = unsafe {
            let mut fds = libc::pollfd {
                fd: self.fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let millis = window.as_millis().min(i32::MAX as u128) as libc::c_int;
            libc::poll(std::ptr::addr_of_mut!(fds), 1, millis)
        };
        if ready < 0 {
            let err = io::Error::last_os_error();
            // EINTR is a normal outcome under signals, not a failure.
            if err.kind() == io::ErrorKind::Interrupted {
                return Ok(None);
            }
            return Err(IcmpReceiverError::Configure(err));
        }
        if ready == 0 {
            return Ok(None);
        }
        // SAFETY: `out` is a valid, exclusively borrowed buffer and `recv` is
        // told its real length, so it cannot write past the end.
        let n = unsafe { libc::recv(self.fd, out.as_mut_ptr().cast(), out.len(), 0) };
        if n < 0 {
            return match io::Error::last_os_error().kind() {
                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => Ok(None),
                // A shared ICMP socket sees other processes' traffic; a
                // connection-style refusal here is not ours and must not
                // abort the scan.
                io::ErrorKind::ConnectionRefused => Ok(None),
                other => Err(IcmpReceiverError::Configure(io::Error::from(other))),
            };
        }
        Ok(Some(n as usize))
    }
}

/// A connected UDP probe socket.
///
/// Kept per-probe on purpose: connecting pins the local endpoint, which is the
/// correlation key the kernel quotes back inside the ICMP error.
#[derive(Debug)]
pub struct ProbeSocket {
    fd: libc::c_int,
    local: SocketAddrV4,
}

impl Drop for ProbeSocket {
    fn drop(&mut self) {
        // SAFETY: `fd` was created by `socket()` in `bind` and is closed once.
        unsafe { libc::close(self.fd) };
    }
}

impl ProbeSocket {
    /// Create a UDP socket bound to an ephemeral port on `local_ip`.
    ///
    /// Binding explicitly is what makes the source port known up front, so the
    /// correlation key can be registered before the probe is sent.
    pub fn bind(local_ip: Ipv4Addr) -> Result<Self, IcmpReceiverError> {
        #[cfg(not(unix))]
        {
            let _ = local_ip;
            return Err(IcmpReceiverError::UnsupportedPlatform);
        }
        #[cfg(unix)]
        {
            // SAFETY: constant AF_INET/SOCK_DGRAM/IPPROTO_UDP, no pointers.
            let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, libc::IPPROTO_UDP) };
            if fd < 0 {
                return Err(IcmpReceiverError::Create(io::Error::last_os_error()));
            }
            // SAFETY: `local` is a fully initialised sockaddr_in; `fd` is live.
            let bound = unsafe {
                let mut addr: libc::sockaddr_in = std::mem::zeroed();
                addr.sin_family = libc::AF_INET as libc::sa_family_t;
                addr.sin_addr.s_addr = u32::from(local_ip).to_be();
                addr.sin_port = 0; // ephemeral
                libc::bind(
                    fd,
                    std::ptr::addr_of!(addr).cast(),
                    std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
                )
            };
            if bound < 0 {
                let err = io::Error::last_os_error();
                // SAFETY: `fd` is live and owned; close it on the error path.
                unsafe { libc::close(fd) };
                return Err(IcmpReceiverError::Configure(err));
            }
            // SAFETY: `fd` is bound, so getsockname cannot fail here.
            let local = unsafe {
                let mut addr: libc::sockaddr_in = std::mem::zeroed();
                let mut len = std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
                libc::getsockname(
                    fd,
                    std::ptr::addr_of_mut!(addr).cast(),
                    std::ptr::addr_of_mut!(len),
                );
                SocketAddrV4::new(
                    Ipv4Addr::from(u32::from_be(addr.sin_addr.s_addr)),
                    u16::from_be(addr.sin_port),
                )
            };
            Ok(ProbeSocket { fd, local })
        }
    }

    /// The local endpoint the kernel will quote back in an ICMP error.
    ///
    /// Before [`ProbeSocket::connect_to`] this is the bind address, which for
    /// a wildcard bind is `0.0.0.0` and is therefore *not* the address the
    /// kernel quotes. Read it only after connecting.
    pub fn local(&self) -> SocketAddrV4 {
        self.local
    }

    /// Connect to `target`, which pins the local endpoint the kernel will use.
    ///
    /// Connecting is what makes the correlation key knowable. A wildcard-bound
    /// socket has no single local address until the route picks one, and
    /// `getsockname` keeps reporting `0.0.0.0`; an ICMP error, by contrast,
    /// quotes the address the datagram actually came from. Connecting first
    /// and re-reading `getsockname` is what makes the two agree.
    pub fn connect_to(&mut self, target: SocketAddrV4) -> Result<(), IcmpReceiverError> {
        #[cfg(not(unix))]
        {
            let _ = target;
            return Err(IcmpReceiverError::UnsupportedPlatform);
        }
        #[cfg(unix)]
        {
            // SAFETY: `target` is converted to a fully initialised
            // sockaddr_in, and `self.fd` is live and owned.
            let rc = unsafe {
                let addr: libc::sockaddr_in = std::mem::zeroed();
                let mut addr = addr;
                addr.sin_family = libc::AF_INET as libc::sa_family_t;
                addr.sin_addr.s_addr = u32::from(*target.ip()).to_be();
                addr.sin_port = target.port().to_be();
                libc::connect(
                    self.fd,
                    std::ptr::addr_of!(addr).cast(),
                    std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
                )
            };
            if rc < 0 {
                return Err(IcmpReceiverError::Configure(io::Error::last_os_error()));
            }
            // Re-read the local endpoint: the route has now chosen it.
            // SAFETY: `self.fd` is connected, so getsockname returns a filled
            // address for a socket of exactly this family and type.
            self.local = unsafe {
                let mut addr: libc::sockaddr_in = std::mem::zeroed();
                let mut len = std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
                libc::getsockname(
                    self.fd,
                    std::ptr::addr_of_mut!(addr).cast(),
                    std::ptr::addr_of_mut!(len),
                );
                SocketAddrV4::new(
                    Ipv4Addr::from(u32::from_be(addr.sin_addr.s_addr)),
                    u16::from_be(addr.sin_port),
                )
            };
            Ok(())
        }
    }

    /// Send `payload` to the connected target.
    ///
    /// A UDP send to any reachable address "succeeds" whether or not anything
    /// is listening, so a return value here proves nothing about the port.
    pub fn send(&self, payload: &[u8]) -> Result<(), IcmpReceiverError> {
        #[cfg(not(unix))]
        {
            let _ = payload;
            return Err(IcmpReceiverError::UnsupportedPlatform);
        }
        #[cfg(unix)]
        {
            // SAFETY: `self.fd` is live and owned; `payload` is a valid slice
            // whose real length is passed.
            let sent = unsafe { libc::send(self.fd, payload.as_ptr().cast(), payload.len(), 0) };
            if sent < 0 {
                return Err(IcmpReceiverError::Configure(io::Error::last_os_error()));
            }
            Ok(())
        }
    }
}
