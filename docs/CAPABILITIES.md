# Eggsec Capabilities Reference

Comprehensive reference of all security testing capabilities available in Eggsec.

## Table of Contents

- [Reconnaissance Modules](#reconnaissance-modules)
- [Fuzzing Payload Types](#fuzzing-payload-types)
- [Detection Modules](#detection-modules)
- [Mobile App Security (Static Analysis)](#mobile-app-security-static-analysis) (dynamic under `mobile-dynamic`; Phase 4c 2026-06-12 partial: supply-chain native-load observation + correlation, regression enrichment, bundle manifest, workflow helper; TUI/MCP/pipeline absent; see docs/MOBILE.md)
- [Stress Testing Types](#stress-testing-types)
- [Protocol Implementations](#protocol-implementations)
- [Scan Profiles](#scan-profiles)
- [CLI Commands Quick Reference](#cli-commands-quick-reference)

---

## Reconnaissance Modules

Eggsec includes 21+ reconnaissance modules for comprehensive target intelligence gathering:

| Module | File | Description |
|--------|------|-------------|
| **Technology Detection** | `crates/eggsec/src/recon/techdetect.rs` | Identifies web servers (Nginx, Apache, IIS, LiteSpeed, OpenResty, Caddy, Traefik), frameworks (Express, Django, Rails, Laravel, Spring, ASP.NET, CakePHP, CodeIgniter, Symfony, Flask, FastAPI, Next.js, Nuxt.js, Gatsby, Hugo, Jekyll), CMS (WordPress, Drupal, Joomla, Magento, Shopify), CDNs (Cloudflare, Akamai, Fastly, CloudFront, BunnyCDN, KeyCDN), databases (MySQL, PostgreSQL, MongoDB, Redis, Elasticsearch, Memcached), JavaScript libraries (React, Vue.js, Angular, jQuery, Prototype, Dojo, Backbone, Lodash), and programming languages (PHP, Ruby, Python, Node.js, Java, C#, Go, Rust) |
| **Subdomain Enumeration** | `crates/eggsec/src/recon/subdomain.rs` | Discovers subdomains via crt.sh, Threatminer API, DNS resolution, and bruteforce wordlist scanning |
| **DNS Records** | `crates/eggsec/src/recon/dns_records.rs` | Retrieves A, AAAA, CNAME, MX, TXT, NS, SOA, and CAA records |
| **SSL/TLS Analysis** | `crates/eggsec/src/recon/ssl.rs` | Analyzes certificates, supported TLS versions, cipher suites, checks for vulnerabilities (expired certs, weak signatures, deprecated protocols like SSLv3, TLSv1.0/1.1) |
| **Reverse DNS** | `crates/eggsec/src/recon/reverse_dns.rs` | Resolves IP addresses to hostnames |
| **Geolocation** | `crates/eggsec/src/recon/geolocation.rs` | Identifies geographic location of IP addresses (country, city, ISP, coordinates) using IPAPI and MaxMind databases |
| **WHOIS Lookup** | `crates/eggsec/src/recon/whois.rs` | Retrieves domain registration info (registrar, creation/expiration dates, nameservers, registrant info) with TLD-specific server routing |
| **ASN Lookup** | `crates/eggsec/src/recon/asn.rs` | Retrieves Autonomous System Number info (ASN, prefix, organization details, abuse contacts) via ARIN RDAP |
| **CVE Mapping** | `crates/eggsec/src/recon/cve.rs` | Maps detected technologies to known CVEs with severity ratings (CRITICAL, HIGH, MEDIUM), queries NVD API with optional API key |
| **CORS Analysis** | `crates/eggsec/src/recon/cors.rs` | Tests for CORS misconfigurations including wildcard with credentials, null origin reflection, arbitrary origin acceptance |
| **Cloud Asset Discovery** | `crates/eggsec/src/recon/cloud.rs` | Enumerates AWS S3 buckets, Azure Blob Storage, GCP Storage, Firebase projects, Heroku apps, and GitHub repositories |
| **Sensitive Content** | `crates/eggsec/src/recon/content.rs` | Scans for 100+ sensitive paths including .env files, Git config, credentials, backups, admin panels, API endpoints, database dumps, logs |
| **JavaScript Analysis** | `crates/eggsec/src/recon/js.rs` | Extracts JavaScript files, finds endpoints, secrets (API keys, passwords, tokens, JWTs), and URLs from JS files |
| **Wayback Machine** | `crates/eggsec/src/recon/wayback.rs` | Retrieves historical snapshots, discovers old endpoints/paths from archive.org |
| **Contact Discovery** | `crates/eggsec/src/recon/email.rs` | Extracts emails, phone numbers, social media handles (Facebook, Twitter/X, Instagram, LinkedIn, GitHub, YouTube, TikTok), and physical addresses |
| **Threat Intelligence** | `crates/eggsec/src/recon/threatintel.rs` | Checks VirusTotal, Shodan, and AlienVault OTX for IP/domain reputation, vulnerabilities, and passive DNS |
| **DNS Enhanced** | `crates/eggsec/src/recon/dns_enhanced.rs` | Advanced DNS enumeration with additional record types and resolution techniques |
| **CVE Lookup** | `crates/eggsec/src/recon/cve_lookup.rs` | Enhanced CVE lookup with detailed vulnerability information |

---

## Fuzzing Payload Types

Eggsec supports 40 security fuzzing payload types:

| Type | Alias | File | Tests For |
|------|-------|------|-----------|
| **SQL Injection** | sqli, sql | `crates/eggsec/src/fuzzer/payloads/sqli.rs` | SQL Injection - 100+ payloads including error-based, UNION-based, time-based (blind), stacked queries, WAF bypasses, encoded variants, DB-specific (MySQL, PostgreSQL, SQL Server, Oracle) |
| **Cross-Site Scripting** | xss | `crates/eggsec/src/fuzzer/payloads/xss.rs` | XSS - 100+ payloads including basic script tags, event handlers (onerror, onload, onfocus), encoded variants, WAF bypasses, polyglots, template injection |
| **Path Traversal** | traversal, lfi, path | `crates/eggsec/src/fuzzer/payloads/traversal.rs` | Path Traversal / Local File Inclusion |
| **Server-Side Request Forgery** | ssrf | `crates/eggsec/src/fuzzer/payloads/ssrf.rs` | SSRF - payloads for internal service access, cloud metadata endpoints |
| **Open Redirect** | redirect, open-redirect | `crates/eggsec/src/fuzzer/payloads/redirect.rs` | Open Redirect - various redirect bypass techniques |
| **Regular Expression DoS** | redos, regex | `crates/eggsec/src/fuzzer/payloads/redos.rs` | ReDoS - ReDoS pattern payloads for regex engine exhaustion |
| **HTTP Header Injection** | headers | `crates/eggsec/src/fuzzer/payloads/headers.rs` | HTTP Header Injection - header manipulation payloads |
| **Compression Bomb** | compression, gzip | `crates/eggsec/src/fuzzer/payloads/compression.rs` | Compression Bomb - ZIP/gzip bombs for DoS via decompression |
| **GraphQL** | graphql | `crates/eggsec/src/fuzzer/payloads/graphql.rs` | GraphQL-specific - introspection, query injection, depth limit bypass, alias overload |
| **OAuth/OIDC** | oauth | `crates/eggsec/src/fuzzer/payloads/oauth.rs` | OAuth/OIDC Testing - redirect URI bypass, scope escalation, state parameter bypass, grant type mixing |
| **JWT** | jwt | `crates/eggsec/src/fuzzer/payloads/jwt.rs` | JWT Testing - algorithm confusion, weak secrets, null signature bypass |
| **IDOR** | idor | `crates/eggsec/src/fuzzer/payloads/idor.rs` | Insecure Direct Object Reference - ID enumeration payloads |
| **Server-Side Template Injection** | ssti | `crates/eggsec/src/fuzzer/payloads/ssti.rs` | SSTI - Jinja2, Twig, ERB, FreeMarker payloads |
| **gRPC** | grpc | `crates/eggsec/src/fuzzer/payloads/grpc.rs` | gRPC Fuzzing - protobuf manipulation and gRPC-specific attacks |
| **XML External Entity** | xxe | `crates/eggsec/src/fuzzer/payloads/xxe.rs` | XXE injection payloads |
| **LDAP Injection** | ldap | `crates/eggsec/src/fuzzer/payloads/ldap.rs` | LDAP-specific payloads |
| **Command Injection** | cmd | `crates/eggsec/src/fuzzer/payloads/cmd.rs` | OS command execution payloads |
| **Deserialization** | deser | `crates/eggsec/src/fuzzer/payloads/deser.rs` | Deserialization vulnerabilities |
| **Host Header Injection** | host | `crates/eggsec/src/fuzzer/payloads/host.rs` | Host manipulation payloads |
| **Cache Poisoning** | cache | `crates/eggsec/src/fuzzer/payloads/cache.rs` | HTTP cache manipulation payloads |
| **CSV Injection** | csv | `crates/eggsec/src/fuzzer/payloads/csv.rs` | Formula injection payloads |
| **SOAP Injection** | soap | `crates/eggsec/src/fuzzer/payloads/soap.rs` | SOAP/XML Injection |
| **WebSocket** | websocket | `crates/eggsec/src/fuzzer/payloads/websocket.rs` | WebSocket Fuzzing |
| **NoSQL Injection** | nosql | `crates/eggsec/src/fuzzer/payloads/nosql.rs` | NoSQL injection payloads |
| **XPath Injection** | xpath | `crates/eggsec/src/fuzzer/payloads/xpath.rs` | XPath injection payloads |
| **Expression Language Injection** | expression | `crates/eggsec/src/fuzzer/payloads/expression.rs` | Expression language injection |
| **Prototype Pollution** | prototype | `crates/eggsec/src/fuzzer/payloads/prototype.rs` | Prototype pollution payloads |
| **Race Condition** | race | `crates/eggsec/src/fuzzer/payloads/race.rs` | Race condition testing |
| **Mass Assignment** | massassign | `crates/eggsec/src/fuzzer/payloads/mass_assign.rs` | Mass assignment testing |
| **Out-of-Band Testing** | oast | `crates/eggsec/src/fuzzer/payloads/oast.rs` | Out-of-band application security testing |
| **CVE Lookup** | cve | `crates/eggsec/src/recon/cve_lookup.rs` | CVE vulnerability testing based on detected technologies |

---

## Detection Modules

Advanced vulnerability detection capabilities:

| Module | File | Description |
|--------|------|-------------|
| **Timing Analyzer** | `crates/eggsec/src/fuzzer/detection/analyzer.rs` | Detects vulnerabilities based on response time differences (blind injection detection) |
| **Pattern Matcher** | `crates/eggsec/src/fuzzer/detection/patterns.rs` | Signature-based vulnerability detection using regex patterns |
| **Aho-Corasick** | `crates/eggsec/src/fuzzer/detection/aho_corasick.rs` | High-performance multi-pattern matching for bulk vulnerability detection |
| **ReDoS Detector** | `crates/eggsec/src/fuzzer/redos_detect.rs` | Executes regexes to identify catastrophic backtracking vulnerabilities |
| **WAF Fingerprinter** | `crates/eggsec/src/fuzzer/waf_fingerprint.rs` | Identifies specific WAF products and versions |
| **Diff Analyzer** | `crates/eggsec/src/fuzzer/diff.rs` | Compares responses to detect anomalies |
| **Rate Limit Detector** | `crates/eggsec/src/fuzzer/rate_limit.rs` | Detects and analyzes rate limiting behavior |

---

## Mobile App Security (Static Analysis)

Static analysis of Android APKs and iOS IPAs for authorized lab/defense use only. Feature-gated behind `mobile`; static-only Phase 1 (no execution, no dynamic instrumentation, no device interaction, no network traffic to the app). All work is offline on user-supplied lab binaries.

| Area | Coverage | File |
|------|----------|------|
| **Manifest / Info.plist** | Package/app ID, version, platform metadata | `crates/eggsec/src/mobile/{mod,apk,ipa}.rs` |
| **Permissions** | Android permissions (normal/dangerous/signature); iOS usage descriptions and sensitive entitlements | `crates/eggsec/src/mobile/{apk,ipa}.rs` |
| **Transport / ATS** | Android `usesCleartextTraffic`, `networkSecurityConfig`; iOS `NSAppTransportSecurity` exceptions (allowsArbitraryLoads, domain exceptions) | `crates/eggsec/src/mobile/{apk,ipa}.rs` |
| **Secrets / Hardcoded Values** | Bounded scan of text assets for API keys, tokens, credentials (isolated scanner) | `crates/eggsec/src/mobile/{apk,ipa}.rs` |
| **Debug / Backup / Exported** | Android `debuggable`, `allowBackup`, `android:exported` on activities/services/receivers/providers; iOS `UIFileSharingEnabled`, `get-task-allow` profile markers | `crates/eggsec/src/mobile/{apk,ipa}.rs` |
| **Signing / Provisioning** | Android v1 signing (META-INF/*.RSA|DSA|EC|CERT.* presence); iOS `_CodeSignature` presence and `embedded.mobileprovision` markers (enterprise, debug/ad-hoc indicators) | `crates/eggsec/src/mobile/{apk,ipa}.rs` |
| **Custom URL Schemes / Extensions** | iOS `CFBundleURLTypes`; Android intent-filter schemes; extension markers | `crates/eggsec/src/mobile/ipa.rs` |

**Feature gate**: Requires `--features mobile` (or `--features full`, which includes it). Dependencies: `zip` (always under feature); `plist` (iOS path only, optional under feature).

**Safety**: Pure-Rust ZIP + plist + bounded AXML extraction. No shelling out. Explicit `mobile` command; lab-only framing. See `crates/eggsec/src/mobile/{mod,apk,ipa}.rs`, `crates/eggsec/src/cli/mobile.rs`, `crates/eggsec/src/commands/handlers/mobile.rs`, and policy integration in `config/policy_decision.rs`.
Dynamic mobile (Android ADB + logcat + Phase 2 (proxy + permissions + correlation; closed 2026-06-12) + static ↔ dynamic correlation) implemented under `mobile-dynamic` (Phase 1 + Phase 2 (closed 2026-06-12) + final polish + close-out polish complete 2026-06-12 per `plans/mobile-dynamic-phase2-closeout-and-phase3-kickoff-plan.md` (executed)). Phase 3/4a (Frida + CorrelationEngine + baseline/regression/evidence bundles + polish handoff) delivered 2026-06-12 under single mobile-dynamic per phase3/phase4 + phase4a-final-polish-handoff-plan.md (executed). Design and future phases in `plans/dynamic-mobile-testing-loadout-design-plan.md`. Same standalone defense-lab pattern: gated `mobile-dynamic` feature, MCP-absent, `to_scan_report_data_dynamic` bridge with `mobile-dynamic-android-*` categories.

---

## Stress Testing Types

| Type | File | Description |
|------|------|-------------|
| **HTTP Flood** | `crates/eggsec/src/stress/http.rs` | Sends high-volume HTTP requests with randomized User-Agent, X-Forwarded-For headers, proxy support |
| **SYN Flood** | `crates/eggsec/src/stress/syn.rs` | TCP SYN packet flooding (requires root + stress-testing feature) |
| **UDP Flood** | `crates/eggsec/src/stress/udp.rs` | UDP packet flooding (requires root + stress-testing feature) |
| **ICMP Flood** | `crates/eggsec/src/stress/icmp.rs` | Ping flood attacks (requires root + stress-testing feature) |
| **Metrics Collection** | `crates/eggsec/src/stress/metrics.rs` | Tracks packets sent, bytes sent, errors, duration |

---

## Protocol Implementations

| Protocol | File | Description |
|----------|------|-------------|
| **REST API** | `crates/eggsec/src/tool/protocol/rest.rs` | REST API server - exposes eggsec tools via HTTP (requires rest-api feature) |
| **gRPC API** | `crates/eggsec/src/tool/protocol/grpc.rs` | gRPC API server - exposes eggsec tools via protocol buffers (requires grpc-api feature) |
| **MCP Server** | `crates/eggsec/src/tool/protocol/mcp/` | MCP (Model Context Protocol) - JSON-RPC server for AI agent integration (built-in) |

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
| **auth** | JWT/OAuth/IDOR security testing (pipeline: PortScan+Fingerprint+EndpointScan+Fuzz; distinct from `auth-test` CLI credential/brute/MFA control validation) |
| **defense-lab** | comprehensive local defense validation (defense-lab mode) |
| **synvoid-local** | Synvoid-specific WAF validation (defense-lab mode) |
| **waf-regression** | WAF payload and evasion regression (defense-lab mode) |
| **protocol-edge** | malformed protocol edge behavior (defense-lab mode) |
| **nse-safe** | sandboxed NSE scripts (defense-lab mode) |

---

## MCP Profiles

Eggsec's MCP server has two profiles that control available tools and safety policies.

### Ops-Agent Profile

Full security testing toolkit for AI agents. All tools are available.

| Capability | Available | Notes |
|------------|-----------|-------|
| Recon | Yes | All 21+ modules |
| Port scanning | Yes | |
| Fingerprinting | Yes | |
| Fuzzing (all types) | Yes | 40 payload types |
| WAF detection/bypass | Yes | |
| WAF stress testing | Yes | |
| Load testing | Yes | |
| Stress testing | Yes | SYN/UDP/ICMP floods |
| Pipeline scanning | Yes | All profiles |
| Session management | Yes | |
| Plan generation | Yes | |

### Coding-Agent Profile

Bounded security validation tools for coding assistants. Restricted toolset with enforced safety.

| Capability | Available | Notes |
|------------|-----------|-------|
| Target validation | Yes | Verify target is local/scope-allowed |
| Re-check findings | Yes | Verify if a finding is fixed/still present |
| Port scanning | Limited | Localhost/private IPs only |
| Fingerprinting | Limited | Localhost/private IPs only |
| Fuzzing (safe types) | Limited | xss, sqli, traversal only |
| WAF detection | Limited | Localhost/private IPs only |
| Load testing | No | Denied by policy |
| Stress testing | No | Denied by policy |
| Broad recon | No | Denied by policy |
| External network | No | Denied by policy (unless scoped) |

### Coding-Agent Safety Defaults

| Setting | Value |
|---------|-------|
| Target policy | `ScopeOrLocalDevOnly` |
| Max concurrency | 5 |
| Max timeout | 60,000 ms |
| Max batch size | 10 |
| External network | Blocked |
| Stress testing | Blocked |
| Broad recon | Blocked |

---

## CLI Commands Quick Reference

### Core Scanning Commands

| Command | Description |
|---------|-------------|
| `eggsec load <url>` | HTTP load testing |
| `eggsec scan-ports <target>` | Port scanning |
| `eggsec scan-endpoints <url>` | Endpoint discovery |
| `eggsec fingerprint <target>` | Service fingerprinting |
| `eggsec fuzz <url> -t <type>` | Security fuzzing |
| `eggsec waf <url>` | WAF detection |
| `eggsec waf-stress <url>` | WAF stress testing |
| `eggsec scan <target> --profile <profile>` | Pipeline scan |
| `eggsec recon <target>` | Reconnaissance |

### API Security Commands

| Command | Description |
|---------|-------------|
| `eggsec graphql <url>` | GraphQL security testing |
| `eggsec oauth <url>` | OAuth/OIDC testing |

### Network Tools

| Command | Description |
|---------|-------------|
| `eggsec packet capture` | Live packet capture |
| `eggsec packet send` | Craft and send packets |
| `eggsec packet dump` | Analyze pcap files |
| `eggsec packet traceroute` | Network path tracing |
| `eggsec icmp <target>` | ICMP ping probes |
| `eggsec traceroute <target>` | Network path tracing |

### Wireless (requires wireless feature)

| Command | Mode | Description |
|---------|------|-------------|
| `eggsec wireless <iface> scan` | defense-lab (passive) | Passive WiFi reconnaissance (iwlist-based) |
| `eggsec wireless <iface> deauth` | defense-lab (active, requires wireless-advanced) | Active WiFi deauth/disassoc (lab-only, requires --allow-active-wireless) |

### Stress Testing (requires stress-testing feature)

| Command | Mode | Description |
|---------|------|-------------|
| `eggsec stress <target> --type <type>` | Defense Lab / Hazardous Lab | SYN/UDP/HTTP/TCP/ICMP flood |
| `eggsec proxy <action>` | Hazardous Lab | Proxy pool management |
| `eggsec icmp <target>` | Hazardous Lab | ICMP echo probes |
| `eggsec traceroute <target>` | Hazardous Lab | Network path tracing |

### Management & Integration

| Command | Description |
|---------|-------------|
| `eggsec cluster coordinator` | Start cluster coordinator |
| `eggsec cluster worker` | Start cluster worker |
| `eggsec remote --port` | Start remote listener |
| `eggsec exec --target` | Execute remote commands |
| `eggsec notify` | Webhook notifications |
| `eggsec report` | Generate reports |
| `eggsec resume <file>` | Resume previous scan |

### API Servers

| Command | Description |
|---------|-------------|
| `eggsec serve --port` | REST API server |
| `eggsec grpc-serve --port` | gRPC API server |
| `eggsec mcp-serve --port` | MCP server for AI agents |
| `eggsec codegg-mcp` | MCP server for coding agents (stdio, coding-agent profile) |

### NSE Scripting

| Command | Description |
|---------|-------------|
| `eggsec nse <target>` | Run NSE scripts |

### Lab Defense Commands

| Command | Mode | Description |
|---------|------|-------------|
| `policy-explain` | - | Explain policy decisions |
| `scope-explain` | - | Explain scope matching |
| `eggsec auth-test <target>` | defense-lab (high-risk) | Credential control validation (brute-force, stuffing, lockout, MFA, rate-limit, timing; policy-gated via `allow_credential_testing`). Intentionally standalone defense-lab CLI (separate from pipeline); local `AuthTestReport`/`AuthFinding` only (direct emit; no `ScanReportData`, no SARIF/JUnit/etc conversion or bridge). Distinct from `ScanProfile::Auth` (JWT/OAuth/IDOR fuzzing via pipeline stages). TUI `AuthTab` (`Tab::Auth`) fully integrated (TabSpec, task system, policy enforcement, session save/restore). See `docs/AUTH_LAB.md` + architecture/auth.md. |
| `eggsec wireless <iface> scan` | defense-lab (passive) | Standalone-complete passive WiFi recon (iwlist): Open/WEP/WPA/WPA2/WPA3/Enterprise + WPS/hidden/transition/weak-signal detection, vuln findings, rogue/Evil-Twin heuristic (passive; security-diff elevates to Medium). Supports `--repeat` (per-scan diffs + temporal summary for change/rogue observation), `--known-good` FILE allowlist (SSID/BSSID/"SSID,BSSID"; suppresses rogue for lab baselines), `--dry-run` (plan/CI mode; no iwlist/privs; valid JSON + notes), `--detect-suspicious` (full rogue details; summarized by default in human output; analysis always runs). Recommendations generated. Optional `to_scan_report_data` bridge (and CLI auto-bridge) for unified reports (SARIF/JUnit/etc). Requires `--features wireless` + root/CAP_NET_ADMIN + wireless-tools/iwlist. Bridged findings use `wireless-*` categories (e.g. wireless-rogue, wireless-security). MCP/agent tool exposure intentionally absent (standalone defense-lab design decision; not a SecurityTool). **Phase 0 (passive) complete**. See docs/WIRELESS.md (incl. Integration with Reporting Pipeline) and architecture/wireless.md (MCP/Agentic section). |
| `eggsec wireless <iface> deauth` | defense-lab (active, requires wireless-advanced) | Active WiFi deauth/disassoc frame injection for authorized lab testing. Subcommand: `scan` (passive, default) or `deauth` (active). Policy: `OperationRisk::Intrusive` + `wireless-advanced` feature + `--allow-active-wireless` flag. Frame builders: `build_deauth_frame`, `build_disassoc_frame`, `inject_frames`, `run_deauth`, `run_disassoc`. Output: `ActiveWirelessAttackResult` + `ActiveWirelessFinding`. Same standalone defense-lab rule: MCP/agent tool exposure intentionally absent. **Phase 1 complete**. See `plans/wireless-active-attacks-loadout-design-plan.md`. |
| `eggsec mobile <path.{apk,ipa}>` (or `eggsec mobile static ...`) | defense-lab (static) | Standalone static analysis of Android APKs and iOS IPAs (manifest, permissions, transport config, secrets, debug/backup/exported components, signing/provisioning). Pure-Rust offline on user-supplied lab binaries only. Feature-gated `mobile`. Policy via SafeActive + required_features:["mobile"]; local MobileScanReport/MobileFinding + optional to_scan_report_data bridge (native --json auto-bridged by report convert). See docs/MOBILE.md (Integration section) and architecture/mobile.md. `eggsec mobile dynamic ...` requires `--features mobile-dynamic` (Phase 1 + Phase 2 (closed 2026-06-12) + final polish + close-out polish complete 2026-06-12; design per `plans/dynamic-mobile-testing-loadout-design-plan.md`; Phase 1 per `plans/mobile-dynamic-phase1-implementation-handoff-plan.md` (executed 2026-06-12); Phase 1 polish (smoke test script, `--list-devices` convenience, troubleshooting, docs) per `plans/mobile-dynamic-post-phase1-polish-and-phase2-planning.md` (executed 2026-06-12); Phase 2 (closed 2026-06-12) per `plans/mobile-dynamic-phase2-closeout-and-phase3-kickoff-plan.md` (executed 2026-06-12); final polish per `plans/mobile-dynamic-phase2-final-polish-handoff-plan.md` (executed 2026-06-12); close-out per `plans/mobile-dynamic-phase2-close-out-polish-plan.md` (executed 2026-06-12)). Same standalone defense-lab pattern as wireless. |
| `eggsec db pentest ...` | defense-lab (Phase 1+3) | Direct (non-web) Postgres/MySQL/MSSQL security assessment for authorized lab/defense use. Phase 1+2+3 (complete 2026-06-12): Postgres + MySQL + MSSQL tiberius checks, lab manifest, budgets, dry-run, `--allow-db-pentest` gate, local `DbPentestReport`/`DbFinding`, optional `to_scan_report_data_db` bridge (auto-bridged via `report convert`). Phase 3 added: TUI tab `Tab::DbPentest` + pipeline `ScanProfile::DbRegression` + advanced gated checks behind `--allow-db-pentest-advanced` + correlation/evidence stubs. Standalone defense-lab (see docs/DATABASE_PENTEST.md, architecture/defense_lab.md, `plans/database-pentesting-phase1-foundation-handoff-plan.md` + `plans/database-pentesting-phase3-advanced-and-integration-handoff-plan.md`). |

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
| `ai-integration` | AI planner, script generation |
| `headless-browser` | DOM XSS and SPA crawling |
| `database` | SQLx-based persistence |
| `container` | Kubernetes/Docker scanning |
| `sbom` | SBOM generation |
| `advanced-hunting` | Advanced threat hunting |
| `compliance` | Compliance scanning |
| `wireless` | WiFi scanning (standalone-complete passive recon + security posture; summary-by-default rogue heuristic; WPS/hidden/transition; `--repeat`/`--known-good`/`--dry-run`/`--detect-suspicious`). TUI tab under feature; MCP/agent tool exposure intentionally absent (standalone defense-lab surface). **Passive Phase 0 complete**; active phases gated by `wireless-advanced`. |
| `wireless-advanced` | Active WiFi attacks (deauth/disassoc frame injection) for authorized lab testing. Subcommand: `eggsec wireless <iface> deauth`. Policy: `OperationRisk::Intrusive` + `wireless-advanced` feature + `--allow-active-wireless` flag. Requires `wireless` feature. **Phase 1 (deauth/disassoc) complete**. |
| `mobile` | Mobile app static analysis (APK/IPA manifest & config checks for authorized lab/defense use only; static-only). Dynamic mobile (Phase 1 + Phase 2 (closed 2026-06-12) + final polish + close-out) shipped under `mobile-dynamic`; Phase 3/4a (Frida + CorrelationEngine + baseline/regression/evidence bundles + polish handoff) delivered 2026-06-12 under single mobile-dynamic per phase3/phase4 + phase4a-final-polish-handoff-plan.md (executed). Future phases per `plans/dynamic-mobile-testing-loadout-design-plan.md`. |
| `db-pentest` | Standalone defense-lab direct Postgres/MySQL/MSSQL security assessment (Phase 1+2+3: postgres/mysql checks + manifest + bridge + real MSSQL tiberius + TUI tab `Tab::DbPentest` + pipeline `ScanProfile::DbRegression` + advanced gated checks + correlation/evidence stubs; requires `--allow-db-pentest` for non-dry runs; dry-run always safe; local types + optional bridge to unified reports via report convert). See `plans/database-pentesting-phase1-foundation-handoff-plan.md` (executed) + `plans/database-pentesting-phase3-advanced-and-integration-handoff-plan.md` (executed). |
| `full` | All features combined |
