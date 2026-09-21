//! Phase B bounded fan-out regression tests (deterministic, loopback-only).
//!
//! Proves the bounded schedulers preserve result semantics:
//! - port result sorting and `max_results` completion-order selection;
//! - endpoint sorting, `include_404`, and `max_results`;
//! - subdomain result membership through the bounded path;
//! - `concurrency == 0` keeps failing where currently rejected.
//!
//! Loopback port scans opt in through the `EGGSEC_ALLOW_LOOPBACK_FIXTURE=1`
//! fixture marker (same mechanism as `scripts/perf-profile.sh`); the marker
//! is set process-locally by these tests and affects only this test binary.

mod common;

use common::{create_test_server, mock_ok};
use std::time::Duration;

/// Opt in to the loopback fixture for port-scan tests in this binary.
fn allow_loopback_fixture() {
    std::env::set_var("EGGSEC_ALLOW_LOOPBACK_FIXTURE", "1");
}

#[tokio::test]
async fn port_scan_rejects_zero_concurrency() {
    let config = eggsec::scanner::ports::PortScanConfig {
        ports: vec![80],
        concurrency: 0,
        timeout_duration: Duration::from_millis(100),
        tui_mode: true,
        spoof_config: eggsec::scanner::spoof::SpoofConfig::default(),
        progress_tx: None,
        max_results: None,
    };
    let err = eggsec::scanner::ports::scan_ports("127.0.0.1", config)
        .await
        .expect_err("concurrency 0 must fail");
    assert!(err.to_string().contains("concurrency"));
}

#[tokio::test]
async fn endpoint_scan_rejects_zero_concurrency() {
    let config = eggsec::scanner::endpoints::EndpointScanConfig {
        base_url: "http://127.0.0.1:9".to_string(),
        endpoints: vec!["/x".to_string()],
        concurrency: 0,
        timeout_duration: Duration::from_millis(100),
        include_404: false,
        tui_mode: true,
        spoof_config: std::sync::Arc::new(eggsec::scanner::spoof::SpoofConfig::default()),
        verify_tls: true,
        progress_tx: None,
        max_results: None,
    };
    let err = eggsec::scanner::endpoints::scan_endpoints(config)
        .await
        .expect_err("concurrency 0 must fail");
    assert!(err.to_string().contains("concurrency"));
}

#[tokio::test]
async fn port_scan_sorting_and_max_results() {
    allow_loopback_fixture();
    let listener_a = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let port_a = listener_a.local_addr().expect("addr").port();
    let listener_b = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let port_b = listener_b.local_addr().expect("addr").port();
    tokio::spawn(async move {
        loop {
            if listener_a.accept().await.is_err() {
                break;
            }
        }
    });
    tokio::spawn(async move {
        loop {
            if listener_b.accept().await.is_err() {
                break;
            }
        }
    });
    // Deliberately unsorted input with closed ports interleaved.
    let (hi, lo) = (port_a.max(port_b), port_a.min(port_b));
    let ports = vec![hi, 9, lo, 18081];

    let config = eggsec::scanner::ports::PortScanConfig {
        ports: ports.clone(),
        concurrency: 4,
        timeout_duration: Duration::from_millis(300),
        tui_mode: true,
        spoof_config: eggsec::scanner::spoof::SpoofConfig::default(),
        progress_tx: None,
        max_results: None,
    };
    let results = eggsec::scanner::ports::scan_ports("127.0.0.1", config)
        .await
        .expect("scan");
    assert_eq!(results.ports_scanned, ports.len() as u32);
    let open: Vec<u16> = results.open_ports.iter().map(|p| p.port).collect();
    assert!(open.contains(&port_a), "missing {port_a}: {open:?}");
    assert!(open.contains(&port_b), "missing {port_b}: {open:?}");
    let mut sorted = open.clone();
    sorted.sort_unstable();
    assert_eq!(open, sorted, "results must be sorted by port");
    assert_eq!(results.total_open_ports, open.len());

    // Completion-order selection: exactly one winner, total still counts both.
    let config = eggsec::scanner::ports::PortScanConfig {
        ports: vec![port_a, port_b],
        concurrency: 2,
        timeout_duration: Duration::from_millis(300),
        tui_mode: true,
        spoof_config: eggsec::scanner::spoof::SpoofConfig::default(),
        progress_tx: None,
        max_results: Some(1),
    };
    let results = eggsec::scanner::ports::scan_ports("127.0.0.1", config)
        .await
        .expect("scan");
    assert_eq!(results.open_ports.len(), 1);
    assert_eq!(results.total_open_ports, 2);
}

#[tokio::test]
async fn endpoint_scan_sorting_include404_and_max_results() {
    let server = create_test_server().await;
    mock_ok("/api").mount(&server).await;
    mock_ok("/admin").mount(&server).await;
    let base = server.uri();

    let scan = |endpoints: Vec<String>, include_404: bool, max_results: Option<usize>| {
        let base = base.clone();
        async move {
            eggsec::scanner::endpoints::scan_endpoints(
                eggsec::scanner::endpoints::EndpointScanConfig {
                    base_url: base,
                    endpoints,
                    concurrency: 4,
                    timeout_duration: Duration::from_secs(5),
                    include_404,
                    tui_mode: true,
                    spoof_config: std::sync::Arc::new(
                        eggsec::scanner::spoof::SpoofConfig::default(),
                    ),
                    verify_tls: true,
                    progress_tx: None,
                    max_results,
                },
            )
            .await
            .expect("scan")
        }
    };

    let endpoints = vec![
        "/missing-xyz".to_string(),
        "/api".to_string(),
        "/admin".to_string(),
    ];
    let results = scan(endpoints.clone(), false, None).await;
    assert_eq!(results.endpoints_scanned, 3);
    let paths: Vec<&str> = results.results.iter().map(|r| r.path.as_str()).collect();
    assert!(paths.contains(&"/api"));
    assert!(paths.contains(&"/admin"));
    assert!(!paths.contains(&"/missing-xyz"));
    // Final sort: interesting first, then status, then path.
    let mut expected = results.results.clone();
    expected.sort_by(|a, b| {
        b.interesting
            .cmp(&a.interesting)
            .then_with(|| a.status_code.cmp(&b.status_code))
            .then_with(|| a.path.cmp(&b.path))
    });
    let order: Vec<&str> = results.results.iter().map(|r| r.path.as_str()).collect();
    let expected_order: Vec<&str> = expected.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(order, expected_order);

    // include_404 surfaces the missing path.
    let results = scan(endpoints.clone(), true, None).await;
    assert!(results.results.iter().any(|r| r.path == "/missing-xyz"));

    // max_results truncates the admitted set.
    let results = scan(endpoints, false, Some(1)).await;
    assert_eq!(results.results.len(), 1);
    assert_eq!(results.total_endpoints_matched, 2);
}

#[tokio::test]
async fn subdomain_bruteforce_bounded_path_empty_domain() {
    // `.invalid` never resolves (RFC 6761): exercises the full bounded
    // JoinSet path without depending on live DNS data.
    let result = eggsec::recon::subdomain::bruteforce_subdomains(
        "invalid",
        &["www".to_string(), "api".to_string(), "mail".to_string()],
        4,
    )
    .await
    .expect("bruteforce");
    assert_eq!(result.domain, "invalid");
    assert!(result.subdomains.is_empty());
    assert_eq!(result.sources, vec!["dns-bruteforce".to_string()]);
}
