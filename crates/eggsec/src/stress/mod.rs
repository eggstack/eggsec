pub mod authorization;
#[cfg(feature = "stress-testing")]
mod http;
#[cfg(feature = "stress-testing")]
mod icmp;
mod metrics;
#[cfg(feature = "stress-testing")]
mod syn;
#[cfg(feature = "stress-testing")]
mod udp;
#[cfg(feature = "stress-testing")]
mod utils;
mod warning;

pub use authorization::StressAuthorization;
pub use metrics::{StressMetrics, StressStats};
pub use warning::{display_warning, require_confirmation};

use crate::error::{EggsecError, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StressType {
    #[serde(rename = "syn")]
    Syn,
    #[serde(rename = "udp")]
    Udp,
    #[serde(rename = "http")]
    Http,
    #[serde(rename = "tcp")]
    Tcp,
    #[serde(rename = "icmp")]
    Icmp,
}

impl std::fmt::Display for StressType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StressType::Syn => write!(f, "SYN flood"),
            StressType::Udp => write!(f, "UDP flood"),
            StressType::Http => write!(f, "HTTP flood"),
            StressType::Tcp => write!(f, "TCP flood"),
            StressType::Icmp => write!(f, "ICMP flood"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StressConfig {
    pub target: String,
    pub port: u16,
    pub stress_type: StressType,
    pub rate_pps: u64,
    pub duration_secs: u64,
    pub concurrency: usize,
    pub spoof_source: bool,
    pub spoof_range: Option<String>,
    pub random_source_port: bool,
    pub payload_size: usize,
    pub use_proxies: bool,
    pub proxy_pool: Option<String>,
}

impl Default for StressConfig {
    fn default() -> Self {
        Self {
            target: String::new(),
            port: 80,
            stress_type: StressType::Http,
            rate_pps: 1000,
            duration_secs: 60,
            concurrency: 10,
            spoof_source: false,
            spoof_range: None,
            random_source_port: true,
            payload_size: 64,
            use_proxies: false,
            proxy_pool: None,
        }
    }
}

pub struct StressTest {
    config: StressConfig,
    authorization: StressAuthorization,
    #[cfg(feature = "stress-testing")]
    metrics: StressMetrics,
}

impl StressTest {
    pub fn new(config: StressConfig) -> Result<Self> {
        let authorization = StressAuthorization::from_scope()?;

        authorization.verify_target(&config.target)?;
        authorization.verify_rate(config.rate_pps)?;
        authorization.verify_duration(config.duration_secs)?;

        Ok(Self {
            config,
            authorization,
            #[cfg(feature = "stress-testing")]
            metrics: StressMetrics::new(),
        })
    }

    pub async fn run(&self) -> Result<StressStats> {
        display_warning(&self.config)?;

        if self.authorization.requires_confirmation() && !require_confirmation()? {
            return Err(EggsecError::Cancelled);
        }
        self.run_inner().await
    }

    pub async fn run_non_interactive(&self) -> Result<StressStats> {
        display_warning(&self.config)?;
        self.run_inner().await
    }

    async fn run_inner(&self) -> Result<StressStats> {
        tracing::info!(
            target = %self.config.target,
            port = self.config.port,
            type = ?self.config.stress_type,
            rate = self.config.rate_pps,
            "Starting stress test"
        );

        #[cfg(feature = "stress-testing")]
        {
            let stats = match self.config.stress_type {
                StressType::Syn => syn::run_syn_flood(&self.config, &self.metrics).await?,
                StressType::Udp => {
                    udp::run_udp_flood(&self.config, Arc::new(self.metrics.clone())).await?
                }
                StressType::Icmp => icmp::run_icmp_flood(&self.config, &self.metrics).await?,
                StressType::Http => http::run_http_flood(&self.config, &self.metrics).await?,
                StressType::Tcp => {
                    return Err(EggsecError::Runtime(
                        "TCP flood is not yet implemented. Use HTTP flood for application-layer testing."
                            .to_string(),
                    ));
                }
            };

            tracing::info!(
                packets_sent = stats.packets_sent,
                bytes_sent = stats.bytes_sent,
                duration_ms = stats.duration_ms,
                "Stress test completed"
            );

            Ok(stats)
        }

        #[cfg(not(feature = "stress-testing"))]
        {
            Err(EggsecError::Runtime(
                "Stress testing requires the 'stress-testing' feature: \
                 cargo build --features stress-testing"
                    .to_string(),
            ))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressResult {
    pub target: String,
    pub stress_type: StressType,
    pub stats: StressStats,
    pub config_used: StressConfigSummary,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressConfigSummary {
    pub rate_pps: u64,
    pub duration_secs: u64,
    pub spoof_source: bool,
    pub used_proxies: bool,
}

#[cfg(all(test, feature = "stress-testing"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_non_interactive_does_not_require_confirmation() {
        let test = StressTest {
            config: StressConfig {
                target: "127.0.0.1".to_string(),
                port: 80,
                stress_type: StressType::Http,
                rate_pps: 1,
                duration_secs: 0,
                concurrency: 1,
                spoof_source: false,
                spoof_range: None,
                random_source_port: true,
                payload_size: 0,
                use_proxies: false,
                proxy_pool: None,
            },
            authorization: StressAuthorization::for_tests(true),
            metrics: StressMetrics::new(),
        };

        let result = test.run_non_interactive().await;
        assert!(
            result.is_ok(),
            "expected non-interactive run to bypass stdin confirmation"
        );
    }
}
