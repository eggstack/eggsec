use clap::Args;

#[derive(Args, Clone)]
pub struct ProxyInterceptArgs {
    /// Address and port to listen on (e.g. "127.0.0.1:8080")
    #[arg(long, default_value = "127.0.0.1:8080")]
    pub listen: String,

    /// Directory for CA certificate storage
    #[arg(long)]
    pub ca_dir: Option<String>,

    /// Generate CA if missing in ca-dir
    #[arg(long, default_value_t = true)]
    pub generate_ca_if_missing: bool,

    /// Path to user-provided CA certificate (PEM)
    #[arg(long)]
    pub ca_cert: Option<String>,

    /// Path to user-provided CA private key (PEM)
    #[arg(long)]
    pub ca_key: Option<String>,

    /// Dry run: produce complete report without binding server
    #[arg(long)]
    pub dry_run: bool,

    /// Output in JSON format
    #[arg(long)]
    pub json: bool,

    /// Output file path
    #[arg(short, long)]
    pub output: Option<String>,

    /// Maximum number of flows to capture
    #[arg(long, default_value_t = 1000)]
    pub max_flows: u64,

    /// Maximum bytes per flow body (0 = unlimited)
    #[arg(long, default_value_t = 65536)]
    pub max_bytes_per_flow: u64,

    /// Maximum session duration in seconds
    #[arg(long, default_value_t = 300)]
    pub max_duration: u64,

    /// Maximum concurrent connections
    #[arg(long, default_value_t = 100)]
    pub max_concurrent: u32,

    /// Allow traffic interception (required for non-dry-run)
    #[arg(long)]
    pub allow_web_proxy: bool,

    /// Manual override reason for audit trail
    #[arg(long)]
    pub manual_override_reason: Option<String>,

    /// Suppress non-essential output
    #[arg(long)]
    pub quiet: bool,

    /// Intercept rule (repeatable, format: "host:path:action")
    #[arg(long)]
    pub intercept_rule: Vec<String>,

    /// Upstream proxy URL (chain through existing proxy)
    #[arg(long)]
    pub upstream_proxy: Option<String>,
}
