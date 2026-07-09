"""WAF detection example using eggsec Python bindings.

Detects whether a target is protected by a Web Application Firewall.

Note: Targets use scanme.nmap.org (Nmap's official test target).
"""

import json

import eggsec


def main():
    target = "scanme.nmap.org"
    target_url = f"http://{target}"

    scope = eggsec.Scope.allow_hosts([target])
    client = eggsec.Client(scope, mode="manual", timeout_ms=5000)

    print(f"[*] Detecting WAF for {target_url}...")
    result = client.detect_waf(target_url)

    print(f"    Detected:  {result.detected}")
    print(f"    WAF name:  {result.waf_name or 'unknown'}")
    print(f"    Confidence:{result.confidence}%")
    print(f"    Server:    {result.server_header or 'N/A'}")
    print(f"    Status:    {result.status_code}")

    if result.matched_headers:
        print(f"    Matched headers: {result.matched_headers}")
    if result.matched_cookies:
        print(f"    Matched cookies: {result.matched_cookies}")
    if result.matched_patterns:
        print(f"    Matched patterns: {result.matched_patterns}")
    if result.request_error:
        print(f"    Error: {result.request_error}")

    # Dict/JSON output
    d = result.to_dict()
    print(f"\n[*] Full result as dict:")
    print(json.dumps(d, indent=2))

    # Using the standalone function
    print(f"\n[*] Using standalone detect_waf()...")
    result2 = eggsec.detect_waf(target_url)
    print(f"    Detected: {result2.detected}, WAF: {result2.waf_name or 'unknown'}")


if __name__ == "__main__":
    main()
