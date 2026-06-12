use crate::commands::handlers::CommandContext;
use crate::constants::DEFAULT_CONFIG_FILE;
use anyhow::Result;
use rustc_hash::FxHashMap;

pub async fn handle_report(ctx: &CommandContext, args: crate::cli::ReportArgs) -> Result<()> {
    use crate::cli::{ReportCommand, ReportFormat};
    use crate::config::ScheduledScan;
    use crate::output::convert;

    match &args.command {
        ReportCommand::Convert(convert_args) => {
            let content = tokio::fs::read_to_string(&convert_args.input).await?;
            // Parse as canonical ScanReportData, or auto-bridge native JSON from
            // standalone defense-lab commands (wireless/mobile) when their feature is enabled.
            // This makes `eggsec <lab> --json -o out.json ; eggsec report convert out.json -f ...` work.
            // Try native ScanReportData first, then cascade through feature-gated defense-lab bridges.
            let report: convert::ScanReportData = if let Ok(r) = serde_json::from_str::<convert::ScanReportData>(&content) {
                r
            } else if let Some(r) = try_bridge_defense_lab(&content) {
                r
            } else {
                return Err(anyhow::anyhow!("Failed to parse input as ScanReportData or any supported defense-lab format"));
            };

            let output = match convert_args.format {
                ReportFormat::Json => serde_json::to_string_pretty(&report)?,
                ReportFormat::Csv => convert::convert_to_csv(&report),
                ReportFormat::Junit => {
                    convert::convert_to_junit(&report).map_err(|e| anyhow::anyhow!(e))?
                }
                ReportFormat::Sarif => {
                    convert::convert_to_sarif(&report).map_err(|e| anyhow::anyhow!(e))?
                }
                ReportFormat::Html => convert::convert_to_html(&report),
                ReportFormat::Markdown => {
                    convert::convert_to_markdown(&report).map_err(|e| anyhow::anyhow!(e))?
                }
            };

            if let Some(output_file) = &convert_args.output {
                tokio::fs::write(output_file, &output).await?;
                println!("Report written to: {}", output_file);
            } else {
                println!("{}", output);
            }
        }
        ReportCommand::Trend(trend_args) => {
            let before =
                convert::load_scan_report(&trend_args.before).map_err(|e| anyhow::anyhow!(e))?;
            let after =
                convert::load_scan_report(&trend_args.after).map_err(|e| anyhow::anyhow!(e))?;

            let before_counts: FxHashMap<String, usize> =
                before
                    .findings
                    .iter()
                    .fold(FxHashMap::default(), |mut acc, f| {
                        *acc.entry(f.severity.clone()).or_insert(0) += 1;
                        acc
                    });
            let after_counts: FxHashMap<String, usize> =
                after
                    .findings
                    .iter()
                    .fold(FxHashMap::default(), |mut acc, f| {
                        *acc.entry(f.severity.clone()).or_insert(0) += 1;
                        acc
                    });

            let mut output = String::new();
            output.push_str("# Security Scan Trend Analysis\n\n");
            output.push_str(&format!(
                "## Target: {} → {}\n\n",
                before.target, after.target
            ));
            output.push_str("| Severity | Before | After | Change |\n");
            output.push_str("|----------|--------|-------|--------|\n");

            for sev in &["critical", "high", "medium", "low", "info"] {
                let before_count = before_counts.get(*sev).unwrap_or(&0);
                let after_count = after_counts.get(*sev).unwrap_or(&0);
                let change = *after_count as i32 - *before_count as i32;
                let change_str = if change > 0 {
                    format!("+{}", change)
                } else {
                    change.to_string()
                };
                output.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    sev, before_count, after_count, change_str
                ));
            }

            if let Some(output_file) = &trend_args.output {
                tokio::fs::write(output_file, &output).await?;
                println!("Trend analysis written to: {}", output_file);
            } else {
                println!("{}", output);
            }
        }
        ReportCommand::Schedule(schedule_args) => match &schedule_args.command {
            crate::cli::ScheduleCommand::List => {
                let config = &ctx.config;
                if config.schedule.is_empty() {
                    println!("No scheduled scans configured.");
                    println!("\nTo add a schedule, use:");
                    println!("  eggsec report schedule add <cron_expr> <target>");
                    println!("\nExample:");
                    println!("  eggsec report schedule add '0 */6 * * *' https://example.com");
                } else {
                    println!("Scheduled Scans:\n");
                    for (i, sched) in config.schedule.iter().enumerate() {
                        println!("  [{}] {} -> {}", i + 1, sched.schedule, sched.target);
                        println!(
                            "       Type: {}, Output: {:?}",
                            sched.scan_type, sched.output
                        );
                        println!();
                    }
                    println!("To generate crontab entries, run:");
                    println!("  eggsec report schedule cron");
                }
            }
            crate::cli::ScheduleCommand::Add(add_args) => {
                let config_path = ctx.config_path().unwrap_or(DEFAULT_CONFIG_FILE);
                let mut config = ctx.config.clone();

                let new_sched = ScheduledScan {
                    schedule: add_args.schedule.clone(),
                    target: add_args.target.clone(),
                    scan_type: add_args.scan_type.clone(),
                    output: add_args.output.clone(),
                    enabled: true,
                };

                config.schedule.push(new_sched);

                let toml_content = toml::to_string_pretty(&config)
                    .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;

                tokio::fs::write(config_path, toml_content).await?;

                println!("Schedule added successfully!");
                println!("  {} -> {}", add_args.schedule, add_args.target);
                println!("\nTo generate crontab entry, run:");
                println!("  eggsec report schedule cron");
            }
            crate::cli::ScheduleCommand::Remove(remove_args) => {
                let config_path = ctx.config_path().unwrap_or(DEFAULT_CONFIG_FILE);
                let mut config = ctx.config.clone();

                if let Ok(idx) = remove_args.id.parse::<usize>() {
                    if idx == 0 || idx > config.schedule.len() {
                        anyhow::bail!("Invalid schedule ID: {}. Use 'eggsec report schedule list' to see valid IDs", idx);
                    }
                    let removed = config.schedule.remove(idx - 1);
                    let toml_content = toml::to_string_pretty(&config)
                        .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;
                    tokio::fs::write(config_path, toml_content).await?;
                    println!(
                        "Removed schedule: {} -> {}",
                        removed.schedule, removed.target
                    );
                } else {
                    anyhow::bail!("Invalid schedule ID: {}", remove_args.id);
                }
            }
            crate::cli::ScheduleCommand::Cron(cron_args) => {
                let config = &ctx.config;

                if config.schedule.is_empty() {
                    println!("No scheduled scans configured.");
                    println!("Add schedules with: eggsec report schedule add <cron_expr> <target>");
                } else {
                    let schedules_to_cron: Vec<_> = if let Some(id) = &cron_args.id {
                        let idx = id
                            .parse::<usize>()
                            .ok()
                            .filter(|&i| i > 0 && i <= config.schedule.len())
                            .map(|i| i - 1);
                        match idx {
                            Some(i) => vec![config.schedule[i].clone()],
                            None => {
                                println!("Invalid schedule ID: {}. Use 'eggsec report schedule list' to see valid IDs", id);
                                println!("Generating crontab for all schedules:\n");
                                config.schedule.clone()
                            }
                        }
                    } else {
                        config.schedule.clone()
                    };

                    for sched in schedules_to_cron {
                        let output_part = match &sched.output {
                            Some(path) => format!(" -o {}", path),
                            None => String::new(),
                        };
                        println!(
                            "{} eggsec scan {} --profile {}{}",
                            sched.schedule, sched.target, sched.scan_type, output_part
                        );
                    }
                }
            }
        },
    }

    Ok(())
}

/// Try to parse native defense-lab JSON into ScanReportData via feature-gated bridges.
/// Returns None if no bridge matches (caller should produce its own error).
fn try_bridge_defense_lab(content: &str) -> Option<crate::output::convert::ScanReportData> {
    #[cfg(feature = "web-proxy")]
    {
        if let Ok(p) = serde_json::from_str::<crate::proxy::intercept::types::WebProxySessionReport>(content) {
            return Some(crate::proxy::intercept::to_scan_report_data_proxy(&p));
        }
    }
    #[cfg(feature = "wireless")]
    {
        if let Ok(w) = serde_json::from_str::<crate::wireless::WirelessScanResult>(content) {
            return Some(crate::wireless::to_scan_report_data(&w));
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(ls) = v.get("last_scan") {
                if let Ok(w) = serde_json::from_value::<crate::wireless::WirelessScanResult>(ls.clone()) {
                    return Some(crate::wireless::to_scan_report_data(&w));
                }
            }
        }
        #[cfg(feature = "wireless-advanced")]
        {
            if let Ok(a) = serde_json::from_str::<crate::wireless::active::ActiveWirelessAttackResult>(content) {
                return Some(crate::wireless::active::to_active_scan_report_data(&a));
            }
        }
    }
    #[cfg(feature = "mobile")]
    {
        if let Ok(m) = serde_json::from_str::<crate::mobile::MobileScanReport>(content) {
            return Some(crate::mobile::to_scan_report_data(&m));
        }
    }
    #[cfg(feature = "mobile-dynamic")]
    {
        if let Ok(d) = serde_json::from_str::<crate::mobile::DynamicMobileReport>(content) {
            return Some(crate::mobile::to_scan_report_data_dynamic(&d));
        }
    }
    #[cfg(feature = "db-pentest")]
    {
        if let Ok(d) = serde_json::from_str::<crate::db_pentest::DbPentestReport>(content) {
            return Some(crate::db_pentest::to_scan_report_data_db(&d));
        }
    }
    None
}
