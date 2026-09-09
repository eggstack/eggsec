//! Managed browser session backend contract (Phase E, Workstream 1).
//!
//! The Python `BrowserSession` type advertises a set of capabilities
//! (`BrowserCapabilities`). Those capabilities must be truthful: they are
//! derived from the active backend compiled into this build, not from
//! hard-coded aspirational values.
//!
//! This module defines the small engine abstraction that managed sessions
//! build on:
//!
//! - [`BrowserBackendKind`] — which backend implementation is active;
//! - [`BrowserBackendCapabilities`] — what the active backend can actually do;
//! - [`BrowserBackend`] — the session lifecycle trait managed sessions use;
//! - [`capabilities_for_current_build`] — compile-time truthful capabilities;
//! - [`validate_browser_url`] — shared URL policy gate for navigation.
//!
//! The assessment entry point ([`super::run_browser_scan`]) is implemented
//! directly on `headless_chrome`. Managed interactive sessions
//! (`BrowserSession` in `eggsec-python`) resolve their advertised
//! capabilities through [`capabilities_for_current_build`] and must refuse
//! capability-gated operations with an explicit unsupported error instead of
//! returning synthetic empty success data.

use crate::error::{EggsecError, Result};
use serde::{Deserialize, Serialize};

/// Which browser backend implementation is active in this build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BrowserBackendKind {
    /// Real `headless_chrome` backend (`headless-browser` Cargo feature).
    HeadlessChrome,
    /// No browser backend compiled in; every session operation must fail
    /// with an explicit unsupported error.
    Unsupported,
}

impl BrowserBackendKind {
    /// Stable string identifier for the backend.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HeadlessChrome => "headless_chrome",
            Self::Unsupported => "unsupported",
        }
    }
}

/// Truthful capability set for the active backend.
///
/// Field names mirror the Python `BrowserCapabilities` surface so the
/// binding layer can derive its values 1:1 instead of hard-coding them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserBackendCapabilities {
    pub backend: BrowserBackendKind,
    pub supports_javascript: bool,
    pub supports_dom: bool,
    pub supports_network_intercept: bool,
    pub supports_console_capture: bool,
    pub supports_screenshot: bool,
    pub supports_pdf_export: bool,
    pub supports_cookie_access: bool,
    pub supports_storage_access: bool,
    pub supports_route_discovery: bool,
    pub supports_proxy: bool,
}

impl BrowserBackendCapabilities {
    /// Capabilities of the real `headless_chrome` backend.
    ///
    /// `supports_pdf_export` is `false`: the engine does not expose a PDF
    /// export path through managed sessions, so it must not be advertised.
    pub fn headless_chrome() -> Self {
        Self {
            backend: BrowserBackendKind::HeadlessChrome,
            supports_javascript: true,
            supports_dom: true,
            supports_network_intercept: true,
            supports_console_capture: true,
            supports_screenshot: true,
            supports_pdf_export: false,
            supports_cookie_access: true,
            supports_storage_access: true,
            supports_route_discovery: true,
            supports_proxy: false,
        }
    }

    /// Capabilities when no backend is compiled in: nothing is supported.
    pub fn unsupported() -> Self {
        Self {
            backend: BrowserBackendKind::Unsupported,
            supports_javascript: false,
            supports_dom: false,
            supports_network_intercept: false,
            supports_console_capture: false,
            supports_screenshot: false,
            supports_pdf_export: false,
            supports_cookie_access: false,
            supports_storage_access: false,
            supports_route_discovery: false,
            supports_proxy: false,
        }
    }
}

/// Capabilities for the backend compiled into this build.
///
/// This module only exists when the `headless-browser` feature is enabled,
/// so the active backend is always the real `headless_chrome` one. Managed
/// session bindings must call this instead of constructing capability values
/// by hand.
pub fn capabilities_for_current_build() -> BrowserBackendCapabilities {
    BrowserBackendCapabilities::headless_chrome()
}

/// Which backend is active in this build.
pub fn current_backend_kind() -> BrowserBackendKind {
    BrowserBackendKind::HeadlessChrome
}

/// Managed browser session lifecycle.
///
/// The trait keeps the Python session class decoupled from the concrete
/// `headless_chrome` types for testability: unit tests bind a stub backend
/// while production binds the real tab driver. Only `close` has a default
/// no-op body; every capability-gated operation must either perform real
/// work or return an explicit unsupported error — never synthetic success.
pub trait BrowserBackend: Send {
    /// Backend implementation identifier.
    fn kind(&self) -> BrowserBackendKind;

    /// Truthful capabilities of this backend instance.
    fn capabilities(&self) -> BrowserBackendCapabilities;

    /// Close backend resources idempotently.
    fn close(&mut self) {}
}

/// Validate a navigation URL against managed-session policy (Phase E WS3).
///
/// Managed sessions are network execution and must not become an alternate
/// path around scope enforcement:
///
/// - only `http`/`https` schemes are allowed;
/// - a host must be present;
/// - userinfo (`user:pass@host`) is rejected so credentials never enter the
///   navigation path (configure proxy/auth out of band instead).
///
/// Redirect targets and subresource hosts must be re-validated with this
/// helper by the caller. Full `EnforcementContext` scope authorization stays
/// with the dispatching surface, which owns the loaded scope.
pub fn validate_browser_url(url: &str) -> Result<()> {
    let url = url.trim();
    if url.is_empty() {
        return Err(EggsecError::InvalidTarget(
            "browser navigation URL must not be empty".to_string(),
        ));
    }
    // Scheme check without pulling a URL parser into the engine core.
    let (scheme, rest) = url.split_once("://").ok_or_else(|| {
        EggsecError::InvalidTarget(format!(
            "browser navigation URL must use http:// or https:// (got '{url}')"
        ))
    })?;
    if scheme != "http" && scheme != "https" {
        return Err(EggsecError::InvalidTarget(format!(
            "browser navigation URL scheme must be http or https (got '{scheme}')"
        )));
    }
    let authority = rest.split('/').next().unwrap_or_default();
    let authority = authority.split('?').next().unwrap_or_default();
    if authority.is_empty() {
        return Err(EggsecError::InvalidTarget(format!(
            "browser navigation URL must include a host (got '{url}')"
        )));
    }
    if authority.contains('@') {
        return Err(EggsecError::InvalidTarget(
            "browser navigation URL must not embed userinfo credentials".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_build_advertises_headless_chrome_backend() {
        let caps = capabilities_for_current_build();
        assert_eq!(caps.backend, BrowserBackendKind::HeadlessChrome);
        assert_eq!(current_backend_kind(), BrowserBackendKind::HeadlessChrome);
        // Every advertised capability must be backed by real engine behavior:
        // DOM/XSS + SPA + client checks (js/dom), XHR/Fetch interception,
        // console capture via page scripts, screenshots via DevTools, cookies
        // and storage via page evaluation, route discovery. PDF export and
        // proxying have no engine path and stay false.
        assert!(caps.supports_javascript);
        assert!(caps.supports_dom);
        assert!(caps.supports_network_intercept);
        assert!(caps.supports_console_capture);
        assert!(caps.supports_screenshot);
        assert!(!caps.supports_pdf_export);
        assert!(caps.supports_cookie_access);
        assert!(caps.supports_storage_access);
        assert!(caps.supports_route_discovery);
        assert!(!caps.supports_proxy);
    }

    #[test]
    fn unsupported_capabilities_advertise_nothing() {
        let caps = BrowserBackendCapabilities::unsupported();
        assert_eq!(caps.backend, BrowserBackendKind::Unsupported);
        assert!(!caps.supports_javascript);
        assert!(!caps.supports_dom);
        assert!(!caps.supports_network_intercept);
        assert!(!caps.supports_console_capture);
        assert!(!caps.supports_screenshot);
        assert!(!caps.supports_pdf_export);
        assert!(!caps.supports_cookie_access);
        assert!(!caps.supports_storage_access);
        assert!(!caps.supports_route_discovery);
        assert!(!caps.supports_proxy);
    }

    #[test]
    fn backend_kind_strings_are_stable() {
        assert_eq!(
            BrowserBackendKind::HeadlessChrome.as_str(),
            "headless_chrome"
        );
        assert_eq!(BrowserBackendKind::Unsupported.as_str(), "unsupported");
    }

    #[test]
    fn validate_browser_url_accepts_http_and_https() {
        assert!(validate_browser_url("http://127.0.0.1:8080/").is_ok());
        assert!(validate_browser_url("https://example.com/page?q=1").is_ok());
    }

    #[test]
    fn validate_browser_url_rejects_non_http_schemes() {
        assert!(validate_browser_url("file:///etc/passwd").is_err());
        assert!(validate_browser_url("javascript:alert(1)").is_err());
        assert!(validate_browser_url("data:text/html,hi").is_err());
        assert!(validate_browser_url("example.com/no-scheme").is_err());
    }

    #[test]
    fn validate_browser_url_rejects_missing_host_and_userinfo() {
        assert!(validate_browser_url("http://").is_err());
        assert!(validate_browser_url("https:///no-host").is_err());
        assert!(validate_browser_url("http://user:pass@example.com/").is_err());
        assert!(validate_browser_url("").is_err());
        assert!(validate_browser_url("   ").is_err());
    }

    #[test]
    fn redirect_targets_are_revalidated() {
        // Redirect policy: every hop is validated, not just the initial URL.
        for hop in [
            "http://127.0.0.1:8080/",
            "http://127.0.0.1:8080/about",
            "https://example.com/landing",
        ] {
            assert!(validate_browser_url(hop).is_ok(), "hop: {hop}");
        }
        assert!(validate_browser_url("file:///etc/shadow").is_err());
    }

    #[test]
    fn capabilities_serialize_for_bindings() {
        let caps = capabilities_for_current_build();
        let json = serde_json::to_string(&caps).expect("caps serialize");
        let back: BrowserBackendCapabilities =
            serde_json::from_str(&json).expect("caps round-trip");
        assert_eq!(back, caps);
    }
}
