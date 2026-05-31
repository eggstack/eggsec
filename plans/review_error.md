# Error Module Architecture Review

**Document:** architecture/error.md
**Reviewed:** 2026-05-31
**Accuracy:** Medium
**Lines Reviewed:** 49

## Verified Claims

- **SlapperError is the primary error enum**: Verified at `error/mod.rs:43-116`
- **Location `error/mod.rs`**: Verified -- single file module at `crates/slapper/src/error/mod.rs`
- **Result<T> type alias**: Verified at `error/mod.rs:170` (`pub type Result<T> = std::result::Result<T, SlapperError>;`)
- **SlapperError derives `thiserror::Error`**: Verified at `error/mod.rs:43` (`#[derive(Debug, Error)]`)
- **Config(String)**: Verified at `error/mod.rs:45-46`
- **InvalidTarget(String)**: Verified at `error/mod.rs:48-49`
- **Network(String)**: Verified at `error/mod.rs:51-52`
- **RequestFailed { method, url, error }**: Verified at `error/mod.rs:54-59`
- **Timeout { timeout_ms, operation }**: Verified at `error/mod.rs:63-64`
- **RateLimited(String)**: Verified at `error/mod.rs:66-67`
- **ScanFailed { stage, error }**: Verified at `error/mod.rs:69-70`
- **Payload(String)**: Verified at `error/mod.rs:72-73`
- **Output(String)**: Verified at `error/mod.rs:75-76`
- **ScopeViolation(String)**: Verified at `error/mod.rs:78-79`
- **Io(std::io::Error)** with `From` impl**: Verified at `error/mod.rs:81-82` (`#[from]`)
- **HttpStatus { status, message }**: Verified at `error/mod.rs:84-85`
- **Http(String)**: Verified at `error/mod.rs:87-88`
- **Parse(String)**: Verified at `error/mod.rs:90-91`
- **Validation(String)**: Verified at `error/mod.rs:93-94`
- **AddressParse(String)**: Verified at `error/mod.rs:96-97`
- **Runtime(String)**: Verified at `error/mod.rs:99-100`
- **Cancelled**: Verified at `error/mod.rs:102-103`
- **Proxy(String)**: Verified at `error/mod.rs:105-106`
- **Recon(String)**: Verified at `error/mod.rs:108-109`
- **LoadTest(String)**: Verified at `error/mod.rs:111-112`
- **Fingerprint(String)**: Verified at `error/mod.rs:114-115`
- **is_timeout() helper**: Verified at `error/mod.rs:120-122`
- **is_network() helper**: Verified at `error/mod.rs:125-127`
- **http_status() helper**: Verified at `error/mod.rs:130-135`
- **with_timeout() helper**: Verified at `error/mod.rs:158-167`
- **From<reqwest::Error> impl**: Verified at `error/mod.rs:172-200` -- converts timeout/connect/status/generic reqwest errors
- **From<anyhow::Error> impl**: Verified at `error/mod.rs:265-273` -- wraps as RequestFailed
- **thiserror derives**: Verified at `error/mod.rs:43`

## Discrepancies

- **Variant count**: Document says "19+ variants" but actual count is **22 variants** (Config, InvalidTarget, Network, RequestFailed, Timeout, RateLimited, ScanFailed, Payload, Output, ScopeViolation, Io, HttpStatus, Http, Parse, Validation, AddressParse, Runtime, Cancelled, Proxy, Recon, LoadTest, Fingerprint). The "19+" is technically accurate but significantly understates the actual count.

## Bugs Found

- No bugs found in the architecture document.

## Improvement Opportunities

- **[Item]: Document all From impls**: The doc only mentions `From` conversions for `reqwest::Error` and `anyhow::Error` (line 45). There are actually **14 additional `From` impls** that are undocumented:
  - `From<toml::de::Error>` at `error/mod.rs:202-206`
  - `From<serde_json::Error>` at `error/mod.rs:208-212`
  - `From<url::ParseError>` at `error/mod.rs:214-218`
  - `From<std::net::AddrParseError>` at `error/mod.rs:220-224`
  - `From<serde_yaml_neo::Error>` at `error/mod.rs:226-230`
  - `From<toml::ser::Error>` at `error/mod.rs:232-236`
  - `From<std::string::FromUtf8Error>` at `error/mod.rs:238-242`
  - `From<tokio::time::error::Elapsed>` at `error/mod.rs:244-251`
  - `From<crate::config::ScopeError>` at `error/mod.rs:253-257`
  - `From<hickory_resolver::net::NetError>` at `error/mod.rs:259-263`
  - `From<std::num::ParseIntError>` at `error/mod.rs:329-333`
  - `From<tokio::sync::AcquireError>` at `error/mod.rs:335-339`
  - `From<quick_xml::Error>` at `error/mod.rs:341-345`
  - `From<maxminddb::MaxMindDbError>` at `error/mod.rs:347-351`
  - `From<reqwest::header::InvalidHeaderValue>` at `error/mod.rs:353-357`
  (priority: high)

- **[Item]: Document feature-gated From impls**: Three `From` impls are feature-gated and not mentioned:
  - `From<crate::ai::AiError>` under `#[cfg(feature = "ai-integration")]` at `error/mod.rs:275-313`
  - `From<crate::packet::CaptureError>` under `#[cfg(feature = "packet-inspection")]` at `error/mod.rs:315-320`
  - `From<crate::packet::TracerouteError>` under `#[cfg(any(feature = "packet-inspection", feature = "stress-testing"))]` at `error/mod.rs:322-327`
  (priority: high)

- **[Item]: Update variant count from "19+" to "22"**: The doc says "19+ variants" at lines 5 and 11. The actual count is 22, which is more than 19 but the "+" makes it vague. An exact count would be more useful. (priority: medium)

- **[Item]: Document reqwest::Error conversion logic**: The `From<reqwest::Error>` impl at `error/mod.rs:172-200` has non-trivial conversion logic -- it distinguishes timeout, connection, HTTP status, and generic errors. This behavioral detail is useful for consumers. (priority: medium)

- **[Item]: Document ScopeError -> SlapperError conversion**: The `From<ScopeError>` impl at `error/mod.rs:253-257` converts all scope errors to `SlapperError::ScopeViolation`. This means scope error details (Validation, FileRead, Parse, etc.) are flattened into a single variant string. This is an important design trade-off worth documenting. (priority: medium)

- **[Item]: Document the `with_timeout()` builder pattern**: The `with_timeout()` method at `error/mod.rs:158-167` is a builder that returns `self` for chaining. This is a non-obvious pattern that could benefit from documentation. (priority: low)

## Stale Items

- **[Implementation Status] "Fully implemented"**: The claim at line 49 that the module is "fully implemented" should be verified against open issues or planned additions. As of this review, the module is complete and functional. (priority: low)
