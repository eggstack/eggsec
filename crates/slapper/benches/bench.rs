use slapper::scanner::endpoints::{scan_endpoints, DEFAULT_ENDPOINTS, EndpointScanConfig};
use slapper::scanner::ports::scan_ports_optimized;
use slapper::scanner::spoof::SpoofConfig;
use slapper::scanner::timing::{PortPriority, TimingConfig, TimingPreset};
use slapper::utils::client_pool::ClientPool;
use std::env;
use std::time::{Duration, Instant};

fn print_header(title: &str) {
    println!("\n{}", "=".repeat(60));
    println!("  {}", title);
    println!("{}\n", "=".repeat(60));
}

fn benchmark_timing_config() {
    print_header("Timing Config Creation Benchmark");

    let presets = ["T0", "T1", "T2", "T3", "T4", "T5"];

    for preset in presets {
        let start = Instant::now();
        for _ in 0..10000 {
            let _ = TimingConfig::from_str(preset);
        }
        let elapsed = start.elapsed();
        println!(
            "{:6} - 10k iterations: {:?} ({} ns/op)",
            preset,
            elapsed,
            elapsed.as_nanos() / 10000
        );
    }
}

fn benchmark_port_priority() {
    print_header("Port Priority Categorization Benchmark");

    let test_ports: Vec<u16> = (1..=10000).collect();

    let start = Instant::now();
    for _ in 0..100 {
        let _ = PortPriority::categorize(&test_ports);
    }
    let elapsed = start.elapsed();
    println!("Categorize 10k ports x100: {:?}", elapsed);
    println!("  Per call: {} ns", elapsed.as_nanos() / 100);

    let start = Instant::now();
    for _ in 0..10000 {
        let _ = PortPriority::get_top_ports(100);
    }
    let elapsed = start.elapsed();
    println!("\nGet top 100 ports x10k: {:?}", elapsed);
    println!("  Per call: {} ns", elapsed.as_nanos() / 10000);
}

fn benchmark_client_pool() {
    print_header("Client Pool Creation Benchmark");

    let start = Instant::now();
    for _ in 0..100 {
        let pool = ClientPool::from_config(
            50,
            Duration::from_secs(10),
            false,
            Some("TestAgent/1.0".to_string()),
            None,
        );
        assert_eq!(pool.pool_size(), 50);
    }
    let elapsed = start.elapsed();
    println!("Create 50-client pool x100: {:?}", elapsed);
    println!("  Per pool: {} ms", elapsed.as_millis() / 100);
}

async fn benchmark_port_scan(host: &str, ports: Vec<u16>, timing: TimingConfig, name: &str) {
    println!(
        "\nPort Scan: {} ({} ports, {})",
        name,
        ports.len(),
        timing.preset
    );

    let start = Instant::now();
    let result =
        scan_ports_optimized(host, ports.clone(), timing, false, SpoofConfig::default()).await;

    match result {
        Ok(scan_result) => {
            let elapsed = start.elapsed();
            let rate = ports.len() as f64 / elapsed.as_secs_f64();
            println!("  Duration: {:?}", elapsed);
            println!("  Open ports: {}", scan_result.open_ports.len());
            println!("  Rate: {:.0} ports/second", rate);
        }
        Err(e) => {
            println!("  Error: {}", e);
        }
    }
}

async fn benchmark_endpoint_scan(url: &str, concurrency: usize) {
    println!(
        "\nEndpoint Scan: {} ({} endpoints, {} concurrency)",
        url,
        DEFAULT_ENDPOINTS.len(),
        concurrency
    );

    let endpoints: Vec<String> = DEFAULT_ENDPOINTS.iter().map(|s| s.to_string()).collect();

    let start = Instant::now();
    let result = scan_endpoints(EndpointScanConfig {
        base_url: url.to_string(),
        endpoints,
        concurrency,
        timeout_duration: Duration::from_secs(5),
        include_404: false,
        tui_mode: false,
        spoof_config: SpoofConfig::default(),
        verify_tls: false,
        progress_tx: None,
        max_results: None,
    })
    .await;

    match result {
        Ok(scan_result) => {
            let elapsed = start.elapsed();
            let rate = DEFAULT_ENDPOINTS.len() as f64 / elapsed.as_secs_f64();
            println!("  Duration: {:?}", elapsed);
            println!("  Endpoints found: {}", scan_result.endpoints_found);
            println!("  Rate: {:.1} requests/second", rate);
        }
        Err(e) => {
            println!("  Error: {}", e);
        }
    }
}

#[tokio::main]
async fn main() {
    println!("\n{}", "=".repeat(60));
    println!("  Slapper Performance Benchmarks");
    println!("{}\n", "=".repeat(60));

    // CPU-bound benchmarks
    benchmark_timing_config();
    benchmark_port_priority();
    benchmark_client_pool();

    // Network benchmarks - get target from args or use default
    let args: Vec<String> = env::args().collect();
    let host = args.get(1).map(|s| s.as_str()).unwrap_or("scanme.nmap.org");
    let url = args
        .get(2)
        .map(|s| s.as_str())
        .unwrap_or("http://scanme.nmap.org");

    println!("\n{}", "=".repeat(60));
    println!("  Network Benchmarks (target: {})", host);
    println!("{}\n", "=".repeat(60));

    // Quick port scan test
    let quick_ports = vec![80, 443, 22, 21, 25, 3306, 5432, 6379, 27017, 8080];
    benchmark_port_scan(
        host,
        quick_ports.clone(),
        TimingConfig::from_preset(TimingPreset::Normal),
        "Normal",
    )
    .await;
    benchmark_port_scan(
        host,
        quick_ports.clone(),
        TimingConfig::from_preset(TimingPreset::Aggressive),
        "Aggressive",
    )
    .await;
    benchmark_port_scan(
        host,
        quick_ports,
        TimingConfig::from_preset(TimingPreset::Insane),
        "Insane",
    )
    .await;

    // Larger port scan
    let medium_ports: Vec<u16> = (1..=1000).collect();
    println!("\n");
    benchmark_port_scan(
        host,
        medium_ports,
        TimingConfig::from_preset(TimingPreset::Insane),
        "1000 ports (Insane)",
    )
    .await;

    // Endpoint scan
    benchmark_endpoint_scan(url, 50).await;
    benchmark_endpoint_scan(url, 100).await;

    println!("\n{}", "=".repeat(60));
    println!("  Benchmarks Complete");
    println!("{}\n", "=".repeat(60));
}
