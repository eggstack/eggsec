#[cfg(feature = "stress-testing")]
use crate::commands::handlers::CommandContext;
#[cfg(feature = "stress-testing")]
use crate::constants::DEFAULT_CONFIG_FILE;
#[cfg(feature = "stress-testing")]
use anyhow::Result;

#[cfg(feature = "stress-testing")]
pub async fn handle_stress(ctx: &CommandContext, args: crate::cli::StressArgs) -> Result<()> {
    use crate::stress::{StressConfig, StressTest, StressType};

    ctx.ensure_scope(&args.target)?;

    let stress_type = match args.stress_type {
        crate::cli::StressTypeArg::Syn => StressType::Syn,
        crate::cli::StressTypeArg::Udp => StressType::Udp,
        crate::cli::StressTypeArg::Http => StressType::Http,
        crate::cli::StressTypeArg::Tcp => StressType::Tcp,
        crate::cli::StressTypeArg::Icmp => StressType::Icmp,
    };

    let (host, port) = crate::utils::parse_host_port(&args.target, 80);

    let config = StressConfig {
        target: host,
        port,
        stress_type,
        rate_pps: args.rate,
        duration_secs: args.duration,
        concurrency: args.concurrency,
        spoof_source: args.spoof,
        spoof_range: args.spoof_range,
        random_source_port: args.random_port,
        payload_size: args.payload_size.unwrap_or(64),
        use_proxies: args.use_proxies,
        proxy_pool: args.proxy_file,
    };

    let stress_test = StressTest::new(config)?;
    let stats = stress_test.run().await?;

    if ctx.json {
        println!("{}", serde_json::to_string_pretty(&stats)?);
    } else {
        println!("\nStress Test Complete:");
        println!("  Packets sent: {}", stats.packets_sent);
        println!("  Bytes sent: {}", stats.bytes_sent);
        println!("  Duration: {} ms", stats.duration_ms);
        if stats.errors > 0 {
            println!("  Errors: {}", stats.errors);
        }
    }

    Ok(())
}

#[cfg(feature = "stress-testing")]
pub async fn handle_proxy(ctx: &CommandContext, args: crate::cli::ProxyArgs) -> Result<()> {
    use crate::cli::ProxyCommand;
    use crate::config::ProxyConfigEntry;
    use crate::proxy::{HealthCheckConfig, HealthChecker, ProxyEntry};

    match &args.command {
        ProxyCommand::Add(add_args) => {
            let proxies = ProxyEntry::load_from_file(&add_args.file)?;
            let count = proxies.len();

            let mut config = ctx.config.clone();
            let new_entries: Vec<ProxyConfigEntry> = proxies
                .iter()
                .map(|p| ProxyConfigEntry {
                    proxy_type: p.proxy_type,
                    address: p.address.clone(),
                    port: p.port,
                    username: p.username.clone(),
                    password: p.password.clone().map(crate::types::SensitiveString::from),
                    weight: Some(p.weight),
                    priority: Some(p.priority as u32),
                    enabled: p.enabled,
                    local_addr: None,
                })
                .collect();

            config.proxies.extend(new_entries);

            let config_path = ctx.config_path().unwrap_or(DEFAULT_CONFIG_FILE);
            let toml_content = toml::to_string_pretty(&config)
                .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;
            tokio::fs::write(config_path, toml_content).await?;

            println!(
                "Loaded {} proxies from {} and saved to config",
                count, add_args.file
            );
            for proxy in proxies.iter().take(5) {
                println!("  - {}", proxy.to_log_key());
            }
            if count > 5 {
                println!("  ... and {} more", count - 5);
            }
        }
        ProxyCommand::List(list_args) => {
            let config = &ctx.config;

            if config.proxies.is_empty() {
                println!("No proxies loaded.");
                println!("Run 'slapper proxy add --file proxies.txt' first.");
                println!("\nProxy file format (one per line):");
                println!("  http://127.0.0.1:8080");
                println!("  socks5://user:pass@proxy:1080");
                println!("  http://proxy:8080 # with optional comment");
            } else {
                println!("Proxy Pool ({} proxies):\n", config.proxies.len());
                for (i, proxy) in config.proxies.iter().enumerate() {
                    println!(
                        "  [{}] {}://{}:{} - {}",
                        i + 1,
                        proxy.proxy_type,
                        proxy.address,
                        proxy.port,
                        if proxy.enabled { "enabled" } else { "disabled" }
                    );
                    if list_args.verbose {
                        println!(
                            "      type: {}, priority: {:?}, weight: {:?}",
                            proxy.proxy_type, proxy.priority, proxy.weight
                        );
                    }
                }
            }
        }
        ProxyCommand::HealthCheck(health_args) => {
            let config = &ctx.config;

            if config.proxies.is_empty() {
                println!("No proxies loaded.");
                println!("Run 'slapper proxy add --file proxies.txt' first.");
                anyhow::bail!("No proxies to check");
            }

            let proxy_entries: Vec<ProxyEntry> = config
                .proxies
                .iter()
                .map(|p| {
                    let mut entry = ProxyEntry::new(p.proxy_type, p.address.clone(), p.port);
                    entry.username = p.username.clone();
                    entry.password = p.password.clone();
                    entry.weight = p.weight.unwrap_or(1);
                    entry.priority = p.priority.unwrap_or(0) as u8;
                    entry.enabled = p.enabled;
                    entry
                })
                .collect();

            let health_config = HealthCheckConfig {
                enabled: true,
                interval_secs: 0,
                timeout_ms: health_args.timeout * 1000,
                test_url: health_args.test_url.clone(),
                max_failures: 0,
            };

            let checker = HealthChecker::new(health_config)?;
            let results = checker.check_all(&proxy_entries).await?;

            println!("Proxy Health Check Results:");
            println!(
                "  Total: {} | Healthy: {} | Unhealthy: {}\n",
                results.total, results.healthy, results.unhealthy
            );

            for result in &results.results {
                let status = if result.is_healthy { "✓" } else { "✗" };
                let latency = result
                    .latency_ms
                    .map(|ms| format!("{}ms", ms))
                    .unwrap_or_else(|| "N/A".to_string());
                let error = result.error.as_deref().unwrap_or("OK");
                println!(
                    "  [{}] {} - {} ({})",
                    status, result.proxy_url, latency, error
                );
            }
        }
        ProxyCommand::Test(test_args) => {
            let proxy_entry = crate::commands::proxy::create_proxy_entry(&test_args.proxy)?;

            let test_url = test_args.test_url.clone();

            let health_config = HealthCheckConfig {
                enabled: true,
                interval_secs: 0,
                timeout_ms: 10000,
                test_url: test_url.clone(),
                max_failures: 0,
            };

            let checker = HealthChecker::new(health_config)?;

            let result = checker.check(&proxy_entry).await;

            println!("Testing proxy: {}", test_args.proxy);
            println!("Target URL: {}", test_url);

            if result.is_healthy {
                println!(
                    "\n[✓] Proxy is healthy (latency: {}ms)",
                    result.latency_ms.unwrap_or(0)
                );
            } else {
                println!(
                    "\n[✗] Proxy failed: {}",
                    result.error.unwrap_or_else(|| "Unknown error".to_string())
                );
            }
        }
    }

    Ok(())
}
