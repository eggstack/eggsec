#!/usr/bin/env python3
"""Browser route and storage audit.

Demonstrates browser session with route discovery, cookie inspection,
and localStorage enumeration.

Requirements:
    - eggsec with headless-browser feature
    - Chromium or compatible browser runtime

Usage:
    python3 docs/python/examples/browser_route_storage_audit.py [url]
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
        collect_cookies=True,
        collect_storage=True,
    )

    with BrowserSession(config) as session:
        session.start()
        nav = session.navigate(target_url)
        print(f"Navigated: {nav.final_url}")
        print(f"Status: {nav.status_code}")

        dom = session.get_dom_snapshot()
        print(f"Forms found: {len(dom.forms)}")
        for form in dom.forms:
            print(f"  Form action={form.action} method={form.method}")
            for field in form.fields:
                print(f"    Field: {field.name} type={field.field_type}")

        print(f"Links found: {len(dom.links)}")

        cookies = session.get_cookies()
        print(f"Cookies: {len(cookies.cookies)}")
        for cookie in cookies.cookies:
            print(f"  {cookie.name} domain={cookie.domain} httpOnly={cookie.http_only}")

        storage = session.get_local_storage()
        print(f"localStorage items: {len(storage)}")

        screenshot = session.take_screenshot()
        print(f"Screenshot: {screenshot}")

        session.stop()


if __name__ == "__main__":
    main()
