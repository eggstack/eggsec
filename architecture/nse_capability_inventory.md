# NSE Capability Inventory

> **Milestone 3 Phase 05 Complete** — Complete inventory of side-effecting NSE Rust helper operations, classified by risk, blocking behavior, profile policy, accounting, cancellation, and reporting needs. All primary helper classes (filesystem, process, network TCP/UDP, DNS, time, randomness, environment, compression, crypto/TLS) are now migrated through `NseCapabilityContext`. This is the source-of-truth inventory that drives wrapper migration in later phases.

> **Milestone 4 complete.** Structured evidence reports are now extracted from capability events, compatibility diagnostics, rule evaluation, and script output. See `NseRunReport.evidence` and `docs/NSE_COMPATIBILITY.md`.

## Overview

The `eggsec-nse` crate provides Lua 5.4 script execution via `mlua`. Lua execution hooks can interrupt Lua bytecode, but once a Lua script enters Rust helper code, blocking filesystem, network, DNS, process, crypto, compression, time, or randomness work must enforce limits and cancellation cooperatively inside the helper path.

This inventory classifies every side-effecting helper operation across the 167 library implementation files in `crates/eggsec-nse/src/libraries/`, plus the executor core, to guide Milestone 3 wrapper migration.

### Capability Classes

| Class | Description |
|-------|-------------|
| `filesystem_read` | Read file contents or metadata |
| `filesystem_write` | Create, modify, delete, or rename files/directories |
| `process_exec` | Execute external commands or spawn processes |
| `network_tcp` | TCP socket operations (connect, send, receive) |
| `network_udp` | UDP socket operations (sendto, receive_from) |
| `dns_resolution` | DNS lookups via system or custom resolver |
| `tls_crypto` | TLS/SSL operations, certificate handling, crypto |
| `compression` | Gzip, deflate, zlib compression/decompression |
| `time_clock` | Wall-clock time reads, sleeps, timers |
| `randomness` | Random number/string generation |
| `environment` | Environment variable access |
| `pure_cpu` | Computation with no I/O side effects |

### Blocking Risk Levels

| Level | Description |
|-------|-------------|
| `none` | Pure computation, no blocking possible |
| `low` | Quick local operation (env read, time read, RNG) |
| `medium` | May block briefly (DNS lookup, small file read) |
| `high` | May block significantly (network I/O, file write, process exec, sleep) |

---

## 1. Process Execution

### CRITICAL — Requires wrapper migration first

| File | Function | Capability | Side Effect | Blocking Risk | Profile Policy | Accounting | Cancellation | Report Event | Notes |
|------|----------|------------|-------------|---------------|----------------|------------|--------------|--------------|-------|
| `libraries/io.rs:263-359` | `io.popen(cmd, mode)` | `process_exec` | ProcessExecution | high | `manual_allowed`, `agent_deny`, `ci_deny` | `process_operations` | **migrated** | `process_exec` | **Migrated (Phase 03)** — arbitrary command execution via `sh -c`; routed through `check_process_exec()` capability wrapper |
| `libraries/os.rs:145-156` | `os.execute(cmd)` | `process_exec` | ProcessExecution | high | `manual_allowed`, `agent_deny`, `ci_deny` | `process_operations` | needs check | `process_exec` | **Safe stub** — returns status=1, no real execution |
| `libraries/nmap.rs:715-729` | `nmap.is_admin()` | `process_exec` | ProcessExecution | medium | `manual_allowed`, `agent_allow_if_scoped`, `ci_allow_local_only` | `process_operations` | **migrated** | `process_exec` | **Migrated (Phase 03)** — executes `id -u` via `check_process_exec()` capability wrapper |
| `libraries/nmap.rs:1124-1139` | `nmap.is_privileged()` | `process_exec` | ProcessExecution | medium | `manual_allowed`, `agent_allow_if_scoped`, `ci_allow_local_only` | `process_operations` | **migrated** | `process_exec` | **Migrated (Phase 03)** — executes `id -u` via `check_process_exec()` capability wrapper |

---

## 2. Filesystem Write/Delete/Rename

### CRITICAL — Requires wrapper migration second

| File | Function | Capability | Side Effect | Blocking Risk | Profile Policy | Accounting | Cancellation | Report Event | Notes |
|------|----------|------------|-------------|---------------|----------------|------------|--------------|--------------|-------|
| `libraries/io.rs:48-123` | `io.open(filename, mode)` | `filesystem_write` | FileSystemWrite | medium | `manual_allowed`, `agent_deny`, `ci_allow_local_only` | `filesystem_operations`, `filesystem_bytes_written` | **migrated** | `fs_write` | **Migrated (Phase 03)** — modes "w", "a", "r+", "w+", "a+" create/modify files; routed through `check_fs_write()` capability wrapper |
| `libraries/io.rs:164-182` | `io.write(file, content)` | `filesystem_write` | FileSystemWrite | medium | `manual_allowed`, `agent_deny`, `ci_allow_local_only` | `filesystem_bytes_written` | **migrated** | `fs_write` | **Migrated (Phase 03)** — routed through `check_fs_write()` per-call, mitigating TOCTOU risk |
| `libraries/io.rs:362-402` | `io.tmpfile()` | `filesystem_write` | FileSystemWrite | medium | `manual_allowed`, `agent_deny`, `ci_allow_local_only` | `filesystem_operations` | **migrated** | `fs_write` | **Migrated (Phase 03)** — creates temp file; routed through `check_fs_write()` capability wrapper |
| `libraries/os.rs:159-178` | `os.remove(filename)` | `filesystem_write` | FileSystemWrite | medium | `manual_allowed`, `agent_deny`, `ci_allow_local_only` | `filesystem_operations` | **migrated** | `fs_write` | **Migrated (Phase 03)** — file deletion; routed through `check_fs_write()` capability wrapper |
| `libraries/os.rs:182-207` | `os.rename(old, new)` | `filesystem_write` | FileSystemWrite | medium | `manual_allowed`, `agent_deny`, `ci_allow_local_only` | `filesystem_operations` | **migrated** | `fs_write` | **Migrated (Phase 03)** — file rename; routed through `check_fs_write()` capability wrapper |
| `libraries/os.rs:216-236` | `os.chdir(path)` | `filesystem_write` | FileSystemWrite | low | `manual_allowed`, `agent_deny`, `ci_allow_local_only` | `filesystem_operations` | needs check | `fs_write` | Changes process working directory; `get_allowed_path()` check |
| `libraries/lfs.rs:146-162` | `lfs.mkdir(path)` | `filesystem_write` | FileSystemWrite | low | `manual_allowed`, `agent_deny`, `ci_allow_local_only` | `filesystem_operations` | **migrated** | `fs_write` | **Migrated (Phase 03)** — directory creation; routed through `check_fs_write()` capability wrapper |
| `libraries/lfs.rs:166-182` | `lfs.rmdir(path)` | `filesystem_write` | FileSystemWrite | low | `manual_allowed`, `agent_deny`, `ci_allow_local_only` | `filesystem_operations` | **migrated** | `fs_write` | **Migrated (Phase 03)** — directory removal; routed through `check_fs_write()` capability wrapper |
| `libraries/lfs.rs:186-202` | `lfs.remove(path)` | `filesystem_write` | FileSystemWrite | low | `manual_allowed`, `agent_deny`, `ci_allow_local_only` | `filesystem_operations` | **migrated** | `fs_write` | **Migrated (Phase 03)** — file removal; routed through `check_fs_write()` capability wrapper |
| `libraries/lfs.rs:206-227` | `lfs.rename(old, new)` | `filesystem_write` | FileSystemWrite | low | `manual_allowed`, `agent_deny`, `ci_allow_local_only` | `filesystem_operations` | **migrated** | `fs_write` | **Migrated (Phase 03)** — rename; routed through `check_fs_write()` capability wrapper |
| `libraries/lfs.rs:231-264` | `lfs.link(source, link, symbolic)` | `filesystem_write` | FileSystemWrite | low | `manual_allowed`, `agent_deny`, `ci_allow_local_only` | `filesystem_operations` | **migrated** | `fs_write` | **Migrated (Phase 03)** — link creation; routed through `check_fs_write()` capability wrapper |
| `libraries/lfs.rs:298-321` | `lfs.touch(path)` | `filesystem_write` | FileSystemWrite | low | `manual_allowed`, `agent_deny`, `ci_allow_local_only` | `filesystem_operations` | **migrated** | `fs_write` | **Migrated (Phase 03)** — create empty file if missing; routed through `check_fs_write()` capability wrapper |
| `libraries/lfs.rs:336-358` | `lfs.set_mode(path, mode)` | `filesystem_write` | FileSystemWrite | low | `manual_allowed`, `agent_deny`, `ci_allow_local_only` | `filesystem_operations` | **migrated** | `fs_write` | **Migrated (Phase 03)** — chmod equivalent; routed through `check_fs_write()` capability wrapper |

---

## 3. Filesystem Read

### HIGH — Requires wrapper migration third

| File | Function | Capability | Side Effect | Blocking Risk | Profile Policy | Accounting | Cancellation | Report Event | Notes |
|------|----------|------------|-------------|---------------|----------------|------------|--------------|--------------|-------|
| `libraries/io.rs:138-162` | `io.read(file, size)` | `filesystem_read` | FileSystemRead | medium | `manual_allowed`, `agent_allow_if_scoped`, `ci_allow_local_only` | `filesystem_bytes_read` | **migrated** | `fs_read` | **Migrated (Phase 03)** — routed through `check_fs_read()` per-call, mitigating TOCTOU risk |
| `libraries/io.rs:240-260` | `io.lines(filename)` | `filesystem_read` | FileSystemRead | medium | `manual_allowed`, `agent_allow_if_scoped`, `ci_allow_local_only` | `filesystem_bytes_read` | **migrated** | `fs_read` | **Migrated (Phase 03)** — reads entire file; routed through `check_fs_read()` capability wrapper |
| `libraries/lfs.rs:46-111` | `lfs.attributes(path)` | `filesystem_read` | FileSystemRead | low | `manual_allowed`, `agent_allow_if_scoped`, `ci_allow_local_only` | `filesystem_operations` | **migrated** | `fs_read` | **Migrated (Phase 03)** — file metadata; routed through `check_fs_read()` capability wrapper |
| `libraries/lfs.rs:115-142` | `lfs.dir(path)` | `filesystem_read` | FileSystemRead | low | `manual_allowed`, `agent_allow_if_scoped`, `ci_allow_local_only` | `filesystem_operations` | **migrated** | `fs_read` | **Migrated (Phase 03)** — directory listing; routed through `check_fs_read()` capability wrapper |
| `libraries/lfs.rs:267-274` | `lfs.currentdir()` | `filesystem_read` | FileSystemRead | low | `manual_allowed`, `agent_allow_if_scoped`, `ci_allow_local_only` | none | none | `fs_read` | Returns cwd; no sandbox check |
| `libraries/lfs.rs:362-395` | `lfs.symlinkattributes(path)` | `filesystem_read` | FileSystemRead | low | `manual_allowed`, `agent_allow_if_scoped`, `ci_allow_local_only` | `filesystem_operations` | needs check | `fs_read` | Symlink metadata; `get_allowed_path()` check |
| `libraries/unpwdb.rs` | `unpwdb.*` | `filesystem_read` | FileSystemRead | medium | `manual_allowed`, `agent_deny`, `ci_allow_local_only` | `filesystem_bytes_read` | needs check | `fs_read` | Reads credential files |
| `libraries/creds.rs` | `creds.*` | `filesystem_read` | FileSystemRead | medium | `manual_allowed`, `agent_deny`, `ci_allow_local_only` | `filesystem_bytes_read`, `network_operations` | needs check | `fs_read` | Credential management; also network |
| `libraries/datafiles.rs` | `datafiles.*` | `filesystem_read` | FileSystemRead | medium | `manual_allowed`, `agent_allow_if_scoped`, `ci_allow_local_only` | `filesystem_bytes_read` | needs check | `fs_read` | Data file reading |
| `libraries/nmap.rs:1246-1259` | `nmap.fetchfile(filename)` | `filesystem_read` | FileSystemRead | medium | `manual_allowed`, `agent_allow_if_scoped`, `ci_allow_local_only` | `filesystem_bytes_read` | needs check | `fs_read` | Searches nmap paths |

---

## 4. Network TCP/UDP

### HIGH — Requires wrapper migration fourth

| File | Function | Capability | Side Effect | Blocking Risk | Profile Policy | Accounting | Cancellation | Report Event | Notes |
|------|----------|------------|-------------|---------------|----------------|------------|--------------|--------------|-------|
| `libraries/socket.rs` (all) | `socket.tcp()`, `socket.udp()`, `socket.sctp()` | `network_tcp`, `network_udp` | NetworkAccess | high | `manual_allowed`, `agent_allow_if_scoped`, `ci_deny` | `network_operations`, `network_bytes_read`, `network_bytes_written` | **migrated** | `net_connect` | **Migrated (Phase 04)** — capability context routes connect/send/receive through `nse_network_tcp_connect`, `nse_network_tcp_send`, `nse_network_tcp_receive`, `nse_network_udp_send`, `nse_network_udp_receive` |
| `libraries/socket.rs` | `socket.tcp_connect()`, `socket.connect()` | `network_tcp` | NetworkAccess | high | `manual_allowed`, `agent_allow_if_scoped`, `ci_deny` | `network_operations` | **migrated** | `net_connect` | **Migrated (Phase 04)** — routed through `nse_network_tcp_connect` capability wrapper |
| `libraries/socket.rs` | `socket.send()`, `socket.receive()` | `network_tcp` | NetworkAccess | high | `manual_allowed`, `agent_allow_if_scoped`, `ci_deny` | `network_bytes_read`, `network_bytes_written` | **migrated** | `net_io` | **Migrated (Phase 04)** — routed through `nse_network_tcp_send`/`nse_network_tcp_receive` |
| `libraries/socket.rs` | `socket.sendto()`, `socket.receive_from()` | `network_udp` | NetworkAccess | high | `manual_allowed`, `agent_allow_if_scoped`, `ci_deny` | `network_bytes_read`, `network_bytes_written` | **migrated** | `net_io` | **Migrated (Phase 04)** — routed through `nse_network_udp_send`/`nse_network_udp_receive` |
| `libraries/socket.rs:664-704` | `socket.resolve_async()` | `dns_resolution` | NetworkAccess | medium | `manual_allowed`, `agent_allow_if_scoped`, `ci_deny` | `network_operations` | **migrated** | `net_resolve` | **Migrated (Phase 04)** — routed through `nse_dns_lookup` capability wrapper |
| `libraries/nmap.rs:306-387` | `nmap.socket_connect()`, `nmap.socket_send()`, `nmap.socket_receive()`, `nmap.socket_close()` | `network_tcp` | NetworkAccess | high | `manual_allowed`, `agent_allow_if_scoped`, `ci_deny` | `network_operations`, `network_bytes_read`, `network_bytes_written` | needs check | `net_connect` | **NO sandbox check** — connection registry bypass |
| `libraries/nmap.rs:1271-1451` | `nmap.async_socket_connect()`, `nmap.async_socket_send()`, `nmap.async_socket_receive()` | `network_tcp` | NetworkAccess | high | `manual_allowed`, `agent_allow_if_scoped`, `ci_deny` | `network_operations`, `network_bytes_read`, `network_bytes_written` | needs check | `net_connect` | **NO sandbox check** — async socket bypass |
| `libraries/comm.rs` (all) | `comm.get_banner()`, `comm.exchange()`, `comm.tryssl()` | `network_tcp` | NetworkAccess | high | `manual_allowed`, `agent_allow_if_scoped`, `ci_deny` | `network_operations`, `network_bytes_read`, `network_bytes_written` | **migrated** | `net_connect` | **Migrated (Phase 04)** — routed through `nse_network_tcp_connect`, `nse_network_tcp_send`, `nse_network_tcp_receive` capability wrappers |
| `libraries/comm.rs` (all) | `comm.get_banner_async()`, `comm.exchange_async()` | `network_tcp` | NetworkAccess | high | `manual_allowed`, `agent_allow_if_scoped`, `ci_deny` | `network_operations`, `network_bytes_read`, `network_bytes_written` | needs check | `net_connect` | **NO sandbox check** — async banner grabbing (not yet migrated) |
| `libraries/http.rs` (all) | `http.get()`, `http.post()`, `http.put()`, `http.delete()`, `http.head()`, `http.options()`, `http.request()` | `network_tcp` | NetworkAccess | high | `manual_allowed`, `agent_allow_if_scoped`, `ci_deny` | `network_operations`, `network_bytes_read`, `network_bytes_written` | needs check | `net_http` | **NO sandbox check** — full HTTP client |
| `libraries/http.rs` (all) | `http.post_host()`, `http.put_data()` | `network_tcp` | NetworkAccess | high | `manual_allowed`, `agent_allow_if_scoped`, `ci_deny` | `network_operations`, `network_bytes_read`, `network_bytes_written` | needs check | `net_http` | **NO sandbox check** |
| `libraries/http.rs` (all) | `http.async_get()`, `http.async_post()`, `http.async_request()` | `network_tcp` | NetworkAccess | high | `manual_allowed`, `agent_allow_if_scoped`, `ci_deny` | `network_operations`, `network_bytes_read`, `network_bytes_written` | needs check | `net_http` | **NO sandbox check** — async HTTP |
| `libraries/smtp.rs` (all) | SMTP operations | `network_tcp` | NetworkAccess | high | `manual_allowed`, `agent_allow_if_scoped`, `ci_deny` | `network_operations`, `network_bytes_read`, `network_bytes_written` | needs check | `net_connect` | **NO sandbox check** — SMTP client |
| `libraries/ssh2.rs` (all) | SSH2 session, auth, channel ops | `network_tcp` | NetworkAccess | high | `manual_allowed`, `agent_allow_if_scoped`, `ci_deny` | `network_operations`, `network_bytes_read`, `network_bytes_written` | needs check | `net_connect` | **NO sandbox check** — SSH2 client |
| Protocol libs (all) | mysql, postgres, mssql, redis, mongodb, ldap, snmp, smb, smb2, smbauth, rdp, vnc, ntp, memcached, imap, pop3, oracle, winrm, radius, sip, tftp, upnp, tns, afp, amqp, ajp, ncp, ndmp, nrpc, citrixxml, ospf, ike, ipp, coap, pgsql, iax2, drda, eigrp, giop, iscsi, jdwp, rsync, socks, rtsp, tn3270, xmpp, isns, membase, bitcoin, bittorrent, cassandra, dicom, knx, multicast, nbd, natpmp, proxy, srvloc, wsdd, xdmcp, bjnp, cvs, dnssd, eap, pppoe, rpcap, rmi, ipmi, irc, versant, omp2, gps, mobileme, ls, telnet, sftp, whois, finger, stun, elasticsearch, kafka, mqtt, websocket, dnsbl | `network_tcp` | NetworkAccess | high | `manual_allowed`, `agent_allow_if_scoped`, `ci_deny` | `network_operations`, `network_bytes_read`, `network_bytes_written` | needs check | `net_connect` | **NO sandbox check** — protocol-specific wrappers |

---

## 5. DNS Resolution

### MEDIUM — Requires wrapper migration fifth

| File | Function | Capability | Side Effect | Blocking Risk | Profile Policy | Accounting | Cancellation | Report Event | Notes |
|------|----------|------------|-------------|---------------|----------------|------------|--------------|--------------|-------|
| `libraries/dns.rs` (all) | `dns.resolve()`, `dns.reverse()`, `dns.query()`, `dns.forward()`, `dns.ptr()` | `dns_resolution` | NetworkAccess | medium | `manual_allowed`, `agent_allow_if_scoped`, `ci_deny` | `network_operations` | **migrated** | `net_resolve` | **Migrated (Phase 04)** — routed through `nse_dns_lookup` capability wrapper |

---

## 6. TLS/Crypto

### MEDIUM — Requires wrapper migration seventh

| File | Function | Capability | Side Effect | Blocking Risk | Profile Policy | Accounting | Cancellation | Report Event | Notes |
|------|----------|------------|-------------|---------------|----------------|------------|--------------|--------------|-------|
| `libraries/ssl.rs` (all) | SSL/TLS connection setup, certificate handling | `tls_crypto` | NetworkAccess | medium | `manual_allowed`, `agent_allow_if_scoped`, `ci_deny` | `network_operations`, `crypto_operations` | needs check | `tls_handshake` | Requires openssl dep; cipher suite enumeration stubbed |
| `libraries/openssl.rs` (all) | OpenSSL crypto operations | `tls_crypto` | NetworkAccess | medium | `manual_allowed`, `agent_allow_if_scoped`, `ci_allow_local_only` | `crypto_operations` | needs check | `crypto_op` | OpenSSL crypto bindings |

---

## 7. Compression

### MEDIUM — Requires wrapper migration sixth

| File | Function | Capability | Side Effect | Blocking Risk | Profile Policy | Accounting | Cancellation | Report Event | Notes |
|------|----------|------------|-------------|---------------|----------------|------------|--------------|--------------|-------|
| Libraries using flate2/zlib | Gzip/deflate compression/decompression | `compression` | none | low | `manual_allowed`, `agent_allow_if_scoped`, `ci_allow_local_only` | `compression_bytes_in/out` | none | `compression` | Pure CPU; no blocking on small inputs |

---

## 8. Environment Access

### LOW — Requires wrapper migration eighth

| File | Function | Capability | Side Effect | Blocking Risk | Profile Policy | Accounting | Cancellation | Report Event | Notes |
|------|----------|------------|-------------|---------------|----------------|------------|--------------|--------------|-------|
| `libraries/os.rs:103-110` | `os.getenv(name)` | `environment` | EnvAccess | low | `manual_allowed`, `agent_allow_if_scoped`, `ci_allow_local_only` | none | none | `env_read` | Sandboxed — reads NSE_ENV only |
| `libraries/os.rs:114-127` | `os.setenv(name, value)` | `environment` | EnvAccess | low | `manual_allowed`, `agent_deny`, `ci_deny` | none | none | `env_write` | Blocked in sandbox |
| `libraries/os.rs:130-142` | `os.unsetenv(name)` | `environment` | EnvAccess | low | `manual_allowed`, `agent_deny`, `ci_deny` | none | none | `env_write` | Blocked in sandbox |
| `libraries/os.rs:209-212` | `os.getcwd()` | `environment` | EnvAccess | low | `manual_allowed`, `agent_allow_if_scoped`, `ci_allow_local_only` | none | none | `env_read` | No sandbox check |
| `libraries/os.rs:314-316` | `os.tmpdir()` | `environment` | EnvAccess | low | `manual_allowed`, `agent_allow_if_scoped`, `ci_allow_local_only` | none | none | `env_read` | No sandbox check |
| `libraries/os.rs:318-323` | `os.hostname()` | `environment` | EnvAccess | low | `manual_allowed`, `agent_allow_if_scoped`, `ci_allow_local_only` | none | none | `env_read` | No sandbox check |

---

## 9. Time/Blocking

### LOW — Requires wrapper migration eighth

| File | Function | Capability | Side Effect | Blocking Risk | Profile Policy | Accounting | Cancellation | Report Event | Notes |
|------|----------|------------|-------------|---------------|----------------|------------|--------------|--------------|-------|
| `libraries/stdnse.rs:172-191` | `stdnse.sleep(seconds)` | `time_clock` | none | high | `manual_allowed`, `agent_deny`, `ci_deny` | none | **needs check** | `sleep` | **Blocks the thread** — no cancellation checks |
| `libraries/stdnse.rs:172-191` | `stdnse.usleep(useconds)` | `time_clock` | none | high | `manual_allowed`, `agent_deny`, `ci_deny` | none | **needs check** | `sleep` | **Blocks the thread** — no cancellation checks |
| `libraries/stdnse.rs:172-191` | `stdnse.nsleep(nanoseconds)` | `time_clock` | none | high | `manual_allowed`, `agent_deny`, `ci_deny` | none | **needs check** | `sleep` | **Blocks the thread** — no cancellation checks |
| `libraries/stdnse.rs:178-185` | `stdnse.clock()`, `stdnse.get_time()` | `time_clock` | none | none | `manual_allowed`, `agent_allow_if_scoped`, `ci_allow_local_only` | none | none | none | Time reads; non-blocking |
| `libraries/os.rs:238-305` | `os.clock()`, `os.date()`, `os.time()`, `os.difftime()` | `time_clock` | none | none | `manual_allowed`, `agent_allow_if_scoped`, `ci_allow_local_only` | none | none | none | Time reads; non-blocking |

---

## 10. Randomness

### LOW — Requires wrapper migration eighth

| File | Function | Capability | Side Effect | Blocking Risk | Profile Policy | Accounting | Cancellation | Report Event | Notes |
|------|----------|------------|-------------|---------------|----------------|------------|--------------|--------------|-------|
| `libraries/rand.rs` (all) | `rand.*` | `randomness` | none | none | `manual_allowed`, `agent_allow_if_scoped`, `ci_allow_local_only` | none | none | none | Random generation; non-blocking |
| `libraries/stdnse.rs:259-269` | `stdnse.random_string()` | `randomness` | none | none | `manual_allowed`, `agent_allow_if_scoped`, `ci_allow_local_only` | none | none | none | Random string; non-blocking |
| `libraries/stdnse.rs:1018-1024` | `stdnse.urandom()` | `randomness` | none | low | `manual_allowed`, `agent_allow_if_scoped`, `ci_allow_local_only` | none | none | none | Reads /dev/urandom; brief blocking |

---

## 11. Pure CPU — No Wrapper Migration Needed

These libraries perform pure computation with no I/O side effects. They require no wrapper migration.

| Library Category | Libraries | Notes |
|------------------|-----------|-------|
| String/Text | `tab`, `stringaux`, `strbuf`, `nse_string`, `nse_table`, `match_lib`, `matchs`, `unicode`, `url` | Pure string manipulation |
| Encoding | `base64`, `base32`, `bin`, `bit` | Encoding/decoding; no I/O |
| Math/Data | `json`, `pcre`, `datetime`, `shortport`, `lpeg`, `lpeg_utility`, `asn1`, `idna`, `punycode`, `tableaux`, `strict`, `nsedebug`, `unittest`, `outlib`, `listop`, `bits`, `formulas`, `geoip` | Pure computation |
| NSE Core | `stdnse` (non-sleep functions), `nmap` (non-network functions) | Formatting, output, state (non-blocking parts) |

---

## Sandbox State Summary

### Already Sandboxed (4 libraries)

| Library | Enforcement | Gap |
|---------|-------------|-----|
| `socket` | `allowed_networks` check on every connect/resolve | `nmap.socket_*()` and `nmap.async_socket_*()` bypass |
| `io` | `get_allowed_path()` on open/lines; `is_command_allowed()` on popen; capability context checks via `check_fs_read()`, `check_fs_write()`, `check_process_exec()` | `io.write()` TOCTOU risk mitigated by per-call checks; `io.read()` TOCTOU risk mitigated |
| `os` | `get_allowed_path()` on remove/rename/chdir; capability context checks via `check_fs_write()`; sandbox blocks setenv/unsetenv | `os.execute()` is safe stub; `nmap.is_admin()` now routed through `check_process_exec()` |
| `lfs` | `get_allowed_path()` on all operations; capability context checks via `check_fs_read()`, `check_fs_write()` | None significant |

### Migrated to Capability Context (Phase 03–05)

| Library | Operations | Wrapper Used | Phase |
|---------|-----------|--------------|-------|
| `io.rs` | `io.open()`, `io.read()`, `io.lines()`, `io.popen()`, `io.tmpfile()`, `io.write()` | `check_fs_read()`, `check_fs_write()`, `check_process_exec()` + executing wrappers | Phase 03 |
| `lfs.rs` | All `lfs.*` operations | `check_fs_read()`, `check_fs_write()` via `NseCapabilityContext::check_capability()` | Phase 03 |
| `os.rs` | `os.remove()`, `os.rename()` | `check_fs_write()` | Phase 03 |
| `nmap.rs` | `nmap.is_admin()`, `nmap.is_privileged()` | `check_process_exec()` | Phase 03 |
| `socket.rs` | `socket.tcp_connect()`, `socket.connect()`, `socket.connect_udp()`, `socket.send()`, `socket.receive()`, `socket.sendto()`, `socket.receive_from()` | `nse_network_tcp_connect`, `nse_network_tcp_send`, `nse_network_tcp_receive`, `nse_network_udp_send`, `nse_network_udp_receive`, `check_network_udp` | Phase 04 |
| `comm.rs` | `comm.get_banner()`, `comm.exchange()`, `comm.tryssl()` | `nse_network_tcp_connect`, `nse_network_tcp_send`, `nse_network_tcp_receive` | Phase 04 |
| `dns.rs` | `dns.resolve()`, `dns.query()`, `dns.forward()`, `dns.ptr()` | `nse_dns_lookup` | Phase 04 |
| `datetime.rs` | `datetime.now()`, `datetime.clock()`, `datetime.date()`, `datetime.time()` | `nse_time_now`, `check_time_clock` | Phase 05 |
| `rand.rs` | `rand.random()`, `rand.num_range()`, `rand.random_string()`, `rand.seed()` | `nse_random_bytes`, `check_randomness` | Phase 05 |
| `openssl.rs` | OpenSSL crypto operations, certificate handling | `check_crypto` | Phase 05 |
| `tls.rs` | TLS connection setup, cipher suite operations | `check_crypto` | Phase 05 |
| `sslcert.rs` | SSL certificate parsing and validation | `check_crypto` | Phase 05 |
| `zlib.rs` | `zlib.compress()`, `zlib.decompress()` | `nse_compress`, `nse_decompress`, `check_compression` | Phase 05 |

### NOT Sandboxed (all others)

All network protocol libraries (http, smtp, ssh2, mysql, postgres, etc.) and the `nmap` library's own socket operations bypass sandbox checks entirely. `socket.rs`, `comm.rs`, and `dns.rs` are now migrated to capability context (Phase 04) but remain in the "already sandboxed" category for legacy checks. The `stdnse.sleep()` family blocks the thread without cancellation checks.

---

## Risk Classification

| Risk Level | Helpers | Count |
|------------|---------|-------|
| **CRITICAL** | `io.popen()`, `nmap.is_admin()`, `nmap.is_privileged()` | 3 functions |
| **HIGH** | Filesystem write/delete/rename (io.open, io.write, os.remove, os.rename, os.chdir, lfs.*), Network TCP/UDP (socket.*, nmap.socket_*, comm.*, http.*, all protocol libs) | ~150+ functions |
| **MEDIUM** | Filesystem read (io.read, io.lines, lfs.*, unpwdb, creds, datafiles, nmap.fetchfile), DNS resolution (dns.*), TLS/crypto (ssl.*, openssl.*) | ~30 functions |
| **LOW** | Environment (os.getenv, os.setenv, os.getcwd, os.tmpdir, os.hostname), Time (stdnse.sleep, os.time, os.date), Randomness (rand.*, stdnse.random_string, stdnse.urandom) | ~20 functions |
| **NONE** | Pure CPU (tab, json, base64, base32, bin, bit, stringaux, strbuf, etc.) | ~100+ functions |

---

## Profile Policy Mapping

### ManualPermissive

| Capability Class | Policy | Notes |
|------------------|--------|-------|
| `filesystem_read` | allow | Accounting only; no restriction |
| `filesystem_write` | allow | Accounting only; no restriction |
| `process_exec` | allow | `io.popen` sandboxed; `os.execute` stubbed |
| `network_tcp` | allow | `socket` sandboxed; protocol libs unsandboxed |
| `network_udp` | allow | `socket` sandboxed |
| `dns_resolution` | allow | No sandbox |
| `tls_crypto` | allow | No sandbox |
| `compression` | allow | No sandbox |
| `time_clock` | allow | Sleep allowed; no cancellation |
| `randomness` | allow | No sandbox |
| `environment` | allow | Sandboxed to NSE_ENV |

### ManualStrict

| Capability Class | Policy | Notes |
|------------------|--------|-------|
| `filesystem_read` | allow within roots | `get_allowed_path()` enforced |
| `filesystem_write` | allow within roots | `get_allowed_path()` enforced |
| `process_exec` | allow | `io.popen` sandboxed; `os.execute` stubbed |
| `network_tcp` | allow within CIDRs | `socket` sandboxed; protocol libs unsandboxed |
| `network_udp` | allow within CIDRs | `socket` sandboxed |
| `dns_resolution` | allow within CIDRs | No sandbox |
| `tls_crypto` | allow within CIDRs | No sandbox |
| `compression` | allow | No sandbox |
| `time_clock` | allow | Sleep allowed; no cancellation |
| `randomness` | allow | No sandbox |
| `environment` | allow | Sandboxed to NSE_ENV |

### AgentSafe

| Capability Class | Policy | Notes |
|------------------|--------|-------|
| `filesystem_read` | deny | Unscoped reads denied; scoped reads only (path under sandbox `allowed_dir` or explicit root) |
| `filesystem_write` | deny | Script files and FS modules denied |
| `process_exec` | deny | No process execution |
| `network_tcp` | allow if scoped | `socket` sandboxed; protocol libs unsandboxed |
| `network_udp` | allow if scoped | `socket` sandboxed |
| `dns_resolution` | allow if scoped | No sandbox |
| `tls_crypto` | allow if scoped | No sandbox |
| `compression` | allow | No sandbox |
| `time_clock` | deny sleep | Time reads allowed |
| `randomness` | allow | No sandbox |
| `environment` | allow | Sandboxed to NSE_ENV |

### CiSafe

| Capability Class | Policy | Notes |
|------------------|--------|-------|
| `filesystem_read` | deny | No filesystem access |
| `filesystem_write` | deny | No filesystem access |
| `process_exec` | deny | No process execution |
| `network_tcp` | deny | Zero network operations |
| `network_udp` | deny | Zero network operations |
| `dns_resolution` | deny | Zero network operations |
| `tls_crypto` | deny | Zero network operations |
| `compression` | allow | Local only |
| `time_clock` | deny sleep | Time reads allowed |
| `randomness` | allow | Local only |
| `environment` | allow | Sandboxed to NSE_ENV |

### CompatibilityLab

| Capability Class | Policy | Notes |
|------------------|--------|-------|
| `filesystem_read` | allow within multi-roots | Includes nmap paths |
| `filesystem_write` | allow within multi-roots | Includes nmap paths |
| `process_exec` | allow | `io.popen` sandboxed; `os.execute` stubbed |
| `network_tcp` | allow | Full access for compat testing |
| `network_udp` | allow | Full access |
| `dns_resolution` | allow | Full access |
| `tls_crypto` | allow | Full access |
| `compression` | allow | No sandbox |
| `time_clock` | allow | Sleep allowed |
| `randomness` | allow | No sandbox |
| `environment` | allow | Sandboxed to NSE_ENV |

---

## Accounting & Report Data Inventory

### Current NseRunReport Fields (Milestone 2)

| Report Field | Current Coverage | Gap |
|--------------|------------------|-----|
| `stats.network_operations` | Socket connect/resolve only | Protocol libs, http, comm, nmap.socket_* not counted |
| `stats.network_bytes_read` | Socket receive only | Protocol libs, http, comm not counted |
| `stats.network_bytes_written` | Socket send only | Protocol libs, http, comm not counted |
| `stats.filesystem_operations` | io.open, lfs.* operations | Not counted for reads outside sandbox |
| `stats.filesystem_bytes_read` | Not tracked | io.read, io.lines, unpwdb, creds, datafiles |
| `stats.filesystem_bytes_written` | Not tracked | io.write, io.tmpfile |
| `stats.limit_violation` | Wall-clock, instruction budget | No FS/network byte limits currently |
| `libraries` | Per-run require() activity | Not a capability snapshot (correct) |
| `rules` | Rule evaluation results | Complete |

### Accounting Needs by Helper

| Helper Family | Accounting Needed | Currently Tracked | Priority |
|---------------|-------------------|-------------------|----------|
| Process exec | `process_operations` | No | High |
| Filesystem write | `filesystem_operations`, `filesystem_bytes_written` | Partial (ops only) | High |
| Filesystem read | `filesystem_bytes_read` | No | Medium |
| Network TCP/UDP | `network_operations`, `network_bytes_read`, `network_bytes_written` | Partial (socket only) | High |
| DNS resolution | `network_operations` | No | Medium |
| TLS/crypto | `crypto_operations` | No | Low |
| Compression | `compression_bytes_in/out` | No | Low |
| Sleep/time | `time_clock` (blocking duration) | No | Medium |

---

## Helper Lifecycle State Machine

### Current State: Raw Lua → Rust Helper

```
Lua Script
  │
  ├─ Lua bytecode execution (interrupt hook fires between instructions)
  │
  └─ Rust helper call (NO cancellation checks inside helper)
       │
       ├─ Filesystem operation (get_allowed_path check if sandboxed)
       ├─ Network operation (is_host_allowed check if socket)
       ├─ Process execution (is_command_allowed check if popen)
       ├─ DNS resolution (no check)
       ├─ Sleep (no cancellation check — blocks thread)
       └─ Pure CPU (no blocking)
```

### Target State: Capability-Wrapped Helpers

```
Lua Script
  │
  ├─ Lua bytecode execution (interrupt hook fires between instructions)
  │
  └─ Capability Wrapper
       │
       ├─ Pre-flight checks
       │   ├─ Cancellation check (NseCancellationToken)
       │   ├─ Limit check (NseResourceCounters)
       │   └─ Profile policy check (NseNetworkPolicy, NseModulePolicy)
       │
       ├─ Accounting
       │   ├─ Increment operation counter
       │   ├─ Track bytes read/written
       │   └─ Emit report event
       │
       ├─ Execute actual operation
       │
       ├─ Post-flight checks
       │   ├─ Update resource counters
       │   └─ Check limit violations
       │
       └─ Error handling
            ├─ Limit exceeded → NseLimitViolation
            ├─ Cancellation requested → NseLimitViolation::ExplicitCancellation
            └─ Policy denied → NseLoadError::BlockedByPolicy
```

---

## Migration Priority Ranking

### Priority 1: Process Execution (CRITICAL)

**Target**: `io.popen()`, `nmap.is_admin()`, `nmap.is_privileged()`

**Rationale**: Arbitrary command execution is the highest-risk helper. `io.popen()` already has sandbox checks but needs cancellation and accounting. `nmap.is_admin()`/`nmap.is_privileged()` execute shell commands without any sandbox checks.

**Wrapper scope**:
- Cancellation check before `Command::new("sh")`
- Accounting increment for `process_operations`
- Profile policy enforcement for `nmap.is_admin()`/`nmap.is_privileged()`

### Priority 2: Filesystem Write/Delete/Rename (CRITICAL)

**Target**: `io.open()` (write modes), `io.write()`, `io.tmpfile()`, `os.remove()`, `os.rename()`, `os.chdir()`, `lfs.mkdir()`, `lfs.rmdir()`, `lfs.remove()`, `lfs.rename()`, `lfs.link()`, `lfs.touch()`, `lfs.set_mode()`

**Rationale**: Filesystem mutation is irreversible and high-risk. `io.write()` has TOCTOU risk. All operations need cancellation checks.

**Wrapper scope**:
- Cancellation check before each operation
- Accounting: `filesystem_operations` + `filesystem_bytes_written`
- TOCTOU mitigation for `io.write()` (per-call path validation)

### Priority 3: Filesystem Read Outside Roots (HIGH)

**Target**: `io.read()`, `io.lines()`, `unpwdb.*`, `creds.*`, `datafiles.*`, `nmap.fetchfile()`

**Rationale**: Reads outside explicit roots can leak sensitive data. `io.read()` has TOCTOU risk.

**Wrapper scope**:
- Cancellation check before each operation
- Accounting: `filesystem_bytes_read`
- TOCTOU mitigation for `io.read()` (per-call path validation)

### Priority 4: Network TCP/UDP (HIGH)

**Target**: All protocol libraries (~100+), `nmap.socket_*()`, `nmap.async_socket_*()`, `comm.*`, `http.*`

**Rationale**: Network I/O is the most common blocking operation. Protocol libraries bypass sandbox entirely. `nmap.socket_*()` bypasses socket sandbox.

**Wrapper scope**:
- Cancellation check before connect/send/receive
- Accounting: `network_operations`, `network_bytes_read`, `network_bytes_written`
- Profile policy enforcement for protocol libs
- Sandbox enforcement for `nmap.socket_*()` and `nmap.async_socket_*()`

### Priority 5: DNS Resolution (MEDIUM)

**Target**: `dns.*` (hickory-resolver), `socket.resolve_async()` (already sandboxed)

**Rationale**: DNS lookups can block and may leak target information. `dns.*` has no sandbox checks.

**Wrapper scope**:
- Cancellation check before resolution
- Accounting: `network_operations`
- Profile policy enforcement

### Priority 6: Compression on Untrusted Inputs (MEDIUM)

**Target**: Libraries using flate2/zlib

**Rationale**: Decompression bombs can cause excessive memory allocation. Low blocking risk on small inputs.

**Wrapper scope**:
- Accounting: `compression_bytes_in/out`
- Size limits on decompression output

### Priority 7: Crypto/TLS Blocking (MEDIUM)

**Target**: `ssl.*`, `openssl.*`

**Rationale**: TLS handshakes can block. Crypto operations may allocate heavily.

**Wrapper scope**:
- Cancellation check before handshake
- Accounting: `crypto_operations`

### Priority 8: Time/Randomness/Environment Reads (LOW)

**Target**: `stdnse.sleep()`, `stdnse.usleep()`, `stdnse.nsleep()`, `os.getenv()`, `os.setenv()`, `os.time()`, `rand.*`

**Rationale**: Sleep is the only high-blocking-risk helper in this group. Environment and time reads are quick.

**Wrapper scope**:
- Cancellation check for sleep operations
- Profile policy enforcement for `os.setenv()`/`os.unsetenv()`

### Priority 9: Pure CPU Helpers (NONE)

**Target**: `tab`, `json`, `base64`, `base32`, `bin`, `bit`, `stringaux`, `strbuf`, etc.

**Rationale**: No I/O side effects. No wrapper migration needed.

---

## Test Matrix

For each high-risk helper class, define future tests:

| Helper Class | Manual Allowed | Agent Denied | CI Denied | Cancellation | Limit Exceeded | Report Event |
|--------------|----------------|--------------|-----------|--------------|----------------|--------------|
| Process exec (`io.popen`) | ✅ accounting | ✅ denied | ✅ denied | ✅ pre-call check | ✅ process_ops limit | ✅ `process_exec` |
| Process exec (`nmap.is_admin`) | ✅ accounting | ✅ denied | ✅ allowed local | ✅ pre-call check | ✅ process_ops limit | ✅ `process_exec` |
| FS write (`io.open` write modes) | ✅ accounting | ✅ denied | ✅ allowed local | ✅ pre-call check | ✅ fs_ops limit | ✅ `fs_write` |
| FS write (`io.write`) | ✅ TOCTOU fix | ✅ denied | ✅ allowed local | ✅ pre-call check | ✅ fs_bytes limit | ✅ `fs_write` |
| FS write (`lfs.*`) | ✅ accounting | ✅ denied | ✅ allowed local | ✅ pre-call check | ✅ fs_ops limit | ✅ `fs_write` |
| FS read (`io.read`) | ✅ TOCTOU fix | ✅ denied | ✅ allowed local | ✅ pre-call check | ✅ fs_bytes limit | ✅ `fs_read` |
| Network TCP (`socket.*`) | ✅ existing sandbox | ✅ scoped only | ✅ denied | ✅ pre-call check | ✅ net_ops limit | ✅ `net_connect` |
| Network TCP (`nmap.socket_*`) | ✅ add sandbox | ✅ scoped only | ✅ denied | ✅ pre-call check | ✅ net_ops limit | ✅ `net_connect` |
| Network TCP (`http.*`) | ✅ add sandbox | ✅ scoped only | ✅ denied | ✅ pre-call check | ✅ net_ops limit | ✅ `net_http` |
| Network TCP (`comm.*`) | ✅ add sandbox | ✅ scoped only | ✅ denied | ✅ pre-call check | ✅ net_ops limit | ✅ `net_connect` |
| Network TCP (protocol libs) | ✅ add sandbox | ✅ scoped only | ✅ denied | ✅ pre-call check | ✅ net_ops limit | ✅ `net_connect` |
| DNS (`dns.*`) | ✅ add sandbox | ✅ scoped only | ✅ denied | ✅ pre-call check | ✅ net_ops limit | ✅ `net_resolve` |
| Sleep (`stdnse.sleep`) | ✅ accounting | ✅ denied | ✅ denied | ✅ pre-call check | ✅ wall_clock | ✅ `sleep` |

---

## Verification

Phase 01 is complete when:

- [x] A capability inventory exists in repo docs.
- [x] Helper side effects are classified by capability, risk, policy, accounting, cancellation, and reporting needs.
- [x] High-priority migration order is clear.
- [x] Later phases can migrate wrappers without redoing broad inventory work.

### Verification Commands

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse
bash scripts/check-architecture-guards.sh
```

---

## Phase 02: Capability Context (Complete)

`NseCapabilityContext` in `capabilities.rs` provides centralized policy enforcement for all side-effecting NSE helpers. This replaces per-profile scattered checks with a single decision engine.

### Core Types

| Type | Location | Purpose |
|------|----------|---------|
| `NseCapabilityContext` | `capabilities.rs:26-45` | Central enforcement: profile_kind, network_policy, limits, cancellation, counters, events |
| `NseCapabilityKind` | `capabilities.rs:48-72` | 11 operation classes: FilesystemRead, FilesystemWrite, ProcessExec, NetworkTcp, NetworkUdp, DnsResolution, TimeClock, Randomness, Crypto, Compression, Environment |
| `NseCapabilityRequest` | `capabilities.rs:93-103` | Request with kind, target, bytes_hint, operation name |
| `NseCapabilityDecision` | `capabilities.rs:106-114` | Allow / Deny{reason} / AllowWithWarning{warning} |
| `NseCapabilityEvent` | `capabilities.rs:145-159` | Recorded event with kind, operation, target, allowed, reason, bytes |

### Profile-Specific Policy

| Profile | Process Exec | FS Write | Network TCP/UDP | DNS | Time/Clock | Environment |
|---------|-------------|----------|-----------------|-----|------------|-------------|
| ManualPermissive | Allow+Warn | Allow+Warn | Allow (sandbox check) | Allow | Allow | Allow |
| ManualStrict | Deny | Allow (sandbox path) | AllowCidrs | Allow | Allow | Deny |
| AgentSafe | Deny | Deny | Scoped to target | Allow | Allow | Deny |
| CiSafe | Deny | Deny | DenyAll | Deny | Deny | Deny |
| CompatibilityLab | Allow+Warn | Allow+Warn | Allow+Warn | Allow | Allow | Allow |

### Integration Points

- **ExecutorCore**: `executor_core.rs` stores `NseCapabilityContext` in `capability_context` field, constructed via `with_policy()` (manual-only), `with_profile()` (preferred for CLI/automated), or `with_full_policy()` (explicit control over all policies)
- **NseRunReport**: `report.rs` includes `capability_events: Vec<NseCapabilityEvent>` and `capability_event_summary: Option<NseCapabilityEventSummary>`
- **Wrappers**: `wrappers.rs` contains wrapper functions (check_time_clock, check_fs_read, check_fs_write, check_network_tcp, check_process_exec, check_dns, check_randomness, check_environment, check_crypto, check_compression, plus executing wrappers for all migrated classes) that route operations through the capability context

### Migration Priority (from Phase 01)

1. Process execution (already wrapped)
2. Filesystem write
3. Filesystem read (already wrapped)
4. Network TCP/UDP (TCP already wrapped)
5. DNS resolution (already wrapped)
6. Compression
7. Crypto/TLS
8. Time/randomness (already wrapped)
9. Environment (already wrapped)
10. Pure CPU (no wrapper needed)

### Architecture Guard

Check 33 in `scripts/check-architecture-guards.sh` detects direct `std::process::Command` in NSE libraries (FAIL after Phase 03). Check 33b detects direct filesystem ops in unmigrated libraries (informational). Check 33c detects direct network calls in unmigrated libraries (informational). Check 34 verifies capability context integration.

## Milestone 3 Completion Summary

**Date:** 2026-07-06

All primary helper classes are migrated through `NseCapabilityContext`. The inventory is complete for: filesystem (io/lfs/os), process execution (os/nmap), network TCP/UDP (socket/comm), DNS (dns), time (datetime), randomness (rand), environment (os), compression (zlib), and crypto/TLS (openssl/tls/sslcert).

### Migration Status

| Capability Class | Status | Libraries | Wrappers |
|-----------------|--------|-----------|----------|
| Filesystem Read | ✅ Migrated | `io.rs`, `lfs.rs`, `os.rs` | `check_fs_read()`, `nse_fs_read_to_string()`, `nse_fs_read()` |
| Filesystem Write | ✅ Migrated | `io.rs`, `lfs.rs`, `os.rs` | `check_fs_write()`, `nse_fs_write()`, `nse_fs_remove_file()`, `nse_fs_create_dir()`, `nse_fs_rename()` |
| Process Execution | ✅ Migrated | `io.rs`, `os.rs`, `nmap.rs` | `check_process_exec()`, `nse_process_exec()` |
| Network TCP | ✅ Migrated | `socket.rs`, `comm.rs` | `nse_network_tcp_connect()`, `nse_network_tcp_send()`, `nse_network_tcp_receive()` |
| Network UDP | ✅ Migrated | `socket.rs` | `nse_network_udp_send()`, `nse_network_udp_receive()`, `check_network_udp()` |
| DNS Resolution | ✅ Migrated | `dns.rs` | `nse_dns_lookup()` |
| Time Clock | ✅ Migrated | `datetime.rs` | `nse_time_now()`, `check_time_clock()` |
| Randomness | ✅ Migrated | `rand.rs` | `nse_random_bytes()`, `check_randomness()` |
| Environment | ✅ Migrated | `os.rs` | `nse_env_var()`, `check_environment()` |
| Compression | ✅ Migrated | `zlib.rs` | `nse_compress()`, `nse_decompress()`, `check_compression()` |
| Crypto/TLS | ✅ Migrated | `openssl.rs`, `tls.rs`, `sslcert.rs` | `check_crypto()` |

### Remaining Deferred Items

- `unpwdb.rs` — password database file reads (protocol-specific internal helper)
- `brute.rs` — brute force helper operations (protocol-specific internal helper)
- `datafiles.rs` — data file reads (protocol-specific internal helper)
- Protocol-specific internal helpers beyond network I/O (smb, ssh, ftp, http, etc.)

These deferred items are protocol-specific internal helpers that do not have their own capability classes. They use migrated network I/O wrappers but may have unmigrated helper calls within their protocol logic.

### Architecture Guard Posture

| Guard | Status | Description |
|-------|--------|-------------|
| Check 33 | FAIL | Detects direct `std::process::Command` in NSE libraries |
| Check 33b | INFO | Detects direct filesystem ops in unmigrated libraries |
| Check 33c | INFO | Detects direct network calls in unmigrated libraries |
| Check 34 | PASS | Verifies capability context integration |
| Check 35 | PASS | Verifies `run_cli_with_profile()` uses `with_profile()` (not `with_policy()`) |
| Check 36 | PASS | Detects automated surfaces calling `with_policy()` (should use `with_profile()` or `with_full_policy()`) |
| Check 37 | INFO | Lists all callers of `ExecutorCore::with_policy()` for audit |
