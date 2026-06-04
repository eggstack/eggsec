pub(crate) const BROWSER_ABOUT: &str = "Run headless browser security testing

Performs DOM XSS detection, SPA route discovery, and client-side security checks
using a headless Chrome instance. Feature-gated behind headless-browser.

Examples:
  slapper browser https://example.com
  slapper browser https://example.com --no-dom-xss
  slapper browser https://example.com --no-spa --no-client-checks
  slapper browser https://example.com --timeout 120000
  slapper browser https://example.com --json
  slapper browser https://example.com -o results.json";

#[derive(clap::Args)]
pub struct BrowserArgs {
    #[arg(help = "Target URL to scan")]
    pub target: String,

    #[arg(long, help = "Skip DOM XSS detection")]
    pub no_dom_xss: bool,

    #[arg(long, help = "Skip SPA route discovery")]
    pub no_spa: bool,

    #[arg(long, help = "Skip client-side security checks")]
    pub no_client_checks: bool,

    #[arg(long, default_value_t = crate::constants::DEFAULT_BROWSER_TIMEOUT_MS, help = "Navigation timeout in milliseconds")]
    pub timeout: u64,

    #[arg(long, help = "Custom XSS payload (default: <img src=x onerror=alert(1)>)")]
    pub xss_payload: Option<String>,

    #[arg(long, help = "Output results as JSON")]
    pub json: bool,

    #[arg(long, short = 'o', help = "Output to file")]
    pub output: Option<String>,

    #[arg(long, short = 'q', help = "Suppress non-essential output")]
    pub quiet: bool,
}
