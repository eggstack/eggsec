use crate::error::Result;
use crate::utils::sanitize_for_logging;
use std::io::{self, Write};

use super::StressConfig;

pub fn display_warning(config: &StressConfig) -> Result<()> {
    let banner = r#"
⚠️  STRESS TESTING MODE - USE WITH EXTREME CAUTION  ⚠️

This mode sends high volumes of traffic to test system resilience.
Unauthorized use against systems you do not own or have explicit
permission to test may violate laws including:

• Computer Fraud and Abuse Act (CFAA) - United States
• Computer Misuse Act - United Kingdom
• Similar laws in virtually all jurisdictions

By proceeding, you confirm:
1. You have explicit written authorization to test this target
2. You understand the potential for service disruption
3. You will monitor the target and stop if issues arise
4. You accept full legal responsibility for your actions
"#;

    eprintln!("{}", banner);

    eprintln!("Test Configuration:");
    eprintln!(
        "  Target:        {}:{}",
        sanitize_for_logging(&config.target),
        config.port
    );
    eprintln!("  Type:          {}", config.stress_type);
    eprintln!("  Rate:          {} packets/second", config.rate_pps);
    eprintln!("  Duration:      {} seconds", config.duration_secs);
    eprintln!("  Concurrency:   {}", config.concurrency);
    eprintln!(
        "  Spoof Source:  {}",
        if config.spoof_source {
            "ENABLED ⚠️"
        } else {
            "Disabled"
        }
    );
    eprintln!(
        "  Proxies:       {}",
        if config.use_proxies {
            "Enabled"
        } else {
            "Disabled"
        }
    );
    eprintln!();

    if config.spoof_source {
        eprintln!("⚠️  IP SPOOFING ENABLED - This requires root privileges and raw socket access.");
        eprintln!("    Spoofed packets may be filtered by upstream routers.");
        eprintln!();
    }

    Ok(())
}

pub fn require_confirmation() -> Result<bool> {
    eprint!("Type 'yes' to proceed with the stress test: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let confirmed = input.trim().to_lowercase() == "yes";

    if !confirmed {
        eprintln!("Test cancelled.");
    }

    Ok(confirmed)
}

pub fn display_completion(stats: &super::StressStats) {
    eprintln!();
    eprintln!("Stress Test Completed");
    eprintln!("Duration:        {:>10} ms", stats.duration_ms);
    eprintln!("Packets Sent:    {:>15}", stats.packets_sent);
    eprintln!("Bytes Sent:      {:>15}", stats.bytes_sent);
    eprintln!("Avg Rate:        {:>10} pps", stats.avg_rate_pps());
    eprintln!("Errors:          {:>15}", stats.errors);
}
