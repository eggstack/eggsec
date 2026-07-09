"""Async scanning example using eggsec Python bindings."""

import eggsec


def main():
    scope = eggsec.Scope.allow_hosts(["127.0.0.1", "localhost"])

    # Async convenience function
    print("Starting async port scan...")
    future = eggsec.async_scan_ports(
        "127.0.0.1",
        [22, 80, 443],
        scope,
        timeout_ms=2000,
    )

    # Poll until complete
    for result in future:
        if result is not None:
            print(f"Scan complete: {result.stats.total_open} open ports")
            for port in result.open_ports:
                print(f"  {port}")

    # AsyncClient with context manager
    print("\nUsing AsyncClient...")

    async def async_scan():
        async with eggsec.AsyncClient(scope) as client:
            future = client.scan_ports("127.0.0.1", [80, 443])
            result = await future
            print(f"Async scan: {result.stats.total_open} open ports")

    import asyncio
    asyncio.run(async_scan())


if __name__ == "__main__":
    main()
