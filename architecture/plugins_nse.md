# Plugins & NSE Integration

Slapper is designed to be highly extensible through its plugin system and full integration with the Nmap Scripting Engine (NSE).

## Plugin System (`slapper-plugin` & `slapper-ruby`)

The plugin system allows developers to extend Slapper's capabilities using high-level languages like Python and Ruby.

### Python Plugins

- **Integration**: Uses `pyo3` to bridge between Rust and Python.
- **Capabilities**: Python plugins can implement custom scanners, fuzzer mutators, or output formatters.
- **Example**: See `examples/plugins/example_scanner.py`.

### Ruby Plugins

- **Integration**: Managed via the `slapper-ruby` crate.
- **Metasploit Integration**: Provides a bridge to Metasploit RPC, allowing Slapper to trigger Metasploit modules directly.
- **Example**: See `examples/plugins/metasploit_example.rb`.

## NSE (Nmap Scripting Engine) Integration (`slapper-nse`)

Slapper includes a full-featured Lua interpreter (via `mlua`) that can run standard Nmap NSE scripts.

### Core Features

- **Compatibility**: Supports a vast majority of existing NSE scripts.
- **Sandbox (`nse-sandbox` feature)**: Optionally restricts dangerous Lua operations (e.g., file system access, network connections) for safer execution of untrusted scripts.
- **NSE Tool (`src/nse_tool.rs`)**: Provides a high-level API for running NSE scripts against targets discovered by Slapper.

### Benefits

- **Instant Capability**: Access to thousands of community-developed security checks from day one.
- **Lua Scripting**: Simple and familiar scripting language for custom security logic.
- **Seamless Integration**: NSE results are integrated into Slapper's finding management and reporting system.
