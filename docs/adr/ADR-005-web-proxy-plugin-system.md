# ADR-005: Web Proxy Plugin System

**Date**: 2026-06-13
**Status**: Accepted
**Deciders**: Eggsec core team

## Context

The web proxy supports HTTP/1, HTTP/2, WebSocket, and gRPC protocols via built-in handlers. As the proxy matures, users need to extend protocol handling for custom or proprietary protocols without modifying core code.

## Decision

Implement a trait-based plugin system (`ProtocolHandler` + `PluginRegistry`) that allows registering custom protocol handlers at runtime.

### Key Design Choices

1. **Trait-based, not dynamic loading**: Plugins are Rust types implementing `ProtocolHandler`. No `libloading`/FFI to avoid unsafety and platform concerns.
2. **Detection + Handling split**: `detect()` is called during protocol detection; `handle()` processes the matched traffic. This two-phase design allows cheap detection before expensive handling.
3. **Registry pattern**: `PluginRegistry` manages registration, lookup, and detection routing. Plugins are prioritized by detection confidence.
4. **Built-in example**: `NonStandardPortHandler` demonstrates the pattern and provides useful functionality out of the box.

### Trade-offs

- **Chosen over dynamic loading**: Avoids unsafe FFI, platform-specific `.so`/`.dylib` loading, and symbol resolution complexity. Users compile plugins into the binary.
- **Chosen over middleware chain**: More explicit than generic middleware; each plugin declares what it handles.
- **Deferred**: Dynamic loading from shared libraries (future phase if needed).

## Consequences

- Plugins must be compiled into the eggsec binary.
- Plugin registration happens at startup before proxy begins listening.
- Detection confidence determines which plugin claims ambiguous traffic.
- The `PluginFinding` type integrates with existing finding infrastructure.
