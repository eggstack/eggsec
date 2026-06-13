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
        let r = CorrelationReference::new(
            CorrelationSource::External,
            "ext-1",
            "Manual",
        )
        .with_confidence(1.5);
        assert_eq!(r.confidence, 1.0);

        let r2 = CorrelationReference::new(
            CorrelationSource::External,
            "ext-2",
            "Manual",
        )
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
        ctx.add_reference(CorrelationReference::new(
            CorrelationSource::DbPentest,
            "db-1",
            "DB finding",
        ).with_confidence(0.8));
        ctx.add_reference(CorrelationReference::new(
            CorrelationSource::AuthTest,
            "auth-1",
            "Auth finding",
        ).with_confidence(0.9));

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
        assert_eq!(hook.parameters.get("target_loadout").unwrap(), "mobile-dynamic");
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
        let r = CorrelationReference::new(
            CorrelationSource::MobileDynamic,
            "mob-1",
            "Mobile finding",
        )
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
            CorrelationReference::new(
                CorrelationSource::DbPentest,
                "db-1",
                "DB finding",
            ),
        );
        let json = serde_json::to_string(&ctx).unwrap();
        let back: CorrelationContext = serde_json::from_str(&json).unwrap();
        assert_eq!(back.summary.total_references, 1);
        assert_eq!(back.get_flow_correlations(0).len(), 1);
    }
}
