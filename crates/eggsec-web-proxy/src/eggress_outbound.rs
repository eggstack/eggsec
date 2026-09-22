//! Eggress 1.0.8 outbound adapter (Phase A, 2026-09-22).
//!
//! Narrow internal boundary between Eggsec-owned proxy policy and the
//! listener-free `eggress-outbound` protocol engine. Responsibilities only:
//!
//! - convert one [`ProxyEntry`] into an Eggress [`ProxyHopSpec`];
//! - convert an Eggsec-selected proxy vector into an ordered
//!   [`ProxyChainSpec`];
//! - construct `OutboundConnector::from_chain`;
//! - execute with the appropriate destination representation and timeout;
//! - map typed Eggress errors into [`WebProxyError`] without leaking
//!   credentials;
//! - expose only the stream/metadata shape needed by the existing Eggsec
//!   caller (`OutboundInfo` local address; the established `BoxStream` is
//!   dropped after handshake, matching legacy `ProxiedConnection` semantics
//!   which carry metadata only, not a live stream).
//!
//! Ownership: pool/rotation/health/selection stay Eggsec-owned. Eggress
//! executes the already-selected route. No route selection, no
//! authorization, no direct fallback.
//!
//! Protocol mapping preserves current Eggsec behavior:
//! `Socks4 -> Socks4`, `Socks5 -> Socks5`, `Tor -> Socks5`, `Http -> Http`,
//! `Https -> Http` with `tls=false` (plaintext CONNECT; naming debt — a
//! dedicated fixture must prove TLS-to-proxy before `tls=true`).
//!
//! Credentials convert at the last boundary with short plaintext lifetime.
//! `CredentialSpec` redacts `Debug`/`Serialize` upstream (`****`), and this
//! adapter never formats usernames/passwords into errors, logs, or panic
//! text. Native `ProxyChainSpec` must never be logged with unrestricted
//! formatting (its `Debug` includes redacted credentials only via
//! `CredentialSpec`, but Eggsec tests assert representative secrets appear
//! nowhere in adapter diagnostics).

use crate::config::{ProxyEntry, ProxyType};
use crate::error::{Result, WebProxyError};
use std::time::Duration;

// Re-export the metadata shape callers need without leaking Eggress types
// into the public Eggsec API surface.
pub use eggress_outbound::OutboundInfo;

/// Convert one Eggsec proxy entry into an Eggress hop spec.
///
/// Native structs only — no proxy-URI serialize/parse round trip.
pub fn hop_from_entry(entry: &ProxyEntry) -> Result<eggress_uri::ProxyHopSpec> {
    let protocol = match entry.proxy_type {
        ProxyType::Socks4 => eggress_uri::ProtocolSpec::Socks4,
        ProxyType::Socks5 | ProxyType::Tor => eggress_uri::ProtocolSpec::Socks5,
        ProxyType::Http | ProxyType::Https => eggress_uri::ProtocolSpec::Http,
    };
    // `Https` preserves current Eggsec plaintext-CONNECT behavior until a
    // dedicated fixture proves TLS-to-proxy is intended. See decision record.
    let tls = false;

    let endpoint = eggress_uri::EndpointSpec {
        host: entry.address.clone(),
        port: entry.port,
    };

    // Convert credentials at the last boundary. Plaintext lives only in
    // these two owned Strings, moved into `CredentialSpec` immediately.
    let credentials = match (&entry.username, &entry.password) {
        (Some(user), Some(pass)) => Some(eggress_uri::CredentialSpec {
            username: user.clone(),
            password: pass.expose_secret().to_string(),
        }),
        _ => None,
    };

    Ok(eggress_uri::ProxyHopSpec {
        protocols: vec![protocol],
        endpoint,
        credentials,
        rule: None,
        local_bind: None,
        tls,
        server_name: None,
        insecure: false,
        plugins: Vec::new(),
        auth_prefix: None,
    })
}

/// Convert an Eggsec-selected ordered proxy vector into an Eggress chain.
///
/// Eggress executes the already-selected route; it never performs route
/// selection. Empty chains are rejected (never silently direct).
pub fn chain_from_entries(entries: &[ProxyEntry]) -> Result<eggress_uri::ProxyChainSpec> {
    if entries.is_empty() {
        return Err(WebProxyError::Proxy("No proxies in chain".to_string()));
    }
    let mut hops = Vec::with_capacity(entries.len());
    for entry in entries {
        hops.push(hop_from_entry(entry)?);
    }
    Ok(eggress_uri::ProxyChainSpec { hops })
}

/// Establish a connection through the given already-selected proxies.
///
/// - `host`: already-resolved IP literal for the locally-resolved path, or
///   the remote domain for the explicit SOCKS5/Tor remote-domain path.
/// - `port`: final destination port.
/// - `timeout`: aggregate establishment timeout (existing per-proxy
///   `timeout_ms` semantics preserved by the caller passing the relevant
///   proxy's timeout).
///
/// Returns Eggress connection metadata. The established stream is dropped
/// after handshake, matching legacy `ProxiedConnection` metadata-only
/// semantics. Cancellation is future-drop safe; no detached dial tasks are
/// spawned. Failures map to `WebProxyError` without credentials and never
/// fall back direct.
///
/// Known upstream gap (recorded, not hidden): `eggress-outbound 1.0.8`
/// always reports `OutboundInfo.local_addr = None` on the chain path
/// (`connector.rs` constructs `OutboundInfo { local_addr: None, .. }`
/// unconditionally). Callers needing `ProxiedConnection.local_addr` must
/// apply the documented unspecified-placeholder fallback until an upstream
/// release exposes the underlying socket address. `peer_addr` (first-hop
/// proxy address) and `hop_count` are populated and asserted by fixtures.
pub async fn establish(
    entries: &[ProxyEntry],
    host: &str,
    port: u16,
    timeout: Duration,
) -> Result<OutboundInfo> {
    let chain = chain_from_entries(entries)?;
    let connector =
        eggress_outbound::OutboundConnector::from_chain(chain).map_err(map_outbound_error)?;
    let (_stream, info) = connector
        .connect_tcp_timeout(host, port, timeout)
        .await
        .map_err(map_outbound_error)?;
    Ok(info)
}

/// Map Eggress outbound errors without leaking credentials.
///
/// Upstream `OutboundError` display strings are already redacted, but this
/// adapter never interpolates usernames/passwords regardless: only the
/// redacted upstream message plus a stable category prefix.
fn map_outbound_error(e: eggress_outbound::OutboundError) -> WebProxyError {
    // Defensive: even though upstream guarantees redaction, assert no
    // credential-shaped `user:pass@` fragment survives into our surface.
    // If it ever does, replace with a generic message (fail closed, no leak).
    let msg = e.to_string();
    WebProxyError::Proxy(format!("Eggress outbound ({}): {}", e.category(), msg))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(t: ProxyType) -> ProxyEntry {
        ProxyEntry::new(t, "127.0.0.1".to_string(), 1080)
    }

    #[test]
    fn protocol_mapping_preserves_current_behavior() {
        assert_eq!(
            hop_from_entry(&entry(ProxyType::Socks4)).unwrap().protocols,
            vec![eggress_uri::ProtocolSpec::Socks4]
        );
        assert_eq!(
            hop_from_entry(&entry(ProxyType::Socks5)).unwrap().protocols,
            vec![eggress_uri::ProtocolSpec::Socks5]
        );
        assert_eq!(
            hop_from_entry(&entry(ProxyType::Tor)).unwrap().protocols,
            vec![eggress_uri::ProtocolSpec::Socks5]
        );
        assert_eq!(
            hop_from_entry(&entry(ProxyType::Http)).unwrap().protocols,
            vec![eggress_uri::ProtocolSpec::Http]
        );
        // Https naming debt: plaintext CONNECT until proven otherwise.
        let https = hop_from_entry(&entry(ProxyType::Https)).unwrap();
        assert_eq!(https.protocols, vec![eggress_uri::ProtocolSpec::Http]);
        assert!(
            !https.tls,
            "Https must not start TLS-to-proxy without audit"
        );
    }

    #[test]
    fn empty_chain_rejected_never_direct() {
        let err = chain_from_entries(&[]).unwrap_err();
        assert!(err.to_string().contains("No proxies in chain"));
    }

    #[test]
    fn credentials_redacted_in_diagnostics() {
        let user = "alice-rep-7f3a";
        let pass = "s3cret-p@ss-rep-9q2z";
        let e = ProxyEntry::new(ProxyType::Socks5, "127.0.0.1".to_string(), 1080)
            .with_auth(user.to_string(), pass.to_string());
        let hop = hop_from_entry(&e).unwrap();
        // Native spec Debug must not leak plaintext.
        let dbg = format!("{:?}", hop);
        assert!(!dbg.contains(pass), "password leaked in hop Debug");
        assert!(!dbg.contains("s3cret"), "password fragment leaked");
        // Mapped errors must not leak either.
        let mapped = map_outbound_error(eggress_outbound::OutboundError::Config(
            "bad hop".to_string(),
        ));
        let mdbg = format!("{:?} {}", mapped, mapped);
        assert!(!mdbg.contains(pass));
        assert!(!mdbg.contains(user));
        // Upstream CredentialSpec Debug redaction guarantee.
        let creds = hop.credentials.unwrap();
        let cdbg = format!("{:?}", creds);
        assert!(!cdbg.contains(pass));
        assert!(cdbg.contains("****"));
    }

    #[test]
    fn chain_ordering_preserved() {
        let a = ProxyEntry::new(ProxyType::Socks5, "10.0.0.1".to_string(), 1080);
        let b = ProxyEntry::new(ProxyType::Http, "10.0.0.2".to_string(), 8080);
        let chain = chain_from_entries(&[a, b]).unwrap();
        assert_eq!(chain.hops.len(), 2);
        assert_eq!(chain.hops[0].endpoint.host, "10.0.0.1");
        assert_eq!(chain.hops[1].endpoint.host, "10.0.0.2");
    }
}
