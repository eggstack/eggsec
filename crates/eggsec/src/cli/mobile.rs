pub(crate) const MOBILE_ABOUT: &str = "Static security analysis of Android APKs and iOS IPAs (lab/defense use only)

Performs static analysis on mobile application packages (.apk for Android, .ipa for iOS).
No dynamic execution, network access, or device interaction.

Detects common issues including:
- Manifest/configuration problems (debuggable, allowBackup, exported components without protection)
- Over-privileged or dangerous permissions
- Insecure transport settings (cleartext HTTP, weak TLS config)
- Hardcoded secrets, API keys, or credentials in resources/strings
- Insecure data storage patterns and backup risks
- Debug/release signing and certificate issues
- WebView and JavaScript bridge risks
- Basic supply-chain / third-party SDK indicators

Results include severity-rated findings, location/context, and remediation guidance.

Supports --json for structured output, -o/--output for file export, -q/--quiet to reduce console noise.

Intended for authorized lab, defense, or internal security validation use only on applications you are permitted to analyze.

Examples:
  eggsec mobile app.apk
  eggsec mobile app.ipa --json
  eggsec mobile /path/to/release.apk -o mobile-report.json
  eggsec mobile app.ipa --quiet
  eggsec mobile app.apk --json --output findings.json
";

#[derive(clap::Args)]
pub struct MobileArgs {
    #[arg(help = "Path to .apk (Android) or .ipa (iOS) file for static analysis")]
    pub path: String,
    #[arg(long, help = "Output results as JSON")]
    pub json: bool,
    #[arg(long, short = 'o', help = "Output to file")]
    pub output: Option<String>,
    #[arg(long, short = 'q', help = "Suppress non-essential output")]
    pub quiet: bool,
}
