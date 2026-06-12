// sample-crypto-trace.js — Phase 3c example user script (uses patterns from common-hooks)
// Run with: eggsec mobile dynamic ... --frida-script sample-crypto-trace.js --dry-run (or real with --allow-frida)

Java.perform(function() {
  var pkg = "com.example.target";

  // Pull in the common crypto + keystore hooks (in practice, copy the blocks or use library:common-hooks)
  // For demo we duplicate the minimal Cipher hook with extra logging.

  try {
    var Cipher = Java.use("javax.crypto.Cipher");
    Cipher.doFinal.overload("[B").implementation = function(b) {
      var ts = Date.now();
      console.log(JSON.stringify({type:"frida-crypto-observation", method:"Cipher.doFinal", pkg:pkg, args_redacted:"[REDACTED]", ret_redacted:"[REDACTED]", ts:ts}));
      // user can add custom logic here
      return this.doFinal(b);
    };
  } catch (e) {}

  try {
    var KS = Java.use("java.security.KeyStore");
    KS.getKey.overload("java.lang.String", "[C").implementation = function(alias, pw) {
      var ts = Date.now();
      console.log(JSON.stringify({type:"frida-crypto-observation", method:"KeyStore.getKey", pkg:pkg, alias:"[REDACTED]", ts:ts}));
      return this.getKey(alias, pw);
    };
  } catch (e) {}
});
