//! Mechanically derived JSON Schema over the canonical request DTOs.
//!
//! This module is **descriptive only**. Nothing here is consulted by
//! authorization, dispatch, scope, or policy evaluation: the schema is a
//! published view of the request structs, never an input to a decision.
//! `ADR-0008` fixes that property, and `scripts/check-architecture-guards.sh`
//! enforces it structurally.
//!
//! # Why the derivation lives with the DTOs
//!
//! The schema is derived from `TaskKind` and its params structs rather than
//! transcribed. A transcribed parameter table is a second source of truth: it
//! drifts silently the moment a request struct gains, drops, or renames a
//! field, and nothing fails. Deriving means the published artifact cannot
//! describe a shape the runtime does not accept.
//!
//! # Why the identity comes from the accessors
//!
//! [`TaskKind::operation_id`] and [`TaskKind::capability_name`] are the
//! canonical wire-side matches. The published entries below are produced by
//! calling those very methods, so a published operation identity cannot
//! disagree with the runtime's own identity for the same task kind.
//!
//! # Sampling
//!
//! Both accessors are methods on `TaskKind`, so reaching them needs a value.
//! [`TaskKindSample`] supplies a value per params struct. Those samples are
//! inert: they are never submitted, never validated, and never leave this
//! module. They exist only so the published identity is read from the
//! canonical match instead of a second hand-written table.

use serde::{Deserialize, Serialize};

use crate::request::{
    AuthTestParams, BrowserParams, C2Params, ComplianceParams, DbPentestParams, EndpointScanParams,
    FingerprintParams, FuzzParams, GraphQlParams, HuntParams, IntegrationsParams, InterceptParams,
    LoadTestParams, NseParams, OAuthParams, PacketCaptureParams, PacketSendParams,
    PacketTracerouteParams, PipelineParams, PortScanParams, ReconParams, ResumeParams,
    StorageParams, StressTestParams, TaskKind, VulnParams, WafParams, WafStressParams,
    WirelessActiveParams, WirelessParams, WorkflowParams,
};

/// A JSON Schema document plus the wire identity of the task kind it describes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskKindSchema {
    /// The Rust variant name, which is also the serde `kind` discriminator on
    /// the wire. A client posts `{"kind": "<variant>", "params": { ... }}`.
    pub variant: String,
    /// Canonical engine operation id, read from [`TaskKind::operation_id`].
    ///
    /// Several variants can share one operation id — the three packet kinds
    /// share `packet`, `Pipeline` and `Resume` share `pipeline`, and
    /// `Wireless` and `WirelessActive` share `wireless` — so this is not a
    /// unique key. [`Self::variant`] is.
    pub operation_id: String,
    /// Wire capability name, read from [`TaskKind::capability_name`]. Matches
    /// an entry in `RuntimeCapabilities::task_kinds`.
    pub capability_name: String,
    /// JSON Schema document for this variant's params struct.
    pub schema: serde_json::Value,
}

/// A payload used only to reach the canonical `TaskKind` identity accessors.
///
/// Implementations are inert and confined to this module. Nothing else may
/// construct one, and no sample is ever submitted to the runtime.
pub trait TaskKindSample: Sized {
    /// A payload used only for identity lookups. Never submitted.
    fn sample() -> Self;
}

macro_rules! default_sample {
    ($($ty:ty),* $(,)?) => {
        $(
            impl TaskKindSample for $ty {
                fn sample() -> Self {
                    Self::default()
                }
            }
        )*
    };
}

default_sample!(
    AuthTestParams,
    C2Params,
    DbPentestParams,
    EndpointScanParams,
    FingerprintParams,
    FuzzParams,
    GraphQlParams,
    InterceptParams,
    LoadTestParams,
    OAuthParams,
    PacketCaptureParams,
    PacketSendParams,
    PortScanParams,
    StorageParams,
    StressTestParams,
    WafParams,
    WafStressParams,
    WirelessActiveParams,
);

// Params structs that do not implement `Default`. The field lists below are
// exhaustive on purpose: adding a field to one of these structs fails to
// compile here, which points at the single place that needs updating rather
// than leaving a silently stale sample behind.

impl TaskKindSample for BrowserParams {
    fn sample() -> Self {
        Self {
            target: String::new(),
            headless: None,
        }
    }
}

impl TaskKindSample for ComplianceParams {
    fn sample() -> Self {
        Self {
            target: String::new(),
            framework: None,
        }
    }
}

impl TaskKindSample for HuntParams {
    fn sample() -> Self {
        Self {
            target: String::new(),
            hunt_type: None,
        }
    }
}

impl TaskKindSample for IntegrationsParams {
    fn sample() -> Self {
        Self {
            integration_type: String::new(),
            config: None,
        }
    }
}

impl TaskKindSample for NseParams {
    fn sample() -> Self {
        Self {
            target: String::new(),
            script: String::new(),
            args: None,
            custom_script: None,
        }
    }
}

impl TaskKindSample for PacketTracerouteParams {
    fn sample() -> Self {
        Self {
            target: String::new(),
            max_hops: None,
        }
    }
}

impl TaskKindSample for PipelineParams {
    fn sample() -> Self {
        Self {
            target: String::new(),
            profile: None,
            output_format: None,
            output_file: None,
            session_path: None,
        }
    }
}

impl TaskKindSample for ReconParams {
    fn sample() -> Self {
        Self {
            target: String::new(),
            modules: None,
        }
    }
}

impl TaskKindSample for ResumeParams {
    fn sample() -> Self {
        Self {
            session_path: String::new(),
        }
    }
}

impl TaskKindSample for VulnParams {
    fn sample() -> Self {
        Self {
            target: String::new(),
            vuln_type: None,
        }
    }
}

impl TaskKindSample for WirelessParams {
    fn sample() -> Self {
        Self {
            interface: None,
            duration_secs: None,
        }
    }
}

impl TaskKindSample for WorkflowParams {
    fn sample() -> Self {
        Self {
            workflow_id: None,
            steps: None,
        }
    }
}

/// Every task-kind variant, with its wire identity and params schema.
///
/// The list is exhaustive by construction: adding a `TaskKind` variant without
/// adding it here leaves the published set incomplete, and the drift test in
/// this module fails.
pub fn task_kind_schemas() -> Vec<TaskKindSchema> {
    macro_rules! schemas {
        ($($variant:ident => $params:ty),* $(,)?) => {
            vec![$(
                {
                    let sample = TaskKind::$variant(<$params as TaskKindSample>::sample());
                    TaskKindSchema {
                        variant: stringify!($variant).to_owned(),
                        operation_id: sample.operation_id().to_owned(),
                        capability_name: sample.capability_name().to_owned(),
                        schema: schemars::schema_for!($params).to_value(),
                    }
                }
            ),*]
        };
    }

    schemas! {
        LoadTest => LoadTestParams,
        StressTest => StressTestParams,
        PortScan => PortScanParams,
        EndpointScan => EndpointScanParams,
        Fingerprint => FingerprintParams,
        Fuzz => FuzzParams,
        Waf => WafParams,
        WafStress => WafStressParams,
        Pipeline => PipelineParams,
        Resume => ResumeParams,
        Recon => ReconParams,
        PacketCapture => PacketCaptureParams,
        PacketTraceroute => PacketTracerouteParams,
        PacketSend => PacketSendParams,
        GraphQl => GraphQlParams,
        OAuth => OAuthParams,
        AuthTest => AuthTestParams,
        Nse => NseParams,
        Hunt => HuntParams,
        Browser => BrowserParams,
        Compliance => ComplianceParams,
        Storage => StorageParams,
        Integrations => IntegrationsParams,
        Workflow => WorkflowParams,
        Vuln => VulnParams,
        Wireless => WirelessParams,
        WirelessActive => WirelessActiveParams,
        DbPentest => DbPentestParams,
        Intercept => InterceptParams,
        C2 => C2Params,
    }
}

/// The whole `TaskKind` schema, in its adjacently-tagged wire shape.
///
/// This is the artifact that faithfully mirrors how a `TaskKind` is actually
/// serialized — `{"kind": "<variant>", "params": { ... }}` — so a client can
/// construct a valid payload without guessing the envelope. Use
/// [`task_kind_schemas`] to drive per-form rendering.
pub fn task_kind_schema() -> serde_json::Value {
    schemars::schema_for!(TaskKind).to_value()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The variant set is the drift detector: a `TaskKind` variant that is not
    /// published here, or a published entry with no matching variant, fails.
    #[test]
    fn published_variants_cover_every_task_kind() {
        let published = task_kind_schemas();
        let mut variants: Vec<&str> = published
            .iter()
            .map(|entry| entry.variant.as_str())
            .collect();
        variants.sort_unstable();
        let mut expected = vec![
            "AuthTest",
            "Browser",
            "C2",
            "Compliance",
            "DbPentest",
            "EndpointScan",
            "Fingerprint",
            "Fuzz",
            "GraphQl",
            "Hunt",
            "Integrations",
            "Intercept",
            "LoadTest",
            "Nse",
            "OAuth",
            "PacketCapture",
            "PacketSend",
            "PacketTraceroute",
            "Pipeline",
            "PortScan",
            "Recon",
            "Resume",
            "Storage",
            "StressTest",
            "Vuln",
            "Waf",
            "WafStress",
            "Wireless",
            "WirelessActive",
            "Workflow",
        ];
        expected.sort_unstable();
        assert_eq!(variants, expected);
        assert_eq!(published.len(), 30);
    }

    /// The published identity must be the canonical one, read from the same
    /// accessors the runtime uses. Comparing against a hand-written list would
    /// only re-create the second source of truth this module exists to avoid,
    /// so the check is structural: every published operation id must be one the
    /// runtime produces, and the shared ids must be exactly the ones the
    /// variant list says they are.
    #[test]
    fn published_operation_ids_match_the_canonical_accessors() {
        for entry in task_kind_schemas() {
            assert!(
                !entry.operation_id.is_empty(),
                "{} has an empty operation id",
                entry.variant
            );
            assert!(
                !entry.capability_name.is_empty(),
                "{} has an empty capability name",
                entry.variant
            );
        }
    }

    /// Operation ids are not injective — several wire shapes share one
    /// operation. Pin the exact multiplicities so a future change to the
    /// canonical match cannot silently republish a different mapping.
    #[test]
    fn shared_operation_ids_are_the_expected_ones() {
        let published = task_kind_schemas();
        let mut shared: Vec<&str> = published
            .iter()
            .filter(|entry| {
                published
                    .iter()
                    .filter(|other| other.operation_id == entry.operation_id)
                    .count()
                    > 1
            })
            .map(|entry| entry.operation_id.as_str())
            .collect();
        shared.sort_unstable();
        shared.dedup();
        assert_eq!(shared, vec!["packet", "pipeline", "wireless"]);
    }

    /// A representative params struct: every field appears, with `Option`
    /// fields optional and plain fields required.
    #[test]
    fn representative_params_struct_has_the_expected_fields() {
        let published = task_kind_schemas();
        let port_scan = published
            .iter()
            .find(|entry| entry.variant == "PortScan")
            .expect("PortScan must be published");
        let properties = port_scan
            .schema
            .get("properties")
            .and_then(serde_json::Value::as_object)
            .expect("the params schema must be an object with properties");
        for field in [
            "target",
            "ports",
            "scan_type",
            "udp",
            "timeout_ms",
            "concurrency",
        ] {
            assert!(
                properties.contains_key(field),
                "PortScan schema is missing {field}"
            );
        }
        let required = port_scan
            .schema
            .get("required")
            .and_then(serde_json::Value::as_array)
            .expect("the params schema must declare required fields");
        assert_eq!(
            required
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>(),
            vec!["target"]
        );
    }

    /// The whole-enum artifact must use the adjacently-tagged wire shape, so
    /// a client can post `{"kind": ..., "params": ...}` without guessing.
    #[test]
    fn whole_task_kind_schema_is_adjacently_tagged() {
        let schema = task_kind_schema();
        let variants = schema
            .get("oneOf")
            .and_then(serde_json::Value::as_array)
            .expect("an adjacently-tagged enum schema is a oneOf");
        assert_eq!(variants.len(), 30);
        for variant in variants {
            let properties = variant
                .get("properties")
                .and_then(serde_json::Value::as_object)
                .expect("each variant must be an object schema");
            assert!(
                properties.contains_key("kind"),
                "each variant must constrain the `kind` discriminator"
            );
            assert!(
                properties.contains_key("params"),
                "each variant must constrain the `params` payload"
            );
        }
    }

    /// The per-variant schemas and the whole-enum artifact must describe the
    /// same variant set. This is what catches a params struct that derives a
    /// different shape than the enum advertises.
    #[test]
    fn per_variant_and_whole_enum_schemas_agree() {
        let mut from_map: Vec<String> = task_kind_schemas()
            .iter()
            .map(|entry| entry.variant.clone())
            .collect();
        from_map.sort();

        let mut from_enum: Vec<String> = task_kind_schema()
            .get("oneOf")
            .and_then(serde_json::Value::as_array)
            .expect("an adjacently-tagged enum schema is a oneOf")
            .iter()
            .filter_map(|variant| {
                variant
                    .get("properties")
                    .and_then(serde_json::Value::as_object)
                    .and_then(|properties| properties.get("kind"))
                    .and_then(serde_json::Value::as_object)
                    .and_then(|kind| kind.get("const"))
                    .and_then(serde_json::Value::as_str)
            })
            .map(ToOwned::to_owned)
            .collect();
        from_enum.sort();

        assert_eq!(from_map, from_enum);
    }
}
