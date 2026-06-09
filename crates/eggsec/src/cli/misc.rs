pub(crate) const REMOTE_ABOUT: &str =
    "MODE: Hazardous Lab | REQUIRED: --scope, explicit authorization

Start remote listener for distributed commands

Listens for remote commands from coordinated scanner nodes.
For authorized distributed scanning infrastructure only.
Requires explicit scope and authorization before execution.

Examples:
  eggsec remote start --port 7890 --auth <psk>
  eggsec remote start --port 7890 --tls-cert cert.pem --tls-key key.pem
  eggsec remote generate-key";

pub(crate) const EXEC_ABOUT: &str =
    "MODE: Hazardous Lab | REQUIRED: --scope, explicit authorization

Execute commands on remote systems

Sends commands to a remote listener for execution.
For authorized distributed scanning infrastructure only.
Requires explicit scope and authorization before execution.

Examples:
  eggsec exec --target 192.168.1.100:7890 --auth <psk> -- ls -la
  eggsec exec --target 10.0.0.5:7890 --auth <psk> -- cat /etc/hosts";

pub(crate) const NOTIFY_ABOUT: &str = "Test and manage notifications

Tests webhook integrations and sends test notifications.
Supports Slack, Discord, Teams, and custom webhooks.

Examples:
  eggsec notify test --slack
  eggsec notify test --discord
  eggsec notify test --webhook https://example.com/hook
  eggsec notify send --finding 'SQL Injection found'";

pub(crate) const CONFIG_ABOUT: &str = "Validate and inspect configuration

Validates configuration files for syntax errors and consistency.
Shows effective configuration when used with --show.

Examples:
  eggsec config validate
  eggsec config validate --config /path/to/config.toml
  eggsec config show";

pub(crate) const DOCTOR_ABOUT: &str = "Check system and runtime dependencies

Verifies that all required dependencies are available for the enabled features.
Reports status of Python, Ruby, Lua, and other optional runtime dependencies.

This command exits with code 0 if all checks pass, or a non-zero code if any check fails.

Examples:
  eggsec doctor
  eggsec doctor --verbose";

#[derive(clap::Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,

    #[arg(short = 'o', long, help = "Output results to file")]
    pub output: Option<String>,
}

#[derive(clap::Subcommand)]
pub enum ConfigCommand {
    #[command(about = "Validate configuration file syntax")]
    Validate(ConfigValidateArgs),
    #[command(about = "Show effective configuration")]
    Show(ConfigShowArgs),
}

#[derive(clap::Args)]
pub struct ConfigValidateArgs {
    #[arg(long, help = "Configuration file path")]
    pub config: Option<String>,
}

#[derive(clap::Args)]
pub struct ConfigShowArgs {
    #[arg(long, help = "Configuration file path")]
    pub config: Option<String>,
}

#[derive(clap::Args)]
pub struct NotifyArgs {
    #[command(subcommand)]
    pub command: NotifyCommand,

    /// Suppress non-essential output
    #[arg(long, short = 'q')]
    pub quiet: bool,

    #[arg(short = 'o', long, help = "Output results to file")]
    pub output: Option<String>,
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

    #[arg(short = 'o', long, help = "Output results to file")]
    pub output: Option<String>,
}

#[derive(clap::Subcommand)]
pub enum RemoteCommand {
    #[command(about = "Generate a new PSK")]
    GenerateKey,
    #[command(about = "Generate TLS certificate for distributed communication")]
    Cert(CertArgs),
    #[command(about = "Start remote listener")]
    Start(RemoteStartArgs),
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
    #[arg(long, short = 'y', help = "Skip confirmation prompt")]
    pub yes: bool,
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
    #[arg(long, short = 'y', help = "Skip confirmation prompt")]
    pub yes: bool,
    #[arg(help = "The command to execute")]
    pub command: Vec<String>,

    #[arg(short = 'o', long, help = "Output results to file")]
    pub output: Option<String>,
}

#[derive(clap::Args)]
pub struct ReportArgs {
    #[command(subcommand)]
    pub command: ReportCommand,

    #[arg(short = 'o', long, help = "Output results to file")]
    pub output: Option<String>,
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
    #[arg(long, help = "TLS certificate file (PEM format)")]
    pub tls_cert: Option<String>,
    #[arg(long, help = "TLS private key file (PEM format)")]
    pub tls_key: Option<String>,
    #[arg(long, help = "Scope file for target validation")]
    pub scope_file: Option<String>,
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
    #[arg(
        long,
        default_value = "ops-agent",
        help = "MCP profile (ops-agent or coding-agent)"
    )]
    pub profile: String,
}

#[derive(clap::Args)]
pub struct CodeggMcpArgs {
    #[arg(long, default_value = "8081", help = "Port to listen on")]
    pub port: u16,
    #[arg(long, default_value = "127.0.0.1", help = "Address to bind to")]
    pub bind: String,
    #[arg(long, help = "API key for authentication")]
    pub api_key: Option<String>,
    #[arg(
        long,
        default_value_t = true,
        help = "Enable stdio mode (default for codegg-mcp)"
    )]
    pub stdio: bool,
    #[arg(
        long,
        default_value = "coding-agent",
        help = "MCP profile (coding-agent recommended for codegg-mcp)"
    )]
    pub profile: String,
}

#[derive(clap::Args)]
pub struct SbomArgs {
    #[command(subcommand)]
    pub command: SbomCommand,
}

#[derive(clap::Subcommand)]
pub enum SbomCommand {
    #[command(about = "Generate SBOM from project")]
    Generate(SbomGenerateArgs),
    #[command(about = "Check for typosquatting risks")]
    CheckTyposquat(SbomTyposquatArgs),
}

#[derive(clap::Args)]
pub struct SbomGenerateArgs {
    #[arg(help = "Project directory path")]
    pub project: String,
    #[arg(
        long,
        default_value = "cyclonedx",
        help = "Output format: cyclonedx, spdx, json"
    )]
    pub format: String,
    #[arg(long, short = 'o', help = "Output file path")]
    pub output: Option<String>,
}

#[derive(clap::Args)]
pub struct SbomTyposquatArgs {
    #[arg(help = "Project directory path")]
    pub project: String,
    #[arg(long, default_value = "0.7", help = "Similarity threshold (0.0-1.0)")]
    pub threshold: f64,
}
