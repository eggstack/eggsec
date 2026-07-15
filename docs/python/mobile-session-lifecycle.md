# Mobile Device and Session Lifecycle

This guide covers mobile device discovery, session management, and dynamic analysis workflows in eggsec-python.

## Device Discovery

```python
from eggsec import MobileDeviceRegistry

registry = MobileDeviceRegistry()
devices = registry.refresh()  # Runs `adb devices -l`

for device in devices:
    print(f"{device.serial}: {device.model} ({device.platform})")
    print(f"  Emulator: {device.is_emulator}, Rooted: {device.is_rooted}")
    print(f"  Transport: {device.transport}, Auth: {device.authorization_status}")
    print(f"  Operations: {device.supported_operations}")
```

## Session Lifecycle

### Sync Session

```python
from eggsec import MobileSession, MobileSessionConfig

config = MobileSessionConfig(
    device_serial="emulator-5554",
    package_id="com.example.app",
    install_app=True,
    capture_logs=True,
    capture_screenshots=True,
    timeout_secs=300,
)

with MobileSession("session-1", "emulator-5554", config) as session:
    session.start()
    session.install_app("/path/to/app.apk")
    session.launch_app()
    # ... perform analysis ...
    session.stop_app()
    session.stop()
```

### Async Session

```python
from eggsec import AsyncMobileSession, MobileSessionConfig

config = MobileSessionConfig(
    device_serial="emulator-5554",
    package_id="com.example.app",
)

async with AsyncMobileSession("async-session-1", "emulator-5554", config) as session:
    await session.async_start()
    await session.async_install_app("/path/to/app.apk")
    await session.async_launch_app()
    # ... perform analysis ...
    await session.async_stop()
```

## Session States

Sessions progress through these states:
`Created` -> `Connecting` -> `Installing` -> `Launching` -> `Running` -> `Capturing` -> `Stopping` -> `Stopped`

Error states: `Failed`, `Cancelled`
Cleanup states: `Uninstalling`, `Cleaning`

## Statistics

```python
stats = session.stats
print(f"Screenshots: {stats.screenshots_captured}")
print(f"Log entries: {stats.log_entries}")
print(f"Network exchanges: {stats.network_exchanges}")
print(f"Artifacts: {stats.artifacts_collected}")
print(f"Duration: {stats.duration_ms}ms")
```

## Device Capabilities

```python
registry = MobileDeviceRegistry()
registry.refresh()

device = registry.get_device("emulator-5554")
if device:
    print(f"Install: {device.supported_operations}")
```

## Error Handling

All session methods raise `ScanError` when:
- The device is not connected
- The operation is not supported
- The session is in an invalid state

```python
from eggsec import ScanError

try:
    session.install_app("/path/to.apk")
except ScanError as e:
    print(f"Install failed: {e}")
```
