# Lab Safety

Eggsec includes high-risk features that can disrupt networks, lock out accounts, or cause denial of service. This document covers safe use of these features.

**All high-risk features should only be used against systems you own or have explicit written authorization to test.**

## Stress Testing

**Risk: Denial of Service**

Stress testing generates high volumes of traffic against a target. It can:
- Overwhelm target services causing downtime
- Saturate network links
- Trigger IDS/IPS alerts
- Impact co-located services on shared infrastructure

**Requirements:**
- Written authorization from the target owner
- Run only in isolated lab environments
- Use `--rate-limit` to cap requests per second
- Use `--concurrency` to limit parallel connections
- Have a shutdown plan and monitoring in place

```bash
# Stress test with explicit limits
eggsec stress "$TARGET" \
  --rate-limit 100 \
  --concurrency 10 \
  --duration 60 \
  --scope scopes/lab.toml
```

**Never** run stress tests against production systems, shared infrastructure, or targets without authorization.

## Packet / Raw Socket Operations

**Risk: Network disruption, requires elevated privileges**

Raw packet operations (crafted packets, IP spoofing, packet capture) require root/sudo and can:
- Disrupt network connections if misconfigured
- Trigger security alerts on network monitoring
- Violate network policies

**Requirements:**
- Root or sudo access
- Isolated network (no production traffic)
- Understanding of packet structure and impact
- `stress-testing` feature flag enabled

```bash
# Packet capture in isolated environment
sudo eggsec packet capture \
  --interface eth1 \
  --filter "tcp port 80" \
  --output captures/

# Packet crafting with explicit target
sudo eggsec packet craft \
  --target lab-host \
  --protocol tcp \
  --dport 80 \
  --scope scopes/lab.toml
```

**Never** use raw packet features on production networks, shared switches, or without understanding the downstream impact.

## WAF Evasion-Resistance Testing

**Risk: May trigger security responses**

WAF bypass testing sends payloads designed to evade web application firewalls. It can:
- Trigger WAF alerts and IP blocks
- Generate security team responses
- Log suspicious activity in WAF dashboards

**Requirements:**
- Authorization from both the application owner and WAF operator
- Coordination with the security operations team
- Use against dedicated test instances when possible

```bash
# WAF testing with scope and rate limiting
eggsec waf detect "$TARGET" \
  --scope scopes/authorized.toml \
  --rate-limit 50
```

## Proxy / Tor Usage

**Risk: Legal considerations, route leaks, attribution issues**

Using Eggsec through proxies or Tor:
- May violate terms of service for proxy providers
- Route leaks can expose your real IP
- Some jurisdictions restrict Tor usage
- Proxy exit nodes can log traffic

**Requirements:**
- Understand the legal implications in your jurisdiction
- Verify proxy configuration (no route leaks)
- Use only with authorized targets
- Document the proxy chain for accountability

```bash
# Proxy usage with explicit configuration
eggsec scan "$TARGET" \
  --proxy socks5://127.0.0.1:9050 \
  --scope scopes/authorized.toml
```

## Authentication Testing

**Risk: Account lockout, credential exposure, legal exposure**

Auth testing attempts credential stuffing, brute force, or session manipulation. It can:
- Lock out legitimate accounts
- Trigger account suspension
- Generate audit logs that may require explanation
- Create legal liability without authorization

**Requirements:**
- Written authorization specifying auth testing scope
- Test accounts dedicated to security testing
- Coordination with the team managing authentication
- Rate limiting to prevent lockout

```bash
# Auth testing with explicit limits
eggsec auth-test "$TARGET" \
  --wordlist test-credentials.txt \
  --max-attempts 50 \
  --concurrency 2 \
  --scope scopes/authorized.toml
```

**Never** use production credentials, test against accounts you do not own, or exceed the authorized testing scope.

## Rate and Concurrency Limits

**Always** use rate and concurrency limits with high-risk features:

| Flag | Purpose | Recommended Range |
|------|---------|-------------------|
| `--rate-limit` | Max requests per second | 10-500 depending on target |
| `--concurrency` | Max parallel connections | 5-50 depending on target |
| `--timeout` | Per-request timeout | 10-30 seconds |

Scope files can enforce rate limits globally:

```toml
max_requests_per_second = 100
```

## Private Lab Recommendation

Run high-risk features in an isolated environment:

| Method | Isolation Level | Setup Effort |
|--------|----------------|--------------|
| **Docker containers** | Good | Low |
| **VMs (Vagrant, libvirt)** | Good | Medium |
| **Dedicated lab network** | Best | High |
| **Cloud sandbox (VPC)** | Good | Medium |

**Minimum lab requirements:**
- Isolated network segment (no routing to production)
- Monitoring and logging enabled
- Ability to tear down and rebuild quickly
- No shared services that could be impacted

```bash
# Docker-based isolated lab
docker network create --driver bridge eggsec-lab
docker run -d --name target --network eggsec-lab vulnerable-app:latest

# Scope the lab network
cat > /tmp/lab-scope.toml << 'EOF'
require_explicit_scope = true
[[allowed_targets]]
cidr = "172.20.0.0/16"
description = "Docker lab network"
EOF

eggsec scan target --scope /tmp/lab-scope.toml
```

## Monitoring and Rollback

When running high-risk features:

1. **Monitor the target** - Watch for service degradation, error spikes, or resource exhaustion
2. **Have a kill switch** - Know how to stop the scan immediately (`Ctrl+C` or `eggsec stop`)
3. **Log everything** - Eggsec logs all operations; review after testing
4. **Expect rollback** - Have a plan to restore the target to its pre-test state
5. **Document the test** - Record what was tested, when, and what the results were

## See Also

- [SAFETY.md](SAFETY.md) - Operation risk tiers and authorization requirements
- [scope.md](scope.md) - Scope model and enforcement details
- [agent-workflows.md](agent-workflows.md) - Agent-oriented workflows
