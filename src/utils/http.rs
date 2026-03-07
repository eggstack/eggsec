#![allow(dead_code)]

use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;

pub fn create_http_client(timeout_secs: u64) -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .context("Failed to create HTTP client")
}

pub fn create_insecure_http_client(timeout_secs: u64) -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .danger_accept_invalid_certs(true)
        .build()
        .context("Failed to create insecure HTTP client")
}

pub fn create_http_client_with_proxy(timeout_secs: u64, proxy: &str) -> Result<Client> {
    let proxy = reqwest::Proxy::http(proxy).context("Invalid proxy URL")?;

    Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .proxy(proxy)
        .build()
        .context("Failed to create HTTP client with proxy")
}

pub fn create_http_client_with_options<F>(timeout_secs: u64, builder_fn: F) -> Result<Client>
where
    F: FnOnce(reqwest::ClientBuilder) -> reqwest::ClientBuilder,
{
    let builder = builder_fn(Client::builder().timeout(Duration::from_secs(timeout_secs)));
    builder.build().context("Failed to create HTTP client")
}

pub fn create_insecure_client_with_options<F>(timeout_secs: u64, builder_fn: F) -> Result<Client>
where
    F: FnOnce(reqwest::ClientBuilder) -> reqwest::ClientBuilder,
{
    let builder = builder_fn(
        Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .danger_accept_invalid_certs(true),
    );
    builder.build().context("Failed to create HTTP client")
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
