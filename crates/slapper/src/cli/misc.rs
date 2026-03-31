pub(crate) const NOTIFY_ABOUT: &str = "Test and manage notifications

Tests webhook integrations and sends test notifications.
Supports Slack, Discord, Teams, and custom webhooks.

Examples:
  slapper notify test --slack
  slapper notify test --discord
  slapper notify test --webhook https://example.com/hook
  slapper notify send --finding 'SQL Injection found'";

#[derive(clap::Args)]
pub struct NotifyArgs {
    #[command(subcommand)]
    pub command: NotifyCommand,
}

#[derive(clap::Subcommand)]
pub enum NotifyCommand {
    #[command(about = "Send a test notification")]
    Test(NotifyTestArgs),
    #[command(about = "Send a notification")]
    Send(NotifySendArgs),
}

#[derive(clap::Args)]
pub struct NotifyTestArgs {
    #[arg(long, help = "Test Slack webhook")]
    pub slack: Option<String>,
    #[arg(long, help = "Test Discord webhook")]
    pub discord: Option<String>,
    #[arg(long, help = "Test Teams webhook")]
    pub teams: Option<String>,
    #[arg(long, help = "Test custom webhook")]
    pub webhook: Option<String>,
    #[arg(long, help = "Webhook secret for custom webhook")]
    pub secret: Option<String>,
}

#[derive(clap::Args)]
pub struct NotifySendArgs {
    #[arg(help = "Message to send")]
    pub message: String,
    #[arg(long, help = "Send to Slack")]
    pub slack: Option<String>,
    #[arg(long, help = "Send to Discord")]
    pub discord: Option<String>,
    #[arg(long, help = "Send to Teams")]
    pub teams: Option<String>,
    #[arg(long, help = "Send to custom webhook")]
    pub webhook: Option<String>,
    #[arg(long, help = "Finding severity (critical/high/medium/low)")]
    pub severity: Option<String>,
    #[arg(long, help = "Target that was scanned")]
    pub target: Option<String>,
}

#[derive(clap::Args)]
pub struct RemoteArgs {
    #[command(subcommand)]
    pub command: RemoteCommand,
}

#[derive(clap::Subcommand)]
pub enum RemoteCommand {
    #[command(about = "Generate a new PSK")]
    GenerateKey,
    #[command(about = "Generate TLS certificate for distributed communication")]
    Cert(CertArgs),
    #[command(about = "Start remote listener")]
    Start(RemoteStartArgs),
    #[command(about = "Stop remote listener")]
    Stop,
}

#[derive(clap::Args)]
pub struct CertArgs {
    #[arg(long, help = "Print instructions for creating TLS cert with openssl")]
    pub openssl: bool,
}

#[derive(clap::Args)]
pub struct RemoteStartArgs {
    #[arg(long, default_value = "7890", help = "Port to listen on")]
    pub port: u16,
    #[arg(long, help = "Pre-shared key for authentication")]
    pub auth: Option<String>,
    #[arg(long, help = "TLS certificate file (PEM format)")]
    pub tls_cert: Option<String>,
    #[arg(long, help = "TLS private key file (PEM format)")]
    pub tls_key: Option<String>,
}

#[derive(clap::Args)]
pub struct ExecArgs {
    #[arg(long, help = "Single target (host:port)")]
    pub target: Option<String>,
    #[arg(long, help = "File containing list of targets (one per line)")]
    pub targets: Option<String>,
    #[arg(long, help = "Pre-shared key for authentication")]
    pub auth: Option<String>,
    #[arg(long, default_value = "60", help = "Timeout in seconds")]
    pub timeout: u64,
    #[arg(long, help = "TLS certificate file (PEM format)")]
    pub tls_cert: Option<String>,
    #[arg(long, help = "TLS private key file (PEM format)")]
    pub tls_key: Option<String>,
    #[arg(
        long,
        default_value = "localhost",
        help = "TLS domain for certificate verification"
    )]
    pub tls_domain: Option<String>,
    #[arg(help = "The command to execute")]
    pub command: Vec<String>,
}

#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
#[derive(clap::Args)]
pub struct PluginArgs {
    #[command(subcommand)]
    pub command: PluginCommand,
}

#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
#[derive(clap::Subcommand)]
pub enum PluginCommand {
    #[command(about = "List available plugins")]
    List(PluginListArgs),
    #[command(about = "Run a plugin against a target")]
    Run(PluginRunArgs),
}

#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
#[derive(clap::Args)]
pub struct PluginListArgs {
    #[arg(short = 'v', long, help = "Show verbose plugin information")]
    pub verbose: bool,
}

#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
#[derive(clap::Args)]
pub struct PluginRunArgs {
    #[arg(help = "Plugin name to run")]
    pub name: String,
    #[arg(help = "Target URL or host")]
    pub target: String,
    #[arg(short = 'o', long, help = "Output results to file")]
    pub output: Option<String>,
}

#[derive(clap::Args)]
pub struct ReportArgs {
    #[command(subcommand)]
    pub command: ReportCommand,
}

#[derive(clap::Subcommand)]
pub enum ReportCommand {
    #[command(about = "Convert scan results between formats")]
    Convert(ReportConvertArgs),
    #[command(about = "Analyze trends between multiple scan results")]
    Trend(ReportTrendArgs),
    #[command(about = "Manage scheduled scans")]
    Schedule(ScheduleArgs),
}

#[derive(clap::Args)]
pub struct ReportConvertArgs {
    #[arg(help = "Input scan results file (JSON)")]
    pub input: String,
    #[arg(short = 'f', long, help = "Output format", value_enum)]
    pub format: ReportFormat,
    #[arg(short = 'o', long, help = "Output file (stdout if not specified)")]
    pub output: Option<String>,
}

#[derive(clap::ValueEnum, Clone)]
pub enum ReportFormat {
    Json,
    Csv,
    Junit,
    Sarif,
    Html,
    Markdown,
}

#[derive(clap::Args)]
pub struct ReportTrendArgs {
    #[arg(help = "Previous scan results file")]
    pub before: String,
    #[arg(help = "Recent scan results file")]
    pub after: String,
    #[arg(short = 'o', long, help = "Output file (stdout if not specified)")]
    pub output: Option<String>,
}

#[derive(clap::Args)]
pub struct ScheduleArgs {
    #[command(subcommand)]
    pub command: ScheduleCommand,
}

#[derive(clap::Subcommand)]
pub enum ScheduleCommand {
    #[command(about = "List scheduled scans")]
    List,
    #[command(about = "Add a new scheduled scan")]
    Add(ScheduleAddArgs),
    #[command(about = "Remove a scheduled scan")]
    Remove(ScheduleRemoveArgs),
    #[command(about = "Generate crontab entry for scheduled scan")]
    Cron(ScheduleCronArgs),
}

#[derive(clap::Args)]
pub struct ScheduleAddArgs {
    #[arg(help = "Cron schedule expression (e.g., '0 */6 * * *')")]
    pub schedule: String,
    #[arg(help = "Target host or URL")]
    pub target: String,
    #[arg(long, help = "Scan type", default_value = "scan")]
    pub scan_type: String,
    #[arg(short = 'o', long, help = "Output results to file")]
    pub output: Option<String>,
}

#[derive(clap::Args)]
pub struct ScheduleRemoveArgs {
    #[arg(help = "Schedule ID to remove")]
    pub id: String,
}

#[derive(clap::Args)]
pub struct ScheduleCronArgs {
    #[arg(
        help = "Schedule ID to generate crontab entry for (optional, generates all if not specified)"
    )]
    pub id: Option<String>,
}

#[cfg(feature = "rest-api")]
#[derive(clap::Args)]
pub struct ServeArgs {
    #[arg(long, default_value = "8080", help = "Port to listen on")]
    pub port: u16,
    #[arg(long, default_value = "127.0.0.1", help = "Address to bind to")]
    pub bind: String,
    #[arg(long, help = "API key for authentication")]
    pub api_key: Option<String>,
}

#[cfg(feature = "rest-api")]
#[derive(clap::Args)]
pub struct McpServeArgs {
    #[arg(long, default_value = "8081", help = "Port to listen on")]
    pub port: u16,
    #[arg(long, default_value = "127.0.0.1", help = "Address to bind to")]
    pub bind: String,
    #[arg(long, help = "API key for authentication")]
    pub api_key: Option<String>,
    #[arg(long, help = "Enable stdio mode for AI assistant integration")]
    pub stdio: bool,
}
