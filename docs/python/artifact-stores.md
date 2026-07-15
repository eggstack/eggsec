# Artifact Stores

This guide covers content-addressed and directory-backed artifact storage.

## Content-Addressed Store

Artifacts are stored and retrieved by their content hash. Identical content is deduplicated automatically.

```python
from eggsec import ContentAddressedArtifactStore

store = ContentAddressedArtifactStore("/tmp/artifacts")
store.initialize()

# Store artifacts
info = store.put(b"Hello, World!", "text/plain")
print(f"Hash: {info.content_hash}")
print(f"Size: {info.size_bytes}")

# Retrieve by hash
data = store.get(info.content_hash)
if data:
    print(f"Content: {data.data.decode()}")
    print(f"Info: {data.info.content_type}")

# Verify integrity
result = store.verify(info.content_hash)
print(f"Valid: {result.valid}")

# List and query
artifacts = store.list_artifacts(limit=10)
total = store.total_size_bytes()

# Prune old/large artifacts
removed = store.prune(max_age_secs=3600, max_size_bytes=1024*1024)
```

## Directory-Backed Store

Artifacts are stored under named paths in a directory.

```python
from eggsec import DirectoryArtifactStore

store = DirectoryArtifactStore("/tmp/artifacts", flat=True)
store.initialize()

# Store by name
info = store.put("screenshot.png", png_bytes, "image/png")

# Retrieve by name
data = store.get("screenshot.png")

# Resolve on-disk path
path = store.resolve_path("screenshot.png")
```

## Artifact Query

```python
from eggsec import ArtifactQuery

query = ArtifactQuery(
    content_type="image/png",
    min_size=100,
    max_size=1024*1024,
    limit=50,
)
```

## Integrity Verification

```python
result = store.verify("abc123hash")
if result.valid:
    print(f"Integrity OK: {result.size_bytes} bytes")
else:
    print(f"Mismatch: expected {result.expected_hash}, got {result.actual_hash}")
```

## Context Manager

```python
with ContentAddressedArtifactStore("/tmp/artifacts") as store:
    store.initialize()
    info = store.put(b"data", "application/octet-stream")
```
