//! Phase A performance baseline harness (manual, loopback-only, informational).
//!
//! Every test here is `#[ignore]`: normal `make check` never executes long
//! timing workloads. Run explicitly in release mode:
//!
//! ```bash
//! bash scripts/perf-profile.sh --suite all --trials 5 --warmup 1
//! cargo test --release -p eggsec --test perf_baseline -- --ignored --nocapture
//! ```
//!
//! Results stream as stable `perf key=value` lines. Timing numbers are
//! informational (never merge gates); structural counters (peak live work,
//! hops, checksums, connection/auth counts) are the durable evidence.

mod common;

use common::{create_test_server, mock_ok};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn trials() -> usize {
    env_usize("EGGSEC_PERF_TRIALS", 5).max(1)
}

fn warmup() -> usize {
    env_usize("EGGSEC_PERF_WARMUP", 1)
}

fn peak_rss_kb() -> Option<u64> {
    // Portable-ish: Linux /proc/self/status VmHWM; None elsewhere.
    let text = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            let digits: String = rest.chars().filter(|c| c.is_ascii_digit()).collect();
            return digits.parse().ok();
        }
    }
    None
}

fn checksum<I: Hash>(items: &[I]) -> u64 {
    let mut h = DefaultHasher::new();
    items.len().hash(&mut h);
    for item in items {
        item.hash(&mut h);
    }
    h.finish()
}

fn loopback_scope() -> eggsec::config::Scope {
    let mut scope = eggsec::config::Scope::new();
    scope.allowed_targets.push(
        eggsec::config::ScopeRule::with_cidr("127.0.0.0/8".to_string()).expect("loopback cidr"),
    );
    scope
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = values.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 1 {
        values[n / 2]
    } else {
        (values[n / 2 - 1] + values[n / 2]) / 2.0
    }
}

// ---------------------------------------------------------------------------
// A3: CPU-only request materialization profile.
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn perf_loadtest_materialization() {
    let iters = 20_000usize;
    let mut throughputs = Vec::new();
    let mut clone_throughputs = Vec::new();
    for trial in 0..warmup() + trials() {
        let (plan, template) = loadtest_plan_template("http://127.0.0.1:9/a");
        let start = Instant::now();
        let mut ok = 0u64;
        for _ in 0..iters {
            if template.scoped_request(&plan).is_ok() {
                ok += 1;
            }
        }
        let elapsed = start.elapsed();
        assert_eq!(ok, iters as u64);
        // Phase C: per-request cost after the optimization is a prototype
        // clone, not a rebuild. Time it on the same iteration count.
        let prototype = template.scoped_request(&plan).expect("prototype");
        let clone_start = Instant::now();
        let mut clone_ok = 0u64;
        for _ in 0..iters {
            let cloned = prototype.clone();
            if cloned.url.as_str() == "http://127.0.0.1:9/a" {
                clone_ok += 1;
            }
        }
        let clone_elapsed = clone_start.elapsed();
        assert_eq!(clone_ok, iters as u64);
        if trial >= warmup() {
            let ops = iters as f64 / elapsed.as_secs_f64();
            throughputs.push(ops);
            let clone_ops = iters as f64 / clone_elapsed.as_secs_f64();
            clone_throughputs.push(clone_ops);
            println!(
                "perf materialization trial={} iters={} wall_ms={} ops_per_sec={:.0} ns_per_op={:.0} clone_wall_ms={} clone_ops_per_sec={:.0} clone_ns_per_op={:.0} ok={}",
                trial,
                iters,
                elapsed.as_millis(),
                ops,
                elapsed.as_nanos() as f64 / iters as f64,
                clone_elapsed.as_millis(),
                clone_ops,
                clone_elapsed.as_nanos() as f64 / iters as f64,
                ok
            );
        }
    }
    println!(
        "perf materialization summary iters={} trials={} warmup={} median_ops_per_sec={:.0} median_clone_ops_per_sec={:.0} rss_kb={:?}",
        iters,
        trials(),
        warmup(),
        median(throughputs),
        median(clone_throughputs),
        peak_rss_kb()
    );
}

fn loadtest_plan_template(
    url: &str,
) -> (
    eggsec::loadtest::LoadTestPlan,
    eggsec::loadtest::RequestTemplate,
) {
    let plan = eggsec::loadtest::LoadTestPlan::new(url.to_string(), 1, 1, Duration::from_secs(5))
        .expect("plan");
    let template = eggsec::loadtest::RequestTemplate {
        headers: vec![("X-Perf".to_string(), "1".to_string())],
        body: None,
        user_agent: "eggsec-perf/1.0".to_string(),
        timeout: eggsec_transport::TimeoutPolicy::with_request_timeout(5),
        redirect: eggsec_transport::RedirectPolicy::SameHostOnly { max_redirects: 5 },
        proxy: eggsec_transport::ProxyIntent::Direct,
        tls: eggsec_transport::TlsPolicy::verified(),
    };
    (plan, template)
}

// ---------------------------------------------------------------------------
// A3: executor through the recording fake (no network; isolates DTO cost).
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct AllowAll;

impl eggsec_transport::NetworkAuthority for AllowAll {
    fn authorize_initial_url(
        &self,
        url: &url::Url,
    ) -> Result<(), eggsec_transport::TransportError> {
        eggsec_transport::reject_url_userinfo(url)
    }
    fn authorize_host(
        &self,
        _h: &str,
        _p: Option<u16>,
        _l: bool,
    ) -> Result<(), eggsec_transport::TransportError> {
        Ok(())
    }
    fn authorize_resolved(
        &self,
        _h: &str,
        c: &[std::net::IpAddr],
    ) -> Result<Vec<std::net::IpAddr>, eggsec_transport::TransportError> {
        Ok(c.to_vec())
    }
    fn authorize_socket(
        &self,
        _h: &str,
        _a: std::net::IpAddr,
        _p: u16,
    ) -> Result<(), eggsec_transport::TransportError> {
        Ok(())
    }
    fn authorize_redirect(
        &self,
        _f: &url::Url,
        t: &url::Url,
    ) -> Result<(), eggsec_transport::TransportError> {
        self.authorize_initial_url(t)
    }
    fn authorize_proxy(
        &self,
        _p: &url::Url,
        _u: &url::Url,
    ) -> Result<(), eggsec_transport::TransportError> {
        Ok(())
    }
    fn check_tls_consistency(
        &self,
        _h: &str,
        _s: Option<&str>,
        _o: Option<&str>,
    ) -> Result<(), eggsec_transport::TransportError> {
        Ok(())
    }
}

#[tokio::test]
#[ignore]
async fn perf_loadtest_fake_concurrency() {
    let total = 5_000u64;
    for concurrency in [1usize, 10, 50, 100] {
        let mut rps_samples = Vec::new();
        for trial in 0..warmup() + trials() {
            let resolver = Arc::new(
                eggsec_transport::InMemoryResolver::new().with("127.0.0.1", vec!["127.0.0.1"]),
            );
            let transport = Arc::new(
                eggsec_transport::RecordingFakeTransport::new(resolver).with_canned(
                    "http://127.0.0.1/",
                    eggsec_transport::CannedResponse {
                        status: eggsec_transport::StatusCode::OK,
                        location: None,
                        body: b"ok".to_vec(),
                    },
                ),
            );
            let plan = eggsec::loadtest::LoadTestPlan::new(
                "http://127.0.0.1/".to_string(),
                total,
                concurrency,
                Duration::from_secs(30),
            )
            .expect("plan");
            let template = eggsec::loadtest::RequestTemplate {
                headers: Vec::new(),
                body: None,
                user_agent: "eggsec-perf/1.0".to_string(),
                timeout: eggsec_transport::TimeoutPolicy::with_request_timeout(30),
                redirect: eggsec_transport::RedirectPolicy::SameHostOnly { max_redirects: 5 },
                proxy: eggsec_transport::ProxyIntent::Direct,
                tls: eggsec_transport::TlsPolicy::verified(),
            };
            let executor = eggsec::loadtest::LoadTestExecutor::new(
                plan,
                template,
                transport.clone(),
                Arc::new(AllowAll),
                tokio_util::sync::CancellationToken::new(),
            );
            let start = Instant::now();
            let results = executor
                .run(&eggsec::loadtest::NoopSink)
                .await
                .expect("run");
            let wall = start.elapsed();
            assert_eq!(results.total_requests, total);
            assert_eq!(transport.hop_count(), total as usize);
            if trial >= warmup() {
                let rps = total as f64 / wall.as_secs_f64().max(f64::EPSILON);
                rps_samples.push(rps);
                println!(
                    "perf loadtest-fake trial={} concurrency={} total={} wall_ms={} rps={:.0} p50={} p95={} p99={} errors={} hops={}",
                    trial,
                    concurrency,
                    total,
                    wall.as_millis(),
                    rps,
                    results.latency_p50_ms,
                    results.latency_p95_ms,
                    results.latency_p99_ms,
                    results.failed_requests,
                    transport.hop_count()
                );
            }
        }
        println!(
            "perf loadtest-fake summary concurrency={} total={} trials={} warmup={} median_rps={:.0} rss_kb={:?}",
            concurrency,
            total,
            trials(),
            warmup(),
            median(rps_samples),
            peak_rss_kb()
        );
    }
}

// ---------------------------------------------------------------------------
// A3: wiremock H1 loopback through the production runner shape.
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn perf_loadtest_wiremock_h1() {
    for concurrency in [1usize, 10, 50] {
        let total = 200u64;
        let mut rps_samples = Vec::new();
        for trial in 0..warmup() + trials() {
            let server = create_test_server().await;
            mock_ok("/").mount(&server).await;
            let runner = eggsec::loadtest::LoadTestRunner::new(
                server.uri(),
                total,
                concurrency,
                Duration::from_secs(10),
            )
            .expect("runner")
            .with_scope(loopback_scope());
            let start = Instant::now();
            let results = runner.run().await.expect("run");
            let wall = start.elapsed();
            assert_eq!(results.total_requests, total);
            if trial >= warmup() {
                let rps = total as f64 / wall.as_secs_f64().max(f64::EPSILON);
                rps_samples.push(rps);
                println!(
                    "perf loadtest-h1 trial={} concurrency={} total={} wall_ms={} rps={:.0} p50={} p95={} failed={}",
                    trial,
                    concurrency,
                    total,
                    wall.as_millis(),
                    rps,
                    results.latency_p50_ms,
                    results.latency_p95_ms,
                    results.failed_requests
                );
            }
        }
        println!(
            "perf loadtest-h1 summary concurrency={} total={} trials={} warmup={} median_rps={:.0} rss_kb={:?}",
            concurrency,
            total,
            trials(),
            warmup(),
            median(rps_samples),
            peak_rss_kb()
        );
    }
}

// ---------------------------------------------------------------------------
// A4: synthetic scheduler proof — retained-handle shape vs bounded JoinSet.
// ---------------------------------------------------------------------------

async fn synthetic_spawn_per_item(total: usize, concurrency: usize) -> (u64, usize) {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
    let live = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::with_capacity(total);
    for i in 0..total {
        let semaphore = semaphore.clone();
        let live = live.clone();
        let peak = peak.clone();
        handles.push(tokio::spawn(async move {
            let Ok(_permit) = semaphore.acquire_owned().await else {
                return 0u64;
            };
            let cur = live.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(cur, Ordering::SeqCst);
            tokio::task::yield_now().await;
            live.fetch_sub(1, Ordering::SeqCst);
            (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        }));
    }
    let mut acc = 0u64;
    for h in handles {
        acc = acc.wrapping_add(h.await.unwrap_or(0));
    }
    let retained = total;
    (
        acc,
        retained
            .min(usize::MAX)
            .saturating_add(peak.load(Ordering::SeqCst) * 0),
    )
}

async fn synthetic_bounded_joinset(
    total: usize,
    concurrency: usize,
    peak_counter: Arc<AtomicUsize>,
    live_counter: Arc<AtomicUsize>,
) -> u64 {
    use tokio::task::JoinSet;
    let mut next = 0usize;
    let mut acc = 0u64;
    let mut set = JoinSet::new();
    // Admit at most `concurrency` futures; admit one per completion.
    while next < total || !set.is_empty() {
        while next < total && set.len() < concurrency {
            let i = next;
            next += 1;
            let live = live_counter.clone();
            let peak = peak_counter.clone();
            set.spawn(async move {
                let cur = live.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(cur, Ordering::SeqCst);
                tokio::task::yield_now().await;
                live.fetch_sub(1, Ordering::SeqCst);
                (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            });
        }
        if let Some(res) = set.join_next().await {
            acc = acc.wrapping_add(res.unwrap_or(0));
        }
    }
    acc
}

#[tokio::test]
#[ignore]
async fn perf_fanout_synthetic() {
    let total = 10_000usize;
    let concurrency = 50usize;
    let mut walls_unbounded = Vec::new();
    let mut walls_bounded = Vec::new();
    let mut checksum_unbounded = 0u64;
    let mut checksum_bounded = 0u64;
    for trial in 0..warmup() + trials() {
        let start = Instant::now();
        let (acc, retained) = synthetic_spawn_per_item(total, concurrency).await;
        let wall = start.elapsed();
        checksum_unbounded = acc;
        if trial >= warmup() {
            walls_unbounded.push(wall.as_secs_f64());
            println!(
                "perf fanout-unbounded trial={} total={} concurrency={} wall_ms={} retained_handles={} checksum={:#x}",
                trial,
                total,
                concurrency,
                wall.as_millis(),
                retained,
                acc
            );
        }

        let peak = Arc::new(AtomicUsize::new(0));
        let live = Arc::new(AtomicUsize::new(0));
        let start = Instant::now();
        let acc = synthetic_bounded_joinset(total, concurrency, peak.clone(), live.clone()).await;
        let wall = start.elapsed();
        checksum_bounded = acc;
        let peak_live = peak.load(Ordering::SeqCst);
        assert!(
            peak_live <= concurrency,
            "bounded scheduler exceeded concurrency: {peak_live} > {concurrency}"
        );
        if trial >= warmup() {
            walls_bounded.push(wall.as_secs_f64());
            println!(
                "perf fanout-bounded trial={} total={} concurrency={} wall_ms={} peak_live={} checksum={:#x}",
                trial,
                total,
                concurrency,
                wall.as_millis(),
                peak_live,
                acc
            );
        }
    }
    assert_eq!(
        checksum_unbounded, checksum_bounded,
        "scheduler shapes must agree on output"
    );
    println!(
        "perf fanout summary total={} concurrency={} trials={} warmup={} median_unbounded_s={:.3} median_bounded_s={:.3} checksum={:#x} rss_kb={:?}",
        total,
        concurrency,
        trials(),
        warmup(),
        median(walls_unbounded),
        median(walls_bounded),
        checksum_bounded,
        peak_rss_kb()
    );
}

// ---------------------------------------------------------------------------
// A4: port-scan loopback fan-out (small + wider closed range).
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn perf_portscan_loopback() {
    // Two loopback listeners give deterministic open ports; neighbors are
    // deterministically closed-ish (fast refuse) on an isolated host.
    let listener_a = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let port_a = listener_a.local_addr().expect("addr").port();
    let listener_b = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let port_b = listener_b.local_addr().expect("addr").port();
    // Keep listeners alive in background (accept-and-drop).
    tokio::spawn(async move {
        loop {
            let Ok((_, _)) = listener_a.accept().await else {
                break;
            };
        }
    });
    tokio::spawn(async move {
        loop {
            let Ok((_, _)) = listener_b.accept().await else {
                break;
            };
        }
    });

    // Candidate set: the two open ports plus a deterministic closed range.
    let mut ports = vec![port_a, port_b];
    for p in [
        9u16, 22, 80, 443, 19227u16, 18080, 18081, 18082, 18083, 18084,
    ] {
        if !ports.contains(&p) {
            ports.push(p);
        }
    }
    // Full-range-ish deterministic sweep (still loopback-fast, no network).
    let sweep: Vec<u16> = (18090u16..18290u16).collect();

    for (label, candidates) in [("small", ports.clone()), ("sweep200", sweep.clone())] {
        let mut walls = Vec::new();
        for trial in 0..warmup() + trials() {
            let config = eggsec::scanner::ports::PortScanConfig {
                ports: candidates.clone(),
                concurrency: 100,
                timeout_duration: Duration::from_millis(300),
                tui_mode: true,
                spoof_config: eggsec::scanner::spoof::SpoofConfig::default(),
                progress_tx: None,
                max_results: None,
            };
            let start = Instant::now();
            let results = eggsec::scanner::ports::scan_ports("127.0.0.1", config)
                .await
                .expect("scan");
            let wall = start.elapsed();
            let mut open: Vec<u16> = results.open_ports.iter().map(|p| p.port).collect();
            open.sort_unstable();
            let sum = checksum(&open);
            // The two known-open ports must be reported in the small case.
            if label == "small" {
                assert!(
                    open.contains(&port_a),
                    "missing open port {port_a}: {open:?}"
                );
                assert!(
                    open.contains(&port_b),
                    "missing open port {port_b}: {open:?}"
                );
            }
            if trial >= warmup() {
                walls.push(wall.as_secs_f64());
                println!(
                    "perf portscan trial={} case={} candidates={} wall_ms={} open={} checksum={:#x}",
                    trial,
                    label,
                    candidates.len(),
                    wall.as_millis(),
                    open.len(),
                    sum
                );
            }
        }
        println!(
            "perf portscan summary case={} candidates={} trials={} warmup={} median_s={:.3} rss_kb={:?}",
            label,
            candidates.len(),
            trials(),
            warmup(),
            median(walls),
            peak_rss_kb()
        );
    }
}

// ---------------------------------------------------------------------------
// A4: endpoint-scan loopback fan-out (1k synthetic paths).
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn perf_endpoint_loopback() {
    let server = create_test_server().await;
    mock_ok("/health").mount(&server).await;
    mock_ok("/api").mount(&server).await;
    let base = server.uri();
    let mut endpoints = vec!["/health".to_string(), "/api".to_string()];
    for i in 0..1000 {
        endpoints.push(format!("/perf-path-{i}"));
    }
    let mut walls = Vec::new();
    for trial in 0..warmup() + trials() {
        let config = eggsec::scanner::endpoints::EndpointScanConfig {
            base_url: base.clone(),
            endpoints: endpoints.clone(),
            concurrency: 20,
            timeout_duration: Duration::from_secs(5),
            include_404: false,
            tui_mode: true,
            spoof_config: Arc::new(eggsec::scanner::spoof::SpoofConfig::default()),
            verify_tls: true,
            progress_tx: None,
            max_results: None,
        };
        let start = Instant::now();
        let results = eggsec::scanner::endpoints::scan_endpoints(config)
            .await
            .expect("scan");
        let wall = start.elapsed();
        assert!(results.endpoints_found >= 2);
        if trial >= warmup() {
            walls.push(wall.as_secs_f64());
            let paths: Vec<&str> = results.results.iter().map(|r| r.path.as_str()).collect();
            println!(
                "perf endpoint trial={} candidates={} wall_ms={} found={} checksum={:#x}",
                trial,
                endpoints.len(),
                wall.as_millis(),
                results.endpoints_found,
                checksum(&paths)
            );
        }
    }
    println!(
        "perf endpoint summary candidates={} trials={} warmup={} median_s={:.3} rss_kb={:?}",
        endpoints.len(),
        trials(),
        warmup(),
        median(walls),
        peak_rss_kb()
    );
}

// ---------------------------------------------------------------------------
// A4: subdomain scheduler shape (synthetic resolver; no network).
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn perf_subdomain_scheduler() {
    let candidates: Vec<String> = (0..2000).map(|i| format!("host-{i}")).collect();
    let concurrency = 50usize;
    // Fake resolver: even hosts "resolve", odd hosts do not.
    let resolve = Arc::new(|name: String| {
        name.split('-')
            .nth(1)
            .is_some_and(|n| n.parse::<usize>().unwrap_or(1) % 2 == 0)
    });
    let mut walls = Vec::new();
    for trial in 0..warmup() + trials() {
        let peak = Arc::new(AtomicUsize::new(0));
        let live = Arc::new(AtomicUsize::new(0));
        let start = Instant::now();
        let found = bounded_subdomain_verify(
            candidates.clone(),
            concurrency,
            resolve.clone(),
            peak.clone(),
            live.clone(),
        )
        .await;
        let wall = start.elapsed();
        let peak_live = peak.load(Ordering::SeqCst);
        assert!(peak_live <= concurrency);
        assert_eq!(found.len(), 1000);
        if trial >= warmup() {
            walls.push(wall.as_secs_f64());
            println!(
                "perf subdomain trial={} candidates={} concurrency={} wall_ms={} peak_live={} found={} checksum={:#x}",
                trial,
                candidates.len(),
                concurrency,
                wall.as_millis(),
                peak_live,
                found.len(),
                checksum(&found)
            );
        }
    }
    println!(
        "perf subdomain summary candidates={} concurrency={} trials={} warmup={} median_s={:.3} rss_kb={:?}",
        candidates.len(),
        concurrency,
        trials(),
        warmup(),
        median(walls),
        peak_rss_kb()
    );
}

async fn bounded_subdomain_verify(
    candidates: Vec<String>,
    concurrency: usize,
    resolve: Arc<impl Fn(String) -> bool + Send + Sync + 'static>,
    peak: Arc<AtomicUsize>,
    live: Arc<AtomicUsize>,
) -> Vec<String> {
    use tokio::task::JoinSet;
    let mut next = 0usize;
    let mut found = Vec::new();
    let mut set: JoinSet<Option<String>> = JoinSet::new();
    while next < candidates.len() || !set.is_empty() {
        while next < candidates.len() && set.len() < concurrency {
            let name = candidates[next].clone();
            next += 1;
            let resolve = resolve.clone();
            let live = live.clone();
            let peak = peak.clone();
            set.spawn(async move {
                let cur = live.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(cur, Ordering::SeqCst);
                tokio::task::yield_now().await;
                let ok = resolve(name.clone());
                live.fetch_sub(1, Ordering::SeqCst);
                ok.then_some(name)
            });
        }
        if let Some(res) = set.join_next().await {
            if let Ok(Some(name)) = res {
                found.push(name);
            }
        }
    }
    found.sort();
    found
}

// ---------------------------------------------------------------------------
// A5: distributed control-plane baseline (plaintext loopback, isolated lab).
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn perf_distributed_control_plane() {
    let psk = eggsec::distributed::generate_psk();
    // Plaintext loopback listener (isolated lab only). The production
    // 60/min/IP rate limit caps total fresh connections, hence the small
    // `messages` count above. PSK/auth behavior is unchanged.
    let listener = eggsec::distributed::RemoteListener::new_plaintext(psk.clone());
    let port = free_port().await;
    let listener = Arc::new(listener);
    let server = listener.clone();
    let server_handle = tokio::spawn(async move {
        let _ = server.start(port).await;
    });
    // Give the listener a moment to bind (loopback only).
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Keep total fresh connections under the production 60/min/IP loopback
    // rate limit: (warmup + trials) * messages * 3 setups. With the default
    // harness trials=3/warmup=1 this is 4 * 4 * 3 = 48 connections.
    let messages = 4usize;
    let mut walls = Vec::new();
    for trial in 0..warmup() + trials() {
        let start = Instant::now();
        let mut heartbeats = 0usize;
        let mut task_requests = 0usize;
        let mut results_sent = 0usize;
        for i in 0..messages {
            // Current behavior: a fresh client (TCP + auth setup) per message.
            let mut client = eggsec::distributed::RemoteClient::new_plaintext(psk.clone());
            client
                .send_heartbeat(
                    "127.0.0.1",
                    port,
                    format!("perf-worker-{i}"),
                    serde_json::json!({"worker_id": format!("perf-worker-{i}"), "status": "idle"})
                        .to_string(),
                )
                .await
                .expect("heartbeat");
            heartbeats += 1;

            let mut client = eggsec::distributed::RemoteClient::new_plaintext(psk.clone());
            let _tasks = client
                .request_tasks("127.0.0.1", port, format!("perf-worker-{i}"), 5)
                .await
                .expect("request");
            task_requests += 1;

            let mut client = eggsec::distributed::RemoteClient::new_plaintext(psk.clone());
            client
                .send_result(
                    "127.0.0.1",
                    port,
                    eggsec::distributed::TaskResult {
                        task_id: format!("perf-task-{i}"),
                        success: true,
                        output: "ok".to_string(),
                        error: None,
                        duration_millis: 1,
                    },
                )
                .await
                .expect("result");
            results_sent += 1;
        }
        let wall = start.elapsed();
        if trial >= warmup() {
            walls.push(wall.as_secs_f64());
            println!(
                "perf distributed trial={} messages={} wall_ms={} heartbeats={} task_requests={} results={} client_setups={} rss_kb={:?}",
                trial,
                messages,
                wall.as_millis(),
                heartbeats,
                task_requests,
                results_sent,
                heartbeats + task_requests + results_sent,
                peak_rss_kb()
            );
        }
    }
    listener.shutdown();
    server_handle.abort();
    println!(
        "perf distributed summary messages_each={} trials={} warmup={} median_s={:.3} note=fresh-client-per-message-baseline rss_kb={:?}",
        messages,
        trials(),
        warmup(),
        median(walls),
        peak_rss_kb()
    );
}

async fn free_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    port
}
