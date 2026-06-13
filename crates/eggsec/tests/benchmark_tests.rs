#[cfg(test)]
mod benchmarks {
    use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

    fn timing_preset_from_str_benchmark(c: &mut Criterion) {
        let mut group = c.benchmark_group("timing_preset");

        for preset in ["T0", "T1", "T2", "T3", "T4", "T5"] {
            group.bench_with_input(BenchmarkId::from_parameter(preset), preset, |b, preset| {
                b.iter(|| eggsec::scanner::timing::TimingPreset::from_str(black_box(preset)));
            });
        }

        group.finish();
    }

    fn port_priority_categorize_benchmark(c: &mut Criterion) {
        let mut group = c.benchmark_group("port_priority");

        let test_ports: Vec<u16> = (1..=10000).collect();

        group.bench_function("categorize 10k ports", |b| {
            b.iter(|| eggsec::scanner::timing::PortPriority::categorize(black_box(&test_ports)));
        });

        group.bench_function("get_top_ports 100", |b| {
            b.iter(|| eggsec::scanner::timing::PortPriority::get_top_ports(black_box(100)));
        });

        group.finish();
    }

    fn timing_config_from_preset_benchmark(c: &mut Criterion) {
        let mut group = c.benchmark_group("timing_config");

        use eggsec::scanner::timing::{TimingConfig, TimingPreset};

        for preset in [
            TimingPreset::Normal,
            TimingPreset::Aggressive,
            TimingPreset::Insane,
        ] {
            group.bench_with_input(
                BenchmarkId::from_parameter(format!("{:?}", preset)),
                &preset,
                |b, preset| {
                    b.iter(|| TimingConfig::from_preset(*black_box(preset)));
                },
            );
        }

        group.finish();
    }

    fn dashmap_benchmark(c: &mut Criterion) {
        use dashmap::DashMap;

        let mut group = c.benchmark_group("concurrency_primitives");

        let iterations = 1000;

        group.bench_function("DashMap insert (1000 items)", |b| {
            b.iter(|| {
                let results: DashMap<u16, u16> = DashMap::new();

                for i in 0..iterations {
                    results.insert(i, i * 2);
                }

                results.len()
            });
        });

        group.bench_function("DashMap concurrent insert (100 items x 10 threads)", |b| {
            use std::sync::Arc;
            use std::thread;

            b.iter(|| {
                let results: Arc<DashMap<u16, u16>> = Arc::new(DashMap::new());
                let mut handles = Vec::new();

                for _ in 0..10 {
                    let results = results.clone();
                    handles.push(thread::spawn(move || {
                        for i in 0..100 {
                            results.insert(i, i * 2);
                        }
                    }));
                }

                for handle in handles {
                    handle.join().ok();
                }

                results.len()
            });
        });

        group.finish();
    }

    criterion_group!(
        benches,
        timing_preset_from_str_benchmark,
        port_priority_categorize_benchmark,
        timing_config_from_preset_benchmark,
        dashmap_benchmark,
        rule_evaluation_benchmark,
        rule_indexed_evaluation_benchmark,
        evidence_bundle_export_benchmark,
        protobuf_encoding_benchmark
    );
    criterion_main!(benches);

    // ==================== Proxy Module Benchmarks ====================

    use eggsec::proxy::intercept::rules::{
        EnhancedRule, EnhancedRuleSet, RuleAction, RuleCondition, RuleContext,
    };

    fn rule_evaluation_benchmark(c: &mut Criterion) {
        let mut group = c.benchmark_group("rule_evaluation");

        // Benchmark: evaluate 100 rules
        let mut rules_100 = EnhancedRuleSet::new();
        for i in 0..100 {
            let condition = if i % 3 == 0 {
                RuleCondition::HostMatches(format!("host-{}.example.com", i))
            } else if i % 3 == 1 {
                RuleCondition::PathMatches(format!("/api/v{}/", i % 10))
            } else {
                RuleCondition::And(vec![
                    RuleCondition::HostMatches("target.example.com".to_string()),
                    RuleCondition::PathMatches(format!("/path/{}", i)),
                ])
            };
            rules_100.add(EnhancedRule::new(
                &format!("rule-{}", i),
                &format!("Rule {}", i),
                condition,
                RuleAction::Intercept,
            ));
        }

        let ctx = RuleContext::new("target.example.com", "/path/50", "GET");
        group.bench_function("evaluate 100 rules", |b| {
            b.iter(|| rules_100.evaluate(black_box(&ctx)));
        });

        // Benchmark: evaluate 1000 rules
        let mut rules_1000 = EnhancedRuleSet::new();
        for i in 0..1000 {
            let condition = RuleCondition::HostMatches(format!("host-{}.example.com", i));
            rules_1000.add(EnhancedRule::new(
                &format!("rule-{}", i),
                &format!("Rule {}", i),
                condition,
                RuleAction::Intercept,
            ));
        }

        group.bench_function("evaluate 1000 rules", |b| {
            b.iter(|| rules_1000.evaluate(black_box(&ctx)));
        });

        group.finish();
    }

    fn rule_indexed_evaluation_benchmark(c: &mut Criterion) {
        let mut group = c.benchmark_group("rule_indexed_evaluation");

        // Benchmark: indexed evaluation with 1000 rules
        let mut rules = EnhancedRuleSet::new();
        for i in 0..1000 {
            let condition = RuleCondition::HostMatches(format!("host-{}.example.com", i));
            rules.add(EnhancedRule::new(
                &format!("rule-{}", i),
                &format!("Rule {}", i),
                condition,
                RuleAction::Intercept,
            ));
        }

        let ctx = RuleContext::new("host-500.example.com", "/", "GET");
        group.bench_function("indexed evaluate 1000 rules", |b| {
            b.iter(|| rules.evaluate_indexed(black_box(&ctx)));
        });

        group.finish();
    }

    fn evidence_bundle_export_benchmark(c: &mut Criterion) {
        use eggsec::proxy::intercept::bundle::EvidenceBundle;
        use eggsec::proxy::intercept::types::{ProxyFlow, WebProxySessionReport};

        let mut group = c.benchmark_group("evidence_bundle");

        // Create a report with 100 flows
        let mut report = WebProxySessionReport::new("127.0.0.1:8080", true);
        for i in 0..100 {
            report.add_flow(ProxyFlow {
                index: i,
                method: "GET".to_string(),
                url: format!("https://example.com/api/{}", i),
                host: "example.com".to_string(),
                path: format!("/api/{}", i),
                request_headers: Default::default(),
                request_body: None,
                response_status: 200,
                response_headers: Default::default(),
                response_body: Some(format!("response-{}", i)),
                is_https: true,
                duration_ms: 50,
                request_body_size: 0,
                response_body_size: 100,
                started_at: chrono::Utc::now().to_rfc3339(),
                completed_at: chrono::Utc::now().to_rfc3339(),
                redaction_applied: None,
                protocol: "http1".to_string(),
            });
        }

        group.bench_function("bundle from_report 100 flows", |b| {
            b.iter(|| EvidenceBundle::from_report(black_box(&report), None));
        });

        let bundle = EvidenceBundle::from_report(&report, None);
        group.bench_function("bundle to_bytes (gzip)", |b| {
            b.iter(|| bundle.to_bytes().unwrap());
        });

        group.finish();
    }

    fn protobuf_encoding_benchmark(c: &mut Criterion) {
        use eggsec::proxy::intercept::protocols::{GrpcCall, GrpcMethodType};

        let mut group = c.benchmark_group("protobuf_encoding");

        // Create a gRPC call with a request body
        let mut call = GrpcCall::new("/package.Service/Method", GrpcMethodType::Unary);
        call.request_body = Some(hex::encode(b"\x0a\x0bhello world\x12\x0ctest"));
        call.response_body = Some(hex::encode(b"\x0a\x0bhello world\x12\x0ctest"));

        group.bench_function("decode_request_body", |b| {
            b.iter(|| call.decode_request_body());
        });

        group.bench_function("decode_response_body", |b| {
            b.iter(|| call.decode_response_body());
        });

        let json = serde_json::json!({"field1": "value1", "field2": 42});
        group.bench_function("encode_request_body", |b| {
            b.iter(|| {
                let mut c = GrpcCall::new("/test", GrpcMethodType::Unary);
                c.encode_request_body(black_box(&json)).unwrap();
            });
        });

        group.finish();
    }
}
