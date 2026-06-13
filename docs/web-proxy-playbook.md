# Web Proxy Manipulation Playbook

Common attack/defense patterns for the Eggsec interactive web proxy in authorized lab environments.

## Authentication & Session Testing

### Token Tampering

**Goal**: Test authorization by modifying JWT/OAuth tokens in intercepted requests.

1. Intercept a request containing an `Authorization: Bearer <token>` header
2. In the TUI, open the edit modal (`e` key) on the header
3. Modify the JWT payload (e.g., change `role: user` to `role: admin`)
4. Forward the modified request
5. Check if the server accepts the tampered token

**Detection**: Server should reject modified tokens with 401/403.

**Defense**: Validate JWT signatures server-side; use short-lived tokens; implement token binding.

### Session Fixation

**Goal**: Test if session tokens can be reused across different client contexts.

1. Capture a login flow to obtain a valid session cookie
2. Export the session (`InterceptSession` save)
3. Load the session in a different context (different IP, User-Agent)
4. Replay requests with the original session cookie

**Detection**: Server should invalidate the session or require re-authentication.

### Cookie Security

**Goal**: Verify cookies have proper security attributes.

1. Intercept a response that sets cookies
2. Inspect the `Set-Cookie` headers in the Detail Pane
3. Check for: `Secure`, `HttpOnly`, `SameSite`, `Path`, `Domain` attributes
4. Modify cookie attributes and replay to test server behavior

**Checklist**:
- `Secure` flag present on authentication cookies
- `HttpOnly` flag present to prevent XSS cookie theft
- `SameSite=Strict` or `Lax` to prevent CSRF
- No overly broad `Domain` or `Path` values

## Input Validation

### SQL Injection via Headers

**Goal**: Test if custom headers are vulnerable to SQL injection.

1. Intercept a request with user-controlled headers (e.g., `X-Forwarded-For`, `Referer`)
2. Add SQL injection payloads to the header value:
   - `' OR 1=1 --`
   - `1' UNION SELECT null,null,null--`
   - `'; WAITFOR DELAY '0:0:5'--`
3. Forward and observe response timing/content

**Detection**: Error messages, different response patterns, time delays.

### XSS via Request Body

**Goal**: Test if reflected XSS appears in error responses.

1. Intercept a POST request with JSON/form body
2. Inject XSS payloads: `<script>alert(1)</script>`, `"><img src=x onerror=alert(1)>`
3. Forward and check if the payload appears in the response body

**Detection**: Payload reflected without encoding in the response.

### Path Traversal

**Goal**: Test for path traversal in file-serving endpoints.

1. Intercept a request to a file-serving endpoint
2. Modify the path: `/../../../etc/passwd`, `/..%2f..%2f..%2fetc/passwd`
3. Forward and check for file content in the response

## API Security

### Method Tampering

**Goal**: Test if endpoints enforce HTTP method restrictions.

1. Intercept a request to a restricted endpoint (e.g., `DELETE /api/users/1`)
2. Replay with different methods: `GET`, `POST`, `PUT`, `PATCH`, `OPTIONS`
3. Check if unauthorized methods succeed

### Content-Type Bypass

**Goal**: Test if APIs validate Content-Type headers.

1. Intercept a JSON API request
2. Change `Content-Type` from `application/json` to `application/x-www-form-urlencoded`
3. Forward and check if the server accepts the request

### Rate Limiting Bypass

**Goal**: Test rate limiting by replaying requests rapidly.

1. Intercept a rate-limited endpoint request
2. Use the Replay action multiple times in quick succession
3. Check if rate limiting triggers or can be bypassed via header manipulation

**Bypass techniques**:
- Add `X-Forwarded-For` with different IPs
- Change `User-Agent` on each request
- Add random query parameters
- Use different HTTP methods

## Header Manipulation

### Host Header Injection

**Goal**: Test for host header-based vulnerabilities.

1. Intercept any request
2. Modify the `Host` header to an internal hostname
3. Forward and check if the server routes to a different backend

### Security Header Inspection

**Goal**: Verify security headers are present and correctly configured.

Check intercepted responses for:
- `Strict-Transport-Security` (HSTS)
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY` or `SAMEORIGIN`
- `Content-Security-Policy`
- `X-XSS-Protection: 0` (modern) or `1; mode=block` (legacy)
- `Referrer-Policy`
- `Permissions-Policy`

### CORS Testing

**Goal**: Test Cross-Origin Resource Sharing configuration.

1. Intercept a request and note the `Origin` header
2. Modify the `Origin` to a different domain
3. Forward and check if `Access-Control-Allow-Origin` reflects the attacker origin

**Detection**: Wildcard or reflected origin indicates misconfigured CORS.

## WebSocket Security

### Message Injection

**Goal**: Test WebSocket message handling for injection vulnerabilities.

1. Intercept WebSocket traffic in the TUI WebSocket detail pane
2. Identify message format (JSON, text, binary)
3. Inject payloads into text messages
4. Forward the modified WebSocket message

### Authentication Bypass

**Goal**: Test if WebSocket connections require authentication.

1. Intercept the WebSocket upgrade request
2. Note the authentication headers/tokens
3. Replay the upgrade request without authentication
4. Check if the server establishes the connection

## Defense Validation

### Content-Security-Policy Enforcement

**Goal**: Verify CSP headers prevent XSS.

1. Intercept a response with CSP headers
2. Attempt to inject inline scripts via the edit modal
3. Check if CSP blocks execution in the browser

### Input Sanitization

**Goal**: Test that user input is properly sanitized.

1. Intercept a form submission
2. Add special characters: `<>&"'\/`
3. Forward and check if the response properly escapes/encodes the input

## Usage Tips

### Using with TUI

1. Start the TUI: `eggsec-tui`
2. Navigate to the Intercept tab
3. Configure listen address and dry-run mode
4. Press Enter to start the session
5. Use `Tab` to cycle focus between Flow List, Detail View, and Action Bar
6. In Detail View, use `↑`/`↓` to cycle through Headers, Body, Manipulations, Rules
7. Press `e` to open the edit modal for the current detail pane
8. After editing, use the Action Bar to Forward, Drop, or Replay the flow

### Using with CLI

```bash
# Quick dry-run for planning
eggsec proxy-intercept --dry-run --json -o plan.json

# Apply rules and capture live traffic
eggsec proxy-intercept \
  --listen 127.0.0.1:8080 \
  --allow-web-proxy \
  --manual-override-reason "Authorized testing" \
  --intercept-rule "*:*:monitor" \
  --max-flows 100 \
  --json -o capture.json

# Convert to different formats
eggsec report convert capture.json -f sarif -o capture.sarif
eggsec report convert capture.json -f html -o capture.html
```

### Session Replay

1. Save a session from the TUI (Action Bar → Save)
2. Load the session later for analysis
3. Export as HAR for browser DevTools import
4. Use the manipulation audit trail to document all changes

## Safety Reminders

- **Lab only**: Always use on systems you own or are authorized to test
- **Scope**: Keep the proxy scope restricted to lab targets
- **Dry-run**: Use `--dry-run` for safe validation before live interception
- **Documentation**: Record all manipulations for your audit trail
- **Cleanup**: Remove CA certificates from trust stores after testing
