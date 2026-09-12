//! `NetworkAuthority` binding over the canonical scope model (Phase B).
//!
//! [`ScopeAuthority`] implements [`eggsec_transport::NetworkAuthority`] using
//! the single canonical policy (`Scope` / `TargetScope` + `classify_address`).
//! No second target-policy language is introduced: every checkpoint
//! delegates to engine scope evaluation with DNS facts supplied by the
//! transport (TOCTOU-closed — the authority never re-resolves through a
//! second path).
//!
//! Binding rules:
//! - [`authorize_resolved`](eggsec_transport::NetworkAuthority::authorize_resolved)
//!   approves a non-empty subset only when **all** candidates are allowed
//!   (mixed authorized/unauthorized answers deny, Phase A test 3) and no
//!   candidate matches an exclusion.
//! - Direct IP literals skip DNS but still pass CIDR + port checks
//!   (Phase A test 10).
//! - Proxy endpoints and ultimate destinations are distinct decisions
//!   (Phase A test 11): authorizing one never authorizes the other.
//! - Insecure TLS never changes authorization (Phase A test 12).

use super::scope::{classify_address, AddressClass, Scope, TargetScope};
use eggsec_transport::{PolicyCheckpoint, TransportError};
use std::net::IpAddr;
use url::Url;

/// Canonical scope authority for outbound transport.
///
/// Borrows the engine [`Scope`]; the transport supplies DNS facts per call.
#[derive(Debug)]
pub struct ScopeAuthority<'a> {
    scope: &'a Scope,
}

impl<'a> ScopeAuthority<'a> {
    /// Bind `scope` as the policy authority (no copy, no second language).
    pub fn new(scope: &'a Scope) -> Self {
        Self { scope }
    }

    fn check_port(
        &self,
        port: Option<u16>,
        checkpoint: PolicyCheckpoint,
    ) -> Result<(), TransportError> {
        if let Some(p) = port {
            if !self.scope.is_port_allowed(p) {
                return Err(TransportError::denied(
                    checkpoint,
                    format!("port {p} is not in allowed scope"),
                ));
            }
        }
        Ok(())
    }

    fn target_for_candidates(host: &str, candidates: &[IpAddr]) -> TargetScope {
        TargetScope {
            host: host.to_string(),
            ip: candidates.first().copied(),
            resolved_addresses: candidates.to_vec(),
        }
    }

    fn ensure_all_allowed(
        &self,
        host: &str,
        candidates: &[IpAddr],
        checkpoint: PolicyCheckpoint,
    ) -> Result<Vec<IpAddr>, TransportError> {
        if candidates.is_empty() {
            return Err(TransportError::denied(
                checkpoint,
                format!("no resolved addresses for '{host}'"),
            ));
        }
        let target = Self::target_for_candidates(host, candidates);

        // Explicit exclusions deny first (hostname or any address).
        // `is_explicitly_excluded` is private; replicate via public ScopeRule
        // matching on host + per-address CIDR checks through evaluate_addresses.
        let (all_allowed, any_excluded, _) =
            target.evaluate_addresses(&self.scope.allowed_targets, &self.scope.excluded_targets);

        // Hostname-only exclusion (patterns like `evil.example`) when no
        // addresses matched CIDR exclusions: check pattern match directly.
        // `evaluate_addresses` already covers address exclusions; check host
        // patterns here for completeness.
        let host_excluded = self
            .scope
            .excluded_targets
            .iter()
            .any(|rule| rule.matches(&target));
        if any_excluded || host_excluded {
            return Err(TransportError::denied(
                checkpoint,
                format!("'{host}' matches an exclusion rule"),
            ));
        }

        if self.scope.allowed_targets.is_empty() {
            if self.scope.require_explicit_scope {
                return Err(TransportError::denied(
                    checkpoint,
                    "explicit scope required but no allowed targets configured",
                ));
            }
            // No rules: block non-public addresses (loopback exempt), mirroring
            // `Scope::is_target_allowed_with_resolver`.
            let blocked: Vec<AddressClass> = candidates
                .iter()
                .map(classify_address)
                .filter(|c| *c != AddressClass::Loopback && c.is_non_public())
                .collect();
            if !blocked.is_empty() {
                return Err(TransportError::denied(
                    checkpoint,
                    format!("non-public address blocked by default policy ({blocked:?})"),
                ));
            }
            return Ok(candidates.to_vec());
        }

        if !all_allowed {
            return Err(TransportError::denied(
                checkpoint,
                format!("not all resolved addresses for '{host}' are in allowed scope"),
            ));
        }

        // Hostname patterns must also match when present: `evaluate_addresses`
        // already requires per-address allowance (CIDR or host match), but
        // when candidates are IPs the host pattern alone is insufficient —
        // the above `all_allowed` covers it. Return the full set (no silent
        // subsetting: mixed answers deny rather than filter).
        Ok(candidates.to_vec())
    }
}

impl eggsec_transport::NetworkAuthority for ScopeAuthority<'_> {
    fn authorize_initial_url(&self, url: &Url) -> Result<(), TransportError> {
        eggsec_transport::reject_url_userinfo(url)?;
        if url.host_str().is_none_or(|h| h.is_empty()) {
            return Err(TransportError::denied(
                PolicyCheckpoint::InitialUrl,
                "URL has no host",
            ));
        }
        Ok(())
    }

    fn authorize_host(
        &self,
        host: &str,
        port: Option<u16>,
        is_ip_literal: bool,
    ) -> Result<(), TransportError> {
        if host.is_empty() {
            return Err(TransportError::denied(PolicyCheckpoint::Host, "empty host"));
        }
        self.check_port(port, PolicyCheckpoint::Host)?;
        if is_ip_literal {
            let addr: IpAddr = host
                .parse()
                .map_err(|_| TransportError::denied(PolicyCheckpoint::Host, "bad IP literal"))?;
            // Direct IP: CIDR policy applies without DNS.
            let approved = self.ensure_all_allowed(host, &[addr], PolicyCheckpoint::Host)?;
            if approved.is_empty() {
                return Err(TransportError::denied(
                    PolicyCheckpoint::Host,
                    "direct IP not in allowed scope",
                ));
            }
        } else {
            // Hostname: pattern pre-check without DNS (full IP check happens
            // at the DNS checkpoint with transport-supplied facts).
            let probe = TargetScope {
                host: host.to_string(),
                ip: None,
                resolved_addresses: Vec::new(),
            };
            let host_matches = self.scope.allowed_targets.is_empty()
                || self.scope.allowed_targets.iter().any(|rule| {
                    if rule.cidr.is_some() {
                        // CIDR-only rules cannot decide without IPs; defer to DNS.
                        return true;
                    }
                    if rule.pattern.is_empty() {
                        return false;
                    }
                    rule.matches(&probe)
                });
            // When only CIDR rules exist, hostnames pass here and are gated
            // at DNS time (fail-closed there). When hostname patterns exist
            // and none match, deny early.
            let has_hostname_rules = self
                .scope
                .allowed_targets
                .iter()
                .any(|r| !r.pattern.is_empty() && !r.pattern.contains('/'));
            if has_hostname_rules && !host_matches {
                // Confirm via exclusion-aware check: if explicitly excluded,
                // report that; otherwise report not-in-scope.
                if self
                    .scope
                    .excluded_targets
                    .iter()
                    .any(|rule| rule.matches(&probe))
                {
                    return Err(TransportError::denied(
                        PolicyCheckpoint::Host,
                        format!("'{host}' matches an exclusion rule"),
                    ));
                }
                return Err(TransportError::denied(
                    PolicyCheckpoint::Host,
                    format!("'{host}' is not in allowed scope"),
                ));
            }
        }
        Ok(())
    }

    fn authorize_resolved(
        &self,
        host: &str,
        candidates: &[IpAddr],
    ) -> Result<Vec<IpAddr>, TransportError> {
        self.ensure_all_allowed(host, candidates, PolicyCheckpoint::Dns)
    }

    fn authorize_socket(&self, host: &str, addr: IpAddr, port: u16) -> Result<(), TransportError> {
        self.check_port(Some(port), PolicyCheckpoint::Socket)?;
        // The address actually dialed must still be in scope (rebinding
        // closed: transport guarantees `addr` ∈ approved ⊆ candidates, and we
        // re-verify against policy here).
        self.ensure_all_allowed(host, &[addr], PolicyCheckpoint::Socket)?;
        Ok(())
    }

    fn authorize_redirect(&self, _from: &Url, to: &Url) -> Result<(), TransportError> {
        self.authorize_initial_url(to).map_err(|e| match e {
            TransportError::PolicyDenied { reason, .. } => {
                TransportError::denied(PolicyCheckpoint::Redirect, reason)
            }
            other => other,
        })?;
        let host = to.host_str().ok_or_else(|| {
            TransportError::denied(PolicyCheckpoint::Redirect, "redirect target has no host")
        })?;
        let port = to.port_or_known_default();
        let is_literal = host.parse::<IpAddr>().is_ok();
        self.authorize_host(host, port, is_literal)
            .map_err(|e| match e {
                TransportError::PolicyDenied { reason, .. } => {
                    TransportError::denied(PolicyCheckpoint::Redirect, reason)
                }
                other => other,
            })
    }

    fn authorize_proxy(&self, proxy_endpoint: &Url, ultimate: &Url) -> Result<(), TransportError> {
        // Distinct decisions: proxy endpoint and ultimate destination are
        // authorized independently; neither implies the other.
        let proxy_host = proxy_endpoint.host_str().ok_or_else(|| {
            TransportError::denied(PolicyCheckpoint::Proxy, "proxy endpoint has no host")
        })?;
        let ultimate_host = ultimate.host_str().ok_or_else(|| {
            TransportError::denied(PolicyCheckpoint::Proxy, "ultimate destination has no host")
        })?;
        for (label, host, url) in [
            ("proxy endpoint", proxy_host, proxy_endpoint),
            ("ultimate destination", ultimate_host, ultimate),
        ] {
            eggsec_transport::reject_url_userinfo(url).map_err(|e| match e {
                TransportError::PolicyDenied { reason, .. } => {
                    TransportError::denied(PolicyCheckpoint::Proxy, format!("{label}: {reason}"))
                }
                other => other,
            })?;
            let port = url.port_or_known_default();
            let is_literal = host.parse::<IpAddr>().is_ok();
            // Pattern + port pre-check; full IP checks happen at DNS/socket
            // time for each endpoint separately.
            self.authorize_host(host, port, is_literal)
                .map_err(|e| match e {
                    TransportError::PolicyDenied { reason, .. } => TransportError::denied(
                        PolicyCheckpoint::Proxy,
                        format!("{label} '{host}': {reason}"),
                    ),
                    other => other,
                })?;
        }
        Ok(())
    }

    fn check_tls_consistency(
        &self,
        request_host: &str,
        sni_override: Option<&str>,
        host_override: Option<&str>,
    ) -> Result<(), TransportError> {
        if let Some(sni) = sni_override {
            if !sni.eq_ignore_ascii_case(request_host) {
                return Err(TransportError::denied(
                    PolicyCheckpoint::TlsConsistency,
                    "SNI override does not match request host",
                ));
            }
        }
        if let Some(host) = host_override {
            if !host.eq_ignore_ascii_case(request_host) {
                return Err(TransportError::denied(
                    PolicyCheckpoint::TlsConsistency,
                    "Host override does not match request host",
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ScopeRule;
    use eggsec_transport::NetworkAuthority;

    fn scope_with_patterns(patterns: &[&str]) -> Scope {
        let mut scope = Scope::new();
        for p in patterns {
            scope.allowed_targets.push(ScopeRule::new(p.to_string()));
        }
        scope
    }

    fn scope_with_cidr(cidr: &str) -> Scope {
        let mut scope = Scope::new();
        scope
            .allowed_targets
            .push(ScopeRule::with_cidr(cidr.to_string()).expect("cidr"));
        scope
    }

    #[test]
    fn userinfo_rejected() {
        let scope = Scope::new();
        let auth = ScopeAuthority::new(&scope);
        let url = Url::parse("http://user:pw@example.com/").expect("url");
        assert!(auth.authorize_initial_url(&url).is_err());
    }

    #[test]
    fn mixed_dns_answers_deny() {
        let scope = scope_with_cidr("93.184.216.0/24");
        let auth = ScopeAuthority::new(&scope);
        let mixed = vec![
            "93.184.216.34".parse().expect("ip"),
            "203.0.113.99".parse().expect("ip"),
        ];
        assert!(auth.authorize_resolved("mixed.example", &mixed).is_err());
    }

    #[test]
    fn proxy_and_target_are_distinct() {
        let scope = scope_with_patterns(&["example.com"]);
        let auth = ScopeAuthority::new(&scope);
        let proxy = Url::parse("http://proxy.internal:8080").expect("proxy");
        let ultimate = Url::parse("http://example.com/").expect("url");
        // Proxy endpoint not in scope → deny even though ultimate is allowed.
        assert!(auth.authorize_proxy(&proxy, &ultimate).is_err());
        let proxy_ok = Url::parse("http://example.com:8080").expect("proxy");
        assert!(auth.authorize_proxy(&proxy_ok, &ultimate).is_ok());
    }

    #[test]
    fn direct_ip_checked_without_dns() {
        let mut scope = Scope::new();
        scope
            .allowed_targets
            .push(ScopeRule::with_cidr("10.0.0.0/8".to_string()).expect("cidr"));
        let auth = ScopeAuthority::new(&scope);
        assert!(auth.authorize_host("10.0.0.5", Some(80), true).is_ok());
        assert!(auth.authorize_host("11.0.0.5", Some(80), true).is_err());
    }
}
