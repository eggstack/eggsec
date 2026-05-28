#[cfg(feature = "stress-testing")]
use clap::ValueEnum;

#[cfg(feature = "stress-testing")]
pub(crate) const ICMP_ABOUT: &str = "Send ICMP echo probes to target host

Performs ICMP ping to measure reachability and round-trip time.
Requires root privileges for raw ICMP sockets.
NOTE: Requires building with --features stress-testing

Examples:
  slapper icmp 8.8.8.8
  slapper icmp example.com -c 10
  slapper icmp 192.168.1.1 --timeout 5 --json";

#[cfg(feature = "stress-testing")]
pub(crate) const TRACEROUTE_ABOUT: &str = "Trace network path to target host

Performs traceroute to discover the path packets take to reach a destination.
Supports both UDP and ICMP modes.
NOTE: Requires building with --features stress-testing

Examples:
  slapper traceroute 8.8.8.8
  slapper traceroute example.com --icmp
  slapper traceroute 192.168.1.1 --max-hops 30";

#[cfg(feature = "stress-testing")]
pub(crate) const STRESS_ABOUT: &str = "Run stress/load testing against target

Performs various stress testing techniques including SYN, UDP, HTTP, TCP, and ICMP floods.
WARNING: Only use on systems you own or have explicit permission to test.
NOTE: Requires building with --features stress-testing

Examples:
  slapper stress example.com --type http -r 1000 -d 60
  slapper stress example.com --type syn -r 5000 -d 30
  slapper stress 192.168.1.1:80 --type udp -r 10000 -d 120";

#[cfg(feature = "stress-testing")]
pub(crate) const PROXY_ABOUT: &str = "Manage proxy pool and rotation

Manages proxy lists for scan distribution and stealth.
Supports SOCKS4, SOCKS5, HTTP, HTTPS, and Tor proxies.
NOTE: Requires building with --features stress-testing

Examples:
  slapper proxy add --file proxies.txt
  slapper proxy list --healthy
  slapper proxy health-check
  slapper proxy rotate";

#[cfg(feature = "stress-testing")]
#[derive(clap::Args)]
pub struct IcmpArgs {
    #[arg(help = "Target host or IP address")]
    pub target: String,
    #[arg(
        short = 'c',
        long,
        default_value = "4",
        help = "Number of ping requests"
    )]
    pub count: u32,
    #[arg(short = 'W', long, default_value = "2", help = "Timeout in seconds")]
    pub timeout: u64,
    #[arg(
        short = 'i',
        long,
        default_value = "1",
        help = "Interval between probes in seconds"
    )]
    pub interval: f64,
    #[arg(long, help = "Output results as JSON")]
    pub json: bool,
}

#[cfg(feature = "stress-testing")]
#[derive(clap::Args)]
pub struct TracerouteArgs {
    #[arg(help = "Target host or IP address")]
    pub target: String,
    #[arg(long, default_value = "30", help = "Maximum number of hops")]
    pub max_hops: u8,
    #[arg(long, default_value = "3", help = "Timeout in seconds")]
    pub timeout: u64,
    #[arg(long, help = "Use ICMP probes (requires root/sudo)")]
    pub icmp: bool,
    #[arg(long, help = "Use UDP probes (default, no root required)")]
    pub udp: bool,
    #[arg(long, help = "Run probes in parallel")]
    pub parallel: bool,
    #[arg(long, help = "Disable reverse DNS lookup")]
    pub no_resolve: bool,
    #[arg(long, help = "Output results as JSON")]
    pub json: bool,
}

#[cfg(feature = "stress-testing")]
#[derive(clap::Args)]
pub struct StressArgs {
    #[arg(help = "Target host or IP:port")]
    pub target: String,
    #[arg(
        long,
        default_value = "http",
        help = "Stress type: syn, udp, http, tcp, icmp"
    )]
    pub stress_type: StressTypeArg,
    #[arg(
        short = 'r',
        long,
        default_value = "1000",
        help = "Rate in packets/requests per second"
    )]
    pub rate: u64,
    #[arg(short = 'd', long, default_value = "60", help = "Duration in seconds")]
    pub duration: u64,
    #[arg(
        short = 'c',
        long,
        default_value = "10",
        help = "Concurrency (number of concurrent connections)"
    )]
    pub concurrency: usize,
    #[arg(long, help = "Source port")]
    pub src_port: Option<u16>,
    #[arg(long, help = "Spoof source IP address")]
    pub source_ip_spoof: bool,
    #[arg(long, help = "Spoof source IP from CIDR range")]
    pub source_ip_range: Option<String>,
    #[arg(long, help = "Random source port for each request")]
    pub random_port: bool,
    #[arg(long, help = "Payload size in bytes (for UDP)")]
    pub payload_size: Option<usize>,
    #[arg(long, help = "Use proxy pool")]
    pub use_proxies: bool,
    #[arg(long, help = "Proxy pool file")]
    pub proxy_file: Option<String>,
    #[arg(long, help = "Output results as JSON")]
    pub json: bool,
    #[arg(long, short = 'y', help = "Skip confirmation prompt")]
    pub yes: bool,
    #[arg(long, short = 'q', help = "Suppress non-essential output")]
    pub quiet: bool,
    #[arg(long, short = 'o', help = "Output file path")]
    pub output: Option<String>,
}

#[cfg(feature = "stress-testing")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum StressTypeArg {
    Syn,
    Udp,
    Http,
    Tcp,
    Icmp,
}

#[cfg(feature = "stress-testing")]
impl std::fmt::Display for StressTypeArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StressTypeArg::Syn => write!(f, "syn"),
            StressTypeArg::Udp => write!(f, "udp"),
            StressTypeArg::Http => write!(f, "http"),
            StressTypeArg::Tcp => write!(f, "tcp"),
            StressTypeArg::Icmp => write!(f, "icmp"),
        }
    }
}

#[cfg(feature = "stress-testing")]
#[derive(clap::Args)]
pub struct ProxyArgs {
    #[command(subcommand)]
    pub command: ProxyCommand,
}

#[cfg(feature = "stress-testing")]
#[derive(clap::Subcommand)]
pub enum ProxyCommand {
    #[command(about = "Add proxies from file")]
    Add(ProxyAddArgs),
    #[command(about = "List available proxies")]
    List(ProxyListArgs),
    #[command(about = "Check health of all proxies")]
    HealthCheck(ProxyHealthArgs),
    #[command(about = "Test a single proxy")]
    Test(ProxyTestArgs),
}

#[cfg(feature = "stress-testing")]
#[derive(clap::Args)]
pub struct ProxyAddArgs {
    #[arg(
        help = "Path to proxy file (one proxy per line, format: type://host:port or type://user:pass@host:port)"
    )]
    pub file: String,
    #[arg(long, help = "Proxy type (if not specified in file)")]
    pub proxy_type: Option<ProxyTypeArg>,
}

#[cfg(feature = "stress-testing")]
#[derive(clap::Args)]
pub struct ProxyListArgs {
    #[arg(long, help = "Show only healthy proxies")]
    pub healthy: bool,
    #[arg(long, help = "Show proxy details")]
    pub verbose: bool,
}

#[cfg(feature = "stress-testing")]
#[derive(clap::Args)]
pub struct ProxyHealthArgs {
    #[arg(
        long,
        default_value = "https://google.com",
        help = "URL to check proxy health"
    )]
    pub test_url: String,
    #[arg(long, default_value = "10", help = "Timeout in seconds")]
    pub timeout: u64,
}

#[cfg(feature = "stress-testing")]
#[derive(clap::Args)]
pub struct ProxyTestArgs {
    #[arg(help = "Proxy to test (format: type://host:port)")]
    pub proxy: String,
    #[arg(long, default_value = "https://google.com", help = "URL to test proxy")]
    pub test_url: String,
}

#[cfg(feature = "stress-testing")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ProxyTypeArg {
    Socks4,
    Socks5,
    Http,
    Https,
    Tor,
}
