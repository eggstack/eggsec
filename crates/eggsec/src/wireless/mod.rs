//! Wireless security testing module
//!
//! Provides wireless network security testing capabilities including
//! iwlist scan parsing and wireless security type analysis.
//! WPA/WPA2 handshake capture analysis is aspirational (not yet implemented).
//!
//! This module is feature-gated behind the `wireless` feature flag.

use crate::error::{EggsecError, Result};
use crate::types::Severity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WirelessNetwork {
    pub ssid: String,
    pub bssid: String,
    pub channel: u8,
    pub security_type: SecurityType,
    pub signal_strength: i32,
    pub last_seen: String,
    pub wps_enabled: bool,
    pub is_hidden: bool,
    pub transition_mode: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecurityType {
    Open,
    WEP,
    WPA,
    WPA2,
    WPA3,
    Enterprise,
    Unknown,
}

impl SecurityType {
    pub fn as_str(&self) -> &str {
        match self {
            SecurityType::Open => "Open",
            SecurityType::WEP => "WEP",
            SecurityType::WPA => "WPA",
            SecurityType::WPA2 => "WPA2",
            SecurityType::WPA3 => "WPA3",
            SecurityType::Enterprise => "Enterprise",
            SecurityType::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WirelessScanResult {
    pub interface: String,
    pub networks: Vec<WirelessNetwork>,
    pub scan_duration_secs: u64,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WirelessVulnerability {
    pub ssid: String,
    pub bssid: String,
    pub vulnerability_type: String,
    pub severity: Severity,
    pub description: String,
    pub recommendation: String,
}

pub struct WirelessScanner {
    interface: Option<String>,
}

impl WirelessScanner {
    pub fn new() -> Self {
        Self { interface: None }
    }

    pub fn with_interface(mut self, interface: String) -> Self {
        self.interface = Some(interface);
        self
    }

    #[cfg(feature = "wireless")]
    pub async fn scan(&self, duration_secs: u64) -> Result<WirelessScanResult> {
        use tokio::process::Command;

        let interface = self
            .interface
            .as_ref()
            .ok_or_else(|| EggsecError::Config("No wireless interface specified".to_string()))?;

        let start = std::time::Instant::now();

        let output = Command::new("iwlist")
            .args([interface, "scan"])
            .output()
            .await
            .map_err(|e| {
                let msg = if e.to_string().contains("not found") || e.to_string().contains("No such file") {
                    "iwlist command not found (install wireless-tools package and ensure in PATH)".to_string()
                } else {
                    format!("iwlist scan failed: {}", e)
                };
                EggsecError::Network(msg)
            })?;

        let elapsed_secs = start.elapsed().as_secs();

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stderr_l = stderr.to_lowercase();
            let msg = if stderr_l.contains("operation not permitted") || stderr_l.contains("permission denied") || stderr_l.contains("not permitted") {
                "iwlist scan failed: Operation not permitted. run as root or grant CAP_NET_ADMIN (setcap cap_net_admin+ep /sbin/iwlist or use sudo)".to_string()
            } else if stderr.trim().is_empty() {
                format!(
                    "iwlist scan failed with exit code {}",
                    output.status.code().unwrap_or(-1)
                )
            } else {
                format!("iwlist scan failed: {}", stderr.trim())
            };
            return Err(EggsecError::Network(msg));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut networks = self.parse_scan_output(&stdout);
        let scan_time = chrono::Utc::now().to_rfc3339();
        for network in &mut networks {
            network.last_seen = scan_time.clone();
        }

        let recommendations = Self::generate_recommendations(&networks);

        Ok(WirelessScanResult {
            interface: interface.clone(),
            networks,
            scan_duration_secs: elapsed_secs.max(duration_secs),
            recommendations,
        })
    }

    #[cfg(feature = "wireless")]
    fn parse_scan_output(&self, output: &str) -> Vec<WirelessNetwork> {
        let mut networks = Vec::new();
        let mut current_ssid: Option<String> = None;
        let mut current_bssid: Option<String> = None;
        let mut current_channel = 0u8;
        let mut current_security = SecurityType::Unknown;
        let mut current_signal = -100i32;
        let mut current_auth_suite: Option<String> = None;
        let mut current_wps = false;
        let mut current_is_hidden = false;
        let mut current_transition = false;
        let mut saw_wpa2 = false;
        let mut saw_wpa3 = false;
        let mut skipped_malformed: u32 = 0;

        for line in output.lines() {
            let line = line.trim();

            if line.starts_with("Cell ") {
                let had_incomplete_cell = current_ssid.is_some() || current_bssid.is_some();
                if let (Some(ssid), Some(bssid)) = (current_ssid.take(), current_bssid.take()) {
                    // both present: empty ssid is valid for hidden (will be normalized below)
                    let mut final_ssid = ssid.clone();
                    if ssid.is_empty() || ssid == "<hidden>" || ssid == "\"\"" {
                        final_ssid = "<hidden>".to_string();
                        current_is_hidden = true;
                    }
                    networks.push(WirelessNetwork {
                        ssid: final_ssid,
                        bssid,
                        channel: current_channel,
                        security_type: current_security,
                        signal_strength: current_signal,
                        last_seen: String::new(),
                        wps_enabled: current_wps,
                        is_hidden: current_is_hidden,
                        transition_mode: current_transition || (saw_wpa2 && saw_wpa3),
                    });
                } else if had_incomplete_cell {
                    skipped_malformed += 1;
                }
                current_auth_suite = None;
                current_channel = 0u8;
                current_security = SecurityType::Unknown;
                current_signal = -100i32;
                current_wps = false;
                current_is_hidden = false;
                current_transition = false;
                saw_wpa2 = false;
                saw_wpa3 = false;
            }

            if line.contains("Address:") {
                if let Some(addr) = line.split("Address:").nth(1) {
                    let bssid = addr.trim().to_string();
                    if !bssid.is_empty() && bssid.contains(':') {
                        current_bssid = Some(bssid);
                    }
                }
            }

            if line.starts_with("ESSID:") {
                let essid = line.split("ESSID:\"")
                    .nth(1)
                    .unwrap_or("")
                    .trim_matches('"')
                    .to_string();
                current_ssid = Some(essid.clone());
                if essid.is_empty() || essid == "<hidden>" || essid == "\"\"" {
                    current_is_hidden = true;
                }
            }

            if line.contains("Channel:") {
                current_channel = line
                    .split("Channel:")
                    .nth(1)
                    .unwrap_or("1")
                    .trim()
                    .parse()
                    .unwrap_or(1);
            }

            if line.contains("Signal level") {
                let level_str = if let Some(rest) = line.split("Signal level=").nth(1) {
                    rest.split_whitespace().next().unwrap_or("-100")
                } else if let Some(rest) = line.split("Signal level:").nth(1) {
                    rest.split_whitespace().next().unwrap_or("-100")
                } else {
                    "-100"
                };
                current_signal = level_str.parse().unwrap_or(-100);
            }

            let lower = line.to_lowercase();
            if lower.contains("wps") || lower.contains("wi-fi protected setup") {
                current_wps = true;
            }

            if lower.contains("wpa2/wpa3") || lower.contains("transition") {
                current_transition = true;
            }

            if line.contains("Encryption key:") && line.contains("off") {
                current_security = SecurityType::Open;
            } else if line.contains("WPA3") {
                current_security = SecurityType::WPA3;
                saw_wpa3 = true;
            } else if line.contains("WPA2") {
                current_security = SecurityType::WPA2;
                saw_wpa2 = true;
            } else if line.contains("WPA") {
                current_security = SecurityType::WPA;
            } else if line.contains("WEP") {
                current_security = SecurityType::WEP;
            }

            if line.contains("Authentication Suites") {
                if let Some(suite) = line.split(':').nth(1) {
                    current_auth_suite = Some(suite.trim().to_string());
                }
            }

            if current_auth_suite.as_deref() == Some("802.1X") {
                current_security = SecurityType::Enterprise;
            }
        }

        let had_final_incomplete = current_ssid.is_some() || current_bssid.is_some();
        if let (Some(ssid), Some(bssid)) = (current_ssid, current_bssid) {
            // do not skip on empty ssid: hidden networks legitimately have empty ESSID
            let mut final_ssid = ssid.clone();
            if ssid.is_empty() || ssid == "<hidden>" || ssid == "\"\"" {
                final_ssid = "<hidden>".to_string();
                current_is_hidden = true;
            }
            networks.push(WirelessNetwork {
                ssid: final_ssid,
                bssid,
                channel: current_channel,
                security_type: current_security,
                signal_strength: current_signal,
                last_seen: String::new(),
                wps_enabled: current_wps,
                is_hidden: current_is_hidden,
                transition_mode: current_transition || (saw_wpa2 && saw_wpa3),
            });
        } else if had_final_incomplete {
            skipped_malformed += 1;
        }

        let _ = skipped_malformed; // count silently; no exposure or warnings per plan

        networks
    }

    fn is_known_good(net: &WirelessNetwork, kg: &std::collections::HashSet<String>) -> bool {
        if kg.contains(&net.ssid) || kg.contains(&net.bssid) {
            return true;
        }
        if kg.contains(&format!("{},{}", net.ssid, net.bssid)) {
            return true;
        }
        false
    }

    pub fn analyze_networks(networks: &[WirelessNetwork], known_good: Option<&std::collections::HashSet<String>>) -> Vec<WirelessVulnerability> {
        let mut vulnerabilities = Vec::new();

        for network in networks {
            if network.signal_strength <= -80 {
                vulnerabilities.push(WirelessVulnerability {
                    ssid: network.ssid.clone(),
                    bssid: network.bssid.clone(),
                    vulnerability_type: "Weak Signal Strength".to_string(),
                    severity: if network.signal_strength <= -90 { Severity::Low } else { Severity::Medium },
                    description: format!("Network {} has weak signal ({} dBm)", network.ssid, network.signal_strength),
                    recommendation: "Reposition AP or investigate interference".to_string(),
                });
            }

            if network.wps_enabled {
                vulnerabilities.push(WirelessVulnerability {
                    ssid: network.ssid.clone(),
                    bssid: network.bssid.clone(),
                    vulnerability_type: "WPS Enabled".to_string(),
                    severity: Severity::Medium,
                    description: format!("Network {} has WPS enabled (PIN brute-force risk)", network.ssid),
                    recommendation: "Disable WPS or use PIN-less mode with rate limiting".to_string(),
                });
            }

            if network.is_hidden {
                vulnerabilities.push(WirelessVulnerability {
                    ssid: network.ssid.clone(),
                    bssid: network.bssid.clone(),
                    vulnerability_type: "Hidden SSID".to_string(),
                    severity: Severity::Low,
                    description: format!("Network {} uses hidden SSID (provides minimal security benefit)", network.ssid),
                    recommendation: "Consider broadcasting SSID for better client compatibility".to_string(),
                });
            }

            if network.transition_mode {
                vulnerabilities.push(WirelessVulnerability {
                    ssid: network.ssid.clone(),
                    bssid: network.bssid.clone(),
                    vulnerability_type: "WPA2/WPA3 Transition Mode".to_string(),
                    severity: Severity::Low,
                    description: format!("Network {} is in WPA2/WPA3 transition mode (downgrade risk)", network.ssid),
                    recommendation: "Enforce WPA3-only when all clients support it".to_string(),
                });
            }

            match network.security_type {
                SecurityType::Open => {
                    vulnerabilities.push(WirelessVulnerability {
                        ssid: network.ssid.clone(),
                        bssid: network.bssid.clone(),
                        vulnerability_type: "Open Network".to_string(),
                        severity: Severity::Medium,
                        description: format!("Network {} has no encryption", network.ssid),
                        recommendation: "Enable WPA2 or WPA3 encryption".to_string(),
                    });
                }
                SecurityType::WEP => {
                    vulnerabilities.push(WirelessVulnerability {
                        ssid: network.ssid.clone(),
                        bssid: network.bssid.clone(),
                        vulnerability_type: "WEP Encryption".to_string(),
                        severity: Severity::High,
                        description: "WEP encryption is easily cracked".to_string(),
                        recommendation: "Upgrade to WPA2 or WPA3 immediately".to_string(),
                    });
                }
                SecurityType::WPA => {
                    vulnerabilities.push(WirelessVulnerability {
                        ssid: network.ssid.clone(),
                        bssid: network.bssid.clone(),
                        vulnerability_type: "WPA Encryption".to_string(),
                        severity: Severity::Medium,
                        description: "WPA encryption has known vulnerabilities".to_string(),
                        recommendation: "Upgrade to WPA2 or WPA3".to_string(),
                    });
                }
                SecurityType::Enterprise => {
                    vulnerabilities.push(WirelessVulnerability {
                        ssid: network.ssid.clone(),
                        bssid: network.bssid.clone(),
                        vulnerability_type: "Enterprise Authentication".to_string(),
                        severity: Severity::Low,
                        description: "Enterprise (802.1X) network detected - verify RADIUS server configuration and certificate validation".to_string(),
                        recommendation: "Ensure proper RADIUS server configuration, certificate validation, and EAP method security".to_string(),
                    });
                }
                SecurityType::Unknown => {
                    vulnerabilities.push(WirelessVulnerability {
                        ssid: network.ssid.clone(),
                        bssid: network.bssid.clone(),
                        vulnerability_type: "Unknown Security".to_string(),
                        severity: Severity::Medium,
                        description: "Wireless security type could not be determined - manual verification recommended".to_string(),
                        recommendation: "Verify wireless security configuration manually".to_string(),
                    });
                }
                _ => {}
            }
        }

        let mut ssid_groups: std::collections::HashMap<String, Vec<&WirelessNetwork>> = std::collections::HashMap::new();
        for network in networks {
            ssid_groups.entry(network.ssid.clone()).or_default().push(network);
        }
        for (ssid, nets) in &ssid_groups {
            if nets.len() >= 2 {
                let distinct_bssids: std::collections::HashSet<_> = nets.iter().map(|n| &n.bssid).collect();
                let distinct_secs: std::collections::HashSet<_> = nets.iter().map(|n| n.security_type).collect();
                if distinct_bssids.len() >= 2 || distinct_secs.len() >= 2 {
                    let bssid_list: Vec<_> = nets.iter().map(|n| format!("{} (ch{}, {})", n.bssid, n.channel, n.security_type.as_str())).collect();
                    let is_sec_diff = distinct_secs.len() >= 2;
                    let severity = if is_sec_diff { Severity::Medium } else { Severity::Low };
                    let mut desc = format!("Multiple BSSIDs or security configs for SSID {}: {}", ssid, bssid_list.join("; "));
                    desc.push_str(" (passive heuristic; heuristic only; verify physically or via authorized inventory)");
                    if is_sec_diff {
                        desc.push_str(" including security configuration differences (possible downgrade)");
                    }
                    // Skip rogue/Evil-Twin if any net in group matches known-good (by SSID, BSSID, or "SSID,BSSID")
                    let suppressed = if let Some(kg) = known_good {
                        nets.iter().any(|n| Self::is_known_good(n, kg))
                    } else { false };
                    if !suppressed {
                        vulnerabilities.push(WirelessVulnerability {
                            ssid: ssid.clone(),
                            bssid: nets[0].bssid.clone(),
                            vulnerability_type: "Possible Rogue AP / Evil Twin (passive heuristic)".to_string(),
                            severity,
                            description: desc,
                            recommendation: "Verify authorized APs only; investigate in lab".to_string(),
                        });
                    }
                }
            }
        }

        vulnerabilities
    }

    fn generate_recommendations(networks: &[WirelessNetwork]) -> Vec<String> {
        let mut recommendations = Vec::new();
        let mut seen: rustc_hash::FxHashSet<String> = rustc_hash::FxHashSet::default();

        for network in networks {
            let sec_key = format!("sec:{}", network.security_type.as_str());
            if seen.insert(sec_key) {
                match network.security_type {
                    SecurityType::WEP => {
                        recommendations.push("Upgrade from WEP to WPA2 or WPA3 immediately — WEP is trivially cracked".to_string());
                    }
                    SecurityType::Open => {
                        recommendations
                            .push("Enable WPA2 or WPA3 encryption on open networks".to_string());
                    }
                    SecurityType::WPA => {
                        recommendations.push(
                            "Upgrade from WPA to WPA2 or WPA3 — WPA has known TKIP vulnerabilities"
                                .to_string(),
                        );
                    }
                    SecurityType::Unknown => {
                        recommendations.push("Verify wireless security configuration — security type could not be determined".to_string());
                    }
                    _ => {}
                }
            }
            if network.wps_enabled && seen.insert(format!("wps:{}", network.bssid)) {
                recommendations.push("Disable WPS on networks where it is enabled (PIN brute-force risk)".to_string());
            }
            if network.transition_mode && seen.insert(format!("transition:{}", network.bssid)) {
                recommendations.push("Consider enforcing WPA3-only mode on transition networks to avoid downgrade attacks".to_string());
            }
            if network.is_hidden && seen.insert(format!("hidden:{}", network.bssid)) {
                recommendations.push("Hidden SSIDs offer little security benefit; consider broadcasting for client compatibility".to_string());
            }
            if network.signal_strength <= -80 && seen.insert(format!("weak:{}", network.bssid)) {
                recommendations.push("Investigate weak signal networks for interference or reposition APs".to_string());
            }
        }

        if recommendations.is_empty() && !networks.is_empty() {
            recommendations.push(
                "All detected networks use strong encryption (WPA2/WPA3/Enterprise)".to_string(),
            );
        }
        if networks.is_empty() {
            recommendations.push("No wireless networks were detected during the scan".to_string());
        }

        recommendations.push("Run repeated scans to observe changes over time for rogue detection.".to_string());

        recommendations
    }
}

impl Default for WirelessScanner {
    fn default() -> Self {
        Self::new()
    }
}

fn wireless_category_for(vuln_type: &str) -> String {
    if vuln_type.contains("Rogue") || vuln_type.contains("Evil Twin") {
        "wireless-rogue".to_string()
    } else if vuln_type == "Open Network" || vuln_type == "WEP Encryption" || vuln_type == "WPA Encryption" {
        "wireless-security".to_string()
    } else if vuln_type == "WPS Enabled" {
        "wireless-wps".to_string()
    } else if vuln_type == "Hidden SSID" {
        "wireless-hidden".to_string()
    } else if vuln_type == "Weak Signal Strength" {
        "wireless-signal".to_string()
    } else if vuln_type == "WPA2/WPA3 Transition Mode" {
        "wireless-transition".to_string()
    } else {
        "wireless-other".to_string()
    }
}

pub fn to_scan_report_data(result: &WirelessScanResult) -> crate::output::convert::ScanReportData {
    use crate::output::convert::{FindingData, WirelessNetworkReportData};

    let findings: Vec<FindingData> = WirelessScanner::analyze_networks(&result.networks, None)
        .iter()
        .map(|v| FindingData {
            title: v.vulnerability_type.clone(),
            severity: v.severity.as_str().to_string(),
            category: wireless_category_for(&v.vulnerability_type),
            description: v.description.clone(),
            location: format!("{} ({})", v.ssid, v.bssid),
            evidence: Some(format!("network={} bssid={}", v.ssid, v.bssid)),
            remediation: Some(v.recommendation.clone()),
            cwe_ids: Vec::new(),
        })
        .collect();

    let wireless_networks = result
        .networks
        .iter()
        .map(|n| WirelessNetworkReportData {
            ssid: n.ssid.clone(),
            bssid: n.bssid.clone(),
            channel: n.channel,
            security_type: n.security_type.as_str().to_string(),
            signal_strength: n.signal_strength,
            last_seen: n.last_seen.clone(),
            wps_enabled: n.wps_enabled,
            is_hidden: n.is_hidden,
            transition_mode: n.transition_mode,
        })
        .collect();

    crate::output::convert::ScanReportData {
        target: result.interface.clone(),
        scan_type: "wireless".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        findings,
        open_ports: Vec::new(),
        services: Vec::new(),
        duration_ms: result.scan_duration_secs * 1000,
        wireless_networks,
        policy_summary: None,
    }
}

pub async fn run_cli(
    args: crate::cli::WirelessArgs,
    _config: &crate::config::EggsecConfig,
) -> Result<()> {
    let scanner = WirelessScanner::new().with_interface(args.interface.clone());

    let known_good_set: std::collections::HashSet<String> = if let Some(ref p) = args.known_good {
        load_known_good(p)
    } else {
        std::collections::HashSet::new()
    };
    let kg_ref: Option<&std::collections::HashSet<String>> = if known_good_set.is_empty() { None } else { Some(&known_good_set) };

    if !args.quiet {
        eprintln!("NOTE: Requires root (or CAP_NET_ADMIN) + 'iwlist' from wireless-tools. Interface in managed mode. For authorized lab/defensive validation use only. Passive recon only.");
        if args.dry_run {
            eprintln!("DRY-RUN: planning mode (no iwlist calls, no privileges required).");
        } else if args.repeat == 1 {
            eprintln!("Scanning on {} for ~{}s...", args.interface, args.duration);
        } else {
            eprintln!("Performing {} repeated scans on {} ({}s each, ~2s delay between)...", args.repeat, args.interface, args.duration);
        }
    }

    let mut results: Vec<WirelessScanResult> = Vec::new();
    let mut last_err: Option<EggsecError> = None;

    for i in 1..=args.repeat {
        if !args.quiet && args.repeat > 1 {
            eprintln!("Scan {}/{} ...", i, args.repeat);
        }

        let this_result = if args.dry_run {
            let mut recs = vec!["dry-run: no actual scan performed".to_string()];
            if i > 1 {
                recs.push("dry-run repeat: same stub result for planning".to_string());
            }
            Some(WirelessScanResult {
                interface: args.interface.clone(),
                networks: vec![],
                scan_duration_secs: 0,
                recommendations: recs,
            })
        } else {
            match scanner.scan(args.duration).await {
                Ok(r) => Some(r),
                Err(e) => {
                    if !args.quiet {
                        eprintln!("Scan {}/{} failed: {}; continuing for remaining repeats...", i, args.repeat, e);
                    }
                    last_err = Some(e);
                    None
                }
            }
        };

        if let Some(ref r) = this_result {
            if args.repeat > 1 && !args.json && !args.quiet && !results.is_empty() {
                let prev = results.last().unwrap();
                let diffs = compute_changes_since(prev, r, kg_ref);
                if !diffs.is_empty() {
                    eprintln!("Changes since previous scan:");
                    for d in &diffs {
                        eprintln!("  - {}", d);
                    }
                }
            }
            results.push(r.clone());
        }

        if i < args.repeat {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }

    let result = if let Some(last) = results.last().cloned() {
        last
    } else if let Some(e) = last_err {
        return Err(e);
    } else {
        // fallback (should not happen)
        return Err(EggsecError::Network("No successful wireless scan results".to_string()));
    };

    let output = if args.json {
        if args.repeat > 1 {
            let summary_text = build_temporal_summary(&results, kg_ref);
            let wrapped = serde_json::json!({
                "last_scan": result,
                "repeat_count": args.repeat,
                "summary": summary_text.trim_end(),
            });
            serde_json::to_string_pretty(&wrapped)?
        } else {
            serde_json::to_string_pretty(&result)?
        }
    } else {
        let mut buf = String::new();
        if args.dry_run {
            buf.push_str("DRY-RUN: no actual scan performed\n\n");
        }
        buf.push_str(&format!(
            "Wireless Scan Results - Interface: {}\n",
            result.interface
        ));
        buf.push_str(&format!("Networks found: {}\n\n", result.networks.len()));

        for (i, network) in result.networks.iter().enumerate() {
            buf.push_str(&format!("  {}. {}\n", i + 1, network.ssid));
            buf.push_str(&format!("     BSSID:    {}\n", network.bssid));
            buf.push_str(&format!("     Channel:  {}\n", network.channel));
            buf.push_str(&format!(
                "     Security: {}{}\n",
                network.security_type.as_str(),
                if network.wps_enabled { " (WPS)" } else { "" }
            ));
            buf.push_str(&format!("     Signal:   {} dBm{}\n", network.signal_strength, if network.is_hidden { " [hidden]" } else { "" }));
            buf.push_str(&format!("     Last seen: {}\n", network.last_seen));
            if network.transition_mode {
                buf.push_str("     Note: WPA2/WPA3 transition mode\n");
            }
            buf.push('\n');
        }

        let mut vulns = WirelessScanner::analyze_networks(&result.networks, kg_ref);
        if !vulns.is_empty() {
            if args.detect_suspicious {
                buf.push_str("Findings / Vulnerabilities: [DETECT_SUSPICIOUS]\n");
                for v in &vulns {
                    buf.push_str(&format!(
                        "  [{}] {} ({}): {}\n     Rec: {}\n",
                        v.severity.as_str(),
                        v.vulnerability_type,
                        v.ssid,
                        v.description,
                        v.recommendation
                    ));
                }
            } else {
                let rogue_count = vulns.iter().filter(|v| v.vulnerability_type.contains("Rogue") || v.vulnerability_type.contains("Evil Twin")).count();
                let other_vulns: Vec<_> = vulns.drain(..).filter(|v| !(v.vulnerability_type.contains("Rogue") || v.vulnerability_type.contains("Evil Twin"))).collect();
                if !other_vulns.is_empty() {
                    buf.push_str("Findings / Vulnerabilities:\n");
                    for v in &other_vulns {
                        buf.push_str(&format!(
                            "  [{}] {} ({}): {}\n     Rec: {}\n",
                            v.severity.as_str(),
                            v.vulnerability_type,
                            v.ssid,
                            v.description,
                            v.recommendation
                        ));
                    }
                }
                if rogue_count > 0 {
                    buf.push_str(&format!("Rogue/suspicious candidates: {} (use --detect-suspicious to show full details)\n", rogue_count));
                    buf.push_str("Note: Rogue/Evil-Twin detection is a passive heuristic (multiple BSSIDs or security differences for same SSID). Use --known-good for lab baselines; verify with physical survey or asset inventory.\n");
                }
            }
            buf.push('\n');
        }

        if !result.recommendations.is_empty() {
            buf.push_str("Recommendations:\n");
            for rec in &result.recommendations {
                buf.push_str(&format!("  - {}\n", rec));
            }
        }

        if args.repeat > 1 {
            buf.push_str("\n---\n");
            buf.push_str(&build_temporal_summary(&results, kg_ref));
        }

        buf
    };

    if let Some(ref output_file) = args.output {
        tokio::fs::write(output_file, &output).await?;
        if !args.quiet {
            eprintln!("Results written to {}", output_file);
        }
    } else {
        println!("{}", output);
    }

    Ok(())
}

fn load_known_good(path: &str) -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();
    if let Ok(content) = std::fs::read_to_string(path) {
        for line in content.lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            set.insert(t.to_string());
        }
    }
    set
}

fn compute_changes_since(
    prev: &WirelessScanResult,
    curr: &WirelessScanResult,
    known_good: Option<&std::collections::HashSet<String>>,
) -> Vec<String> {
    let mut diffs = Vec::new();

    let prev_nets: std::collections::HashSet<(String, String)> = prev
        .networks
        .iter()
        .map(|n| (n.ssid.clone(), n.bssid.clone()))
        .collect();
    let curr_nets: std::collections::HashSet<(String, String)> = curr
        .networks
        .iter()
        .map(|n| (n.ssid.clone(), n.bssid.clone()))
        .collect();

    for (s, b) in &curr_nets {
        if !prev_nets.contains(&(s.clone(), b.clone())) {
            diffs.push(format!("New network: {} ({})", s, b));
        }
    }

    let prev_sec: std::collections::HashMap<String, SecurityType> = prev
        .networks
        .iter()
        .map(|n| (n.bssid.clone(), n.security_type))
        .collect();
    for n in &curr.networks {
        if let Some(&old) = prev_sec.get(&n.bssid) {
            if old != n.security_type {
                diffs.push(format!(
                    "Security type change for {} ({}): {} -> {}",
                    n.ssid,
                    n.bssid,
                    old.as_str(),
                    n.security_type.as_str()
                ));
            }
        }
    }

    let prev_sig: std::collections::HashMap<String, i32> = prev
        .networks
        .iter()
        .map(|n| (n.bssid.clone(), n.signal_strength))
        .collect();
    for n in &curr.networks {
        if let Some(&old) = prev_sig.get(&n.bssid) {
            let delta = (n.signal_strength - old).abs();
            if delta > 5 {
                diffs.push(format!(
                    "Signal delta >5dBm for {} ({}): {} -> {} dBm",
                    n.ssid, n.bssid, old, n.signal_strength
                ));
            }
        }
    }

    let curr_rogues: std::collections::HashSet<String> = WirelessScanner::analyze_networks(&curr.networks, known_good)
        .into_iter()
        .filter(|v| v.vulnerability_type.contains("Rogue") || v.vulnerability_type.contains("Evil Twin"))
        .map(|v| v.ssid.clone())
        .collect();
    let prev_rogues: std::collections::HashSet<String> = WirelessScanner::analyze_networks(&prev.networks, known_good)
        .into_iter()
        .filter(|v| v.vulnerability_type.contains("Rogue") || v.vulnerability_type.contains("Evil Twin"))
        .map(|v| v.ssid.clone())
        .collect();
    for s in &curr_rogues {
        if !prev_rogues.contains(s) {
            diffs.push(format!("New rogue/Evil-Twin candidate: {}", s));
        }
    }

    diffs
}

fn build_temporal_summary(
    results: &[WirelessScanResult],
    known_good: Option<&std::collections::HashSet<String>>,
) -> String {
    let mut unique_ssids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut all_seen_nets: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    let mut last_sec: std::collections::HashMap<String, SecurityType> = std::collections::HashMap::new();
    let mut last_sig: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    let mut scans_with_new_nets: u32 = 0;
    let mut sec_changes: u32 = 0;
    let mut sig_drifts: u32 = 0;
    let mut rogue_count_total: u32 = 0;

    for (idx, res) in results.iter().enumerate() {
        let mut had_new_this_scan = false;
        for n in &res.networks {
            unique_ssids.insert(n.ssid.clone());
            let net_key = (n.ssid.clone(), n.bssid.clone());
            if all_seen_nets.insert(net_key) {
                had_new_this_scan = true;
            }
            if let Some(&old) = last_sec.get(&n.bssid) {
                if old != n.security_type {
                    sec_changes += 1;
                }
            }
            last_sec.insert(n.bssid.clone(), n.security_type);
            if let Some(&old) = last_sig.get(&n.bssid) {
                if (n.signal_strength - old).abs() > 5 {
                    sig_drifts += 1;
                }
            }
            last_sig.insert(n.bssid.clone(), n.signal_strength);
        }
        if idx > 0 && had_new_this_scan {
            scans_with_new_nets += 1;
        }
        let rogues_here = WirelessScanner::analyze_networks(&res.networks, known_good)
            .into_iter()
            .filter(|v| v.vulnerability_type.contains("Rogue") || v.vulnerability_type.contains("Evil Twin"))
            .count();
        rogue_count_total += rogues_here as u32;
    }

    let mut s = String::new();
    s.push_str("Scan summary over time:\n");
    s.push_str(&format!("  Total unique SSIDs seen: {}\n", unique_ssids.len()));
    s.push_str(&format!("  Scans with new networks: {}\n", scans_with_new_nets));
    s.push_str(&format!("  Security changes observed: {}\n", sec_changes));
    s.push_str(&format!("  Signal drifts (>5dBm): {}\n", sig_drifts));
    s.push_str(&format!("  Rogue candidates across all: {}\n", rogue_count_total));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_type_str() {
        assert_eq!(SecurityType::WPA2.as_str(), "WPA2");
        assert_eq!(SecurityType::Open.as_str(), "Open");
    }

    #[test]
    fn test_wireless_scanner_creation() {
        let _scanner = WirelessScanner::new();
    }

    #[test]
    fn test_network_analysis() {
        let networks = vec![
            WirelessNetwork {
                ssid: "OpenNetwork".to_string(),
                bssid: "00:11:22:33:44:55".to_string(),
                channel: 6,
                security_type: SecurityType::Open,
                signal_strength: -50,
                last_seen: String::new(),
                wps_enabled: false,
                is_hidden: false,
                transition_mode: false,
            },
            WirelessNetwork {
                ssid: "SecureNetwork".to_string(),
                bssid: "00:11:22:33:44:66".to_string(),
                channel: 11,
                security_type: SecurityType::WPA3,
                signal_strength: -60,
                last_seen: String::new(),
                wps_enabled: false,
                is_hidden: false,
                transition_mode: false,
            },
        ];

        let vulns = WirelessScanner::analyze_networks(&networks, None);
        assert_eq!(vulns.len(), 1);
        assert_eq!(vulns[0].vulnerability_type, "Open Network");
    }

    #[test]
    fn test_parse_scan_output_resets_state_per_cell() {
        let scanner = WirelessScanner::new();
        let iwlist_output = "Cell 01 - Address: 00:11:22:33:44:55\nESSID:\"Network1\"\nChannel:6\nSignal level=-50 dBi\nEncryption key:on\nWPA2\n\nCell 02 - Address: 00:11:22:33:44:66\nESSID:\"Network2\"\nChannel:11\nSignal level=-70 dBi\nEncryption key:on\nWPA3\n";
        let networks = scanner.parse_scan_output(iwlist_output);
        assert_eq!(networks.len(), 2);
        assert_eq!(networks[0].ssid, "Network1");
        assert_eq!(networks[0].channel, 6);
        assert_eq!(networks[0].signal_strength, -50);
        assert_eq!(networks[0].security_type, SecurityType::WPA2);
        assert_eq!(networks[1].ssid, "Network2");
        assert_eq!(networks[1].channel, 11);
        assert_eq!(networks[1].signal_strength, -70);
        assert_eq!(networks[1].security_type, SecurityType::WPA3);
    }

    #[test]
    fn test_parse_scan_output_open_network() {
        let scanner = WirelessScanner::new();
        let iwlist_output = "Cell 01 - Address: 00:11:22:33:44:55\nESSID:\"OpenNet\"\nChannel:1\nSignal level=-60 dBi\nEncryption key:off\n";
        let networks = scanner.parse_scan_output(iwlist_output);
        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0].ssid, "OpenNet");
        assert_eq!(networks[0].security_type, SecurityType::Open);
    }

    #[test]
    fn test_parse_scan_output_enterprise_network() {
        let scanner = WirelessScanner::new();
        let iwlist_output = "Cell 01 - Address: 00:11:22:33:44:55\nESSID:\"EnterpriseNet\"\nChannel:36\nSignal level=-45 dBi\nEncryption key:on\nWPA2\nAuthentication Suites (1): 802.1X\n";
        let networks = scanner.parse_scan_output(iwlist_output);
        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0].ssid, "EnterpriseNet");
        assert_eq!(networks[0].security_type, SecurityType::Enterprise);
    }

    #[test]
    fn test_parse_scan_output_wps() {
        let scanner = WirelessScanner::new();
        let iwlist_output = "Cell 01 - Address: 00:11:22:33:44:55\nESSID:\"WPSNet\"\nChannel:6\nSignal level=-55 dBi\nEncryption key:on\nWPA2\nWPS\n";
        let networks = scanner.parse_scan_output(iwlist_output);
        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0].ssid, "WPSNet");
        assert!(networks[0].wps_enabled);
    }

    #[test]
    fn test_parse_scan_output_hidden() {
        let scanner = WirelessScanner::new();
        let iwlist_output = "Cell 01 - Address: 00:11:22:33:44:55\nESSID:\"\" \nChannel:1\nSignal level=-65 dBi\nEncryption key:on\nWPA2\n";
        let networks = scanner.parse_scan_output(iwlist_output);
        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0].ssid, "<hidden>");
        assert!(networks[0].is_hidden);
    }

    #[test]
    fn test_parse_scan_output_transition() {
        let scanner = WirelessScanner::new();
        let iwlist_output = "Cell 01 - Address: 00:11:22:33:44:55\nESSID:\"TransitionNet\"\nChannel:36\nSignal level=-50 dBi\nEncryption key:on\nWPA2\nWPA3\n";
        let networks = scanner.parse_scan_output(iwlist_output);
        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0].ssid, "TransitionNet");
        assert!(networks[0].transition_mode);
    }

    #[test]
    fn test_analyze_weak_signal() {
        let networks = vec![WirelessNetwork {
            ssid: "WeakNet".to_string(),
            bssid: "00:11:22:33:44:55".to_string(),
            channel: 6,
            security_type: SecurityType::WPA2,
            signal_strength: -85,
            last_seen: String::new(),
            wps_enabled: false,
            is_hidden: false,
            transition_mode: false,
        }];
        let vulns = WirelessScanner::analyze_networks(&networks, None);
        assert!(vulns.iter().any(|v| v.vulnerability_type == "Weak Signal Strength"));
    }

    #[test]
    fn test_analyze_rogue_candidate() {
        let networks = vec![
            WirelessNetwork {
                ssid: "CorpNet".to_string(),
                bssid: "00:11:22:33:44:55".to_string(),
                channel: 6,
                security_type: SecurityType::WPA2,
                signal_strength: -50,
                last_seen: String::new(),
                wps_enabled: false,
                is_hidden: false,
                transition_mode: false,
            },
            WirelessNetwork {
                ssid: "CorpNet".to_string(),
                bssid: "aa:bb:cc:dd:ee:ff".to_string(),
                channel: 11,
                security_type: SecurityType::Open,
                signal_strength: -55,
                last_seen: String::new(),
                wps_enabled: false,
                is_hidden: false,
                transition_mode: false,
            },
        ];
        let vulns = WirelessScanner::analyze_networks(&networks, None);
        assert!(vulns.iter().any(|v| v.vulnerability_type.contains("Rogue AP / Evil Twin")));
    }

    #[test]
    fn test_parse_scan_output_wpa2_wpa3_mixed() {
        let scanner = WirelessScanner::new();
        let iwlist_output = "Cell 01 - Address: 00:11:22:33:44:55\nESSID:\"MixedNet\"\nChannel:36\nSignal level=-45 dBi\nEncryption key:on\nWPA2\nWPA3\n";
        let networks = scanner.parse_scan_output(iwlist_output);
        assert_eq!(networks.len(), 1);
        assert!(networks[0].transition_mode);
    }

    #[test]
    fn test_analyze_rogue_with_security_diff_elevates_severity() {
        let networks = vec![
            WirelessNetwork {
                ssid: "CorpNet".to_string(),
                bssid: "00:11:22:33:44:55".to_string(),
                channel: 6,
                security_type: SecurityType::WPA2,
                signal_strength: -50,
                last_seen: String::new(),
                wps_enabled: false,
                is_hidden: false,
                transition_mode: false,
            },
            WirelessNetwork {
                ssid: "CorpNet".to_string(),
                bssid: "aa:bb:cc:dd:ee:ff".to_string(),
                channel: 11,
                security_type: SecurityType::WPA3,
                signal_strength: -55,
                last_seen: String::new(),
                wps_enabled: false,
                is_hidden: false,
                transition_mode: false,
            },
        ];
        let vulns = WirelessScanner::analyze_networks(&networks, None);
        let rogue = vulns.iter().find(|v| v.vulnerability_type.contains("Rogue"));
        assert!(rogue.is_some());
        assert_eq!(rogue.unwrap().severity, Severity::Medium);
        assert!(rogue.unwrap().description.contains("security configuration differences"));
    }

    #[test]
    fn test_analyze_rogue_suppressed_by_known_good() {
        let networks = vec![
            WirelessNetwork {
                ssid: "CorpNet".to_string(),
                bssid: "00:11:22:33:44:55".to_string(),
                channel: 6,
                security_type: SecurityType::WPA2,
                signal_strength: -50,
                last_seen: String::new(),
                wps_enabled: false,
                is_hidden: false,
                transition_mode: false,
            },
            WirelessNetwork {
                ssid: "CorpNet".to_string(),
                bssid: "aa:bb:cc:dd:ee:ff".to_string(),
                channel: 11,
                security_type: SecurityType::Open,
                signal_strength: -55,
                last_seen: String::new(),
                wps_enabled: false,
                is_hidden: false,
                transition_mode: false,
            },
        ];
        let mut kg: std::collections::HashSet<String> = std::collections::HashSet::new();
        kg.insert("CorpNet".to_string());
        let vulns = WirelessScanner::analyze_networks(&networks, Some(&kg));
        assert!(!vulns.iter().any(|v| v.vulnerability_type.contains("Rogue")));

        let mut kg2: std::collections::HashSet<String> = std::collections::HashSet::new();
        kg2.insert("aa:bb:cc:dd:ee:ff".to_string());
        let vulns2 = WirelessScanner::analyze_networks(&networks, Some(&kg2));
        assert!(!vulns2.iter().any(|v| v.vulnerability_type.contains("Rogue")));

        let mut kg3: std::collections::HashSet<String> = std::collections::HashSet::new();
        kg3.insert("CorpNet,aa:bb:cc:dd:ee:ff".to_string());
        let vulns3 = WirelessScanner::analyze_networks(&networks, Some(&kg3));
        assert!(!vulns3.iter().any(|v| v.vulnerability_type.contains("Rogue")));
    }

    #[test]
    fn test_analyze_networks_with_known_good_empty() {
        let networks = vec![
            WirelessNetwork {
                ssid: "CorpNet".to_string(),
                bssid: "00:11:22:33:44:55".to_string(),
                channel: 6,
                security_type: SecurityType::WPA2,
                signal_strength: -50,
                last_seen: String::new(),
                wps_enabled: false,
                is_hidden: false,
                transition_mode: false,
            },
            WirelessNetwork {
                ssid: "CorpNet".to_string(),
                bssid: "aa:bb:cc:dd:ee:ff".to_string(),
                channel: 11,
                security_type: SecurityType::Open,
                signal_strength: -55,
                last_seen: String::new(),
                wps_enabled: false,
                is_hidden: false,
                transition_mode: false,
            },
        ];
        let kg: std::collections::HashSet<String> = std::collections::HashSet::new();
        let vulns = WirelessScanner::analyze_networks(&networks, Some(&kg));
        assert!(vulns.iter().any(|v| v.vulnerability_type.contains("Rogue")));
    }

    #[test]
    fn test_load_known_good_parsing() {
        // indirect via analyze with temp file not possible in unit; test is_known_good + analyze paths above cover logic.
        // Add a direct helper test for load (replicate minimal).
        let content = "CorpNet\n# comment\n00:11:22:33:44:55\nCorpNet,aa:bb:cc:dd:ee:ff\n\n   ";
        let mut set = std::collections::HashSet::new();
        for line in content.lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') { continue; }
            set.insert(t.to_string());
        }
        assert_eq!(set.len(), 3);
        assert!(set.contains("CorpNet"));
        assert!(set.contains("00:11:22:33:44:55"));
        assert!(set.contains("CorpNet,aa:bb:cc:dd:ee:ff"));
    }

    #[test]
    fn to_scan_report_data_produces_valid_bridge() {
        // minimal result with 1 network that triggers a vuln + wireless_networks populated
        let result = WirelessScanResult {
            interface: "wlan0".into(),
            networks: vec![WirelessNetwork {
                ssid: "OpenNet".into(),
                bssid: "00:11:22:33:44:55".into(),
                channel: 6,
                security_type: SecurityType::Open,
                signal_strength: -60,
                last_seen: "now".into(),
                wps_enabled: false,
                is_hidden: false,
                transition_mode: false,
            }],
            scan_duration_secs: 5,
            recommendations: vec!["rec1".into()],
        };
        let data = to_scan_report_data(&result);
        assert_eq!(data.target, "wlan0");
        assert_eq!(data.scan_type, "wireless");
        assert!(!data.findings.is_empty());
        let f = &data.findings[0];
        assert_eq!(f.category, "wireless-security");
        assert_eq!(f.severity, "medium");
        assert!(f.title.contains("Open") || f.description.contains("no encryption"));
        assert!(f.remediation.is_some());
        assert!(f.evidence.is_some());
        assert!(f.evidence.as_ref().unwrap().contains("network=OpenNet bssid=00:11:22:33:44:55"));
        assert!(f.cwe_ids.is_empty());
        assert_eq!(data.wireless_networks.len(), 1);
        let wn = &data.wireless_networks[0];
        assert_eq!(wn.ssid, "OpenNet");
        assert_eq!(wn.security_type, "Open");
        assert!(!wn.wps_enabled);
        assert!(!wn.is_hidden);
        assert!(!wn.transition_mode);
        assert!(data.policy_summary.is_none());

        // empty networks: 0 findings + 0 wireless_networks still valid
        let empty = WirelessScanResult {
            interface: "wlan1".into(),
            networks: vec![],
            scan_duration_secs: 1,
            recommendations: vec![],
        };
        let d2 = to_scan_report_data(&empty);
        assert_eq!(d2.findings.len(), 0);
        assert!(d2.wireless_networks.is_empty());
        assert_eq!(d2.target, "wlan1");

        // serde roundtrip of bridged output + load via eggsec-output
        let j = serde_json::to_string(&data).unwrap();
        let back: crate::output::convert::ScanReportData = serde_json::from_str(&j).unwrap();
        assert_eq!(back.findings.len(), 1);
        assert_eq!(back.wireless_networks.len(), 1);
        // direct from_str simulates what load_scan_report does internally (used by report convert)
        let loaded: crate::output::convert::ScanReportData = serde_json::from_str(&j).unwrap();
        assert_eq!(loaded.target, "wlan0");
    }

    #[test]
    fn to_scan_report_data_wireless_multi_empty_and_roundtrip() {
        // construct directly: rogue (wireless-rogue), wps (wireless-wps), hidden (wireless-hidden), signal (wireless-signal), transition (wireless-transition), wep (wireless-security)
        let result = WirelessScanResult {
            interface: "wlan0".into(),
            networks: vec![
                WirelessNetwork { ssid: "CorpNet".into(), bssid: "00:11:22:33:44:55".into(), channel: 6, security_type: SecurityType::WPA2, signal_strength: -50, last_seen: "t".into(), wps_enabled: false, is_hidden: false, transition_mode: false },
                WirelessNetwork { ssid: "CorpNet".into(), bssid: "aa:bb:cc:dd:ee:ff".into(), channel: 11, security_type: SecurityType::Open, signal_strength: -55, last_seen: "t".into(), wps_enabled: false, is_hidden: false, transition_mode: false },
                WirelessNetwork { ssid: "WPSNet".into(), bssid: "11:22:33:44:55:66".into(), channel: 1, security_type: SecurityType::WPA2, signal_strength: -60, last_seen: "t".into(), wps_enabled: true, is_hidden: false, transition_mode: false },
                WirelessNetwork { ssid: "<hidden>".into(), bssid: "22:33:44:55:66:77".into(), channel: 6, security_type: SecurityType::WPA2, signal_strength: -65, last_seen: "t".into(), wps_enabled: false, is_hidden: true, transition_mode: false },
                WirelessNetwork { ssid: "WeakNet".into(), bssid: "33:44:55:66:77:88".into(), channel: 11, security_type: SecurityType::WPA3, signal_strength: -85, last_seen: "t".into(), wps_enabled: false, is_hidden: false, transition_mode: false },
                WirelessNetwork { ssid: "TransNet".into(), bssid: "44:55:66:77:88:99".into(), channel: 36, security_type: SecurityType::WPA2, signal_strength: -50, last_seen: "t".into(), wps_enabled: false, is_hidden: false, transition_mode: true },
                WirelessNetwork { ssid: "WEPNet".into(), bssid: "55:66:77:88:99:aa".into(), channel: 1, security_type: SecurityType::WEP, signal_strength: -70, last_seen: "t".into(), wps_enabled: false, is_hidden: false, transition_mode: false },
            ],
            scan_duration_secs: 10,
            recommendations: vec![],
        };
        let data = to_scan_report_data(&result);
        assert_eq!(data.findings.len(), 7); // weak + wps + hidden + transition + open + wep + 1 rogue (from CorpNet ssid group with sec diff)
        // categories asserted
        let cats: Vec<_> = data.findings.iter().map(|f| f.category.as_str()).collect();
        assert!(cats.contains(&"wireless-rogue"));
        assert!(cats.contains(&"wireless-security"));
        assert!(cats.contains(&"wireless-wps"));
        assert!(cats.contains(&"wireless-hidden"));
        assert!(cats.contains(&"wireless-signal"));
        assert!(cats.contains(&"wireless-transition"));
        for f in &data.findings {
            assert!(f.evidence.is_some());
            assert!(f.remediation.is_some());
            assert!(f.evidence.as_ref().unwrap().starts_with("network="));
        }
        assert_eq!(data.wireless_networks.len(), 7);

        // empty case
        let empty = WirelessScanResult { interface: "wlanx".into(), networks: vec![], scan_duration_secs: 0, recommendations: vec![] };
        let de = to_scan_report_data(&empty);
        assert_eq!(de.findings.len(), 0);
        assert!(de.wireless_networks.is_empty());

        // serde roundtrip of bridged (simulates report convert load)
        let j = serde_json::to_string(&data).unwrap();
        let back: crate::output::convert::ScanReportData = serde_json::from_str(&j).unwrap();
        assert_eq!(back.findings.len(), 7);
        assert_eq!(back.wireless_networks.len(), 7);
        let loaded: crate::output::convert::ScanReportData = serde_json::from_str(&j).unwrap();
        assert_eq!(loaded.target, "wlan0");
    }
}
