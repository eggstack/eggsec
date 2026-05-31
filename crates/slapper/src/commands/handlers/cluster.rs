use crate::commands::handlers::CommandContext;
use anyhow::Result;

pub async fn handle_cluster(ctx: &CommandContext, args: crate::cli::ClusterArgs) -> Result<()> {
    use crate::cli::ClusterCommand;
    use crate::distributed::{RemoteClient, RemoteListener};

    match &args.command {
        ClusterCommand::Worker(worker_args) => {
            let psk = get_psk_from_args_or_config(
                worker_args.psk.clone(),
                ctx.config
                    .remote
                    .psk
                    .as_ref()
                    .map(|s| s.expose_secret().to_string()),
                "PSK is required for worker mode. Use --psk <key> or set [remote.psk] in config"
                    .to_string(),
            )?;

            println!("Starting worker node...");
            println!("  Coordinator: {}", worker_args.coordinator);
            println!("  Workers: {}", worker_args.workers);

            let worker_id = worker_args.worker_id.clone().unwrap_or_else(|| {
                use std::time::{SystemTime, UNIX_EPOCH};
                let duration = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default();
                format!("worker-{}", duration.as_millis())
            });

            let (host, port) = extract_host_and_port(&worker_args.coordinator);

            let mut client = RemoteClient::new(psk.clone());

            println!(
                "Worker '{}' connecting to coordinator at {}:{}",
                worker_id, host, port
            );

            let result = client
                .execute(
                    &host,
                    port,
                    vec!["slapper".to_string(), "--version".to_string()],
                    Some(30),
                )
                .await;

            match result {
                Ok(_r) => {
                    eprintln!("[EXPERIMENTAL] Cluster worker mode is not fully implemented.");
                    println!("Connected to coordinator successfully!");
                    println!("Worker '{}' ready. Press Ctrl+C to stop.", worker_id);
                    println!("\nWorker functionality: Use 'slapper remote exec' to execute commands on this worker.");
                }
                Err(e) => {
                    anyhow::bail!("Failed to connect to coordinator: {}. Make sure the coordinator is running with 'slapper cluster coordinator --psk <key>'", e);
                }
            }
        }
        ClusterCommand::Coordinator(coordinator_args) => {
            let bind_addr = coordinator_args
                .bind
                .clone()
                .unwrap_or_else(|| "0.0.0.0".to_string());

            let psk = coordinator_args
                .psk
                .clone()
                .or_else(|| {
                    ctx.config
                        .remote
                        .psk
                        .as_ref()
                        .map(|s| s.expose_secret().to_string())
                })
                .unwrap_or_else(|| {
                    let key = crate::distributed::generate_psk();
                    println!("No PSK provided. Generated key (add to config for persistence):");
                    println!("  {}", key);
                    key
                });

            println!("Starting distributed coordinator...");
            println!("  Bind: {}:{}", bind_addr, coordinator_args.port);

            let listener = RemoteListener::new(psk);

            println!(
                "Coordinator started on {}:{}",
                bind_addr, coordinator_args.port
            );
            println!(
                "Workers can connect using: slapper cluster worker --coordinator {}:{} --psk <key>",
                bind_addr, coordinator_args.port
            );

            tokio::select! {
                result = listener.start(coordinator_args.port) => {
                    if let Err(e) = result {
                        anyhow::bail!("Coordinator error: {}", e);
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    println!("\nShutting down coordinator...");
                    listener.shutdown();
                }
            }
        }
        ClusterCommand::Status(status_args) => {
            if let Some(addr) = &status_args.coordinator {
                println!("Fetching cluster status from {}...", addr);

                let psk = get_psk_from_args_or_config(
                    None,
                    ctx.config
                        .remote
                        .psk
                        .as_ref()
                        .map(|s| s.expose_secret().to_string()),
                    "No PSK configured. Set [remote.psk] in config or provide --psk".to_string(),
                )?;

                let (host, port) = extract_host_and_port(addr);

                let mut client = RemoteClient::new(psk);

                match client
                    .execute(
                        &host,
                        port,
                        vec![
                            "slapper".to_string(),
                            "cluster".to_string(),
                            "status".to_string(),
                        ],
                        Some(10),
                    )
                    .await
                {
                    Ok(r) => {
                        println!("{}", r.output);
                    }
                    Err(e) => {
                        println!("Failed to connect: {}", e);
                    }
                }
            } else {
                println!("Cluster Status:");
                println!("  No coordinator address provided.");
                println!("  Start a coordinator with: slapper cluster coordinator --psk <key>");
                println!(
                    "  Then check status with: slapper cluster status --coordinator <address>"
                );
            }
        }
    }

    Ok(())
}

pub async fn handle_remote(ctx: &CommandContext, args: crate::cli::RemoteArgs) -> Result<()> {
    use crate::cli::RemoteCommand;
    use crate::distributed::{generate_psk, RemoteListener, TlsConfig};

    match &args.command {
        RemoteCommand::GenerateKey => {
            let key = generate_psk();
            println!("{}", key);
            println!("\nAdd to your config file:");
            println!("[remote]");
            println!("psk = \"{}\"", key);
        }
        RemoteCommand::Cert(_cert_args) => {
            println!("TLS Certificate Generation");
            println!("=========================");
            println!();
            println!("To create a TLS certificate for distributed communication,");
            println!("use OpenSSL to generate PEM files:");
            println!();
            println!("  # Generate private key and certificate");
            println!("  openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes -subj '/CN=localhost'");
            println!();
            println!("Usage:");
            println!("  slapper remote start --tls-cert cert.pem --tls-key key.pem");
            println!();
            println!("Note: Both certificate and key paths must be provided.");
        }
        RemoteCommand::Start(start_args) => {
            let psk = start_args
                .auth
                .clone()
                .or_else(|| {
                    ctx.config
                        .remote
                        .psk
                        .as_ref()
                        .map(|s| s.expose_secret().to_string())
                })
                .unwrap_or_else(|| {
                    println!(
                        "No PSK provided. Using generated key (add to config for persistence):"
                    );
                    let key = generate_psk();
                    println!("  {}", key);
                    key
                });

            let listener = if let Some(tls_cert) = &start_args.tls_cert {
                let tls_key = start_args.tls_key.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("TLS key path required when using --tls-cert")
                })?;
                let tls_config = TlsConfig {
                    cert_path: tls_cert.clone().into(),
                    key_path: tls_key.clone().into(),
                };
                RemoteListener::with_tls(psk, tls_config)?
            } else {
                RemoteListener::new(psk)
            };

            if listener.is_tls() {
                println!("TLS enabled");
            }

            tokio::select! {
                result = listener.start(start_args.port) => {
                    if let Err(e) = result {
                        anyhow::bail!("Remote listener error: {}", e);
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    println!("\nShutting down...");
                    listener.shutdown();
                }
            }
        }
        RemoteCommand::Stop => {
            eprintln!("[STUB] Remote stop is not yet implemented.");
            println!("(Note: This requires a running listener to respond to stop signal)");
        }
    }

    Ok(())
}

pub async fn handle_exec(ctx: &CommandContext, args: crate::cli::ExecArgs) -> Result<()> {
    use crate::distributed::{RemoteClient, RemoteResult};

    let psk = get_psk_from_args_or_config(
        args.auth.clone(),
        ctx.config
            .remote
            .psk
            .as_ref()
            .map(|s| s.expose_secret().to_string()),
        "No PSK provided. Use --auth or set [remote.psk] in config".to_string(),
    )?;

    let mut client = if let Some(_tls_cert) = &args.tls_cert {
        let domain = args.tls_domain.as_deref().unwrap_or("localhost");
        RemoteClient::with_tls(psk, domain)
            .map_err(|e| anyhow::anyhow!("Failed to initialize TLS: {}", e))?
    } else {
        RemoteClient::new(psk)
    };

    if client.is_tls() {
        println!("TLS enabled");
    }

    let targets: Vec<String> = if let Some(targets_file) = &args.targets {
        tokio::fs::read_to_string(targets_file)
            .await?
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && !s.starts_with('#'))
            .collect()
    } else if let Some(target) = &args.target {
        vec![target.clone()]
    } else {
        anyhow::bail!("Either --target or --targets must be specified");
    };

    let mut results: Vec<RemoteResult> = Vec::new();

    for target in &targets {
        ctx.ensure_scope(target)?;
        let (host, port) = crate::utils::parse_host_port(target, ctx.config.remote.default_port);

        println!("Executing on {}:{}...", host, port);

        match client
            .execute(&host, port, args.command.clone(), Some(args.timeout))
            .await
        {
            Ok(result) => {
                results.push(result);
            }
            Err(e) => {
                results.push(RemoteResult::new(
                    target.clone(),
                    false,
                    String::new(),
                    Some(e.to_string()),
                    0,
                ));
            }
        }
    }

    println!("\n--- Execution Results ---\n");
    for result in &results {
        if result.success {
            println!("[{}] Success:", result.hostname);
            println!("{}", result.output);
        } else {
            println!(
                "[{}] Failed: {}",
                result.hostname,
                result.error.as_ref().unwrap_or(&String::new())
            );
        }
        println!();
    }

    let success_count = results.iter().filter(|r| r.success).count();
    println!("Completed: {}/{} successful", success_count, results.len());

    Ok(())
}

fn get_psk_from_args_or_config(
    args_psk: Option<String>,
    config_psk: Option<String>,
    error_msg: String,
) -> Result<String> {
    args_psk
        .or(config_psk)
        .ok_or_else(|| anyhow::anyhow!(error_msg))
}

fn extract_host_and_port(addr: &str) -> (String, u16) {
    if let Some(addr) = addr.strip_prefix('[') {
        if let Some(bracket_end) = addr.find("]:") {
            let host = addr[..bracket_end].to_string();
            let port: u16 = addr[bracket_end + 2..].parse().unwrap_or_else(|_| 22);
            return (host, port);
        }
    }
    if let Some((host, port_str)) = addr.rsplit_once(':') {
        if let Ok(port) = port_str.parse::<u16>() {
            return (host.to_string(), port);
        }
    }
    (addr.to_string(), 22)
}
