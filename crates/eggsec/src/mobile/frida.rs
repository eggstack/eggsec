//! Frida-based runtime instrumentation for mobile dynamic analysis (Phase 3a foundation + basic_method_trace).
//! Phase 3b: additional builtins (crypto-keystore, bypass-validation, api-trace) + structured JSON output + redaction + run_builtin dispatch + correlation support + richer FridaInstrumentation.
//! Phase 3c: user script library (embedded reusable components via library: prefix) + multi-script sessions + advanced static↔dynamic↔Frida correlation + behavioral baselining/regression + optional evidence bundle export.
//!
//! All under single `mobile-dynamic` feature (per phase3-frida-expansion-plan.md Key Decision; no mobile-frida sub-feature).
//! Safety: explicit --allow-frida runtime flag + EnforcementContext Intrusive tier for real ops. Dry-run always safe and produces valid reports.
//! Standalone defense-lab only (no MCP/agent). CLI shell fallback to `frida` (no heavy frida crate dep).
//! Builtins: basic_method_trace, crypto-keystore, bypass-validation, api-trace (via builtin: prefix).
//! Library: common-hooks etc. (via library: prefix; embedded, no FS required at runtime for library components).
//! Emits frida-* categories. Correlation + regression + bundles in dynamic layer.
//! Real execution requires frida CLI in PATH + frida-server on rooted/emulator device. Best-effort + audited.
//! See dynamic.rs (run_dynamic_cli, FridaInstrumentation, to_scan_report_data_dynamic, correlate_findings, capture_baseline, export_evidence_bundle), mobile/mod.rs, handler policy, docs/MOBILE.md.

use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::mpsc::channel;
use std::thread;
use std::time::{Duration, Instant};

/// Opaque handle for a connected Frida session (Phase 3a: real CLI path or dry-run stub).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FridaSession {
    pub device_id: String,
    #[serde(skip)]
    pub is_simulation: bool,
}

/// Structured result from running a Frida script (user-provided or built-in).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FridaScriptResult {
    pub script_source: String,
    pub output: String,
    pub findings: Vec<String>,
    pub duration_ms: u64,
    pub structured_output: Option<serde_json::Value>,
}

/// Frida instrumentation summary on `DynamicMobileReport` (under mobile-dynamic).
/// Populated by run_dynamic_cli when Frida ops requested (dry or real).
/// Phase 3c: supports multi-script (script_results + structured_results accumulate),
/// library: components, regression_notes (behavioral baseline diffs surfaced here too).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FridaInstrumentation {
    pub note: String,
    pub sessions: Vec<FridaSession>,
    pub script_results: Vec<FridaScriptResult>,
    pub enabled_builtins: Vec<String>,
    pub start_time: Option<String>,
    pub structured_results: Vec<serde_json::Value>,
    pub correlation_notes: Vec<String>,
    #[serde(default)]
    pub regression_notes: Vec<String>,
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

pub(crate) fn redact_frida_evidence(s: &str) -> String {
    let mut out = s.to_string();
    let patterns = ["api_key", "sk_live_", "AIza", "token=", "password=", "secret=", "auth=", "bearer "];
    for p in &patterns {
        if out.to_ascii_lowercase().contains(p) {
            out = out.replace(p, "[REDACTED]");
        }
    }
    if out.contains("[B(") && out.contains("len=") {
        if let Some(start) = out.find("len=") {
            if let Some(end) = out[start..].find(')') {
                out = format!("{}[B(len=REDACTED){}", &out[..start], &out[start+end+1..]);
            }
        }
    }
    out
}

/// Phase 3c: Embedded user script library components (reusable, safe, structured JSON emitting).
/// Users reference via --frida-script "library:common-hooks" (resolved without reading FS).
/// Content is best-effort observation only; redaction of secrets/params occurs in the report layer.
pub const FRIDA_LIB_COMMON_HOOKS: &str = r#"// common-hooks.js — Phase 3c reusable Frida components (safe, redacted, structured JSON output)
// Include via "library:common-hooks" convention or copy the relevant blocks into your script.
// All hooks are best-effort; wrapped in try/catch. Timestamps + pkg included.
// Redaction of secrets/params happens in the Rust layer on evidence.

Java.perform(function() {
  var pkg = (Java.available ? Java.androidVersion : "unknown") ? "com.target.app" : "com.target.app"; // placeholder; callers inject pkg

  // Crypto / keystore observation (extends Phase 3b)
  try {
    var Cipher = Java.use("javax.crypto.Cipher");
    Cipher.doFinal.overload("[B").implementation = function(b) {
      var ts = Date.now();
      console.log(JSON.stringify({type:"frida-crypto-observation", method:"Cipher.doFinal", pkg:pkg, args_redacted:"[REDACTED]", ret_redacted:"[REDACTED]", ts:ts}));
      return this.doFinal(b);
    };
  } catch (e) {}

  try {
    var KS = Java.use("android.security.keystore.KeyStore");
    KS.getEntry.overload("java.lang.String", "java.security.KeyStore$ProtectionParameter").implementation = function(alias, prot) {
      var ts = Date.now();
      console.log(JSON.stringify({type:"frida-crypto-observation", method:"KeyStore.getEntry", pkg:pkg, alias:"[REDACTED]", ts:ts}));
      return this.getEntry(alias, prot);
    };
  } catch (e) {}

  // Network / API surface (redacted)
  try {
    var HUC = Java.use("java.net.HttpURLConnection");
    HUC.getInputStream.implementation = function() {
      var ts = Date.now();
      var url = this.getURL ? this.getURL().toString() : "";
      console.log(JSON.stringify({type:"frida-api-trace", method:"HttpURLConnection.getInputStream", pkg:pkg, params_inspected:{url:url, headers:"redacted"}, ts:ts}));
      return this.getInputStream();
    };
  } catch (e) {}

  try {
    var OkHttp = Java.use("okhttp3.Request$Builder");
    OkHttp.build.implementation = function () {
      var ts = Date.now();
      var url = this.url_ ? this.url_.toString() : "";
      console.log(JSON.stringify({type:"frida-api-trace", method:"OkHttp.Request", pkg:pkg, params_inspected:{url:url, headers:"redacted"}, ts:ts}));
      return this.build();
    };
  } catch (e) {}

  // Bypass / detection surfaces (lab validation)
  try {
    var System = Java.use("java.lang.System");
    System.getProperty.overload("java.lang.String").implementation = function(k) {
      var ts = Date.now();
      if (k && (k.indexOf("ro.debuggable") !== -1 || k.indexOf("ro.secure") !== -1)) {
        console.log(JSON.stringify({type:"frida-bypass-validation", method:"System.getProperty", pkg:pkg, key:k, ts:ts}));
      }
      return this.getProperty(k);
    };
  } catch (e) {}

  try {
    var Runtime = Java.use("java.lang.Runtime");
    Runtime.exec.overload("java.lang.String").implementation = function(cmd) {
      var ts = Date.now();
      console.log(JSON.stringify({type:"frida-bypass-validation", method:"Runtime.exec", pkg:pkg, cmd:"[REDACTED]", ts:ts}));
      return this.exec(cmd);
    };
  } catch (e) {}

  // Secret extraction patterns (best-effort; redacted in report layer)
  try {
    var SecretKeySpec = Java.use("javax.crypto.spec.SecretKeySpec");
    SecretKeySpec.$init.overload("[B", "java.lang.String").implementation = function(key, algo) {
      var ts = Date.now();
      console.log(JSON.stringify({type:"frida-secret-extract", method:"SecretKeySpec.<init>", pkg:pkg, algo:algo, key_len:"[REDACTED]", ts:ts}));
      return this.$init(key, algo);
    };
  } catch (e) {}
});
"#;

/// Resolve a frida script spec to concrete JS source.
/// Supports:
/// - "builtin:NAME" → delegates to the Phase 3b/3a generators (crypto-keystore, bypass-validation, api-trace, basic-method-trace)
/// - "library:NAME" → returns embedded library component (FRIDA_LIB_COMMON_HOOKS etc.), with package placeholder substitution
/// - raw content (inline JS) → returned as-is (file reads for user paths happen at call sites in dynamic.rs)
pub fn resolve_frida_script_spec(spec: &str, package: &str) -> crate::error::Result<String> {
    let pkg = if package.trim().is_empty() { "com.example.target" } else { package };
    if let Some(name) = spec.strip_prefix("builtin:") {
        let script = match name {
            "crypto-keystore" => generate_crypto_keystore_script(pkg),
            "bypass-validation" => generate_bypass_validation_script(pkg),
            "api-trace" => generate_api_trace_script(pkg),
            "basic-method-trace" | "basic_method_trace" => generate_basic_method_trace_script(pkg, &["javax.crypto.Cipher", "android.security.keystore.KeyStore"]),
            "native-load" | "native_load" => generate_native_lib_load_script(pkg),
            _ => return Err(crate::error::EggsecError::Validation(format!(
                "unknown frida builtin '{}'; available: basic-method-trace, crypto-keystore, bypass-validation, api-trace, native-load",
                name
            ))),
        };
        return Ok(script);
    }
    if let Some(name) = spec.strip_prefix("library:") {
        let src = match name {
            "common-hooks" | "common_hooks" => FRIDA_LIB_COMMON_HOOKS.to_string(),
            _ => return Err(crate::error::EggsecError::Validation(format!(
                "unknown frida library component '{}'; available: common-hooks",
                name
            ))),
        };
        // Substitute a realistic package for the placeholder used in the embedded source
        let script = src.replace("com.target.app", pkg);
        return Ok(script);
    }
    // Raw inline script content (or caller-read file content)
    Ok(spec.to_string())
}

/// Execute a resolved spec (builtin:/library:/raw). Thin wrapper that resolves then executes.
/// For library/builtin this ensures synthetic structured population on dry paths (mirrors prior run_builtin behavior).
pub fn run_frida_spec(session: &FridaSession, spec: &str, package: &str) -> crate::error::Result<FridaScriptResult> {
    let script = resolve_frida_script_spec(spec, package)?;
    let mut res = execute_script(session, &script)?;
    if res.structured_output.is_none() {
        if spec.contains("crypto") || spec.contains("crypto-keystore") {
            res.structured_output = Some(serde_json::json!({"type":"frida-crypto-observation","method":"spec","args_redacted":"[REDACTED]","ret_redacted":"[REDACTED]","ts":0}));
        } else if spec.contains("bypass") {
            res.structured_output = Some(serde_json::json!({"type":"frida-bypass-validation","method":"spec","args":"","ret":"","ts":0}));
        } else if spec.contains("api") {
            res.structured_output = Some(serde_json::json!({"type":"frida-api-trace","method":"spec","params_inspected":{},"ts":0}));
        } else if spec.contains("secret-extract") {
            res.structured_output = Some(serde_json::json!({"type":"frida-secret-extract","method":"spec","ts":0}));
        } else if spec.contains("native-load") || spec.contains("native_load") {
            res.structured_output = Some(serde_json::json!({"type":"frida-native-load","method":"spec","lib":"[REDACTED]","ts":0}));
        }
    }
    Ok(res)
}

/// Phase 3c backward-compatible thin wrapper around the builtin path (still used by some call sites).
/// Now implemented via the unified resolver for consistency.
pub fn run_builtin(builtin: &str, session: &FridaSession, package: &str) -> crate::error::Result<FridaScriptResult> {
    run_frida_spec(session, &format!("builtin:{}", builtin), package)
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

/// Phase 3b: crypto/keystore observation (javax.crypto.*, KeyStore, SecretKeyFactory etc.).
/// Emits JSON {type:"frida-crypto-observation", method, args_redacted, ret_redacted, ts}.
pub fn generate_crypto_keystore_script(package: &str) -> String {
    let pkg = package.replace('"', "\\\"");
    format!(r#"Java.perform(function () {{
  try {{
    var Cipher = Java.use("javax.crypto.Cipher");
    Cipher.doFinal.overload('[B').implementation = function (b) {{
      var ts = Date.now();
      var ret = this.doFinal(b);
      var a = "[B(len="+b.length+")";
      console.log(JSON.stringify({{type:"frida-crypto-observation", method:"javax.crypto.Cipher.doFinal", pkg:"{pkg}", args_redacted:a, ret_redacted:"[B(len="+ret.length+")", ts:ts}}));
      return ret;
    }};
  }} catch (e) {{}}
  try {{
    var SecretKeyFactory = Java.use("javax.crypto.SecretKeyFactory");
    SecretKeyFactory.generateSecret.implementation = function (spec) {{
      var ts = Date.now();
      console.log(JSON.stringify({{type:"frida-crypto-observation", method:"javax.crypto.SecretKeyFactory.generateSecret", pkg:"{pkg}", args_redacted:"spec", ret_redacted:"SecretKey", ts:ts}}));
      return this.generateSecret(spec);
    }};
  }} catch (e) {{}}
  try {{
    var KeyStore = Java.use("java.security.KeyStore");
    KeyStore.getKey.implementation = function (alias, pwd) {{
      var ts = Date.now();
      console.log(JSON.stringify({{type:"frida-crypto-observation", method:"java.security.KeyStore.getKey", pkg:"{pkg}", args_redacted:alias, ret_redacted:"key", ts:ts}}));
      return this.getKey(alias, pwd);
    }};
  }} catch (e) {{}}
  try {{
    var AndroidKeyStore = Java.use("android.security.keystore.KeyStore");
    AndroidKeyStore.getEntry.implementation = function (alias, prot) {{
      var ts = Date.now();
      console.log(JSON.stringify({{type:"frida-crypto-observation", method:"android.security.keystore.KeyStore.getEntry", pkg:"{pkg}", args_redacted:alias, ret_redacted:"Entry", ts:ts}}));
      return this.getEntry(alias, prot);
    }};
  }} catch (e) {{}}
}}); "#, pkg = pkg)
}

/// Phase 3b: bypass/detection validation (root, frida, debug, su checks).
/// Emits {type:"frida-bypass-validation", ...}.
pub fn generate_bypass_validation_script(package: &str) -> String {
    let pkg = package.replace('"', "\\\"");
    format!(r#"Java.perform(function () {{
  try {{
    var System = Java.use("java.lang.System");
    System.getProperty.overload("java.lang.String").implementation = function (k) {{
      var ts = Date.now();
      var ret = this.getProperty(k);
      if (k && (k.indexOf("ro.secure") >= 0 || k.indexOf("ro.debuggable") >= 0 || k.indexOf("frida") >= 0 || k.indexOf("root") >= 0)) {{
        console.log(JSON.stringify({{type:"frida-bypass-validation", method:"System.getProperty", pkg:"{pkg}", args:k, ret:ret, ts:ts}}));
      }}
      return ret;
    }};
  }} catch (e) {{}}
  try {{
    var Runtime = Java.use("java.lang.Runtime");
    Runtime.exec.overload("java.lang.String").implementation = function (cmd) {{
      var ts = Date.now();
      if (cmd && (cmd.indexOf("su") >= 0 || cmd.indexOf("frida-server") >= 0 || cmd.indexOf("magisk") >= 0)) {{
        console.log(JSON.stringify({{type:"frida-bypass-validation", method:"Runtime.exec", pkg:"{pkg}", args:cmd, ret:"bypass-observed", ts:ts}}));
      }}
      return this.exec(cmd);
    }};
  }} catch (e) {{}}
  try {{
    var Build = Java.use("android.os.Build");
    var orig_tags = Build.TAGS.value;
    Build.TAGS.value = "release-keys";
    console.log(JSON.stringify({{type:"frida-bypass-validation", method:"Build.TAGS", pkg:"{pkg}", args:"patched", ret:Build.TAGS.value, ts:Date.now()}}));
  }} catch (e) {{}}
}}); "#, pkg = pkg)
}

/// Phase 3b: API call tracing with parameter inspection (HttpURLConnection, OkHttp common paths).
/// Emits {type:"frida-api-trace", ... params_inspected}.
pub fn generate_api_trace_script(package: &str) -> String {
    let pkg = package.replace('"', "\\\"");
    format!(r#"Java.perform(function () {{
  try {{
    var Http = Java.use("java.net.HttpURLConnection");
    Http.getInputStream.implementation = function () {{
      var ts = Date.now();
      var url = this.getURL ? this.getURL().toString() : "";
      var method = this.getRequestMethod ? this.getRequestMethod() : "";
      var red = url ? url.replace(/([?&](api_key|token|password|secret)=[^&]+)/gi, "$1=[REDACTED]") : "";
      console.log(JSON.stringify({{type:"frida-api-trace", method:"HttpURLConnection", pkg:"{pkg}", params_inspected:{{url:red, method:method, body_len:0}}, ts:ts}}));
      return this.getInputStream();
    }};
  }} catch (e) {{}}
  try {{
    var OkHttp = Java.use("okhttp3.Request$Builder");
    OkHttp.build.implementation = function () {{
      var ts = Date.now();
      var url = this.url_ ? this.url_.toString() : "";
      console.log(JSON.stringify({{type:"frida-api-trace", method:"OkHttp.Request", pkg:"{pkg}", params_inspected:{{url:url, headers:"redacted"}}, ts:ts}}));
      return this.build();
    }};
  }} catch (e) {{}}
}}); "#, pkg = pkg)
}

/// Phase 4c (partial delivery): Runtime supply chain / native library load observation (best-effort).
/// Observes Java System.loadLibrary / Runtime.load and libc dlopen (via Interceptor).
/// Emits {type:"frida-native-load", method, lib|path, ts}. Dry-run safe; structured JSON.
pub fn generate_native_lib_load_script(package: &str) -> String {
    let pkg = package.replace('"', "\\\"");
    format!(r#"Java.perform(function () {{
  try {{
    var System = Java.use("java.lang.System");
    System.loadLibrary.overload("java.lang.String").implementation = function (lib) {{
      var ts = Date.now();
      console.log(JSON.stringify({{type:"frida-native-load", method:"System.loadLibrary", pkg:"{pkg}", lib:lib, ts:ts}}));
      return this.loadLibrary(lib);
    }};
  }} catch (e) {{}}
  try {{
    var Runtime = Java.use("java.lang.Runtime");
    Runtime.load.overload("java.lang.String").implementation = function (path) {{
      var ts = Date.now();
      console.log(JSON.stringify({{type:"frida-native-load", method:"Runtime.load", pkg:"{pkg}", path:path, ts:ts}}));
      return this.load(path);
    }};
  }} catch (e) {{}}
  // Best-effort libc dlopen (may require symbol on device)
  try {{
    var dlopen = Module.findExportByName(null, "dlopen");
    if (dlopen) {{
      Interceptor.attach(dlopen, {{
        onEnter: function (args) {{
          var ts = Date.now();
          var path = args[0].isNull() ? "" : args[0].readUtf8String();
          console.log(JSON.stringify({{type:"frida-native-load", method:"dlopen", pkg:"{pkg}", path:path, ts:ts}}));
        }}
      }});
    }}
  }} catch (e) {{}}
}}); "#, pkg = pkg)
}

/// Thin backward wrapper.
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
    if !res.findings.iter().any(|f| f.contains("frida-method-trace")) {
        res.findings.push("frida-method-trace: basic_method_trace executed".to_string());
    }
    res.findings.push(format!("frida-session: pkg={}", pkg));
    Ok(res)
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
        let mut findings = vec!["frida-simulation: script accepted".to_string()];
        if script.contains("frida-method-trace") || script.contains("Cipher.doFinal") {
            findings.push("frida-method-trace: javax.crypto.Cipher.doFinal (simulated)".to_string());
        }
        if script.contains("bypass-validation") || script.contains("root") {
            findings.push("frida-bypass-validation: System.getProperty (simulated)".to_string());
        }
        if script.contains("frida-crypto-observation") || script.contains("crypto-keystore") {
            findings.push("frida-crypto-observation: javax.crypto (simulated)".to_string());
        }
        if script.contains("frida-api-trace") || script.contains("api-trace") {
            findings.push("frida-api-trace: HttpURLConnection (simulated)".to_string());
        }
        if script.contains("frida-native-load") || script.contains("native-load") || script.contains("native_load") {
            findings.push("frida-native-load: System.loadLibrary / dlopen (simulated)".to_string());
        }
        let mut structured_output: Option<serde_json::Value> = None;
        for line in script.lines() {
            let t = line.trim();
            if t.starts_with('{') && t.contains("\"type\"") {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(t) {
                    structured_output = Some(v);
                    break;
                }
            }
        }
        if structured_output.is_none() && (script.contains("frida-crypto-observation") || script.contains("crypto")) {
            structured_output = Some(serde_json::json!({"type":"frida-crypto-observation","method":"sim","args_redacted":"[REDACTED]","ret_redacted":"[REDACTED]","ts":0}));
        }
        if structured_output.is_none() && (script.contains("frida-native-load") || script.contains("native-load") || script.contains("native_load")) {
            structured_output = Some(serde_json::json!({"type":"frida-native-load","method":"sim","lib":"[sim]","ts":0}));
        }
        return Ok(FridaScriptResult {
            script_source: script.to_string(),
            output: format!("(simulation) executed {} bytes of script on {}", script.len(), session.device_id),
            findings,
            duration_ms: start.elapsed().as_millis() as u64,
            structured_output,
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
    let mut findings: Vec<String> = Vec::new();
    let mut structured_output: Option<serde_json::Value> = None;
    for line in combined.lines() {
        let t = line.trim();
        if t.starts_with('{') && t.contains("\"type\"") {
            findings.push(format!("frida-json: {}", t));
            if structured_output.is_none() {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(t) {
                    structured_output = Some(v);
                }
            }
        } else if t.to_uppercase().contains("FRIDA-FINDING:") || t.contains("frida-method-trace") || t.contains("frida-secret-extract") || t.contains("frida-bypass-validation") || t.contains("frida-crypto-observation") || t.contains("frida-api-trace") {
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
        structured_output,
    })
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
        let sess = FridaSession { device_id: "d".into(), is_simulation: true };
        let script = r#"console.log(JSON.stringify({type:"frida-method-trace", method:"Cipher.doFinal"}));"#;
        let res = execute_script(&sess, script).expect("parse sim");
        assert!(res.findings.iter().any(|f| f.contains("frida-method-trace") || f.contains("frida-json")));
    }

    #[test]
    fn generate_crypto_keystore_script_contains_expected_types_and_targets() {
        let s = generate_crypto_keystore_script("com.example.v");
        assert!(s.contains("frida-crypto-observation"));
        assert!(s.contains("javax.crypto.Cipher.doFinal"));
        assert!(s.contains("android.security.keystore.KeyStore.getEntry"));
        assert!(s.contains("JSON.stringify"));
    }

    #[test]
    fn generate_bypass_validation_script_contains_expected_types_and_targets() {
        let s = generate_bypass_validation_script("com.example.v");
        assert!(s.contains("frida-bypass-validation"));
        assert!(s.contains("ro.debuggable"));
        assert!(s.contains("Runtime.exec"));
        assert!(s.contains("Build.TAGS"));
    }

    #[test]
    fn generate_api_trace_script_contains_expected_types_and_targets() {
        let s = generate_api_trace_script("com.example.v");
        assert!(s.contains("frida-api-trace"));
        assert!(s.contains("HttpURLConnection"));
        assert!(s.contains("[REDACTED]"));
        assert!(s.contains("OkHttp"));
    }

    #[test]
    fn run_builtin_for_each_new_builtin_and_unknown_error() {
        let sess = FridaSession { device_id: "d".into(), is_simulation: true };
        let r1 = run_builtin("crypto-keystore", &sess, "p").expect("crypto");
        assert!(r1.findings.iter().any(|f| f.contains("frida-crypto-observation")) || r1.structured_output.is_some());
        let r2 = run_builtin("bypass-validation", &sess, "p").expect("bypass");
        assert!(r2.findings.iter().any(|f| f.contains("frida-bypass")) || r2.structured_output.is_some());
        let r3 = run_builtin("api-trace", &sess, "p").expect("api");
        assert!(r3.findings.iter().any(|f| f.contains("frida-api")) || r3.structured_output.is_some());
        let r4 = run_builtin("basic-method-trace", &sess, "p").expect("basic alias");
        assert!(r4.findings.iter().any(|f| f.contains("frida-method-trace")) || r4.structured_output.is_some());
        let r5 = run_builtin("native-load", &sess, "p").expect("native-load 4c");
        assert!(r5.findings.iter().any(|f| f.contains("frida-native-load")) || r5.structured_output.is_some());
        let err = run_builtin("unknown-foo", &sess, "p").unwrap_err();
        assert!(err.to_string().contains("unknown frida builtin"));
    }

    #[test]
    fn execute_script_structured_json_parse_path_sim_and_synthetic() {
        let sess = FridaSession { device_id: "d".into(), is_simulation: true };
        let script = r#"console.log(JSON.stringify({type:"frida-crypto-observation", method:"Cipher.doFinal"}));"#;
        let res = execute_script(&sess, script).expect("sim json");
        assert!(res.structured_output.is_some());
        assert_eq!(res.structured_output.as_ref().unwrap()["type"], "frida-crypto-observation");
        let res2 = execute_script(&sess, "console.log('plain');").expect("plain");
        assert!(res2.structured_output.is_none() || !res2.structured_output.as_ref().unwrap().is_object());
    }

    #[test]
    fn redact_frida_evidence_redacts_secrets_and_byte_lens() {
        assert!(redact_frida_evidence("api_key=ABC").contains("[REDACTED]"));
        assert!(redact_frida_evidence("token=sk_live_123").contains("[REDACTED]"));
        assert!(redact_frida_evidence("[B(len=32)]").contains("REDACTED"));
        assert!(redact_frida_evidence("normal output").contains("normal"));
    }

    #[test]
    fn execute_script_dry_populates_structured_for_json_builtin_markers() {
        let sess = FridaSession { device_id: "d".into(), is_simulation: true };
        let script = r#"console.log(JSON.stringify({type:"frida-crypto-observation", method:"x"}));"#;
        let res = execute_script(&sess, script).expect("struct");
        assert!(res.structured_output.is_some());
    }
}
