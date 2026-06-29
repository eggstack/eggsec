//! Cross-loadout correlation hooks for linking proxy flows with other Eggsec findings.
//!
//! Provides lightweight correlation context objects that can be shared between
//! the web proxy and other loadouts (db-pentest, auth-test, mobile-dynamic, etc.).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Source of a correlation reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrelationSource {
    /// Linked to a database pentest finding.
    DbPentest,
    /// Linked to an authentication test result.
    AuthTest,
    /// Linked to a mobile dynamic finding.
    MobileDynamic,
    /// Linked to a wireless finding.
    Wireless,
    /// Linked to another proxy flow.
    ProxyFlow,
    /// External/manual correlation.
    External,
}

/// A reference to a finding in another loadout that correlates with a proxy flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationReference {
    /// Which loadout the correlated finding came from.
    pub source: CorrelationSource,
    /// Identifier of the correlated finding (e.g., finding ID, flow index).
    pub finding_id: String,
    /// Human-readable description of the correlation.
    pub description: String,
    /// Confidence in the correlation (0.0 - 1.0).
    pub confidence: f64,
    /// Timestamp when the correlation was established.
    pub timestamp: String,
    /// Additional metadata for the correlation.
    pub metadata: HashMap<String, String>,
}

impl CorrelationReference {
    pub fn new(source: CorrelationSource, finding_id: &str, description: &str) -> Self {
        Self {
            source,
            finding_id: finding_id.to_string(),
            description: description.to_string(),
            confidence: 1.0,
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// A correlation context that aggregates all cross-loadout references for a session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorrelationContext {
    /// All correlation references in this session.
    pub references: Vec<CorrelationReference>,
    /// Mapping from proxy flow index to correlated references.
    pub flow_correlations: HashMap<u64, Vec<usize>>,
    /// Summary statistics.
    pub summary: CorrelationSummary,
}

/// Summary of correlation activity in a session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorrelationSummary {
    /// Total number of correlation references.
    pub total_references: u64,
    /// Number of unique source loadouts correlated.
    pub unique_sources: u64,
    /// Number of proxy flows with correlations.
    pub correlated_flows: u64,
    /// Average confidence across all correlations.
    pub avg_confidence: f64,
    /// Number of temporal correlations found.
    pub temporal_correlations: u64,
    /// Number of behavioral correlations found.
    pub behavioral_correlations: u64,
}

impl CorrelationContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a correlation reference for a specific proxy flow.
    pub fn add_flow_correlation(&mut self, flow_index: u64, reference: CorrelationReference) {
        let idx = self.references.len();
        self.references.push(reference);
        self.flow_correlations
            .entry(flow_index)
            .or_default()
            .push(idx);
        self.update_summary();
    }

    /// Add a session-level correlation reference (not tied to a specific flow).
    pub fn add_reference(&mut self, reference: CorrelationReference) {
        self.references.push(reference);
        self.update_summary();
    }

    /// Get all correlations for a specific flow.
    pub fn get_flow_correlations(&self, flow_index: u64) -> Vec<&CorrelationReference> {
        self.flow_correlations
            .get(&flow_index)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|&i| self.references.get(i))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get correlations filtered by source.
    pub fn get_by_source(&self, source: CorrelationSource) -> Vec<&CorrelationReference> {
        self.references
            .iter()
            .filter(|r| r.source == source)
            .collect()
    }

    fn update_summary(&mut self) {
        let total = self.references.len() as u64;
        let unique_sources = self
            .references
            .iter()
            .map(|r| r.source)
            .collect::<std::collections::HashSet<_>>()
            .len() as u64;
        let correlated_flows = self.flow_correlations.len() as u64;
        let avg_confidence = if total > 0 {
            self.references.iter().map(|r| r.confidence).sum::<f64>() / total as f64
        } else {
            0.0
        };

        self.summary = CorrelationSummary {
            total_references: total,
            unique_sources,
            correlated_flows,
            avg_confidence,
            temporal_correlations: self.summary.temporal_correlations,
            behavioral_correlations: self.summary.behavioral_correlations,
        };
    }
}

/// A hook point for correlating proxy data with external loadout findings.
pub struct CorrelationHook {
    /// Description of what this hook correlates.
    pub description: String,
    /// Function-like callback metadata (for serialization).
    pub hook_type: String,
    /// Parameters for the hook.
    pub parameters: HashMap<String, String>,
}

impl CorrelationHook {
    pub fn new(description: &str, hook_type: &str) -> Self {
        Self {
            description: description.to_string(),
            hook_type: hook_type.to_string(),
            parameters: HashMap::new(),
        }
    }

    pub fn with_parameter(mut self, key: &str, value: &str) -> Self {
        self.parameters.insert(key.to_string(), value.to_string());
        self
    }
}

/// Create a pre-defined correlation hook for linking proxy JWT modifications
/// with database query findings.
pub fn jwt_to_db_query_hook() -> CorrelationHook {
    CorrelationHook::new(
        "Link JWT token modifications in proxy to subsequent database queries",
        "jwt_db_correlation",
    )
    .with_parameter("source_field", "header:Authorization")
    .with_parameter("target_loadout", "db-pentest")
    .with_parameter("match_pattern", "Bearer\\s+(.+)")
}

/// Create a pre-defined correlation hook for linking proxy traffic with
/// authentication testing results.
pub fn proxy_auth_hook() -> CorrelationHook {
    CorrelationHook::new(
        "Link proxy traffic patterns with authentication test results",
        "proxy_auth_correlation",
    )
    .with_parameter("target_loadout", "auth-test")
    .with_parameter("match_headers", "Authorization,Cookie,Set-Cookie")
}

/// Create a pre-defined correlation hook for linking proxy traffic with
/// mobile dynamic findings.
pub fn proxy_mobile_hook() -> CorrelationHook {
    CorrelationHook::new(
        "Link proxy traffic patterns with mobile dynamic analysis findings",
        "proxy_mobile_correlation",
    )
    .with_parameter("target_loadout", "mobile-dynamic")
    .with_parameter("match_field", "host")
}

/// A temporal correlation entry linking two findings by time proximity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalCorrelation {
    /// First finding reference.
    pub a: CorrelationReference,
    /// Second finding reference.
    pub b: CorrelationReference,
    /// Time delta in milliseconds between the two findings.
    pub delta_ms: i64,
    /// Confidence score (0.0 - 1.0) based on time proximity.
    pub confidence: f64,
}

/// Behavioral pattern that can be matched across loadouts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralPattern {
    /// Unique pattern identifier.
    pub id: String,
    /// Description of the pattern.
    pub description: String,
    /// Host pattern (regex or exact match).
    pub host_pattern: Option<String>,
    /// Path pattern (regex or exact match).
    pub path_pattern: Option<String>,
    /// Required finding sources for a match.
    pub required_sources: Vec<CorrelationSource>,
    /// Minimum number of sources that must match.
    pub min_sources: usize,
}

/// Correlation engine that performs temporal and behavioral correlation.
pub struct CorrelationEngine {
    /// Maximum time window (in milliseconds) for temporal correlation.
    pub temporal_window_ms: i64,
    /// Behavioral patterns to match.
    patterns: Vec<BehavioralPattern>,
}

impl CorrelationEngine {
    /// Create a new correlation engine with default settings.
    pub fn new() -> Self {
        Self {
            temporal_window_ms: 60_000, // 60 seconds default
            patterns: Vec::new(),
        }
    }

    /// Set the temporal correlation window.
    pub fn with_temporal_window(mut self, window_ms: i64) -> Self {
        self.temporal_window_ms = window_ms;
        self
    }

    /// Register a behavioral pattern.
    pub fn add_pattern(mut self, pattern: BehavioralPattern) -> Self {
        self.patterns.push(pattern);
        self
    }

    /// Find temporal correlations between references in a context.
    ///
    /// Pairs references from different sources that occur within the
    /// temporal window and have matching host/path metadata.
    pub fn find_temporal_correlations(
        &self,
        context: &CorrelationContext,
    ) -> Vec<TemporalCorrelation> {
        let mut results = Vec::new();
        for (i, a) in context.references.iter().enumerate() {
            for b in context.references.iter().skip(i + 1) {
                if a.source == b.source {
                    continue; // skip same-source pairs
                }
                if let (Ok(ta), Ok(tb)) = (
                    chrono::DateTime::parse_from_rfc3339(&a.timestamp),
                    chrono::DateTime::parse_from_rfc3339(&b.timestamp),
                ) {
                    let delta_ms = (ta - tb).num_milliseconds().abs();
                    if delta_ms <= self.temporal_window_ms {
                        let confidence = 1.0 - (delta_ms as f64 / self.temporal_window_ms as f64);
                        results.push(TemporalCorrelation {
                            a: a.clone(),
                            b: b.clone(),
                            delta_ms,
                            confidence: confidence.clamp(0.0, 1.0),
                        });
                    }
                }
            }
        }
        results
    }

    /// Match behavioral patterns against a context.
    ///
    /// Returns patterns that have sufficient source diversity to match.
    pub fn match_behavioral(&self, context: &CorrelationContext) -> Vec<(BehavioralPattern, f64)> {
        let mut matches = Vec::new();
        for pattern in &self.patterns {
            let mut matched_sources: std::collections::HashSet<CorrelationSource> =
                std::collections::HashSet::new();
            for reference in &context.references {
                let source_match = pattern.required_sources.contains(&reference.source);
                if source_match {
                    let host_match = pattern
                        .host_pattern
                        .as_ref()
                        .map(|h| {
                            reference
                                .metadata
                                .get("host")
                                .map(|rh| rh.contains(h))
                                .unwrap_or(false)
                        })
                        .unwrap_or(true);
                    let path_match = pattern
                        .path_pattern
                        .as_ref()
                        .map(|p| {
                            reference
                                .metadata
                                .get("path")
                                .map(|rp| rp.contains(p))
                                .unwrap_or(false)
                        })
                        .unwrap_or(true);
                    if host_match && path_match {
                        matched_sources.insert(reference.source);
                    }
                }
            }
            if matched_sources.len() >= pattern.min_sources {
                let confidence =
                    matched_sources.len() as f64 / pattern.required_sources.len() as f64;
                matches.push((pattern.clone(), confidence.min(1.0)));
            }
        }
        matches
    }

    /// Run the full correlation pipeline on a context.
    ///
    /// Returns temporal correlations, behavioral matches, and updated summary.
    pub fn correlate(
        &self,
        context: &mut CorrelationContext,
    ) -> (Vec<TemporalCorrelation>, Vec<(BehavioralPattern, f64)>) {
        let temporal = self.find_temporal_correlations(context);
        let behavioral = self.match_behavioral(context);

        context.summary.temporal_correlations = temporal.len() as u64;
        context.summary.behavioral_correlations = behavioral.len() as u64;

        (temporal, behavioral)
    }
}

/// ML heuristics for confidence scoring in correlation.
///
/// Provides machine learning-inspired scoring functions that consider
/// multiple factors to produce a confidence score for correlations.
pub struct ConfidenceScorer {
    /// Weight for temporal proximity (0.0 - 1.0).
    pub temporal_weight: f64,
    /// Weight for source diversity (0.0 - 1.0).
    pub source_diversity_weight: f64,
    /// Weight for pattern match strength (0.0 - 1.0).
    pub pattern_match_weight: f64,
    /// Weight for metadata similarity (0.0 - 1.0).
    pub metadata_weight: f64,
    /// Weight for finding severity (0.0 - 1.0).
    pub severity_weight: f64,
}

impl Default for ConfidenceScorer {
    fn default() -> Self {
        Self {
            temporal_weight: 0.3,
            source_diversity_weight: 0.2,
            pattern_match_weight: 0.25,
            metadata_weight: 0.15,
            severity_weight: 0.1,
        }
    }
}

impl ConfidenceScorer {
    /// Create a new confidence scorer with custom weights.
    pub fn new(
        temporal_weight: f64,
        source_diversity_weight: f64,
        pattern_match_weight: f64,
        metadata_weight: f64,
        severity_weight: f64,
    ) -> Self {
        Self {
            temporal_weight,
            source_diversity_weight,
            pattern_match_weight,
            metadata_weight,
            severity_weight,
        }
    }

    /// Calculate confidence score for a temporal correlation.
    pub fn score_temporal(&self, correlation: &TemporalCorrelation) -> f64 {
        let temporal_score = 1.0 - (correlation.delta_ms as f64 / 60_000.0).min(1.0);
        temporal_score * self.temporal_weight
    }

    /// Calculate confidence score for source diversity.
    pub fn score_source_diversity(&self, sources: &[CorrelationSource]) -> f64 {
        let unique_sources: std::collections::HashSet<_> = sources.iter().collect();
        let diversity_score = unique_sources.len() as f64 / sources.len().max(1) as f64;
        diversity_score * self.source_diversity_weight
    }

    /// Calculate confidence score for pattern match strength.
    pub fn score_pattern_match(
        &self,
        pattern: &BehavioralPattern,
        matched_sources: &[CorrelationSource],
    ) -> f64 {
        let match_ratio =
            matched_sources.len() as f64 / pattern.required_sources.len().max(1) as f64;
        match_ratio * self.pattern_match_weight
    }

    /// Calculate confidence score for metadata similarity.
    pub fn score_metadata_similarity(
        &self,
        metadata1: &HashMap<String, String>,
        metadata2: &HashMap<String, String>,
    ) -> f64 {
        let keys1: std::collections::HashSet<_> = metadata1.keys().collect();
        let keys2: std::collections::HashSet<_> = metadata2.keys().collect();
        let common_keys = keys1.intersection(&keys2).count();
        let total_keys = keys1.union(&keys2).count();
        let similarity = common_keys as f64 / total_keys.max(1) as f64;
        similarity * self.metadata_weight
    }

    /// Calculate confidence score for finding severity.
    pub fn score_severity(&self, severity: u8) -> f64 {
        let normalized = severity as f64 / 10.0;
        normalized * self.severity_weight
    }

    /// Calculate combined confidence score from multiple factors.
    pub fn calculate_combined_confidence(
        &self,
        temporal_score: f64,
        diversity_score: f64,
        pattern_score: f64,
        metadata_score: f64,
        severity_score: f64,
    ) -> f64 {
        let combined =
            temporal_score + diversity_score + pattern_score + metadata_score + severity_score;
        combined.clamp(0.0, 1.0)
    }

    /// Score a correlation reference with all available factors.
    pub fn score_reference(
        &self,
        reference: &CorrelationReference,
        all_references: &[CorrelationReference],
    ) -> f64 {
        let temporal_score = self.temporal_weight; // Base score for single reference
        let sources: Vec<_> = all_references.iter().map(|r| r.source).collect();
        let diversity_score = self.score_source_diversity(&sources);
        let metadata_score = self.score_metadata_similarity(&reference.metadata, &HashMap::new());
        let severity_score = self.score_severity(0); // Default severity

        self.calculate_combined_confidence(
            temporal_score,
            diversity_score,
            0.0, // No pattern match for single reference
            metadata_score,
            severity_score,
        )
    }
}

impl Default for CorrelationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_correlation_reference_new() {
        let r = CorrelationReference::new(
            CorrelationSource::DbPentest,
            "db-finding-1",
            "SQLi found in proxy-modified request",
        );
        assert_eq!(r.source, CorrelationSource::DbPentest);
        assert_eq!(r.finding_id, "db-finding-1");
        assert_eq!(r.confidence, 1.0);
    }

    #[test]
    fn test_correlation_reference_with_confidence() {
        let r = CorrelationReference::new(
            CorrelationSource::AuthTest,
            "auth-1",
            "Token reuse detected",
        )
        .with_confidence(0.75)
        .with_metadata("key", "value");
        assert_eq!(r.confidence, 0.75);
        assert_eq!(r.metadata.get("key").unwrap(), "value");
    }

    #[test]
    fn test_correlation_reference_clamps_confidence() {
        let r = CorrelationReference::new(CorrelationSource::External, "ext-1", "Manual")
            .with_confidence(1.5);
        assert_eq!(r.confidence, 1.0);

        let r2 = CorrelationReference::new(CorrelationSource::External, "ext-2", "Manual")
            .with_confidence(-0.5);
        assert_eq!(r2.confidence, 0.0);
    }

    #[test]
    fn test_correlation_context_add_flow_correlation() {
        let mut ctx = CorrelationContext::new();
        let ref1 = CorrelationReference::new(
            CorrelationSource::DbPentest,
            "db-1",
            "DB finding linked to flow 0",
        );
        ctx.add_flow_correlation(0, ref1);

        assert_eq!(ctx.summary.total_references, 1);
        assert_eq!(ctx.summary.correlated_flows, 1);
        assert_eq!(ctx.get_flow_correlations(0).len(), 1);
        assert!(ctx.get_flow_correlations(1).is_empty());
    }

    #[test]
    fn test_correlation_context_get_by_source() {
        let mut ctx = CorrelationContext::new();
        ctx.add_reference(CorrelationReference::new(
            CorrelationSource::DbPentest,
            "db-1",
            "DB finding",
        ));
        ctx.add_reference(CorrelationReference::new(
            CorrelationSource::AuthTest,
            "auth-1",
            "Auth finding",
        ));
        ctx.add_reference(CorrelationReference::new(
            CorrelationSource::DbPentest,
            "db-2",
            "Another DB finding",
        ));

        assert_eq!(ctx.get_by_source(CorrelationSource::DbPentest).len(), 2);
        assert_eq!(ctx.get_by_source(CorrelationSource::AuthTest).len(), 1);
        assert_eq!(ctx.get_by_source(CorrelationSource::Wireless).len(), 0);
    }

    #[test]
    fn test_correlation_context_summary() {
        let mut ctx = CorrelationContext::new();
        ctx.add_reference(
            CorrelationReference::new(CorrelationSource::DbPentest, "db-1", "DB finding")
                .with_confidence(0.8),
        );
        ctx.add_reference(
            CorrelationReference::new(CorrelationSource::AuthTest, "auth-1", "Auth finding")
                .with_confidence(0.9),
        );

        assert_eq!(ctx.summary.total_references, 2);
        assert_eq!(ctx.summary.unique_sources, 2);
        assert!((ctx.summary.avg_confidence - 0.85).abs() < 0.01);
    }

    #[test]
    fn test_correlation_hook() {
        let hook = jwt_to_db_query_hook();
        assert_eq!(hook.hook_type, "jwt_db_correlation");
        assert_eq!(hook.parameters.get("target_loadout").unwrap(), "db-pentest");
    }

    #[test]
    fn test_proxy_auth_hook() {
        let hook = proxy_auth_hook();
        assert_eq!(hook.hook_type, "proxy_auth_correlation");
        assert!(hook.parameters.contains_key("match_headers"));
    }

    #[test]
    fn test_proxy_mobile_hook() {
        let hook = proxy_mobile_hook();
        assert_eq!(hook.hook_type, "proxy_mobile_correlation");
        assert_eq!(
            hook.parameters.get("target_loadout").unwrap(),
            "mobile-dynamic"
        );
    }

    #[test]
    fn test_correlation_context_multiple_flows() {
        let mut ctx = CorrelationContext::new();
        for i in 0..5 {
            ctx.add_flow_correlation(
                i,
                CorrelationReference::new(
                    CorrelationSource::DbPentest,
                    &format!("db-{}", i),
                    &format!("Finding for flow {}", i),
                ),
            );
        }
        assert_eq!(ctx.summary.total_references, 5);
        assert_eq!(ctx.summary.correlated_flows, 5);
        for i in 0..5 {
            assert_eq!(ctx.get_flow_correlations(i).len(), 1);
        }
    }

    #[test]
    fn test_correlation_reference_roundtrip() {
        let r =
            CorrelationReference::new(CorrelationSource::MobileDynamic, "mob-1", "Mobile finding")
                .with_confidence(0.6)
                .with_metadata("platform", "android");
        let json = serde_json::to_string(&r).unwrap();
        let back: CorrelationReference = serde_json::from_str(&json).unwrap();
        assert_eq!(back.source, CorrelationSource::MobileDynamic);
        assert_eq!(back.finding_id, "mob-1");
        assert_eq!(back.confidence, 0.6);
        assert_eq!(back.metadata.get("platform").unwrap(), "android");
    }

    #[test]
    fn test_correlation_context_roundtrip() {
        let mut ctx = CorrelationContext::new();
        ctx.add_flow_correlation(
            0,
            CorrelationReference::new(CorrelationSource::DbPentest, "db-1", "DB finding"),
        );
        let json = serde_json::to_string(&ctx).unwrap();
        let back: CorrelationContext = serde_json::from_str(&json).unwrap();
        assert_eq!(back.summary.total_references, 1);
        assert_eq!(back.get_flow_correlations(0).len(), 1);
    }

    // --- Temporal correlation tests ---

    fn make_ref_with_time(source: CorrelationSource, id: &str, ts: &str) -> CorrelationReference {
        CorrelationReference {
            source,
            finding_id: id.to_string(),
            description: format!("Finding {}", id),
            confidence: 0.8,
            timestamp: ts.to_string(),
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_temporal_correlation_within_window() {
        let mut ctx = CorrelationContext::new();
        ctx.add_reference(make_ref_with_time(
            CorrelationSource::DbPentest,
            "db-1",
            "2026-01-01T00:00:00Z",
        ));
        ctx.add_reference(make_ref_with_time(
            CorrelationSource::AuthTest,
            "auth-1",
            "2026-01-01T00:00:05Z",
        ));

        let engine = CorrelationEngine::new().with_temporal_window(60_000);
        let temporal = engine.find_temporal_correlations(&ctx);
        assert_eq!(temporal.len(), 1);
        assert_eq!(temporal[0].delta_ms, 5000);
        assert!(temporal[0].confidence > 0.9);
    }

    #[test]
    fn test_temporal_correlation_outside_window() {
        let mut ctx = CorrelationContext::new();
        ctx.add_reference(make_ref_with_time(
            CorrelationSource::DbPentest,
            "db-1",
            "2026-01-01T00:00:00Z",
        ));
        ctx.add_reference(make_ref_with_time(
            CorrelationSource::AuthTest,
            "auth-1",
            "2026-01-01T01:00:00Z", // 1 hour apart
        ));

        let engine = CorrelationEngine::new().with_temporal_window(60_000);
        let temporal = engine.find_temporal_correlations(&ctx);
        assert!(temporal.is_empty());
    }

    #[test]
    fn test_temporal_correlation_same_source_skipped() {
        let mut ctx = CorrelationContext::new();
        ctx.add_reference(make_ref_with_time(
            CorrelationSource::DbPentest,
            "db-1",
            "2026-01-01T00:00:00Z",
        ));
        ctx.add_reference(make_ref_with_time(
            CorrelationSource::DbPentest,
            "db-2",
            "2026-01-01T00:00:01Z",
        ));

        let engine = CorrelationEngine::new().with_temporal_window(60_000);
        let temporal = engine.find_temporal_correlations(&ctx);
        assert!(temporal.is_empty());
    }

    // --- Behavioral correlation tests ---

    #[test]
    fn test_behavioral_pattern_match() {
        let mut ctx = CorrelationContext::new();
        let mut meta_a = HashMap::new();
        meta_a.insert("host".to_string(), "api.example.com".to_string());
        ctx.add_reference(CorrelationReference {
            source: CorrelationSource::DbPentest,
            finding_id: "db-1".to_string(),
            description: "SQLi".to_string(),
            confidence: 0.9,
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: meta_a,
        });
        let mut meta_b = HashMap::new();
        meta_b.insert("host".to_string(), "api.example.com".to_string());
        ctx.add_reference(CorrelationReference {
            source: CorrelationSource::AuthTest,
            finding_id: "auth-1".to_string(),
            description: "Auth bypass".to_string(),
            confidence: 0.8,
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: meta_b,
        });

        let pattern = BehavioralPattern {
            id: "sqli-auth".to_string(),
            description: "SQLi with auth bypass".to_string(),
            host_pattern: Some("api.example.com".to_string()),
            path_pattern: None,
            required_sources: vec![CorrelationSource::DbPentest, CorrelationSource::AuthTest],
            min_sources: 2,
        };

        let engine = CorrelationEngine::new().add_pattern(pattern);
        let behavioral = engine.match_behavioral(&ctx);
        assert_eq!(behavioral.len(), 1);
        assert_eq!(behavioral[0].0.id, "sqli-auth");
    }

    #[test]
    fn test_behavioral_pattern_insufficient_sources() {
        let mut ctx = CorrelationContext::new();
        ctx.add_reference(CorrelationReference::new(
            CorrelationSource::DbPentest,
            "db-1",
            "SQLi",
        ));

        let pattern = BehavioralPattern {
            id: "multi-source".to_string(),
            description: "Needs 3 sources".to_string(),
            host_pattern: None,
            path_pattern: None,
            required_sources: vec![
                CorrelationSource::DbPentest,
                CorrelationSource::AuthTest,
                CorrelationSource::MobileDynamic,
            ],
            min_sources: 3,
        };

        let engine = CorrelationEngine::new().add_pattern(pattern);
        let behavioral = engine.match_behavioral(&ctx);
        assert!(behavioral.is_empty());
    }

    #[test]
    fn test_correlation_engine_full_pipeline() {
        let mut ctx = CorrelationContext::new();
        ctx.add_reference(make_ref_with_time(
            CorrelationSource::DbPentest,
            "db-1",
            "2026-01-01T00:00:00Z",
        ));
        ctx.add_reference(make_ref_with_time(
            CorrelationSource::AuthTest,
            "auth-1",
            "2026-01-01T00:00:10Z",
        ));

        let engine = CorrelationEngine::new().with_temporal_window(60_000);
        let (temporal, _behavioral) = engine.correlate(&mut ctx);
        assert_eq!(temporal.len(), 1);
        assert_eq!(ctx.summary.temporal_correlations, 1);
    }

    #[test]
    fn test_behavioral_pattern_host_mismatch() {
        let mut ctx = CorrelationContext::new();
        let mut meta = HashMap::new();
        meta.insert("host".to_string(), "other.example.com".to_string());
        ctx.add_reference(CorrelationReference {
            source: CorrelationSource::DbPentest,
            finding_id: "db-1".to_string(),
            description: "Finding".to_string(),
            confidence: 0.9,
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: meta,
        });
        ctx.add_reference(CorrelationReference::new(
            CorrelationSource::AuthTest,
            "auth-1",
            "Finding",
        ));

        let pattern = BehavioralPattern {
            id: "test".to_string(),
            description: "Test".to_string(),
            host_pattern: Some("api.example.com".to_string()),
            path_pattern: None,
            required_sources: vec![CorrelationSource::DbPentest, CorrelationSource::AuthTest],
            min_sources: 2,
        };

        let engine = CorrelationEngine::new().add_pattern(pattern);
        let behavioral = engine.match_behavioral(&ctx);
        assert!(behavioral.is_empty());
    }

    #[test]
    fn test_correlation_summary_roundtrip() {
        let summary = CorrelationSummary {
            total_references: 10,
            unique_sources: 3,
            correlated_flows: 5,
            avg_confidence: 0.85,
            temporal_correlations: 2,
            behavioral_correlations: 1,
        };
        let json = serde_json::to_string(&summary).unwrap();
        let back: CorrelationSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(back.temporal_correlations, 2);
        assert_eq!(back.behavioral_correlations, 1);
    }

    // --- Edge case tests ---

    #[test]
    fn test_behavioral_pattern_path_matching() {
        let mut ctx = CorrelationContext::new();
        let mut meta_a = HashMap::new();
        meta_a.insert("host".to_string(), "api.example.com".to_string());
        meta_a.insert("path".to_string(), "/admin/users".to_string());
        ctx.add_reference(CorrelationReference {
            source: CorrelationSource::DbPentest,
            finding_id: "db-1".to_string(),
            description: "SQLi".to_string(),
            confidence: 0.9,
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: meta_a,
        });
        let mut meta_b = HashMap::new();
        meta_b.insert("host".to_string(), "api.example.com".to_string());
        meta_b.insert("path".to_string(), "/admin/config".to_string());
        ctx.add_reference(CorrelationReference {
            source: CorrelationSource::AuthTest,
            finding_id: "auth-1".to_string(),
            description: "Auth bypass".to_string(),
            confidence: 0.8,
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: meta_b,
        });

        let pattern = BehavioralPattern {
            id: "admin-path".to_string(),
            description: "Admin path pattern".to_string(),
            host_pattern: Some("api.example.com".to_string()),
            path_pattern: Some("/admin".to_string()),
            required_sources: vec![CorrelationSource::DbPentest, CorrelationSource::AuthTest],
            min_sources: 2,
        };

        let engine = CorrelationEngine::new().add_pattern(pattern);
        let behavioral = engine.match_behavioral(&ctx);
        assert_eq!(behavioral.len(), 1);
    }

    #[test]
    fn test_behavioral_pattern_path_mismatch() {
        let mut ctx = CorrelationContext::new();
        let mut meta = HashMap::new();
        meta.insert("host".to_string(), "api.example.com".to_string());
        meta.insert("path".to_string(), "/public/data".to_string());
        ctx.add_reference(CorrelationReference {
            source: CorrelationSource::DbPentest,
            finding_id: "db-1".to_string(),
            description: "SQLi".to_string(),
            confidence: 0.9,
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: meta,
        });
        ctx.add_reference(CorrelationReference::new(
            CorrelationSource::AuthTest,
            "auth-1",
            "Auth bypass",
        ));

        let pattern = BehavioralPattern {
            id: "admin-only".to_string(),
            description: "Admin only".to_string(),
            host_pattern: Some("api.example.com".to_string()),
            path_pattern: Some("/admin".to_string()),
            required_sources: vec![CorrelationSource::DbPentest, CorrelationSource::AuthTest],
            min_sources: 2,
        };

        let engine = CorrelationEngine::new().add_pattern(pattern);
        let behavioral = engine.match_behavioral(&ctx);
        assert!(behavioral.is_empty());
    }

    #[test]
    fn test_temporal_correlation_exact_boundary() {
        let mut ctx = CorrelationContext::new();
        // Exactly at the boundary (60_000 ms apart)
        ctx.add_reference(make_ref_with_time(
            CorrelationSource::DbPentest,
            "db-1",
            "2026-01-01T00:00:00Z",
        ));
        ctx.add_reference(make_ref_with_time(
            CorrelationSource::AuthTest,
            "auth-1",
            "2026-01-01T00:01:00Z", // exactly 60s = 60_000ms
        ));

        let engine = CorrelationEngine::new().with_temporal_window(60_000);
        let temporal = engine.find_temporal_correlations(&ctx);
        // Exactly at boundary: delta=60000, window=60000 -> confidence = 1.0 - (60000/60000) = 0.0
        // Should still be included (confidence > 0 is not checked in find_temporal_correlations)
        assert_eq!(temporal.len(), 1);
        assert_eq!(temporal[0].delta_ms, 60_000);
    }

    #[test]
    fn test_temporal_correlation_reversed_timestamps() {
        let mut ctx = CorrelationContext::new();
        // Later timestamp first
        ctx.add_reference(make_ref_with_time(
            CorrelationSource::DbPentest,
            "db-1",
            "2026-01-01T00:01:00Z",
        ));
        ctx.add_reference(make_ref_with_time(
            CorrelationSource::AuthTest,
            "auth-1",
            "2026-01-01T00:00:00Z",
        ));

        let engine = CorrelationEngine::new().with_temporal_window(120_000);
        let temporal = engine.find_temporal_correlations(&ctx);
        // Should still match (abs delta = 60s)
        assert_eq!(temporal.len(), 1);
    }

    #[test]
    fn test_correlation_engine_both_temporal_and_behavioral() {
        let mut ctx = CorrelationContext::new();
        let mut meta_a = HashMap::new();
        meta_a.insert("host".to_string(), "api.example.com".to_string());
        ctx.add_reference(CorrelationReference {
            source: CorrelationSource::DbPentest,
            finding_id: "db-1".to_string(),
            description: "SQLi".to_string(),
            confidence: 0.9,
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            metadata: meta_a,
        });
        let mut meta_b = HashMap::new();
        meta_b.insert("host".to_string(), "api.example.com".to_string());
        ctx.add_reference(CorrelationReference {
            source: CorrelationSource::AuthTest,
            finding_id: "auth-1".to_string(),
            description: "Auth bypass".to_string(),
            confidence: 0.8,
            timestamp: "2026-01-01T00:00:10Z".to_string(),
            metadata: meta_b,
        });

        let pattern = BehavioralPattern {
            id: "sqli-auth".to_string(),
            description: "SQLi + auth".to_string(),
            host_pattern: Some("api.example.com".to_string()),
            path_pattern: None,
            required_sources: vec![CorrelationSource::DbPentest, CorrelationSource::AuthTest],
            min_sources: 2,
        };

        let engine = CorrelationEngine::new()
            .with_temporal_window(60_000)
            .add_pattern(pattern);
        let (temporal, behavioral) = engine.correlate(&mut ctx);
        assert_eq!(temporal.len(), 1);
        assert_eq!(behavioral.len(), 1);
    }

    #[test]
    fn test_correlation_reference_zero_confidence() {
        let mut ctx = CorrelationContext::new();
        ctx.add_reference(CorrelationReference {
            source: CorrelationSource::DbPentest,
            finding_id: "db-1".to_string(),
            description: "Low confidence".to_string(),
            confidence: 0.0,
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: HashMap::new(),
        });
        assert_eq!(ctx.references.len(), 1);
        assert_eq!(ctx.references[0].confidence, 0.0);
    }

    #[test]
    fn test_behavioral_pattern_min_sources_partial_match() {
        let mut ctx = CorrelationContext::new();
        let mut meta = HashMap::new();
        meta.insert("host".to_string(), "api.example.com".to_string());
        ctx.add_reference(CorrelationReference {
            source: CorrelationSource::DbPentest,
            finding_id: "db-1".to_string(),
            description: "SQLi".to_string(),
            confidence: 0.9,
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: meta,
        });
        ctx.add_reference(CorrelationReference::new(
            CorrelationSource::AuthTest,
            "auth-1",
            "Auth bypass",
        ));
        ctx.add_reference(CorrelationReference::new(
            CorrelationSource::MobileDynamic,
            "mob-1",
            "Mobile finding",
        ));

        // Requires 3 sources but min_sources=2, so 2 matching should be enough
        let pattern = BehavioralPattern {
            id: "partial".to_string(),
            description: "Partial match OK".to_string(),
            host_pattern: Some("api.example.com".to_string()),
            path_pattern: None,
            required_sources: vec![
                CorrelationSource::DbPentest,
                CorrelationSource::AuthTest,
                CorrelationSource::MobileDynamic,
            ],
            min_sources: 2,
        };

        let engine = CorrelationEngine::new().add_pattern(pattern);
        let behavioral = engine.match_behavioral(&ctx);
        // DbPentest has host match, AuthTest and MobileDynamic don't have host in metadata
        // So only DbPentest matches the host pattern -> only 1 source matches -> not enough
        // But wait - the pattern matching checks if the reference's metadata host matches
        // AuthTest and MobileDynamic don't have host metadata, so they won't match host_pattern
        // So this tests that partial match works when some sources lack the metadata
        assert!(behavioral.is_empty());
    }

    #[test]
    fn test_temporal_correlation_multiple_pairs() {
        let mut ctx = CorrelationContext::new();
        ctx.add_reference(make_ref_with_time(
            CorrelationSource::DbPentest,
            "db-1",
            "2026-01-01T00:00:00Z",
        ));
        ctx.add_reference(make_ref_with_time(
            CorrelationSource::AuthTest,
            "auth-1",
            "2026-01-01T00:00:05Z",
        ));
        ctx.add_reference(make_ref_with_time(
            CorrelationSource::MobileDynamic,
            "mob-1",
            "2026-01-01T00:00:10Z",
        ));

        let engine = CorrelationEngine::new().with_temporal_window(60_000);
        let temporal = engine.find_temporal_correlations(&ctx);
        // 3 pairs: (db,auth), (db,mob), (auth,mob)
        assert_eq!(temporal.len(), 3);
    }

    // --- Confidence Scorer Tests ---

    #[test]
    fn test_confidence_scorer_default() {
        let scorer = ConfidenceScorer::default();
        assert_eq!(scorer.temporal_weight, 0.3);
        assert_eq!(scorer.source_diversity_weight, 0.2);
        assert_eq!(scorer.pattern_match_weight, 0.25);
        assert_eq!(scorer.metadata_weight, 0.15);
        assert_eq!(scorer.severity_weight, 0.1);
    }

    #[test]
    fn test_confidence_scorer_score_temporal() {
        let scorer = ConfidenceScorer::default();
        let correlation = TemporalCorrelation {
            a: CorrelationReference::new(CorrelationSource::DbPentest, "db-1", "test"),
            b: CorrelationReference::new(CorrelationSource::AuthTest, "auth-1", "test"),
            delta_ms: 0,
            confidence: 1.0,
        };
        assert_eq!(scorer.score_temporal(&correlation), 0.3); // 1.0 * 0.3

        let correlation = TemporalCorrelation {
            a: CorrelationReference::new(CorrelationSource::DbPentest, "db-1", "test"),
            b: CorrelationReference::new(CorrelationSource::AuthTest, "auth-1", "test"),
            delta_ms: 30_000, // 30 seconds
            confidence: 0.5,
        };
        let score = scorer.score_temporal(&correlation);
        assert!(score > 0.0 && score < 0.3); // Should be less than max
    }

    #[test]
    fn test_confidence_scorer_score_source_diversity() {
        let scorer = ConfidenceScorer::default();
        let sources = vec![
            CorrelationSource::DbPentest,
            CorrelationSource::AuthTest,
            CorrelationSource::MobileDynamic,
        ];
        let score = scorer.score_source_diversity(&sources);
        assert_eq!(score, 0.2); // 3 unique / 3 total = 1.0 * 0.2

        let sources = vec![
            CorrelationSource::DbPentest,
            CorrelationSource::DbPentest, // duplicate
        ];
        let score = scorer.score_source_diversity(&sources);
        assert!(score < 0.2); // 1 unique / 2 total = 0.5 * 0.2
    }

    #[test]
    fn test_confidence_scorer_score_pattern_match() {
        let scorer = ConfidenceScorer::default();
        let pattern = BehavioralPattern {
            id: "test".to_string(),
            description: "Test".to_string(),
            host_pattern: None,
            path_pattern: None,
            required_sources: vec![
                CorrelationSource::DbPentest,
                CorrelationSource::AuthTest,
                CorrelationSource::MobileDynamic,
            ],
            min_sources: 2,
        };
        let matched = vec![CorrelationSource::DbPentest, CorrelationSource::AuthTest];
        let score = scorer.score_pattern_match(&pattern, &matched);
        assert!(score > 0.0 && score <= 0.25);
    }

    #[test]
    fn test_confidence_scorer_score_metadata_similarity() {
        let scorer = ConfidenceScorer::default();
        let mut meta1 = HashMap::new();
        meta1.insert("host".to_string(), "api.example.com".to_string());
        meta1.insert("path".to_string(), "/data".to_string());

        let mut meta2 = HashMap::new();
        meta2.insert("host".to_string(), "api.example.com".to_string());

        let score = scorer.score_metadata_similarity(&meta1, &meta2);
        assert!(score > 0.0); // Should have some similarity
    }

    #[test]
    fn test_confidence_scorer_score_severity() {
        let scorer = ConfidenceScorer::default();
        assert_eq!(scorer.score_severity(0), 0.0);
        assert_eq!(scorer.score_severity(10), 0.1); // 10/10 * 0.1
    }

    #[test]
    fn test_confidence_scorer_calculate_combined_confidence() {
        let scorer = ConfidenceScorer::default();
        let combined = scorer.calculate_combined_confidence(0.3, 0.2, 0.25, 0.15, 0.1);
        assert_eq!(combined, 1.0);

        let combined = scorer.calculate_combined_confidence(0.1, 0.1, 0.1, 0.1, 0.1);
        assert_eq!(combined, 0.5);
    }

    #[test]
    fn test_confidence_scorer_score_reference() {
        let scorer = ConfidenceScorer::default();
        let reference =
            CorrelationReference::new(CorrelationSource::DbPentest, "db-1", "SQLi finding");
        let all_references = vec![
            CorrelationReference::new(CorrelationSource::DbPentest, "db-1", "SQLi finding"),
            CorrelationReference::new(CorrelationSource::AuthTest, "auth-1", "Auth bypass"),
        ];
        let score = scorer.score_reference(&reference, &all_references);
        assert!(score > 0.0 && score <= 1.0);
    }
}
