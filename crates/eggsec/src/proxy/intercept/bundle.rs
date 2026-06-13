//! Evidence bundle v2 for the web proxy module.
//!
//! Packages a complete interactive web proxy session into a compressed JSON archive
//! suitable for reporting, cross-loadout correlation, and forensic review.
//! Uses flate2 gzipped JSON (no tar dependency).

use crate::error::{EggsecError, Result};
use crate::proxy::intercept::correlation::CorrelationReference;
use crate::proxy::intercept::rules::EnhancedRuleSet;
use crate::proxy::intercept::types::WebProxySessionReport;
use serde::{Deserialize, Serialize};
use std::io::Write;

/// Top-level evidence bundle structure.
///
/// Contains all session data in a single serializable object that gets
/// compressed into a `.json.gz` archive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceBundle {
    /// Bundle format version.
    pub version: String,
    /// Manifest with session metadata.
    pub manifest: BundleManifest,
    /// Proxy flows.
    pub flows: Vec<crate::proxy::intercept::types::ProxyFlow>,
    /// WebSocket sessions.
    pub ws_sessions: Vec<crate::proxy::intercept::protocols::WebSocketSession>,
    /// HTTP/2 sessions.
    pub http2_sessions: Vec<crate::proxy::intercept::protocols::Http2Session>,
    /// gRPC sessions.
    pub grpc_sessions: Vec<crate::proxy::intercept::protocols::GrpcSession>,
    /// Enhanced rule set snapshot.
    pub rules: EnhancedRuleSet,
    /// Manipulation audit trail.
    pub manipulations: Vec<crate::proxy::intercept::types::ManipulationRecord>,
    /// Cross-loadout correlation references.
    pub correlations: Vec<CorrelationReference>,
}

/// Session metadata for the bundle manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    /// Target address (listen address).
    pub target: String,
    /// Scope description (if any).
    pub scope: Option<String>,
    /// Session start timestamp (RFC 3339).
    pub started_at: String,
    /// Session end timestamp (RFC 3339).
    pub ended_at: String,
    /// Operator/user identifier.
    pub user: Option<String>,
    /// Whether this was a dry-run session.
    pub dry_run: bool,
    /// Total flows captured.
    pub flow_count: usize,
    /// Total WebSocket sessions.
    pub ws_session_count: usize,
    /// Total HTTP/2 sessions.
    pub http2_session_count: usize,
    /// Total gRPC sessions.
    pub grpc_session_count: usize,
    /// Total manipulations performed.
    pub manipulation_count: usize,
    /// Total correlation references.
    pub correlation_count: usize,
    /// Number of rules in the rule set.
    pub rule_count: usize,
}

impl EvidenceBundle {
    /// Build an `EvidenceBundle` from a session report and optional rule set.
    pub fn from_report(
        report: &WebProxySessionReport,
        rules: Option<&EnhancedRuleSet>,
    ) -> Self {
        let rule_set = rules.cloned().unwrap_or_default();
        let correlations: Vec<CorrelationReference> = report
            .correlation_refs
            .iter()
            .cloned()
            .collect();

        let manifest = BundleManifest {
            target: report.listen_addr.clone(),
            scope: None,
            started_at: report.started_at.clone(),
            ended_at: report.ended_at.clone(),
            user: None,
            dry_run: report.dry_run,
            flow_count: report.flows.len(),
            ws_session_count: report.ws_sessions.len(),
            http2_session_count: report.http2_sessions.len(),
            grpc_session_count: report.grpc_sessions.len(),
            manipulation_count: report.manipulations.len(),
            correlation_count: correlations.len(),
            rule_count: rule_set.len(),
        };

        Self {
            version: "2".to_string(),
            manifest,
            flows: report.flows.clone(),
            ws_sessions: report.ws_sessions.clone(),
            http2_sessions: report.http2_sessions.clone(),
            grpc_sessions: report.grpc_sessions.clone(),
            rules: rule_set,
            manipulations: report.manipulations.clone(),
            correlations,
        }
    }

    /// Serialize the bundle to compressed JSON bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let json = serde_json::to_vec_pretty(self)
            .map_err(|e| EggsecError::Proxy(format!("Failed to serialize evidence bundle: {}", e)))?;
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder
            .write_all(&json)
            .map_err(|e| EggsecError::Proxy(format!("Failed to compress evidence bundle: {}", e)))?;
        encoder
            .finish()
            .map_err(|e| EggsecError::Proxy(format!("Failed to finish gzip stream: {}", e)))
    }

    /// Deserialize a bundle from compressed JSON bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        use std::io::Read;
        let mut decoder = flate2::read::GzDecoder::new(data);
        let mut json = String::new();
        decoder
            .read_to_string(&mut json)
            .map_err(|e| EggsecError::Proxy(format!("Failed to decompress evidence bundle: {}", e)))?;
        serde_json::from_str(&json)
            .map_err(|e| EggsecError::Proxy(format!("Failed to deserialize evidence bundle: {}", e)))
    }

    /// Reconstruct a `WebProxySessionReport` from this bundle.
    pub fn to_session_report(&self) -> WebProxySessionReport {
        WebProxySessionReport {
            listen_addr: self.manifest.target.clone(),
            ca_fingerprint: String::new(),
            dry_run: self.manifest.dry_run,
            flows: self.flows.clone(),
            budget: Default::default(),
            policy_decision: None,
            actions_performed: Vec::new(),
            manifest_matched: false,
            started_at: self.manifest.started_at.clone(),
            ended_at: self.manifest.ended_at.clone(),
            duration_ms: 0,
            https_intercepted: self.flows.iter().filter(|f| f.is_https).count() as u64,
            http_logged: self.flows.iter().filter(|f| !f.is_https).count() as u64,
            blocked: 0,
            redacted: 0,
            errors: Vec::new(),
            manipulations: self.manipulations.clone(),
            ws_sessions: self.ws_sessions.clone(),
            http2_sessions: self.http2_sessions.clone(),
            grpc_sessions: self.grpc_sessions.clone(),
            correlation: None,
            correlation_refs: self.correlations.clone(),
        }
    }
}

/// Export an evidence bundle from a session report to a file path.
///
/// Writes a gzipped JSON file and returns the path on success.
pub fn export_evidence_bundle(
    report: &WebProxySessionReport,
    rules: Option<&EnhancedRuleSet>,
    bundle_path: &str,
) -> Result<String> {
    let bundle = EvidenceBundle::from_report(report, rules);
    let bytes = bundle.to_bytes()?;
    let mut file = std::fs::File::create(bundle_path)
        .map_err(|e| EggsecError::Proxy(format!("Failed to create bundle file {}: {}", bundle_path, e)))?;
    file.write_all(&bytes)
        .map_err(|e| EggsecError::Proxy(format!("Failed to write bundle file: {}", e)))?;
    Ok(bundle_path.to_string())
}

/// Import an evidence bundle from a gzipped JSON file.
pub fn import_evidence_bundle(bundle_path: &str) -> Result<EvidenceBundle> {
    let data = std::fs::read(bundle_path)
        .map_err(|e| EggsecError::Proxy(format!("Failed to read bundle file {}: {}", bundle_path, e)))?;
    EvidenceBundle::from_bytes(&data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::intercept::correlation::{CorrelationReference, CorrelationSource};
    use crate::proxy::intercept::rules::{EnhancedRule, RuleAction, RuleCondition};
    use crate::proxy::intercept::types::*;
    use std::collections::HashMap;

    fn sample_report() -> WebProxySessionReport {
        let mut report = WebProxySessionReport::new("127.0.0.1:8080", false);
        report.ca_fingerprint = "aa:bb:cc".to_string();
        report.flows.push(ProxyFlow {
            index: 0,
            method: "GET".to_string(),
            url: "https://example.com/".to_string(),
            host: "example.com".to_string(),
            path: "/".to_string(),
            request_headers: HashMap::new(),
            request_body: None,
            response_status: 200,
            response_headers: HashMap::new(),
            response_body: None,
            is_https: true,
            duration_ms: 150,
            request_body_size: 0,
            response_body_size: 1024,
            started_at: chrono::Utc::now().to_rfc3339(),
            completed_at: chrono::Utc::now().to_rfc3339(),
            redaction_applied: None,
            protocol: "http1".to_string(),
        });
        report.manipulations.push(ManipulationRecord {
            flow_index: 0,
            direction: ProxyFlowDirection::Request,
            field: "header:Authorization".to_string(),
            before: Some("Bearer old".to_string()),
            after: Some("Bearer new".to_string()),
            reason: "Token refresh".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        report.correlation_refs.push(CorrelationReference::new(
            CorrelationSource::DbPentest,
            "db-finding-1",
            "SQLi in proxy-modified request",
        ));
        report
    }

    fn sample_rules() -> EnhancedRuleSet {
        let mut rules = EnhancedRuleSet::new();
        rules.add(EnhancedRule::new(
            "r1",
            "Block evil",
            RuleCondition::HostMatches("evil.com".to_string()),
            RuleAction::Block,
        ));
        rules
    }

    #[test]
    fn test_evidence_bundle_roundtrip() {
        let report = sample_report();
        let rules = sample_rules();
        let bundle = EvidenceBundle::from_report(&report, Some(&rules));

        assert_eq!(bundle.version, "2");
        assert_eq!(bundle.manifest.target, "127.0.0.1:8080");
        assert_eq!(bundle.manifest.flow_count, 1);
        assert_eq!(bundle.manifest.manipulation_count, 1);
        assert_eq!(bundle.manifest.correlation_count, 1);
        assert_eq!(bundle.manifest.rule_count, 1);

        let bytes = bundle.to_bytes().expect("serialize");
        let restored = EvidenceBundle::from_bytes(&bytes).expect("deserialize");

        assert_eq!(restored.version, "2");
        assert_eq!(restored.manifest.flow_count, 1);
        assert_eq!(restored.manifest.manipulation_count, 1);
        assert_eq!(restored.manifest.correlation_count, 1);
        assert_eq!(restored.manifest.rule_count, 1);
        assert_eq!(restored.flows[0].host, "example.com");
        assert_eq!(restored.manipulations[0].field, "header:Authorization");
        assert_eq!(restored.correlations[0].finding_id, "db-finding-1");
    }

    #[test]
    fn test_evidence_bundle_to_session_report() {
        let report = sample_report();
        let rules = sample_rules();
        let bundle = EvidenceBundle::from_report(&report, Some(&rules));
        let restored = bundle.to_session_report();

        assert_eq!(restored.listen_addr, "127.0.0.1:8080");
        assert_eq!(restored.flows.len(), 1);
        assert_eq!(restored.manipulations.len(), 1);
        assert_eq!(restored.correlation_refs.len(), 1);
        assert_eq!(restored.https_intercepted, 1);
    }

    #[test]
    fn test_export_import_roundtrip() {
        let report = sample_report();
        let rules = sample_rules();
        let dir = std::env::temp_dir().join("eggsec_proxy_bundle_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_bundle.json.gz");

        let path_str = path.to_str().unwrap();
        export_evidence_bundle(&report, Some(&rules), path_str).expect("export");

        let imported = import_evidence_bundle(path_str).expect("import");
        assert_eq!(imported.manifest.target, "127.0.0.1:8080");
        assert_eq!(imported.flows.len(), 1);
        assert_eq!(imported.manipulations.len(), 1);
        assert_eq!(imported.correlations.len(), 1);
        assert_eq!(imported.rules.len(), 1);

        let _ = std::fs::remove_file(path_str);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn test_evidence_bundle_empty_report() {
        let report = WebProxySessionReport::new("0.0.0.0:9090", true);
        let bundle = EvidenceBundle::from_report(&report, None);

        assert_eq!(bundle.manifest.flow_count, 0);
        assert_eq!(bundle.manifest.rule_count, 0);
        assert!(bundle.flows.is_empty());
        assert!(bundle.rules.is_empty());

        let bytes = bundle.to_bytes().expect("serialize empty");
        let restored = EvidenceBundle::from_bytes(&bytes).expect("deserialize empty");
        assert_eq!(restored.manifest.flow_count, 0);
    }

    #[test]
    fn test_evidence_bundle_manifest_fields() {
        let report = sample_report();
        let bundle = EvidenceBundle::from_report(&report, None);

        assert_eq!(bundle.manifest.target, "127.0.0.1:8080");
        assert!(!bundle.manifest.dry_run);
        assert!(bundle.manifest.scope.is_none());
        assert!(bundle.manifest.user.is_none());
    }

    #[test]
    fn test_evidence_bundle_ws_http2_grpc_counts() {
        let mut report = sample_report();
        report.ws_sessions.push(
            crate::proxy::intercept::protocols::WebSocketSession::new(
                "wss://example.com/ws",
                "example.com",
                "/ws",
                true,
            ),
        );
        let mut h2 =
            crate::proxy::intercept::protocols::Http2Session::new("example.com", true);
        h2.add_stream(crate::proxy::intercept::protocols::Http2Stream::new(
            1, "GET", "/data",
        ));
        report.http2_sessions.push(h2);
        let mut grpc =
            crate::proxy::intercept::protocols::GrpcSession::new("example.com", true);
        grpc.add_call(crate::proxy::intercept::protocols::GrpcCall::new(
            "/pkg.Svc/Method",
            crate::proxy::intercept::protocols::GrpcMethodType::Unary,
        ));
        report.grpc_sessions.push(grpc);

        let bundle = EvidenceBundle::from_report(&report, None);
        assert_eq!(bundle.manifest.ws_session_count, 1);
        assert_eq!(bundle.manifest.http2_session_count, 1);
        assert_eq!(bundle.manifest.grpc_session_count, 1);

        let bytes = bundle.to_bytes().expect("serialize");
        let restored = EvidenceBundle::from_bytes(&bytes).expect("deserialize");
        assert_eq!(restored.ws_sessions.len(), 1);
        assert_eq!(restored.http2_sessions.len(), 1);
        assert_eq!(restored.grpc_sessions.len(), 1);
    }
}
