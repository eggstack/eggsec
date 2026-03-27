"""
Slapper Python Plugin Example

# Name: example_scanner
# Version: 1.0.0
# Description: Example security scanner plugin
# Author: Slapper Team

This example demonstrates how to create custom security scanning plugins
for Slapper using Python.

To use this plugin:
1. Enable Python plugin support: cargo build --features python-plugins
2. Copy this file to ~/.config/slapper/plugins/
3. Run: ./slapper plugin run example_scanner https://example.com
"""

import json


def register_checks():
    """Register available security checks provided by this plugin."""
    return [
        {
            "name": "security_headers",
            "type": "scan",
            "description": "Check for missing security headers",
        },
        {
            "name": "exposed_endpoints",
            "type": "scan",
            "description": "Check for commonly exposed sensitive endpoints",
        },
    ]


def run_check(check_name, target):
    """Execute a named check against the target.

    Args:
        check_name: The name of the check to run (from register_checks)
        target: The target URL or hostname

    Returns:
        List of JSON-encoded finding strings
    """
    if check_name == "security_headers":
        return _check_security_headers(target)
    elif check_name == "exposed_endpoints":
        return _check_exposed_endpoints(target)
    return []


def _check_security_headers(target):
    """Check for missing security headers."""
    findings = []

    required_headers = [
        ("X-Frame-Options", "medium"),
        ("X-Content-Type-Options", "medium"),
        ("Strict-Transport-Security", "high"),
        ("Content-Security-Policy", "high"),
    ]

    # In a real plugin, make HTTP requests and check headers:
    # import urllib.request
    # req = urllib.request.urlopen(target)
    # for header, severity in required_headers:
    #     if header not in req.headers:
    #         findings.append(json.dumps({...}))

    return findings


def _check_exposed_endpoints(target):
    """Check for commonly exposed sensitive endpoints."""
    findings = []

    sensitive_paths = [
        "/.env",
        "/.git/config",
        "/config.php",
        "/backup.sql",
        "/admin",
        "/phpinfo.php",
    ]

    # In a real plugin, check each path:
    # import urllib.request
    # for path in sensitive_paths:
    #     try:
    #         req = urllib.request.urlopen(f"{target}{path}")
    #         if req.status == 200:
    #             findings.append(json.dumps({
    #                 "title": f"Exposed endpoint: {path}",
    #                 "severity": "high",
    #                 "description": f"Sensitive path {path} is accessible",
    #                 "location": f"{target}{path}",
    #             }))
    #     except Exception:
    #         pass

    return findings
