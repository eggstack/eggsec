//! The in-flight correlation table.
//!
//! ICMP errors arrive out of band, unordered, possibly duplicated, and possibly
//! after the probe they refer to has expired. Tying one back to the probe that
//! caused it is the only non-trivial part of the scanner, and it is pure
//! bookkeeping over an explicit `now` — there is no clock call anywhere in this
//! module, so every ordering is directly testable.
//!
//! # Memory is O(in-flight), never O(ports)
//!
//! An entry exists only between registration and expiry. A 65 535-port sweep
//! holds at most `concurrency` entries, because the driver registers a probe
//! and awaits its verdict before starting another.

use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

/// A probe awaiting an ICMP verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingProbe {
    /// The port this probe is deciding.
    port: u16,
    /// When this probe's response window closes.
    deadline: Instant,
}

/// What happened when an ICMP error was offered to the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Correlation {
    /// Matched an in-flight probe. The entry has been consumed.
    Matched { port: u16 },
    /// No in-flight probe owned this key. Expected for late errors (the probe
    /// already expired), duplicates, and another process's traffic sharing the
    /// socket. Never re-inserted: a stale error must not be allowed to
    /// misattribute to a source port the kernel later reuses.
    Orphan,
}

/// Source-port collision when registering a probe.
///
/// The kernel assigns ephemeral ports, so two concurrent probes can pick the
/// same one. Surfacing this is better than silently overwriting a live entry,
/// which would make one port's verdict be reported for another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeCollision {
    pub probe_src: Ipv4Addr,
    pub probe_src_port: u16,
}

impl std::fmt::Display for ProbeCollision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "probe source port {} on {} is already in flight",
            self.probe_src_port, self.probe_src
        )
    }
}

impl std::error::Error for ProbeCollision {}

/// Correlates out-of-band ICMP errors to in-flight probes.
#[derive(Debug)]
pub struct CorrelationTable {
    entries: HashMap<(Ipv4Addr, u16), PendingProbe>,
    /// Keys whose window closed without an error.
    expired: u64,
    /// Errors that matched no live entry.
    orphans: u64,
    /// Registrations refused because the key was already live.
    collisions: u64,
    /// Errors that matched a live entry. Kept apart from `orphans` because
    /// `HostState::derive` needs the correlated total, and reusing `orphans`
    /// for it inverts the logic at every call site.
    matched: u64,
}

impl Default for CorrelationTable {
    fn default() -> Self {
        Self::new()
    }
}

impl CorrelationTable {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            expired: 0,
            orphans: 0,
            collisions: 0,
            matched: 0,
        }
    }

    /// Register a probe awaiting a verdict, keyed by the local endpoint the
    /// kernel will quote back to us.
    pub fn register(
        &mut self,
        probe_src: Ipv4Addr,
        probe_src_port: u16,
        port: u16,
        now: Instant,
        window: Duration,
    ) -> Result<(), ProbeCollision> {
        let key = (probe_src, probe_src_port);
        if self.entries.contains_key(&key) {
            self.collisions += 1;
            return Err(ProbeCollision {
                probe_src,
                probe_src_port,
            });
        }
        self.entries.insert(
            key,
            PendingProbe {
                port,
                deadline: now + window,
            },
        );
        Ok(())
    }

    /// Offer an ICMP error to the table.
    ///
    /// Consuming the entry makes duplicate delivery idempotent: the second
    /// copy finds nothing and is counted as an orphan rather than emitting a
    /// second verdict for the same port.
    pub fn correlate(&mut self, key: (Ipv4Addr, u16)) -> Correlation {
        match self.entries.remove(&key) {
            Some(probe) => {
                self.matched += 1;
                Correlation::Matched { port: probe.port }
            }
            None => {
                self.orphans += 1;
                Correlation::Orphan
            }
        }
    }

    /// Drop entries whose window has closed, returning the ports that expired
    /// without an error.
    pub fn sweep(&mut self, now: Instant) -> Vec<u16> {
        let mut expired_ports = Vec::new();
        self.entries.retain(|_, probe| {
            if probe.deadline <= now {
                self.expired += 1;
                expired_ports.push(probe.port);
                false
            } else {
                true
            }
        });
        expired_ports
    }

    /// Number of probes still awaiting a verdict.
    pub fn in_flight(&self) -> usize {
        self.entries.len()
    }

    /// Probes that expired with no ICMP.
    pub fn expired_count(&self) -> u64 {
        self.expired
    }

    /// Errors that matched no live entry.
    pub fn orphan_count(&self) -> u64 {
        self.orphans
    }

    /// Registrations refused as key collisions.
    pub fn collision_count(&self) -> u64 {
        self.collisions
    }

    /// Errors that matched a live entry.
    pub fn matched_count(&self) -> u64 {
        self.matched
    }
}
