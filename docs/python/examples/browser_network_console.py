#!/usr/bin/env python3
"""Browser network and console capture.

Demonstrates capturing network requests and console events during
a browser security assessment.

Requirements:
    - eggsec with headless-browser feature
    - Chromium or compatible browser runtime

Usage:
    python3 docs/python/examples/browser_network_console.py [url]
"""

import sys

import eggsec
from eggsec import BrowserSession, BrowserSessionConfig


def main():
    target_url = sys.argv[1] if len(sys.argv) > 1 else "https://example.com"

    features = eggsec.features()
    if not features.get("headless-browser", False):
        print("Error: 'headless-browser' feature not compiled.")
        print("Build with: maturin develop --features headless-browser")
        sys.exit(1)

    config = BrowserSessionConfig(
        target_url=target_url,
        collect_console=True,
        collect_network=True,
    )

    with BrowserSession(config) as session:
        session.start()
        nav = session.navigate(target_url)
        print(f"Navigated: {nav.final_url}")

        network = session.get_network_events()
        print(f"Network events: {len(network)}")
        for req in network:
            print(f"  {req.method} {req.url} -> {req.status_code}")

        console = session.get_console_events()
        print(f"Console events: {len(console)}")
        for evt in console:
            print(f"  [{evt.level}] {evt.message}")

        session.stop()


if __name__ == "__main__":
    main()
