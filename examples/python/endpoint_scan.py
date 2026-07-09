"""Endpoint discovery example using eggsec Python bindings."""

import eggsec


def main():
    scope = eggsec.Scope.allow_hosts(["127.0.0.1", "localhost"])
    client = eggsec.Client(scope)

    config = eggsec.EndpointScanConfig(
        base_url="http://127.0.0.1",
        endpoints=[
            "/",
            "/admin",
            "/login",
            "/api",
            "/robots.txt",
            "/.env",
            "/backup",
            "/config",
            "/debug",
            "/health",
        ],
        timeout_ms=3000,
    )

    result = client.scan_endpoints(config)

    print(f"Target: {result.target}")
    print(f"Scanned: {result.scanned} endpoints")
    print(f"Found: {result.found}")
    print(f"Interesting: {result.interesting}")
    print(f"Elapsed: {result.elapsed_ms}ms")
    print()

    for finding in result.findings:
        marker = " [INTERESTING]" if finding.interesting else ""
        length = f" ({finding.content_length} bytes)" if finding.content_length else ""
        print(f"  {finding.status} {finding.path}{length}{marker}")


if __name__ == "__main__":
    main()
