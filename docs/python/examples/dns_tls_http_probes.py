#!/usr/bin/env python3
"""DNS, TLS, and HTTP one-shot probes.

Exercises the standalone probe functions (`dns_query`, `tls_probe`,
`http_probe`) against a public domain. Demonstrates sync and async
variants and shows how to interpret probe results.

Requirements:
    - eggsec (default features)
    - Network access to the target

Usage:
    python3 docs/python/examples/dns_tls_http_probes.py
"""

import asyncio
import sys

import eggsec
from eggsec import (
    dns_query,
    tls_probe,
    http_probe,
    async_dns_query,
    async_tls_probe,
    async_http_probe,
)

TARGET = sys.argv[1] if len(sys.argv) > 1 else "example.com"


def run_sync():
    print(f"=== Sync probes against {TARGET} ===\n")

    # DNS query — A records
    dns = dns_query(TARGET, record_types=["A"], timeout_ms=5000)
    print(f"DNS: response_code={dns.response_code}, authoritative={dns.authoritative}")
    for rec in dns.records:
        print(f"  {rec.record_type} {rec.name} -> {rec.data} (TTL {rec.ttl})")
    print(f"  resolver={dns.resolver_used}, elapsed={dns.timing.elapsed_ms:.0f}ms")
    if dns.error:
        print(f"  error: {dns.error}")

    # TLS probe — certificate inspection
    tls = tls_probe(TARGET, port=443, timeout_ms=10000)
    print(f"\nTLS: has_tls={tls.has_tls}, version={tls.tls_version}")
    if tls.certificate:
        cert = tls.certificate
        print(f"  subject={cert.subject}")
        print(f"  issuer={cert.issuer}")
        print(f"  valid_until={cert.valid_until}")
        print(f"  expired={cert.is_expired}, days_left={cert.days_until_expiry}")
        if cert.subject_alternative_names:
            print(f"  SANs: {cert.subject_alternative_names[:5]}")
    if tls.issues:
        for issue in tls.issues:
            print(f"  [{issue.severity}] {issue.code}: {issue.description}")
    if tls.error:
        print(f"  error: {tls.error}")

    # HTTP probe — GET request
    http = http_probe(f"https://{TARGET}/", timeout_ms=10000, follow_redirects=True)
    print(f"\nHTTP: status={http.status_code}, url={http.url}")
    if http.final_url and http.final_url != http.url:
        print(f"  redirected_to={http.final_url}, redirect_count={http.redirect_count}")
    content_type = next((v for k, v in http.headers if k.lower() == "content-type"), None)
    if content_type:
        print(f"  content_type={content_type}")
    print(f"  body_bytes={http.body_bytes.__len__() if http.body_bytes else 0}")
    print(f"  elapsed={http.timing.elapsed_ms:.0f}ms")
    if http.error:
        print(f"  error: {http.error}")


async def run_async():
    print(f"\n=== Async probes against {TARGET} ===\n")

    dns = await async_dns_query(TARGET, record_types=["A", "AAAA"], timeout_ms=5000)
    print(f"DNS: {dns.response_code} ({len(dns.records)} records)")

    tls = await async_tls_probe(TARGET, port=443, timeout_ms=10000)
    print(f"TLS: has_tls={tls.has_tls}, version={tls.tls_version}")

    http = await async_http_probe(f"https://{TARGET}/", timeout_ms=10000)
    print(f"HTTP: status={http.status_code}")


def main():
    run_sync()
    asyncio.run(run_async())


if __name__ == "__main__":
    main()
