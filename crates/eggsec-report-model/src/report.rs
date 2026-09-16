//! Stable serializable scan-report DTOs.
//!
//! These types moved verbatim from `eggsec-output::convert` (Phase B). The
//! model owns the data only: filesystem loading (`load_scan_report`),
//! JUnit/SARIF/HTML/Markdown/CSV conversion, and renderer adapters stay in
//! `eggsec-output`, which implements them over these types.

use serde::{Deserialize, Serialize};

/// Serializable scan report exchanged by domain producers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanReportData {
    pub target: String,
    pub scan_type: String,
    pub timestamp: String,
    pub findings: Vec<FindingData>,
    pub open_ports: Vec<PortData>,
    pub services: Vec<ServiceData>,
    pub duration_ms: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wireless_networks: Vec<WirelessNetworkReportData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_summary: Option<crate::PolicySummary>,
}

/// Serializable finding exchanged by domain producers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingData {
    pub title: String,
    pub severity: String,
    pub category: String,
    pub description: String,
    pub location: String,
    pub evidence: Option<String>,
    pub remediation: Option<String>,
    #[serde(alias = "cve_ids")]
    pub cwe_ids: Vec<String>,
}

/// Serializable open-port record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortData {
    pub port: u16,
    pub status: String,
    pub protocol: Option<String>,
    pub state: Option<String>,
    pub service: Option<String>,
    pub version: Option<String>,
    pub banner: Option<String>,
}

/// Serializable detected-service record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceData {
    pub service: String,
    pub version: Option<String>,
    pub banner: Option<String>,
}

/// Serializable wireless-network report record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WirelessNetworkReportData {
    pub ssid: String,
    pub bssid: String,
    pub channel: u8,
    pub security_type: String,
    pub signal_strength: i32,
    pub last_seen: String,
    #[serde(default)]
    pub wps_enabled: bool,
    #[serde(default)]
    pub is_hidden: bool,
    #[serde(default)]
    pub transition_mode: bool,
}
