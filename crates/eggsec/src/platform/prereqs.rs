//! Central prerequisite matrix for platform-sensitive domains.
//!
//! Read-only detection only. No test in this module requires root, hardware,
//! or network access. Privileged checks report status; they never attempt to
//! acquire privilege.

use serde::{Deserialize, Serialize};

/// Status of a single prerequisite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PrereqStatus {
    /// Present and usable.
    Available,
    /// Absent on this host (binary missing, interface down, etc.).
    Missing,
    /// Not supported on this OS/arch (e.g. `iwlist` outside Linux).
    Unsupported,
    /// Present in principle but needs privilege the current user lacks.
    PrivilegeRequired,
}

impl PrereqStatus {
    /// True when the prerequisite blocks real-hardware/live execution.
    pub fn is_blocking(&self) -> bool {
        !matches!(self, Self::Available)
    }
}

/// One row of the capability matrix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prerequisite {
    /// Stable id, e.g. `os-linux`, `bin-adb`, `priv-root`.
    pub id: String,
    /// Human label, e.g. `Linux OS`.
    pub label: String,
    /// Detection outcome.
    pub status: PrereqStatus,
    /// Short factual detail (OS name, binary path, etc.).
    pub detail: String,
    /// How to satisfy it, or empty when nothing actionable applies.
    pub fix: String,
}

impl Prerequisite {
    fn available(id: &str, label: &str, detail: impl Into<String>) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            status: PrereqStatus::Available,
            detail: detail.into(),
            fix: String::new(),
        }
    }

    fn missing(id: &str, label: &str, detail: impl Into<String>, fix: impl Into<String>) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            status: PrereqStatus::Missing,
            detail: detail.into(),
            fix: fix.into(),
        }
    }

    fn unsupported(
        id: &str,
        label: &str,
        detail: impl Into<String>,
        fix: impl Into<String>,
    ) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            status: PrereqStatus::Unsupported,
            detail: detail.into(),
            fix: fix.into(),
        }
    }

    fn privilege_required(
        id: &str,
        label: &str,
        detail: impl Into<String>,
        fix: impl Into<String>,
    ) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            status: PrereqStatus::PrivilegeRequired,
            detail: detail.into(),
            fix: fix.into(),
        }
    }
}

/// Per-domain prerequisite set plus fixture-vs-hardware guidance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainPrerequisites {
    /// Stable domain id: `mobile-dynamic`, `packet-inspection`, `wireless`,
    /// `wireless-advanced`, `stress-testing`, `nse`.
    pub domain: String,
    /// Cargo feature gate(s).
    pub features: Vec<String>,
    /// Rows of the matrix for this domain.
    pub prerequisites: Vec<Prerequisite>,
    /// True when deterministic fixture tests run without hardware/privilege.
    pub fixture_supported: bool,
    /// True when real-hardware/live execution is possible on this host.
    pub live_supported: bool,
    /// Canonical fixture entry point (script or `cargo test` selector).
    pub fixture_command: String,
    /// Human note about what stays manual/hardware-gated.
    pub hardware_note: String,
}

impl DomainPrerequisites {
    /// First blocking prerequisite, if any.
    pub fn first_blocking(&self) -> Option<&Prerequisite> {
        self.prerequisites.iter().find(|p| p.status.is_blocking())
    }
}

/// Full host report: OS facts plus every domain matrix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformReport {
    pub os: String,
    pub arch: String,
    pub kernel: String,
    pub is_root: bool,
    pub cap_net_raw: bool,
    pub cap_net_admin: bool,
    pub compiled_features: Vec<String>,
    pub domains: Vec<DomainPrerequisites>,
}

impl PlatformReport {
    /// Collect the current host report (read-only, never requires privilege).
    pub fn collect() -> Self {
        let domains = all_domain_ids()
            .iter()
            .map(|d| report_for(d))
            .collect::<Vec<_>>();
        Self {
            os: current_os(),
            arch: current_arch(),
            kernel: current_kernel(),
            is_root: is_root(),
            cap_net_raw: has_cap_net_raw(),
            cap_net_admin: has_cap_net_admin(),
            compiled_features: compiled_features(),
            domains,
        }
    }

    /// Human-readable multi-line rendering for `eggsec doctor`.
    pub fn format_human(&self) -> String {
        let mut out = String::new();
        out.push_str("=== Eggsec Platform Report ===\n");
        out.push_str(&format!(
            "host: {} / {} (kernel {})\n",
            self.os, self.arch, self.kernel
        ));
        out.push_str(&format!(
            "privileges: root={} cap_net_raw={} cap_net_admin={}\n",
            yes_no(self.is_root),
            yes_no(self.cap_net_raw),
            yes_no(self.cap_net_admin)
        ));
        out.push_str(&format!(
            "compiled features: {}\n",
            if self.compiled_features.is_empty() {
                "(none of the platform domains compiled)".to_string()
            } else {
                self.compiled_features.join(", ")
            }
        ));
        for d in &self.domains {
            out.push_str(&format!(
                "\n[{}] fixture={} live={} (features: {})\n",
                d.domain,
                yes_no(d.fixture_supported),
                yes_no(d.live_supported),
                d.features.join(", ")
            ));
            for p in &d.prerequisites {
                let mark = match p.status {
                    PrereqStatus::Available => "ok  ",
                    PrereqStatus::Missing => "miss",
                    PrereqStatus::Unsupported => "unsup",
                    PrereqStatus::PrivilegeRequired => "priv ",
                };
                out.push_str(&format!("  {} {}: {}\n", mark, p.label, p.detail));
                if !p.fix.is_empty() && p.status.is_blocking() {
                    out.push_str(&format!("        fix: {}\n", p.fix));
                }
            }
            out.push_str(&format!("  fixture: {}\n", d.fixture_command));
            if !d.hardware_note.is_empty() {
                out.push_str(&format!("  hardware: {}\n", d.hardware_note));
            }
        }
        out.push_str("\nFixture tests never require root or hardware. Live tests are\n");
        out.push_str("isolated (netns/emulator/lab) and skip with the prerequisite above\n");
        out.push_str("when it is absent. Traffic stays in loopback or the fixture subnet.\n");
        out
    }
}

fn yes_no(v: bool) -> &'static str {
    if v {
        "yes"
    } else {
        "no"
    }
}

// ---------------------------------------------------------------------------
// Low-level detectors (read-only, best-effort).
// ---------------------------------------------------------------------------

/// Current OS name (`std::env::consts::OS`).
pub fn current_os() -> String {
    std::env::consts::OS.to_string()
}

/// Current CPU architecture (`std::env::consts::ARCH`).
pub fn current_arch() -> String {
    std::env::consts::ARCH.to_string()
}

/// Best-effort kernel/release string. Reads `/proc/sys/kernel/osrelease` on
/// Linux, falls back to `unknown` elsewhere.
pub fn current_kernel() -> String {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/sys/kernel/osrelease")
            .map(|s| s.trim().to_string())
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "unknown".to_string())
    }
    #[cfg(not(target_os = "linux"))]
    {
        "unknown".to_string()
    }
}

/// True when the effective UID is 0 (Unix only; always false elsewhere).
pub fn is_root() -> bool {
    #[cfg(unix)]
    {
        libc_geteuid() == 0
    }
    #[cfg(not(unix))]
    {
        false
    }
}

#[cfg(unix)]
fn libc_geteuid() -> u32 {
    // `libc` is only an optional engine dependency (packet/stress features),
    // so call the C function directly to keep this module dependency-light.
    unsafe extern "C" {
        fn geteuid() -> u32;
    }
    // SAFETY: trivial syscall wrapper with no preconditions.
    unsafe { geteuid() }
}

/// Best-effort `CAP_NET_RAW` probe: true only when root on Linux. Unprivileged
/// users cannot hold the capability, so reporting `is_root` as the proxy
/// avoids attempting any privileged operation.
pub fn has_cap_net_raw() -> bool {
    current_os() == "linux" && is_root()
}

/// Best-effort `CAP_NET_ADMIN` probe: same conservative proxy as RAW.
pub fn has_cap_net_admin() -> bool {
    current_os() == "linux" && is_root()
}

/// True when `name` resolves via `PATH` search (no shell-out to the binary).
pub fn has_binary(name: &str) -> bool {
    if name.is_empty() || name.contains('/') || name.contains('\\') {
        return false;
    }
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|dir| {
            let candidate = dir.join(name);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::metadata(&candidate)
                    .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
            }
            #[cfg(not(unix))]
            {
                let with_exe = dir.join(format!("{name}.exe"));
                candidate.is_file() || with_exe.is_file()
            }
        })
    })
}

/// Names under `/sys/class/net` (Linux) or a best-effort fallback elsewhere.
pub fn list_interfaces() -> Vec<String> {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_dir("/sys/class/net")
            .map(|entries| {
                let mut names: Vec<String> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect();
                names.sort();
                names
            })
            .unwrap_or_default()
    }
    #[cfg(not(target_os = "linux"))]
    {
        vec!["lo".to_string()]
    }
}

/// One-line capability summary for logs and skip messages.
pub fn capabilities_summary() -> String {
    format!(
        "os={} arch={} root={} net_raw={} net_admin={}",
        current_os(),
        current_arch(),
        yes_no(is_root()),
        yes_no(has_cap_net_raw()),
        yes_no(has_cap_net_admin())
    )
}

fn compiled_features() -> Vec<String> {
    let mut out = Vec::new();
    for (id, present) in [
        ("mobile", cfg!(feature = "mobile")),
        ("mobile-dynamic", cfg!(feature = "mobile-dynamic")),
        ("packet-inspection", cfg!(feature = "packet-inspection")),
        ("wireless", cfg!(feature = "wireless")),
        ("wireless-advanced", cfg!(feature = "wireless-advanced")),
        ("stress-testing", cfg!(feature = "stress-testing")),
        ("nse", cfg!(feature = "nse")),
    ] {
        if present {
            out.push(id.to_string());
        }
    }
    out
}

/// Stable domain ids covered by the matrix.
pub fn all_domain_ids() -> Vec<String> {
    [
        "mobile-dynamic",
        "packet-inspection",
        "wireless",
        "wireless-advanced",
        "stress-testing",
        "nse",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// Build the matrix for one domain id. Unknown ids yield an empty matrix with
/// `live_supported=false` (fail-closed for callers that gate on this).
pub fn report_for(domain: &str) -> DomainPrerequisites {
    match domain {
        "mobile-dynamic" => mobile_dynamic_matrix(),
        "packet-inspection" => packet_matrix(),
        "wireless" => wireless_matrix(),
        "wireless-advanced" => wireless_advanced_matrix(),
        "stress-testing" => stress_matrix(),
        "nse" => nse_matrix(),
        _ => DomainPrerequisites {
            domain: domain.to_string(),
            features: Vec::new(),
            prerequisites: Vec::new(),
            fixture_supported: false,
            live_supported: false,
            fixture_command: String::new(),
            hardware_note: format!("unknown domain '{domain}'"),
        },
    }
}

/// Fail-closed skip reason for a domain: `None` when live execution looks
/// possible, `Some(reason)` naming the first blocking prerequisite otherwise.
/// Fixture (hardware-free) tests must not call this; they always run.
pub fn skip_reason_for(domain: &str) -> Option<String> {
    let matrix = report_for(domain);
    matrix.first_blocking().map(|p| {
        let mut reason = format!(
            "SKIP {domain}: {} ({}) [{}]",
            p.label,
            p.detail,
            capabilities_summary()
        );
        if !p.fix.is_empty() {
            reason.push_str(&format!(" fix: {}", p.fix));
        }
        reason
    })
}

fn os_prereq(linux_only: bool) -> Prerequisite {
    let os = current_os();
    if linux_only && os != "linux" {
        Prerequisite::unsupported(
            "os-linux",
            "Linux OS",
            format!("current os={os}"),
            "run on Linux for live capture/raw-socket/iwlist paths; fixture tests run anywhere",
        )
    } else {
        Prerequisite::available("os", "Operating system", format!("os={os}"))
    }
}

fn privilege_prereq(id: &str, label: &str, has: bool, detail: &str, fix: &str) -> Prerequisite {
    if has {
        Prerequisite::available(id, label, detail)
    } else {
        Prerequisite::privilege_required(id, label, detail, fix)
    }
}

fn binary_prereq(id: &str, label: &str, binary: &str, fix: &str) -> Prerequisite {
    if has_binary(binary) {
        Prerequisite::available(id, label, format!("{binary} found in PATH"))
    } else {
        Prerequisite::missing(id, label, format!("{binary} not in PATH"), fix)
    }
}

fn mobile_dynamic_matrix() -> DomainPrerequisites {
    let os = os_prereq(false);
    let adb = binary_prereq(
        "bin-adb",
        "ADB binary",
        "adb",
        "install Android SDK platform-tools; pure-Rust emulator probe works without it",
    );
    let frida_cli = if has_binary("frida") {
        Prerequisite::available("bin-frida", "Frida CLI", "frida found in PATH")
    } else {
        // Optional external: dry-run and builtin-script planning work without it.
        Prerequisite::missing(
            "bin-frida",
            "Frida CLI (optional)",
            "frida not in PATH; instrumentation planning/dry-run still works",
            "install frida-tools for live instrumentation; dry-run needs nothing",
        )
    };
    let emulator_hint = if has_binary("emulator") {
        Prerequisite::available("bin-emulator", "Android emulator", "emulator found in PATH")
    } else {
        Prerequisite::missing(
            "bin-emulator",
            "Android emulator/device",
            "no emulator binary and no ADB device probe attempted here",
            "start an AVD (see scripts/setup_android_emulator.sh) or connect a lab device",
        )
    };
    let compiled = cfg!(feature = "mobile-dynamic");
    let feature = if compiled {
        Prerequisite::available(
            "feature-mobile-dynamic",
            "Feature mobile-dynamic",
            "compiled in this build",
        )
    } else {
        Prerequisite::missing(
            "feature-mobile-dynamic",
            "Feature mobile-dynamic",
            "not compiled; rebuild with --features mobile-dynamic",
            "cargo build -p eggsec-cli --features mobile-dynamic",
        )
    };
    // Fixture tests (mock ADB server, dry-run reports) always run.
    let live_supported = compiled
        && (has_binary("adb") || has_binary("emulator"))
        && os.status == PrereqStatus::Available;
    DomainPrerequisites {
        domain: "mobile-dynamic".to_string(),
        features: vec!["mobile".to_string(), "mobile-dynamic".to_string()],
        prerequisites: vec![os, feature, adb, frida_cli, emulator_hint],
        fixture_supported: true,
        live_supported,
        fixture_command: "cargo test -p eggsec-mobile-lab adb_ && ./scripts/test-mobile-dynamic.sh (dry-run)".to_string(),
        hardware_note: "live runs need a lab AVD/device you own plus --allow-dynamic-mobile (--allow-frida for instrumentation)".to_string(),
    }
}

fn packet_matrix() -> DomainPrerequisites {
    let os = os_prereq(true);
    let compiled = cfg!(feature = "packet-inspection");
    let feature = if compiled {
        Prerequisite::available(
            "feature-packet",
            "Feature packet-inspection",
            "compiled in this build",
        )
    } else {
        Prerequisite::missing(
            "feature-packet",
            "Feature packet-inspection",
            "not compiled; rebuild with --features packet-inspection",
            "cargo check -p eggsec --features packet-inspection (needs libpcap-dev)",
        )
    };
    let priv_raw = privilege_prereq(
        "priv-net-raw",
        "CAP_NET_RAW / root",
        has_cap_net_raw(),
        if has_cap_net_raw() {
            "running with capture privilege"
        } else {
            "running without capture privilege; live capture will skip"
        },
        "run live captures in scripts/setup_packet_netns.sh or with sudo/CAP_NET_RAW; fixture tests need nothing",
    );
    let ifaces = list_interfaces();
    let iface = if ifaces.iter().any(|n| n == "lo") {
        Prerequisite::available("iface-lo", "Loopback interface", "lo present")
    } else {
        Prerequisite::missing(
            "iface-lo",
            "Loopback interface",
            "lo not listed",
            "live loopback checks need lo; fixture parser tests still run",
        )
    };
    let live_supported = compiled && os.status == PrereqStatus::Available && has_cap_net_raw();
    DomainPrerequisites {
        domain: "packet-inspection".to_string(),
        features: vec!["packet-inspection".to_string()],
        prerequisites: vec![os, feature, priv_raw, iface],
        fixture_supported: true,
        live_supported,
        fixture_command: "cargo test -p eggsec --lib packet:: (parser/craft/hexdump, no privilege)".to_string(),
        hardware_note: "live capture stays in scripts/setup_packet_netns.sh namespace or loopback; never run the full suite as root".to_string(),
    }
}

fn wireless_matrix() -> DomainPrerequisites {
    let os = os_prereq(true);
    let compiled = cfg!(feature = "wireless");
    let feature = if compiled {
        Prerequisite::available(
            "feature-wireless",
            "Feature wireless",
            "compiled in this build",
        )
    } else {
        Prerequisite::missing(
            "feature-wireless",
            "Feature wireless",
            "not compiled; rebuild with --features wireless",
            "cargo check -p eggsec --features wireless",
        )
    };
    let iwlist = binary_prereq(
        "bin-iwlist",
        "iwlist (wireless-tools)",
        "iwlist",
        "sudo apt-get install wireless-tools (Linux only); --dry-run needs nothing",
    );
    let priv_admin = privilege_prereq(
        "priv-net-admin",
        "CAP_NET_ADMIN / root",
        has_cap_net_admin(),
        if has_cap_net_admin() {
            "running with scan privilege"
        } else {
            "running without scan privilege; real scans skip"
        },
        "run real scans as root/CAP_NET_ADMIN in a lab; fixture tests need nothing",
    );
    let live_supported = compiled
        && os.status == PrereqStatus::Available
        && has_binary("iwlist")
        && has_cap_net_admin();
    DomainPrerequisites {
        domain: "wireless".to_string(),
        features: vec!["wireless".to_string()],
        prerequisites: vec![os, feature, iwlist, priv_admin],
        fixture_supported: true,
        live_supported,
        fixture_command: "cargo test -p eggsec --lib wireless:: (parser/heuristic/fixture frames)"
            .to_string(),
        hardware_note: "passive scans need a managed/up lab interface; RF stays manual/lab-only"
            .to_string(),
    }
}

fn wireless_advanced_matrix() -> DomainPrerequisites {
    let mut base = wireless_matrix();
    let compiled = cfg!(feature = "wireless-advanced");
    let feature = if compiled {
        Prerequisite::available(
            "feature-wireless-advanced",
            "Feature wireless-advanced",
            "compiled in this build",
        )
    } else {
        Prerequisite::missing(
            "feature-wireless-advanced",
            "Feature wireless-advanced",
            "not compiled; rebuild with --features wireless-advanced",
            "cargo check -p eggsec --features wireless-advanced (lab only)",
        )
    };
    let monitor = Prerequisite::missing(
        "iface-monitor",
        "Monitor-mode interface (lab)",
        "not probed automatically; lab hardware only",
        "create wlan0mon via airmon-ng on owned lab hardware; frame tests use fixtures",
    );
    base.domain = "wireless-advanced".to_string();
    base.features = vec!["wireless".to_string(), "wireless-advanced".to_string()];
    base.prerequisites.push(feature);
    base.prerequisites.push(monitor);
    base.live_supported = false; // never auto-claimed; manual lab procedure only
    base.fixture_command =
        "cargo test -p eggsec --lib wireless::active:: (frame crafting, dry-run only)".to_string();
    base.hardware_note =
        "real RF transmission is an explicitly documented manual lab test, never CI".to_string();
    base
}

fn stress_matrix() -> DomainPrerequisites {
    let os = os_prereq(true);
    let compiled = cfg!(feature = "stress-testing");
    let feature = if compiled {
        Prerequisite::available(
            "feature-stress",
            "Feature stress-testing",
            "compiled in this build",
        )
    } else {
        Prerequisite::missing(
            "feature-stress",
            "Feature stress-testing",
            "not compiled; rebuild with --features stress-testing",
            "cargo check -p eggsec --features stress-testing (lab only)",
        )
    };
    let priv_raw = privilege_prereq(
        "priv-net-raw",
        "CAP_NET_RAW / root",
        has_cap_net_raw(),
        if has_cap_net_raw() {
            "running with raw-socket privilege"
        } else {
            "running without raw-socket privilege; live floods skip"
        },
        "live stress stays in an isolated lab namespace with scope+budgets; fixture tests need nothing",
    );
    DomainPrerequisites {
        domain: "stress-testing".to_string(),
        features: vec!["stress-testing".to_string()],
        prerequisites: vec![os, feature, priv_raw],
        fixture_supported: true,
        live_supported: false, // manual lab only, never auto-claimed
        fixture_command: "cargo test -p eggsec --lib stress:: (auth/metrics/caps, no traffic)"
            .to_string(),
        hardware_note: "live floods are manual, scoped, rate/duration-capped lab runs only"
            .to_string(),
    }
}

fn nse_matrix() -> DomainPrerequisites {
    let compiled = cfg!(feature = "nse");
    let feature = if compiled {
        Prerequisite::available("feature-nse", "Feature nse", "compiled in this build")
    } else {
        Prerequisite::missing(
            "feature-nse",
            "Feature nse",
            "not compiled; rebuild with --features nse",
            "cargo check -p eggsec --features nse (needs libssl-dev)",
        )
    };
    DomainPrerequisites {
        domain: "nse".to_string(),
        features: vec!["nse".to_string()],
        prerequisites: vec![
            Prerequisite::available("os", "Operating system", format!("os={}", current_os())),
            feature,
        ],
        fixture_supported: true,
        live_supported: compiled,
        fixture_command: "./scripts/test-nse.sh (safe scripts only)".to_string(),
        hardware_note: "no hardware needed; script fixtures are local".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_domains_have_matrices() {
        for id in all_domain_ids() {
            let m = report_for(&id);
            assert_eq!(m.domain, id);
            assert!(!m.prerequisites.is_empty(), "domain {id} has no rows");
            assert!(
                !m.fixture_command.is_empty(),
                "domain {id} has no fixture command"
            );
            assert!(
                m.fixture_supported,
                "domain {id} must support fixture tests"
            );
        }
    }

    #[test]
    fn unknown_domain_is_fail_closed() {
        let m = report_for("does-not-exist");
        assert!(!m.live_supported);
        assert!(!m.fixture_supported);
        assert!(m.first_blocking().is_none());
    }

    #[test]
    fn skip_reason_names_the_blocker() {
        // wireless-advanced never claims live support, so it always skips.
        let reason = skip_reason_for("wireless-advanced")
            .expect("advanced wireless must always have a skip reason");
        assert!(reason.starts_with("SKIP wireless-advanced:"));
        assert!(reason.contains("fix:"));
    }

    #[test]
    fn report_serializes_and_human_format_is_stable() {
        let report = PlatformReport::collect();
        assert_eq!(report.domains.len(), all_domain_ids().len());
        let json = serde_json::to_string(&report).expect("report serializes");
        assert!(json.contains("mobile-dynamic"));
        assert!(json.contains("packet-inspection"));
        let human = report.format_human();
        assert!(human.contains("=== Eggsec Platform Report ==="));
        assert!(human.contains("Fixture tests never require root"));
    }

    #[test]
    fn detectors_are_read_only_and_total() {
        let _ = current_os();
        let _ = current_arch();
        let _ = current_kernel();
        let _ = is_root();
        let _ = has_cap_net_raw();
        let _ = has_cap_net_admin();
        assert!(!has_binary(""));
        assert!(!has_binary("definitely/not-a-binary"));
        let _ = list_interfaces();
        let _ = capabilities_summary();
        let _ = compiled_features();
    }

    #[test]
    fn lifecycle_loop_is_stable_over_repetition() {
        // Bounded repetition catches formatting/serialization regressions
        // without hardware or privilege.
        for _ in 0..20 {
            let report = PlatformReport::collect();
            let human = report.format_human();
            assert!(human.contains("fixture="));
            let json = serde_json::to_string(&report).expect("serializes every iteration");
            let back: PlatformReport =
                serde_json::from_str(&json).expect("round-trips every iteration");
            assert_eq!(back.domains.len(), report.domains.len());
        }
    }
}
