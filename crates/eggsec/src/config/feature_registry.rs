//! Authoritative compile-time feature registry.
//!
//! This module is the **single source of truth** for all `eggsec` crate features.
//! Every feature declared in `Cargo.toml [features]` must appear here exactly once.
//!
//! Production code uses [`feature_state`] to check availability. Domain descriptors,
//! operation metadata, command registration, and tool registration all derive their
//! feature checks from this registry.
//!
//! # Fail-closed contract
//!
//! - Known enabled feature → `true`
//! - Known disabled feature → `false`
//! - Unknown feature → `false` (and returns `FeatureState::Unknown` via the fallible API)

use std::fmt;

// ─── Feature categories ─────────────────────────────────────────────────────

/// Classification of feature purpose for diagnostics and contributor guidance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeatureCategory {
    /// Protocol adapters: tool-api, rest-api, grpc-api, ws-api, websocket
    ProtocolAdapter,
    /// Domain capabilities: db-pentest, mobile, wireless, web-proxy, evasion, postex, c2, nse
    DomainCapability,
    /// MCP/protocol exposure markers: db-pentest-mcp, web-proxy-mcp, c2-mcp
    ProtocolExposure,
    /// Marker-only features with no deps: advanced-hunting, compliance, etc.
    MarkerOnly,
    /// Database backend drivers: db-pentest-mssql-tiberius, db-pentest-mongodb, db-pentest-redis
    BackendDriver,
    /// Platform-sensitive features: stress-testing, packet-inspection, nse-ssh2, nse-sandbox
    PlatformSensitive,
    /// Storage/output integrations: database, sbom, container, pdf
    StorageIntegration,
    /// Aggregate features: full
    Aggregate,
    /// Security risk features: insecure-tls
    SecurityRisk,
    /// AI integration: ai-integration
    AiIntegration,
    /// Advanced extensions: mobile-dynamic, wireless-advanced, transparent-proxy, dynamic-plugins
    AdvancedExtension,
}

// ─── Feature state ───────────────────────────────────────────────────────────

/// The compile-time state of a feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeatureState {
    /// Feature is declared and currently compiled in.
    Enabled,
    /// Feature is declared but not currently compiled in.
    Disabled,
    /// Feature name is not in the registry (typo or missing entry).
    Unknown,
}

impl FeatureState {
    /// Returns `true` if the feature is enabled.
    pub fn is_enabled(self) -> bool {
        matches!(self, FeatureState::Enabled)
    }
}

impl fmt::Display for FeatureState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FeatureState::Enabled => write!(f, "enabled"),
            FeatureState::Disabled => write!(f, "disabled"),
            FeatureState::Unknown => write!(f, "unknown"),
        }
    }
}

// ─── Registry entry ──────────────────────────────────────────────────────────

/// Metadata for a single feature in the registry.
#[derive(Debug, Clone, Copy)]
pub struct FeatureEntry {
    /// The Cargo feature name (kebab-case).
    pub name: &'static str,
    /// Compile-time enabled state via `cfg!()`.
    pub enabled: bool,
    /// Feature category for diagnostics.
    pub category: FeatureCategory,
    /// User-facing hint for enabling this feature.
    pub hint: &'static str,
}

// ─── Macro ───────────────────────────────────────────────────────────────────

/// Build the authoritative feature registry.
///
/// Each entry maps a feature name to its `cfg!()` state, category, and hint.
/// The macro generates:
/// - `ALL_FEATURES: &[FeatureEntry]` — the complete list
/// - `feature_state(name) -> FeatureState` — fail-closed lookup
/// - `is_feature_enabled(name) -> bool` — convenience boolean (fail-closed)
/// - `is_known_feature(name) -> bool` — name validation
/// - `feature_missing_hint(name) -> Option<&str>` — diagnostic hints
macro_rules! feature_registry {
    ( $( $name:expr => {
        enabled: $enabled:expr,
        category: $category:ident,
        hint: $hint:expr
    } ),* $(,)? ) => {
        /// All features in the registry, in declaration order.
        pub static ALL_FEATURES: &[FeatureEntry] = &[
            $(
                FeatureEntry {
                    name: $name,
                    enabled: $enabled,
                    category: FeatureCategory::$category,
                    hint: $hint,
                },
            )*
        ];

        /// Look up the compile-time state of a feature by name.
        ///
        /// Returns `FeatureState::Enabled`, `FeatureState::Disabled`, or
        /// `FeatureState::Unknown` for unrecognized names.
        pub fn feature_state(name: &str) -> FeatureState {
            match name {
                $( $name => {
                    if $enabled {
                        FeatureState::Enabled
                    } else {
                        FeatureState::Disabled
                    }
                } )*
                _ => FeatureState::Unknown,
            }
        }

        /// Returns `true` if the named feature is currently compiled in.
        ///
        /// **Fail-closed**: unknown feature names return `false`.
        pub fn is_feature_enabled(name: &str) -> bool {
            feature_state(name).is_enabled()
        }

        /// Returns `true` if the named feature is in the registry.
        pub fn is_known_feature(name: &str) -> bool {
            !matches!(feature_state(name), FeatureState::Unknown)
        }

        /// Returns a diagnostic hint for enabling a missing feature, or `None`
        /// if the feature name is not in the registry.
        pub fn feature_missing_hint(name: &str) -> Option<&'static str> {
            match name {
                $( $name => Some($hint), )*
                _ => None,
            }
        }

        /// Classifies a feature by its category. Panics on unknown names —
        /// use only after confirming the feature is known.
        pub fn classify_feature(name: &str) -> Option<FeatureCategory> {
            match name {
                $( $name => Some(FeatureCategory::$category), )*
                _ => None,
            }
        }
    };
}

// ─── Registry definition ─────────────────────────────────────────────────────
//
// This is the single authoritative list. Every Cargo feature (except `default`)
// must appear here exactly once. To add a new feature:
//
// 1. Add it to `Cargo.toml [features]`
// 2. Add an entry here with the correct `cfg!()`, category, and hint
// 3. Tests will validate bidirectional coverage automatically

feature_registry! {
    // ── Protocol adapters ───────────────────────────────────────────────────
    "tool-api" => {
        enabled: cfg!(feature = "tool-api"),
        category: ProtocolAdapter,
        hint: "enable feature 'tool-api' in Cargo.toml: cargo build --features tool-api"
    },
    "rest-api" => {
        enabled: cfg!(feature = "rest-api"),
        category: ProtocolAdapter,
        hint: "enable feature 'rest-api' in Cargo.toml: cargo build --features rest-api"
    },
    "grpc-api" => {
        enabled: cfg!(feature = "grpc-api"),
        category: ProtocolAdapter,
        hint: "enable feature 'grpc-api' in Cargo.toml: cargo build --features grpc-api"
    },
    "ws-api" => {
        enabled: cfg!(feature = "ws-api"),
        category: ProtocolAdapter,
        hint: "enable feature 'ws-api' in Cargo.toml: cargo build --features ws-api"
    },
    "websocket" => {
        enabled: cfg!(feature = "websocket"),
        category: ProtocolAdapter,
        hint: "enable feature 'websocket' in Cargo.toml: cargo build --features websocket"
    },

    // ── Domain capabilities ─────────────────────────────────────────────────
    "db-pentest" => {
        enabled: cfg!(feature = "db-pentest"),
        category: DomainCapability,
        hint: "enable feature 'db-pentest' in Cargo.toml: cargo build --features db-pentest"
    },
    "mobile" => {
        enabled: cfg!(feature = "mobile"),
        category: DomainCapability,
        hint: "enable feature 'mobile' in Cargo.toml: cargo build --features mobile"
    },
    "wireless" => {
        enabled: cfg!(feature = "wireless"),
        category: DomainCapability,
        hint: "enable feature 'wireless' in Cargo.toml: cargo build --features wireless"
    },
    "web-proxy" => {
        enabled: cfg!(feature = "web-proxy"),
        category: DomainCapability,
        hint: "enable feature 'web-proxy' in Cargo.toml: cargo build --features web-proxy"
    },
    "evasion" => {
        enabled: cfg!(feature = "evasion"),
        category: DomainCapability,
        hint: "enable feature 'evasion' in Cargo.toml: cargo build --features evasion"
    },
    "postex" => {
        enabled: cfg!(feature = "postex"),
        category: DomainCapability,
        hint: "enable feature 'postex' in Cargo.toml: cargo build --features postex"
    },
    "c2" => {
        enabled: cfg!(feature = "c2"),
        category: DomainCapability,
        hint: "enable feature 'c2' in Cargo.toml: cargo build --features c2"
    },
    "nse" => {
        enabled: cfg!(feature = "nse"),
        category: DomainCapability,
        hint: "enable feature 'nse' in Cargo.toml: cargo build --features nse"
    },

    // ── Protocol exposure markers ───────────────────────────────────────────
    "db-pentest-mcp" => {
        enabled: cfg!(feature = "db-pentest-mcp"),
        category: ProtocolExposure,
        hint: "enable feature 'db-pentest-mcp' in Cargo.toml: cargo build --features db-pentest-mcp"
    },
    "web-proxy-mcp" => {
        enabled: cfg!(feature = "web-proxy-mcp"),
        category: ProtocolExposure,
        hint: "enable feature 'web-proxy-mcp' in Cargo.toml: cargo build --features web-proxy-mcp"
    },
    "c2-mcp" => {
        enabled: cfg!(feature = "c2-mcp"),
        category: ProtocolExposure,
        hint: "enable feature 'c2-mcp' in Cargo.toml: cargo build --features c2-mcp"
    },

    // ── Marker-only features ────────────────────────────────────────────────
    "advanced-hunting" => {
        enabled: cfg!(feature = "advanced-hunting"),
        category: MarkerOnly,
        hint: "enable feature 'advanced-hunting' in Cargo.toml: cargo build --features advanced-hunting"
    },
    "compliance" => {
        enabled: cfg!(feature = "compliance"),
        category: MarkerOnly,
        hint: "enable feature 'compliance' in Cargo.toml: cargo build --features compliance"
    },
    "external-integrations" => {
        enabled: cfg!(feature = "external-integrations"),
        category: MarkerOnly,
        hint: "enable feature 'external-integrations' in Cargo.toml: cargo build --features external-integrations"
    },
    "finding-workflow" => {
        enabled: cfg!(feature = "finding-workflow"),
        category: MarkerOnly,
        hint: "enable feature 'finding-workflow' in Cargo.toml: cargo build --features finding-workflow"
    },
    "vuln-management" => {
        enabled: cfg!(feature = "vuln-management"),
        category: MarkerOnly,
        hint: "enable feature 'vuln-management' in Cargo.toml: cargo build --features vuln-management"
    },
    "cloud" => {
        enabled: cfg!(feature = "cloud"),
        category: MarkerOnly,
        hint: "enable feature 'cloud' in Cargo.toml: cargo build --features cloud"
    },
    "git-secrets" => {
        enabled: cfg!(feature = "git-secrets"),
        category: MarkerOnly,
        hint: "enable feature 'git-secrets' in Cargo.toml: cargo build --features git-secrets"
    },
    "api-schema" => {
        enabled: cfg!(feature = "api-schema"),
        category: MarkerOnly,
        hint: "enable feature 'api-schema' in Cargo.toml: cargo build --features api-schema"
    },
    "daemon-client" => {
        enabled: cfg!(feature = "daemon-client"),
        category: MarkerOnly,
        hint: "enable feature 'daemon-client' in Cargo.toml: cargo build --features daemon-client"
    },
    "test-helpers" => {
        enabled: cfg!(feature = "test-helpers"),
        category: MarkerOnly,
        hint: "enable feature 'test-helpers' in Cargo.toml: cargo build --features test-helpers"
    },

    // ── Backend drivers ─────────────────────────────────────────────────────
    "db-pentest-mssql-tiberius" => {
        enabled: cfg!(feature = "db-pentest-mssql-tiberius"),
        category: BackendDriver,
        hint: "enable feature 'db-pentest-mssql-tiberius' in Cargo.toml: cargo build --features db-pentest-mssql-tiberius"
    },
    "db-pentest-mongodb" => {
        enabled: cfg!(feature = "db-pentest-mongodb"),
        category: BackendDriver,
        hint: "enable feature 'db-pentest-mongodb' in Cargo.toml: cargo build --features db-pentest-mongodb"
    },
    "db-pentest-redis" => {
        enabled: cfg!(feature = "db-pentest-redis"),
        category: BackendDriver,
        hint: "enable feature 'db-pentest-redis' in Cargo.toml: cargo build --features db-pentest-redis"
    },

    // ── Platform-sensitive features ─────────────────────────────────────────
    "stress-testing" => {
        enabled: cfg!(feature = "stress-testing"),
        category: PlatformSensitive,
        hint: "enable feature 'stress-testing' in Cargo.toml: cargo build --features stress-testing"
    },
    "packet-inspection" => {
        enabled: cfg!(feature = "packet-inspection"),
        category: PlatformSensitive,
        hint: "enable feature 'packet-inspection' in Cargo.toml: cargo build --features packet-inspection"
    },
    "nse-ssh2" => {
        enabled: cfg!(feature = "nse-ssh2"),
        category: PlatformSensitive,
        hint: "enable feature 'nse-ssh2' in Cargo.toml: cargo build --features nse-ssh2"
    },
    "nse-sandbox" => {
        enabled: cfg!(feature = "nse-sandbox"),
        category: PlatformSensitive,
        hint: "enable feature 'nse-sandbox' in Cargo.toml: cargo build --features nse-sandbox"
    },
    "headless-browser" => {
        enabled: cfg!(feature = "headless-browser"),
        category: PlatformSensitive,
        hint: "enable feature 'headless-browser' in Cargo.toml: cargo build --features headless-browser"
    },

    // ── Storage/output integrations ─────────────────────────────────────────
    "database" => {
        enabled: cfg!(feature = "database"),
        category: StorageIntegration,
        hint: "enable feature 'database' in Cargo.toml: cargo build --features database"
    },
    "sbom" => {
        enabled: cfg!(feature = "sbom"),
        category: StorageIntegration,
        hint: "enable feature 'sbom' in Cargo.toml: cargo build --features sbom"
    },
    "container" => {
        enabled: cfg!(feature = "container"),
        category: StorageIntegration,
        hint: "enable feature 'container' in Cargo.toml: cargo build --features container"
    },
    "pdf" => {
        enabled: cfg!(feature = "pdf"),
        category: StorageIntegration,
        hint: "enable feature 'pdf' in Cargo.toml: cargo build --features pdf"
    },

    // ── Security risk features ──────────────────────────────────────────────
    "insecure-tls" => {
        enabled: cfg!(feature = "insecure-tls"),
        category: SecurityRisk,
        hint: "enable feature 'insecure-tls' in Cargo.toml: cargo build --features insecure-tls"
    },

    // ── AI integration ──────────────────────────────────────────────────────
    "ai-integration" => {
        enabled: cfg!(feature = "ai-integration"),
        category: AiIntegration,
        hint: "enable feature 'ai-integration' in Cargo.toml: cargo build --features ai-integration"
    },

    // ── Advanced extensions ─────────────────────────────────────────────────
    "mobile-dynamic" => {
        enabled: cfg!(feature = "mobile-dynamic"),
        category: AdvancedExtension,
        hint: "enable feature 'mobile-dynamic' in Cargo.toml (requires 'mobile' first): cargo build --features mobile-dynamic"
    },
    "wireless-advanced" => {
        enabled: cfg!(feature = "wireless-advanced"),
        category: AdvancedExtension,
        hint: "enable feature 'wireless-advanced' in Cargo.toml (requires 'wireless' first): cargo build --features wireless-advanced"
    },
    "transparent-proxy" => {
        enabled: cfg!(feature = "transparent-proxy"),
        category: AdvancedExtension,
        hint: "enable feature 'transparent-proxy' in Cargo.toml: cargo build --features transparent-proxy"
    },
    "dynamic-plugins" => {
        enabled: cfg!(feature = "dynamic-plugins"),
        category: AdvancedExtension,
        hint: "enable feature 'dynamic-plugins' in Cargo.toml: cargo build --features dynamic-plugins"
    },

    // ── Process-host adapter features ───────────────────────────────────────
    "cli" => {
        enabled: cfg!(feature = "cli"),
        category: ProtocolAdapter,
        hint: "enable feature 'cli' in Cargo.toml: cargo build --features cli"
    },
    "email-notifications" => {
        enabled: cfg!(feature = "email-notifications"),
        category: ProtocolAdapter,
        hint: "enable feature 'email-notifications' in Cargo.toml: cargo build --features email-notifications"
    },
    "logging-subscriber" => {
        enabled: cfg!(feature = "logging-subscriber"),
        category: ProtocolAdapter,
        hint: "enable feature 'logging-subscriber' in Cargo.toml: cargo build --features logging-subscriber"
    },
    "config-watch" => {
        enabled: cfg!(feature = "config-watch"),
        category: ProtocolAdapter,
        hint: "enable feature 'config-watch' in Cargo.toml: cargo build --features config-watch"
    },

    // ── Aggregate features ──────────────────────────────────────────────────
    "full" => {
        enabled: cfg!(feature = "full"),
        category: Aggregate,
        hint: "enable feature 'full' in Cargo.toml: cargo build --features full"
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_all_cargo_features() {
        // Parse Cargo.toml to get declared features
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let manifest_str =
            std::fs::read_to_string(&manifest_path).expect("failed to read Cargo.toml");
        let manifest: toml::Value = manifest_str.parse().expect("failed to parse Cargo.toml");

        let features_table = manifest
            .get("features")
            .and_then(|v| v.as_table())
            .expect("no [features] table in Cargo.toml");

        let excluded = ["default"];

        // Every Cargo feature (except excluded) must be in the registry
        for cargo_feat in features_table.keys() {
            if excluded.contains(&cargo_feat.as_str()) {
                continue;
            }
            assert!(
                is_known_feature(cargo_feat),
                "Cargo.toml feature '{}' not in feature registry — add it to feature_registry!",
                cargo_feat
            );
        }

        // Every registry feature must exist in Cargo.toml
        for entry in ALL_FEATURES {
            assert!(
                features_table.contains_key(entry.name),
                "Registry feature '{}' not found in Cargo.toml [features] — remove from registry or add to Cargo.toml",
                entry.name
            );
        }
    }

    #[test]
    fn all_features_are_classified() {
        for entry in ALL_FEATURES {
            assert!(
                classify_feature(entry.name).is_some(),
                "feature '{}' has no classification",
                entry.name
            );
        }
    }

    #[test]
    fn unknown_feature_returns_unknown() {
        assert_eq!(feature_state("nonexistent-feature"), FeatureState::Unknown);
        assert!(!is_feature_enabled("nonexistent-feature"));
        assert!(!is_known_feature("nonexistent-feature"));
        assert_eq!(feature_missing_hint("nonexistent-feature"), None);
    }

    #[test]
    fn feature_categories_match() {
        // Verify specific category assignments for key features
        assert_eq!(
            classify_feature("db-pentest"),
            Some(FeatureCategory::DomainCapability)
        );
        assert_eq!(
            classify_feature("rest-api"),
            Some(FeatureCategory::ProtocolAdapter)
        );
        assert_eq!(
            classify_feature("db-pentest-mcp"),
            Some(FeatureCategory::ProtocolExposure)
        );
        assert_eq!(classify_feature("full"), Some(FeatureCategory::Aggregate));
        assert_eq!(
            classify_feature("insecure-tls"),
            Some(FeatureCategory::SecurityRisk)
        );
    }
}
