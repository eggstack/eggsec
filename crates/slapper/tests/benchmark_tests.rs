#[cfg(test)]
mod benchmarks {
    use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

    fn timing_preset_from_str_benchmark(c: &mut Criterion) {
        let mut group = c.benchmark_group("timing_preset");

        for preset in ["T0", "T1", "T2", "T3", "T4", "T5"] {
            group.bench_with_input(BenchmarkId::from_parameter(preset), preset, |b, preset| {
                b.iter(|| slapper::scanner::timing::TimingPreset::from_str(black_box(preset)));
            });
        }

        group.finish();
    }

    fn port_priority_categorize_benchmark(c: &mut Criterion) {
        let mut group = c.benchmark_group("port_priority");

        let test_ports: Vec<u16> = (1..=10000).collect();

        group.bench_function("categorize 10k ports", |b| {
            b.iter(|| slapper::scanner::timing::PortPriority::categorize(black_box(&test_ports)));
        });

        group.bench_function("get_top_ports 100", |b| {
            b.iter(|| slapper::scanner::timing::PortPriority::get_top_ports(black_box(100)));
        });

        group.finish();
    }

    fn timing_config_from_preset_benchmark(c: &mut Criterion) {
        let mut group = c.benchmark_group("timing_config");

        use slapper::scanner::timing::{TimingConfig, TimingPreset};

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
        dashmap_benchmark
    );
    criterion_main!(benches);
}
