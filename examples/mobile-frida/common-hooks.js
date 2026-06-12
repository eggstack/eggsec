// common-hooks.js — Phase 3c reusable Frida components (safe, redacted, structured JSON output)
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
