//! Engine adaptation above the load-test core (Phase D WS2).
//!
//! Translates CLI/TUI/tool/runtime DTOs, `EggsecConfig` defaults, auth
//! context, and policy-approved targets into a plain [`LoadTestPlan`] plus
//! transport-neutral request-template parts. The reusable plan never stores
//! [`CommonHttpArgs`]: that type is an Eggsec surface adapter, not a
//! load-test domain primitive.
//!
//! Header/auth application uses the canonical transport-neutral path
//! ([`eggsec_transport::apply_auth_headers`] semantics over a `HeaderMap`)
//! rather than reimplementing Basic/Bearer/API-key/cookie parsing inside the
//! executor. The parsing of CLI flag shapes (`user:pass`, `Name:value`)
//! stays here, at the adapter boundary.

use std::collections::HashMap;
use std::time::Duration;

use super::plan::{normalize_method, LoadTestPlan, RatePolicy};

/// Transport-neutral request template derived from the adapter.
///
/// The executor compiles this plus the [`LoadTestPlan`] into one prototype
/// [`eggsec_transport::ScopedHttpRequest`] once per run (method/URL/headers/
/// body + timeout/redirect/proxy/TLS/hints values) and dispatches cheap
/// clones of it. Per-request authorization still runs inside
/// [`eggsec_transport::HttpTransport::execute`]; nothing policy-relevant is
/// cached.
#[derive(Debug, Clone)]
pub struct RequestTemplate {
    /// Extra headers as ordered pairs (auth already applied).
    pub headers: Vec<(String, String)>,
    /// Replayable body bytes.
    pub body: Option<Vec<u8>>,
    /// `User-Agent` value (also carried in `hints`).
    pub user_agent: String,
    /// Timeout policy value.
    pub timeout: eggsec_transport::TimeoutPolicy,
    /// Redirect policy value.
    pub redirect: eggsec_transport::RedirectPolicy,
    /// Proxy intent value.
    pub proxy: eggsec_transport::ProxyIntent,
    /// TLS policy value.
    pub tls: eggsec_transport::TlsPolicy,
}

impl RequestTemplate {
    /// Build the per-request transport DTO (pure, no I/O).
    pub fn scoped_request(
        &self,
        plan: &LoadTestPlan,
    ) -> Result<eggsec_transport::ScopedHttpRequest, String> {
        use eggsec_transport::{RequestBody, ScopedHttpRequest, TransportHints};

        let method: eggsec_transport::Method = plan
            .method
            .parse()
            .map_err(|e| format!("invalid method '{}': {e}", plan.method))?;
        let url =
            url::Url::parse(&plan.url).map_err(|e| format!("invalid URL '{}': {e}", plan.url))?;
        let mut req = ScopedHttpRequest::new(method, url).map_err(|e| e.to_string())?;

        let mut pairs: HashMap<String, String> = HashMap::new();
        for (k, v) in &self.headers {
            pairs.insert(k.clone(), v.clone());
        }
        req = req.with_headers(&pairs).map_err(|e| e.to_string())?;
        let user_agent_name: eggsec_transport::HeaderName = "user-agent"
            .parse()
            .map_err(|e| format!("bad header name: {e}"))?;
        if !self.user_agent.is_empty() && !req.headers.contains_key(&user_agent_name) {
            req = req
                .with_header("User-Agent", &self.user_agent)
                .map_err(|e| e.to_string())?;
        }
        req = req.with_body(match &self.body {
            Some(b) => RequestBody::from_bytes(b.clone()),
            None => RequestBody::Empty,
        });
        req = req
            .with_timeout(self.timeout.clone())
            .with_redirect(self.redirect)
            .with_proxy(self.proxy.clone())
            .with_tls(self.tls.clone())
            .with_hints(TransportHints {
                user_agent: Some(self.user_agent.clone()),
                tcp_nodelay: true,
            });
        Ok(req)
    }
}

/// Parse CLI auth-flag shapes into header/cookie maps (adapter-local).
///
/// Shapes (stable, matching the pre-Phase-D runner):
/// - `auth: "user:pass"` → `Authorization: Basic base64(user:pass)`.
/// - `bearer` → `Authorization: Bearer <token>` (overwrites Basic).
/// - `cookie` → merged into the `Cookie` header (single value form).
/// - `api_key: "Name:value"` → that header; bare value → `X-API-Key`.
fn auth_maps(
    auth: Option<&str>,
    bearer: Option<&str>,
    cookie: Option<&str>,
    api_key: Option<&str>,
) -> (HashMap<String, String>, HashMap<String, String>) {
    use base64::{engine::general_purpose, Engine as _};

    let mut headers = HashMap::new();
    let mut cookies = HashMap::new();

    if let Some(auth) = auth {
        let parts: Vec<&str> = auth.splitn(2, ':').collect();
        if parts.len() == 2 {
            let encoded = general_purpose::STANDARD.encode(format!("{}:{}", parts[0], parts[1]));
            headers.insert("Authorization".to_string(), format!("Basic {encoded}"));
        } else {
            tracing::warn!("Invalid auth format (expected 'user:password'), ignoring basic auth");
        }
    }
    if let Some(bearer) = bearer {
        headers.insert("Authorization".to_string(), format!("Bearer {bearer}"));
    }
    if let Some(cookie) = cookie {
        // `Cookie: a=1; b=2` shape: split into the cookie map so the
        // canonical merge (not string concat) applies.
        for pair in cookie.split(';') {
            let pair = pair.trim();
            if pair.is_empty() {
                continue;
            }
            if let Some((k, v)) = pair.split_once('=') {
                cookies.insert(k.trim().to_string(), v.trim().to_string());
            } else {
                cookies.insert(pair.to_string(), String::new());
            }
        }
    }
    if let Some(api_key) = api_key {
        if api_key.contains(':') {
            let parts: Vec<&str> = api_key.splitn(2, ':').collect();
            headers.insert(parts[0].to_string(), parts[1].to_string());
        } else {
            headers.insert("X-API-Key".to_string(), api_key.to_string());
        }
    }
    (headers, cookies)
}

/// Apply auth maps through the canonical transport-neutral helper.
///
/// Returns the merged `Cookie` header value when cookies exist (so the
/// caller can set it as a plain header pair without touching `HeaderMap`
/// directly).
fn apply_auth_canonical(
    headers: &mut HashMap<String, String>,
    auth_headers: &HashMap<String, String>,
    auth_cookies: &HashMap<String, String>,
) {
    for (k, v) in auth_headers {
        headers.insert(k.clone(), v.clone());
    }
    if !auth_cookies.is_empty() {
        let existing = headers.get("Cookie").map(String::as_str);
        let merged = eggsec_transport::merge_cookie_header(existing, auth_cookies);
        headers.insert("Cookie".to_string(), merged);
    }
}

/// Adapter input: plain run config + engine defaults.
///
/// This mirrors the pre-Phase-D `LoadTestRunConfig` + `EggsecConfig` merge
/// without storing either type in the reusable plan.
pub struct AdapterInput<'a> {
    pub url: &'a str,
    pub requests: u64,
    pub concurrency: usize,
    pub timeout: Duration,
    pub method: &'a str,
    pub body: Option<&'a str>,
    pub headers: &'a [String],
    pub insecure: bool,
    pub proxy: Option<&'a str>,
    pub proxy_auth: Option<&'a str>,
    pub auth: Option<&'a str>,
    pub bearer: Option<&'a str>,
    pub cookie: Option<&'a str>,
    pub api_key: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub rate_limit: Option<u32>,
}

/// Build a [`LoadTestPlan`] + [`RequestTemplate`] from adapter input and
/// engine defaults.
///
/// `config_timeout_secs`, `config_verify_tls`, `config_proxy`,
/// `config_proxy_auth`, `config_rate`, `config_user_agent`, and
/// `config_default_headers` carry the `EggsecConfig` contribution; `None`
/// means "no engine defaults" (pure `CommonHttpArgs`-style input).
#[allow(clippy::too_many_arguments)]
pub fn plan_from_adapter(
    input: AdapterInput<'_>,
    config_timeout_secs: Option<u64>,
    config_verify_tls: bool,
    config_proxy: Option<String>,
    config_proxy_auth: Option<String>,
    config_rate: Option<u32>,
    config_user_agent: Option<String>,
    config_default_headers: &HashMap<String, String>,
) -> Result<(LoadTestPlan, RequestTemplate), String> {
    let timeout = if input.timeout.is_zero() {
        Duration::from_secs(config_timeout_secs.unwrap_or(30).max(1))
    } else {
        input.timeout
    };

    let mut plan = LoadTestPlan::new(
        input.url.to_string(),
        input.requests,
        input.concurrency,
        timeout,
    )?;
    let method = normalize_method(input.method);
    plan.method = method;
    plan.body = input.body.map(|b| b.as_bytes().to_vec());
    plan.rate = RatePolicy::from_optional(input.rate_limit.or(config_rate));
    if input.rate_limit == Some(0) {
        tracing::warn!("Rate limit of 0 is invalid, ignoring rate limit setting");
        plan.rate = RatePolicy::Unlimited;
    } else if let Some(rate) = input.rate_limit {
        if rate > 100_000 {
            tracing::warn!(
                "Rate limit {} req/s exceeds recommended maximum of 100,000; \
                 rate limiting may be ineffective at this level",
                rate
            );
        }
    }

    // Headers: explicit `--header` pairs first, then config defaults (which
    // do not overwrite explicit values), then canonical auth application.
    let mut merged: HashMap<String, String> = HashMap::new();
    for (k, v) in crate::utils::parse_headers(input.headers) {
        merged.insert(k, v);
    }
    for (k, v) in config_default_headers {
        merged.entry(k.clone()).or_insert_with(|| v.clone());
    }
    let (auth_headers, auth_cookies) =
        auth_maps(input.auth, input.bearer, input.cookie, input.api_key);
    apply_auth_canonical(&mut merged, &auth_headers, &auth_cookies);

    let mut header_pairs: Vec<(String, String)> = merged.into_iter().collect();
    header_pairs.sort_by(|a, b| a.0.cmp(&b.0));
    plan.headers = header_pairs.clone();

    let insecure = input.insecure || !config_verify_tls;
    if insecure {
        tracing::warn!(
            "TLS certificate verification disabled. This is insecure and should only \
             be used in isolated testing environments."
        );
    }
    let proxy_url = input.proxy.map(str::to_string).or(config_proxy);
    let proxy_auth = input.proxy_auth.map(str::to_string).or(config_proxy_auth);
    let proxy = proxy_intent(proxy_url.as_deref(), proxy_auth.as_deref())?;

    let user_agent = input
        .user_agent
        .map(str::to_string)
        .or(config_user_agent)
        .unwrap_or_else(crate::utils::http::tool_user_agent);

    let template = RequestTemplate {
        headers: header_pairs,
        body: plan.body.clone(),
        user_agent,
        timeout: eggsec_transport::TimeoutPolicy {
            request_timeout: timeout,
            connect_timeout: None,
        },
        redirect: eggsec_transport::RedirectPolicy::SameHostOnly {
            max_redirects: eggsec_core::constants::http::DEFAULT_MAX_REDIRECTS as u8,
        },
        proxy,
        tls: if insecure {
            eggsec_transport::TlsPolicy::insecure()
        } else {
            eggsec_transport::TlsPolicy::verified()
        },
    };
    Ok((plan, template))
}

/// Build a transport-neutral proxy intent from CLI/config strings.
fn proxy_intent(
    proxy: Option<&str>,
    proxy_auth: Option<&str>,
) -> Result<eggsec_transport::ProxyIntent, String> {
    use eggsec_transport::{ProxyCredential, ProxyIntent};
    let Some(endpoint) = proxy else {
        return Ok(ProxyIntent::Direct);
    };
    let url =
        url::Url::parse(endpoint).map_err(|e| format!("invalid proxy URL '{endpoint}': {e}"))?;
    let credential = match proxy_auth {
        Some(auth) => {
            let parts: Vec<&str> = auth.splitn(2, ':').collect();
            if parts.len() == 2 {
                Some(ProxyCredential::new(parts[0], parts[1]))
            } else {
                tracing::warn!("Invalid proxy auth format (expected 'user:password'), ignoring");
                None
            }
        }
        None => None,
    };
    // `Proxy::all`-equivalent: route every scheme through the endpoint.
    Ok(ProxyIntent::All {
        endpoint: url,
        credential,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input<'a>(url: &'a str, headers: &'a [String]) -> AdapterInput<'a> {
        AdapterInput {
            url,
            requests: 5,
            concurrency: 2,
            timeout: Duration::from_secs(5),
            method: "GET",
            body: None,
            headers,
            insecure: false,
            proxy: None,
            proxy_auth: None,
            auth: None,
            bearer: None,
            cookie: None,
            api_key: None,
            user_agent: None,
            rate_limit: None,
        }
    }

    #[test]
    fn basic_auth_encodes_and_bearer_wins() {
        let in_ = AdapterInput {
            auth: Some("user:pass"),
            bearer: Some("tok"),
            ..input("http://example.com/", &[])
        };
        let (plan, _) = plan_from_adapter(in_, None, true, None, None, None, None, &HashMap::new())
            .expect("plan");
        let auth = plan
            .headers
            .iter()
            .find(|(k, _)| k == "Authorization")
            .expect("auth header");
        assert_eq!(auth.1, "Bearer tok");
    }

    #[test]
    fn api_key_shapes() {
        let in_ = AdapterInput {
            api_key: Some("X-Custom:k"),
            ..input("http://example.com/", &[])
        };
        let (plan, _) = plan_from_adapter(in_, None, true, None, None, None, None, &HashMap::new())
            .expect("plan");
        assert!(plan
            .headers
            .iter()
            .any(|(k, v)| k == "X-Custom" && v == "k"));

        let in_ = AdapterInput {
            api_key: Some("bare"),
            ..input("http://example.com/", &[])
        };
        let (plan, _) = plan_from_adapter(in_, None, true, None, None, None, None, &HashMap::new())
            .expect("plan");
        assert!(plan
            .headers
            .iter()
            .any(|(k, v)| k == "X-API-Key" && v == "bare"));
    }

    #[test]
    fn cookie_merges_with_existing_header() {
        let headers = vec!["Cookie: keep=1".to_string()];
        let in_ = AdapterInput {
            cookie: Some("session=new"),
            ..input("http://example.com/", &headers)
        };
        let (plan, _) = plan_from_adapter(in_, None, true, None, None, None, None, &HashMap::new())
            .expect("plan");
        let cookie = plan
            .headers
            .iter()
            .find(|(k, _)| k == "Cookie")
            .expect("cookie");
        assert!(cookie.1.contains("keep=1"), "lost: {}", cookie.1);
        assert!(cookie.1.contains("session=new"), "missing: {}", cookie.1);
    }

    #[test]
    fn proxy_intent_parses_endpoint_and_credential() {
        let in_ = AdapterInput {
            proxy: Some("http://127.0.0.1:8080"),
            proxy_auth: Some("u:p"),
            ..input("http://example.com/", &[])
        };
        let (_, template) =
            plan_from_adapter(in_, None, true, None, None, None, None, &HashMap::new())
                .expect("plan");
        assert!(template.proxy.uses_proxy());
    }

    #[test]
    fn zero_rate_is_unlimited_and_timeout_falls_back() {
        let in_ = AdapterInput {
            rate_limit: Some(0),
            timeout: Duration::ZERO,
            ..input("http://example.com/", &[])
        };
        let (plan, _) =
            plan_from_adapter(in_, Some(42), true, None, None, None, None, &HashMap::new())
                .expect("plan");
        assert_eq!(plan.rate, RatePolicy::Unlimited);
        assert_eq!(plan.timeout, Duration::from_secs(42));
    }

    #[test]
    fn template_builds_transport_request() {
        let (plan, template) = plan_from_adapter(
            input("http://example.com/a", &[]),
            None,
            true,
            None,
            None,
            None,
            None,
            &HashMap::new(),
        )
        .expect("plan");
        let req = template.scoped_request(&plan).expect("request");
        assert_eq!(req.url.as_str(), "http://example.com/a");
        assert!(req.tls.is_verified());
    }
}
