# Reconnaissance Module

The Reconnaissance module focuses on passive and active information gathering about a target before performing more intrusive scanning or fuzzing.

## Core Capabilities (`src/recon/`)

### Network & Infrastructure

- **DNS (`dns_records.rs`, `dns_enhanced.rs`)**: Comprehensive DNS enumeration, including A, AAAA, MX, TXT, and CNAME records.
- **Subdomain Discovery (`subdomain.rs`)**: Finding subdomains using wordlists, search engines, and certificate transparency logs.
- **WHOIS & ASN (`whois.rs`, `asn.rs`)**: Extracting ownership and network registration information.
- **Geolocation (`geolocation.rs`)**: Identifying the physical location of target IPs.

### Web & Technology

- **Tech Detection (`techdetect.rs`)**: Identifying the software stack (CMS, frameworks, web servers) using signatures and HTTP headers.
- **Content Analysis (`content.rs`, `js.rs`)**: Analyzing page content and JavaScript files for interesting information like endpoints, API keys, or version strings.
- **Wayback Machine (`wayback.rs`)**: Retrieving historical URLs for a domain to find forgotten or retired endpoints.
- **CORS Testing (`cors.rs`)**: Identifying misconfigured Cross-Origin Resource Sharing policies.

### Vulnerability Mapping

- **CVE Lookup (`cve_lookup.rs`, `cve.rs`)**: Mapping identified software versions to known vulnerabilities (CVEs).
- **Threat Intelligence (`threatintel.rs`)**: Checking targets against known blacklists or threat intelligence feeds.

### Cloud & Containers (`cloud/`, `containers.rs`)

- **Cloud Enumeration**: Identifying if a target is hosted on AWS, GCP, or Azure and looking for common misconfigurations (e.g., open S3 buckets).
- **Container Discovery**: Detecting containerized environments (Kubernetes, Docker) and associated management interfaces.

## Recon Runner (`runner.rs`)

The `runner.rs` file orchestrates all these recon tasks, often running them in parallel to maximize efficiency while respecting rate limits.
