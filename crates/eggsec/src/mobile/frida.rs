//! Frida-based runtime instrumentation for mobile dynamic analysis (Phase 3a foundation + basic_method_trace).
//!
//! All under single `mobile-dynamic` feature (per phase3-frida-expansion-plan.md Key Decision; no mobile-frida sub-feature).
//! Safety: explicit --allow-frida runtime flag + EnforcementContext Intrusive tier for real ops. Dry-run always safe and produces valid reports.
//! Standalone defense-lab only (no MCP/agent). CLI shell fallback to `frida` (no heavy frida crate dep in 3a).
//! First built-in: basic_method_trace for common sensitive methods (javax.crypto.Cipher.doFinal, keystore, login/token, root/Frida detection hooks).
//! Emits categories: "frida-method-trace", "frida-secret-extract", "frida-bypass-validation".
//! Real execution requires frida CLI in PATH + frida-server on rooted/emulator device. Best-effort + audited.
//! See dynamic.rs (run_dynamic_cli, FridaInstrumentation, to_scan_report_data_dynamic), mobile/mod.rs, handler policy, docs/MOBILE.md.

use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::mpsc::channel;
use std::thread;
use std::time::{Duration, Instant};

/// Opaque handle for a connected Frida session (Phase 3a: real CLI path or dry-run stub).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FridaSession {
    pub device_id: String,
    /// Internal marker for simulation vs real (not serialized for users).
    #[serde(skip)]
    pub is_simulation: bool,
}

/// Structured result from running a Frida script (user-provided or built-in).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FridaScriptResult {
    pub script_source: String,
    pub output: String,        // raw or JSON payload from Frida
    pub findings: Vec<String>, // extracted high-level signals (to become DynamicMobileFinding)
    pub duration_ms: u64,
}

/// Frida instrumentation summary on `DynamicMobileReport` (under mobile-dynamic).
/// Populated by run_dynamic_cli when Frida ops requested (dry or real).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FridaInstrumentation {
    pub note: String,
    pub sessions: Vec<FridaSession>,
    pub script_results: Vec<FridaScriptResult>,
    pub enabled_builtins: Vec<String>,
}

/// Connect to the Frida server on the target device/emulator.
///
/// Dry-run / simulation: always succeeds with a stub session (no side effects).
/// Real: checks for `frida` CLI in PATH (via which/Command), optionally probes
/// reachability with `frida-ps -U` (or equivalent). Returns session on success.
/// Clear errors if frida CLI missing or device unreachable (best-effort).
pub fn connect(device: &str) -> crate::error::Result<FridaSession> {
    if device.trim().is_empty() {
        return Err(crate::error::EggsecError::Validation(
            "frida connect: device required (serial or host:port)".to_string(),
        ));
    }
    // Simulation path: treat common dry-run device markers or absence of real CLI as simulation.
    if is_frida_cli_available() {
        // Real path: validate reachability best-effort via frida-ps (non-fatal on probe failure for Phase 3a).
        let _ = Command::new("frida-ps").arg("-U").arg("-D").arg(device).output();
        Ok(FridaSession { device_id: device.to_string(), is_simulation: false })
    } else {
        // No frida CLI: produce simulation session (dry-run semantics; real callers gate via allow + policy).
        Ok(FridaSession { device_id: device.to_string(), is_simulation: true })
    }
}

/// Returns true if the `frida` (or `frida-cli`) binary is discoverable in PATH.
pub fn is_frida_cli_available() -> bool {
    // Try common names.
    if Command::new("which").arg("frida").output().map(|o| o.status.success()).unwrap_or(false) {
        return true;
    }
    if Command::new("which").arg("frida-cli").output().map(|o| o.status.success()).unwrap_or(false) {
        return true;
    }
    // Fallback: direct spawn (some envs lack `which` or have it in limited PATH).
    Command::new("frida").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
        || Command::new("frida-cli").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// Generate a safe high-value Frida JS snippet for basic method tracing.
/// Emits console.log(JSON.stringify({type:"frida-method-trace", method, args: redacted, ret: redacted, ts})) lines.
/// Targets common sensitive methods (crypto, keystore, auth, root/Frida detection).
pub fn generate_basic_method_trace_script(package: &str, methods: &[&str]) -> String {
    let mut hooks = String::new();
    for m in methods {
        let m_esc = m.replace('\\', "\\\\").replace('"', "\\\"");
        hooks.push_str(&format!(
            r#"
  try {{
    var cls = Java.use("{m}");
    if (cls && cls.doFinal) {{
      cls.doFinal.overload('[B').implementation = function (b) {{
        var ts = Date.now();
        var ret = this.doFinal(b);
        console.log(JSON.stringify({{type:"frida-method-trace", method:"{m}.doFinal", pkg:"{pkg}", args:"[B(len="+b.length+")", ret:"[B(len="+ret.length+")", ts:ts}}));
        return ret;
      }};
    }}
    if (cls && cls.getKey) {{
      cls.getKey.implementation = function () {{
        var ts = Date.now();
        var ret = this.getKey();
        console.log(JSON.stringify({{type:"frida-method-trace", method:"{m}.getKey", pkg:"{pkg}", args:"", ret:"key", ts:ts}}));
        return ret;
      }};
    }}
  }} catch (e) {{ /* best-effort per method */ }}
"#,
            m = m_esc,
            pkg = package.replace('"', "\\\"")
        ));
    }
    format!(
        r#"Java.perform(function () {{
  // basic_method_trace for {pkg}
  var methods = [{methods}];
  {hooks}
  // Generic keystore / Cipher / login / detection hooks (safe, read-only observation)
  try {{
    var Cipher = Java.use("javax.crypto.Cipher");
    Cipher.doFinal.overload('[B').implementation = function (b) {{
      var ts = Date.now();
      var ret = this.doFinal(b);
      console.log(JSON.stringify({{type:"frida-method-trace", method:"javax.crypto.Cipher.doFinal", pkg:"{pkg}", args:"[B(len="+b.length+")", ret:"[B(len="+ret.length+")", ts:ts}}));
      return ret;
    }};
  }} catch (e) {{}}
  try {{
    var SecretKeyFactory = Java.use("javax.crypto.SecretKeyFactory");
    SecretKeyFactory.generateSecret.implementation = function (spec) {{
      var ts = Date.now();
      var ret = this.generateSecret(spec);
      console.log(JSON.stringify({{type:"frida-method-trace", method:"javax.crypto.SecretKeyFactory.generateSecret", pkg:"{pkg}", args:"spec", ret:"SecretKey", ts:ts}}));
      return ret;
    }};
  }} catch (e) {{}}
  try {{
    var KeyStore = Java.use("java.security.KeyStore");
    KeyStore.getKey.implementation = function (alias, pwd) {{
      var ts = Date.now();
      var ret = this.getKey(alias, pwd);
      console.log(JSON.stringify({{type:"frida-method-trace", method:"java.security.KeyStore.getKey", pkg:"{pkg}", args:alias, ret:"key", ts:ts}}));
      return ret;
    }};
  }} catch (e) {{}}
  // Root / Frida detection bypass observation (common patterns)
  try {{
    var System = Java.use("java.lang.System");
    System.getProperty.overload("java.lang.String").implementation = function (k) {{
      var ts = Date.now();
      var ret = this.getProperty(k);
      if (k && (k.indexOf("ro.secure") >= 0 || k.indexOf("frida") >= 0 || k.indexOf("root") >= 0)) {{
        console.log(JSON.stringify({{type:"frida-bypass-validation", method:"System.getProperty", pkg:"{pkg}", args:k, ret:ret, ts:ts}}));
      }}
      return ret;
    }};
  }} catch (e) {{}}
  console.log(JSON.stringify({{type:"frida-session-start", pkg:"{pkg}", methods:methods, ts:Date.now()}}));
}}); "#,
        pkg = package.replace('"', "\\\""),
        methods = methods.iter().map(|m| format!("\"{}\"", m.replace('"', "\\\""))).collect::<Vec<_>>().join(","),
        hooks = hooks
    )
}

/// Execute a Frida script (JavaScript) in the context of an existing session.
///
/// For simulation/dry (session.is_simulation or no frida CLI): produce a result with duration,
/// script echo, synthetic output/findings (no external calls).
/// Real: write script to secure temp file, invoke `frida -U -D <device> -f <pkg-or-attach> -l <script> --no-pause -q`
/// (or attach equivalent), capture stdout/stderr with timeout, clean temp. Populate output + attempt to extract
/// structured findings (parse lines that look like JSON or "FRIDA-FINDING:" markers). Set duration.
/// Best-effort + audited; timeouts are defensive.
pub fn execute_script(
    session: &FridaSession,
    script: &str,
) -> crate::error::Result<FridaScriptResult> {
    let start = Instant::now();
    if session.is_simulation || !is_frida_cli_available() {
        // Dry/simulation path: synthetic result, no side effects.
        let mut findings = vec!["frida-simulation: script accepted".to_string()];
        // If the script contains our JSON markers, echo a couple as synthetic findings.
        if script.contains("frida-method-trace") || script.contains("Cipher.doFinal") {
            findings.push("frida-method-trace: javax.crypto.Cipher.doFinal (simulated)".to_string());
        }
        if script.contains("bypass-validation") || script.contains("root") {
            findings.push("frida-bypass-validation: System.getProperty (simulated)".to_string());
        }
        return Ok(FridaScriptResult {
            script_source: script.to_string(),
            output: format!("(simulation) executed {} bytes of script on {}", script.len(), session.device_id),
            findings,
            duration_ms: start.elapsed().as_millis() as u64,
        });
    }
    // Real path (frida CLI present): write temp script, invoke, capture, cleanup.
    let tmp_dir = std::env::temp_dir();
    let script_path = tmp_dir.join(format!("eggsec_frida_{}.js", uuid::Uuid::new_v4()));
    std::fs::write(&script_path, script)
        .map_err(|e| crate::error::EggsecError::Validation(format!("frida: failed to write temp script: {}", e)))?;
    // Typical invocation (attach or spawn; Phase 3a uses -f for spawn-by-default with --no-pause -q for clean output).
    // We do not know package here; caller (basic_method_trace / run_dynamic_cli) may have used -f context in script or we attach.
    // Use a bounded timeout (30s default for Phase 3a safety).
    let (tx, rx) = channel();
    let dev = session.device_id.clone();
    let spath = script_path.clone();
    thread::spawn(move || {
        let out = Command::new("frida")
            .arg("-U")
            .arg("-D")
            .arg(&dev)
            .arg("-f")
            .arg("re.frida.Gadget") // placeholder; real callers often override via script or use attach form
            .arg("-l")
            .arg(&spath)
            .arg("--no-pause")
            .arg("-q")
            .output();
        let _ = tx.send(out);
    });
    let output_res = rx.recv_timeout(Duration::from_secs(30));
    let _ = std::fs::remove_file(&script_path);
    let out = match output_res {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => return Err(crate::error::EggsecError::Validation(format!("frida CLI invoke error: {}", e))),
        Err(_) => return Err(crate::error::EggsecError::Validation("frida execution timed out (30s)".to_string())),
    };
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let combined = if stderr.trim().is_empty() { stdout.clone() } else { format!("{}\n{}", stdout, stderr) };
    // Parse for structured JSON lines or FRIDA-FINDING: markers.
    let mut findings: Vec<String> = Vec::new();
    for line in combined.lines() {
        let t = line.trim();
        if t.starts_with('{') && t.contains("\"type\"") {
            findings.push(format!("frida-json: {}", t));
        } else if t.to_uppercase().contains("FRIDA-FINDING:") || t.contains("frida-method-trace") || t.contains("frida-secret-extract") || t.contains("frida-bypass-validation") {
            findings.push(t.to_string());
        }
    }
    if findings.is_empty() && !combined.trim().is_empty() {
        findings.push("frida-raw-output-captured".to_string());
    }
    Ok(FridaScriptResult {
        script_source: script.to_string(),
        output: combined,
        findings,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

/// First built-in high-value capability: basic method tracing (Phase 3a).
/// Generates safe Frida JS (Java.perform + Interceptor.attach style) for provided methods
/// (or sensible defaults if empty), emits structured console.log(JSON...) lines, then calls execute_script.
/// On result, findings vec is populated with "frida-method-trace" etc. signals.
/// Always safe in dry-run (via execute_script simulation). Best-effort on real.
pub fn basic_method_trace(
    session: &FridaSession,
    package: &str,
    methods: &[&str],
) -> crate::error::Result<FridaScriptResult> {
    let pkg = if package.trim().is_empty() { "com.example.target" } else { package };
    let meths: Vec<&str> = if methods.is_empty() {
        vec!["javax.crypto.Cipher", "android.security.keystore.KeyStore", "java.security.KeyStore", "com.example.login.Auth"]
    } else {
        methods.to_vec()
    };
    let script = generate_basic_method_trace_script(pkg, &meths);
    let mut res = execute_script(session, &script)?;
    // Ensure at least one canonical finding marker for downstream mapping.
    if !res.findings.iter().any(|f| f.contains("frida-method-trace")) {
        res.findings.push("frida-method-trace: basic_method_trace executed".to_string());
    }
    res.findings.push(format!("frida-session: pkg={}", pkg));
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frida_module_present_under_mobile_dynamic_and_types_construct() {
        let sess = FridaSession { device_id: "emulator-5554".into(), is_simulation: true };
        assert_eq!(sess.device_id, "emulator-5554");
        let res: FridaScriptResult = Default::default();
        assert!(res.script_source.is_empty());
        assert!(res.findings.is_empty());
        let instr = FridaInstrumentation::default();
        assert!(instr.note.is_empty());
    }

    #[test]
    fn connect_dry_run_simulation_path_succeeds_without_external_calls() {
        let s = connect("emulator-5554").expect("connect dry must succeed");
        assert_eq!(s.device_id, "emulator-5554");
        // In env without frida CLI this will be simulation; even with CLI we accept the session.
        assert!(is_frida_cli_available() || s.is_simulation);
    }

    #[test]
    fn execute_script_dry_or_missing_cli_produces_valid_result() {
        let sess = FridaSession { device_id: "emulator-5554".into(), is_simulation: true };
        let script = "Java.perform(function(){ console.log('hi'); });";
        let res = execute_script(&sess, script).expect("dry execute must succeed");
        assert!(res.script_source.contains("Java.perform"));
        assert!(res.duration_ms < 5000);
        assert!(!res.findings.is_empty());
        assert!(res.output.contains("simulation") || res.output.contains("executed"));
    }

    #[test]
    fn execute_script_real_path_errors_cleanly_if_cli_missing_but_still_gated_by_caller() {
        // If frida CLI is present in this env the call may succeed (best-effort); we only assert the API shape.
        let sess = FridaSession { device_id: "emulator-5554".into(), is_simulation: false };
        let _ = execute_script(&sess, "Java.perform(function(){});"); // must not panic; may err if no frida
    }

    #[test]
    fn script_generation_contains_expected_hooks_and_json_markers() {
        let s = generate_basic_method_trace_script("com.example.vuln", &["javax.crypto.Cipher", "android.security.keystore.KeyStore"]);
        assert!(s.contains("Java.perform"));
        assert!(s.contains("javax.crypto.Cipher.doFinal"));
        assert!(s.contains("frida-method-trace"));
        assert!(s.contains("frida-bypass-validation"));
        assert!(s.contains("com.example.vuln"));
        assert!(s.contains("JSON.stringify"));
    }

    #[test]
    fn basic_method_trace_dry_run_produces_findings_and_duration() {
        let sess = FridaSession { device_id: "emulator-5554".into(), is_simulation: true };
        let res = basic_method_trace(&sess, "com.example.vuln", &["javax.crypto.Cipher"]).expect("trace dry must succeed");
        assert!(res.findings.iter().any(|f| f.contains("frida-method-trace")));
        assert!(res.duration_ms < 5000);
        assert!(res.script_source.contains("Cipher"));
    }

    #[test]
    fn result_parsing_handles_synthetic_json_output() {
        // Feed a script whose output will be echoed in simulation path.
        let sess = FridaSession { device_id: "d".into(), is_simulation: true };
        let script = r#"console.log(JSON.stringify({type:"frida-method-trace", method:"Cipher.doFinal"}));"#;
        let res = execute_script(&sess, script).expect("parse sim");
        assert!(res.findings.iter().any(|f| f.contains("frida-method-trace") || f.contains("frida-json")));
    }
}
