---
name: intercepting_proxy
description: "HTTP/HTTPS intercepting proxy with dynamic SSL certificate generation for MITM testing"
triggers:
  - intercept proxy
  - mitm
  - ssl intercept
  - dynamic certificate
  - http proxy
  - traffic interception
  - intercepting proxy
  - mitmproxy
  - ssl MITM
metadata:
  category: proxy
  tools: [proxy, intercept]
  scope: targets
---

## Overview

Slapper provides an intercepting proxy with dynamic SSL certificate generation for security testing. This enables HTTP/HTTPS traffic interception, request/response modification, and monitoring capabilities.

## Capabilities

- **Dynamic SSL Certificates**: Real-time certificate generation for HTTPS interception
- **Request Interception**: Capture and optionally modify requests in transit
- **Response Interception**: Monitor or modify server responses
- **Monitor Mode**: Passive traffic logging without modification
- **Configurable Rules**: Define interception rules based on URL, method, headers
- **TLS Passthrough**: Forward TLS connections without interception

## Key Types

```rust
// Proxy server
pub struct ProxyServer {
    addr: SocketAddr,
    cert_generator: CertGenerator,
    rules: Arc<RwLock<RuleSet>>,
    mode: InterceptMode,
}

// Intercept modes
pub enum InterceptMode {
    Monitor,        // Passive logging only
    Intercept,      // Active interception
    Passthrough,    // No interception
}

// Interception rules
pub struct InterceptRule {
    pub name: String,
    pub pattern: String,
    pub action: RuleAction,
}

pub enum RuleAction {
    Allow,
    Block,
    Modify,
    Log,
}

pub struct RuleSet {
    pub rules: Vec<InterceptRule>,
    pub default_action: RuleAction,
}

// Certificate generator
pub struct CertGenerator {
    // Generates dynamic certificates on-demand
}
```

## Usage

### CLI Usage

```bash
# Start proxy in monitor mode
slapper proxy --listen 127.0.0.1:8080

# Start proxy in intercept mode
slapper proxy --listen 127.0.0.1:8080 --mode intercept

# With custom rules
slapper proxy --listen 127.0.0.1:8080 --rules rules.yaml
```

### API Usage

```rust
use slapper::proxy::intercept::{ProxyServer, InterceptMode, InterceptRule, RuleAction};

let proxy = ProxyServer::new("127.0.0.1:8080".parse()?)?
    .with_mode(InterceptMode::Intercept);

proxy.add_rule(InterceptRule {
    name: "block-login".to_string(),
    pattern: ".*login.*".to_string(),
    action: RuleAction::Log,
});

proxy.start().await?;
```

## Certificate Handling

The proxy generates certificates dynamically using `rcgen`. For HTTPS interception to work:

1. Client must trust the proxy CA certificate
2. Certificates are generated on-demand per target domain
3. Certificate is valid for the specific target only

### Trusting the CA

```bash
# Export CA cert
slapper proxy export-ca --file ca.crt

# Import to system trust store (requires root)
cp ca.crt /usr/local/share/ca-certificates/slapper-proxy.crt
update-ca-certificates
```

## Rule Configuration

```yaml
rules:
  - name: Log SQL injection attempts
    pattern: ".*(\\bunion\\b|\\bselect\\b).*"
    action: log
  - name: Block known malicious
    pattern: ".*evil\\.example\\.com.*"
    action: block
  - name: Allow health checks
    pattern: ".*/health.*"
    action: allow

default_action: allow
```

## Triggers

Keywords that activate this skill: `intercept proxy`, `mitm`, `ssl intercept`, `dynamic certificate`, `http proxy`, `traffic interception`, `mitmproxy`, `ssl MITM`
