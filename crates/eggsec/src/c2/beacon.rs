use super::{BeaconProtocol, BeaconResult, C2Campaign};

pub fn simulate_beacons(campaign: &C2Campaign) -> Vec<BeaconResult> {
    let mut results = Vec::new();

    // Generate beacons based on campaign phases
    for phase in &campaign.phases {
        for technique in &phase.mitre_techniques {
            let (protocol, interval_ms, jitter_percent, success, evidence) = match technique.as_str()
            {
                "T1071.001" => (
                    BeaconProtocol::Https,
                    60000,
                    25,
                    true,
                    Some(format!(
                        "dry-run: HTTP/S beacon established (phase: {})",
                        phase.name
                    )),
                ),
                "T1071.004" => (
                    BeaconProtocol::Dns,
                    300000,
                    50,
                    true,
                    Some(format!(
                        "dry-run: DNS beacon established (phase: {})",
                        phase.name
                    )),
                ),
                "T1573" | "T1573.002" => (
                    BeaconProtocol::Tcp,
                    120000,
                    15,
                    true,
                    Some(format!(
                        "dry-run: encrypted channel established (phase: {})",
                        phase.name
                    )),
                ),
                "T1001" => (
                    BeaconProtocol::Dns,
                    600000,
                    75,
                    true,
                    Some(format!(
                        "dry-run: DNS-over-HTTPS beacon (phase: {})",
                        phase.name
                    )),
                ),
                _ => (
                    BeaconProtocol::Http,
                    300000,
                    30,
                    true,
                    Some(format!(
                        "dry-run: generic beacon established (phase: {})",
                        phase.name
                    )),
                ),
            };

            results.push(BeaconResult {
                protocol,
                interval_ms,
                jitter_percent,
                success,
                evidence,
            });
        }
    }

    // Always include at least one beacon
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

    #[test]
    fn test_simulate_beacons_produces_results() {
        let campaign = test_campaign();
        let beacons = simulate_beacons(&campaign);
        assert!(!beacons.is_empty());
        assert!(beacons.iter().all(|b| b.success));
    }

    #[test]
    fn test_simulate_beacons_empty_campaign() {
        let campaign = C2Campaign {
            id: "empty".to_string(),
            name: "Empty".to_string(),
            description: "Empty".to_string(),
            mitre_profile: "None".to_string(),
            phases: Vec::new(),
        };
        let beacons = simulate_beacons(&campaign);
        assert_eq!(beacons.len(), 1);
    }

    #[test]
    fn test_beacon_protocols_match_techniques() {
        let campaign = test_campaign();
        let beacons = simulate_beacons(&campaign);
        let protocols: Vec<_> = beacons.iter().map(|b| b.protocol).collect();
        assert!(protocols.contains(&BeaconProtocol::Https));
        assert!(protocols.contains(&BeaconProtocol::Tcp));
    }
}
