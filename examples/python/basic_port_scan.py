"""Basic port scan example using eggsec Python bindings."""

import eggsec


def main():
    # Create a scope allowing localhost
    scope = eggsec.Scope.allow_hosts(["127.0.0.1", "localhost"])

    # Scan common ports on localhost
    result = eggsec.scan_ports(
        target="127.0.0.1",
        ports=[22, 80, 443, 8080],
        scope=scope,
        timeout_ms=2000,
    )

    print(f"Target: {result.target}")
    print(f"Scanned: {result.scanned_ports} ports")
    print(f"Open: {result.stats.total_open}")
    print(f"Elapsed: {result.elapsed_ms}ms")

    for port in result.open_ports:
        print(f"  {port}")


if __name__ == "__main__":
    main()
