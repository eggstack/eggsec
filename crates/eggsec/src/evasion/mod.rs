//! Evasion detection module (feature-gated behind `evasion`).
//!
//! Defense-lab-only module for validating that security controls detect common
//! evasion techniques. Maps detections to MITRE ATT&CK IDs and produces
//! structured reports with confidence scores and recommendations.
//!
//! Safety: all operations are either dry-run (synthetic results only) or
//! perform passive observation (file existence, process enumeration, network
//! patterns). No active exploitation. Explicit lab-only framing.
//!
//! Standalone defense-lab surface. No MCP/agent/TUI/pipeline integration.

use crate::error::Result;
use crate::types::Severity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvasionTarget {
    pub target_type: EvasionTargetType,
    pub path: Option<String>,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvasionTargetType {
    Process,
    File,
    Network,
    Registry,
    Memory,
}

impl EvasionTargetType {
    pub fn as_str(&self) -> &str {
        match self {
            EvasionTargetType::Process => "process",
            EvasionTargetType::File => "file",
            EvasionTargetType::Network => "network",
            EvasionTargetType::Registry => "registry",
            EvasionTargetType::Memory => "memory",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvasionTechnique {
    pub id: String,
    pub name: String,
    pub mitre_id: Option<String>,
    pub category: EvasionCategory,
    pub risk_level: EvasionRisk,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvasionCategory {
    Syscall,
    HookBypass,
    Obfuscation,
    Injection,
    AntiAnalysis,
    TrafficObfuscation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvasionRisk {
    Low,
    Medium,
    High,
    Critical,
}

impl EvasionRisk {
    pub fn as_str(&self) -> &str {
        match self {
            EvasionRisk::Low => "low",
            EvasionRisk::Medium => "medium",
            EvasionRisk::High => "high",
            EvasionRisk::Critical => "critical",
        }
    }

    pub fn to_severity(&self) -> Severity {
        match self {
            EvasionRisk::Low => Severity::Low,
            EvasionRisk::Medium => Severity::Medium,
            EvasionRisk::High => Severity::High,
            EvasionRisk::Critical => Severity::Critical,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvasionDetection {
    pub technique: EvasionTechnique,
    pub detected: bool,
    pub confidence: f64,
    pub evidence: Option<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvasionReport {
    pub target: String,
    pub detections: Vec<EvasionDetection>,
    pub summary: EvasionSummary,
    pub timestamp: String,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvasionSummary {
    pub total_techniques: usize,
    pub detected: usize,
    pub not_detected: usize,
    pub detection_rate: f64,
}

pub struct EvasionScanner {
    dry_run: bool,
    techniques: Vec<EvasionTechnique>,
}

impl EvasionScanner {
    pub fn new(dry_run: bool) -> Self {
        Self {
            dry_run,
            techniques: Self::default_techniques(),
        }
    }

    fn default_techniques() -> Vec<EvasionTechnique> {
        vec![
            EvasionTechnique {
                id: "evasion-syscall-001".to_string(),
                name: "Direct Syscall Detection".to_string(),
                mitre_id: Some("T1106".to_string()),
                category: EvasionCategory::Syscall,
                risk_level: EvasionRisk::High,
                description: "Detects direct syscall usage that bypasses standard API hooks".to_string(),
            },
            EvasionTechnique {
                id: "evasion-syscall-002".to_string(),
                name: "Indirect Syscall Detection".to_string(),
                mitre_id: Some("T1106".to_string()),
                category: EvasionCategory::Syscall,
                risk_level: EvasionRisk::High,
                description: "Detects indirect syscall patterns used to evade userland hooks".to_string(),
            },
            EvasionTechnique {
                id: "evasion-hook-001".to_string(),
                name: "ETW Patching Detection".to_string(),
                mitre_id: Some("T1562.006".to_string()),
                category: EvasionCategory::HookBypass,
                risk_level: EvasionRisk::Critical,
                description: "Detects Event Tracing for Windows bypass attempts".to_string(),
            },
            EvasionTechnique {
                id: "evasion-hook-002".to_string(),
                name: "AMSI Bypass Detection".to_string(),
                mitre_id: Some("T1562.001".to_string()),
                category: EvasionCategory::HookBypass,
                risk_level: EvasionRisk::Critical,
                description: "Detects Antimalware Scan Interface bypass attempts".to_string(),
            },
            EvasionTechnique {
                id: "evasion-hook-003".to_string(),
                name: "Userland Hook Unhooking".to_string(),
                mitre_id: Some("T1014".to_string()),
                category: EvasionCategory::HookBypass,
                risk_level: EvasionRisk::High,
                description: "Detects removal of userland API hooks placed by security products".to_string(),
            },
            EvasionTechnique {
                id: "evasion-obf-001".to_string(),
                name: "String Obfuscation Detection".to_string(),
                mitre_id: Some("T1027".to_string()),
                category: EvasionCategory::Obfuscation,
                risk_level: EvasionRisk::Medium,
                description: "Detects encoded/encrypted strings used to hide malicious payloads".to_string(),
            },
            EvasionTechnique {
                id: "evasion-obf-002".to_string(),
                name: "Code Segment Obfuscation".to_string(),
                mitre_id: Some("T1027.005".to_string()),
                category: EvasionCategory::Obfuscation,
                risk_level: EvasionRisk::Medium,
                description: "Detects code obfuscation techniques in executables".to_string(),
            },
            EvasionTechnique {
                id: "evasion-inj-001".to_string(),
                name: "Process Hollowing Detection".to_string(),
                mitre_id: Some("T1055.012".to_string()),
                category: EvasionCategory::Injection,
                risk_level: EvasionRisk::Critical,
                description: "Detects process hollowing injection technique".to_string(),
            },
            EvasionTechnique {
                id: "evasion-inj-002".to_string(),
                name: "DLL Side-Loading Detection".to_string(),
                mitre_id: Some("T1574.002".to_string()),
                category: EvasionCategory::Injection,
                risk_level: EvasionRisk::High,
                description: "Detects DLL side-loading for defense evasion".to_string(),
            },
            EvasionTechnique {
                id: "evasion-inj-003".to_string(),
                name: "Reflective DLL Loading".to_string(),
                mitre_id: Some("T1620".to_string()),
                category: EvasionCategory::Injection,
                risk_level: EvasionRisk::Critical,
                description: "Detects reflective DLL injection without touching disk".to_string(),
            },
            EvasionTechnique {
                id: "evasion-anti-001".to_string(),
                name: "VM Detection".to_string(),
                mitre_id: Some("T1497.001".to_string()),
                category: EvasionCategory::AntiAnalysis,
                risk_level: EvasionRisk::Medium,
                description: "Detects virtual machine / sandbox detection techniques".to_string(),
            },
            EvasionTechnique {
                id: "evasion-anti-002".to_string(),
                name: "Debugger Detection".to_string(),
                mitre_id: Some("T1622".to_string()),
                category: EvasionCategory::AntiAnalysis,
                risk_level: EvasionRisk::Medium,
                description: "Detects debugger presence checks".to_string(),
            },
            EvasionTechnique {
                id: "evasion-anti-003".to_string(),
                name: "Timing-Based Evasion".to_string(),
                mitre_id: Some("T1497".to_string()),
                category: EvasionCategory::AntiAnalysis,
                risk_level: EvasionRisk::Low,
                description: "Detects sleep/timing checks used to evade automated analysis".to_string(),
            },
            EvasionTechnique {
                id: "evasion-traffic-001".to_string(),
                name: "Domain Fronting Detection".to_string(),
                mitre_id: Some("T1090.004".to_string()),
                category: EvasionCategory::TrafficObfuscation,
                risk_level: EvasionRisk::High,
                description: "Detects domain fronting for C2 traffic obfuscation".to_string(),
            },
            EvasionTechnique {
                id: "evasion-traffic-002".to_string(),
                name: "DNS-over-HTTPS Tunneling".to_string(),
                mitre_id: Some("T1071.004".to_string()),
                category: EvasionCategory::TrafficObfuscation,
                risk_level: EvasionRisk::High,
                description: "Detects DNS-over-HTTPS used to tunnel C2 communications".to_string(),
            },
            EvasionTechnique {
                id: "evasion-traffic-003".to_string(),
                name: "Jittered Beacon Detection".to_string(),
                mitre_id: Some("T1071".to_string()),
                category: EvasionCategory::TrafficObfuscation,
                risk_level: EvasionRisk::Medium,
                description: "Detects C2 beacon traffic with randomized timing jitter".to_string(),
            },
        ]
    }

    pub async fn scan(&self, target: &EvasionTarget) -> Result<EvasionReport> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let target_label = match (&target.target_type, &target.path, target.pid) {
            (t, Some(p), _) => format!("{}:{}", t.as_str(), p),
            (t, None, Some(pid)) => format!("{}:pid={}", t.as_str(), pid),
            (t, None, None) => t.as_str().to_string(),
        };

        let detections = if self.dry_run {
            self.dry_run_detections()
        } else {
            self.real_detections(target).await
        };

        let detected = detections.iter().filter(|d| d.detected).count();
        let total = detections.len();

        let summary = EvasionSummary {
            total_techniques: total,
            detected,
            not_detected: total - detected,
            detection_rate: if total > 0 {
                detected as f64 / total as f64
            } else {
                0.0
            },
        };

        Ok(EvasionReport {
            target: target_label,
            detections,
            summary,
            timestamp,
            dry_run: self.dry_run,
        })
    }

    fn dry_run_detections(&self) -> Vec<EvasionDetection> {
        self.techniques
            .iter()
            .map(|technique| {
                let confidence = match technique.risk_level {
                    EvasionRisk::Critical => 0.85,
                    EvasionRisk::High => 0.75,
                    EvasionRisk::Medium => 0.65,
                    EvasionRisk::Low => 0.55,
                };
                EvasionDetection {
                    technique: technique.clone(),
                    detected: true,
                    confidence,
                    evidence: Some(format!(
                        "dry-run: synthetic detection for {} ({})",
                        technique.name, technique.id
                    )),
                    recommendations: vec![
                        format!("Verify {} detection in live environment", technique.name),
                        "Review security control logging for this technique".to_string(),
                    ],
                }
            })
            .collect()
    }

    async fn real_detections(&self, target: &EvasionTarget) -> Vec<EvasionDetection> {
        let mut detections = Vec::new();
        for technique in &self.techniques {
            let detection = match technique.category {
                EvasionCategory::Syscall => self.check_syscall_evasion(technique, target).await,
                EvasionCategory::HookBypass => self.check_hook_bypass(technique, target).await,
                EvasionCategory::Obfuscation => self.check_obfuscation(technique, target).await,
                EvasionCategory::Injection => self.check_injection(technique, target).await,
                EvasionCategory::AntiAnalysis => self.check_anti_analysis(technique, target).await,
                EvasionCategory::TrafficObfuscation => {
                    self.check_traffic_obfuscation(technique, target).await
                }
            };
            detections.push(detection);
        }
        detections
    }

    async fn check_syscall_evasion(
        &self,
        technique: &EvasionTechnique,
        target: &EvasionTarget,
    ) -> EvasionDetection {
        let mut detected = false;
        let mut evidence = None;
        let mut confidence = 0.0;

        if let EvasionTargetType::Process = target.target_type {
            if let Some(path) = &target.path {
                if let Ok(bytes) = tokio::fs::read(path).await {
                    let syscall_patterns: &[&[u8]] = &[
                        b"syscall", b"NtCreateFile", b"NtWriteVirtualMemory", b"ZwCreateSection",
                    ];
                    let mut matches = 0;
                    for pattern in syscall_patterns {
                        if bytes.windows(pattern.len()).any(|w| w == *pattern) {
                            matches += 1;
                        }
                    }
                    if matches > 0 {
                        detected = true;
                        confidence = 0.3 + (matches as f64 * 0.15).min(0.5);
                        evidence = Some(format!("Found {} syscall-related patterns in binary", matches));
                    }
                }
            }
        }

        EvasionDetection {
            technique: technique.clone(),
            detected,
            confidence,
            evidence,
            recommendations: if detected {
                vec![
                    "Monitor for direct syscall usage via ETW or kernel callbacks".to_string(),
                    "Implement syscall hooking detection in security controls".to_string(),
                ]
            } else {
                vec!["No indicators found; technique may use advanced evasion".to_string()]
            },
        }
    }

    async fn check_hook_bypass(
        &self,
        technique: &EvasionTechnique,
        target: &EvasionTarget,
    ) -> EvasionDetection {
        let mut detected = false;
        let mut evidence = None;
        let mut confidence = 0.0;

        if let Some(path) = &target.path {
            if let Ok(bytes) = tokio::fs::read(path).await {
                match technique.id.as_str() {
                    "evasion-hook-001" => {
                        let patterns: &[&[u8]] = &[b"EtwpEventWrite", b"EventWrite", b"ntdll!Etwp"];
                        for p in patterns {
                            if bytes.windows(p.len()).any(|w| w == *p) {
                                detected = true;
                                confidence = 0.4;
                                evidence = Some(format!("Found ETW-related symbol: {:?}", String::from_utf8_lossy(p)));
                                break;
                            }
                        }
                    }
                    "evasion-hook-002" => {
                        let patterns: &[&[u8]] = &[b"AmsiScanBuffer", b"AmsiScanString", b"amsiInitFailed", b"amsi.dll"];
                        for p in patterns {
                            if bytes.windows(p.len()).any(|w| w == *p) {
                                detected = true;
                                confidence = 0.45;
                                evidence = Some(format!("Found AMSI-related symbol: {:?}", String::from_utf8_lossy(p)));
                                break;
                            }
                        }
                    }
                    "evasion-hook-003" => {
                        let patterns: &[&[u8]] = &[b"VirtualProtect", b"NtProtectVirtualMemory"];
                        let mut found = 0;
                        for p in patterns {
                            if bytes.windows(p.len()).any(|w| w == *p) { found += 1; }
                        }
                        if found > 0 {
                            detected = true;
                            confidence = 0.25;
                            evidence = Some(format!("Found {} memory protection APIs (potential unhooking)", found));
                        }
                    }
                    _ => {}
                }
            }
        }

        EvasionDetection {
            technique: technique.clone(),
            detected,
            confidence,
            evidence,
            recommendations: if detected {
                vec![
                    "Enable kernel-level callback monitoring".to_string(),
                    "Implement integrity checks on security DLL hooks".to_string(),
                ]
            } else {
                vec!["No bypass indicators found; verify with runtime analysis".to_string()]
            },
        }
    }

    async fn check_obfuscation(
        &self,
        technique: &EvasionTechnique,
        target: &EvasionTarget,
    ) -> EvasionDetection {
        let mut detected = false;
        let mut evidence = None;
        let mut confidence = 0.0;

        if let Some(path) = &target.path {
            if let Ok(bytes) = tokio::fs::read(path).await {
                match technique.id.as_str() {
                    "evasion-obf-001" => {
                        let mut suspicious = 0;
                        for window in bytes.windows(32) {
                            let xor_count = window.iter().filter(|&&b| b == window[0]).count();
                            if xor_count > 24 { suspicious += 1; }
                        }
                        if suspicious > 5 {
                            detected = true;
                            confidence = 0.35;
                            evidence = Some(format!("Found {} suspicious XOR-encoded patterns", suspicious));
                        }
                    }
                    "evasion-obf-002" => {
                        let nop_count = bytes.iter().filter(|&&b| b == 0x90).count();
                        let total = bytes.len();
                        if total > 0 && nop_count as f64 / total as f64 > 0.3 {
                            detected = true;
                            confidence = 0.3;
                            evidence = Some(format!("High NOP ratio: {:.1}% of binary is NOP instructions", nop_count as f64 / total as f64 * 100.0));
                        }
                    }
                    _ => {}
                }
            }
        }

        EvasionDetection {
            technique: technique.clone(),
            detected,
            confidence,
            evidence,
            recommendations: if detected {
                vec![
                    "Apply static analysis with deobfuscation tools".to_string(),
                    "Use dynamic analysis to observe decoded strings at runtime".to_string(),
                ]
            } else {
                vec!["No obfuscation indicators; may use advanced packing".to_string()]
            },
        }
    }

    async fn check_injection(
        &self,
        technique: &EvasionTechnique,
        target: &EvasionTarget,
    ) -> EvasionDetection {
        let mut detected = false;
        let mut evidence = None;
        let mut confidence = 0.0;

        match target.target_type {
            EvasionTargetType::Process => {
                if let Some(pid) = target.pid {
                    let _ = &pid; // used in #[cfg(target_os = "linux")] block
                    #[cfg(target_os = "linux")]
                    {
                        let maps_path = format!("/proc/{}/maps", pid);
                        if let Ok(maps) = tokio::fs::read_to_string(&maps_path).await {
                            match technique.id.as_str() {
                                "evasion-inj-001" => {
                                    let rwx_count = maps.lines().filter(|l| l.contains("rwxp") && !l.contains("/")).count();
                                    if rwx_count > 0 {
                                        detected = true;
                                        confidence = 0.5;
                                        evidence = Some(format!("Found {} anonymous RWX memory regions (potential hollowing)", rwx_count));
                                    }
                                }
                                "evasion-inj-002" => {
                                    let suspicious = maps.lines().filter(|l| l.contains("/tmp/") || l.contains("/dev/shm/")).count();
                                    if suspicious > 0 {
                                        detected = true;
                                        confidence = 0.55;
                                        evidence = Some(format!("Found {} libraries loaded from temp/shared memory paths", suspicious));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            EvasionTargetType::File => {
                if let Some(path) = &target.path {
                    if let Ok(bytes) = tokio::fs::read(path).await {
                        if technique.id == "evasion-inj-003" {
                            let patterns: &[&[u8]] = &[b"LoadLibraryA", b"GetProcAddress", b"VirtualAlloc"];
                            let mut found = 0;
                            for p in patterns {
                                if bytes.windows(p.len()).any(|w| w == *p) { found += 1; }
                            }
                            if found >= 2 {
                                detected = true;
                                confidence = 0.4;
                                evidence = Some(format!("Found {} reflective loading APIs (potential reflective DLL)", found));
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        EvasionDetection {
            technique: technique.clone(),
            detected,
            confidence,
            evidence,
            recommendations: if detected {
                vec![
                    "Enable process creation monitoring with memory scanning".to_string(),
                    "Implement DLL load auditing and path validation".to_string(),
                ]
            } else {
                vec!["No injection indicators; requires memory forensics for full coverage".to_string()]
            },
        }
    }

    async fn check_anti_analysis(
        &self,
        technique: &EvasionTechnique,
        target: &EvasionTarget,
    ) -> EvasionDetection {
        let mut detected = false;
        let mut evidence = None;
        let mut confidence = 0.0;

        if let Some(path) = &target.path {
            if let Ok(bytes) = tokio::fs::read(path).await {
                match technique.id.as_str() {
                    "evasion-anti-001" => {
                        let patterns: &[&[u8]] = &[b"VMware", b"VirtualBox", b"VBOX", b"QEMU", b"Xen", b"Hyper-V", b"svm", b"kvm"];
                        let mut found = 0;
                        for p in patterns {
                            if bytes.windows(p.len()).any(|w| w == *p) { found += 1; }
                        }
                        if found > 0 {
                            detected = true;
                            confidence = 0.3 + (found as f64 * 0.1).min(0.4);
                            evidence = Some(format!("Found {} VM-related strings (potential VM detection)", found));
                        }
                    }
                    "evasion-anti-002" => {
                        let patterns: &[&[u8]] = &[b"IsDebuggerPresent", b"CheckRemoteDebuggerPresent", b"NtQueryInformationProcess", b"OutputDebugString"];
                        let mut found = 0;
                        for p in patterns {
                            if bytes.windows(p.len()).any(|w| w == *p) { found += 1; }
                        }
                        if found > 0 {
                            detected = true;
                            confidence = 0.35;
                            evidence = Some(format!("Found {} debugger detection APIs", found));
                        }
                    }
                    "evasion-anti-003" => {
                        let patterns: &[&[u8]] = &[b"SleepEx", b"QueryPerformanceCounter", b"rdtsc", b"rdtscp"];
                        let mut found = 0;
                        for p in patterns {
                            if bytes.windows(p.len()).any(|w| w == *p) { found += 1; }
                        }
                        if found > 1 {
                            detected = true;
                            confidence = 0.25;
                            evidence = Some(format!("Found {} timing-related APIs (potential timing evasion)", found));
                        }
                    }
                    _ => {}
                }
            }
        }

        EvasionDetection {
            technique: technique.clone(),
            detected,
            confidence,
            evidence,
            recommendations: if detected {
                vec![
                    "Implement anti-VM-detection countermeasures in analysis environment".to_string(),
                    "Use hardware-level debugging to avoid detection".to_string(),
                ]
            } else {
                vec!["No anti-analysis indicators; may use runtime-only checks".to_string()]
            },
        }
    }

    async fn check_traffic_obfuscation(
        &self,
        technique: &EvasionTechnique,
        _target: &EvasionTarget,
    ) -> EvasionDetection {
        let (detected, confidence, evidence) = match technique.id.as_str() {
            "evasion-traffic-001" => (false, 0.0, Some("Domain fronting detection requires TLS SNI inspection (use proxy interception)".to_string())),
            "evasion-traffic-002" => (false, 0.0, Some("DoH tunneling detection requires DNS traffic capture and analysis".to_string())),
            "evasion-traffic-003" => (false, 0.0, Some("Jittered beacon detection requires extended network flow monitoring".to_string())),
            _ => (false, 0.0, None),
        };

        EvasionDetection {
            technique: technique.clone(),
            detected,
            confidence,
            evidence,
            recommendations: if detected {
                vec![
                    "Implement JA3/JA3S fingerprinting for C2 detection".to_string(),
                    "Monitor for anomalous DNS query patterns".to_string(),
                ]
            } else {
                vec![
                    "Use network proxy interception for traffic analysis".to_string(),
                    "Deploy network flow monitoring with timing analysis".to_string(),
                ]
            },
        }
    }

    pub fn techniques(&self) -> &[EvasionTechnique] {
        &self.techniques
    }
}

fn evasion_category_for(category: &EvasionCategory) -> String {
    match category {
        EvasionCategory::Syscall => "evasion-syscall".to_string(),
        EvasionCategory::HookBypass => "evasion-hook-bypass".to_string(),
        EvasionCategory::Obfuscation => "evasion-obfuscation".to_string(),
        EvasionCategory::Injection => "evasion-injection".to_string(),
        EvasionCategory::AntiAnalysis => "evasion-anti-analysis".to_string(),
        EvasionCategory::TrafficObfuscation => "evasion-traffic-obfuscation".to_string(),
    }
}

pub fn to_scan_report_data(report: &EvasionReport) -> crate::output::convert::ScanReportData {
    use crate::output::convert::FindingData;

    let findings: Vec<FindingData> = report
        .detections
        .iter()
        .filter(|d| d.detected)
        .map(|d| FindingData {
            title: d.technique.name.clone(),
            severity: d.technique.risk_level.to_severity().as_str().to_string(),
            category: evasion_category_for(&d.technique.category),
            description: format!("{} (confidence: {:.0}%)", d.technique.description, d.confidence * 100.0),
            location: report.target.clone(),
            evidence: d.evidence.clone(),
            remediation: d.recommendations.first().cloned(),
            cwe_ids: Vec::new(),
        })
        .collect();

    crate::output::convert::ScanReportData {
        target: report.target.clone(),
        scan_type: "evasion".to_string(),
        timestamp: report.timestamp.clone(),
        findings,
        open_ports: Vec::new(),
        services: Vec::new(),
        duration_ms: 0,
        wireless_networks: Vec::new(),
        policy_summary: None,
    }
}

pub async fn run_cli(
    args: crate::cli::EvasionArgs,
    _config: &crate::config::EggsecConfig,
) -> Result<()> {
    let target_type = args.parsed_target_type();
    let target = EvasionTarget {
        target_type,
        path: args.target.clone(),
        pid: args.pid,
    };
    let scanner = EvasionScanner::new(args.dry_run);

    if !args.quiet {
        if args.dry_run {
            eprintln!("DRY-RUN: planning mode (no real detection checks performed).");
        } else {
            eprintln!("NOTE: Defense-lab only. Performing real evasion detection checks.");
        }
        eprintln!("Scanning {} techniques against target...", scanner.techniques().len());
    }

    let report = scanner.scan(&target).await?;

    let output = if args.json {
        serde_json::to_string_pretty(&report)?
    } else {
        let mut buf = String::new();
        if report.dry_run {
            buf.push_str("DRY-RUN: no real detection checks performed\n\n");
        }
        buf.push_str(&format!("Evasion Detection Report - Target: {}\n", report.target));
        buf.push_str(&format!("Techniques checked: {}\n", report.summary.total_techniques));
        buf.push_str(&format!("Detected: {} | Not detected: {} | Rate: {:.0}%\n\n",
            report.summary.detected, report.summary.not_detected, report.summary.detection_rate * 100.0));

        for d in &report.detections {
            if d.detected {
                buf.push_str(&format!("  [{}] {} (confidence: {:.0}%)\n",
                    d.technique.risk_level.as_str(), d.technique.name, d.confidence * 100.0));
                if let Some(ref evidence) = d.evidence {
                    buf.push_str(&format!("    Evidence: {}\n", evidence));
                }
                for rec in &d.recommendations {
                    buf.push_str(&format!("    -> {}\n", rec));
                }
                buf.push('\n');
            }
        }

        let not_detected: Vec<_> = report.detections.iter().filter(|d| !d.detected).collect();
        if !not_detected.is_empty() {
            buf.push_str("Not detected:\n");
            for d in &not_detected {
                buf.push_str(&format!("  - {} ({})\n", d.technique.name, d.technique.id));
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evasion_risk_as_str() {
        assert_eq!(EvasionRisk::Low.as_str(), "low");
        assert_eq!(EvasionRisk::Medium.as_str(), "medium");
        assert_eq!(EvasionRisk::High.as_str(), "high");
        assert_eq!(EvasionRisk::Critical.as_str(), "critical");
    }

    #[test]
    fn test_evasion_risk_to_severity() {
        assert_eq!(EvasionRisk::Low.to_severity(), Severity::Low);
        assert_eq!(EvasionRisk::Medium.to_severity(), Severity::Medium);
        assert_eq!(EvasionRisk::High.to_severity(), Severity::High);
        assert_eq!(EvasionRisk::Critical.to_severity(), Severity::Critical);
    }

    #[test]
    fn test_evasion_scanner_creation() {
        let scanner = EvasionScanner::new(true);
        assert!(scanner.dry_run);
        assert_eq!(scanner.techniques().len(), 16);
        let scanner2 = EvasionScanner::new(false);
        assert!(!scanner2.dry_run);
    }

    #[test]
    fn test_default_techniques_categories() {
        let techniques = EvasionScanner::default_techniques();
        let mut categories: std::collections::HashSet<EvasionCategory> = std::collections::HashSet::new();
        for t in &techniques { categories.insert(t.category); }
        assert_eq!(categories.len(), 6);
        assert!(categories.contains(&EvasionCategory::Syscall));
        assert!(categories.contains(&EvasionCategory::HookBypass));
        assert!(categories.contains(&EvasionCategory::Obfuscation));
        assert!(categories.contains(&EvasionCategory::Injection));
        assert!(categories.contains(&EvasionCategory::AntiAnalysis));
        assert!(categories.contains(&EvasionCategory::TrafficObfuscation));
    }

    #[test]
    fn test_techniques_have_mitre_ids() {
        let techniques = EvasionScanner::default_techniques();
        for t in &techniques {
            assert!(t.mitre_id.is_some(), "Technique {} should have a MITRE ID", t.id);
        }
    }

    #[test]
    fn test_techniques_have_unique_ids() {
        let techniques = EvasionScanner::default_techniques();
        let mut ids: Vec<&str> = techniques.iter().map(|t| t.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), techniques.len(), "All technique IDs should be unique");
    }

    #[tokio::test]
    async fn test_dry_run_scan_produces_all_detected() {
        let scanner = EvasionScanner::new(true);
        let target = EvasionTarget { target_type: EvasionTargetType::Process, path: None, pid: None };
        let report = scanner.scan(&target).await.unwrap();
        assert!(report.dry_run);
        assert_eq!(report.detections.len(), 16);
        assert!(report.detections.iter().all(|d| d.detected));
        assert_eq!(report.summary.total_techniques, 16);
        assert_eq!(report.summary.detected, 16);
        assert_eq!(report.summary.not_detected, 0);
        assert!((report.summary.detection_rate - 1.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_dry_run_confidence_by_risk_level() {
        let scanner = EvasionScanner::new(true);
        let target = EvasionTarget { target_type: EvasionTargetType::File, path: None, pid: None };
        let report = scanner.scan(&target).await.unwrap();
        for d in &report.detections {
            let expected = match d.technique.risk_level {
                EvasionRisk::Critical => 0.85,
                EvasionRisk::High => 0.75,
                EvasionRisk::Medium => 0.65,
                EvasionRisk::Low => 0.55,
            };
            assert!((d.confidence - expected).abs() < f64::EPSILON,
                "Confidence mismatch for {}: got {}, expected {}", d.technique.id, d.confidence, expected);
        }
    }

    #[tokio::test]
    async fn test_real_scan_nonexistent_target() {
        let scanner = EvasionScanner::new(false);
        let target = EvasionTarget { target_type: EvasionTargetType::Process, path: Some("/nonexistent/binary".to_string()), pid: None };
        let report = scanner.scan(&target).await.unwrap();
        assert!(!report.dry_run);
        let detected_count = report.detections.iter().filter(|d| d.detected).count();
        assert!(detected_count < report.detections.len(), "Non-existent target should have some not-detected results");
    }

    #[tokio::test]
    async fn test_scan_with_target_path() {
        let scanner = EvasionScanner::new(true);
        let target = EvasionTarget { target_type: EvasionTargetType::File, path: Some("/usr/bin/ls".to_string()), pid: None };
        let report = scanner.scan(&target).await.unwrap();
        assert!(report.target.contains("file:/usr/bin/ls"));
    }

    #[tokio::test]
    async fn test_scan_with_pid() {
        let scanner = EvasionScanner::new(true);
        let target = EvasionTarget { target_type: EvasionTargetType::Process, path: None, pid: Some(1234) };
        let report = scanner.scan(&target).await.unwrap();
        assert!(report.target.contains("pid=1234"));
    }

    #[test]
    fn test_evasion_category_for_mapping() {
        assert_eq!(evasion_category_for(&EvasionCategory::Syscall), "evasion-syscall");
        assert_eq!(evasion_category_for(&EvasionCategory::HookBypass), "evasion-hook-bypass");
        assert_eq!(evasion_category_for(&EvasionCategory::Obfuscation), "evasion-obfuscation");
        assert_eq!(evasion_category_for(&EvasionCategory::Injection), "evasion-injection");
        assert_eq!(evasion_category_for(&EvasionCategory::AntiAnalysis), "evasion-anti-analysis");
        assert_eq!(evasion_category_for(&EvasionCategory::TrafficObfuscation), "evasion-traffic-obfuscation");
    }

    #[test]
    fn test_to_scan_report_data_bridge() {
        let report = EvasionReport {
            target: "test-target".to_string(),
            detections: vec![
                EvasionDetection {
                    technique: EvasionTechnique {
                        id: "evasion-syscall-001".to_string(), name: "Direct Syscall Detection".to_string(),
                        mitre_id: Some("T1106".to_string()), category: EvasionCategory::Syscall,
                        risk_level: EvasionRisk::High, description: "Test detection".to_string(),
                    },
                    detected: true, confidence: 0.75, evidence: Some("test evidence".to_string()),
                    recommendations: vec!["test rec".to_string()],
                },
                EvasionDetection {
                    technique: EvasionTechnique {
                        id: "evasion-anti-001".to_string(), name: "VM Detection".to_string(),
                        mitre_id: Some("T1497.001".to_string()), category: EvasionCategory::AntiAnalysis,
                        risk_level: EvasionRisk::Medium, description: "Not detected".to_string(),
                    },
                    detected: false, confidence: 0.0, evidence: None,
                    recommendations: vec!["none".to_string()],
                },
            ],
            summary: EvasionSummary { total_techniques: 2, detected: 1, not_detected: 1, detection_rate: 0.5 },
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            dry_run: true,
        };
        let bridge = to_scan_report_data(&report);
        assert_eq!(bridge.target, "test-target");
        assert_eq!(bridge.scan_type, "evasion");
        assert_eq!(bridge.findings.len(), 1);
        assert_eq!(bridge.findings[0].title, "Direct Syscall Detection");
        assert_eq!(bridge.findings[0].severity, "high");
        assert_eq!(bridge.findings[0].category, "evasion-syscall");
    }

    #[test]
    fn test_to_scan_report_data_empty() {
        let report = EvasionReport {
            target: "empty".to_string(), detections: Vec::new(),
            summary: EvasionSummary { total_techniques: 0, detected: 0, not_detected: 0, detection_rate: 0.0 },
            timestamp: "2024-01-01T00:00:00Z".to_string(), dry_run: true,
        };
        let bridge = to_scan_report_data(&report);
        assert_eq!(bridge.findings.len(), 0);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let report = EvasionReport {
            target: "test".to_string(), detections: Vec::new(),
            summary: EvasionSummary { total_techniques: 0, detected: 0, not_detected: 0, detection_rate: 0.0 },
            timestamp: "2024-01-01T00:00:00Z".to_string(), dry_run: true,
        };
        let json = serde_json::to_string(&report).unwrap();
        let deserialized: EvasionReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.target, report.target);
        assert_eq!(deserialized.dry_run, report.dry_run);
    }

    #[test]
    fn test_evasion_target_type_serialization() {
        let json = serde_json::to_string(&EvasionTargetType::Process).unwrap();
        assert_eq!(json, "\"process\"");
        let json = serde_json::to_string(&EvasionTargetType::Network).unwrap();
        assert_eq!(json, "\"network\"");
    }

    #[test]
    fn test_evasion_category_serialization() {
        let json = serde_json::to_string(&EvasionCategory::HookBypass).unwrap();
        assert_eq!(json, "\"hook_bypass\"");
        let json = serde_json::to_string(&EvasionCategory::TrafficObfuscation).unwrap();
        assert_eq!(json, "\"traffic_obfuscation\"");
    }

    #[test]
    fn test_evasion_risk_serialization() {
        let json = serde_json::to_string(&EvasionRisk::Critical).unwrap();
        assert_eq!(json, "\"critical\"");
    }
}
