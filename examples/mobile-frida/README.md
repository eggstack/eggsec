# Mobile Frida Script Library (Phase 3c)

Reusable Frida JavaScript components and examples for `eggsec mobile dynamic ... --frida-script`.

All under the single `mobile-dynamic` feature. Dry-run safe. Real runs require `--allow-frida` (Intrusive) + frida CLI + frida-server on a lab device you control.

## Usage

- **Builtins** (no files needed): `--frida-script "builtin:crypto-keystore"` (also: bypass-validation, api-trace, basic-method-trace).
- **Library snippets** (embedded): `--frida-script "library:common-hooks"` (or compose in your own script).
- **Your own scripts**: point at any .js file. You can copy/paste or `require`-style patterns from the library files below (or inline the hooks).

The `eggsec` binary supports the "library:NAME" and "builtin:NAME" conventions directly in the --frida-script value (resolved at runtime with no external FS read for "library:").

For complex sessions, repeat `--frida-script` (or pass multiple) to run sequentially in one dynamic session; results accumulate in `frida_instrumentation.script_results[]` and `structured_results[]`.

## Library Files

- `common-hooks.js` — safe, redacted-by-default hooks for crypto, network, bypass surfaces, and common secret patterns. Emits structured JSON `type:"frida-..."` lines for easy parsing + correlation.
- `sample-crypto-trace.js` — example user script that uses common hooks + extra Cipher tracing.
- `sample-api-bypass.js` — example combining api-trace + bypass-validation observations.

## Writing Your Own

Start from the library pieces, or write minimal targeted hooks. Prefer `JSON.stringify({type:"frida-...", ...})` for output so `structured_output` and correlation work.

Example minimal:
```js
Java.perform(function() {
  var Cipher = Java.use("javax.crypto.Cipher");
  Cipher.doFinal.overload("[B").implementation = function(b) {
    var ts = Date.now();
    console.log(JSON.stringify({type:"frida-crypto-observation", method:"Cipher.doFinal", pkg:"com.your.app", ts:ts}));
    return this.doFinal(b);
  };
});
```

Redaction happens server-side for evidence in reports (secrets like api_key, sk_live_, tokens, byte[] len=...).

## Correlation & Regression

Frida output (structured + findings) feeds:
- `correlate_findings` (static ↔ dynamic ↔ Frida + traffic)
- Baseline/regression when you supply `--baseline old.json` (or capture one and compare later)

See docs/MOBILE.md "Phase 3c" and architecture/mobile.md for details.

## Safety

- Dry-run (`--dry-run`) always safe, produces complete reports + bridges + bundles.
- Real: lab only; explicit `--allow-frida`; best-effort cleanup.
- Standalone defense-lab surface (no MCP/agent/TUI/pipeline integration in this release).

## References

- Plan: plans/mobile-dynamic-phase3-frida-expansion-plan.md (Phase 3c section)
- Implementation: crates/eggsec/src/mobile/frida.rs (library consts + compose + run_builtin), dynamic.rs (multi-script, baseline, bundle, correlate)
- Smoke: scripts/test-mobile-dynamic.sh (Phase 3c leg)
- Docs: docs/MOBILE.md, architecture/mobile.md
