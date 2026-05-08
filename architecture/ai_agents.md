# AI & Agents Module

Slapper features deep integration with AI models for analysis, payload generation, and autonomous security testing via the Model Context Protocol (MCP).

## AI Integration (`src/ai/`)

### Adaptive Fuzzing (`adaptive.rs`)

Using AI to analyze target responses and adjust fuzzing strategies in real-time. This includes selecting more promising payloads or mutators based on observed feedback.

### Payload Generation (`payloads.rs`, `script_gen.rs`)

Generating complex, context-aware payloads (e.g., specific SQLi or XSS) that are more likely to bypass WAFs or trigger vulnerabilities in modern web applications.

### WAF Bypass Suggestions (`waf_bypass.rs`)

The AI can analyze detected WAF signatures and suggest novel bypass techniques that aren't part of the standard library.

### AI Client (`client.rs`)

An abstraction layer for interacting with different LLM providers (e.g., OpenAI, Anthropic) while handling rate limits, retries, and token management.

## Autonomous Agents (`src/agent/`)

Slapper can run as an autonomous scanning agent that executes configured schedules, enforces operational constraints, and handles alert routing.

- **Agent Runner (`mod.rs`)**: Core polling loop, scheduled scan dispatch, and event handling.
- **Memory (`memory.rs`)**: Maintains longitudinal context and baseline-aware finding comparisons.
- **Portfolio (`portfolio.rs`)**: Stores targets, schedules, and scan history metadata.
- **Constraints (`constraints/`)**: Enforces do-not-do rules, target restrictions, and scan/rate limits.
- **Skills (`skills.rs`)**: Represents discrete capabilities the agent can employ (e.g., "scan", "fuzz", "recon").

## MCP Integration

Slapper implements the **Model Context Protocol (MCP)**, allowing it to be used as a "tool" by other AI agents or integrated into larger AI-driven security platforms.
