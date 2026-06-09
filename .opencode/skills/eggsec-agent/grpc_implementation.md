---
name: grpc_implementation
description: gRPC server implementation status and patterns in Eggsec
triggers:
  - gRPC
  - tonic
  - protobuf
  - RPC
metadata:
  category: code_quality
  tools: [grpc]
  scope: implementation
---

## Overview

This skill documents the gRPC implementation status in Eggsec. The infrastructure is complete but the RPC methods are stubs.

## Current Status (2026-04-29)

**Infrastructure: COMPLETE**
- Proto definition: `tool/protocol/grpc.proto`
- Generated code: `generated/eggsec.tool.v1.rs` (~1000 lines)
- Server setup: `tool/protocol/grpc.rs:start_grpc_server()`
- Service implementation: `tool/protocol/grpc.rs:ToolServiceImpl`

**RPC Methods: STUBS** (not yet implemented)
- `list_tools` - Returns `Err(Status::unimplemented)`
- `get_tool` - Returns `Err(Status::unimplemented)`
- `execute_tool` - Returns `Err(Status::unimplemented)`
- `stream_execute_tool` - Returns `Err(Status::unimplemented)`
- `get_capabilities` - Returns `Err(Status::unimplemented)`

## Proto Definition

**Location**: `crates/eggsec/src/tool/protocol/grpc.proto`

**Service definition**:
```protobuf
service ToolService {
  rpc ListTools(ListToolsRequest) returns (ListToolsResponse);
  rpc GetTool(GetToolRequest) returns (ToolDefinition);
  rpc ExecuteTool(ExecuteToolRequest) returns (ExecuteToolResponse);
  rpc StreamExecuteTool(StreamExecuteToolRequest) returns (stream StreamingResponse);
  rpc GetCapabilities(CapabilitiesRequest) returns (CapabilitiesResponse);
}
```

## Implementation Pattern

**Current stub pattern** (in `tool/protocol/grpc.rs`):
```rust
#[tonic::async_trait]
impl ToolService for ToolServiceImpl {
    async fn list_tools(
        &self,
        request: Request<ListToolsRequest>,
    ) -> Result<Response<ListToolsResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn get_tool(
        &self,
        request: Request<GetToolRequest>,
    ) -> Result<Response<ToolDefinition>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    // ... similar for other methods
}
```

## To Implement

To complete the gRPC implementation:

1. **ListTools**: Query the `ToolRegistry` for all registered tools
2. **GetTool**: Look up specific tool by name from registry
3. **ExecuteTool**: Convert request to `ToolRequest`, execute via dispatcher, return `ToolResponse`
4. **StreamExecuteTool**: Use async streaming for progress updates
5. **GetCapabilities**: Return server capabilities (supported surfaces, features)

## CLI Integration

**Args** (from `cli/mod.rs`):
```rust
pub struct GrpcServerArgs {
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,
    #[arg(long, default_value = "50051")]
    pub port: u16,
    #[arg(long)]
    pub api_key: Option<String>,
}
```

**Handler** (from `commands/handlers/grpc.rs`):
```rust
pub async fn handle_grpc(ctx: &CommandContext, args: GrpcServerArgs) -> Result<()> {
    let service = GrpcService::new(registry, args.api_key);
    start_grpc_server(args.host, args.port, service).await
}
```

## Verification Commands

```bash
cargo build --release -p eggsec --features full
./eggsec grpc --help
```

## Related Skills

- `mcp_protocol` - MCP protocol implementation
- `rest_api` - REST API patterns