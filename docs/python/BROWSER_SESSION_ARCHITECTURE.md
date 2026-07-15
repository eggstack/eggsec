# Browser Session Architecture

## Overview

The browser session subsystem provides managed headless browser lifecycle
management for DOM XSS detection, SPA crawling, and client-side security
assessment. It is implemented in the `eggsec` engine (`crates/eggsec/src/browser/`)
and exposed to Python via `eggsec-python` bindings (Release 4, feature:
`headless-browser`).

All browser session types are **provisional** — scope-checked and policy-gated
but not yet part of the stable-core operation registry.

## Architecture Layers

```
┌─────────────────────────────────────────────────────────┐
│  Python Surface (eggsec.browser)                        │
│  BrowserSession, BrowserCapabilities, session state     │
├─────────────────────────────────────────────────────────┤
│  PyO3 Bindings (crates/eggsec-python/src/browser.rs)   │
│  DTOs, conversion, sync/async dispatch                  │
├─────────────────────────────────────────────────────────┤
│  Eggsec Browser Engine (crates/eggsec/src/browser/)     │
│  BrowserController, DOM inspector, event router         │
├─────────────────────────────────────────────────────────┤
│  Headless Browser Backend (headless-browser feature)    │
│  Chromium/WebKit process management, CDP protocol       │
└─────────────────────────────────────────────────────────┘
```

## Browser Capabilities

`BrowserCapabilities` declares what the managed browser instance can do
before a session starts. Capabilities are resolved from the configured
feature set and platform constraints.

| Field | Type | Description |
|-------|------|-------------|
| `javascript` | `bool` | JavaScript execution enabled |
| `dom_access` | `bool` | DOM tree inspection and mutation |
| `network_intercept` | `bool` | Request/response interception |
| `console_capture` | `bool` | Console log capture |
| `screenshot` | `bool` | Visual capture capability |
| `cookie_access` | `bool` | Cookie jar read/write |
| `storage_access` | `bool` | localStorage/sessionStorage access |
| `script_injection` | `bool` | Custom script injection into page context |

Capabilities are immutable once resolved. If a requested capability is
unavailable on the platform, session creation fails fast with a structured
error rather than silently degrading.

## Browser Session Configuration

`BrowserSessionConfig` holds the parameters for creating a new browser
session. Configuration is validated at construction time.

| Field | Type | Description |
|-------|------|-------------|
| `target_url` | `String` | Initial navigation target |
| `viewport_width` | `u32` | Viewport width (default 1280) |
| `viewport_height` | `u32` | Viewport height (default 720) |
| `user_agent` | `Option<String>` | Custom user agent string |
| `timeout_ms` | `u64` | Navigation/operation timeout |
| `enable_javascript` | `bool` | JavaScript execution toggle |
| `enable_network_log` | `bool` | Capture network requests |
| `enable_console_log` | `bool` | Capture console output |
| `extra_headers` | `HashMap<String, String>` | Additional HTTP headers |
| `proxy_addr` | `Option<String>` | Upstream proxy for traffic routing |

The configuration is serialized into the session metadata for audit and
replay purposes.

## Session State Lifecycle

`BrowserSessionState` is a state machine governing the browser session
lifetime. Transitions are recorded as events for correlation.

```
Created → Navigating → Active → Navigating → Active → … → Closing → Closed
                                  ↓ (error)
                               Error → Closing → Closed
```

| State | Description |
|-------|-------------|
| `Created` | Session allocated, browser process not yet spawned |
| `Navigating` | Page load in progress, awaiting `load` or `domcontentloaded` |
| `Active` | Page loaded, ready for inspection and script execution |
| `Closing` | Teardown in progress, flushing events and artifacts |
| `Closed` | Session fully released, process terminated |
| `Error` | Unrecoverable failure; transitions to `Closing` automatically |

State transitions emit `RuntimeEvent` entries with the session ID, enabling
the daemon and UI layers to track browser lifecycle without direct process
handles.

## Security Primitives

Each browser session exposes security-relevant primitives as structured
DTOs. These are the primary interface for DOM-based security assessment.

### DOM Inspector

Provides read access to the page DOM tree without executing scripts in
the page context. Used for DOM XSS sink/source identification.

| Method | Returns | Description |
|--------|---------|-------------|
| `get_document()` | `DomNode` | Full DOM tree |
| `query_selectorAll(selector)` | `Vec<DomNode>` | CSS selector matches |
| `get_attribute(node_id, name)` | `Option<String>` | Element attribute value |
| `get_inner_text(node_id)` | `String` | Element text content |

### Network Monitor

Captures HTTP/HTTPS requests and responses for security analysis.

Each captured request produces a `NetworkEntry` with:
- Method, URL, status code
- Request/response headers
- Timing (start, end, duration)
- Initiator (script, link, redirect chain)

### Console Logger

Captures browser console output for error detection and client-side
vulnerability analysis.

Each console entry includes:
- Log level (error, warn, info, debug)
- Message text
- Source URL and line number
- Timestamp

### Cookie Access

Reads and manipulates the browser cookie jar for session testing.

| Method | Description |
|--------|-------------|
| `get_cookies()` | List all cookies for current origin |
| `get_cookie(name)` | Get a specific cookie value |
| `set_cookie(name, value, domain, path)` | Set a cookie |
| `delete_cookie(name, domain)` | Remove a cookie |

### Storage Access

Provides access to client-side storage for session analysis.

| Method | Description |
|--------|-------------|
| `get_local_storage()` | Dump localStorage key-value pairs |
| `get_session_storage()` | Dump sessionStorage key-value pairs |
| `clear_local_storage()` | Clear localStorage |
| `clear_session_storage()` | Clear sessionStorage |

### Screenshot

Captures visual state of the page for evidence and visual regression.

Returns a `Screenshot` with:
- PNG-encoded image data
- Viewport dimensions
- Timestamp
- Optional clip region

## Script Execution Boundary

Scripts injected into the browser page execute in the browser's native
JavaScript context, isolated from the Eggsec Python process. The boundary
enforces:

1. **No direct Python calls**: Injected JavaScript cannot call back into
   the PyO3 bindings. Communication is event-based via `postMessage` or
   DOM mutation observation.

2. **Timeout enforcement**: All script injection has a configurable timeout.
   Scripts exceeding the timeout are terminated and an error event is
   emitted.

3. **Content Security Policy respect**: If the page's CSP blocks inline
   scripts, injection attempts fail with a structured `CspViolation` error
   rather than silently failing.

4. **Origin isolation**: Scripts injected on one origin cannot access
   storage or cookies from a different origin, even if the session has
   navigated across origins.

The script execution boundary is critical for preventing assessment tools
from becoming attack vectors themselves.

## Event Correlation

All browser session events carry a correlation ID linking them to the
creating operation. This enables:

- **Per-operation grouping**: Events from a single `BrowserSession`
  operation are grouped by `operation_correlation_id`.
- **Timeline reconstruction**: Events carry `timestamp_ms` for ordered
  replay in the UI and daemon.
- **Artifact linkage**: Artifacts (screenshots, DOM snapshots, network
  captures) reference the session ID and operation correlation ID for
  content-addressed retrieval.

Event types emitted by a browser session:

| Event Kind | Data | When |
|------------|------|------|
| `session.created` | `BrowserSessionConfig` | Session allocated |
| `session.navigated` | `url, status, timing` | Page load complete |
| `session.script.injected` | `script_id, target, result` | Custom script executed |
| `session.network.request` | `NetworkEntry` | HTTP request captured |
| `session.network.response` | `NetworkEntry` | HTTP response captured |
| `session.console.log` | `ConsoleEntry` | Console output captured |
| `session.dom.snapshot` | `DomNode` | DOM tree snapshot taken |
| `session.screenshot` | `Screenshot` | Visual capture taken |
| `session.cookie.changed` | `CookieEntry` | Cookie set/deleted |
| `session.error` | `ErrorKind, message` | Unrecoverable failure |
| `session.closing` | `Duration` | Teardown started |
| `session.closed` | `Duration` | Session fully released |

Events flow through the same `RuntimeEvent` channel used by daemon sessions,
enabling unified event streaming regardless of session type.
