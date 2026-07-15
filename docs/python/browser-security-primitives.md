# Browser Security Primitives

This guide covers browser session security assessment primitives in eggsec-python.

## Session Lifecycle

```python
from eggsec import BrowserSession, BrowserSessionConfig

config = BrowserSessionConfig(
    target_url="https://example.com",
    headless=True,
    viewport_width=1280,
    viewport_height=720,
    timeout_ms=30000,
    collect_console=True,
    collect_network=True,
    collect_cookies=True,
    collect_storage=True,
)

with BrowserSession(config) as browser:
    browser.start()
    nav = browser.navigate("https://example.com/login")
    print(f"Final URL: {nav.final_url}")
    print(f"Status: {nav.status_code}")
    print(f"Redirects: {nav.redirect_chain}")
```

## DOM Inspection

```python
snapshot = browser.get_dom_snapshot()
print(f"URL: {snapshot.url}")
print(f"Title: {snapshot.title}")
print(f"Forms: {len(snapshot.forms)}")
print(f"Links: {len(snapshot.links)}")
print(f"Scripts: {len(snapshot.scripts)}")

for form in snapshot.forms:
    print(f"  Form: {form.action} ({form.method})")
    for field in form.fields:
        print(f"    {field.name}: {field.field_type} (required={field.required})")
```

## Console Events

```python
events = browser.get_console_events()
for event in events:
    print(f"[{event.level}] {event.message}")
    if event.source:
        print(f"  Source: {event.source}:{event.line_number}")
```

## Network Events

```python
network = browser.get_network_events()
for req in network:
    print(f"{req.method} {req.url} -> {req.status_code}")
    print(f"  Content-Type: {req.content_type}")
    print(f"  Size: {req.size_bytes} bytes")
    print(f"  Duration: {req.duration_ms}ms")
```

## Cookie and Storage Inspection

```python
storage = browser.get_cookies()
print(f"Cookies: {len(storage.cookies)}")
for cookie in storage.cookies:
    print(f"  {cookie.name}: domain={cookie.domain}, secure={cookie.secure}")

print(f"LocalStorage: {len(storage.local_storage)} items")
print(f"SessionStorage: {len(storage.session_storage)} items")
```

## Screenshots

```python
artifact = browser.take_screenshot()
print(f"Screenshot: {artifact.artifact_id}")
```

## Script Execution

```python
try:
    result = browser.execute_script("document.title")
    print(f"Title: {result}")
except ScanError as e:
    print(f"Script execution requires browser engine: {e}")
```

## Async Operations

```python
from eggsec import AsyncBrowserSession

async with AsyncBrowserSession(config) as browser:
    await browser.async_start()
    nav = await browser.async_navigate("https://example.com")
    snapshot = await browser.async_get_dom_snapshot()
    events = await browser.async_get_console_events()
```

## Browser Capabilities

```python
from eggsec import BrowserCapabilities

caps = BrowserCapabilities(
    engine="chromium",
    supports_javascript=True,
    supports_dom=True,
    supports_network_intercept=True,
    supports_console_capture=True,
    supports_screenshot=True,
    supports_cookie_access=True,
    supports_storage_access=True,
    supports_route_discovery=True,
)
```
