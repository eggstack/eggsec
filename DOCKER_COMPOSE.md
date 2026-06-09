# Docker Compose Quick Reference

## Profiles

| Profile | Services | Use Case |
|---------|----------|----------|
| `scanner` | eggsec only | Run eggsec against external targets |
| `testing` | eggsec + dvwa + juice-shop + webhook-receiver | Test against vulnerable containers |
| `storage` | elasticsearch + kibana | Store/visualize results from external eggsec |
| `full` | everything | Complete testing environment |

---

## Quick Start

### 1. Testing Mode (Recommended for development)
```bash
# Start vulnerable targets only
docker-compose --profile testing up -d dvwa

# Wait for DVWA to be ready, then run eggsec
docker-compose --profile testing run --rm eggsec scan-endpoints http://dvwa.target.local
```

### 2. Full Test Environment
```bash
# Start everything
docker-compose --profile full up -d

# Run scans against DVWA
docker-compose --profile full run --rm eggsec fuzz http://dvwa.target.local/login -t xss,sqli
docker-compose --profile full run --rm eggsec scan-endpoints http://dvwa.target.local

# Run scans against Juice Shop
docker-compose --profile full run --rm eggsec scan-endpoints http://juice-shop.target.local:3000

# View results in Kibana (http://localhost:5601)
```

### 3. Scanner Mode (against external targets)
```bash
# Note: Use host networking or configure proxy for external targets
docker-compose --profile scanner run --rm eggsec scan example.com
```

---

## Common Commands

### Port Scanning
```bash
docker-compose --profile testing run --rm eggsec scan-ports dvwa.target.local -p 1-1000
```

### Endpoint Discovery
```bash
docker-compose --profile testing run --rm eggsec scan-endpoints http://dvwa.target.local
docker-compose --profile testing run --rm eggsec scan-endpoints http://juice-shop.target.local:3000
```

### Fuzzing
```bash
# Basic XSS scan
docker-compose --profile testing run --rm eggsec fuzz http://dvwa.target.local/login -t xss

# SQL Injection
docker-compose --profile testing run --rm eggsec fuzz http://dvwa.target.local/vulnerabilities/sqli/ -t sqli

# All payloads
docker-compose --profile testing run --rm eggsec fuzz http://dvwa.target.local/vulnerabilities/fi/ -t all
```

### Full Pipeline Scan
```bash
docker-compose --profile testing run --rm eggsec scan http://dvwa.target.local --profile deep
```

### With Storage (Elasticsearch)
```bash
# Start storage services
docker-compose --profile storage up -d

# Index results (requires eggsec config for elasticsearch)
docker-compose --profile full run -e EGGSEC_OUTPUT_ELASTIC=http://elasticsearch:9200 eggsec scan http://dvwa.target.local
```

---

## Target URLs

| Service | URL | Notes |
|---------|-----|-------|
| DVWA | http://dvwa.target.local:80 | Default creds: admin/password |
| Juice Shop | http://juice-shop.target.local:3000 | |
| Webhook Receiver | http://webhook.target.local:8080 | Check incoming requests |

---

## Troubleshooting

### DVWA not ready
```bash
# Check DVWA status
docker-compose --profile testing ps

# View DVWA logs
docker-compose --profile testing logs dvwa

# Reinitialize DVWA database
docker-compose --profile testing exec dvwa service mysql start && php /var/www/html/setup.php
```

### Network issues
```bash
# Verify network connectivity from eggsec
docker-compose --profile testing run --rm eggsec sh -c "curl -v http://dvwa.target.local"
```

### View scan results
```bash
# Results are saved to ./scan-results
ls -la scan-results/

# Or view in container
docker-compose --profile testing run --rm eggsec ls /app/results
```
