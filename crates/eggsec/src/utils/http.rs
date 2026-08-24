use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;

use super::client_pool::ClientPool;
use crate::constants;

static HTTP_CLIENT_POOL: std::sync::LazyLock<ClientPool> = std::sync::LazyLock::new(|| {
    ClientPool::new(
        10,
        Duration::from_secs(constants::DEFAULT_POOL_IDLE_TIMEOUT_SECS),
        false,
        None,
        None,
    )
});

static INSECURE_HTTP_CLIENT_POOL: std::sync::LazyLock<ClientPool> =
    std::sync::LazyLock::new(|| {
        ClientPool::new(
            10,
            Duration::from_secs(constants::DEFAULT_POOL_IDLE_TIMEOUT_SECS),
            true,
            None,
            None,
        )
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
    HTTP_CLIENT_POOL.get().unwrap_or_else(|| {
        // First try with full options
        if let Ok(client) = Client::builder()
            .pool_max_idle_per_host(constants::DEFAULT_POOL_MAX_IDLE_PER_HOST)
            .pool_idle_timeout(Duration::from_secs(
                constants::DEFAULT_POOL_IDLE_TIMEOUT_SECS,
            ))
            .tcp_nodelay(true)
            .build()
        {
            return client;
        }

        // Fallback to minimal client
        tracing::warn!("Failed to create HTTP client with full options, using minimal client");
        Client::new()
    })
}

pub fn get_shared_insecure_http_client() -> Client {
    crate::install_tls_provider();
    tracing::warn!(
        "Using shared HTTP client with disabled TLS certificate verification; use get_shared_http_client for verified TLS"
    );
    INSECURE_HTTP_CLIENT_POOL.get().unwrap_or_else(|| {
        // First try with full options
        if let Ok(client) = Client::builder()
            .pool_max_idle_per_host(constants::DEFAULT_POOL_MAX_IDLE_PER_HOST)
            .pool_idle_timeout(Duration::from_secs(
                constants::DEFAULT_POOL_IDLE_TIMEOUT_SECS,
            ))
            .tcp_nodelay(true)
            .danger_accept_invalid_certs(true)
            .build()
        {
            return client;
        }

        // Fallback to minimal insecure client
        tracing::warn!(
            "Failed to create insecure HTTP client with full options, using minimal client"
        );
        Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap()
    })
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
