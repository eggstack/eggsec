//! Canonical operation request contracts — Phase C convergence.
//!
//! This module is the single dependency-light owner for execution parameter
//! defaults, validation, and normalization shared across CLI, TUI, runtime,
//! tool/protocol, and Python surfaces.
//!
//! # Ownership
//!
//! - Canonical defaults live here as `pub const DEFAULT_*`.
//! - Normalization/validation lives here as pure functions
//!   (`parse_port_spec`, `resolve_load_test_counts`, `parse_scan_type`,
//!   `normalize_timeout_*`, `normalize_concurrency`, `parse_scan_profile`,
//!   `normalize_target_value`).
//! - Typed request structs per operation family carry serde support for
//!   runtime/protocol/Python schemas and expose `normalize()` which applies
//!   defaults + bounds and returns a validated `Normalized*` value.
//! - This module contains **no authorization decision**. It produces a
//!   normalized request; `eggsec::config::EnforcementContext` still decides
//!   authorization via `OperationMetadata` + `ApprovedOperation`.
//!
//! Frontend parse DTOs (Clap args, TUI state, runtime `TaskKind` params,
//! `ToolRequest.params` JSON, Python DTOs) must be shallow adapters into
//! these canonical types. Policy-relevant and execution-relevant
//! normalization must happen here, once.

use serde::{Deserialize, Serialize};

// ── Canonical defaults (single owner) ──

/// Default port spec for port scanning. Matches CLI `PortScanArgs` default.
pub const DEFAULT_PORT_SCAN_PORTS: &str = "1-1024";
/// Default port list for service fingerprinting. Matches CLI `FingerprintArgs`.
pub const DEFAULT_FINGERPRINT_PORTS: &str = "80,443,22,21,25,3306,5432,6379,27017";
/// Maximum ports allowed in a single normalized port spec.
pub const MAX_PORT_COUNT: usize = 65_535;

/// Default concurrency per family (canonical owner).
pub const DEFAULT_PORT_SCAN_CONCURRENCY: usize = 100;
pub const DEFAULT_ENDPOINT_CONCURRENCY: usize = 20;
pub const DEFAULT_FINGERPRINT_CONCURRENCY: usize = 20;
pub const DEFAULT_FUZZ_CONCURRENCY: usize = 10;
pub const DEFAULT_GRAPHQL_CONCURRENCY: usize = 10;
pub const DEFAULT_OAUTH_CONCURRENCY: usize = 10;
pub const DEFAULT_WAF_CONCURRENCY: usize = 10;
pub const DEFAULT_WAF_STRESS_CONCURRENCY: usize = 20;
pub const DEFAULT_LOAD_CONCURRENCY: usize = 10;
pub const DEFAULT_AUTH_CONCURRENCY: usize = 1;

/// Default timeouts in seconds (canonical owner; mirrors CLI `timeout.rs`).
pub const DEFAULT_PORT_SCAN_TIMEOUT_SECS: u64 = 2;
pub const DEFAULT_ENDPOINT_TIMEOUT_SECS: u64 = 10;
pub const DEFAULT_FINGERPRINT_TIMEOUT_SECS: u64 = 5;
pub const DEFAULT_FUZZ_TIMEOUT_SECS: u64 = 10;
pub const DEFAULT_WAF_TIMEOUT_SECS: u64 = 15;
pub const DEFAULT_GRAPHQL_TIMEOUT_SECS: u64 = 15;
pub const DEFAULT_OAUTH_TIMEOUT_SECS: u64 = 15;
pub const DEFAULT_AUTH_TIMEOUT_SECS: u64 = 10;
pub const DEFAULT_LOAD_DURATION_SECS: u64 = 30;
pub const DEFAULT_HUNT_TIMEOUT_SECS: u64 = 30;

/// Timeout/concurrency bounds enforced during normalization.
pub const MIN_TIMEOUT_SECS: u64 = 1;
pub const MAX_TIMEOUT_SECS: u64 = 600;
pub const MIN_TIMEOUT_MS: u64 = 100;
pub const MAX_TIMEOUT_MS: u64 = 600_000;
pub const MIN_CONCURRENCY: usize = 1;
pub const MAX_CONCURRENCY: usize = 1000;

/// Default load-test counts.
pub const DEFAULT_LOAD_REQUESTS: u64 = 100;

/// Default fuzz settings (canonical owner; matches CLI `FuzzArgs`).
pub const DEFAULT_FUZZ_PAYLOAD_TYPE: &str = "all";
pub const DEFAULT_FUZZ_MODE: &str = "sequential";
pub const DEFAULT_FUZZ_METHOD: &str = "GET";
pub const DEFAULT_FUZZ_MUTATION_COUNT: usize = 3;

/// Default load-test request body cap. Bodies are operator-supplied request
/// payloads; an unbounded body is a wire-exposure amplifier for daemon/agent
/// clients, so it is capped rather than trusted.
pub const MAX_LOAD_BODY_BYTES: usize = 64 * 1024;
/// Maximum request headers accepted on a load-test run.
pub const MAX_LOAD_HEADERS: usize = 32;
/// Maximum length of a single header field name.
pub const MAX_LOAD_HEADER_NAME_BYTES: usize = 64;
/// Maximum length of a single header field value.
pub const MAX_LOAD_HEADER_VALUE_BYTES: usize = 4096;

/// Methods whose semantics forbid a request body (RFC 9110 §9.3.1/§9.3.2).
pub const BODYLESS_HTTP_METHODS: &[&str] = &["GET", "HEAD"];

/// Header field names a load-test caller may not set (case-insensitive).
///
/// * `Host` — belongs to the transport, which derives it from the authorized
///   scope target; letting a request override it would retarget the traffic.
/// * Hop-by-hop headers (RFC 9110 §7.6.1) — connection-scoped, not forwarded
///   by intermediaries, and `Connection`/`Transfer-Encoding` in particular can
///   desync framing.
/// * `Content-Length` — computed by the transport from the body it sends.
pub const DENIED_LOAD_HEADERS: &[&str] = &[
    "host",
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
    "content-length",
];

/// Default GraphQL introspection flags (canonical owner; matches CLI defaults
/// of `true` for introspection/depth-bypass/alias-overload).
pub const DEFAULT_GRAPHQL_INTROSPECTION: bool = true;
pub const DEFAULT_GRAPHQL_DEPTH_BYPASS: bool = true;
pub const DEFAULT_GRAPHQL_ALIAS_OVERLOAD: bool = true;

/// Default OAuth test flags (canonical owner; matches CLI defaults of `true`).
pub const DEFAULT_OAUTH_REDIRECT_TEST: bool = true;
pub const DEFAULT_OAUTH_SCOPE_TEST: bool = true;
pub const DEFAULT_OAUTH_STATE_TEST: bool = true;
pub const DEFAULT_OAUTH_GRANT_TEST: bool = true;

/// Default scan profile name.
pub const DEFAULT_SCAN_PROFILE: &str = "quick";

/// Known scan profile names (must stay in sync with `ScanProfile`).
pub const KNOWN_SCAN_PROFILES: &[&str] = &[
    "quick",
    "endpoint",
    "web",
    "waf",
    "full",
    "api",
    "recon",
    "stealth",
    "deep",
    "vuln",
    "auth",
    "defense-lab",
];

// ── Errors ──

/// Normalization/validation failure. No authorization semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizationError(pub String);

impl std::fmt::Display for NormalizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for NormalizationError {}

fn err(msg: impl Into<String>) -> NormalizationError {
    NormalizationError(msg.into())
}

// ── Typed enums (no free-form strings on stable paths) ──

/// TCP scan type for port scanning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanType {
    #[default]
    Syn,
    Null,
    Fin,
    Xmas,
}

impl ScanType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Syn => "syn",
            Self::Null => "null",
            Self::Fin => "fin",
            Self::Xmas => "xmas",
        }
    }
}

/// Parse a scan-type string (case-insensitive). `None`/empty means default.
pub fn parse_scan_type(raw: Option<&str>) -> Result<ScanType, NormalizationError> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(ScanType::Syn),
        Some(s) => match s.to_ascii_lowercase().as_str() {
            "syn" => Ok(ScanType::Syn),
            "null" => Ok(ScanType::Null),
            "fin" => Ok(ScanType::Fin),
            "xmas" => Ok(ScanType::Xmas),
            other => Err(err(format!(
                "unknown scan-type '{other}' (expected syn|null|fin|xmas)"
            ))),
        },
    }
}

/// Canonical scan profile name (validated, no silent fallback).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanProfileName(pub String);

impl ScanProfileName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Parse a scan profile name. Rejects unknown profiles instead of silently
/// falling back to `quick` (legacy `dispatch_inner` behavior).
pub fn parse_scan_profile(raw: Option<&str>) -> Result<ScanProfileName, NormalizationError> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(ScanProfileName(DEFAULT_SCAN_PROFILE.to_string())),
        Some(s) => {
            let lowered = s.to_ascii_lowercase();
            if KNOWN_SCAN_PROFILES.contains(&lowered.as_str()) {
                Ok(ScanProfileName(lowered))
            } else {
                Err(err(format!(
                    "unknown scan profile '{s}' (expected one of: {})",
                    KNOWN_SCAN_PROFILES.join(", ")
                )))
            }
        }
    }
}

// ── Primitive normalization ──

/// Normalize a target value: trim whitespace, reject empty.
pub fn normalize_target_value(raw: &str) -> Result<String, NormalizationError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(err("target must not be empty"));
    }
    Ok(trimmed.to_string())
}

/// Normalize timeout in seconds into bounds.
pub fn normalize_timeout_secs(raw: Option<u64>, default: u64) -> Result<u64, NormalizationError> {
    let v = raw.unwrap_or(default);
    if v < MIN_TIMEOUT_SECS {
        return Err(err(format!(
            "timeout {v}s below minimum {MIN_TIMEOUT_SECS}s"
        )));
    }
    if v > MAX_TIMEOUT_SECS {
        return Err(err(format!(
            "timeout {v}s above maximum {MAX_TIMEOUT_SECS}s"
        )));
    }
    Ok(v)
}

/// Normalize timeout in milliseconds into bounds.
pub fn normalize_timeout_ms(raw: Option<u64>, default_ms: u64) -> Result<u64, NormalizationError> {
    let v = raw.unwrap_or(default_ms);
    if v < MIN_TIMEOUT_MS {
        return Err(err(format!(
            "timeout {v}ms below minimum {MIN_TIMEOUT_MS}ms"
        )));
    }
    if v > MAX_TIMEOUT_MS {
        return Err(err(format!(
            "timeout {v}ms above maximum {MAX_TIMEOUT_MS}ms"
        )));
    }
    Ok(v)
}

/// Normalize concurrency into bounds.
pub fn normalize_concurrency(
    raw: Option<usize>,
    default: usize,
) -> Result<usize, NormalizationError> {
    let v = raw.unwrap_or(default);
    if v < MIN_CONCURRENCY {
        return Err(err(format!(
            "concurrency {v} below minimum {MIN_CONCURRENCY}"
        )));
    }
    if v > MAX_CONCURRENCY {
        return Err(err(format!(
            "concurrency {v} above maximum {MAX_CONCURRENCY}"
        )));
    }
    Ok(v)
}

/// Parsed port spec: normalized spec string + port count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedPortSpec {
    pub normalized: String,
    pub count: usize,
}

/// Parse and validate a port spec like `1-1024` or `22,80,443`.
///
/// Returns the normalized spec and the port count. Enforces the
/// `MAX_PORT_COUNT` bound. Empty specs are rejected.
pub fn parse_port_spec(
    raw: Option<&str>,
    default: &str,
) -> Result<ParsedPortSpec, NormalizationError> {
    let spec = raw
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(default)
        .trim();
    if spec.is_empty() {
        return Err(err("port spec must not be empty"));
    }
    let mut count: usize = 0;
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            return Err(err(format!("invalid empty entry in port spec '{spec}'")));
        }
        if let Some((start_s, end_s)) = part.split_once('-') {
            let start: u32 = start_s
                .trim()
                .parse()
                .map_err(|_| err(format!("invalid port range '{part}' in '{spec}'")))?;
            let end: u32 = end_s
                .trim()
                .parse()
                .map_err(|_| err(format!("invalid port range '{part}' in '{spec}'")))?;
            if start == 0 || end == 0 || start > 65_535 || end > 65_535 {
                return Err(err(format!("port out of range in '{part}' (1-65535)")));
            }
            if start > end {
                return Err(err(format!("invalid port range '{part}' (start > end)")));
            }
            count = count.saturating_add((end - start + 1) as usize);
        } else {
            let port: u32 = part
                .parse()
                .map_err(|_| err(format!("invalid port '{part}' in '{spec}'")))?;
            if port == 0 || port > 65_535 {
                return Err(err(format!("port out of range '{part}' (1-65535)")));
            }
            count = count.saturating_add(1);
        }
        if count > MAX_PORT_COUNT {
            return Err(err(format!(
                "port spec '{spec}' exceeds maximum {MAX_PORT_COUNT} ports"
            )));
        }
    }
    if count == 0 {
        return Err(err(format!("port spec '{spec}' selects no ports")));
    }
    Ok(ParsedPortSpec {
        normalized: spec.to_string(),
        count,
    })
}

/// Resolve load-test `requests` vs legacy `connections` semantics.
///
/// Canonical rule (single owner): explicit `requests` wins; otherwise legacy
/// `connections` is treated as the total request count; otherwise
/// `DEFAULT_LOAD_REQUESTS`. Concurrency defaults separately to
/// `DEFAULT_LOAD_CONCURRENCY`.
pub fn resolve_load_test_counts(
    requests: Option<u64>,
    connections: Option<u32>,
) -> Result<(u64, usize), NormalizationError> {
    let total = match (requests, connections) {
        (Some(r), _) => r,
        (None, Some(c)) => u64::from(c),
        (None, None) => DEFAULT_LOAD_REQUESTS,
    };
    if total == 0 {
        return Err(err("load-test requests must be greater than 0"));
    }
    if total > 10_000_000 {
        return Err(err(format!(
            "load-test requests {total} above maximum 10000000"
        )));
    }
    let concurrency = connections.unwrap_or(DEFAULT_LOAD_CONCURRENCY as u32) as usize;
    let concurrency = normalize_concurrency(Some(concurrency), DEFAULT_LOAD_CONCURRENCY)?;
    Ok((total, concurrency))
}

/// Is `s` a valid RFC 9110 `token` (header field name / method grammar)?
pub fn is_http_token(s: &str) -> bool {
    !s.is_empty()
        && s.bytes().all(|b| {
            b.is_ascii_alphanumeric()
                || matches!(
                    b,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'|'
                        | b'~'
                )
        })
}

/// Normalize an HTTP method to canonical uppercase form.
///
/// Blank input falls back to [`DEFAULT_FUZZ_METHOD`]. The value must be an RFC
/// 9110 token: methods are case-sensitive on the wire, but every method in
/// general use is uppercase, so `post` and `POST` are the same operator intent
/// and canonicalizing here keeps the TUI, CLI, REST and MCP surfaces from
/// diverging on casing alone.
pub fn normalize_http_method(raw: Option<&str>) -> Result<String, NormalizationError> {
    let method = raw
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_FUZZ_METHOD)
        .to_ascii_uppercase();
    if !is_http_token(&method) {
        return Err(err(format!(
            "load-test method {method:?} is not a valid HTTP token"
        )));
    }
    Ok(method)
}

/// Validate a load-test request body against the resolved method.
///
/// Bodies are rejected for bodyless methods rather than silently dropped, so a
/// caller that expects its payload to be sent learns that it is not.
pub fn normalize_load_test_body(
    method: &str,
    body: Option<&str>,
) -> Result<Option<String>, NormalizationError> {
    let Some(body) = body.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    if BODYLESS_HTTP_METHODS.contains(&method) {
        return Err(err(format!(
            "load-test body is not allowed with method {method}: \
             use POST, PUT, PATCH or DELETE"
        )));
    }
    if body.len() > MAX_LOAD_BODY_BYTES {
        return Err(err(format!(
            "load-test body {} bytes above maximum {MAX_LOAD_BODY_BYTES}",
            body.len()
        )));
    }
    Ok(Some(body.to_string()))
}

/// Validate and canonicalize load-test request headers to `Name: Value` form.
///
/// Each entry must be a single `Name: Value` header. Names that would retarget
/// the request, break connection framing, or are connection-scoped are
/// refused — these arrive over the wire, so a daemon/agent client must not be
/// able to inject them. CR/LF in a value is refused because it is the header
/// injection primitive.
pub fn normalize_load_test_headers(raw: &[String]) -> Result<Vec<String>, NormalizationError> {
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    if raw.len() > MAX_LOAD_HEADERS {
        return Err(err(format!(
            "load-test headers {} above maximum {MAX_LOAD_HEADERS}",
            raw.len()
        )));
    }
    let mut out = Vec::with_capacity(raw.len());
    for entry in raw {
        let (name, value) = entry.split_once(':').ok_or_else(|| {
            err(format!(
                "load-test header {entry:?} is missing ':' (expected \"Name: Value\")"
            ))
        })?;
        let name = name.trim();
        let value = value.trim();
        if !is_http_token(name) {
            return Err(err(format!(
                "load-test header name {name:?} is not a valid HTTP token"
            )));
        }
        if name.len() > MAX_LOAD_HEADER_NAME_BYTES {
            return Err(err(format!(
                "load-test header name {} bytes above maximum {MAX_LOAD_HEADER_NAME_BYTES}",
                name.len()
            )));
        }
        if DENIED_LOAD_HEADERS.contains(&name.to_ascii_lowercase().as_str()) {
            return Err(err(format!(
                "load-test header {name} is not settable by callers"
            )));
        }
        if value.len() > MAX_LOAD_HEADER_VALUE_BYTES {
            return Err(err(format!(
                "load-test header {name} value {} bytes above maximum \
                 {MAX_LOAD_HEADER_VALUE_BYTES}",
                value.len()
            )));
        }
        if value.contains(['\r', '\n']) {
            return Err(err(format!(
                "load-test header {name} value must not contain CR or LF"
            )));
        }
        out.push(format!("{name}:{value}"));
    }
    Ok(out)
}

// ── Canonical typed requests ──

/// Canonical port-scan request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortScanRequest {
    pub target: String,
    #[serde(default)]
    pub ports: Option<String>,
    /// TCP technique. A sibling of `udp`, not a value of it: `syn`/`null`/
    /// `fin`/`xmas` describe how a TCP handshake is attempted, so folding
    /// `udp` in here would let a scan type bypass technique validation and be
    /// silently swallowed by the engine's `_ => Syn` fallback.
    #[serde(default)]
    pub scan_type: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub concurrency: Option<usize>,
    /// Scan with UDP instead of TCP.
    #[serde(default)]
    pub udp: Option<bool>,
}

/// Normalized port-scan request (validated, defaults applied).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedPortScan {
    pub target: String,
    pub ports: String,
    pub port_count: usize,
    pub scan_type: ScanType,
    pub timeout_ms: u64,
    pub concurrency: usize,
    /// Whether to scan UDP instead of TCP.
    ///
    /// UDP cannot prove a port is open: silence is ambiguous between open,
    /// filtered, rate-limited-closed and host-down. A UDP result is therefore
    /// `closed`/`filtered`/`open|filtered` with a host-liveness verdict, and
    /// never a bare "open".
    pub udp: bool,
}

impl PortScanRequest {
    pub fn normalize(&self) -> Result<NormalizedPortScan, NormalizationError> {
        let target = normalize_target_value(&self.target)?;
        let parsed = parse_port_spec(self.ports.as_deref(), DEFAULT_PORT_SCAN_PORTS)?;
        let scan_type = parse_scan_type(self.scan_type.as_deref())?;
        let timeout_ms =
            normalize_timeout_ms(self.timeout_ms, DEFAULT_PORT_SCAN_TIMEOUT_SECS * 1000)?;
        let concurrency = normalize_concurrency(self.concurrency, DEFAULT_PORT_SCAN_CONCURRENCY)?;
        Ok(NormalizedPortScan {
            target,
            ports: parsed.normalized,
            port_count: parsed.count,
            scan_type,
            timeout_ms,
            concurrency,
            udp: self.udp.unwrap_or(false),
        })
    }

    pub fn operation_id(&self) -> &'static str {
        "scan-ports"
    }
}

/// Canonical endpoint-scan request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointScanRequest {
    pub target: String,
    #[serde(default)]
    pub concurrency: Option<usize>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub wordlist: Option<String>,
    /// Keep 404 responses in the result set. Absent means the engine default
    /// (exclude), preserving the CLI's opt-in `--include-404` semantics.
    #[serde(default)]
    pub include_404: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedEndpointScan {
    pub target: String,
    pub concurrency: usize,
    pub timeout_secs: u64,
    pub wordlist: Option<String>,
    pub include_404: bool,
}

impl EndpointScanRequest {
    pub fn normalize(&self) -> Result<NormalizedEndpointScan, NormalizationError> {
        Ok(NormalizedEndpointScan {
            target: normalize_target_value(&self.target)?,
            concurrency: normalize_concurrency(self.concurrency, DEFAULT_ENDPOINT_CONCURRENCY)?,
            timeout_secs: normalize_timeout_secs(self.timeout_secs, DEFAULT_ENDPOINT_TIMEOUT_SECS)?,
            wordlist: self.wordlist.clone().filter(|s| !s.trim().is_empty()),
            include_404: self.include_404.unwrap_or(false),
        })
    }

    pub fn operation_id(&self) -> &'static str {
        "scan-endpoints"
    }
}

/// Canonical fingerprint request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FingerprintRequest {
    pub target: String,
    #[serde(default)]
    pub ports: Option<String>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub concurrency: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedFingerprint {
    pub target: String,
    pub ports: String,
    pub port_count: usize,
    pub timeout_secs: u64,
    pub concurrency: usize,
}

impl FingerprintRequest {
    pub fn normalize(&self) -> Result<NormalizedFingerprint, NormalizationError> {
        let target = normalize_target_value(&self.target)?;
        let parsed = parse_port_spec(self.ports.as_deref(), DEFAULT_FINGERPRINT_PORTS)?;
        Ok(NormalizedFingerprint {
            target,
            ports: parsed.normalized,
            port_count: parsed.count,
            timeout_secs: normalize_timeout_secs(
                self.timeout_secs,
                DEFAULT_FINGERPRINT_TIMEOUT_SECS,
            )?,
            concurrency: normalize_concurrency(self.concurrency, DEFAULT_FINGERPRINT_CONCURRENCY)?,
        })
    }

    pub fn operation_id(&self) -> &'static str {
        "fingerprint"
    }
}

/// Canonical fuzz request (core subset; advanced flags pass through).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FuzzRequest {
    pub target: String,
    #[serde(default)]
    pub payload_type: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub param: Option<String>,
    #[serde(default)]
    pub threads: Option<u32>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub mutations: Option<bool>,
    #[serde(default)]
    pub mutation_count: Option<usize>,
    #[serde(default)]
    pub graphql_introspection: Option<bool>,
    #[serde(default)]
    pub graphql_depth_bypass: Option<bool>,
    #[serde(default)]
    pub graphql_alias_overload: Option<bool>,
    #[serde(default)]
    pub oauth_redirect_test: Option<bool>,
    #[serde(default)]
    pub oauth_scope_test: Option<bool>,
    #[serde(default)]
    pub oauth_state_test: Option<bool>,
    #[serde(default)]
    pub oauth_grant_test: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedFuzz {
    pub target: String,
    pub payload_type: String,
    pub mode: String,
    pub method: String,
    pub param: Option<String>,
    pub threads: usize,
    pub timeout_secs: u64,
    pub mutations: bool,
    pub mutation_count: usize,
    pub graphql_introspection: bool,
    pub graphql_depth_bypass: bool,
    pub graphql_alias_overload: bool,
    pub oauth_redirect_test: bool,
    pub oauth_scope_test: bool,
    pub oauth_state_test: bool,
    pub oauth_grant_test: bool,
}

impl FuzzRequest {
    pub fn normalize(&self) -> Result<NormalizedFuzz, NormalizationError> {
        let target = normalize_target_value(&self.target)?;
        let payload_type = self
            .payload_type
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_FUZZ_PAYLOAD_TYPE.to_string());
        let mode = self
            .mode
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_FUZZ_MODE.to_string());
        let method = self
            .method
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_FUZZ_METHOD.to_string());
        let threads =
            normalize_concurrency(self.threads.map(|v| v as usize), DEFAULT_FUZZ_CONCURRENCY)?;
        let timeout_secs = normalize_timeout_secs(self.timeout_secs, DEFAULT_FUZZ_TIMEOUT_SECS)?;
        let mutations = self.mutations.unwrap_or(false);
        // Canonical default is 3 (matches CLI). An explicit 0 disables mutations.
        let mutation_count = self.mutation_count.unwrap_or(DEFAULT_FUZZ_MUTATION_COUNT);
        Ok(NormalizedFuzz {
            target,
            payload_type,
            mode,
            method,
            param: self.param.clone().filter(|s| !s.trim().is_empty()),
            threads,
            timeout_secs,
            mutations,
            mutation_count,
            graphql_introspection: self
                .graphql_introspection
                .unwrap_or(DEFAULT_GRAPHQL_INTROSPECTION),
            graphql_depth_bypass: self
                .graphql_depth_bypass
                .unwrap_or(DEFAULT_GRAPHQL_DEPTH_BYPASS),
            graphql_alias_overload: self
                .graphql_alias_overload
                .unwrap_or(DEFAULT_GRAPHQL_ALIAS_OVERLOAD),
            oauth_redirect_test: self
                .oauth_redirect_test
                .unwrap_or(DEFAULT_OAUTH_REDIRECT_TEST),
            oauth_scope_test: self.oauth_scope_test.unwrap_or(DEFAULT_OAUTH_SCOPE_TEST),
            oauth_state_test: self.oauth_state_test.unwrap_or(DEFAULT_OAUTH_STATE_TEST),
            oauth_grant_test: self.oauth_grant_test.unwrap_or(DEFAULT_OAUTH_GRANT_TEST),
        })
    }

    pub fn operation_id(&self) -> &'static str {
        "fuzz"
    }
}

/// Canonical WAF detection request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WafDetectRequest {
    pub target: String,
    #[serde(default)]
    pub bypass_mode: Option<bool>,
    #[serde(default)]
    pub techniques: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedWafDetect {
    pub target: String,
    pub bypass_mode: bool,
    pub techniques: Vec<String>,
}

impl WafDetectRequest {
    pub fn normalize(&self) -> Result<NormalizedWafDetect, NormalizationError> {
        Ok(NormalizedWafDetect {
            target: normalize_target_value(&self.target)?,
            bypass_mode: self.bypass_mode.unwrap_or(false),
            techniques: self.techniques.clone().unwrap_or_default(),
        })
    }

    pub fn operation_id(&self) -> &'static str {
        "waf-detect"
    }
}

/// Canonical WAF stress request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WafStressRequest {
    pub target: String,
    #[serde(default)]
    pub requests: Option<u32>,
    #[serde(default)]
    pub concurrency: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedWafStress {
    pub target: String,
    pub requests: u64,
    pub concurrency: usize,
}

impl WafStressRequest {
    pub fn normalize(&self) -> Result<NormalizedWafStress, NormalizationError> {
        let target = normalize_target_value(&self.target)?;
        let requests = u64::from(self.requests.unwrap_or(100));
        if requests == 0 {
            return Err(err("waf-stress requests must be greater than 0"));
        }
        Ok(NormalizedWafStress {
            target,
            requests,
            concurrency: normalize_concurrency(self.concurrency, DEFAULT_WAF_STRESS_CONCURRENCY)?,
        })
    }

    pub fn operation_id(&self) -> &'static str {
        "waf-stress"
    }
}

/// Canonical load-test request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadTestRequest {
    pub target: String,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub requests: Option<u64>,
    #[serde(default)]
    pub connections: Option<u32>,
    #[serde(default)]
    pub duration_secs: Option<u32>,
    #[serde(default)]
    pub rate_limit: Option<u32>,
    /// Optional request body. Rejected for bodyless methods by `normalize()`.
    #[serde(default)]
    pub body: Option<String>,
    /// Request headers as `Name: Value` entries. Validated by `normalize()`.
    #[serde(default)]
    pub headers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedLoadTest {
    pub target: String,
    pub method: String,
    pub requests: u64,
    pub concurrency: usize,
    pub duration_secs: u64,
    pub rate_limit: Option<u32>,
    pub body: Option<String>,
    pub headers: Vec<String>,
}

impl LoadTestRequest {
    pub fn normalize(&self) -> Result<NormalizedLoadTest, NormalizationError> {
        let target = normalize_target_value(&self.target)?;
        let (requests, concurrency) = resolve_load_test_counts(self.requests, self.connections)?;
        let duration_secs = normalize_timeout_secs(
            self.duration_secs.map(u64::from),
            DEFAULT_LOAD_DURATION_SECS,
        )?;
        let method = normalize_http_method(self.method.as_deref())?;
        let body = normalize_load_test_body(&method, self.body.as_deref())?;
        let headers = normalize_load_test_headers(&self.headers)?;
        Ok(NormalizedLoadTest {
            target,
            method,
            requests,
            concurrency,
            duration_secs,
            rate_limit: self.rate_limit,
            body,
            headers,
        })
    }

    pub fn operation_id(&self) -> &'static str {
        "load-test"
    }
}

/// Canonical recon request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconRequest {
    pub target: String,
    #[serde(default)]
    pub modules: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedRecon {
    pub target: String,
    pub modules: Vec<String>,
}

impl ReconRequest {
    pub fn normalize(&self) -> Result<NormalizedRecon, NormalizationError> {
        Ok(NormalizedRecon {
            target: normalize_target_value(&self.target)?,
            modules: self.modules.clone().unwrap_or_default(),
        })
    }

    pub fn operation_id(&self) -> &'static str {
        "recon"
    }
}

/// Canonical pipeline request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineRequest {
    pub target: String,
    #[serde(default)]
    pub profile: Option<String>,
    /// Report format for `output_file`. `None` means the engine default.
    #[serde(default)]
    pub output_format: Option<String>,
    /// Destination path for the rendered report, relative to the engine's
    /// configured export directory.
    #[serde(default)]
    pub output_file: Option<String>,
    /// Absolute path to write a resumable scan checkpoint to. `None` (the
    /// default) writes no checkpoint, so an ordinary scan leaves nothing
    /// behind on disk. Unlike `output_file` this is NOT relative to the export
    /// directory: a checkpoint is engine state, not a rendered artifact, and
    /// resolving it against an operator-chosen report directory would make the
    /// resume list undiscoverable.
    #[serde(default)]
    pub session_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedPipeline {
    pub target: String,
    pub profile: ScanProfileName,
    /// Always concrete: the writer never re-picks a default. Dispatch writes
    /// only when `output_file` is present; a format without a destination is
    /// not an error, just a no-op.
    pub output_format: PipelineOutputFormat,
    pub output_file: Option<String>,
    /// Carried through verbatim; `None` means "write no checkpoint".
    pub session_path: Option<String>,
}

/// Canonical pipeline report format.
///
/// This is the tool-core owner of the format contract. The engine's
/// `crate::types::OutputFormat` and `eggsec_output::OutputFormat` each declare
/// the same eight variants; this enum is what the wire speaks, and the engine
/// maps it to whichever of those the selected writer needs. Adding a variant
/// here requires adding it to both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PipelineOutputFormat {
    #[default]
    Pretty,
    Json,
    Compact,
    Html,
    Csv,
    Sarif,
    Junit,
    Markdown,
}

impl PipelineOutputFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pretty => "pretty",
            Self::Json => "json",
            Self::Compact => "compact",
            Self::Html => "html",
            Self::Csv => "csv",
            Self::Sarif => "sarif",
            Self::Junit => "junit",
            Self::Markdown => "markdown",
        }
    }
}

/// Parse a pipeline output format (case-insensitive). `None`/empty means the
/// engine default, which is HTML.
pub fn parse_pipeline_output_format(
    raw: Option<&str>,
) -> Result<PipelineOutputFormat, NormalizationError> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(PipelineOutputFormat::Html),
        Some(s) => match s.to_ascii_lowercase().as_str() {
            "pretty" => Ok(PipelineOutputFormat::Pretty),
            "json" => Ok(PipelineOutputFormat::Json),
            "compact" => Ok(PipelineOutputFormat::Compact),
            "html" => Ok(PipelineOutputFormat::Html),
            "csv" => Ok(PipelineOutputFormat::Csv),
            "sarif" => Ok(PipelineOutputFormat::Sarif),
            "junit" => Ok(PipelineOutputFormat::Junit),
            "markdown" | "md" => Ok(PipelineOutputFormat::Markdown),
            other => Err(err(format!(
                "unknown output format '{other}' (expected pretty|json|compact|html|csv|sarif|junit|markdown)"
            ))),
        },
    }
}

/// Lexically reject output paths that must never reach the filesystem.
///
/// This is the transport-side guard only: it catches the obvious abuse
/// (NUL truncation, `..` traversal, absurd length) without touching the
/// filesystem, which a DTO crate must not do. Containment within the engine's
/// export directory is enforced separately at dispatch by
/// `crate::utils::validation::validate_path`, which does resolve the real path.
pub fn normalize_output_path(raw: &str) -> Result<String, NormalizationError> {
    const MAX_OUTPUT_PATH_BYTES: usize = 4096;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(err("output_file must not be empty"));
    }
    if trimmed.len() > MAX_OUTPUT_PATH_BYTES {
        return Err(err(format!(
            "output_file {} bytes above maximum {MAX_OUTPUT_PATH_BYTES}",
            trimmed.len()
        )));
    }
    if trimmed.contains('\0') {
        return Err(err("output_file must not contain NUL"));
    }
    // A bare `..` is the traversal primitive; reject it anywhere in the path
    // rather than only at the head, so `a/../../b` cannot slip through.
    if std::path::Path::new(trimmed)
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(err("output_file must not contain '..' path components"));
    }
    Ok(trimmed.to_string())
}

impl PipelineRequest {
    pub fn normalize(&self) -> Result<NormalizedPipeline, NormalizationError> {
        let output_file = self
            .output_file
            .as_deref()
            .map(normalize_output_path)
            .transpose()?;
        Ok(NormalizedPipeline {
            target: normalize_target_value(&self.target)?,
            profile: parse_scan_profile(self.profile.as_deref())?,
            output_format: parse_pipeline_output_format(self.output_format.as_deref())?,
            output_file,
            session_path: self.session_path.clone(),
        })
    }

    pub fn operation_id(&self) -> &'static str {
        "pipeline"
    }
}

/// Canonical resume request.
///
/// Resumes a saved scan checkpoint. It declares the `pipeline` operation id
/// because it runs the same stage set, which is also how the `resume` CLI
/// command is routed and how its enforcement is resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeRequest {
    /// Path to the checkpoint to resume. The target is read from the
    /// checkpoint itself, not carried here.
    pub session_path: String,
}

impl ResumeRequest {
    pub fn operation_id(&self) -> &'static str {
        "pipeline"
    }
}

/// Canonical GraphQL request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphQlRequest {
    pub target: String,
    #[serde(default)]
    pub introspection: Option<bool>,
    #[serde(default)]
    pub inject: Option<bool>,
    #[serde(default)]
    pub depth_bypass: Option<bool>,
    #[serde(default)]
    pub alias_overload: Option<bool>,
    #[serde(default)]
    pub concurrency: Option<usize>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedGraphQl {
    pub target: String,
    pub introspection: bool,
    pub inject: bool,
    pub depth_bypass: bool,
    pub alias_overload: bool,
    pub concurrency: usize,
    pub timeout_secs: u64,
}

impl GraphQlRequest {
    pub fn normalize(&self) -> Result<NormalizedGraphQl, NormalizationError> {
        Ok(NormalizedGraphQl {
            target: normalize_target_value(&self.target)?,
            introspection: self.introspection.unwrap_or(DEFAULT_GRAPHQL_INTROSPECTION),
            inject: self.inject.unwrap_or(false),
            depth_bypass: self.depth_bypass.unwrap_or(DEFAULT_GRAPHQL_DEPTH_BYPASS),
            alias_overload: self
                .alias_overload
                .unwrap_or(DEFAULT_GRAPHQL_ALIAS_OVERLOAD),
            concurrency: normalize_concurrency(self.concurrency, DEFAULT_GRAPHQL_CONCURRENCY)?,
            timeout_secs: normalize_timeout_secs(self.timeout_secs, DEFAULT_GRAPHQL_TIMEOUT_SECS)?,
        })
    }

    pub fn operation_id(&self) -> &'static str {
        "graphql"
    }
}

/// Canonical OAuth request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthRequest {
    pub target: String,
    #[serde(default)]
    pub flow: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub redirect_uri: Option<String>,
    #[serde(default)]
    pub redirect_test: Option<bool>,
    #[serde(default)]
    pub scope_test: Option<bool>,
    #[serde(default)]
    pub state_test: Option<bool>,
    #[serde(default)]
    pub grant_test: Option<bool>,
    #[serde(default)]
    pub concurrency: Option<usize>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedOAuth {
    pub target: String,
    pub flow: Option<String>,
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
    pub redirect_test: bool,
    pub scope_test: bool,
    pub state_test: bool,
    pub grant_test: bool,
    pub concurrency: usize,
    pub timeout_secs: u64,
}

impl OAuthRequest {
    pub fn normalize(&self) -> Result<NormalizedOAuth, NormalizationError> {
        Ok(NormalizedOAuth {
            target: normalize_target_value(&self.target)?,
            flow: self.flow.clone().filter(|s| !s.trim().is_empty()),
            client_id: self.client_id.clone().filter(|s| !s.trim().is_empty()),
            redirect_uri: self.redirect_uri.clone().filter(|s| !s.trim().is_empty()),
            redirect_test: self.redirect_test.unwrap_or(DEFAULT_OAUTH_REDIRECT_TEST),
            scope_test: self.scope_test.unwrap_or(DEFAULT_OAUTH_SCOPE_TEST),
            state_test: self.state_test.unwrap_or(DEFAULT_OAUTH_STATE_TEST),
            grant_test: self.grant_test.unwrap_or(DEFAULT_OAUTH_GRANT_TEST),
            concurrency: normalize_concurrency(self.concurrency, DEFAULT_OAUTH_CONCURRENCY)?,
            timeout_secs: normalize_timeout_secs(self.timeout_secs, DEFAULT_OAUTH_TIMEOUT_SECS)?,
        })
    }

    pub fn operation_id(&self) -> &'static str {
        "oauth"
    }
}

/// Canonical auth-test request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthTestRequest {
    pub target: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub credential_list: Option<String>,
    #[serde(default)]
    pub credential_file: Option<String>,
    #[serde(default)]
    pub max_attempts: Option<usize>,
    #[serde(default)]
    pub concurrency: Option<usize>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedAuthTest {
    pub target: String,
    pub username: Option<String>,
    pub credential_list: Option<String>,
    pub credential_file: Option<String>,
    pub max_attempts: usize,
    pub concurrency: usize,
    pub timeout_secs: u64,
}

impl AuthTestRequest {
    pub fn normalize(&self) -> Result<NormalizedAuthTest, NormalizationError> {
        let max_attempts = self.max_attempts.unwrap_or(100);
        if max_attempts == 0 {
            return Err(err("auth-test max_attempts must be greater than 0"));
        }
        Ok(NormalizedAuthTest {
            target: normalize_target_value(&self.target)?,
            username: self.username.clone().filter(|s| !s.trim().is_empty()),
            credential_list: self
                .credential_list
                .clone()
                .filter(|s| !s.trim().is_empty()),
            credential_file: self
                .credential_file
                .clone()
                .filter(|s| !s.trim().is_empty()),
            max_attempts,
            concurrency: normalize_concurrency(self.concurrency, DEFAULT_AUTH_CONCURRENCY)?,
            timeout_secs: normalize_timeout_secs(self.timeout_secs, DEFAULT_AUTH_TIMEOUT_SECS)?,
        })
    }

    pub fn operation_id(&self) -> &'static str {
        "auth-test"
    }
}

/// Canonical database-assessment request (representative feature-gated,
/// high-risk domain for matrix coverage).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbPentestRequest {
    pub target: String,
    pub db_type: String,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub checks: Option<String>,
    #[serde(default)]
    pub max_queries: Option<u64>,
    #[serde(default)]
    pub max_duration_secs: Option<u64>,
    #[serde(default)]
    pub dry_run: Option<bool>,
    #[serde(default)]
    pub allow_advanced: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedDbPentest {
    pub target: String,
    pub db_type: String,
    pub port: Option<u16>,
    pub checks: String,
    pub max_queries: u64,
    pub max_duration_secs: u64,
    pub dry_run: bool,
    pub allow_advanced: bool,
}

impl DbPentestRequest {
    pub fn normalize(&self) -> Result<NormalizedDbPentest, NormalizationError> {
        let target = normalize_target_value(&self.target)?;
        let db_type = self.db_type.trim().to_ascii_lowercase();
        if db_type.is_empty() {
            return Err(err("db-pentest db_type must not be empty"));
        }
        match db_type.as_str() {
            "postgres" | "postgresql" | "mysql" | "mssql" | "mongodb" | "mongo" | "redis" => {}
            other => {
                return Err(err(format!(
                    "unknown db_type '{other}' (expected postgres|mysql|mssql|mongodb|redis)"
                )))
            }
        }
        Ok(NormalizedDbPentest {
            target,
            db_type,
            port: self.port,
            checks: self
                .checks
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "all".to_string()),
            max_queries: self.max_queries.unwrap_or(200),
            max_duration_secs: self.max_duration_secs.unwrap_or(120),
            dry_run: self.dry_run.unwrap_or(true),
            allow_advanced: self.allow_advanced.unwrap_or(false),
        })
    }

    pub fn operation_id(&self) -> &'static str {
        "db-pentest"
    }
}

/// Canonical local-file request (representative local-file domain for matrix
/// coverage). Storage operations carry no network target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageRequest {
    pub storage_type: String,
    #[serde(default)]
    pub path: Option<String>,
    /// Database host. Defaults to `localhost`.
    #[serde(default)]
    pub host: Option<String>,
    /// Database port. Defaults to 5432.
    #[serde(default)]
    pub port: Option<u16>,
    /// Database name. Defaults to `eggsec`.
    #[serde(default)]
    pub database: Option<String>,
    /// Database user. Defaults to `postgres`.
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub max_connections: Option<u32>,
    /// Storage operation. Must be one of [`KNOWN_STORAGE_MODES`].
    #[serde(default)]
    pub mode: Option<String>,
    /// Scan to scope `list_findings` to.
    #[serde(default)]
    pub scan_id: Option<String>,
    /// CVE identifier for `search_cve`.
    #[serde(default)]
    pub cve_id: Option<String>,
    /// Minimum severity filter for `list_findings`.
    #[serde(default)]
    pub severity_filter: Option<String>,
    /// Name of the environment variable holding the database password.
    ///
    /// The password itself is never a wire field: see
    /// [`NormalizedStorage::password_env`].
    #[serde(default)]
    pub password_env: Option<String>,
}

/// Storage operations the engine actually implements.
pub const KNOWN_STORAGE_MODES: &[&str] = &["connect", "list_scans", "list_findings", "search_cve"];

/// Default storage mode when the caller does not choose one.
pub const DEFAULT_STORAGE_MODE: &str = "list_scans";

/// Storage connection defaults (canonical owner; matches
/// `crate::storage::StorageConfig::default()`).
pub const DEFAULT_STORAGE_HOST: &str = "localhost";
pub const DEFAULT_STORAGE_PORT: u16 = 5432;
pub const DEFAULT_STORAGE_DATABASE: &str = "eggsec";
pub const DEFAULT_STORAGE_USERNAME: &str = "postgres";
pub const DEFAULT_STORAGE_MAX_CONNECTIONS: u32 = 10;
pub const MAX_STORAGE_MAX_CONNECTIONS: u32 = 256;

/// Severity names accepted by `severity_filter` (lowercase wire form of
/// `eggsec_core::types::Severity`).
pub const KNOWN_SEVERITIES: &[&str] = &["critical", "high", "medium", "low", "info"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedStorage {
    pub storage_type: String,
    pub path: Option<String>,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub max_connections: u32,
    pub mode: String,
    pub scan_id: Option<String>,
    pub cve_id: Option<String>,
    pub severity_filter: Option<String>,
    /// Environment-variable *name* carrying the password, never the secret.
    pub password_env: Option<String>,
}

/// Validate a `password_env` name: a portable POSIX environment-variable name.
///
/// Rejecting anything else keeps a caller from smuggling a value through a
/// field that only ever names an environment variable.
fn normalize_password_env(raw: &str) -> Result<String, NormalizationError> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(err("password_env must not be empty"));
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap_or('\0');
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(err(format!(
            "password_env {name:?} must start with a letter or underscore"
        )));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(err(format!(
            "password_env {name:?} may only contain letters, digits and underscores"
        )));
    }
    Ok(name.to_string())
}

impl StorageRequest {
    pub fn normalize(&self) -> Result<NormalizedStorage, NormalizationError> {
        let storage_type = self.storage_type.trim();
        if storage_type.is_empty() {
            return Err(err("storage_type must not be empty"));
        }

        // Fail closed on an unknown mode: the executor turns an unknown mode
        // into a runtime error *after* dialling the database, so validating
        // here is what stops a typo from opening a connection at all.
        let mode = self
            .mode
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(DEFAULT_STORAGE_MODE)
            .to_ascii_lowercase();
        if !KNOWN_STORAGE_MODES.contains(&mode.as_str()) {
            return Err(err(format!(
                "unknown storage mode '{mode}' (expected {})",
                KNOWN_STORAGE_MODES.join("|")
            )));
        }

        let port = self.port.unwrap_or(DEFAULT_STORAGE_PORT);
        if port == 0 {
            return Err(err("storage port must be greater than 0"));
        }

        let max_connections = self
            .max_connections
            .unwrap_or(DEFAULT_STORAGE_MAX_CONNECTIONS);
        if max_connections == 0 || max_connections > MAX_STORAGE_MAX_CONNECTIONS {
            return Err(err(format!(
                "storage max_connections {max_connections} must be between 1 and \
                 {MAX_STORAGE_MAX_CONNECTIONS}"
            )));
        }

        let severity_filter = self
            .severity_filter
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_ascii_lowercase);
        if let Some(ref sev) = severity_filter {
            if !KNOWN_SEVERITIES.contains(&sev.as_str()) {
                return Err(err(format!(
                    "unknown severity_filter '{sev}' (expected {})",
                    KNOWN_SEVERITIES.join("|")
                )));
            }
        }

        let cve_id = self
            .cve_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        if mode == "search_cve" && cve_id.is_none() {
            return Err(err("storage mode 'search_cve' requires cve_id"));
        }

        let password_env = self
            .password_env
            .as_deref()
            .map(normalize_password_env)
            .transpose()?;

        Ok(NormalizedStorage {
            storage_type: storage_type.to_string(),
            path: self.path.clone().filter(|s| !s.trim().is_empty()),
            host: self
                .host
                .clone()
                .map(|h| h.trim().to_string())
                .filter(|h| !h.is_empty())
                .unwrap_or_else(|| DEFAULT_STORAGE_HOST.to_string()),
            port,
            database: self
                .database
                .clone()
                .map(|d| d.trim().to_string())
                .filter(|d| !d.is_empty())
                .unwrap_or_else(|| DEFAULT_STORAGE_DATABASE.to_string()),
            username: self
                .username
                .clone()
                .map(|u| u.trim().to_string())
                .filter(|u| !u.is_empty())
                .unwrap_or_else(|| DEFAULT_STORAGE_USERNAME.to_string()),
            max_connections,
            mode,
            scan_id: self
                .scan_id
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            cve_id,
            severity_filter,
            password_env,
        })
    }

    pub fn operation_id(&self) -> &'static str {
        "storage"
    }
}

// ── Operation-owned JSON parsing (single owner for ToolRequest.params) ──

/// Parse `ToolRequest.params` JSON through operation-owned canonical types.
///
/// This is the single parsing entry point for tool/protocol/Python surfaces.
/// Wire JSON stays `serde_json::Value` at the boundary, but typed parsing
/// happens here — not in per-protocol ad hoc structs.
///
/// On success returns the canonical operation ID. Callers that need the
/// normalized request can then call the typed `normalize()` above.
pub fn validate_tool_params(
    operation_id: &str,
    params: &serde_json::Value,
) -> Result<&'static str, NormalizationError> {
    match operation_id {
        "scan-ports" => {
            let req: PortScanRequest = serde_json::from_value(params.clone())
                .map_err(|e| err(format!("scan-ports params invalid: {e}")))?;
            req.normalize()?;
            Ok("scan-ports")
        }
        "scan-endpoints" => {
            let req: EndpointScanRequest = serde_json::from_value(params.clone())
                .map_err(|e| err(format!("scan-endpoints params invalid: {e}")))?;
            req.normalize()?;
            Ok("scan-endpoints")
        }
        "fingerprint" => {
            let req: FingerprintRequest = serde_json::from_value(params.clone())
                .map_err(|e| err(format!("fingerprint params invalid: {e}")))?;
            req.normalize()?;
            Ok("fingerprint")
        }
        "fuzz" => {
            let req: FuzzRequest = serde_json::from_value(params.clone())
                .map_err(|e| err(format!("fuzz params invalid: {e}")))?;
            req.normalize()?;
            Ok("fuzz")
        }
        "waf-detect" => {
            let req: WafDetectRequest = serde_json::from_value(params.clone())
                .map_err(|e| err(format!("waf-detect params invalid: {e}")))?;
            req.normalize()?;
            Ok("waf-detect")
        }
        "waf-stress" => {
            let req: WafStressRequest = serde_json::from_value(params.clone())
                .map_err(|e| err(format!("waf-stress params invalid: {e}")))?;
            req.normalize()?;
            Ok("waf-stress")
        }
        "load-test" => {
            let req: LoadTestRequest = serde_json::from_value(params.clone())
                .map_err(|e| err(format!("load-test params invalid: {e}")))?;
            req.normalize()?;
            Ok("load-test")
        }
        "recon" => {
            let req: ReconRequest = serde_json::from_value(params.clone())
                .map_err(|e| err(format!("recon params invalid: {e}")))?;
            req.normalize()?;
            Ok("recon")
        }
        "pipeline" => {
            let req: PipelineRequest = serde_json::from_value(params.clone())
                .map_err(|e| err(format!("pipeline params invalid: {e}")))?;
            req.normalize()?;
            Ok("pipeline")
        }
        "graphql" => {
            let req: GraphQlRequest = serde_json::from_value(params.clone())
                .map_err(|e| err(format!("graphql params invalid: {e}")))?;
            req.normalize()?;
            Ok("graphql")
        }
        "oauth" => {
            let req: OAuthRequest = serde_json::from_value(params.clone())
                .map_err(|e| err(format!("oauth params invalid: {e}")))?;
            req.normalize()?;
            Ok("oauth")
        }
        "auth-test" => {
            let req: AuthTestRequest = serde_json::from_value(params.clone())
                .map_err(|e| err(format!("auth-test params invalid: {e}")))?;
            req.normalize()?;
            Ok("auth-test")
        }
        "db-pentest" => {
            let req: DbPentestRequest = serde_json::from_value(params.clone())
                .map_err(|e| err(format!("db-pentest params invalid: {e}")))?;
            req.normalize()?;
            Ok("db-pentest")
        }
        "storage" => {
            let req: StorageRequest = serde_json::from_value(params.clone())
                .map_err(|e| err(format!("storage params invalid: {e}")))?;
            req.normalize()?;
            Ok("storage")
        }
        other => Err(err(format!("unknown operation '{other}'"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_spec_defaults_and_bounds() {
        let parsed = parse_port_spec(None, DEFAULT_PORT_SCAN_PORTS).unwrap();
        assert_eq!(parsed.normalized, "1-1024");
        assert_eq!(parsed.count, 1024);
        assert!(parse_port_spec(Some(""), DEFAULT_PORT_SCAN_PORTS).is_ok());
        assert!(parse_port_spec(Some("0"), DEFAULT_PORT_SCAN_PORTS).is_err());
        assert!(parse_port_spec(Some("1-70000"), DEFAULT_PORT_SCAN_PORTS).is_err());
        assert!(
            parse_port_spec(Some("22,80,443"), DEFAULT_PORT_SCAN_PORTS)
                .unwrap()
                .count
                == 3
        );
    }

    #[test]
    fn scan_type_parsing() {
        assert_eq!(parse_scan_type(None).unwrap(), ScanType::Syn);
        assert_eq!(parse_scan_type(Some("SYN")).unwrap(), ScanType::Syn);
        assert_eq!(parse_scan_type(Some("xmas")).unwrap(), ScanType::Xmas);
        assert!(parse_scan_type(Some("bogus")).is_err());
    }

    #[test]
    fn scan_profile_rejects_unknown() {
        assert_eq!(
            parse_scan_profile(None).unwrap().as_str(),
            DEFAULT_SCAN_PROFILE
        );
        assert_eq!(parse_scan_profile(Some("web")).unwrap().as_str(), "web");
        assert!(parse_scan_profile(Some("bogus-profile")).is_err());
    }

    #[test]
    fn load_test_legacy_connections_semantics() {
        let (req, conc) = resolve_load_test_counts(Some(1000), Some(10)).unwrap();
        assert_eq!((req, conc), (1000, 10));
        let (req, conc) = resolve_load_test_counts(None, Some(25)).unwrap();
        assert_eq!((req, conc), (25, 25));
        let (req, conc) = resolve_load_test_counts(None, None).unwrap();
        assert_eq!(
            (req, conc),
            (DEFAULT_LOAD_REQUESTS, DEFAULT_LOAD_CONCURRENCY)
        );
    }

    #[test]
    fn fuzz_canonical_defaults_match_cli() {
        let req = FuzzRequest {
            target: "https://example.com".into(),
            payload_type: None,
            mode: None,
            method: None,
            param: None,
            threads: None,
            timeout_secs: None,
            mutations: None,
            mutation_count: None,
            graphql_introspection: None,
            graphql_depth_bypass: None,
            graphql_alias_overload: None,
            oauth_redirect_test: None,
            oauth_scope_test: None,
            oauth_state_test: None,
            oauth_grant_test: None,
        };
        let n = req.normalize().unwrap();
        assert_eq!(n.payload_type, DEFAULT_FUZZ_PAYLOAD_TYPE);
        assert_eq!(n.method, DEFAULT_FUZZ_METHOD);
        assert_eq!(n.mutation_count, DEFAULT_FUZZ_MUTATION_COUNT);
        assert_eq!(n.graphql_introspection, DEFAULT_GRAPHQL_INTROSPECTION);
        assert_eq!(n.oauth_redirect_test, DEFAULT_OAUTH_REDIRECT_TEST);
    }

    #[test]
    fn endpoint_canonical_concurrency_is_20() {
        let req = EndpointScanRequest {
            target: "https://example.com".into(),
            concurrency: None,
            timeout_secs: None,
            wordlist: None,
            include_404: None,
        };
        let n = req.normalize().unwrap();
        assert_eq!(n.concurrency, DEFAULT_ENDPOINT_CONCURRENCY);
        assert_eq!(n.concurrency, 20);
        assert_eq!(n.timeout_secs, DEFAULT_ENDPOINT_TIMEOUT_SECS);
    }

    #[test]
    fn fingerprint_canonical_ports_match_cli() {
        let req = FingerprintRequest {
            target: "example.com".into(),
            ports: None,
            timeout_secs: None,
            concurrency: None,
        };
        let n = req.normalize().unwrap();
        assert_eq!(n.ports, DEFAULT_FINGERPRINT_PORTS);
        assert_eq!(n.timeout_secs, DEFAULT_FINGERPRINT_TIMEOUT_SECS);
    }

    #[test]
    fn target_normalization_rejects_empty() {
        assert!(normalize_target_value("  ").is_err());
        assert_eq!(
            normalize_target_value("  example.com  ").unwrap(),
            "example.com"
        );
    }

    #[test]
    fn tool_params_validation_single_owner() {
        let params = serde_json::json!({"target": "example.com"});
        assert_eq!(validate_tool_params("recon", &params).unwrap(), "recon");
        let bad = serde_json::json!({"target": "   "});
        assert!(validate_tool_params("recon", &bad).is_err());
        assert!(validate_tool_params("bogus-op", &params).is_err());
    }

    // ── load-test request shape ──

    fn load_test_req(
        method: Option<&str>,
        body: Option<&str>,
        headers: &[&str],
    ) -> LoadTestRequest {
        LoadTestRequest {
            target: "https://example.com".into(),
            method: method.map(str::to_string),
            requests: Some(10),
            connections: Some(2),
            duration_secs: None,
            rate_limit: None,
            body: body.map(str::to_string),
            headers: headers.iter().map(|h| h.to_string()).collect(),
        }
    }

    #[test]
    fn load_test_method_defaults_and_canonicalizes() {
        assert_eq!(
            load_test_req(None, None, &[]).normalize().unwrap().method,
            "GET"
        );
        assert_eq!(
            load_test_req(Some("  "), None, &[])
                .normalize()
                .unwrap()
                .method,
            "GET"
        );
        assert_eq!(
            load_test_req(Some(" post "), None, &[])
                .normalize()
                .unwrap()
                .method,
            "POST"
        );
        // Not an RFC 9110 token.
        assert!(load_test_req(Some("PO ST"), None, &[]).normalize().is_err());
        assert!(load_test_req(Some("GET\r\nX: y"), None, &[])
            .normalize()
            .is_err());
    }

    #[test]
    fn load_test_body_rejected_for_bodyless_methods() {
        // Rejected, not silently dropped: a caller expecting its payload to be
        // sent must learn that it is not.
        for method in ["GET", "head", "Head"] {
            let err = load_test_req(Some(method), Some("payload"), &[])
                .normalize()
                .unwrap_err();
            assert!(
                err.to_string().contains("not allowed with method"),
                "unexpected error for {method}: {err}"
            );
        }
        assert_eq!(
            load_test_req(Some("POST"), Some("payload"), &[])
                .normalize()
                .unwrap()
                .body
                .as_deref(),
            Some("payload")
        );
        // Blank body is absent, not an empty payload.
        assert!(load_test_req(Some("GET"), Some("   "), &[])
            .normalize()
            .unwrap()
            .body
            .is_none());
        assert!(load_test_req(Some("POST"), Some(""), &[])
            .normalize()
            .unwrap()
            .body
            .is_none());
    }

    #[test]
    fn load_test_body_size_is_capped() {
        let big = "x".repeat(MAX_LOAD_BODY_BYTES + 1);
        let err = load_test_req(Some("POST"), Some(&big), &[])
            .normalize()
            .unwrap_err();
        assert!(err.to_string().contains("above maximum"), "{err}");

        let ok = "x".repeat(MAX_LOAD_BODY_BYTES);
        assert!(load_test_req(Some("POST"), Some(&ok), &[])
            .normalize()
            .is_ok());
    }

    #[test]
    fn load_test_headers_canonicalize_to_name_colon_value() {
        let n = load_test_req(
            Some("POST"),
            None,
            &["X-Probe: eggsec", "  Accept  :  application/json  "],
        )
        .normalize()
        .unwrap();
        assert_eq!(
            n.headers,
            vec![
                "X-Probe:eggsec".to_string(),
                "Accept:application/json".to_string()
            ]
        );
    }

    #[test]
    fn load_test_headers_reject_malformed_and_injected_entries() {
        // Missing colon.
        assert!(load_test_req(Some("POST"), None, &["X-Probe"])
            .normalize()
            .is_err());
        // Empty name.
        assert!(load_test_req(Some("POST"), None, &[":value"])
            .normalize()
            .is_err());
        // Non-token name.
        assert!(load_test_req(Some("POST"), None, &["X Probe: v"])
            .normalize()
            .is_err());
        // CR/LF in value — the header-injection primitive.
        assert!(
            load_test_req(Some("POST"), None, &["X-Probe: a\r\nX-Evil: b"])
                .normalize()
                .is_err()
        );
    }

    #[test]
    fn load_test_headers_reject_retargeting_and_framing_headers() {
        // Deny-list is matched case-insensitively.
        for name in [
            "Host",
            "host",
            "Connection",
            "Transfer-Encoding",
            "content-length",
            "TE",
            "Upgrade",
        ] {
            let entry = format!("{name}: x");
            let err = load_test_req(Some("POST"), None, &[entry.as_str()])
                .normalize()
                .unwrap_err();
            assert!(
                err.to_string().contains("not settable by callers"),
                "unexpected error for {name}: {err}"
            );
        }
        // An unrelated header is unaffected.
        assert!(load_test_req(Some("POST"), None, &["X-Host: x"])
            .normalize()
            .is_ok());
    }

    #[test]
    fn load_test_headers_count_and_size_are_capped() {
        let too_many: Vec<String> = (0..=MAX_LOAD_HEADERS)
            .map(|i| format!("X-H{i}: v"))
            .collect();
        let refs: Vec<&str> = too_many.iter().map(String::as_str).collect();
        let err = load_test_req(Some("POST"), None, &refs)
            .normalize()
            .unwrap_err();
        assert!(err.to_string().contains("above maximum"), "{err}");

        let long_name = format!("{}: v", "x".repeat(MAX_LOAD_HEADER_NAME_BYTES + 1));
        assert!(load_test_req(Some("POST"), None, &[long_name.as_str()])
            .normalize()
            .is_err());

        let long_value = format!("X-Probe: {}", "v".repeat(MAX_LOAD_HEADER_VALUE_BYTES + 1));
        assert!(load_test_req(Some("POST"), None, &[long_value.as_str()])
            .normalize()
            .is_err());
    }

    // ── storage ──

    fn storage_req() -> StorageRequest {
        StorageRequest {
            storage_type: "postgres".into(),
            path: None,
            host: None,
            port: None,
            database: None,
            username: None,
            max_connections: None,
            mode: None,
            scan_id: None,
            cve_id: None,
            severity_filter: None,
            password_env: None,
        }
    }

    #[test]
    fn storage_defaults_match_engine_storage_config() {
        let n = storage_req().normalize().unwrap();
        assert_eq!(n.mode, DEFAULT_STORAGE_MODE);
        assert_eq!(n.host, DEFAULT_STORAGE_HOST);
        assert_eq!(n.port, DEFAULT_STORAGE_PORT);
        assert_eq!(n.database, DEFAULT_STORAGE_DATABASE);
        assert_eq!(n.username, DEFAULT_STORAGE_USERNAME);
        assert_eq!(n.max_connections, DEFAULT_STORAGE_MAX_CONNECTIONS);
    }

    #[test]
    fn storage_mode_fails_closed_on_unknown() {
        // The executor turns an unknown mode into an error only *after*
        // dialling the database, so this must reject before any I/O.
        // Blank is "unset", not a mode name, so it falls back to the default.
        for mode in ["", "  "] {
            let mut req = storage_req();
            req.mode = Some(mode.to_string());
            let n = req.normalize().unwrap();
            assert_eq!(n.mode, DEFAULT_STORAGE_MODE, "mode {mode:?}");
        }
        // Unknown modes are rejected regardless of casing.
        for mode in ["read", "READ", "delete", "drop", "list scans"] {
            let mut req = storage_req();
            req.mode = Some(mode.to_string());
            assert!(
                req.normalize().is_err(),
                "unknown mode {mode:?} was accepted"
            );
        }
        // Known modes, case-insensitively.
        for mode in KNOWN_STORAGE_MODES {
            let mut req = storage_req();
            req.mode = Some(mode.to_ascii_uppercase());
            // `search_cve` has its own required-argument rule, asserted above.
            if *mode == "search_cve" {
                req.cve_id = Some("CVE-2021-44228".into());
            }
            assert_eq!(req.normalize().unwrap().mode, *mode);
        }
    }

    #[test]
    fn storage_search_cve_requires_a_cve_id() {
        let mut req = storage_req();
        req.mode = Some("search_cve".into());
        let err = req.normalize().unwrap_err();
        assert!(err.to_string().contains("requires cve_id"), "{err}");

        req.cve_id = Some("CVE-2021-44228".into());
        let n = req.normalize().unwrap();
        assert_eq!(n.cve_id.as_deref(), Some("CVE-2021-44228"));
    }

    #[test]
    fn storage_severity_filter_is_validated() {
        let mut req = storage_req();
        req.severity_filter = Some("HIGH".into());
        assert_eq!(
            req.normalize().unwrap().severity_filter.as_deref(),
            Some("high")
        );
        req.severity_filter = Some("bogus".into());
        assert!(req.normalize().is_err());
        // Blank is absent, not a literal empty filter.
        req.severity_filter = Some("   ".into());
        assert!(req.normalize().unwrap().severity_filter.is_none());
    }

    #[test]
    fn storage_port_and_pool_are_bounded() {
        let mut req = storage_req();
        req.port = Some(0);
        assert!(req.normalize().is_err());
        req.port = Some(5432);
        req.max_connections = Some(0);
        assert!(req.normalize().is_err());
        req.max_connections = Some(MAX_STORAGE_MAX_CONNECTIONS + 1);
        assert!(req.normalize().is_err());
        req.max_connections = Some(1);
        assert_eq!(req.normalize().unwrap().max_connections, 1);
    }

    #[test]
    fn storage_password_is_a_variable_name_never_a_value() {
        let mut req = storage_req();
        req.password_env = Some("EGGSEC_PG_PASSWORD".into());
        assert_eq!(
            req.normalize().unwrap().password_env.as_deref(),
            Some("EGGSEC_PG_PASSWORD")
        );
        // A value cannot masquerade as a variable name.
        for bad in ["", "  ", "1BAD", "has space", "has-dash", "a.b"] {
            req.password_env = Some(bad.to_string());
            assert!(
                req.normalize().is_err(),
                "password_env {bad:?} was accepted as a variable name"
            );
        }
    }

    #[test]
    fn storage_absent_optional_fields_deserialize_for_older_wire_clients() {
        // `runtime.request.StorageParams` is embedded in `TaskKind` and is
        // persisted into the daemon's snapshot JSON, so every added field must
        // carry `#[serde(default)]` or older payloads stop parsing.
        let legacy = serde_json::json!({"storage_type": "postgres"});
        let req: StorageRequest = serde_json::from_value(legacy).unwrap();
        let n = req.normalize().unwrap();
        assert_eq!(n.mode, DEFAULT_STORAGE_MODE);
        assert!(n.password_env.is_none());
        assert!(n.severity_filter.is_none());
    }

    #[test]
    fn load_test_absent_headers_deserialize_for_older_wire_clients() {
        // `body`/`headers` are `#[serde(default)]`, so a payload written before
        // they existed still parses (a missing `Option` is not defaulted by
        // serde without the attribute).
        let legacy = serde_json::json!({
            "target": "https://example.com",
            "method": "POST",
        });
        let req: LoadTestRequest = serde_json::from_value(legacy).unwrap();
        let n = req.normalize().unwrap();
        assert!(n.body.is_none());
        assert!(n.headers.is_empty());
    }
}
