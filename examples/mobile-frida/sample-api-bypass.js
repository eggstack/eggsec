// sample-api-bypass.js — Phase 3c example combining api-trace + bypass-validation
// Demonstrates multi-category emission for correlation and regression workflows.

Java.perform(function() {
  var pkg = "com.example.target";

  try {
    var HUC = Java.use("java.net.HttpURLConnection");
    HUC.getInputStream.implementation = function() {
      var ts = Date.now();
      console.log(JSON.stringify({type:"frida-api-trace", method:"HttpURLConnection", pkg:pkg, params_inspected:{url:"[REDACTED]"}, ts:ts}));
      return this.getInputStream();
    };
  } catch (e) {}

  try {
    var Build = Java.use("android.os.Build");
    // Observe (do not actually bypass in this sample) — for detection validation
    var tags = Build.TAGS.value;
    console.log(JSON.stringify({type:"frida-bypass-validation", method:"Build.TAGS.observe", pkg:pkg, value:"[REDACTED]", ts:Date.now()}));
  } catch (e) {}
});
