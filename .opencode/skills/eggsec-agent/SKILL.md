# Eggsec Agent Skill

Agent-specific workflows and patterns for autonomous security testing.

## Overview

This skill directory contains specialized guides for the Eggsec autonomous agent system. Each file covers a specific domain or capability.

## Available Skill Files

### Core Agent Capabilities
- `eggsec-agent.md` - Main agent architecture and workflow
- `autonomous_security_agent.md` - Autonomous agent patterns
- `agent_observability.md` - Agent monitoring and observability
- `agent_thread_safety.md` - Thread safety patterns for agents

### Security Testing Domains
- `sql_injection_fuzzing.md` - SQL injection testing
- `cross_site_scripting.md` - XSS testing
- `command_injection_testing.md` - Command injection testing
- `path_traversal_testing.md` - Path traversal testing
- `ssrf_testing.md` - SSRF testing
- `oauth_oidc_testing.md` - OAuth/OIDC testing
- `graphql_security_testing.md` - GraphQL security
- `ldap_injection_testing.md` - LDAP injection
- `nosql_injection.md` - NoSQL injection
- `xpath_injection.md` - XPath injection
- `expression_injection.md` - Expression injection
- `formula_injection_testing.md` - Formula injection
- `log_injection_testing.md` - Log injection
- `prototype_pollution.md` - Prototype pollution
- `mass_assignment.md` - Mass assignment testing
- `race_condition.md` - Race condition testing
- `cors_security_testing.md` - CORS testing
- `websocket_api.md` - WebSocket security
- `dns_reconnaissance.md` - DNS recon
- `subdomain_enumeration.md` - Subdomain enumeration
- `port_scanning.md` - Port scanning
- `endpoint_discovery.md` - Endpoint discovery
- `http_load_testing.md` - Load testing
- `waf_detection_bypass.md` - WAF testing
- `ssl_tls_audit.md` - SSL/TLS auditing
- `vulnerability_management.md` - Vulnerability management
- `security_compliance_checks.md` - Compliance checks
- `security_assessment_pipeline.md` - Security pipelines

### Technical Implementation
- `fuzz_core.md` - Fuzzing engine internals
- `nse_sandbox.md` - NSE script sandboxing
- `grpc_implementation.md` - gRPC implementation
- `websocket_api.md` - WebSocket API
- `intercepting_proxy.md` - Intercepting proxy
- `oast_integration.md` - OAST integration
- `mcp_protocol.md` - MCP protocol

### Code Quality & Patterns
- `code_quality_patterns.md` - Code quality guidelines
- `security_fix_patterns.md` - Security fix patterns
- `performance_patterns.md` - Performance optimization
- `rust_dependency_migration.md` - Dependency migration

### Infrastructure
- `config_management.md` - Configuration management
- `template_based_scanning.md` - Template-based scanning
- `wireless_security_testing.md` - Wireless security
- `web_search_integration.md` - Web search integration
- `alert_notification.md` - Alert notifications

### TUI Components
- `tui_improvements.md` - TUI improvement patterns
- `tui_tab_indexing.md` - Tab indexing system
- `tui_theme_system.md` - Theme system
- `tui_session_persistence.md` - Session persistence
- `tui_clipboard.md` - Clipboard integration

### Planning
- `plan_improvement.md` - Plan improvement workflows

## Using These Skills

Load individual skill files as needed based on the task:
```bash
# Example: Load the SQL injection skill
# (Skills are loaded automatically when their patterns are detected)
```

## Resources
- `crates/eggsec/src/agent/AGENTS.override.md` - Agent module guidance
- `AGENTS.md` - General project guidelines
- `ARCHITECTURE.md` - Overall design
