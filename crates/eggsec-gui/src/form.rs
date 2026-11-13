//! The generic form engine.
//!
//! One mapping from a published JSON Schema property to one control
//! descriptor, and one assembly path from control values to a typed
//! [`RunRequest`]. There is no per-operation code anywhere in this module:
//! a task kind the engine has not published yet gets a form with no GUI
//! change at all.
//!
//! # The schema is descriptive, never authoritative
//!
//! The schema drives *form shape* only. It is never consulted to decide
//! whether an operation may run, what its risk is, or which features it
//! needs — those come from the daemon's operation catalog and its live
//! capabilities.
//!
//! # Validation is layered
//!
//! 1. **Advisory**, from the schema: the control's declared type and bounds.
//! 2. **Exact**, from the DTOs: the assembled request is deserialized into
//!    the same [`RunRequest`] the engine will deserialize, so a value of the
//!    wrong type is rejected with the engine's own rules.
//! 3. **Authoritative**, in the engine: the engine's own validation still
//!    decides whether the task may run.
//!
//! Layers one and two exist to save the operator a round trip. They never
//! stand in for layer three, and neither of them encodes a per-operation
//! rule.

use eggsec_daemon_protocol::operations::TaskKindDescription;
use eggsec_runtime::request::{RunRequest, RuntimeSurface};
use serde_json::{Map, Value};

/// The kind of control a form field renders as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ControlKind {
    /// A single-line text input.
    Text,
    /// A numeric input.
    Number,
    /// A tri-state boolean: unset, true, or false.
    Checkbox,
    /// A repeatable list of strings.
    List,
    /// A free-form JSON value.
    Json,
}

/// One form field, derived from one JSON Schema property.
///
/// The descriptor carries everything the webview needs to render the field
/// and nothing it needs to *decide*: the schema's `type` and bounds are
/// already resolved into a control kind and numeric limits.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Control {
    /// The property name — the field's key in the request's `params`.
    pub name: String,
    /// The control the webview renders.
    pub kind: ControlKind,
    /// Whether the schema lists this field as required.
    pub required: bool,
    /// The schema's `description`, when it has one.
    pub description: Option<String>,
    /// The schema's `minimum`, when it has one.
    pub minimum: Option<u64>,
    /// The upper bound derived from the schema's `format`, when it has one.
    pub maximum: Option<u64>,
    /// The schema's `format`, when it has one.
    pub format: Option<String>,
    /// A placeholder hint. The published schema carries no examples, so this
    /// is always `None` today; the field exists so a future schema that does
    /// publish one needs no contract change.
    pub placeholder: Option<String>,
}

/// A form assembly or validation failure.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FormError {
    /// The field the error is about, when the error is field-level.
    pub field: Option<String>,
    /// The message shown to the operator.
    pub message: String,
}

impl FormError {
    /// An error about one field.
    pub fn field(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: Some(name.into()),
            message: message.into(),
        }
    }

    /// An error about the form as a whole.
    pub fn global(message: impl Into<String>) -> Self {
        Self {
            field: None,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for FormError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.field {
            Some(field) => write!(f, "{field}: {}", self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for FormError {}

/// The controls for one task kind.
///
/// One control per schema property. The order is deterministic — required
/// fields first, then alphabetical — so the form's layout does not shift
/// between runs.
pub fn controls_for(task_kind: &TaskKindDescription) -> Vec<Control> {
    let schema = &task_kind.request_schema;
    let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
        return Vec::new();
    };
    let required = required_names(schema);

    let mut controls: Vec<Control> = properties
        .iter()
        .map(|(name, property)| control_for(name, property, required.contains(&name.as_str())))
        .collect();
    controls.sort_by(|left, right| {
        right
            .required
            .cmp(&left.required)
            .then_with(|| left.name.cmp(&right.name))
    });
    controls
}

/// Map one JSON Schema property to one control.
///
/// An unrecognized shape degrades to a control that can still carry the
/// value: an object or a non-string array becomes a JSON text area (a
/// free-text field would be lossy), and an unknown scalar becomes a text
/// input. It never panics and never drops the field.
pub fn control_for(name: &str, property: &Value, required: bool) -> Control {
    let kind = control_kind(property);
    let format = property
        .get("format")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let maximum = match kind {
        ControlKind::Number => maximum_for(format.as_deref()),
        _ => None,
    };

    Control {
        name: name.to_owned(),
        kind,
        required,
        description: property
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_owned),
        minimum: property.get("minimum").and_then(Value::as_u64),
        maximum,
        format,
        placeholder: None,
    }
}

/// The control kind for one property schema.
fn control_kind(property: &Value) -> ControlKind {
    // The unconstrained `true` schema — one field today — accepts any JSON
    // value, so it renders as a JSON text area.
    if !property.is_object() {
        return ControlKind::Json;
    }

    let declared = property.get("type");
    let types: Vec<&str> = match declared {
        Some(Value::String(single)) => vec![single.as_str()],
        Some(Value::Array(items)) => items.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };

    if types.contains(&"array") {
        // A list of strings is a repeatable text control. Any other array —
        // and any object — is rendered as a JSON text area, because a
        // string-list control would silently lose the structure.
        return if array_items_are_strings(property) {
            ControlKind::List
        } else {
            ControlKind::Json
        };
    }

    match types.iter().find(|kind| **kind != "null").copied() {
        Some("string") => ControlKind::Text,
        Some("integer") => ControlKind::Number,
        Some("boolean") => ControlKind::Checkbox,
        // An object is rendered as a JSON text area: a free-text field
        // would lose the structure.
        Some("object") => ControlKind::Json,
        // Documented fallback for a shape the engine has not published
        // before: a free-text input. It never panics and never drops the
        // field.
        _ => ControlKind::Text,
    }
}

/// Whether an array property's items are plain strings.
fn array_items_are_strings(property: &Value) -> bool {
    property
        .get("items")
        .and_then(Value::as_object)
        .and_then(|items| items.get("type"))
        .and_then(Value::as_str)
        .is_some_and(|kind| kind == "string")
}

/// The numeric upper bound implied by a schema `format`.
///
/// The published schema states a `minimum` but never a `maximum`; the
/// upper bound comes from the integer width the format names. `uint` is
/// `usize`, whose width is platform-dependent — the 64-bit width is used
/// here, and the typed deserialization remains the exact authority.
fn maximum_for(format: Option<&str>) -> Option<u64> {
    match format {
        Some("uint16") => Some(u16::MAX as u64),
        Some("uint32") => Some(u32::MAX as u64),
        Some("uint64") | Some("uint") => Some(u64::MAX),
        _ => None,
    }
}

/// The schema's `required` list.
fn required_names(schema: &Value) -> Vec<&str> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map(|names| names.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

/// Whether a property's type accepts `null`.
fn accepts_null(property: &Value) -> bool {
    match property.get("type") {
        Some(Value::String(single)) => single == "null",
        Some(Value::Array(items)) => items.iter().any(|item| item.as_str() == Some("null")),
        _ => false,
    }
}

/// Assemble a typed [`RunRequest`] from the values a form collected.
///
/// `values` maps a property name to the value the operator supplied. A
/// `null` value means *not provided* — never an empty string, a zero, or a
/// `false`. The distinction matters: several published parameters carry
/// meaningful defaults, and one of them is a fail-safe flag that defaults
/// to `true`. A form that sent `false` for an untouched field would turn a
/// simulation into a live action.
///
/// A field that is not required and whose type does not accept `null` is
/// **omitted** from the request rather than sent as `null` or as a zero
/// value, so the request DTO applies its own default.
pub fn assemble_request(
    task_kind: &TaskKindDescription,
    values: &Value,
) -> Result<RunRequest, FormError> {
    let schema = &task_kind.request_schema;
    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| FormError::global("the published request schema has no properties"))?;
    let required = required_names(schema);

    let mut params = Map::new();
    for (name, property) in properties {
        let supplied = values.get(name).cloned().unwrap_or(Value::Null);

        if supplied.is_null() {
            if required.contains(&name.as_str()) {
                return Err(FormError::field(
                    name.clone(),
                    format!("`{name}` is required"),
                ));
            }
            if accepts_null(property) {
                params.insert(name.clone(), Value::Null);
            }
            // Otherwise the field is omitted, so the request DTO applies its
            // own default rather than the GUI overriding it.
            continue;
        }

        check_value(name, property, &supplied)?;
        params.insert(name.clone(), supplied);
    }

    let surface = serde_json::to_value(RuntimeSurface::DesktopGuiManual).map_err(|error| {
        FormError::global(format!(
            "cannot serialize the desktop GUI's runtime surface: {error}"
        ))
    })?;

    // The request's wire shape is `{"task_kind": {"kind": ..., "params": ...},
    // "requested_by": ..., "surface": ..., "labels": ...}`: `TaskKind` is
    // adjacently tagged, and `RunRequest` carries it as a named field. The
    // GUI never constructs a `TaskKind` variant by hand — it builds the
    // tagged JSON and lets the DTO do the typing.
    let request = serde_json::json!({
        "task_kind": {
            "kind": task_kind.variant,
            "params": Value::Object(params),
        },
        "requested_by": Value::Null,
        "surface": surface,
        "labels": Value::Array(Vec::new()),
    });

    serde_json::from_value(request).map_err(|error| {
        let message = error.to_string();
        FormError {
            field: missing_field(&message),
            message,
        }
    })
}

/// Check one supplied value against its property schema.
///
/// This is the advisory layer: it reads the schema's own `type` and
/// `format`, so it encodes no per-operation rule.
fn check_value(name: &str, property: &Value, value: &Value) -> Result<(), FormError> {
    // The unconstrained `true` schema accepts any JSON value.
    if !property.is_object() {
        return Ok(());
    }

    let declared = property.get("type");
    let types: Vec<&str> = match declared {
        Some(Value::String(single)) => vec![single.as_str()],
        Some(Value::Array(items)) => items.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };
    if types.is_empty() {
        // An unknown shape: the typed deserialization is the authority.
        return Ok(());
    }

    if !types.iter().any(|kind| value_matches(kind, value)) {
        return Err(FormError::field(
            name.to_owned(),
            format!(
                "`{name}` expects {}, but {} was supplied",
                expected_label(&types),
                value_label(value)
            ),
        ));
    }

    if let Some(maximum) = maximum_for(property.get("format").and_then(Value::as_str)) {
        if let Some(number) = value.as_u64() {
            if number > maximum {
                return Err(FormError::field(
                    name.to_owned(),
                    format!("`{name}` accepts values up to {maximum}, but {number} was supplied"),
                ));
            }
        }
    }

    Ok(())
}

/// Whether a value matches one declared JSON Schema type.
fn value_matches(kind: &str, value: &Value) -> bool {
    match kind {
        "string" => value.is_string(),
        "integer" => value.is_i64() || value.is_u64(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        "null" => value.is_null(),
        _ => true,
    }
}

/// A human label for a set of declared types.
fn expected_label(types: &[&str]) -> String {
    let mut labels: Vec<String> = types
        .iter()
        .filter(|kind| **kind != "null")
        .map(|kind| type_label(kind))
        .collect();
    if types.contains(&"null") {
        labels.push("null".to_owned());
    }
    if labels.is_empty() {
        "any value".to_owned()
    } else {
        labels.join(" or ")
    }
}

/// A human label for one JSON Schema type.
fn type_label(kind: &str) -> String {
    match kind {
        "string" => "a string".to_owned(),
        "integer" => "an integer".to_owned(),
        "number" => "a number".to_owned(),
        "boolean" => "a boolean".to_owned(),
        "array" => "a list".to_owned(),
        "object" => "an object".to_owned(),
        "null" => "null".to_owned(),
        other => format!("a {other}"),
    }
}

/// A human label for one supplied value.
fn value_label(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(_) => "a boolean".to_owned(),
        Value::Number(_) => "a number".to_owned(),
        Value::String(_) => "a string".to_owned(),
        Value::Array(_) => "a list".to_owned(),
        Value::Object(_) => "an object".to_owned(),
    }
}

/// The field serde names in a missing-field error.
fn missing_field(message: &str) -> Option<String> {
    const MARKER: &str = "missing field `";
    let start = message.find(MARKER)?;
    let rest = &message[start + MARKER.len()..];
    let end = rest.find('`')?;
    Some(rest[..end].to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every published task kind must get a control per property, and the
    /// required set must match the schema's own `required` list.
    #[test]
    fn controls_match_every_published_schema() {
        let response = eggsec_daemon_protocol::operations::describe_operations();
        let mut total = 0usize;
        for task_kind in &response.task_kinds {
            let schema = &task_kind.request_schema;
            let properties = schema
                .get("properties")
                .and_then(Value::as_object)
                .expect("every published schema has properties");
            let required = required_names(schema);

            let controls = controls_for(task_kind);
            assert_eq!(
                controls.len(),
                properties.len(),
                "{} must produce one control per property",
                task_kind.variant
            );
            for control in &controls {
                assert!(
                    properties.contains_key(&control.name),
                    "{} produced a control for an unknown property",
                    task_kind.variant
                );
                assert_eq!(
                    control.required,
                    required.contains(&control.name.as_str()),
                    "{} disagrees with the schema about `{}` being required",
                    task_kind.variant,
                    control.name
                );
            }
            total += controls.len();
        }
        assert_eq!(
            total, 149,
            "the published schemas carry 149 fields in total"
        );
    }

    /// The controls are ordered deterministically: required first, then
    /// alphabetically.
    #[test]
    fn controls_are_ordered_required_first_then_alphabetically() {
        let response = eggsec_daemon_protocol::operations::describe_operations();
        for task_kind in &response.task_kinds {
            let controls = controls_for(task_kind);
            let mut expected = controls.clone();
            expected.sort_by(|left, right| {
                right
                    .required
                    .cmp(&left.required)
                    .then_with(|| left.name.cmp(&right.name))
            });
            assert_eq!(controls, expected); // Required fields come before optional ones.
            let first_optional = controls.iter().position(|control| !control.required);
            if let Some(index) = first_optional {
                assert!(
                    controls[index..].iter().all(|control| !control.required),
                    "{} mixes required and optional controls",
                    task_kind.variant
                );
            }
        }
    }

    /// One test per measured schema shape.
    #[test]
    fn control_for_covers_every_measured_shape() {
        let required_text = control_for("target", &serde_json::json!({"type": "string"}), true);
        assert_eq!(required_text.kind, ControlKind::Text);
        assert!(required_text.required);

        let optional_text = control_for(
            "ports",
            &serde_json::json!({"type": ["string", "null"]}),
            false,
        );
        assert_eq!(optional_text.kind, ControlKind::Text);
        assert!(!optional_text.required);

        let required_number = control_for("count", &serde_json::json!({"type": "integer"}), true);
        assert_eq!(required_number.kind, ControlKind::Number);

        let optional_number = control_for(
            "count",
            &serde_json::json!({"type": ["integer", "null"], "format": "uint16", "minimum": 0}),
            false,
        );
        assert_eq!(optional_number.kind, ControlKind::Number);
        assert_eq!(optional_number.minimum, Some(0));
        assert_eq!(optional_number.maximum, Some(u16::MAX as u64));
        assert_eq!(optional_number.format.as_deref(), Some("uint16"));

        let required_boolean = control_for("udp", &serde_json::json!({"type": "boolean"}), true);
        assert_eq!(required_boolean.kind, ControlKind::Checkbox);

        let optional_boolean = control_for(
            "udp",
            &serde_json::json!({"type": ["boolean", "null"], "default": null}),
            false,
        );
        assert_eq!(optional_boolean.kind, ControlKind::Checkbox);

        let list = control_for(
            "steps",
            &serde_json::json!({"type": ["array", "null"], "items": {"type": "string"}}),
            false,
        );
        assert_eq!(list.kind, ControlKind::List);

        let unconstrained = control_for("config", &Value::Bool(true), false);
        assert_eq!(unconstrained.kind, ControlKind::Json);
    }

    /// The documented fallback: an unrecognized shape degrades to a control
    /// that can still carry the value. It never panics and never drops the
    /// field.
    #[test]
    fn control_for_falls_back_without_panicking() {
        let unknown_scalar = control_for("mystery", &serde_json::json!({"type": "wat"}), false);
        assert_eq!(unknown_scalar.kind, ControlKind::Text);

        let object_shape = control_for(
            "nested",
            &serde_json::json!({"type": "object", "properties": {"a": {"type": "string"}}}),
            false,
        );
        assert_eq!(object_shape.kind, ControlKind::Json);

        let object_array = control_for(
            "items",
            &serde_json::json!({"type": "array", "items": {"type": "object"}}),
            false,
        );
        assert_eq!(object_array.kind, ControlKind::Json);
    }

    /// A valid form deserializes into a typed request with the right
    /// variant — for every published task kind.
    #[test]
    fn assemble_request_round_trips_every_published_task_kind() {
        let response = eggsec_daemon_protocol::operations::describe_operations();
        for task_kind in &response.task_kinds {
            let schema = &task_kind.request_schema;
            let properties = schema
                .get("properties")
                .and_then(Value::as_object)
                .expect("properties");

            // Fill every field with a value of the shape the schema declares.
            let mut values = Map::new();
            for (name, property) in properties {
                let kind = control_kind(property);
                let value = match kind {
                    ControlKind::Text => Value::String("value".to_owned()),
                    ControlKind::Number => Value::from(1u64),
                    ControlKind::Checkbox => Value::Bool(true),
                    ControlKind::List => Value::Array(vec![Value::String("a".to_owned())]),
                    ControlKind::Json => Value::Object(Map::new()),
                };
                values.insert(name.clone(), value);
            }

            let request = assemble_request(task_kind, &Value::Object(values))
                .unwrap_or_else(|error| panic!("{}: {error}", task_kind.variant));

            assert_eq!(
                request.surface,
                RuntimeSurface::DesktopGuiManual,
                "{} must be stamped with the GUI's surface",
                task_kind.variant
            );
            assert!(request.requested_by.is_none());
            assert!(request.labels.is_empty());

            let serialized = serde_json::to_value(&request).expect("serializable");
            let kind = serialized
                .get("task_kind")
                .and_then(|task_kind| task_kind.get("kind"))
                .and_then(Value::as_str)
                .expect("the task kind's discriminator");
            assert_eq!(kind, task_kind.variant);
        }
    }

    /// A required field the operator left blank is a field-level error.
    #[test]
    fn a_missing_required_field_names_the_field() {
        let response = eggsec_daemon_protocol::operations::describe_operations();
        let task_kind = response
            .task_kinds
            .iter()
            .find(|task_kind| task_kind.variant == "PortScan")
            .expect("PortScan is published");
        let error = assemble_request(task_kind, &Value::Object(Map::new()))
            .expect_err("a blank required field is an error");
        assert_eq!(error.field.as_deref(), Some("target"));
        assert!(error.message.contains("required"));
    }

    /// A value of the wrong type is rejected with a field-level error.
    #[test]
    fn a_wrongly_typed_value_names_the_field_and_the_expected_type() {
        let response = eggsec_daemon_protocol::operations::describe_operations();
        let task_kind = response
            .task_kinds
            .iter()
            .find(|task_kind| task_kind.variant == "PortScan")
            .expect("PortScan is published");
        let values = serde_json::json!({"target": 42});
        let error = assemble_request(task_kind, &values).expect_err("a number is not a string");
        assert_eq!(error.field.as_deref(), Some("target"));
        assert!(error.message.contains("a string"), "{}", error.message);
    }

    /// A value beyond the format's width is rejected before it reaches the
    /// engine.
    #[test]
    fn an_out_of_range_number_is_rejected() {
        let response = eggsec_daemon_protocol::operations::describe_operations();
        let task_kind = response
            .task_kinds
            .iter()
            .find(|task_kind| task_kind.variant == "PacketSend")
            .expect("PacketSend is published");
        let schema = &task_kind.request_schema;
        let port = schema
            .get("properties")
            .and_then(|p| p.as_object())
            .and_then(|p| p.get("port"))
            .expect("PacketSend has a port");
        let format = port
            .get("format")
            .and_then(Value::as_str)
            .expect("a format");
        assert_eq!(format, "uint16");

        // Every other field gets a value of its declared shape, so the only
        // thing wrong with this form is the out-of-range port.
        let mut values = Map::new();
        for (name, property) in schema
            .get("properties")
            .and_then(Value::as_object)
            .expect("properties")
        {
            let value = if name == "port" {
                Value::from(u64::from(u16::MAX) + 1)
            } else {
                match control_kind(property) {
                    ControlKind::Text => Value::String("x".to_owned()),
                    ControlKind::Number => Value::from(1u64),
                    ControlKind::Checkbox => Value::Bool(true),
                    ControlKind::List => Value::Array(vec![Value::String("a".to_owned())]),
                    ControlKind::Json => Value::Object(Map::new()),
                }
            };
            values.insert(name.clone(), value);
        }
        let error = assemble_request(task_kind, &Value::Object(values))
            .expect_err("a value beyond uint16 is rejected");
        assert_eq!(error.field.as_deref(), Some("port"));
    }

    /// A field that is not required and does not accept `null` is omitted
    /// from the request, so the request DTO applies its own default. The
    /// fail-safe `dry_run` flag is the case that matters: an untouched form
    /// must leave it at `true`, never turn it into `false`.
    #[test]
    fn an_untouched_optional_field_keeps_the_dto_s_own_default() {
        let response = eggsec_daemon_protocol::operations::describe_operations();
        let task_kind = response
            .task_kinds
            .iter()
            .find(|task_kind| task_kind.variant == "WirelessActive")
            .expect("WirelessActive is published");

        let mut values = Map::new();
        for (name, property) in task_kind
            .request_schema
            .get("properties")
            .and_then(|p| p.as_object())
            .expect("properties")
        {
            let kind = control_kind(property);
            let value = match kind {
                ControlKind::Text => Value::String("x".to_owned()),
                ControlKind::Number => Value::from(1u64),
                ControlKind::Checkbox => Value::Bool(false),
                ControlKind::List => Value::Array(vec![]),
                ControlKind::Json => Value::Null,
            };
            values.insert(name.clone(), value);
        }
        // The operator never touched the fields that carry a non-null
        // default.
        for untouched in ["dry_run", "frame_count", "rate_limit"] {
            values.insert(untouched.to_owned(), Value::Null);
        }

        let request =
            assemble_request(task_kind, &Value::Object(values)).expect("the request assembles");
        let serialized = serde_json::to_value(&request).expect("serializable");
        let params = serialized
            .get("task_kind")
            .and_then(|task_kind| task_kind.get("params"))
            .expect("params");
        assert_eq!(
            params.get("dry_run"),
            Some(&serde_json::Value::Bool(true)),
            "an untouched optional field must keep the DTO's own default: {params}"
        );
        assert_eq!(
            params.get("frame_count"),
            Some(&serde_json::Value::from(100u64)),
            "an untouched optional field must keep the DTO's own default: {params}"
        );
    }

    /// A field that accepts `null` is sent as `null` when the operator did
    /// not provide it.
    #[test]
    fn an_untouched_nullable_field_is_sent_as_null() {
        let response = eggsec_daemon_protocol::operations::describe_operations();
        let task_kind = response
            .task_kinds
            .iter()
            .find(|task_kind| task_kind.variant == "PortScan")
            .expect("PortScan is published");
        let mut values = Map::new();
        values.insert("target".to_owned(), Value::String("10.0.0.1".to_owned()));
        let request = assemble_request(task_kind, &Value::Object(values)).expect("assembles");
        let serialized = serde_json::to_value(&request).expect("serializable");
        let params = serialized
            .get("task_kind")
            .and_then(|task_kind| task_kind.get("params"))
            .expect("params");
        assert_eq!(params.get("ports"), Some(&Value::Null));
    }

    /// The surface is produced by serializing the runtime's own enum, so it
    /// cannot drift from the runtime's label.
    #[test]
    fn the_surface_is_the_runtime_s_own_value() {
        let response = eggsec_daemon_protocol::operations::describe_operations();
        let task_kind = response.task_kinds.first().expect("a task kind");
        let mut values = Map::new();
        for (name, property) in task_kind
            .request_schema
            .get("properties")
            .and_then(|p| p.as_object())
            .expect("properties")
        {
            let kind = control_kind(property);
            let value = match kind {
                ControlKind::Text => Value::String("x".to_owned()),
                ControlKind::Number => Value::from(1u64),
                ControlKind::Checkbox => Value::Bool(true),
                ControlKind::List => Value::Array(vec![Value::String("a".to_owned())]),
                ControlKind::Json => Value::Object(Map::new()),
            };
            values.insert(name.clone(), value);
        }
        let request = assemble_request(task_kind, &Value::Object(values)).expect("assembles");
        let serialized = serde_json::to_value(&request).expect("serializable");
        assert_eq!(
            serialized.get("surface"),
            Some(&serde_json::to_value(RuntimeSurface::DesktopGuiManual).expect("serializable"))
        );
        assert_eq!(
            RuntimeSurface::DesktopGuiManual.label(),
            "desktop-gui-manual",
            "the audit-trail label must not drift"
        );
    }
}
