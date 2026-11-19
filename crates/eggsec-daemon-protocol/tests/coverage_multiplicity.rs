//! The published coverage model, pinned mechanically.
//!
//! The desktop GUI renders one row per published task kind, so the published
//! description must keep a known shape: a known number of task kinds, a known
//! number of distinct operations behind them, and a known number of wire
//! shapes per operation.
//!
//! # Why the multiplicities and not the names
//!
//! A task kind is a *wire shape*, not an operation: several wire shapes share
//! one operation, and the client's coverage model has to know both counts to
//! stay honest. Transcribing the catalog here would create a second source of
//! truth that drifts silently, so this file asserts counts and consistency
//! derived from the published payload at runtime, and pins only the
//! multiplicity map itself.
//!
//! Nothing here participates in authorization or dispatch: it reads the same
//! published, read-only description a client would receive.

use std::collections::{BTreeMap, BTreeSet};

use eggsec_daemon_protocol::operations::describe_operations;

/// The canonical number of published task kinds (wire shapes).
const EXPECTED_TASK_KIND_COUNT: usize = 30;

/// The canonical number of published catalog operations.
const EXPECTED_OPERATION_COUNT: usize = 35;

/// The canonical number of distinct operations the task kinds resolve to.
///
/// Fewer than [`EXPECTED_TASK_KIND_COUNT`] because several wire shapes share
/// one operation.
const EXPECTED_DISTINCT_OPERATION_ID_COUNT: usize = 28;

/// The canonical number of distinct capability names the task kinds advertise.
///
/// One per wire shape: a capability name identifies a task kind, not an
/// operation, so this count must equal [`EXPECTED_TASK_KIND_COUNT`].
const EXPECTED_DISTINCT_CAPABILITY_NAME_COUNT: usize = 30;

/// The number of operations that exactly one wire shape resolves to.
const EXPECTED_SINGLE_SHAPE_OPERATION_COUNT: usize = 26;

/// The number of operations that exactly two wire shapes resolve to.
const EXPECTED_TWO_SHAPE_OPERATION_COUNT: usize = 2;

/// The number of operations that exactly three wire shapes resolve to.
///
/// None: the packet family used to be the only one with three wire shapes,
/// and the capture's split into its own `packet-capture` operation left
/// `packet` with the two shapes that address a remote host.
const EXPECTED_THREE_SHAPE_OPERATION_COUNT: usize = 0;

/// The pinned multiplicity map: how many wire shapes resolve to each
/// operation.
///
/// This is the one transcribed thing in this file, and it is the point of the
/// file: a silent change to the multiplicities — a task kind added, dropped,
/// or repointed at another operation — changes this map and fails the test.
/// The wire-shape names behind each count are deliberately absent: they are
/// read from the published payload, never restated here.
const EXPECTED_OPERATION_MULTIPLICITIES: &[(&str, usize)] = &[
    ("auth-test", 1),
    ("browser", 1),
    ("c2", 1),
    ("compliance", 1),
    ("db-pentest", 1),
    ("fingerprint", 1),
    ("fuzz", 1),
    ("graphql", 1),
    ("hunt", 1),
    ("integrations", 1),
    ("load-test", 1),
    ("nse", 1),
    ("oauth", 1),
    ("packet", 2),
    ("packet-capture", 1),
    ("pipeline", 2),
    ("proxy-intercept", 1),
    ("recon", 1),
    ("scan-endpoints", 1),
    ("scan-ports", 1),
    ("storage", 1),
    ("stress-test", 1),
    ("vuln", 1),
    ("waf-detect", 1),
    ("waf-stress", 1),
    ("wireless", 1),
    ("wireless-deauth", 1),
    ("workflow", 1),
];

/// Count the wire shapes that resolve to each operation.
///
/// The map is derived from the published payload, never transcribed.
fn operation_multiplicities() -> BTreeMap<String, usize> {
    let response = describe_operations();
    let mut multiplicities: BTreeMap<String, usize> = BTreeMap::new();
    for task_kind in &response.task_kinds {
        *multiplicities
            .entry(task_kind.operation_id.clone())
            .or_insert(0) += 1;
    }
    multiplicities
}

/// The published description carries the canonical number of wire shapes and
/// the canonical number of catalog operations.
#[test]
fn published_coverage_carries_the_canonical_counts() {
    let response = describe_operations();
    assert_eq!(
        response.task_kinds.len(),
        EXPECTED_TASK_KIND_COUNT,
        "the published task-kind count changed"
    );
    assert_eq!(
        response.operations.len(),
        EXPECTED_OPERATION_COUNT,
        "the published catalog operation count changed"
    );
}

/// The task kinds resolve to exactly the canonical number of distinct
/// operations, and each operation's multiplicity is the pinned one.
///
/// This is the assertion that catches a task kind being added, dropped, or
/// repointed at a different operation.
#[test]
fn every_operation_keeps_its_pinned_wire_shape_multiplicity() {
    let multiplicities = operation_multiplicities();
    assert_eq!(
        multiplicities.len(),
        EXPECTED_DISTINCT_OPERATION_ID_COUNT,
        "the number of distinct operations behind the task kinds changed"
    );

    let expected: BTreeMap<String, usize> = EXPECTED_OPERATION_MULTIPLICITIES
        .iter()
        .map(|(id, count)| ((*id).to_owned(), *count))
        .collect();
    assert_eq!(
        multiplicities, expected,
        "the wire-shape multiplicity map changed"
    );

    assert_eq!(
        multiplicities.values().filter(|count| **count == 1).count(),
        EXPECTED_SINGLE_SHAPE_OPERATION_COUNT
    );
    assert_eq!(
        multiplicities.values().filter(|count| **count == 2).count(),
        EXPECTED_TWO_SHAPE_OPERATION_COUNT
    );
    assert_eq!(
        multiplicities.values().filter(|count| **count == 3).count(),
        EXPECTED_THREE_SHAPE_OPERATION_COUNT
    );
}

/// The multiplicities sum back to the canonical task-kind count, so the map
/// cannot lose or gain a wire shape without the counts noticing.
#[test]
fn the_multiplicities_sum_to_the_task_kind_count() {
    let multiplicities = operation_multiplicities();
    let total: usize = multiplicities.values().sum();
    assert_eq!(total, EXPECTED_TASK_KIND_COUNT);
}

/// Several wire shapes share one operation, and a client rendering one row
/// per wire shape must be able to tell the two apart: the shared operations
/// are exactly the ones with more than one wire shape.
#[test]
fn shared_operations_are_exactly_the_ones_with_several_wire_shapes() {
    let multiplicities = operation_multiplicities();
    let shared: Vec<&str> = multiplicities
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(id, _)| id.as_str())
        .collect();
    assert_eq!(
        shared,
        vec!["packet", "pipeline"],
        "the set of operations shared by several wire shapes changed"
    );
}

/// Every task kind names an operation that the same payload publishes, so a
/// client can look the operation's risk and exposure up without a second
/// call.
#[test]
fn every_task_kind_resolves_to_a_published_operation() {
    let response = describe_operations();
    let published: BTreeSet<&str> = response
        .operations
        .iter()
        .map(|operation| operation.id.as_str())
        .collect();
    for task_kind in &response.task_kinds {
        assert!(
            published.contains(task_kind.operation_id.as_str()),
            "{} names operation {}, which the same payload does not publish",
            task_kind.variant,
            task_kind.operation_id
        );
    }
}

/// Every task kind advertises a non-empty capability name, and the capability
/// names are one-per-wire-shape: a capability name identifies a task kind, so
/// two task kinds sharing one would make the client's coverage rows collide.
#[test]
fn every_task_kind_advertises_a_distinct_non_empty_capability_name() {
    let response = describe_operations();
    let mut capability_names: BTreeSet<&str> = BTreeSet::new();
    for task_kind in &response.task_kinds {
        assert!(
            !task_kind.capability_name.trim().is_empty(),
            "{} advertises an empty capability name",
            task_kind.variant
        );
        assert!(
            capability_names.insert(task_kind.capability_name.as_str()),
            "{} advertises capability name {}, which another task kind already uses",
            task_kind.variant,
            task_kind.capability_name
        );
    }
    assert_eq!(
        capability_names.len(),
        EXPECTED_DISTINCT_CAPABILITY_NAME_COUNT
    );
}

/// Every task kind carries a non-empty request schema, so a client can render
/// a form for it without a second call.
#[test]
fn every_task_kind_carries_a_non_empty_request_schema() {
    let response = describe_operations();
    for task_kind in &response.task_kinds {
        let schema = &task_kind.request_schema;
        assert!(
            schema.is_object(),
            "{} carries a request schema that is not an object",
            task_kind.variant
        );
        assert!(
            !schema.as_object().expect("an object").is_empty(),
            "{} carries an empty request schema",
            task_kind.variant
        );
    }
}
