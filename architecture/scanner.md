# Scanner Module

The Scanner module is responsible for the "discovery" phase of a security assessment. It includes port scanning, service identification, and endpoint discovery.

## Core Capabilities (`src/scanner/`)

### Port Scanning (`ports/`)

High-performance TCP and UDP port scanning.

- **TCP Connect Scan**: Standard 3-way handshake.
- **SYN Scan**: Half-open scanning for speed and stealth (requires elevated privileges).
- **Service Fingerprinting**: Once a port is found open, Slapper attempts to identify the running service and version.

### Endpoint Discovery (`endpoints.rs`)

Finding hidden files and directories on web servers.

- **Wordlist-based Brute Forcing**: Uses extensive wordlists to find common endpoints.
- **Recursive Scanning**: Automatically crawls discovered directories.
- **Custom Payload Support**: Allows for targeted discovery based on specific technologies.

### Fingerprinting (`fingerprint.rs`, `cms/`)

Identifying the technology stack of a target.

- **HTTP Banner Grabbing**: Extracting information from server headers.
- **Technology Detection**: Identifying frameworks (e.g., React, Django), databases, and CMS (e.g., WordPress, Drupal).
- **CVE Mapping**: Automatically mapping discovered versions to known vulnerabilities.

### Advanced Probing (`icmp_probe.rs`, `udp_fingerprint.rs`)

- **ICMP Probing**: Host discovery using echo requests and other ICMP types.
- **UDP Fingerprinting**: Identifying services on UDP ports through specific probe payloads.
- **Spoofing (`spoof.rs`)**: Techniques for source IP spoofing and decoys (where supported).

## Timing and Performance (`timing.rs`)

The scanner uses "Timing Templates" (similar to Nmap's -T0 through -T5) to control the speed and aggressiveness of scans, ensuring they stay within the limits of the target network and the user's requirements.

## Integration

Discovered information is often fed into the **Fuzzer** or **Vulnerability Management** modules for further analysis.
