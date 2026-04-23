pub(crate) const STORAGE_ABOUT: &str = "Database storage and query operations

Manage scan results and findings in PostgreSQL.

Examples:
  slapper storage query --sql 'SELECT * FROM scans'
  slapper storage export --scan-id <id>
  slapper storage stats";

#[derive(clap::Args)]
pub struct StorageArgs {
    #[command(subcommand)]
    pub command: StorageCommand,
}

#[derive(clap::Subcommand)]
pub enum StorageCommand {
    #[command(about = "Execute a SQL query against the database")]
    Query(StorageQueryArgs),
    #[command(about = "Export scan results to JSON")]
    Export(StorageExportArgs),
    #[command(about = "Show database statistics")]
    Stats(StorageStatsArgs),
    #[command(about = "Initialize the database schema")]
    Init(StorageInitArgs),
}

#[derive(clap::Args)]
pub struct StorageQueryArgs {
    #[arg(long, help = "SQL query to execute")]
    pub sql: Option<String>,
    #[arg(long, help = "Query type", default_value = "recent_scans")]
    pub query: Option<String>,
    #[arg(long, help = "Limit results")]
    pub limit: Option<usize>,
}

#[derive(clap::Args)]
pub struct StorageExportArgs {
    #[arg(long, help = "Scan ID to export")]
    pub scan_id: Option<String>,
    #[arg(long, help = "Finding ID to export")]
    pub finding_id: Option<String>,
    #[arg(short = 'o', long, help = "Output file path")]
    pub output: Option<String>,
    #[arg(long, help = "Export format", default_value = "json")]
    pub format: String,
}

#[derive(clap::Args)]
pub struct StorageStatsArgs {
    #[arg(long, help = "Show statistics for specific scan ID")]
    pub scan_id: Option<String>,
}

#[derive(clap::Args)]
pub struct StorageInitArgs {
    #[arg(long, help = "Drop existing tables first")]
    pub force: bool,
}