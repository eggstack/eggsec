# Docker Setup for Eggsec

This directory contains Docker configuration for setting up Eggsec and its test targets in a controlled environment.

## Quick Start

```bash
# Build the image
docker build -t eggsec:latest .

# Run basic scanner (no test targets)
docker compose --profile scanner up -d eggsec

# Run with test targets (DVWA, Juice Shop, etc.)
docker compose --profile testing up -d

# Full stack including Elasticsearch/Kibana
docker compose --profile full up -d
```

## Available Services

| Service | Port | Description | Profile |
|---------|------|-------------|---------|
| eggsec | - | Main security testing toolkit | scanner, testing, full, distributed |
| dvwa | 8080 | Damn Vulnerable Web App | testing, full |
| juice-shop | 3000 | OWASP Juice Shop | testing, full |
| metasploitable | 2222, 3306, etc. | Metasploitable2 | testing, full |
| elasticsearch | 9200 | Scan results storage | storage, full |
| kibana | 5601 | Results visualization | storage, full |
| webhook-receiver | 8081 | Test webhook notifications | testing, full |
| tor-proxy | 9050 | Tor SOCKS proxy for anonymization | testing, full |
| redis | 6379 | Task queue for distributed mode | distributed, full |

## Profiles

- **scanner**: Core eggsec tool only
- **testing**: Test targets (DVWA, Juice Shop, Metasploitable, webhook receiver)
- **storage**: Elasticsearch + Kibana
- **distributed**: Redis for distributed scanning
- **full**: All services

## Features Enabled

The Dockerfile builds with `--features full`, enabling:

- **stress-testing**: SYN floods, ICMP, raw socket operations
- **packet-inspection**: Live packet capture, hexdump, traceroute

## Security Capabilities

The eggsec container is granted:
- `NET_RAW` - Raw socket access for ping/scanning
- `NET_ADMIN` - Network administration for stress testing

These are required for packet-level operations. For full packet capture capabilities, run with `--privileged`.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| EGGSEC_LOG_LEVEL | info | Logging level |
| EGGSEC_CONFIG | - | Path to config file |
| EGGSEC_RATE_LIMIT | 50 | Requests per second |
| REDIS_URL | - | Redis connection for distributed mode |

## Volume Mounts

- `./scan-results:/app/results` - Scan output directory
- `eggsec-sessions:/home/eggsec/.local/share/eggsec` - Session data
- `elastic-data:/usr/share/elasticsearch/data` - Elasticsearch data
- `redis-data:/data` - Redis persistence

## Running Specific Test Scenarios

### Web Application Testing
```bash
docker compose --profile testing up -d dvwa juice-shop
docker compose run eggsec scan http://dvwa-target
```

### Anonymized Scanning
```bash
docker compose --profile full up -d tor-proxy
docker compose run eggsec scan http://target --proxy socks5://tor-proxy:9050
```

### Distributed Scanning
```bash
# Start coordinator
docker compose --profile distributed up -d redis
docker compose run eggsec cluster coordinator --port 9000

# In another terminal, start workers
docker compose run eggsec cluster worker --coordinator eggsec:9000
```

### WAF Detection & Bypass Testing
```bash
docker compose --profile testing up -d dvwa
docker compose run eggsec waf --target http://dvwa-target
```

## Building for Specific Features

To build with only specific features, modify the Dockerfile:

```dockerfile
# Stress testing only  
RUN cargo build --release --features stress-testing
```

## Troubleshooting

### Permission Denied Errors
Ensure the container has proper capabilities or run with `--privileged`.

### Redis Connection Failed
Ensure Redis is running and accessible: `docker compose --profile distributed up -d redis`

### Tor Proxy Not Working
Tor may need time to bootstrap. Check logs: `docker compose logs tor-proxy`

## Notes

- All test targets are intentionally vulnerable - do not expose to untrusted networks
- Default credentials where applicable are documented in target service configs
- The eggsec container runs as non-root user for safety, with elevated capabilities only when needed
