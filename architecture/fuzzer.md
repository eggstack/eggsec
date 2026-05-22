# Fuzzer Module

The Fuzzer is the most advanced part of Slapper, designed to find vulnerabilities by sending semi-random or specifically crafted data to a target and analyzing the response.

## Core Architecture (`src/fuzzer/`)

### Fuzzing Engine (`engine/`)

The core loop that manages targets, payloads, and detections.

- **State Management (`state.rs`)**: Keeps track of progress, discovered vulnerabilities, and session information.
- **Mutator (`mutator.rs`)**: Applies transformations to payloads (e.g., encoding, truncation, bit-flipping).
- **Rate Limiting (`rate_limit.rs`)**: Ensures the fuzzer doesn't overwhelm the target or the local network.
- **Execution Modes**: Sequential (one at a time), Burst (concurrent up to 500), Adaptive (rate-limited)

### Payloads (`payloads/`)

Slapper comes with a vast library of payloads for different vulnerability types:

- **Injection**: SQLi, XSS, Command Injection, Template Injection.
- **File System**: Path Traversal, LFI/RFI.
- **Logic**: Authentication bypass, Parameter Pollution.
- **Grammar-based Fuzzing (`grammar.rs`)**: Generates structured payloads for complex protocols or data formats.

### Detection (`detection/`)

Algorithms for identifying if a fuzzing attempt was successful.

- **Error-based**: Looking for specific database errors or stack traces.
- **Boolean-based**: Comparing responses for "True" vs "False" conditions.
- **Time-based**: Detecting delays that indicate successful injection.
- **Diffing (`diff.rs`)**: Comparing the response of a fuzzed request against a baseline "clean" request.

### WAF Fingerprinting & Bypass (`waf_fingerprint.rs`)

Specialized logic to detect Web Application Firewalls and apply bypass techniques (e.g., specific encodings, header manipulations) automatically.

## Specialized Fuzzing

- **API Schema Fuzzing (`api_schema/`)**: Automatically generates tests based on OpenAPI (Swagger) or gRPC definitions.
- **Advanced Threat Hunting (`advanced.rs`)**: Uses more complex patterns to find obscure vulnerabilities.
- **ReDoS Detection (`redos_detect.rs`)**: Specifically targets Regular Expression Denial of Service vulnerabilities.

## Feedback Loop

The fuzzer is designed to be "smart," using feedback from the target (e.g., changes in response time or body content) to prioritize certain payloads or mutators.

## Code Conventions

### Hash Collections
Use `rustc_hash::FxHashMap` and `rustc_hash::FxHashSet` instead of `std::collections::HashMap/HashSet` for performance.

### Magic Numbers
Extract magic numbers to named constants at module level:
```rust
const DEFAULT_SPIKE_THRESHOLD: f64 = 3.0;
const DEFAULT_REDOS_THRESHOLD_MS: u64 = 5000;
const BODY_LENGTH_ANOMALY_THRESHOLD: isize = 1000;
```

### Error Handling
Prefer explicit error handling over `unwrap_or_default()`:
```rust
// Instead of:
let body = response.text().await.unwrap_or_default();

// Use:
let body = match response.text().await {
    Ok(text) => text,
    Err(e) => {
        tracing::debug!("Failed to read response body: {}", e);
        String::new()
    }
};
```
