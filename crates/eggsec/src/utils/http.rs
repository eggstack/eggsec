use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;

use crate::constants;

/// Honest tool identification (not evasion).
///
/// Moved from the removed `utils::stealth` module (Phase A): the only live
/// use was this identifier in loadtest/fuzzer. Evasion semantics
/// (`StealthConfig`, rotating user-agents, browser/TLS fingerprints) were dead
/// code and were removed rather than relocated.
pub fn tool_user_agent() -> String {
    format!("Eggsec/{}", env!("CARGO_PKG_VERSION"))
}

fn build_shared_client(insecure: bool) -> Option<Client> {
    let mut builder = Client::builder()
        .pool_max_idle_per_host(constants::DEFAULT_POOL_MAX_IDLE_PER_HOST)
        .pool_idle_timeout(Duration::from_secs(
            constants::DEFAULT_POOL_IDLE_TIMEOUT_SECS,
        ))
        .tcp_nodelay(true)
        .redirect(super::same_host_redirect_policy(
            constants::http::DEFAULT_MAX_REDIRECTS as usize,
        ));
    if insecure {
        builder = builder.danger_accept_invalid_certs(true);
    }
    builder.build().ok()
}

// Phase A WS6: single long-lived cloned client. Each `reqwest::Client` already
// manages its own internal connection pool; the removed N-client round-robin
// pool sharded reusable connections/TLS state with identical per-client config
// (same timeout/UA/proxy), providing no isolation property.
static SHARED_HTTP_CLIENT: std::sync::LazyLock<Client> = std::sync::LazyLock::new(|| {
    crate::install_tls_provider();
    build_shared_client(false).unwrap_or_else(|| {
        tracing::warn!(
            "Failed to create shared HTTP client with full options, using minimal client"
        );
        Client::new()
    })
});

static SHARED_INSECURE_HTTP_CLIENT: std::sync::LazyLock<Client> = std::sync::LazyLock::new(|| {
    crate::install_tls_provider();
    build_shared_client(true).unwrap_or_else(|| {
            tracing::warn!(
                "Failed to create insecure HTTP client with full options, using minimal client"
            );
            Client::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap_or_else(|e| {
                    tracing::error!(error = %e, "Failed to create insecure fallback HTTP client; using verified client");
                    Client::new()
                })
        })
});

pub fn create_http_client(timeout_secs: u64) -> Result<Client> {
    crate::install_tls_provider();
    Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .pool_max_idle_per_host(constants::DEFAULT_POOL_MAX_IDLE_PER_HOST)
        .pool_idle_timeout(Duration::from_secs(
            constants::DEFAULT_POOL_IDLE_TIMEOUT_SECS,
        ))
        .tcp_nodelay(true)
        .build()
        .context("Failed to create HTTP client")
}

pub fn get_shared_http_client() -> Client {
    crate::install_tls_provider();
    SHARED_HTTP_CLIENT.clone()
}

pub fn get_shared_insecure_http_client() -> Client {
    crate::install_tls_provider();
    tracing::warn!(
        "Using shared HTTP client with disabled TLS certificate verification; use get_shared_http_client for verified TLS"
    );
    SHARED_INSECURE_HTTP_CLIENT.clone()
}

/// Creates an HTTP client that accepts invalid TLS certificates.
///
/// # Security Warning
///
/// **This function disables TLS certificate verification.** The client will
/// accept any certificate, including self-signed, expired, or mismatched certificates.
///
/// # When to Use
///
/// - Testing against local development servers with self-signed certificates
/// - Testing behind SSL-terminating proxies or load balancers
/// - Controlled testing environments where certificate validation is not needed
///
/// # Security Risks
///
/// Using this client in production or against untrusted targets exposes
/// connections to man-in-the-middle (MITM) attacks. An attacker could:
/// - Intercept and read sensitive data transmitted over HTTPS
/// - Impersonate the target server without detection
/// - Inject malicious content into responses
///
/// # Recommendation
///
/// Only use this for testing in isolated environments. For production testing,
/// ensure proper certificates are installed on target systems.
pub fn create_insecure_http_client(timeout_secs: u64) -> Result<Client> {
    crate::install_tls_provider();
    tracing::warn!(
        "Creating HTTP client with disabled TLS certificate verification. \
         This is insecure and should only be used in isolated testing environments."
    );
    Client::builder()
        .cookie_store(true)
        .timeout(Duration::from_secs(timeout_secs))
        .pool_max_idle_per_host(constants::DEFAULT_POOL_MAX_IDLE_PER_HOST)
        .pool_idle_timeout(Duration::from_secs(
            constants::DEFAULT_POOL_IDLE_TIMEOUT_SECS,
        ))
        .tcp_nodelay(true)
        .danger_accept_invalid_certs(true)
        .build()
        .context("Failed to create insecure HTTP client")
}

pub fn create_http_client_with_proxy(timeout_secs: u64, proxy: &str) -> Result<Client> {
    crate::install_tls_provider();
    let proxy = reqwest::Proxy::http(proxy).context("Invalid proxy URL")?;

    Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .pool_max_idle_per_host(constants::DEFAULT_POOL_MAX_IDLE_PER_HOST)
        .pool_idle_timeout(Duration::from_secs(
            constants::DEFAULT_POOL_IDLE_TIMEOUT_SECS,
        ))
        .tcp_nodelay(true)
        .proxy(proxy)
        .build()
        .context("Failed to create HTTP client with proxy")
}

pub fn create_http_client_with_options<F>(timeout_secs: u64, builder_fn: F) -> Result<Client>
where
    F: FnOnce(reqwest::ClientBuilder) -> reqwest::ClientBuilder,
{
    crate::install_tls_provider();
    let builder = builder_fn(
        Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .pool_max_idle_per_host(constants::DEFAULT_POOL_MAX_IDLE_PER_HOST)
            .pool_idle_timeout(Duration::from_secs(
                constants::DEFAULT_POOL_IDLE_TIMEOUT_SECS,
            ))
            .tcp_nodelay(true),
    );
    builder.build().context("Failed to create HTTP client")
}

/// Creates an HTTP client with custom options that accepts invalid TLS certificates.
///
/// # Security Warning
///
/// **This function disables TLS certificate verification.** The client will
/// accept any certificate, including self-signed, expired, or mismatched certificates.
///
/// This is a variant of [`create_insecure_http_client`] that allows custom
/// builder options to be applied before certificate verification is disabled.
///
/// # When to Use
///
/// - Testing against local development servers with self-signed certificates
/// - Testing behind SSL-terminating proxies or load balancers
/// - Controlled testing environments where certificate validation is not needed
///
/// # Security Risks
///
/// Using this client in production or against untrusted targets exposes
/// connections to man-in-the-middle (MITM) attacks. An attacker could:
/// - Intercept and read sensitive data transmitted over HTTPS
/// - Impersonate the target server without detection
/// - Inject malicious content into responses
///
/// # Recommendation
///
/// Only use this for testing in isolated environments. For production testing,
/// ensure proper certificates are installed on target systems.
pub fn create_insecure_client_with_options<F>(timeout_secs: u64, builder_fn: F) -> Result<Client>
where
    F: FnOnce(reqwest::ClientBuilder) -> reqwest::ClientBuilder,
{
    crate::install_tls_provider();
    tracing::warn!(
        "Creating HTTP client with custom options and disabled TLS certificate verification. \
         This is insecure and should only be used in isolated testing environments."
    );
    let builder = builder_fn(
        Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .pool_max_idle_per_host(constants::DEFAULT_POOL_MAX_IDLE_PER_HOST)
            .pool_idle_timeout(Duration::from_secs(
                constants::DEFAULT_POOL_IDLE_TIMEOUT_SECS,
            ))
            .tcp_nodelay(true)
            .danger_accept_invalid_certs(true),
    );
    builder.build().context("Failed to create HTTP client")
}

/// A redirect policy that follows at most `max_redirects` redirects but only
/// when the redirect target stays on the same host as the original request.
///
/// Scoped tooling must never be bounced onto out-of-scope hosts via 3xx
/// responses (e.g., a target answering `302 -> http://169.254.169.254/`).
/// Cross-host redirects are stopped (not an error), so the redirect response
/// itself is still surfaced to the caller.
pub fn same_host_redirect_policy(max_redirects: usize) -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(move |attempt| {
        if attempt.previous().len() >= max_redirects {
            return attempt.error("too many redirects");
        }
        let same_host = match attempt.previous().first() {
            Some(original) => original.host_str() == attempt.url().host_str(),
            None => true,
        };
        if same_host {
            attempt.follow()
        } else {
            tracing::warn!(
                to = %attempt.url(),
                "blocked cross-host redirect to out-of-scope host"
            );
            attempt.stop()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_http_client() {
        let client = create_http_client(30);
        assert!(client.is_ok());
    }

    #[test]
    fn test_create_insecure_http_client() {
        let client = create_insecure_http_client(30);
        assert!(client.is_ok());
    }
}
