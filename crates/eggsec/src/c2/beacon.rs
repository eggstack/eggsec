use super::{BeaconProtocol, BeaconResult, C2Campaign};

/// Beacon URI paths mapped to MITRE technique IDs.
fn beacon_uri_for_technique(technique: &str) -> &'static str {
    match technique {
        "T1071.001" => "/beacon",
        "T1071.004" => "/dns-query",
        "T1573" | "T1573.002" => "/checkin",
        "T1001" => "/status",
        _ => "/heartbeat",
    }
}

/// Determine the beacon protocol for a technique (used in both dry-run and real modes).
fn protocol_for_technique(technique: &str) -> BeaconProtocol {
    match technique {
        "T1071.001" => BeaconProtocol::Https,
        "T1071.004" => BeaconProtocol::Dns,
        "T1573" | "T1573.002" => BeaconProtocol::Tcp,
        "T1001" => BeaconProtocol::Dns,
        _ => BeaconProtocol::Http,
    }
}

/// Default beacon parameters for a protocol.
fn default_params(protocol: BeaconProtocol) -> (u64, u32) {
    match protocol {
        BeaconProtocol::Https => (60000, 25),
        BeaconProtocol::Dns => (300000, 50),
        BeaconProtocol::Tcp => (120000, 15),
        BeaconProtocol::Http => (300000, 30),
        BeaconProtocol::Custom => (600000, 75),
    }
}

/// Produce dry-run synthetic beacon results (no I/O).
fn dry_run_beacons(campaign: &C2Campaign) -> Vec<BeaconResult> {
    let mut results = Vec::new();

    for phase in &campaign.phases {
        for technique in &phase.mitre_techniques {
            let protocol = protocol_for_technique(technique);
            let (interval_ms, jitter_percent) = default_params(protocol);

            results.push(BeaconResult {
                protocol,
                interval_ms,
                jitter_percent,
                success: true,
                evidence: Some(format!(
                    "dry-run: {:?} beacon established (phase: {})",
                    protocol, phase.name
                )),
            });
        }
    }

    if results.is_empty() {
        results.push(BeaconResult {
            protocol: BeaconProtocol::Https,
            interval_ms: 60000,
            jitter_percent: 25,
            success: true,
            evidence: Some("dry-run: default HTTPS beacon established".to_string()),
        });
    }

    results
}

/// Produce real beacon results by making actual network requests to the target.
///
/// Uses reqwest with a 10s timeout. Connection failures produce `success: false`
/// with error evidence. No panics — all errors are caught.
async fn real_beacons(campaign: &C2Campaign, target: &str) -> Vec<BeaconResult> {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .danger_accept_invalid_certs(true)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return vec![BeaconResult {
                protocol: BeaconProtocol::Https,
                interval_ms: 0,
                jitter_percent: 0,
                success: false,
                evidence: Some(format!("failed to build HTTP client: {}", e)),
            }];
        }
    };

    let mut results = Vec::new();

    for phase in &campaign.phases {
        for technique in &phase.mitre_techniques {
            let protocol = protocol_for_technique(technique);
            let (interval_ms, jitter_percent) = default_params(protocol);

            let result = match protocol {
                BeaconProtocol::Https | BeaconProtocol::Http => {
                    let scheme = if protocol == BeaconProtocol::Https {
                        "https"
                    } else {
                        "http"
                    };
                    let uri = beacon_uri_for_technique(technique);
                    let url = format!("{}://{}{}", scheme, target, uri);

                    let start = std::time::Instant::now();
                    match client.get(&url).send().await {
                        Ok(resp) => {
                            let latency_ms = start.elapsed().as_millis() as u64;
                            let status = resp.status().as_u16();
                            let success =
                                resp.status().is_success() || resp.status().is_redirection();
                            BeaconResult {
                                protocol,
                                interval_ms,
                                jitter_percent,
                                success,
                                evidence: Some(format!(
                                    "HTTP {} from {} (latency: {}ms)",
                                    status, url, latency_ms
                                )),
                            }
                        }
                        Err(e) => BeaconResult {
                            protocol,
                            interval_ms,
                            jitter_percent,
                            success: false,
                            evidence: Some(format!("{} request failed: {} ({})", scheme, e, url)),
                        },
                    }
                }
                BeaconProtocol::Dns => {
                    let start = std::time::Instant::now();
                    match tokio::net::lookup_host(format!("{}:53", target)).await {
                        Ok(addrs) => {
                            let latency_ms = start.elapsed().as_millis() as u64;
                            let addr_list: Vec<String> =
                                addrs.map(|a| a.ip().to_string()).collect();
                            BeaconResult {
                                protocol,
                                interval_ms,
                                jitter_percent,
                                success: true,
                                evidence: Some(format!(
                                    "DNS resolution for {}: {} (latency: {}ms)",
                                    target,
                                    addr_list.join(", "),
                                    latency_ms
                                )),
                            }
                        }
                        Err(e) => BeaconResult {
                            protocol,
                            interval_ms,
                            jitter_percent,
                            success: false,
                            evidence: Some(format!("DNS resolution failed for {}: {}", target, e)),
                        },
                    }
                }
                BeaconProtocol::Tcp => {
                    let start = std::time::Instant::now();
                    match tokio::net::TcpStream::connect(format!("{}:443", target)).await {
                        Ok(_stream) => {
                            let latency_ms = start.elapsed().as_millis() as u64;
                            BeaconResult {
                                protocol,
                                interval_ms,
                                jitter_percent,
                                success: true,
                                evidence: Some(format!(
                                    "TCP connection to {}:{} succeeded (latency: {}ms)",
                                    target, 443, latency_ms
                                )),
                            }
                        }
                        Err(e) => BeaconResult {
                            protocol,
                            interval_ms,
                            jitter_percent,
                            success: false,
                            evidence: Some(format!(
                                "TCP connection to {}:{} failed: {}",
                                target, 443, e
                            )),
                        },
                    }
                }
                BeaconProtocol::Custom => {
                    // Custom protocols fall back to HTTP
                    let url = format!("http://{}/custom", target);
                    let start = std::time::Instant::now();
                    match client.get(&url).send().await {
                        Ok(resp) => {
                            let latency_ms = start.elapsed().as_millis() as u64;
                            let status = resp.status().as_u16();
                            BeaconResult {
                                protocol,
                                interval_ms,
                                jitter_percent,
                                success: resp.status().is_success(),
                                evidence: Some(format!(
                                    "Custom beacon HTTP {} from {} (latency: {}ms)",
                                    status, url, latency_ms
                                )),
                            }
                        }
                        Err(e) => BeaconResult {
                            protocol,
                            interval_ms,
                            jitter_percent,
                            success: false,
                            evidence: Some(format!("Custom beacon failed: {} ({})", e, url)),
                        },
                    }
                }
            };

            results.push(result);
        }
    }

    if results.is_empty() {
        // Fallback: single HTTPS beacon
        let url = format!("https://{}/heartbeat", target);
        let start = std::time::Instant::now();
        let result = match client.get(&url).send().await {
            Ok(resp) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                let status = resp.status().as_u16();
                BeaconResult {
                    protocol: BeaconProtocol::Https,
                    interval_ms: 60000,
                    jitter_percent: 25,
                    success: resp.status().is_success(),
                    evidence: Some(format!(
                        "HTTP {} from {} (latency: {}ms)",
                        status, url, latency_ms
                    )),
                }
            }
            Err(e) => BeaconResult {
                protocol: BeaconProtocol::Https,
                interval_ms: 60000,
                jitter_percent: 25,
                success: false,
                evidence: Some(format!("HTTPS request failed: {} ({})", e, url)),
            },
        };
        results.push(result);
    }

    results
}

/// Simulate C2 beacons. Produces dry-run synthetic results or real network requests
/// depending on the `dry_run` flag.
pub async fn simulate_beacons(
    campaign: &C2Campaign,
    target: &str,
    dry_run: bool,
) -> Vec<BeaconResult> {
    if dry_run {
        dry_run_beacons(campaign)
    } else {
        real_beacons(campaign, target).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c2::CampaignPhase;

    fn test_campaign() -> C2Campaign {
        C2Campaign {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            mitre_profile: "Test".to_string(),
            phases: vec![CampaignPhase {
                id: "p1".to_string(),
                name: "Phase 1".to_string(),
                description: "Test phase".to_string(),
                mitre_techniques: vec!["T1071.001".to_string(), "T1573".to_string()],
                order: 1,
            }],
        }
    }

    #[tokio::test]
    async fn test_dry_run_beacons_produces_results() {
        let campaign = test_campaign();
        let beacons = simulate_beacons(&campaign, "localhost", true).await;
        assert!(!beacons.is_empty());
        assert!(beacons.iter().all(|b| b.success));
        // Dry-run evidence contains "dry-run:"
        assert!(beacons
            .iter()
            .all(|b| b.evidence.as_ref().map_or(false, |e| e.contains("dry-run"))));
    }

    #[tokio::test]
    async fn test_dry_run_beacons_empty_campaign() {
        let campaign = C2Campaign {
            id: "empty".to_string(),
            name: "Empty".to_string(),
            description: "Empty".to_string(),
            mitre_profile: "None".to_string(),
            phases: Vec::new(),
        };
        let beacons = simulate_beacons(&campaign, "localhost", true).await;
        assert_eq!(beacons.len(), 1);
    }

    #[tokio::test]
    async fn test_dry_run_beacon_protocols_match_techniques() {
        let campaign = test_campaign();
        let beacons = simulate_beacons(&campaign, "localhost", true).await;
        let protocols: Vec<_> = beacons.iter().map(|b| b.protocol).collect();
        assert!(protocols.contains(&BeaconProtocol::Https));
        assert!(protocols.contains(&BeaconProtocol::Tcp));
    }

    #[tokio::test]
    async fn test_real_beacon_connect_refused() {
        // Target that definitely won't respond — port 1 on loopback is almost always filtered
        let campaign = test_campaign();
        let beacons = simulate_beacons(&campaign, "127.0.0.1:1", false).await;
        assert!(!beacons.is_empty());
        // All beacons should fail (connection refused or timeout)
        assert!(beacons.iter().all(|b| !b.success));
        // Evidence should describe the error, not contain "dry-run"
        assert!(beacons.iter().all(|b| b
            .evidence
            .as_ref()
            .map_or(false, |e| !e.contains("dry-run"))));
    }

    #[tokio::test]
    async fn test_real_beacon_dns_resolution() {
        // localhost always resolves
        let campaign = C2Campaign {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            mitre_profile: "Test".to_string(),
            phases: vec![CampaignPhase {
                id: "p1".to_string(),
                name: "DNS Phase".to_string(),
                description: "Test".to_string(),
                mitre_techniques: vec!["T1071.004".to_string()],
                order: 1,
            }],
        };
        let beacons = simulate_beacons(&campaign, "localhost", false).await;
        assert_eq!(beacons.len(), 1);
        assert_eq!(beacons[0].protocol, BeaconProtocol::Dns);
        // DNS resolution of localhost should succeed
        assert!(beacons[0].success);
    }

    #[test]
    fn test_protocol_for_technique() {
        assert_eq!(protocol_for_technique("T1071.001"), BeaconProtocol::Https);
        assert_eq!(protocol_for_technique("T1071.004"), BeaconProtocol::Dns);
        assert_eq!(protocol_for_technique("T1573"), BeaconProtocol::Tcp);
        assert_eq!(protocol_for_technique("T1001"), BeaconProtocol::Dns);
        assert_eq!(protocol_for_technique("unknown"), BeaconProtocol::Http);
    }

    #[test]
    fn test_default_params() {
        let (interval, jitter) = default_params(BeaconProtocol::Https);
        assert_eq!(interval, 60000);
        assert_eq!(jitter, 25);
    }
}
