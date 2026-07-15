# Mobile Static/Dynamic Convergence

This guide covers bridging static APK/IPA analysis with dynamic session testing.

## Static Analysis Summary

After running `analyze_apk` or `analyze_ipa`, you can generate a `StaticAnalysisSummary` that captures the key findings:

```python
from eggsec import StaticAnalysisSummary

summary = StaticAnalysisSummary(
    package_id="com.example.app",
    package_name="Example App",
    version="1.2.3",
    min_sdk=21,
    target_sdk=33,
    permissions=["android.permission.INTERNET", "android.permission.CAMERA"],
    urls=["https://api.example.com", "https://cdn.example.com"],
    certificates=["sha256/ABC123..."],
    exported_components=["com.example.app.MainActivity"],
    activities=["com.example.app.MainActivity", "com.example.app.SettingsActivity"],
    services=["com.example.app.SyncService"],
    deep_links=["example://open?id=123"],
    native_libraries=["libnative.so"],
    hardcoded_secrets=["api_key=sk-12345"],
)
```

## Generating Dynamic Analysis Plans

The `StaticAnalysisSummary` can automatically generate a `DynamicAnalysisPlan`:

```python
plan = summary.to_dynamic_plan()

print(f"Package: {plan.package_id}")
print(f"Targets: {len(plan.targets)}")
print(f"Frida needed: {plan.use_frida}")
print(f"Instrumentation: {plan.instrumentation_focus}")

for target in plan.targets:
    print(f"  [{target.priority}] {target.target_type}: {target.identifier}")
```

The plan identifies:
- URLs to probe (network testing)
- Exported components (IPC testing)
- Deep links (navigation testing)
- Hardcoded secrets (secret detection)
- Whether Frida is needed (native library presence)

## Instrumentation Boundary

For dynamic analysis with instrumentation:

```python
from eggsec import InstrumentationConfig, InstrumentationScript

config = InstrumentationConfig(
    session_id="dyn-1",
    device_serial="emulator-5554",
    package_id="com.example.app",
    timeout_secs=300,
    max_output_bytes=10 * 1024 * 1024,  # 10MB
    allow_system_hooks=False,
    scripts=[
        InstrumentationScript(
            name="ssl-pinning-bypass",
            source_hash="abc123...",
            script_type="frida",
            built_in=True,
            description="Bypass SSL certificate pinning",
            target_classes=["javax.net.ssl.SSLContext"],
        ),
    ],
    hooks=["android.widget.TextView.setText"],
)
```

## Evidence Collection

Dynamic analysis produces structured evidence:

```python
from eggsec import MobileEvidence, MobileEvidenceKind

evidence = MobileEvidence(
    evidence_id="ev-1",
    session_id="dyn-1",
    kind=MobileEvidenceKind.NetworkTrace,
    device_serial="emulator-5554",
    package_id="com.example.app",
    timestamp_ms=1234567890,
    content_type="application/octet-stream",
    content_hash="sha256/...",
    size_bytes=1024,
    description="Captured HTTPS traffic during login flow",
    linked_static_evidence="static-perm-internet",
)
```

## Complete Workflow

```python
from eggsec import (
    StaticAnalysisSummary, MobileSession, MobileSessionConfig,
    MobileEvidenceCollection, MobileEvidenceKind,
)

# 1. Static analysis produces summary
summary = StaticAnalysisSummary(
    package_id="com.example.app",
    permissions=["android.permission.INTERNET"],
    urls=["https://api.example.com"],
    exported_components=["com.example.app.MainActivity"],
)

# 2. Generate dynamic plan
plan = summary.to_dynamic_plan()

# 3. Create session with static-informed config
config = MobileSessionConfig(
    device_serial="emulator-5554",
    package_id=summary.package_id,
    capture_logs=True,
    capture_network=True,
    allow_frida=plan.use_frida,
)

# 4. Run dynamic session
with MobileSession("dyn-1", "emulator-5554", config) as session:
    session.start()
    session.launch_app()
    # Dynamic findings link back to static evidence
    # via linked_static_evidence field
    session.stop()
```
