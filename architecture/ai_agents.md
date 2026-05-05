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

Slapper can run as an autonomous agent that can receive high-level goals and plan its own security assessment.

- **Agent Runner (`mod.rs`)**: The core loop of the autonomous agent.
- **Planner (`planner.rs`)**: Translates high-level goals into a series of Slapper commands and stages.
- **Memory (`memory.rs`)**: Maintains long-term context about the target and past actions to avoid redundancy and improve strategy.
- **Skills (`skills.rs`)**: Represents discrete capabilities the agent can employ (e.g., "scan", "fuzz", "recon").

## MCP Integration

Slapper implements the **Model Context Protocol (MCP)**, allowing it to be used as a "tool" by other AI agents or integrated into larger AI-driven security platforms.
