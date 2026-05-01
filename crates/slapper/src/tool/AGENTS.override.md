# Tool Module Override

Specialized guidance for the tool abstraction layer.

## SecurityTool Trait

`tool/traits.rs:117` has `SecurityTool` trait for tool abstraction.

## ToolRegistry

`tool/registry.rs:9` has `ToolRegistry` for managing tool instances.

Feature-gated behind `tool-api` (enabled by `rest-api`, `grpc-api`, `nse`).

## Protocol Implementations

`tool/protocol/`:
- `mcp/` - MCP server (`handlers/server.rs`, `handlers/helpers.rs`)
- `openai/` - OpenAI-compatible chat completions
- `rest.rs` - REST API (scope validation implemented)
- `grpc.rs` - gRPC service

## Tool Implementations

`tool/implementations/` - Recon, scanner, fuzzer, waf, search, etc.