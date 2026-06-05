use super::timeout::HUNT_TIMEOUT;
use super::CommonHttpArgs;

pub(crate) const HUNT_ABOUT: &str = "Run advanced vulnerability hunting against a target. \
    Performs attack chain analysis, business logic flaw detection, race condition testing, \
    authorization bypass testing, and session security analysis.";

#[derive(clap::Args)]
pub struct HuntArgs {
    #[arg(help = "Target URL to hunt (e.g., https://example.com)")]
    pub target: String,

    #[arg(long, help = "Skip attack chain detection")]
    pub skip_chains: bool,

    #[arg(long, help = "Skip business logic checks")]
    pub skip_business: bool,

    #[arg(long, help = "Skip race condition checks")]
    pub skip_race: bool,

    #[arg(long, help = "Skip authorization bypass checks")]
    pub skip_authz: bool,

    #[arg(long, help = "Skip session security checks")]
    pub skip_session: bool,

    #[arg(long, default_value_t = 10, help = "Max concurrent requests")]
    pub concurrency: usize,

    #[arg(long, default_value_t = HUNT_TIMEOUT, help = "Per-request timeout in seconds")]
    pub timeout: u64,

    #[arg(long, help = "Output format (json, pretty, csv, html, markdown, sarif, junit)")]
    pub format: Option<String>,

    #[arg(long, help = "Output file path (defaults to stdout for pretty/json)")]
    pub output: Option<String>,

    #[command(flatten)]
    pub common: CommonHttpArgs,

    #[arg(long, help = "Output in JSON format")]
    pub json: bool,
}
