pub(crate) const EVASION_ABOUT: &str = r#"MODE: Defense Lab | Lab-only; authorized use only.

Detect common evasion techniques used by malware and advanced threats.
Validates whether security controls can detect common defense evasion patterns.
Maps detections to MITRE ATT&CK IDs with confidence scores.

WARNING: This is a standalone defense-lab capability. It performs passive analysis
of targets (file inspection, process enumeration, network patterns) to detect
evasion technique indicators. It is not for offensive use.

Examples:
  eggsec evasion --target /path/to/binary --dry-run --json
  eggsec evasion --target /path/to/binary --type file
  eggsec evasion --target /path/to/binary --type process --pid 1234
  eggsec evasion --target /path/to/binary --dry-run -o evasion-report.json
  eggsec evasion --type network --dry-run --json

Requires building with --features evasion.
All dry-run operations produce complete reports with synthetic data (no side effects).
Real mode requires explicit --allow-evasion-testing flag.
"#;

#[derive(clap::Args, Clone)]
pub struct EvasionArgs {
    /// Target path (binary file, process path, etc.)
    #[arg(long, value_name = "PATH")]
    pub target: Option<String>,

    /// Target type (process, file, network, registry, memory)
    #[arg(long = "type", value_name = "TYPE")]
    pub target_type: Option<String>,

    /// Process ID (for process target type)
    #[arg(long, value_name = "PID")]
    pub pid: Option<u32>,

    /// Plan/dry-run mode: produce complete report with synthetic data, no real checks
    #[arg(long)]
    pub dry_run: bool,

    /// Output results as JSON
    #[arg(long)]
    pub json: bool,

    /// Write output to file instead of stdout
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<String>,

    /// Suppress non-essential output
    #[arg(long, short = 'q')]
    pub quiet: bool,
}

impl EvasionArgs {
    pub fn parsed_target_type(&self) -> crate::evasion::EvasionTargetType {
        match self.target_type.as_deref() {
            Some("file") | Some("f") => crate::evasion::EvasionTargetType::File,
            Some("network") | Some("n") => crate::evasion::EvasionTargetType::Network,
            Some("registry") | Some("r") => crate::evasion::EvasionTargetType::Registry,
            Some("memory") | Some("m") => crate::evasion::EvasionTargetType::Memory,
            _ => crate::evasion::EvasionTargetType::Process,
        }
    }
}
