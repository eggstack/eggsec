pub(crate) const STORAGE_ABOUT: &str = "Database storage and query operations

Manage scan results and findings in PostgreSQL.

Connection defaults to localhost:5432 database 'eggsec' as user 'postgres'.
The password is read from the EGGSEC_STORAGE_PASSWORD environment variable --
never from a flag, so it does not land in shell history or `ps` output.

Examples:
  eggsec storage stats
  EGGSEC_STORAGE_PASSWORD=secret eggsec storage query --query recent_scans
  eggsec storage export --scan-id <id> --host db.internal --username eggsec
  eggsec storage init --host db.internal --password-env DB_PASSWORD";

/// Connection settings shared by every `eggsec storage` subcommand.
///
/// Flattened into each subcommand rather than declared once on `StorageArgs`
/// so that `eggsec storage stats --host ...` works: Clap places global
/// options before the subcommand, which is the wrong ergonomics for a
/// per-invocation connection.
#[derive(clap::Args, Clone)]
pub struct StorageConnectionArgs {
    #[arg(
        long,
        default_value = eggsec_tool_core::operation_request::DEFAULT_STORAGE_HOST,
        help = "Database host"
    )]
    pub host: String,
    #[arg(
        long,
        default_value_t = eggsec_tool_core::operation_request::DEFAULT_STORAGE_PORT,
        help = "Database port"
    )]
    pub port: u16,
    #[arg(
        long,
        default_value = eggsec_tool_core::operation_request::DEFAULT_STORAGE_DATABASE,
        help = "Database name"
    )]
    pub database: String,
    #[arg(
        long,
        default_value = eggsec_tool_core::operation_request::DEFAULT_STORAGE_USERNAME,
        help = "Database user"
    )]
    pub username: String,
    #[arg(
        long,
        default_value = "EGGSEC_STORAGE_PASSWORD",
        help = "Name of the environment variable holding the password (the value is never read from a flag)"
    )]
    pub password_env: String,
    #[arg(
        long,
        default_value_t = eggsec_tool_core::operation_request::DEFAULT_STORAGE_MAX_CONNECTIONS,
        help = "Maximum database connections"
    )]
    pub max_connections: u32,
}

impl StorageConnectionArgs {
    /// Build the engine config, resolving the password from the environment.
    pub fn resolve(&self) -> crate::storage::StorageConfig {
        crate::storage::resolve_config(
            self.host.clone(),
            self.port,
            self.database.clone(),
            self.username.clone(),
            Some(self.password_env.as_str()),
            self.max_connections,
        )
    }
}

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
    #[command(flatten)]
    pub conn: StorageConnectionArgs,
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
    #[command(flatten)]
    pub conn: StorageConnectionArgs,
}

#[derive(clap::Args)]
pub struct StorageStatsArgs {
    #[arg(long, help = "Show statistics for specific scan ID")]
    pub scan_id: Option<String>,
    #[command(flatten)]
    pub conn: StorageConnectionArgs,
}

#[derive(clap::Args)]
pub struct StorageInitArgs {
    #[arg(long, help = "Drop existing tables first")]
    pub force: bool,
    #[command(flatten)]
    pub conn: StorageConnectionArgs,
}
