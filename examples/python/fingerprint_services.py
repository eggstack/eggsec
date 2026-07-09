"""Service fingerprinting example using eggsec Python bindings."""

import eggsec


def main():
    scope = eggsec.Scope.allow_hosts(["127.0.0.1", "localhost"])
    client = eggsec.Client(scope)

    result = client.fingerprint_services(
        "127.0.0.1",
        [22, 80, 443, 3306, 5432, 8080, 8443],
        timeout_ms=3000,
    )

    print(f"Target: {result.target}")
    print(f"Scanned: {result.scanned} ports")
    print(f"Identified: {result.identified} services")
    print(f"Elapsed: {result.elapsed_ms}ms")
    print()

    for svc in result.services:
        version = f" {svc.version}" if svc.version else ""
        product = f" ({svc.product})" if svc.product else ""
        banner = f' banner="{svc.banner[:50]}"' if svc.banner else ""
        print(f"  {svc.port}/tcp — {svc.service}{product}{version} [{svc.confidence}%]{banner}")


if __name__ == "__main__":
    main()
