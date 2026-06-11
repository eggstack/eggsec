pub(crate) const WIRELESS_ABOUT: &str = "Scan wireless networks for security issues

Performs iwlist-based wireless network enumeration and security analysis.
Detects Open, WEP, WPA, WPA2, WPA3, and Enterprise (802.1X) networks.
Detects WPS, hidden SSIDs, WPA2/WPA3 transition modes, weak signals, and basic rogue/Evil-Twin candidates (same SSID, differing BSSID/security).
Generates vulnerability findings and security recommendations.
NOTE: Requires building with --features wireless and root (or CAP_NET_ADMIN) privileges for iwlist scan.
Interface must be in managed mode and up. Use only on authorized networks in lab/defense-validation contexts.
This is passive reconnaissance only. Run repeated scans for change/rogue observation.

Examples:
  eggsec wireless wlan0
  eggsec wireless wlan0 --json
  eggsec wireless wlan0 -o results.json
  eggsec wireless wlan0 --duration 15
  eggsec wireless wlan0 --repeat 5";

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
    #[arg(long, default_value_t = 1, help = "Number of scans to perform (repeat for change/rogue observation; basic)")]
    pub repeat: u32,
    #[arg(long, help = "Enable verbose suspicious/rogue network heuristics (analysis always runs; this emphasizes in output)")]
    pub detect_suspicious: bool,
}
