use crate::wireless::active::attacks::deauth::run_deauth;
use crate::wireless::active::{ActiveAttackConfig, ActiveWirelessAttackResult};
use anyhow::Result;

/// Handles execution of a WirelessActive task.
/// Called by the task dispatcher when it encounters TaskConfig::WirelessActive.
pub async fn handle_wireless_active_task(
    interface: String,
    attack_type: String,
    bssid: Option<String>,
    client: Option<String>,
    frame_count: u64,
    rate_limit: u64,
    dry_run: bool,
) -> Result<ActiveWirelessAttackResult> {
    if attack_type != "deauth" {
        anyhow::bail!("Unsupported attack type from TUI: {}", attack_type);
    }

    let bssid_bytes = match bssid {
        Some(b) => ActiveAttackConfig::parse_mac(&b)
            .ok_or_else(|| anyhow::anyhow!("Invalid BSSID: {}", b))?,
        None => return Err(anyhow::anyhow!("BSSID is required for deauth")),
    };

    let client_bytes = client.and_then(|c| ActiveAttackConfig::parse_mac(&c));

    let config = ActiveAttackConfig {
        interface: interface.clone(),
        bssid: Some(bssid_bytes),
        client: client_bytes,
        reason_code: 7,
        max_frames: frame_count.min(1000),
        frames_per_second: rate_limit.min(100),
        dry_run,
    };

    let broadcast = client_bytes.is_none();
    let result = run_deauth(&config, broadcast).await?;

    Ok(result)
}

/// Registration helper - call this during app/worker initialization
/// to register the WirelessActive handler with the task system.
pub fn register_wireless_active_handler() {
    // This function can be extended to register with a central
    // task registry or dispatcher if one exists.
    // For now it serves as a clear hook point.
    tracing::info!("WirelessActive handler registered");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_compiles() {
        // Basic smoke test that the module compiles and types are correct
    }
}
