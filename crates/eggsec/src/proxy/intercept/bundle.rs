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
    /// HMAC-SHA256 signature for bundle integrity (hex-encoded).
    /// Computed over the serialized manifest fields (excluding this signature).
    #[serde(default)]
    pub signature: Option<String>,
    /// Timestamp when the bundle was signed (RFC 3339).
    #[serde(default)]
    pub signed_at: Option<String>,
    /// Signing key identifier (for key rotation tracking).
    #[serde(default)]
    pub signing_key_id: Option<String>,
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
            .to_vec();

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
            signature: None,
            signed_at: None,
            signing_key_id: None,
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

    /// Sign the bundle manifest with HMAC-SHA256 for integrity verification.
    ///
    /// The signature is computed over a canonical representation of the manifest fields.
    /// After signing, the `signature`, `signed_at`, and `signing_key_id` fields are set.
    ///
    /// # Arguments
    /// * `key` - HMAC signing key (bytes)
    /// * `key_id` - Optional key identifier for tracking
    pub fn sign(&mut self, key: &[u8], key_id: Option<&str>) -> Result<()> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let canonical = self.manifest_canonical_string();
        let mut mac = HmacSha256::new_from_slice(key)
            .map_err(|e| EggsecError::Proxy(format!("HMAC key error: {}", e)))?;
        mac.update(canonical.as_bytes());
        let signature = mac.finalize().into_bytes();

        self.manifest.signature = Some(hex::encode(signature));
        self.manifest.signed_at = Some(chrono::Utc::now().to_rfc3339());
        self.manifest.signing_key_id = key_id.map(|s| s.to_string());

        Ok(())
    }

    /// Verify the bundle signature with HMAC-SHA256.
    ///
    /// Returns `Ok(true)` if the signature is valid, `Ok(false)` if invalid,
    /// or `Err` if the bundle is unsigned or verification fails.
    pub fn verify(&self, key: &[u8]) -> Result<bool> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let provided_sig = self
            .manifest
            .signature
            .as_deref()
            .ok_or_else(|| EggsecError::Proxy("Bundle is unsigned".to_string()))?;

        let sig_bytes = hex::decode(provided_sig)
            .map_err(|e| EggsecError::Proxy(format!("Invalid signature hex: {}", e)))?;

        let canonical = self.manifest_canonical_string();
        let mut mac = HmacSha256::new_from_slice(key)
            .map_err(|e| EggsecError::Proxy(format!("HMAC key error: {}", e)))?;
        mac.update(canonical.as_bytes());

        Ok(mac.verify_slice(&sig_bytes).is_ok())
    }

    /// Generate a canonical string representation of the manifest for signing.
    ///
    /// This ensures consistent signature computation regardless of serialization order.
    fn manifest_canonical_string(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.manifest.target,
            self.manifest.scope.as_deref().unwrap_or(""),
            self.manifest.started_at,
            self.manifest.ended_at,
            self.manifest.user.as_deref().unwrap_or(""),
            self.manifest.dry_run,
            self.manifest.flow_count,
            self.manifest.ws_session_count,
            self.manifest.http2_session_count,
            self.manifest.grpc_session_count,
            self.manifest.manipulation_count,
            self.manifest.correlation_count,
            self.manifest.rule_count,
        )
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

/// Export a signed evidence bundle from a session report to a file path.
///
/// Signs the bundle with HMAC-SHA256 before writing. The signature provides
/// integrity verification but not authenticity (symmetric key).
pub fn export_signed_evidence_bundle(
    report: &WebProxySessionReport,
    rules: Option<&EnhancedRuleSet>,
    bundle_path: &str,
    signing_key: &[u8],
    key_id: Option<&str>,
) -> Result<String> {
    let mut bundle = EvidenceBundle::from_report(report, rules);
    bundle.sign(signing_key, key_id)?;
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

/// Differences between two evidence bundles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleDiff {
    /// Flows present in `baseline` but not in `other`.
    pub flows_added: Vec<u64>,
    /// Flows present in `other` but not in `baseline`.
    pub flows_removed: Vec<u64>,
    /// Flows present in both but with different data.
    pub flows_modified: Vec<u64>,
    /// Manipulations present in `baseline` but not in `other`.
    pub manipulations_added: usize,
    /// Manipulations present in `other` but not in `baseline`.
    pub manipulations_removed: usize,
    /// Rules present in `baseline` but not in `other`.
    pub rules_added: usize,
    /// Rules present in `other` but not in `baseline`.
    pub rules_removed: usize,
    /// Correlations present in `baseline` but not in `other`.
    pub correlations_added: usize,
    /// Correlations present in `other` but not in `baseline`.
    pub correlations_removed: usize,
    /// Whether the manifests differ.
    pub manifest_changed: bool,
}

impl BundleDiff {
    /// Returns true if no differences were found.
    pub fn is_empty(&self) -> bool {
        self.flows_added.is_empty()
            && self.flows_removed.is_empty()
            && self.flows_modified.is_empty()
            && self.manipulations_added == 0
            && self.manipulations_removed == 0
            && self.rules_added == 0
            && self.rules_removed == 0
            && self.correlations_added == 0
            && self.correlations_removed == 0
            && !self.manifest_changed
    }

    /// Human-readable summary of the diff.
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if !self.flows_added.is_empty() {
            parts.push(format!("{} flows added", self.flows_added.len()));
        }
        if !self.flows_removed.is_empty() {
            parts.push(format!("{} flows removed", self.flows_removed.len()));
        }
        if !self.flows_modified.is_empty() {
            parts.push(format!("{} flows modified", self.flows_modified.len()));
        }
        if self.manipulations_added > 0 {
            parts.push(format!("{} manipulations added", self.manipulations_added));
        }
        if self.manipulations_removed > 0 {
            parts.push(format!("{} manipulations removed", self.manipulations_removed));
        }
        if self.rules_added > 0 {
            parts.push(format!("{} rules added", self.rules_added));
        }
        if self.rules_removed > 0 {
            parts.push(format!("{} rules removed", self.rules_removed));
        }
        if self.correlations_added > 0 {
            parts.push(format!("{} correlations added", self.correlations_added));
        }
        if self.correlations_removed > 0 {
            parts.push(format!("{} correlations removed", self.correlations_removed));
        }
        if self.manifest_changed {
            parts.push("manifest changed".to_string());
        }
        if parts.is_empty() {
            "No differences".to_string()
        } else {
            parts.join("; ")
        }
    }
}

/// Compare two evidence bundles and return the differences.
///
/// `baseline` is the older/reference bundle; `other` is the newer/compared bundle.
/// Flows are matched by their `index` field; manipulations, rules, and correlations
/// are compared by count.
pub fn compare_bundles(baseline: &EvidenceBundle, other: &EvidenceBundle) -> BundleDiff {
    use std::collections::HashSet;

    let baseline_flow_indices: HashSet<u64> = baseline.flows.iter().map(|f| f.index).collect();
    let other_flow_indices: HashSet<u64> = other.flows.iter().map(|f| f.index).collect();

    let flows_added: Vec<u64> = other_flow_indices
        .difference(&baseline_flow_indices)
        .copied()
        .collect();
    let flows_removed: Vec<u64> = baseline_flow_indices
        .difference(&other_flow_indices)
        .copied()
        .collect();

    // Check for modified flows (same index, different data)
    let mut flows_modified = Vec::new();
    for b_flow in &baseline.flows {
        if let Some(o_flow) = other.flows.iter().find(|f| f.index == b_flow.index) {
            let b_json = serde_json::to_string(b_flow).unwrap_or_default();
            let o_json = serde_json::to_string(o_flow).unwrap_or_default();
            if b_json != o_json {
                flows_modified.push(b_flow.index);
            }
        }
    }

    let manipulations_diff =
        baseline.manipulations.len() as i64 - other.manipulations.len() as i64;
    let manipulations_added = if manipulations_diff > 0 {
        manipulations_diff as usize
    } else {
        0
    };
    let manipulations_removed = if manipulations_diff < 0 {
        (-manipulations_diff) as usize
    } else {
        0
    };

    let rules_diff = baseline.rules.len() as i64 - other.rules.len() as i64;
    let rules_added = if rules_diff > 0 { rules_diff as usize } else { 0 };
    let rules_removed = if rules_diff < 0 { (-rules_diff) as usize } else { 0 };

    let corr_diff =
        baseline.correlations.len() as i64 - other.correlations.len() as i64;
    let correlations_added = if corr_diff > 0 { corr_diff as usize } else { 0 };
    let correlations_removed = if corr_diff < 0 { (-corr_diff) as usize } else { 0 };

    let manifest_changed = baseline.manifest.flow_count != other.manifest.flow_count
        || baseline.manifest.started_at != other.manifest.started_at
        || baseline.manifest.ended_at != other.manifest.ended_at;

    BundleDiff {
        flows_added,
        flows_removed,
        flows_modified,
        manipulations_added,
        manipulations_removed,
        rules_added,
        rules_removed,
        correlations_added,
        correlations_removed,
        manifest_changed,
    }
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

    #[test]
    fn test_compare_bundles_identical() {
        let report = sample_report();
        let rules = sample_rules();
        let bundle_a = EvidenceBundle::from_report(&report, Some(&rules));
        let bundle_b = EvidenceBundle::from_report(&report, Some(&rules));
        let diff = compare_bundles(&bundle_a, &bundle_b);
        assert!(diff.is_empty());
        assert_eq!(diff.summary(), "No differences");
    }

    #[test]
    fn test_compare_bundles_added_flow() {
        let report_a = sample_report();
        let mut report_b = sample_report();
        report_b.flows.push(ProxyFlow {
            index: 1,
            method: "POST".to_string(),
            url: "https://example.com/api".to_string(),
            host: "example.com".to_string(),
            path: "/api".to_string(),
            request_headers: std::collections::HashMap::new(),
            request_body: None,
            response_status: 201,
            response_headers: std::collections::HashMap::new(),
            response_body: None,
            is_https: true,
            duration_ms: 50,
            request_body_size: 0,
            response_body_size: 0,
            started_at: chrono::Utc::now().to_rfc3339(),
            completed_at: chrono::Utc::now().to_rfc3339(),
            redaction_applied: None,
            protocol: "http1".to_string(),
        });

        let bundle_a = EvidenceBundle::from_report(&report_a, None);
        let bundle_b = EvidenceBundle::from_report(&report_b, None);
        let diff = compare_bundles(&bundle_a, &bundle_b);

        assert_eq!(diff.flows_added, vec![1]);
        assert!(diff.flows_removed.is_empty());
        assert!(!diff.is_empty());
        assert!(diff.summary().contains("1 flows added"));
    }

    #[test]
    fn test_compare_bundles_removed_flow() {
        let mut report_a = sample_report();
        report_a.flows.push(ProxyFlow {
            index: 1,
            method: "POST".to_string(),
            url: "https://example.com/api".to_string(),
            host: "example.com".to_string(),
            path: "/api".to_string(),
            request_headers: std::collections::HashMap::new(),
            request_body: None,
            response_status: 201,
            response_headers: std::collections::HashMap::new(),
            response_body: None,
            is_https: true,
            duration_ms: 50,
            request_body_size: 0,
            response_body_size: 0,
            started_at: chrono::Utc::now().to_rfc3339(),
            completed_at: chrono::Utc::now().to_rfc3339(),
            redaction_applied: None,
            protocol: "http1".to_string(),
        });
        let report_b = sample_report();

        let bundle_a = EvidenceBundle::from_report(&report_a, None);
        let bundle_b = EvidenceBundle::from_report(&report_b, None);
        let diff = compare_bundles(&bundle_a, &bundle_b);

        assert!(diff.flows_added.is_empty());
        assert_eq!(diff.flows_removed, vec![1]);
        assert!(!diff.is_empty());
    }

    #[test]
    fn test_compare_bundles_modified_flow() {
        let report_a = sample_report();
        let mut report_b = sample_report();
        report_b.flows[0].response_status = 500;

        let bundle_a = EvidenceBundle::from_report(&report_a, None);
        let bundle_b = EvidenceBundle::from_report(&report_b, None);
        let diff = compare_bundles(&bundle_a, &bundle_b);

        assert!(diff.flows_added.is_empty());
        assert!(diff.flows_removed.is_empty());
        assert_eq!(diff.flows_modified, vec![0]);
        assert!(diff.summary().contains("1 flows modified"));
    }

    #[test]
    fn test_compare_bundles_diff_summary() {
        let report_a = sample_report();
        let mut report_b = sample_report();
        report_b.flows[0].response_status = 500;
        report_b.manipulations.push(ManipulationRecord {
            flow_index: 0,
            direction: ProxyFlowDirection::Request,
            field: "body".to_string(),
            before: None,
            after: Some("injected".to_string()),
            reason: "test".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        let bundle_a = EvidenceBundle::from_report(&report_a, None);
        let bundle_b = EvidenceBundle::from_report(&report_b, None);
        let diff = compare_bundles(&bundle_a, &bundle_b);
        let summary = diff.summary();

        assert!(summary.contains("1 flows modified"));
        assert!(summary.contains("1 manipulations removed"));
    }
}
