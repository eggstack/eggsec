pub(crate) const MOBILE_ABOUT: &str = r#"Static security analysis of Android APKs and iOS IPAs + gated dynamic runtime testing (lab/defense use only).

SUBCOMMANDS:
  static            Pure-Rust static analysis of .apk/.ipa (default / legacy direct path form also supported).
  dynamic           Android dynamic/runtime testing via ADB (install/launch/logcat/uninstall). Requires --features mobile-dynamic.

Use 'eggsec mobile static --help' or 'eggsec mobile dynamic --help' for details.

Static: offline manifest/config checks only. No execution or device interaction.
Dynamic (mobile-dynamic feature, Phase 1 + Phase 2 + Phase 3b Frida): controlled ADB + logcat + proxy + traffic-capture + runtime-permission operations + optional Frida instrumentation (basic_method_trace + crypto-keystore + bypass-validation + api-trace via --frida-script with builtin: prefix) on lab devices/emulators you own/authorize. All actions audited. Dry-run supported. Real Frida requires --allow-frida (Intrusive policy tier) + frida CLI + frida-server on device.

Build with --features mobile (static) or --features mobile-dynamic (dynamic + static + Frida Phase 3b).

Examples:
  eggsec mobile app.apk                          # legacy direct static
  eggsec mobile static app.apk --json
  eggsec mobile static /path/to/app.ipa -o out.json
  eggsec mobile dynamic test.apk --device emulator-5554 --dry-run --json
  eggsec mobile dynamic /tmp/vuln.apk --device emulator-5554 --install --launch .Main --capture-logs --duration 30 --uninstall-after --allow-dynamic-mobile
  eggsec mobile dynamic --list-devices
  eggsec mobile dynamic --list-devices --device emulator-5554
  # Phase 3b/3c Frida (dry-run always safe; real requires --allow-frida)
  eggsec mobile dynamic test.apk --device emulator-5554 --dry-run --frida-script /tmp/trace.js --json
  # Phase 3b builtins + Phase 3c library components (repeatable for multi-script sessions)
  eggsec mobile dynamic test.apk --device emulator-5554 --dry-run --frida-script "builtin:crypto-keystore" --frida-script "library:common-hooks" --json
  eggsec mobile dynamic /tmp/vuln.apk --device emulator-5554 --allow-dynamic-mobile --allow-frida --frida-script trace.js --package com.example.vuln
  # Phase 3c regression + bundle
  eggsec mobile dynamic test.apk --device emulator-5554 --dry-run --frida-script "library:common-hooks" --baseline /tmp/baseline.json --evidence-bundle /tmp/evidence.json.gz --json
"#;

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
    #[command(
        name = "dynamic",
        about = "Controlled dynamic run on lab Android device/emulator (install/launch/log/uninstall)"
    )]
    Dynamic(Box<DynamicMobileArgs>),
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

#[allow(dead_code)]
pub(crate) const MOBILE_DYNAMIC_ABOUT: &str = r#"MODE: Defense Lab | Lab-only; authorized use only.

Dynamic Android runtime testing (ADB + logcat analysis + Phase 3b Frida instrumentation) for defense validation and regression.

Controlled install/launch/observe/uninstall cycle on lab device/emulator.
Captures runtime logs, extracts high-signal findings (permissions, crashes, cleartext, secrets in logs).
Optional Frida: --frida-script (user JS or "builtin:NAME" for crypto-keystore / bypass-validation / api-trace / basic-method-trace).

Use 'eggsec mobile dynamic --help' for details.

Requires building with --features mobile-dynamic (implies mobile).
Pure-Rust ADB TCP for emulators + optional external 'adb' for discovery.

All actions are audited in the report (actions_performed). Dry-run always supported and produces complete valid output.
Real runs require explicit --allow-dynamic-mobile (audited). Real Frida requires --allow-frida (Intrusive policy tier) + frida CLI + frida-server on device.

WARNING: Installs and runs user-supplied test builds on devices/emulators YOU control and are authorized to test.
Never use on production devices or apps with real user data. Best-effort uninstall is attempted; manual cleanup may be required on error.
Frida ops are intrusive (runtime hooking); use --dry-run for safe simulation.

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
  # Phase 3b/3c Frida (dry-run safe)
  eggsec mobile dynamic test.apk --device emulator-5554 --dry-run --frida-script /tmp/trace.js --json
  eggsec mobile dynamic test.apk --device emulator-5554 --dry-run --frida-script "builtin:crypto-keystore" --frida-script "library:common-hooks" --json
  eggsec mobile dynamic test.apk --device emulator-5554 --allow-dynamic-mobile --allow-frida --frida-script trace.js --package com.example.vuln
  # Phase 3c regression + bundle
  eggsec mobile dynamic test.apk --device emulator-5554 --dry-run --frida-script "library:common-hooks" --baseline /tmp/baseline.json --evidence-bundle /tmp/evidence.json.gz --json
"#;

/// Dynamic mobile args (Phase 1 ADB core + log capture; Phase 2 (proxy/permissions/correlation) +
/// Frida + CorrelationEngine under single mobile-dynamic).
#[derive(clap::Args, Clone)]
pub struct DynamicMobileArgs {
    #[arg(help = "Path to .apk (Android test build only) for dynamic run")]
    pub target: String,

    #[arg(
        long,
        help = "Device serial (e.g. emulator-5554) or host:port (e.g. 127.0.0.1:5555)"
    )]
    pub device: Option<String>,

    #[arg(long, help = "Install the APK via adb (pm install -r)")]
    pub install: bool,

    #[arg(
        long,
        help = "Launch via am start (e.g. --launch '.MainActivity' or 'com.pkg/.MainActivity')"
    )]
    pub launch: Option<String>,

    #[arg(
        long,
        help = "Capture logcat output during/after launch for runtime analysis"
    )]
    pub capture_logs: bool,

    #[arg(
        long,
        default_value_t = 60,
        help = "Duration in seconds for log capture (bounded)"
    )]
    pub duration: u64,

    #[arg(long, help = "Uninstall the package after run (best-effort cleanup)")]
    pub uninstall_after: bool,

    #[arg(
        long,
        help = "Plan/dry-run mode: simulate all actions, produce valid structured report, touch nothing"
    )]
    pub dry_run: bool,

    #[arg(long, help = "Output results as JSON")]
    pub json: bool,

    #[arg(long, short = 'o', help = "Output to file")]
    pub output: Option<String>,

    #[arg(
        long,
        short = 'q',
        help = "Suppress non-essential output and the lab warning note"
    )]
    pub quiet: bool,

    #[arg(
        long,
        help = "Explicit confirmation for dynamic mobile execution (required for any non-dry-run real device actions; recorded for audit)"
    )]
    pub allow_dynamic_mobile: bool,

    #[arg(
        long,
        value_name = "FILE",
        help = "Path to optional lab manifest TOML (allowed_device_serials + allowed_packages; advisory in Phase 1)"
    )]
    pub lab_manifest: Option<String>,

    #[arg(
        long,
        help = "List reachable devices/emulators via pure-Rust probe (+ external adb convenience if in PATH) and exit. Target APK may be omitted or a placeholder."
    )]
    pub list_devices: bool,

    // mobile-dynamic extensions: proxy + traffic-capture + runtime-permission operations
    #[arg(
        long,
        value_name = "HOST:PORT",
        help = "Configure device global HTTP proxy for the run (e.g. 127.0.0.1:8080). Requires user-managed MITM CA on device for HTTPS inspection. Device setting only (no auto mitmproxy start)."
    )]
    pub proxy: Option<String>,
    #[arg(
        long,
        help = "After run, reset/clear the global HTTP proxy on device (best-effort)."
    )]
    pub reset_proxy: bool,
    #[arg(long = "grant-permission", value_name = "PERM", action = clap::ArgAction::Append, help = "Grant runtime permission(s) to package (pm grant). Repeatable. e.g. android.permission.CAMERA")]
    pub grant_permissions: Vec<String>,
    #[arg(long = "revoke-permission", value_name = "PERM", action = clap::ArgAction::Append, help = "Revoke runtime permission(s) (pm revoke). Repeatable.")]
    pub revoke_permissions: Vec<String>,
    #[arg(
        long,
        help = "Snapshot current permission state for the target package (via dumpsys) and include in report."
    )]
    pub list_permissions: bool,
    #[arg(
        long,
        value_name = "FILE",
        help = "Path to traffic capture (mitmproxy text log or minimal HAR JSON) to parse for traffic_summary + findings. Complements --proxy."
    )]
    pub traffic_capture: Option<String>,

    // Phase 3b/3c Frida (under mobile-dynamic; runtime gated by --allow-frida + policy)
    // Repeatable: --frida-script a.js --frida-script "builtin:crypto-keystore" --frida-script "library:common-hooks"
    // Supports user .js files, "builtin:NAME", "library:NAME" (Phase 3c reusable components).
    #[arg(long, value_name = "SPEC", action = clap::ArgAction::Append, help = "Frida script spec (repeatable for multi-script Phase 3c). File path, or \"builtin:NAME\" (crypto-keystore|bypass-validation|api-trace|basic-method-trace), or \"library:NAME\" (common-hooks etc.). Real requires --allow-frida (Intrusive). Dry-run safe.")]
    pub frida_script: Vec<String>,
    #[arg(
        long,
        help = "Explicit confirmation for Frida instrumentation (required for any non-dry-run Frida operations; recorded for audit). Real Frida also needs frida CLI + frida-server on device."
    )]
    pub allow_frida: bool,

    // Phase 3c behavioral regression + evidence bundle
    #[arg(
        long,
        value_name = "FILE",
        help = "Path to prior baseline JSON (MobileBaseline) for regression diff vs current run (Phase 3c). Dry-run safe."
    )]
    pub baseline: Option<String>,
    #[arg(
        long,
        value_name = "FILE",
        help = "Path to write gzipped evidence bundle (report + traffic + frida + actions) after run (Phase 3c, optional). Uses flate2."
    )]
    pub evidence_bundle: Option<String>,
}
