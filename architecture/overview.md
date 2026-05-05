# Slapper Architectural Overview

Slapper is a high-performance, async-first security testing toolkit built in Rust. It is designed for penetration testers and security researchers, offering a wide range of capabilities from reconnaissance to advanced fuzzing and distributed scanning.

## Core Module Groups

- **[CLI & Commands](cli_commands.md)**: Command-line argument parsing and command dispatch.
- **[Configuration](config.md)**: TOML/YAML configuration loading and scope enforcement.
- **[Scanner](scanner.md)**: Port scanning, service fingerprinting, and endpoint discovery.
- **[Fuzzer](fuzzer.md)**: Advanced security fuzzing engine with support for 22 payload types.
- **[WAF](waf.md)**: Web Application Firewall detection and bypass techniques.
- **[Reconnaissance](recon.md)**: Passive and active recon (DNS, WHOIS, SSL, CVE mapping).
- **[Load Testing](loadtest.md)**: High-performance HTTP load testing with real-time metrics.
- **[Pipeline](pipeline.md)**: Orchestration of chained security assessment profiles.
- **[AI & Agents](ai_agents.md)**: AI-driven analysis, payload generation, and autonomous agent integration via MCP.
- **[TUI](tui.md)**: Real-time Terminal User Interface for interactive scanning.
- **[Output & Reporting](output.md)**: Support for multiple formats including JSON, SARIF, and PDF.
- **[Distributed](distributed.md)**: Scalable worker/coordinator architecture for large-scale assessments.
- **[Networking](networking.md)**: Packet capture, crafting, and low-level stress testing.
- **[Plugins & NSE](plugins_nse.md)**: Extensibility via Python, Ruby, and Nmap Scripting Engine.

## Specialized Modules

- **Authentication (`auth`)**: Support for various authentication mechanisms (Basic, Bearer, OAuth, Custom).
- **Headless Browser (`browser`)**: Integration with headless Chrome for DOM XSS and SPA crawling.
- **Compliance (`compliance`)**: Scanning and reporting against security standards (e.g., OWASP, PCI-DSS).
- **Container Security (`container`)**: Kubernetes and Docker-specific security checks.
- **Integrations (`integrations`)**: Connectors for Jira, GitHub, GitLab, and other external tools.
- **Storage (`storage`)**: Persistence layer for findings, history, and configuration using SQLx.
- **Supply Chain (`supply_chain`)**: Tools for generating and analyzing SBOMs (Software Bill of Materials).
- **Vulnerability Management (`vuln`, `workflow`)**: Triage, prioritization, and lifecycle management of discovered vulnerabilities.

## Workspace Crates

- **`slapper`**: The core toolkit crate containing the majority of the logic.
- **`slapper-plugin`**: A flexible plugin system supporting Python and Ruby extensions, allowing for easy custom scanner development.
- **`slapper-nse`**: A full integration of the Nmap Scripting Engine (NSE), allowing Slapper to run thousands of existing Nmap scripts.
- **`slapper-ruby`**: Specialized bridge for Ruby-based tools and Metasploit RPC integration.

## Design Principles

- **Async-First**: Built on top of `tokio` for high concurrency and performance.
- **Modular & Extensible**: Heavy use of feature flags and a robust plugin system.
- **Security-Focused**: Built-in WAF bypass, payload generation, and threat hunting features.
- **Standardized**: Support for industry-standard formats like SARIF and SPDX.

---
*This overview serves as an index for detailed component documentation.*
