use criterion::{black_box, criterion_group, criterion_main, Criterion};
use eggsec::proxy::intercept::{
    EnhancedRule, EnhancedRuleSet, RuleAction, RuleCondition, RuleContext,
};
use eggsec::proxy::intercept::types::{ProxyFlow, FlowBuffer, WebProxySessionReport};
use std::collections::HashMap;

fn make_flow(index: u64) -> ProxyFlow {
    ProxyFlow {
        index,
        method: "GET".to_string(),
        url: format!("https://example.com/path/{}", index),
        host: "example.com".to_string(),
        path: format!("/path/{}", index),
        request_headers: HashMap::new(),
        request_body: None,
        response_status: 200,
        response_headers: HashMap::new(),
        response_body: None,
        is_https: true,
        duration_ms: 100,
        request_body_size: 0,
        response_body_size: 1024,
        started_at: "2026-01-01T00:00:00Z".to_string(),
        completed_at: "2026-01-01T00:00:00Z".to_string(),
        redaction_applied: None,
        protocol: "http1".to_string(),
    }
}

fn bench_rule_condition_simple(c: &mut Criterion) {
    let ctx = RuleContext::new("example.com", "/api/data", "POST");
    c.bench_function("rule_condition_host_matches", |b| {
        let cond = RuleCondition::HostMatches("example.com".to_string());
        b.iter(|| black_box(cond.evaluate(&ctx)));
    });
    c.bench_function("rule_condition_path_matches", |b| {
        let cond = RuleCondition::PathMatches("/api/*".to_string());
        b.iter(|| black_box(cond.evaluate(&ctx)));
    });
    c.bench_function("rule_condition_and", |b| {
        let cond = RuleCondition::And(vec![
            RuleCondition::HostMatches("example.com".to_string()),
            RuleCondition::MethodMatches("POST".to_string()),
            RuleCondition::PathMatches("/api/*".to_string()),
        ]);
        b.iter(|| black_box(cond.evaluate(&ctx)));
    });
    c.bench_function("rule_condition_not", |b| {
        let cond = RuleCondition::Not(Box::new(RuleCondition::HostMatches("evil.com".to_string())));
        b.iter(|| black_box(cond.evaluate(&ctx)));
    });
}

fn bench_rule_condition_nested(c: &mut Criterion) {
    let ctx = RuleContext::new("example.com", "/api/v2/users", "GET");
    c.bench_function("rule_condition_deeply_nested_or", |b| {
        let cond = RuleCondition::Or(vec![
            RuleCondition::And(vec![
                RuleCondition::HostMatches("example.com".to_string()),
                RuleCondition::PathMatches("/api/*".to_string()),
            ]),
            RuleCondition::And(vec![
                RuleCondition::HostMatches("other.com".to_string()),
                RuleCondition::PathMatches("/data/*".to_string()),
            ]),
            RuleCondition::MethodMatches("GET".to_string()),
        ]);
        b.iter(|| black_box(cond.evaluate(&ctx)));
    });
}

fn bench_enhanced_rule_set(c: &mut Criterion) {
    let mut group = c.benchmark_group("enhanced_rule_set");

    for n_rules in [10, 100, 1000] {
        let mut ruleset = EnhancedRuleSet::new();
        for i in 0..n_rules {
            let rule = EnhancedRule::new(
                &format!("rule-{}", i),
                &format!("Rule {}", i),
                RuleCondition::HostMatches(format!("host-{}.example.com", i)),
                RuleAction::Monitor,
            );
            ruleset.add(rule);
        }

        let ctx = RuleContext::new("host-500.example.com", "/test", "GET");
        group.bench_function(format!("evaluate_{}_rules", n_rules), |b| {
            b.iter(|| black_box(ruleset.evaluate(&ctx)));
        });
    }
    group.finish();
}

fn bench_flow_buffer(c: &mut Criterion) {
    let mut group = c.benchmark_group("flow_buffer");

    group.bench_function("push_no_eviction", |b| {
        b.iter_batched(
            || FlowBuffer::new(10000),
            |mut buf| {
                for i in 0..100 {
                    buf.push(make_flow(i));
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("push_with_eviction", |b| {
        b.iter_batched(
            || FlowBuffer::new(100),
            |mut buf| {
                for i in 0..500 {
                    buf.push(make_flow(i));
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_flow_serialization(c: &mut Criterion) {
    let flow = make_flow(1);
    c.bench_function("flow_serialize_json", |b| {
        b.iter(|| black_box(serde_json::to_string(&flow).unwrap()));
    });

    let json = serde_json::to_string(&flow).unwrap();
    c.bench_function("flow_deserialize_json", |b| {
        b.iter(|| black_box(serde_json::from_str::<ProxyFlow>(&json).unwrap()));
    });
}

fn bench_session_report(c: &mut Criterion) {
    c.bench_function("session_report_add_100_flows", |b| {
        b.iter_batched(
            || WebProxySessionReport::new("127.0.0.1:8080", false),
            |mut report| {
                for i in 0..100 {
                    report.add_flow(make_flow(i));
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    benches,
    bench_rule_condition_simple,
    bench_rule_condition_nested,
    bench_enhanced_rule_set,
    bench_flow_buffer,
    bench_flow_serialization,
    bench_session_report,
);
criterion_main!(benches);
