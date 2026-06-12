pub(crate) const MOBILE_ABOUT: &str = "Static security analysis of Android APKs and iOS IPAs + gated dynamic runtime testing (lab/defense use only).

SUBCOMMANDS:
  static            Pure-Rust static analysis of .apk/.ipa (default / legacy direct path form also supported).
  dynamic           Android dynamic/runtime testing via ADB (install/launch/logcat/uninstall). Requires --features mobile-dynamic.

Use 'eggsec mobile static --help' or 'eggsec mobile dynamic --help' for details.

Static: offline manifest/config checks only. No execution or device interaction.
Dynamic (Phase 1): controlled ADB + logcat analysis on lab devices/emulators you own/authorize. All actions audited. Dry-run supported.

Build with --features mobile (static) or --features mobile-dynamic (dynamic + static).

Examples:
  eggsec mobile app.apk                          # legacy direct static
  eggsec mobile static app.apk --json
  eggsec mobile static /path/to/app.ipa -o out.json
  eggsec mobile dynamic test.apk --device emulator-5554 --dry-run --json
  eggsec mobile dynamic /tmp/vuln.apk --device emulator-5554 --install --launch .Main --capture-logs --duration 30 --uninstall-after --allow-dynamic-mobile
  eggsec mobile dynamic --list-devices
  eggsec mobile dynamic --list-devices --device emulator-5554
 ";

/// Top-level args for `eggsec mobile ...`.
/// Supports legacy direct `eggsec mobile <path.{apk,ipa}>` (treated as static) and subcommands.
#[derive(clap::Args, Clone)]
pub struct MobileArgs {
    /// Legacy direct path form for static analysis (e.g. `eggsec mobile app.apk`).
    /// When a subcommand is present this is ignored.
    #[arg(help = "Path to .apk/.ipa for legacy direct static analysis (implies 'static')")]
    pub path: Option<String>,

    #[command(subcommand)]
    pub command: Option<MobileSubcommand>,

    // Common output flags (apply to legacy direct and passed down to subs where applicable)
    #[arg(long, help = "Output results as JSON")]
    pub json: bool,
    #[arg(long, short = 'o', help = "Output to file")]
    pub output: Option<String>,
    #[arg(long, short = 'q', help = "Suppress non-essential output")]
    pub quiet: bool,
}

#[derive(clap::Subcommand, Clone)]
pub enum MobileSubcommand {
    /// Static security analysis of Android APKs and iOS IPAs (pure-Rust, lab binaries only)
    #[command(name = "static")]
    Static(MobileStaticArgs),

    /// Dynamic Android runtime testing (ADB + logcat analysis). Requires feature mobile-dynamic.
    #[cfg(feature = "mobile-dynamic")]
    #[command(name = "dynamic", about = "Controlled dynamic run on lab Android device/emulator (install/launch/log/uninstall)")]
    Dynamic(DynamicMobileArgs),
}

/// Static analysis args (used by 'mobile static <path>' and legacy direct path).
#[derive(clap::Args, Clone)]
pub struct MobileStaticArgs {
    #[arg(help = "Path to .apk (Android) or .ipa (iOS) file for static analysis")]
    pub path: String,
    #[arg(long, help = "Output results as JSON")]
    pub json: bool,
    #[arg(long, short = 'o', help = "Output to file")]
    pub output: Option<String>,
    #[arg(long, short = 'q', help = "Suppress non-essential output")]
    pub quiet: bool,
}

pub(crate) const MOBILE_DYNAMIC_ABOUT: &str = "MODE: Defense Lab | Lab-only; authorized use only.

Dynamic Android runtime testing (ADB + logcat analysis) for defense validation and regression.

Controlled install/launch/observe/uninstall cycle on lab device/emulator.
Captures runtime logs, extracts high-signal findings (permissions, crashes, cleartext, secrets in logs).

Use 'eggsec mobile dynamic --help' for details.

Requires building with --features mobile-dynamic (implies mobile).
Pure-Rust ADB TCP for emulators + optional external 'adb' for discovery.

All actions are audited in the report (actions_performed). Dry-run always supported and produces complete valid output.
Real runs require explicit --allow-dynamic-mobile (audited) + provenance-controlled test APK + preferably a --lab-manifest.

WARNING: Installs and runs user-supplied test builds on devices/emulators YOU control and are authorized to test.
Never use on production devices or apps with real user data. Best-effort uninstall is attempted; manual cleanup may be required on error.

Examples:
  eggsec mobile dynamic app.apk --device emulator-5554 --dry-run --json
  eggsec mobile dynamic /path/to/test.apk \
    --device emulator-5554 \
    --install --launch .MainActivity \
    --capture-logs --duration 90 \
    --uninstall-after \
    --allow-dynamic-mobile \
    --lab-manifest examples/lab-mobile.toml
  eggsec mobile dynamic --list-devices
 ";

/// Dynamic mobile args (Phase 1: Android ADB core + log capture).
#[derive(clap::Args, Clone)]
pub struct DynamicMobileArgs {
    #[arg(help = "Path to .apk (Android test build only) for dynamic run")]
    pub target: String,

    #[arg(long, help = "Device serial (e.g. emulator-5554) or host:port (e.g. 127.0.0.1:5555)")]
    pub device: Option<String>,

    #[arg(long, help = "Install the APK via adb (pm install -r)")]
    pub install: bool,

    #[arg(long, help = "Launch via am start (e.g. --launch '.MainActivity' or 'com.pkg/.MainActivity')")]
    pub launch: Option<String>,

    #[arg(long, help = "Capture logcat output during/after launch for runtime analysis")]
    pub capture_logs: bool,

    #[arg(long, default_value_t = 60, help = "Duration in seconds for log capture (bounded)")]
    pub duration: u64,

    #[arg(long, help = "Uninstall the package after run (best-effort cleanup)")]
    pub uninstall_after: bool,

    #[arg(long, help = "Plan/dry-run mode: simulate all actions, produce valid structured report, touch nothing")]
    pub dry_run: bool,

    #[arg(long, help = "Output results as JSON")]
    pub json: bool,

    #[arg(long, short = 'o', help = "Output to file")]
    pub output: Option<String>,

    #[arg(long, short = 'q', help = "Suppress non-essential output and the lab warning note")]
    pub quiet: bool,

    #[arg(long, help = "Explicit confirmation for dynamic mobile execution (required for any non-dry-run real device actions; recorded for audit)")]
    pub allow_dynamic_mobile: bool,

    #[arg(long, value_name = "FILE", help = "Path to optional lab manifest TOML (allowed_device_serials + allowed_packages; advisory in Phase 1)")]
    pub lab_manifest: Option<String>,

    #[arg(long, help = "List reachable devices/emulators via pure-Rust probe (+ external adb convenience if in PATH) and exit. Target APK may be omitted or a placeholder.")]
    pub list_devices: bool,
}
