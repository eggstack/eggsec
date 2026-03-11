# NSE Library Implementation Plan

## Overview

This document outlines the implementation plan for missing and incomplete NSE (Nmap Scripting Engine) libraries in the Slapper project.

## Current State Assessment

### Libraries Already Implemented (Solid)
- `bit` - Full bitwise operations
- `json` - Full JSON encode/decode
- `datafiles` - Good coverage of nmap-services, protocols, RPC, MAC prefixes
- `shortport` - Comprehensive port matching functions
- `unpwdb` - Username/password utilities
- `vulns` - CVE database access
- `bit` - Bitwise operations

### Libraries Implemented but Need Registration
The following modules exist but are NOT registered in `executor.rs`:
- `base64` 
- `base32`
- `bin`
- `datetime`
- `rand`
- `strbuf`
- `tab`
- `stringaux`
- `target`
- `creds`
- `url`
- `openssl`
- `pcre`
- `io`
- `os`
- `unittest`
- `nsedebug`
- `strict`
- `match_lib` (as `r#match`)
- Various protocol libraries (http2, mqtt, kafka, websocket, telnet, sftp, whois, finger)

---

## Phase 1: Critical Missing Libraries (HIGH PRIORITY)

### 1.1 Create `re` Module (Regex Library)

**Priority**: CRITICAL - Many NSE scripts depend on regex
**Location**: `src/nse/libraries/re.rs`

Nmap's `re` module provides Lua-friendly regex wrapper around POSIX regex or PCRE.

```rust
// Key functions to implement:
// re.match(str, pattern [, options]) -> table
// re.find(str, pattern [, options]) -> start, end, captures...
// re.gsub(str, pattern, replacement [, options]) -> newstr, count
// re.split(str, pattern [, options]) -> table
// re.compile(pattern [, options]) -> pattern
// re.version() -> string
```

### 1.2 Create `httpspider` Module

**Priority**: CRITICAL - Many HTTP scripts depend on it
**Location**: `src/nse/libraries/httpspider.rs`

The httpspider library provides web crawling and page parsing functionality.

```rust
// Key functions to implement:
// httpsprawl(url, options) -> enumerationider.c
// httpspider.fetch(url, options) -> response
// httpspider.parse(html) -> links, forms, etc.
// httpspider.allowed(code) -> bool
// httpspider.filter(link) -> bool
```

### 1.3 Create `lpeg` Module (Optional)

**Priority**: MEDIUM - Used by some parsing scripts
**Location**: `src/nse/libraries/lpeg.rs`

LPeg is a powerful pattern matching library. However, mlua has limited support. Consider using the regex module as alternative.

---

## Phase 2: Library Registration Fixes (HIGH PRIORITY)

### 2.1 Register Existing Libraries in Executor

Add the following to `src/nse/executor.rs` in `register_libraries()`:

```rust
// Add these registrations:
crate::nse::libraries::base64::register_base64_library(&self.lua)?;
crate::nse::libraries::base32::register_base32_library(&self.lua)?;
crate::nse::libraries::datetime::register_datetime_library(&self.lua)?;
crate::nse::libraries::rand::register_rand_library(&self.lua)?;
crate::nse::libraries::url::register_url_library(&self.lua)?;
crate::nse::libraries::creds::register_creds_library(&self.lua)?;
crate::nse::libraries::openssl::register_openssl_library(&self.lua)?;
crate::nse::libraries::pcre::register_pcre_library(&self.lua)?;
crate::nse::libraries::io::register_io_library(&self.lua)?;
crate::nse::libraries::os::register_os_library(&self.lua)?;
crate::nse::libraries::unittest::register_unittest_library(&self.lua)?;
crate::nse::libraries::target::register_target_library(&self.lua)?;
crate::nse::libraries::strbuf::register_strbuf_library(&self.lua)?;
crate::nse::libraries::tab::register_tab_library(&self.lua)?;
crate::nse::libraries::stringaux::register_stringaux_library(&self.lua)?;
```

### 2.2 Add Global Table Registrations

Add to `setup_globals()` in executor.rs:
```rust
globals.set("re", self.lua.create_table()?)?;
globals.set("httpspider", self.lua.create_table()?)?;
globals.set("base64", self.lua.create_table()?)?;
globals.set("base32", self.lua.create_table()?)?;
globals.set("datetime", self.lua.create_table()?)?;
globals.set("rand", self.lua.create_table()?)?;
globals.set("url", self.lua.create_table()?)?;
globals.set("creds", self.lua.create_table()?)?;
globals.set("openssl", self.lua.create_table()?)?;
globals.set("pcre", self.lua.create_table()?)?;
globals.set("io", self.lua.create_table()?)?;
globals.set("os", self.lua.create_table()?)?;
globals.set("unittest", self.lua.create_table()?)?;
globals.set("target", self.lua.create_table()?)?;
```

### 2.3 Update mod.rs

Add to `src/nse/libraries/mod.rs`:
```rust
pub mod re;          // NEW
pub mod httpspider;  // NEW
pub mod lpeg;       // NEW (optional)
```

---

## Phase 3: Performance Improvements (MEDIUM PRIORITY)

### 3.1 Async HTTP Library

**Current Issue**: `http.rs` uses blocking `reqwest::blocking::Client`

**Solution**: Implement async HTTP using `reqwest::Client`

```rust
// Add async versions of HTTP functions:
http.set(
    "async_get",
    lua.create_async_function(|lua, (host, port, path): (String, u16, String)| {
        async move {
            // async HTTP implementation
        }
    })?,
)?;
```

### 3.2 Connection Pooling

**Current Issue**: New HTTP client created per request

**Solution**: Use static connection pool

```rust
static HTTP_CLIENT: once_cell::sync::Lazy<reqwest::Client> = 
    once_cell::sync::Lazy::new(|| {
        reqwest::Client::builder()
            .pool_max_idle_per_host(10)
            .build()
            .unwrap()
    });
```

### 3.3 Async Socket Support

**Current Issue**: `socket.rs` uses blocking I/O

**Solution**: Implement async socket wrapper using tokio

### 3.4 Script Bytecode Caching

**Current Issue**: Scripts recompiled on each require

**Solution**: Implement module-level bytecode cache

---

## Phase 4: Advanced Libraries (LOWER PRIORITY)

### 4.1 `target` Library

**Status**: Exists but needs registration
**Location**: `src/nse/libraries/target.rs`

Functions to ensure:
- `target.domainname()` -> string
- `target.hostname()` -> string  
- `target.ip()` -> string
- `target.address()` -> string

### 4.2 `io` / `os` Libraries

**Status**: Stubs need completion

### 4.3 Enhanced `unittest`

**Status**: Basic, needs completion
- `unittest.test()` - Run test suite
- `unittest.assert()` - Assertions
- `unittest.output_results()` - Test output

### 4.4 Protocol-Specific Libraries

These exist but may need enhancement:
- `ldap` - Stub only
- `snmp` - Basic only
- `mysql` - Basic only  
- `mssql` - Basic only
- `postgres` - Basic only

---

## Implementation Checklist

### Phase 1 Tasks
- [ ] Create `src/nse/libraries/re.rs` - Regex library
- [ ] Create `src/nse/libraries/httpspider.rs` - Web spider library
- [ ] Update `mod.rs` to include new modules
- [ ] Test with NSE scripts that use regex/httpspider

### Phase 2 Tasks
- [ ] Register base64, base32, datetime, rand in executor
- [ ] Register url, creds, openssl, pcre in executor
- [ ] Register io, os, unittest, target in executor
- [ ] Update setup_globals() for all new tables

### Phase 3 Tasks
- [ ] Implement async HTTP client in http.rs
- [ ] Add connection pooling (static client)
- [ ] Implement async socket wrapper
- [ ] Add bytecode caching for require()

### Phase 4 Tasks
- [ ] Enhance target library with full implementation
- [ ] Complete io/os libraries
- [ ] Enhance protocol libraries (ldap, snmp, etc.)

---

## Notes

1. **Testing**: After each phase, test with actual NSE scripts from nmap repository
2. **Lua Version**: mlua uses Lua 5.4, Nmap uses Lua 5.3 - watch for compatibility issues
3. **Dependencies**: Some new modules may require additional Rust crates (e.g., scraper for httpspider)
4. **Breaking Changes**: Upgrading mlua may require API changes
