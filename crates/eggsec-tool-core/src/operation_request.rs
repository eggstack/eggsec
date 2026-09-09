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

// ── Canonical typed requests ──

/// Canonical port-scan request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortScanRequest {
    pub target: String,
    #[serde(default)]
    pub ports: Option<String>,
    #[serde(default)]
    pub scan_type: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub concurrency: Option<usize>,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedEndpointScan {
    pub target: String,
    pub concurrency: usize,
    pub timeout_secs: u64,
    pub wordlist: Option<String>,
}

impl EndpointScanRequest {
    pub fn normalize(&self) -> Result<NormalizedEndpointScan, NormalizationError> {
        Ok(NormalizedEndpointScan {
            target: normalize_target_value(&self.target)?,
            concurrency: normalize_concurrency(self.concurrency, DEFAULT_ENDPOINT_CONCURRENCY)?,
            timeout_secs: normalize_timeout_secs(self.timeout_secs, DEFAULT_ENDPOINT_TIMEOUT_SECS)?,
            wordlist: self.wordlist.clone().filter(|s| !s.trim().is_empty()),
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedLoadTest {
    pub target: String,
    pub method: String,
    pub requests: u64,
    pub concurrency: usize,
    pub duration_secs: u64,
    pub rate_limit: Option<u32>,
}

impl LoadTestRequest {
    pub fn normalize(&self) -> Result<NormalizedLoadTest, NormalizationError> {
        let target = normalize_target_value(&self.target)?;
        let (requests, concurrency) = resolve_load_test_counts(self.requests, self.connections)?;
        let duration_secs = normalize_timeout_secs(
            self.duration_secs.map(u64::from),
            DEFAULT_LOAD_DURATION_SECS,
        )?;
        let method = self
            .method
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_FUZZ_METHOD.to_string());
        Ok(NormalizedLoadTest {
            target,
            method,
            requests,
            concurrency,
            duration_secs,
            rate_limit: self.rate_limit,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedPipeline {
    pub target: String,
    pub profile: ScanProfileName,
}

impl PipelineRequest {
    pub fn normalize(&self) -> Result<NormalizedPipeline, NormalizationError> {
        Ok(NormalizedPipeline {
            target: normalize_target_value(&self.target)?,
            profile: parse_scan_profile(self.profile.as_deref())?,
        })
    }

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedStorage {
    pub storage_type: String,
    pub path: Option<String>,
}

impl StorageRequest {
    pub fn normalize(&self) -> Result<NormalizedStorage, NormalizationError> {
        let storage_type = self.storage_type.trim();
        if storage_type.is_empty() {
            return Err(err("storage_type must not be empty"));
        }
        Ok(NormalizedStorage {
            storage_type: storage_type.to_string(),
            path: self.path.clone().filter(|s| !s.trim().is_empty()),
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
}
