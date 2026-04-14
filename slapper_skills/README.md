# Slapper Skills

This directory contains skill files for the Slapper autonomous security agent system.

**Note:** These skills are for **using** Slapper's agent system, not for working on the Slapper codebase itself. The `AGENTS.md` file in the project root contains guidelines for AI agents working on the Slapper codebase.

## Purpose

Skills define how the autonomous agent should approach security testing workflows. Each skill contains:
- YAML frontmatter with metadata and trigger keywords
- Markdown body with capabilities, usage examples, and best practices

## Quick Start

```bash
# Build with agent support
cargo build --release --features "rest-api ai-integration"

# List available skills
./slapper agent skills list

# Load skills
./slapper agent skills load ./slapper_skills/

# Show skill details
./slapper agent skills show sql_injection_fuzzing
```

## Skills Index

### Reconnaissance
| Skill | Description | Tools |
|-------|-------------|-------|
| `dns_reconnaissance.md` | DNS lookup and enumeration | recon |
| `ssl_tls_analysis.md` | Certificate and TLS analysis | recon |
| `subdomain_enumeration.md` | Subdomain discovery | recon |
| `web_search_integration.md` | CVE and vulnerability research | search |

### Scanning
| Skill | Description | Tools |
|-------|-------------|-------|
| `port_scanning.md` | Network port and service scanning | scanner |
| `endpoint_discovery.md` | Web path and directory discovery | scanner |

### Fuzzing
| Skill | Description | Tools |
|-------|-------------|-------|
| `sql_injection_fuzzing.md` | SQL injection testing | fuzzer |
| `cross_site_scripting.md` | XSS vulnerability testing | fuzzer |
| `path_traversal_testing.md` | Path traversal and LFI/RFI | fuzzer |
| `ssrf_testing.md` | Server-Side Request Forgery | fuzzer |
| `command_injection_testing.md` | OS command injection | fuzzer |
| `ldap_injection_testing.md` | LDAP injection testing | fuzzer |
| `formula_injection_testing.md` | CSV/spreadsheet formula injection | fuzzer |
| `log_injection_testing.md` | Log injection and falsification | fuzzer |

### API Testing
| Skill | Description | Tools |
|-------|-------------|-------|
| `graphql_security_testing.md` | GraphQL API security | fuzzer |
| `oauth_oidc_testing.md` | OAuth/OIDC security | fuzzer |
| `cors_security_testing.md` | CORS misconfiguration | fuzzer |
| `authentication_security_testing.md` | Auth mechanism testing | auth_test |

### Protection
| Skill | Description | Tools |
|-------|-------------|-------|
| `waf_detection_bypass.md` | WAF detection and bypass | waf |

### Load Testing
| Skill | Description | Tools |
|-------|-------------|-------|
| `http_load_testing.md` | Performance and stress testing | loadtest |

### Compliance
| Skill | Description | Tools |
|-------|-------------|-------|
| `security_compliance_checks.md` | Security header verification | compliance |

### Pipeline
| Skill | Description | Tools |
|-------|-------------|-------|
| `security_assessment_pipeline.md` | Full security assessment | pipeline |

### Agent
| Skill | Description | Tools |
|-------|-------------|-------|
| `autonomous_security_agent.md` | Agent configuration and usage | agent |

## Usage

### Loading Skills

```bash
# List all available skills
slapper agent skills list

# Load skills from this directory
slapper agent skills load /path/to/slapper_skills/

# Show specific skill details
slapper agent skills show sql_injection_fuzzing
```

### Agent Configuration

Agents use these skills to guide their behavior when interacting with targets. The skill triggers are matched against user input to activate appropriate capabilities.

## Format

Each skill follows the YAML + Markdown format:

```yaml
---
name: skill_name
description: "Brief description"
triggers:
  - trigger1
  - trigger2
metadata:
  category: category
  tools: [tool1, tool2]
  scope: targets
---

## Overview
<detailed description>

## Usage
<code examples>

## Triggers
Keywords that activate this skill
```

## Integration

Skills are loaded by the `SkillLoader` in `agent/skills.rs`. The `SkillRegistry` provides:
- `find_by_trigger()` - Match triggers to skills
- `find_by_tool()` - Find skills for specific tools
- `get_prompts_for_context()` - Generate context-aware prompts