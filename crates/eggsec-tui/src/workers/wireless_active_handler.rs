use crate::wireless::active::attacks::deauth::run_deauth;
use crate::wireless::active::{ActiveAttackConfig, ActiveWirelessAttackResult};
use anyhow::Result;

/// Handles a WirelessActive task from the TUI.
///
/// This is called when a `TaskConfig::WirelessActive` task is processed by the worker system.
/// It extracts the parameters, builds the config, calls `run_deauth()`, and returns the result.
pub async fn handle_wireless_active_task(
    interface: String,
    attack_type: String,
    bssid: Option<String>,
    client: Option<String>,
    frame_count: u64,
    rate_limit: u64,
    dry_run: bool,
) -> Result<ActiveWirelessAttackResult> {
    // Only deauth is supported in Phase 1
    if attack_type != "deauth" {
        anyhow::bail!("Unsupported attack type in TUI: {}", attack_type);
    }

    let bssid_bytes = match bssid {
        Some(b) => ActiveAttackConfig::parse_mac(&b)
            .ok_or_else(|| anyhow::anyhow!("Invalid BSSID format: {}", b))?,
        None => return Err(anyhow::anyhow!("BSSID is required for deauth attack")),
    };

    let client_bytes = match client {
        Some(c) => ActiveAttackConfig::parse_mac(&c),
        None => None,
    };

    let config = ActiveAttackConfig {
        interface: interface.clone(),
        bssid: Some(bssid_bytes),
        client: client_bytes,
        reason_code: 7, // Default: Class 3 frame from non-associated STA
        max_frames: frame_count.min(1000),
        frames_per_second: rate_limit.min(100),
        dry_run,
    };

    // For broadcast vs targeted: if no client is provided, treat as broadcast
    let broadcast = client_bytes.is_none();

    let result = run_deauth(&config, broadcast).await?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mac_in_handler() {
        let mac = ActiveAttackConfig::parse_mac("AA:BB:CC:DD:EE:FF");
        assert!(mac.is_some());
    }
}
