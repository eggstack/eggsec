//! Address classification for scope evaluation (Phase D WS7).
//!
//! Cohesive module extracted from `config/scope.rs`: IP address facts only.
//! The resolver reports facts; policy decides authorization.
//!
//! Stable facade: `config/scope.rs` re-exports everything here, so
//! `crate::config::scope::{AddressClass, classify_address, is_private_ip}`
//! and `crate::config::{AddressClass, classify_address, is_private_ip}`
//! keep working.

use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// Classification of an IP address based on RFC definitions and Eggsec policy.
///
/// Used by scope evaluation to determine authorization for each resolved address.
/// The resolver reports facts; policy decides whether they are authorized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AddressClass {
    /// Public routable address (e.g. 8.8.8.8, 2001:4860:4860::8888).
    Public,
    /// Loopback address (127.0.0.0/8, ::1, IPv4-mapped loopback).
    Loopback,
    /// RFC 1918 private (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16) or IPv6 ULA (fc00::/7).
    Private,
    /// Link-local address (169.254.0.0/16, fe80::/10).
    LinkLocal,
    /// IPv4-mapped IPv6 loopback (::ffff:127.0.0.1).
    IPv4MappedLoopback,
    /// Unspecified address (0.0.0.0, ::).
    Unspecified,
    /// Multicast address (224.0.0.0/4, ff00::/8).
    Multicast,
}

impl AddressClass {
    /// Stable kebab-case string for audit and decision records.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Loopback => "loopback",
            Self::Private => "private",
            Self::LinkLocal => "link-local",
            Self::IPv4MappedLoopback => "ipv4-mapped-loopback",
            Self::Unspecified => "unspecified",
            Self::Multicast => "multicast",
        }
    }

    /// Returns `true` if this address class is non-public (loopback, private, link-local,
    /// IPv4-mapped loopback, unspecified, or multicast).
    ///
    /// Used by scope authorization to determine if an address requires explicit scope rules.
    /// Public addresses are allowed by default; non-public addresses require explicit authorization.
    pub fn is_non_public(&self) -> bool {
        !matches!(self, Self::Public)
    }
}

impl std::fmt::Display for AddressClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Classify an IP address into its [`AddressClass`].
///
/// This function reports facts only — it does not authorize or reject.
/// Policy evaluation uses the class to determine scope compliance.
pub fn classify_address(ip: &IpAddr) -> AddressClass {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            if v4.is_loopback() || octets[0] == 127 {
                AddressClass::Loopback
            } else if octets[0] == 0 {
                AddressClass::Unspecified
            } else if octets[0] >= 224 && octets[0] <= 239 {
                AddressClass::Multicast
            } else if octets[0] == 10
                || (octets[0] == 172 && (16..=31).contains(&octets[1]))
                || (octets[0] == 192 && octets[1] == 168)
            {
                AddressClass::Private
            } else if octets[0] == 169 && octets[1] == 254 {
                AddressClass::LinkLocal
            } else {
                AddressClass::Public
            }
        }
        IpAddr::V6(v6) => {
            // Check for IPv4-mapped addresses first (before loopback check)
            if let Some(v4) = v6.to_ipv4_mapped() {
                // IPv4-mapped IPv6: ::ffff:a.b.c.d — classify the embedded v4
                // but use IPv4MappedLoopback for mapped loopback addresses
                if v4.is_loopback() {
                    AddressClass::IPv4MappedLoopback
                } else {
                    classify_address(&IpAddr::V4(v4))
                }
            } else if v6.is_loopback() {
                AddressClass::Loopback
            } else if v6.is_unspecified() {
                AddressClass::Unspecified
            } else if (v6.segments()[0] & 0xff00) == 0xff00 {
                AddressClass::Multicast
            } else if (v6.segments()[0] & 0xfe00) == 0xfc00 {
                AddressClass::Private
            } else if (v6.segments()[0] & 0xffc0) == 0xfe80 {
                AddressClass::LinkLocal
            } else {
                AddressClass::Public
            }
        }
    }
}

/// Legacy broad check: returns `true` for any address that is not globally
/// routable (RFC1918 private, loopback, link-local, IPv6 ULA / link-local).
///
/// This is intentionally broader than [`classify_address`]: it groups loopback
/// and link-local alongside true RFC1918 addresses. New policy code should
/// prefer [`classify_address`] and check `AddressClass::Private` explicitly so
/// loopback/link-local are handled separately.
pub fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            octets[0] == 10
                || (octets[0] == 172 && (16..=31).contains(&octets[1]))
                || (octets[0] == 192 && octets[1] == 168)
                || (octets[0] == 169 && octets[1] == 254)
                || (octets[0] == 127)
        }
        IpAddr::V6(ipv6) => {
            ipv6.is_loopback()
                || (ipv6.segments()[0] & 0xfe00) == 0xfc00
                || (ipv6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn classify_loopback_v4() {
        let ip = IpAddr::from_str("127.0.0.1").expect("loopback");
        assert_eq!(classify_address(&ip), AddressClass::Loopback);
    }

    #[test]
    fn classify_private_v4() {
        let ip = IpAddr::from_str("192.168.1.1").expect("private");
        assert_eq!(classify_address(&ip), AddressClass::Private);
    }

    #[test]
    fn classify_public_v4() {
        let ip = IpAddr::from_str("8.8.8.8").expect("public");
        assert_eq!(classify_address(&ip), AddressClass::Public);
    }

    #[test]
    fn private_check_is_broader_than_classify() {
        // Loopback is not `Private` but is covered by the legacy broad check.
        let loopback = IpAddr::from_str("127.0.0.1").expect("loopback");
        assert_ne!(classify_address(&loopback), AddressClass::Private);
        assert!(is_private_ip(&loopback));
    }
}
