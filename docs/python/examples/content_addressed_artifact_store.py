#!/usr/bin/env python3
"""Content-addressed artifact store.

Demonstrates storing, retrieving, and verifying artifacts using
content-addressed storage with deduplication and integrity checks.

Requirements:
    - eggsec (default features)

Usage:
    python3 docs/python/examples/content_addressed_artifact_store.py
"""

import tempfile
import os

from eggsec import ContentAddressedArtifactStore


def main():
    store_dir = os.path.join(tempfile.gettempdir(), "demo-artifacts")

    with ContentAddressedArtifactStore(store_dir) as store:
        store.initialize()

        data1 = b"Hello, security assessment!"
        info1 = store.put(data1, "text/plain", metadata_json='{"source":"demo"}')
        print(f"Stored: hash={info1.content_hash[:16]}... size={info1.size_bytes}")

        data2 = b"Another artifact for testing"
        info2 = store.put(data2, "application/octet-stream")
        print(f"Stored: hash={info2.content_hash[:16]}... size={info2.size_bytes}")

        info3 = store.put(data1, "text/plain")
        print(f"Dedup: same hash={info1.content_hash == info3.content_hash}")

        retrieved = store.get(info1.content_hash)
        if retrieved:
            print(f"Retrieved: {bytes(retrieved.data).decode()}")

        integrity = store.verify(info1.content_hash)
        print(f"Integrity: valid={integrity.valid}")

        total = store.total_size_bytes()
        print(f"Total store size: {total} bytes")

    print("Store directory cleaned up")


if __name__ == "__main__":
    main()
