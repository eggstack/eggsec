use crate::commands::handlers::CommandContext;
use anyhow::Result;

pub async fn handle_packet(_ctx: &CommandContext, _args: crate::cli::PacketArgs) -> Result<()> {
    #[cfg(feature = "packet-inspection")]
    {
        use crate::cli::PacketSubcommand;
        use crate::packet::cli as packet_cli;

        // Scope check for subcommands that target external hosts
        match &args.command {
            PacketSubcommand::Send(send_args) => {
                ctx.ensure_scope(&send_args.target)?;
            }
            PacketSubcommand::Traceroute(trace_args) => {
                ctx.ensure_scope(&trace_args.target)?;
            }
            _ => {}
        }

        match args.command {
            PacketSubcommand::Capture(cap_args) => {
                packet_cli::handle_packet_capture(cap_args, ctx.json).await?;
            }
            PacketSubcommand::Send(send_args) => {
                packet_cli::handle_packet_send(send_args, ctx.json).await?;
            }
            PacketSubcommand::Dump(dump_args) => {
                packet_cli::handle_packet_dump(dump_args, ctx.json)?;
            }
            PacketSubcommand::Traceroute(trace_args) => {
                packet_cli::handle_packet_traceroute(trace_args, ctx.json).await?;
            }
            PacketSubcommand::Interfaces => {
                packet_cli::handle_packet_interfaces()?;
            }
        }

        return Ok(());
    }

    #[cfg(not(feature = "packet-inspection"))]
    {
        anyhow::bail!("Packet inspection requires 'packet-inspection' feature. Build with: cargo build --features packet-inspection");
    }
}

#[cfg(feature = "stress-testing")]
pub async fn handle_icmp(ctx: &CommandContext, args: crate::cli::IcmpArgs) -> Result<()> {
    use crate::scanner::icmp_probe;
    use std::time::Duration;

    ctx.ensure_scope(&args.target)?;

    let timeout = Duration::from_secs(args.timeout);
    let interval = Duration::from_secs_f64(args.interval);

    let (results, stats) =
        icmp_probe::ping_host(&args.target, args.count, timeout, interval).await?;

    if ctx.json {
        use crate::scanner::PingResult;
        #[derive(serde::Serialize)]
        struct IcmpJsonOutput<'a> {
            target: &'a str,
            stats: IcmpStatsJson,
            results: &'a Vec<PingResult>,
        }
        #[derive(serde::Serialize)]
        struct IcmpStatsJson {
            sent: u32,
            received: u32,
            lost: u32,
            min_rtt_ms: Option<f64>,
            max_rtt_ms: Option<f64>,
            avg_rtt_ms: Option<f64>,
        }
        let output = IcmpJsonOutput {
            target: &args.target,
            stats: IcmpStatsJson {
                sent: stats.sent,
                received: stats.received,
                lost: stats.lost,
                min_rtt_ms: stats.min_rtt.map(|r| r.as_secs_f64() * 1000.0),
                max_rtt_ms: stats.max_rtt.map(|r| r.as_secs_f64() * 1000.0),
                avg_rtt_ms: stats.avg_rtt.map(|r| r.as_secs_f64() * 1000.0),
            },
            results: &results,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("PING {}: {} data bytes", args.target, 56);
        for r in &results {
            println!(
                "{} bytes from {}: icmp_seq={} ttl={} time={:.3} ms",
                r.payload_size,
                r.target,
                r.sequence,
                r.ttl,
                r.rtt.as_secs_f64() * 1000.0
            );
        }
        println!("\n--- {} ping statistics ---", args.target);
        println!(
            "{} packets transmitted, {} packets received, {}% packet loss",
            stats.sent,
            stats.received,
            if stats.sent > 0 {
                (stats.lost as f64 / stats.sent as f64 * 100.0) as u32
            } else {
                0
            }
        );
        if let Some(rtt) = stats.min_rtt {
            println!(
                "round-trip min/avg/max = {:.3}/{:.3}/{:.3} ms",
                rtt.as_secs_f64() * 1000.0,
                stats.avg_rtt.unwrap_or_default().as_secs_f64() * 1000.0,
                stats.max_rtt.unwrap_or_default().as_secs_f64() * 1000.0
            );
        }
    }

    Ok(())
}

#[cfg(feature = "stress-testing")]
pub async fn handle_traceroute(
    ctx: &CommandContext,
    args: crate::cli::TracerouteArgs,
) -> Result<()> {
    use crate::packet::traceroute::{Traceroute, TracerouteConfig};
    use std::time::Duration;

    ctx.ensure_scope(&args.target)?;

    let config = TracerouteConfig {
        target: args.target.clone(),
        max_hops: args.max_hops,
        timeout: Duration::from_secs(args.timeout),
        max_retries: 3,
        first_ttl: 1,
        port: 33434,
        use_icmp: args.icmp,
        packet_size: 32,
        parallel_probes: args.parallel,
        resolve_names: !args.no_resolve,
        max_concurrent_probes: 6,
    };

    let traceroute = Traceroute::new(config);
    let result = traceroute.run().await?;

    if ctx.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "traceroute to {} ({}), {} hops max\n",
            result.target, result.resolved_address, result.total_hops
        );
        for hop in &result.hops {
            let addr = hop.address.clone().unwrap_or_else(|| "*".to_string());
            let rtt_str = hop
                .rtt_ms
                .map(|ms| format!("{:.3} ms", ms))
                .unwrap_or_else(|| "*".to_string());
            println!(" {:2}  {:<20} {}", hop.hop, addr, rtt_str);
        }
        if !result.success {
            println!("\nTrace incomplete (destination not reached).");
        }
    }

    Ok(())
}
