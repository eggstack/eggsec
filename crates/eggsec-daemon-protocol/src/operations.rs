//! Read-only publication of the canonical operation catalog and the
//! mechanically derived request schema (`ADR-0008`).
//!
//! # One source of truth
//!
//! The identity and policy fields in [`OperationDescription`] are read from
//! `eggsec_policy::ALL_OPERATION_METADATA` — the canonical catalog — and are
//! never transcribed here. The parameter structure in
//! [`TaskKindDescription`] is derived from the canonical request DTOs by
//! `eggsec_runtime::schema`, never hand-written.
//!
//! # Descriptive, not authoritative
//!
//! Nothing in this module participates in authorization, dispatch, scope, or
//! policy evaluation. It is a published *view* of the operation set, for
//! frontends that need to render a form for an operation they can already
//! invoke. A client's view of an operation never grants, narrows, or widens
//! what the engine will run: the engine's own enforcement remains
//! authoritative, and engine-side validation remains authoritative for the
//! contents of any field.

use eggsec_policy::catalog::OperationMetadata;
use eggsec_policy::{Capability, IntendedUse, OperationMode, OperationRisk};
use serde::{Deserialize, Serialize};

/// Wire description of one canonical operation.
///
/// Every field is read from [`OperationMetadata`]; see the module docs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationDescription {
    /// Canonical operation id, e.g. `scan-ports`.
    pub id: String,
    /// Human-readable name, e.g. `Port Scan`.
    pub display_name: String,
    /// Assessment mode the operation belongs to.
    pub mode: OperationMode,
    /// Declared risk tier.
    pub risk: OperationRisk,
    /// Declared intended uses.
    pub intended_uses: Vec<IntendedUse>,
    /// Engine features the operation requires.
    pub required_features: Vec<String>,
    /// Policy flags the operation requires.
    pub required_policy_flags: Vec<String>,
    /// Runtime capabilities the operation requires.
    pub required_capabilities: Vec<Capability>,
    /// Target policy requirement, as the catalog's canonical label
    /// (`no-target`, `target-required`, `explicit-scope-required`, ...).
    pub target_policy: String,
    /// Whether the operation is exposed on the manual surface.
    pub manual_exposable: bool,
    /// Whether the operation is exposed on the TUI surface.
    pub tui_exposable: bool,
    /// Whether the operation is exposed over MCP.
    pub mcp_exposable: bool,
    /// Whether the operation is exposed over REST.
    pub rest_exposable: bool,
    /// Whether the operation is exposed to the security agent.
    pub agent_exposable: bool,
    /// Whether the operation is exposed over gRPC.
    pub grpc_exposable: bool,
}

/// Wire description of one task-kind wire shape and its request schema.
///
/// A task kind is a wire shape, not an operation: the three packet kinds
/// share the `packet` operation, `Pipeline` and `Resume` share `pipeline`, and
/// `Wireless` and `WirelessActive` share `wireless`. A client renders one
/// form per task kind and looks the operation's risk and exposure up by
/// [`Self::operation_id`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskKindDescription {
    /// The wire `kind` discriminator, e.g. `PortScan`.
    pub variant: String,
    /// Canonical operation id this task kind executes.
    pub operation_id: String,
    /// Wire capability name this task kind advertises.
    pub capability_name: String,
    /// JSON Schema for the task kind's params struct.
    pub request_schema: serde_json::Value,
}

/// Response payload for `ClientCommand::DescribeOperations`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescribeOperationsResponse {
    /// Every canonical operation in the catalog.
    pub operations: Vec<OperationDescription>,
    /// Every task-kind wire shape, with its request schema.
    pub task_kinds: Vec<TaskKindDescription>,
    /// The whole `TaskKind` schema in its adjacently-tagged wire shape, so a
    /// client can post `{"kind": ..., "params": ...}` without guessing the
    /// envelope.
    pub task_kind_schema: serde_json::Value,
}

/// Build the describe-operations payload from the canonical sources.
///
/// Read-only and side-effect free: it touches no session, task, or registry
/// state, so it is safe to call concurrently.
pub fn describe_operations() -> DescribeOperationsResponse {
    DescribeOperationsResponse {
        operations: eggsec_policy::ALL_OPERATION_METADATA
            .iter()
            .map(describe_operation)
            .collect(),
        task_kinds: eggsec_runtime::schema::task_kind_schemas()
            .into_iter()
            .map(|entry| TaskKindDescription {
                variant: entry.variant,
                operation_id: entry.operation_id,
                capability_name: entry.capability_name,
                request_schema: entry.schema,
            })
            .collect(),
        task_kind_schema: eggsec_runtime::schema::task_kind_schema(),
    }
}

/// Describe one catalog entry.
fn describe_operation(metadata: &OperationMetadata) -> OperationDescription {
    OperationDescription {
        id: metadata.id.to_owned(),
        display_name: metadata.display_name.to_owned(),
        mode: metadata.mode,
        risk: metadata.risk,
        intended_uses: metadata.intended_uses.to_vec(),
        required_features: metadata
            .required_features
            .iter()
            .map(|feature| (*feature).to_owned())
            .collect(),
        required_policy_flags: metadata
            .required_policy_flags
            .iter()
            .map(|flag| (*flag).to_owned())
            .collect(),
        required_capabilities: metadata.required_capabilities.to_vec(),
        target_policy: metadata.target_policy.to_string(),
        manual_exposable: metadata.manual_exposable,
        tui_exposable: metadata.tui_exposable,
        mcp_exposable: metadata.mcp_exposable,
        rest_exposable: metadata.rest_exposable,
        agent_exposable: metadata.agent_exposable,
        grpc_exposable: metadata.grpc_exposable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_catalog_operation_is_described() {
        let response = describe_operations();
        assert_eq!(
            response.operations.len(),
            eggsec_policy::ALL_OPERATION_METADATA.len()
        );
    }

    /// The published identity and policy fields must match the catalog field
    /// for field. If this fails, the response was transcribed rather than
    /// read from the catalog.
    #[test]
    fn published_operations_match_the_catalog_field_for_field() {
        let response = describe_operations();
        for metadata in eggsec_policy::ALL_OPERATION_METADATA {
            let published = response
                .operations
                .iter()
                .find(|operation| operation.id == metadata.id)
                .expect("every catalog operation must be published");
            assert_eq!(published.display_name, metadata.display_name);
            assert_eq!(published.mode, metadata.mode);
            assert_eq!(published.risk, metadata.risk);
            assert_eq!(published.intended_uses, metadata.intended_uses);
            assert_eq!(
                published.required_features,
                metadata
                    .required_features
                    .iter()
                    .map(|feature| (*feature).to_owned())
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                published.required_policy_flags,
                metadata
                    .required_policy_flags
                    .iter()
                    .map(|flag| (*flag).to_owned())
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                published.required_capabilities,
                metadata.required_capabilities
            );
            assert_eq!(published.target_policy, metadata.target_policy.to_string());
            assert_eq!(published.manual_exposable, metadata.manual_exposable);
            assert_eq!(published.tui_exposable, metadata.tui_exposable);
            assert_eq!(published.mcp_exposable, metadata.mcp_exposable);
            assert_eq!(published.rest_exposable, metadata.rest_exposable);
            assert_eq!(published.agent_exposable, metadata.agent_exposable);
            assert_eq!(published.grpc_exposable, metadata.grpc_exposable);
        }
    }

    /// Every published task kind must name a canonical operation.
    #[test]
    fn published_task_kinds_name_canonical_operations() {
        let response = describe_operations();
        assert_eq!(response.task_kinds.len(), 30);
        for task_kind in &response.task_kinds {
            assert!(
                response
                    .operations
                    .iter()
                    .any(|operation| operation.id == task_kind.operation_id),
                "{} names operation {}, which is not in the catalog",
                task_kind.variant,
                task_kind.operation_id
            );
        }
    }

    /// The response carries operation identity, policy metadata, and
    /// parameter structure — and nothing else.
    ///
    /// A credential, a secret, or a scope rule would have to arrive either as
    /// an extra field on the metadata half or as an extra value on the
    /// parameter half. The first is ruled out by the closed field set; the
    /// second by the fact that the parameter half is a JSON Schema — property
    /// names, types, and the serde defaults of the request structs — which
    /// contains no request values at all.
    #[test]
    fn response_carries_no_credential_or_scope_material() {
        const ALLOWED_OPERATION_FIELDS: [&str; 15] = [
            "id",
            "display_name",
            "mode",
            "risk",
            "intended_uses",
            "required_features",
            "required_policy_flags",
            "required_capabilities",
            "target_policy",
            "manual_exposable",
            "tui_exposable",
            "mcp_exposable",
            "rest_exposable",
            "agent_exposable",
            "grpc_exposable",
        ];

        let response = describe_operations();
        for operation in &response.operations {
            let json = serde_json::to_value(operation).expect("serializable");
            let object = json.as_object().expect("an object");
            assert_eq!(
                object.len(),
                ALLOWED_OPERATION_FIELDS.len(),
                "the published operation description carries an unexpected field"
            );
            for key in object.keys() {
                assert!(
                    ALLOWED_OPERATION_FIELDS.contains(&key.as_str()),
                    "unexpected published field: {key}"
                );
            }
        }

        // No scope rule, secret, or bearer token is published. The canonical
        // catalog and the request DTOs contain no such material, so a
        // published artifact cannot introduce it.
        let serialized = serde_json::to_string(&response).expect("serializable");
        for forbidden in [
            "\"cidr\"",
            "\"scope_rules\"",
            "\"api_key\"",
            "\"secret\"",
            "\"bearer\"",
        ] {
            assert!(
                !serialized.contains(forbidden),
                "the published response must not contain {forbidden} material"
            );
        }
    }

    /// The payload is read-only: two calls produce equivalent results.
    #[test]
    fn payload_is_reproducible() {
        let first = describe_operations();
        let second = describe_operations();
        assert_eq!(
            serde_json::to_value(&first).expect("serializable"),
            serde_json::to_value(&second).expect("serializable")
        );
    }
}
