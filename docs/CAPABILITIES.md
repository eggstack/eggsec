# Slapper Capabilities Reference

Comprehensive reference of all security testing capabilities available in Slapper.

## Table of Contents

- [Reconnaissance Modules](#reconnaissance-modules)
- [Fuzzing Payload Types](#fuzzing-payload-types)
- [Detection Modules](#detection-modules)
- [Stress Testing Types](#stress-testing-types)
- [Protocol Implementations](#protocol-implementations)
- [Scan Profiles](#scan-profiles)
- [CLI Commands Quick Reference](#cli-commands-quick-reference)

---

## Reconnaissance Modules

Slapper includes 18 reconnaissance modules for comprehensive target intelligence gathering:

| Module | File | Description |
|--------|------|-------------|
| **Technology Detection** | `src/recon/techdetect.rs` | Identifies web servers (Nginx, Apache, IIS, LiteSpeed, OpenResty, Caddy, Traefik), frameworks (Express, Django, Rails, Laravel, Spring, ASP.NET, CakePHP, CodeIgniter, Symfony, Flask, FastAPI, Next.js, Nuxt.js, Gatsby, Hugo, Jekyll), CMS (WordPress, Drupal, Joomla, Magento, Shopify), CDNs (Cloudflare, Akamai, Fastly, CloudFront, BunnyCDN, KeyCDN), databases (MySQL, PostgreSQL, MongoDB, Redis, Elasticsearch, Memcached), JavaScript libraries (React, Vue.js, Angular, jQuery, Prototype, Dojo, Backbone, Lodash), and programming languages (PHP, Ruby, Python, Node.js, Java, C#, Go, Rust) |
| **Subdomain Enumeration** | `src/recon/subdomain.rs` | Discovers subdomains via crt.sh, Threatminer API, DNS resolution, and bruteforce wordlist scanning |
| **DNS Records** | `src/recon/dns_records.rs` | Retrieves A, AAAA, CNAME, MX, TXT, NS, SOA, and CAA records |
| **SSL/TLS Analysis** | `src/recon/ssl.rs` | Analyzes certificates, supported TLS versions, cipher suites, checks for vulnerabilities (expired certs, weak signatures, deprecated protocols like SSLv3, TLSv1.0/1.1) |
| **Reverse DNS** | `src/recon/reverse_dns.rs` | Resolves IP addresses to hostnames |
| **Geolocation** | `src/recon/geolocation.rs` | Identifies geographic location of IP addresses (country, city, ISP, coordinates) using IPAPI and MaxMind databases |
| **WHOIS Lookup** | `src/recon/whois.rs` | Retrieves domain registration info (registrar, creation/expiration dates, nameservers, registrant info) with TLD-specific server routing |
| **ASN Lookup** | `src/recon/asn.rs` | Retrieves Autonomous System Number info (ASN, prefix, organization details, abuse contacts) via ARIN RDAP |
| **CVE Mapping** | `src/recon/cve.rs` | Maps detected technologies to known CVEs with severity ratings (CRITICAL, HIGH, MEDIUM), queries NVD API with optional API key |
| **CORS Analysis** | `src/recon/cors.rs` | Tests for CORS misconfigurations including wildcard with credentials, null origin reflection, arbitrary origin acceptance |
| **Cloud Asset Discovery** | `src/recon/cloud.rs` | Enumerates AWS S3 buckets, Azure Blob Storage, GCP Storage, Firebase projects, Heroku apps, and GitHub repositories |
| **Sensitive Content** | `src/recon/content.rs` | Scans for 100+ sensitive paths including .env files, Git config, credentials, backups, admin panels, API endpoints, database dumps, logs |
| **JavaScript Analysis** | `src/recon/js.rs` | Extracts JavaScript files, finds endpoints, secrets (API keys, passwords, tokens, JWTs), and URLs from JS files |
| **Wayback Machine** | `src/recon/wayback.rs` | Retrieves historical snapshots, discovers old endpoints/paths from archive.org |
| **Contact Discovery** | `src/recon/email.rs` | Extracts emails, phone numbers, social media handles (Facebook, Twitter/X, Instagram, LinkedIn, GitHub, YouTube, TikTok), and physical addresses |
| **Threat Intelligence** | `src/recon/threatintel.rs` | Checks VirusTotal, Shodan, and AlienVault OTX for IP/domain reputation, vulnerabilities, and passive DNS |
| **DNS Enhanced** | `src/recon/dns_enhanced.rs` | Advanced DNS enumeration with additional record types and resolution techniques |
| **CVE Lookup** | `src/recon/cve_lookup.rs` | Enhanced CVE lookup with detailed vulnerability information |

---

## Fuzzing Payload Types

Slapper supports 24 security fuzzing payload types:

| Type | Alias | File | Tests For |
|------|-------|------|-----------|
| **SQL Injection** | sqli, sql | `src/fuzzer/payloads/sqli.rs` | SQL Injection - 100+ payloads including error-based, UNION-based, time-based (blind), stacked queries, WAF bypasses, encoded variants, DB-specific (MySQL, PostgreSQL, SQL Server, Oracle) |
| **Cross-Site Scripting** | xss | `src/fuzzer/payloads/xss.rs` | XSS - 100+ payloads including basic script tags, event handlers (onerror, onload, onfocus), encoded variants, WAF bypasses, polyglots, template injection |
| **Path Traversal** | traversal, lfi, path | `src/fuzzer/payloads/traversal.rs` | Path Traversal / Local File Inclusion |
| **Server-Side Request Forgery** | ssrf | `src/fuzzer/payloads/ssrf.rs` | SSRF - payloads for internal service access, cloud metadata endpoints |
| **Open Redirect** | redirect, open-redirect | `src/fuzzer/payloads/redirect.rs` | Open Redirect - various redirect bypass techniques |
| **Regular Expression DoS** | redos, regex | `src/fuzzer/payloads/redos.rs` | ReDoS - ReDoS pattern payloads for regex engine exhaustion |
| **HTTP Header Injection** | headers | `src/fuzzer/payloads/headers.rs` | HTTP Header Injection - header manipulation payloads |
| **Compression Bomb** | compression, gzip | `src/fuzzer/payloads/compression.rs` | Compression Bomb - ZIP/gzip bombs for DoS via decompression |
| **GraphQL** | graphql | `src/fuzzer/payloads/graphql.rs` | GraphQL-specific - introspection, query injection, depth limit bypass, alias overload |
| **OAuth/OIDC** | oauth | `src/fuzzer/payloads/oauth.rs` | OAuth/OIDC Testing - redirect URI bypass, scope escalation, state parameter bypass, grant type mixing |
| **JWT** | jwt | `src/fuzzer/payloads/jwt.rs` | JWT Testing - algorithm confusion, weak secrets, null signature bypass |
| **IDOR** | idor | `src/fuzzer/payloads/idor.rs` | Insecure Direct Object Reference - ID enumeration payloads |
| **Server-Side Template Injection** | ssti | `src/fuzzer/payloads/ssti.rs` | SSTI - Jinja2, Twig, ERB, FreeMarker payloads |
| **gRPC** | grpc | `src/fuzzer/payloads/grpc.rs` | gRPC Fuzzing - protobuf manipulation and gRPC-specific attacks |
| **XML External Entity** | xxe | `src/fuzzer/payloads/xxe.rs` | XXE injection payloads |
| **LDAP Injection** | ldap | `src/fuzzer/payloads/ldap.rs` | LDAP-specific payloads |
| **Command Injection** | cmd | `src/fuzzer/payloads/cmd.rs` | OS command execution payloads |
| **Deserialization** | deser | `src/fuzzer/payloads/deser.rs` | Deserialization vulnerabilities |
| **Host Header Injection** | host | `src/fuzzer/payloads/host.rs` | Host manipulation payloads |
| **Cache Poisoning** | cache | `src/fuzzer/payloads/cache.rs` | HTTP cache manipulation payloads |
| **CSV Injection** | csv | `src/fuzzer/payloads/csv.rs` | Formula injection payloads |
| **SOAP Injection** | soap | `src/fuzzer/payloads/soap.rs` | SOAP/XML Injection |
| **WebSocket** | websocket | `src/fuzzer/payloads/websocket.rs` | WebSocket Fuzzing |
| **CVE Lookup** | cve | `src/recon/cve_lookup.rs` | CVE vulnerability testing based on detected technologies |

---

## Detection Modules

Advanced vulnerability detection capabilities:

| Module | File | Description |
|--------|------|-------------|
| **Timing Analyzer** | `src/fuzzer/detection/analyzer.rs` | Detects vulnerabilities based on response time differences (blind injection detection) |
| **Pattern Matcher** | `src/fuzzer/detection/patterns.rs` | Signature-based vulnerability detection using regex patterns |
| **Aho-Corasick** | `src/fuzzer/detection/aho_corasick.rs` | High-performance multi-pattern matching for bulk vulnerability detection |
| **ReDoS Detector** | `src/fuzzer/redos_detect.rs` | Executes regexes to identify catastrophic backtracking vulnerabilities |
| **WAF Fingerprinter** | `src/fuzzer/waf_fingerprint.rs` | Identifies specific WAF products and versions |
| **Diff Analyzer** | `src/fuzzer/diff.rs` | Compares responses to detect anomalies |
| **Rate Limit Detector** | `src/fuzzer/rate_limit.rs` | Detects and analyzes rate limiting behavior |

---

## Stress Testing Types

| Type | File | Description |
|------|------|-------------|
| **HTTP Flood** | `src/stress/http.rs` | Sends high-volume HTTP requests with randomized User-Agent, X-Forwarded-For headers, proxy support |
| **SYN Flood** | `src/stress/syn.rs` | TCP SYN packet flooding (requires root + stress-testing feature) |
| **UDP Flood** | `src/stress/udp.rs` | UDP packet flooding (requires root + stress-testing feature) |
| **ICMP Flood** | `src/stress/icmp.rs` | Ping flood attacks (requires root + stress-testing feature) |
| **Metrics Collection** | `src/stress/metrics.rs` | Tracks packets sent, bytes sent, errors, duration |

---

## Protocol Implementations

| Protocol | File | Description |
|----------|------|-------------|
| **REST API** | `src/tool/protocol/rest.rs` | REST API server - exposes slapper tools via HTTP (requires rest-api feature) |
| **gRPC API** | `src/tool/protocol/grpc.rs` | gRPC API server - exposes slapper tools via protocol buffers (requires grpc-api feature) |
| **MCP Server** | `src/tool/protocol/mcp.rs` | MCP (Model Context Protocol) - JSON-RPC server for AI agent integration (requires mcp-server feature) |

---

## Scan Profiles

Chained security assessment pipelines:

| Profile | Stages |
|---------|--------|
| **quick** | Port scan + fingerprint |
| **endpoint** | Quick + endpoint discovery |
| **web** | Endpoint + web fuzzing (sqli, xss, ssrf, etc.) |
| **waf** | Web + WAF detection and bypass testing |
| **full** | All stages including load testing |
| **api** | GraphQL/JWT/OAuth focused |
| **recon** | Intelligence-led with tech detection and CVE mapping |
| **stealth** | Web scan with evasion techniques (randomized delays, header rotation) |
| **deep** | Web scan with mutation fuzzing enabled |
| **vuln** | CVE-prioritized fuzzing based on detected technologies |
| **auth** | JWT/OAuth/IDOR security testing |

---

## CLI Commands Quick Reference

### Core Scanning Commands

| Command | Description |
|---------|-------------|
| `slapper load <url>` | HTTP load testing |
| `slapper scan-ports <target>` | Port scanning |
| `slapper scan-endpoints <url>` | Endpoint discovery |
| `slapper fingerprint <target>` | Service fingerprinting |
| `slapper fuzz <url> -t <type>` | Security fuzzing |
| `slapper waf <url>` | WAF detection |
| `slapper waf-stress <url>` | WAF stress testing |
| `slapper scan <target> --profile <profile>` | Pipeline scan |
| `slapper recon <target>` | Reconnaissance |

### API Security Commands

| Command | Description |
|---------|-------------|
| `slapper graphql <url>` | GraphQL security testing |
| `slapper oauth <url>` | OAuth/OIDC testing |

### Network Tools

| Command | Description |
|---------|-------------|
| `slapper packet capture` | Live packet capture |
| `slapper packet send` | Craft and send packets |
| `slapper packet dump` | Analyze pcap files |
| `slapper packet traceroute` | Network path tracing |
| `slapper icmp <target>` | ICMP ping probes |
| `slapper traceroute <target>` | Network path tracing |

### Stress Testing (requires stress-testing feature)

| Command | Description |
|---------|-------------|
| `slapper stress <target> --type <type>` | SYN/UDP/HTTP/TCP/ICMP flood |
| `slapper proxy <action>` | Proxy pool management |

### Management & Integration

| Command | Description |
|---------|-------------|
| `slapper cluster coordinator` | Start cluster coordinator |
| `slapper cluster worker` | Start cluster worker |
| `slapper remote --port` | Start remote listener |
| `slapper exec --target` | Execute remote commands |
| `slapper notify` | Webhook notifications |
| `slapper report` | Generate reports |
| `slapper resume <file>` | Resume previous scan |

### API Servers

| Command | Description |
|---------|-------------|
| `slapper serve --port` | REST API server |
| `slapper grpc-serve --port` | gRPC API server |
| `slapper mcp-serve --port` | MCP server for AI agents |

### NSE Scripting

| Command | Description |
|---------|-------------|
| `slapper nse <target>` | Run NSE scripts |

---

## Build Features

| Feature | Description |
|---------|-------------|
| `default` | Core: load testing, scanning, fuzzing, WAF testing |
| `stress-testing` | SYN/UDP/HTTP/TCP/ICMP floods, proxy management |
| `packet-inspection` | Live packet capture and crafting |
| `nse` | Nmap Scripting Engine support |
| `rest-api` | REST API server |
| `grpc-api` | gRPC API server |
| `mcp-server` | MCP server for AI integration |
| `full` | All features combined |
