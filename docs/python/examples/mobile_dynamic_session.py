#!/usr/bin/env python3
"""Dynamic mobile session with logs and screenshots.

Demonstrates mobile device discovery, session lifecycle, app launch,
log capture, and screenshot collection.

Requirements:
    - eggsec with mobile feature
    - ADB-compatible device or emulator

Usage:
    python3 docs/python/examples/mobile_dynamic_session.py [device_serial]
"""

import sys

import eggsec
from eggsec import (
    MobileDeviceRegistry,
    MobileSessionConfig,
    MobileSession,
)


def main():
    device_serial = sys.argv[1] if len(sys.argv) > 1 else None

    features = eggsec.features()
    if not features.get("mobile", False):
        print("Error: 'mobile' feature not compiled.")
        print("Build with: maturin develop --features mobile")
        sys.exit(1)

    registry = MobileDeviceRegistry()
    devices = registry.refresh()
    if not devices:
        print("No mobile devices found")
        sys.exit(1)

    device = devices[0] if device_serial is None else next(
        (d for d in devices if d.serial == device_serial), devices[0]
    )
    print(f"Using device: {device.serial} ({device.platform})")

    config = MobileSessionConfig(
        device_serial=device.serial,
        capture_logs=True,
        capture_screenshots=True,
    )

    with MobileSession(f"demo-{device.serial}", device.serial, config) as session:
        session.start()
        print(f"Session state: {session.state}")

        session.launch_app()
        print("App launched")

        screenshot = session.capture_screenshot()
        print(f"Screenshot captured: {screenshot}")

        try:
            logs = session.get_logs()
            print(f"Log entries: {len(logs)}")
        except eggsec.ScanError as e:
            print(f"Log capture unavailable: {e}")

        stats = session.stats
        print(f"Stats: screenshots={stats.screenshots_captured}, logs={stats.log_entries}")

        session.stop()
        print(f"Session closed: {session.state}")


if __name__ == "__main__":
    main()
