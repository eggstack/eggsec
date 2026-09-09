#!/usr/bin/env python3
"""make_test_apk.py — generate a minimal deterministic test APK from source.

Creates a small ZIP with an AndroidManifest.xml and placeholder entries only.
No third-party fetch, no real code, no secrets. Safe for lab use and CI
fixtures. The APK is intentionally minimal: manifest/package discovery,
static analysis, and ADB push/install lifecycle only.

Usage:
    python3 scripts/make_test_apk.py --out /tmp/eggsec-test.apk
    python3 scripts/make_test_apk.py --out /tmp/eggsec-test.apk --package com.example.vuln.test
"""
from __future__ import annotations

import argparse
import hashlib
import zipfile
from pathlib import Path

MANIFEST_TEMPLATE = """<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="{package}" android:versionCode="1" android:versionName="1.0">
  <uses-permission android:name="android.permission.INTERNET"/>
  <application android:label="EggsecFixture" android:debuggable="true">
    <activity android:name=".MainActivity">
      <intent-filter>
        <action android:name="android.intent.action.MAIN"/>
        <category android:name="android.intent.category.LAUNCHER"/>
      </intent-filter>
    </activity>
  </application>
</manifest>
"""


def build_apk(out: Path, package: str) -> None:
    manifest = MANIFEST_TEMPLATE.format(package=package)
    # Fixed timestamps keep the artifact deterministic.
    date = (2026, 1, 1, 0, 0, 0)
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as zf:
        for name, data in [
            ("AndroidManifest.xml", manifest.encode()),
            ("classes.dex", b"dex-placeholder-for-fixture-only"),
            ("resources.arsc", b"arsc-placeholder"),
        ]:
            info = zipfile.ZipInfo(name, date_time=date)
            info.compress_type = zipfile.ZIP_DEFLATED
            zf.writestr(info, data)
    digest = hashlib.sha256(out.read_bytes()).hexdigest()
    print(f"wrote {out} ({out.stat().st_size} bytes) sha256={digest}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate a minimal fixture APK.")
    parser.add_argument("--out", required=True, help="Output .apk path")
    parser.add_argument("--package", default="com.example.vuln.test")
    args = parser.parse_args()
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    build_apk(out, args.package)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
