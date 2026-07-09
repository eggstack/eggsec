"""Port scan with JSON output using eggsec Python bindings."""

import json

import eggsec


def main():
    # Create a client for repeated scans
    scope = eggsec.Scope.allow_hosts(["127.0.0.1"])
    client = eggsec.Client(scope, mode="manual", timeout_ms=2000)

    # Scan using the client API
    result = client.scan_ports("127.0.0.1", [22, 80, 443])

    # Get JSON output
    json_str = result.to_json()
    data = json.loads(json_str)
    print(json.dumps(data, indent=2))

    # Or convert to dict
    d = result.to_dict()
    print(f"\nDict keys: {list(d.keys())}")


if __name__ == "__main__":
    main()
