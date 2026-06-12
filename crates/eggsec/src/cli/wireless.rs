pub(crate) const WIRELESS_ABOUT: &str = "MODE: Defense Lab | Lab-only; authorized use only.

WiFi reconnaissance and active attack primitives for defense validation.

SUBCOMMANDS:
  scan              Passive wireless network scanning and security analysis (default)
  deauth            Send deauthentication/disassociation frames (active, high-risk)
  capture-handshake Capture WPA/WPA2 handshake (active, high-risk, Phase 2 - not yet implemented)

Use 'eggsec wireless <iface> <subcommand> --help' for details on each subcommand.

Requires building with --features wireless (passive) or --features wireless-advanced (active).
Real scans require root (or CAP_NET_ADMIN) + 'iwlist' from wireless-tools.
Active attacks require monitor-mode interface + root/CAP_NET_ADMIN + explicit authorization.

Use only on authorized networks in lab/defense-validation contexts.

Examples:
  sudo eggsec wireless wlan0                          # Passive scan (default)
  sudo eggsec wireless wlan0 --json                   # JSON output
  sudo eggsec wireless wlan0 --repeat 3 --duration 10 # Repeated scans
  sudo eggsec wireless wlan0 --dry-run --json         # Plan mode
  sudo eggsec wireless wlan0 deauth --bssid AA:BB:CC:DD:EE:FF --dry-run  # Dry run deauth
  sudo eggsec wireless wlan0 deauth --bssid AA:BB:CC:DD:EE:FF --client 11:22:33:44:55:66 --count 20";

/// Passive scan arguments (shared with the original WirelessArgs)
#[derive(clap::Args, Clone)]
pub struct WirelessScanArgs {
    #[arg(help = "Output results as JSON")]
    pub json: bool,
    #[arg(long, short = 'o', help = "Output to file")]
    pub output: Option<String>,
    #[arg(long, short = 'q', help = "Suppress non-essential output")]
    pub quiet: bool,
    #[arg(long, default_value_t = 10, help = "Scan duration in seconds")]
    pub duration: u64,
    #[arg(long, default_value_t = 1, help = "Number of scans to perform (repeat for change/rogue observation)")]
    pub repeat: u32,
    #[arg(long, help = "Enable verbose suspicious/rogue network heuristics")]
    pub detect_suspicious: bool,
    #[arg(long, help = "Plan mode: show what would be scanned without performing iwlist calls")]
    pub dry_run: bool,
    #[arg(long, value_name = "FILE", help = "Path to simple allowlist (one entry per line: SSID or BSSID or SSID,BSSID)")]
    pub known_good: Option<String>,
}

impl Default for WirelessScanArgs {
    fn default() -> Self {
        Self {
            json: false,
            output: None,
            quiet: false,
            duration: 10,
            repeat: 1,
            detect_suspicious: false,
            dry_run: false,
            known_good: None,
        }
    }
}

/// Active deauthentication/disassociation attack arguments
#[derive(clap::Args, Clone)]
pub struct DeauthArgs {
    /// Target AP BSSID in AA:BB:CC:DD:EE:FF format
    #[arg(long, help = "Target AP BSSID (e.g., AA:BB:CC:DD:EE:FF)")]
    pub bssid: String,

    /// Target client MAC in AA:BB:CC:DD:EE:FF format (omit for broadcast deauth)
    #[arg(long, help = "Target client MAC address (omit for broadcast deauth to all clients)")]
    pub client: Option<String>,

    /// Number of frames to send
    #[arg(long, default_value_t = 50, help = "Number of deauth frames to send")]
    pub count: u64,

    /// 802.11 reason code
    #[arg(long, default_value_t = 7, help = "802.11 reason code (default: 7 = class 3 from unassoc)")]
    pub reason_code: u16,

    /// Send deauth to broadcast address (all clients)
    #[arg(long, help = "Send deauth to broadcast address (all clients on AP)")]
    pub broadcast: bool,

    /// Monitor-mode interface override
    #[arg(long, help = "Specify monitor-mode interface (default: use main interface)")]
    pub monitor_iface: Option<String>,

    /// Maximum frames to send (budget)
    #[arg(long, default_value_t = 100, help = "Maximum frames to send (hard budget)")]
    pub max_frames: u64,

    /// Frames per second rate limit
    #[arg(long, default_value_t = 10, help = "Frame injection rate (frames per second)")]
    pub fps: u64,

    /// Output results as JSON
    #[arg(long, help = "Output results as JSON")]
    pub json: bool,

    /// Output to file
    #[arg(long, short = 'o', help = "Output to file")]
    pub output: Option<String>,

    /// Dry run mode (no frames sent, valid JSON output)
    #[arg(long, help = "Plan mode: show what would be sent without transmitting")]
    pub dry_run: bool,

    /// Confirm active wireless attack (required for non-dry-run)
    #[arg(long, help = "Confirm active wireless attack execution (required for non-dry-run)")]
    pub allow_active_wireless: bool,

    /// Reason for manual override (recorded for audit)
    #[arg(long, help = "Reason for manual override (recorded for audit)")]
    pub manual_override_reason: Option<String>,
}

#[derive(clap::Subcommand, Clone)]
pub enum WirelessSubcommand {
    /// Passive wireless network scanning and security analysis (default)
    #[command(name = "scan")]
    Scan(WirelessScanArgs),

    /// Send deauthentication/disassociation frames (active, high-risk, lab-only)
    #[command(name = "deauth")]
    Deauth(DeauthArgs),
}

#[derive(clap::Args)]
pub struct WirelessArgs {
    #[arg(help = "Wireless interface name (e.g., wlan0)")]
    pub interface: String,

    #[command(subcommand)]
    pub command: Option<WirelessSubcommand>,
}
