pub(crate) const WIRELESS_ABOUT: &str = "Scan wireless networks for security issues

Performs iwlist-based wireless network enumeration and security analysis.
Detects Open, WEP, WPA, WPA2, WPA3, and Enterprise (802.1X) networks.
Generates vulnerability findings and security recommendations.
NOTE: Requires building with --features wireless and root privileges for iwlist scan

Examples:
  eggsec wireless wlan0
  eggsec wireless wlan0 --json
  eggsec wireless wlan0 -o results.json
  eggsec wireless wlan0 --duration 15";

#[derive(clap::Args)]
pub struct WirelessArgs {
    #[arg(help = "Wireless interface name (e.g., wlan0)")]
    pub interface: String,
    #[arg(long, help = "Output results as JSON")]
    pub json: bool,
    #[arg(long, short = 'o', help = "Output to file")]
    pub output: Option<String>,
    #[arg(long, short = 'q', help = "Suppress non-essential output")]
    pub quiet: bool,
    #[arg(long, default_value_t = 10, help = "Scan duration in seconds")]
    pub duration: u64,
}
