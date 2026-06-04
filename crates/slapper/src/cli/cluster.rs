pub(crate) const CLUSTER_ABOUT: &str = "Manage distributed scanning cluster

Starts worker or coordinator nodes for distributed scanning.
Workers execute tasks, coordinators manage job distribution.

Examples:
  slapper cluster worker --workers 4
  slapper cluster coordinator --port 9000
  slapper cluster status";

#[derive(clap::Args)]
pub struct ClusterArgs {
    #[command(subcommand)]
    pub command: ClusterCommand,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Suppress non-essential output
    #[arg(short, long)]
    pub quiet: bool,
}

#[derive(clap::Subcommand)]
pub enum ClusterCommand {
    #[command(about = "Start a worker node")]
    Worker(ClusterWorkerArgs),
    #[command(about = "Start a coordinator node")]
    Coordinator(ClusterCoordinatorArgs),
    #[command(about = "Show cluster status")]
    Status(ClusterStatusArgs),
    #[command(about = "Enqueue a task for workers to execute")]
    AddTask(ClusterAddTaskArgs),
}

#[derive(clap::Args)]
pub struct ClusterWorkerArgs {
    #[arg(long, default_value = "localhost:9000", help = "Coordinator address")]
    pub coordinator: String,
    #[arg(long, default_value = "4", help = "Number of worker threads")]
    pub workers: usize,
    #[arg(long, help = "Worker ID (auto-generated if not set)")]
    pub worker_id: Option<String>,
    #[arg(long, help = "Pre-shared key for authentication")]
    pub psk: Option<String>,
    #[arg(long, default_value = "30", help = "Heartbeat interval in seconds")]
    pub heartbeat_interval: u64,
}

#[derive(clap::Args)]
pub struct ClusterCoordinatorArgs {
    #[arg(long, default_value = "9000", help = "Port to listen on")]
    pub port: u16,
    #[arg(long, help = "Bind address (default: 0.0.0.0)")]
    pub bind: Option<String>,
    #[arg(long, help = "Maximum workers")]
    pub max_workers: Option<usize>,
    #[arg(long, help = "Pre-shared key for worker authentication")]
    pub psk: Option<String>,
}

#[derive(clap::Args)]
pub struct ClusterStatusArgs {
    #[arg(long, help = "Coordinator address (for remote status)")]
    pub coordinator: Option<String>,
}

#[derive(clap::Args)]
pub struct ClusterAddTaskArgs {
    #[arg(long, help = "Coordinator address (host:port)")]
    pub coordinator: String,
    #[arg(long, help = "Pre-shared key for authentication")]
    pub psk: Option<String>,
    #[arg(long, help = "Task type (PortScan, ServiceFingerprint, EndpointDiscovery, Fuzz, WafTest, LoadTest, Recon)")]
    pub task_type: String,
    #[arg(long, help = "Target to scan")]
    pub target: String,
    #[arg(long, help = "Task payload as JSON string")]
    pub payload: Option<String>,
    #[arg(long, help = "Job ID (auto-generated if not set)")]
    pub job_id: Option<String>,
}
