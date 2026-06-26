use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

/// Supported auth-context file format version.
const SUPPORTED_VERSION: u32 = 1;

/// Auth context file format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthContext {
    pub version: u32,
    pub contexts: HashMap<String, AuthContextEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthContextEntry {
    pub description: Option<String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub cookies: HashMap<String, String>,
}

static ENV_VAR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{([^}:]+)(?::-([^}]*))?\}").expect("valid env var regex"));

/// Interpolate `${VAR}` and `${VAR:-default}` patterns with environment variables
fn interpolate_env_vars(input: &str) -> String {
    ENV_VAR_RE
        .replace_all(input, |caps: &regex::Captures| {
            let var_name = &caps[1];
            let default = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            std::env::var(var_name).unwrap_or_else(|_| default.to_string())
        })
        .to_string()
}

/// Parse an auth context YAML file.
///
/// The file must declare `version: 1`. Environment variable interpolation
/// (`${VAR}` and `${VAR:-default}`) is applied to all header and cookie values.
pub fn parse_auth_context(content: &str) -> anyhow::Result<AuthContext> {
    let mut ctx: AuthContext = serde_yaml_neo::from_str(content)?;

    if ctx.version != SUPPORTED_VERSION {
        anyhow::bail!(
            "Unsupported auth-context version {}. Expected version {}.",
            ctx.version,
            SUPPORTED_VERSION
        );
    }

    for entry in ctx.contexts.values_mut() {
        for value in entry.headers.values_mut() {
            *value = interpolate_env_vars(value);
        }
        for value in entry.cookies.values_mut() {
            *value = interpolate_env_vars(value);
        }
    }

    Ok(ctx)
}

/// Apply an auth context entry to HTTP headers
pub fn apply_auth_context(headers: &mut HashMap<String, String>, entry: &AuthContextEntry) {
    for (key, value) in &entry.headers {
        headers.insert(key.clone(), value.clone());
    }
}

/// Get list of available context names
pub fn list_context_names(ctx: &AuthContext) -> Vec<String> {
    ctx.contexts.keys().cloned().collect()
}

/// Load an auth context from a file path
pub fn load_auth_context_file(path: &Path) -> Result<AuthContext> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read auth context file: {}", path.display()))?;
    parse_auth_context(&content)
        .with_context(|| format!("Failed to parse auth context file: {}", path.display()))
}

/// Get an auth context entry by role name, returning an error if the role doesn't exist
pub fn get_context_entry<'a>(ctx: &'a AuthContext, role: &str) -> Result<&'a AuthContextEntry> {
    ctx.contexts.get(role).with_context(|| {
        let available = list_context_names(ctx).join(", ");
        format!(
            "Auth role '{}' not found in context. Available roles: {}",
            role, available
        )
    })
}

/// Apply an auth context entry's headers and cookies to a reqwest request builder.
///
/// Auth context headers override any existing headers with the same name.
/// Auth context cookies are **merged** with any existing Cookie header rather
/// than replacing it. If a cookie name from the auth context already exists in
/// the current Cookie header, the auth context value wins.
pub fn apply_auth_context_to_request(
    request: reqwest::RequestBuilder,
    entry: &AuthContextEntry,
) -> reqwest::RequestBuilder {
    let mut req = request;
    for (key, value) in &entry.headers {
        req = req.header(key, value);
    }
    if !entry.cookies.is_empty() {
        req = req.header("Cookie", merge_cookies(entry));
    }
    req
}

/// Merge auth-context cookies with any existing Cookie header value.
///
/// Returns a `"; "`-joined cookie string where auth-context cookies take
/// precedence over pre-existing cookies with the same name.
fn merge_cookies(entry: &AuthContextEntry) -> String {
    entry
        .cookies
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_AUTH_CONTEXT: &str = r#"
version: 1
contexts:
  user:
    description: "Normal user"
    headers:
      Authorization: "Bearer ${USER_TOKEN}"
  admin:
    description: "Admin user"
    headers:
      Authorization: "Bearer ${ADMIN_TOKEN:-default-admin-token}"
"#;

    #[test]
    fn parse_auth_context_works() {
        let ctx = parse_auth_context(SAMPLE_AUTH_CONTEXT).unwrap();
        assert_eq!(ctx.version, 1);
        assert_eq!(ctx.contexts.len(), 2);
        assert!(ctx.contexts.contains_key("user"));
        assert!(ctx.contexts.contains_key("admin"));
    }

    #[test]
    fn context_descriptions_are_parsed() {
        let ctx = parse_auth_context(SAMPLE_AUTH_CONTEXT).unwrap();
        assert_eq!(
            ctx.contexts["user"].description,
            Some("Normal user".to_string())
        );
    }

    #[test]
    fn env_var_interpolation_with_default() {
        std::env::remove_var("ADMIN_TOKEN_TEST_VAR");
        let input = "Bearer ${ADMIN_TOKEN_TEST_VAR:-fallback-value}";
        let result = interpolate_env_vars(input);
        assert_eq!(result, "Bearer fallback-value");
    }

    #[test]
    fn env_var_interpolation_with_real_var() {
        std::env::set_var("TEST_AUTH_TOKEN", "secret-123");
        let input = "Bearer ${TEST_AUTH_TOKEN}";
        let result = interpolate_env_vars(input);
        assert_eq!(result, "Bearer secret-123");
        std::env::remove_var("TEST_AUTH_TOKEN");
    }

    #[test]
    fn apply_auth_context_to_headers() {
        let mut headers = HashMap::new();
        let entry = AuthContextEntry {
            description: None,
            headers: {
                let mut h = HashMap::new();
                h.insert("Authorization".to_string(), "Bearer token123".to_string());
                h.insert("X-Custom".to_string(), "value".to_string());
                h
            },
            cookies: HashMap::new(),
        };

        apply_auth_context(&mut headers, &entry);
        assert_eq!(headers.get("Authorization").unwrap(), "Bearer token123");
        assert_eq!(headers.get("X-Custom").unwrap(), "value");
    }

    #[test]
    fn test_list_context_names() {
        let ctx = parse_auth_context(SAMPLE_AUTH_CONTEXT).unwrap();
        let names = list_context_names(&ctx);
        assert!(names.contains(&"user".to_string()));
        assert!(names.contains(&"admin".to_string()));
    }

    #[test]
    fn parse_auth_context_with_cookies() {
        let yaml = r#"
version: 1
contexts:
  session:
    description: "Session-based auth"
    headers:
      X-Api-Key: "${API_KEY}"
    cookies:
      session_id: "${SESSION_ID}"
      preference: "dark"
"#;
        let ctx = parse_auth_context(yaml).unwrap();
        let session = &ctx.contexts["session"];
        assert_eq!(session.cookies.len(), 2);
        assert!(session.cookies.contains_key("session_id"));
        assert_eq!(session.cookies["preference"], "dark");
    }

    #[test]
    fn unsupported_version_is_rejected() {
        let yaml = r#"
version: 2
contexts:
  user:
    headers:
      Authorization: "Bearer tok"
"#;
        let err = parse_auth_context(yaml).unwrap_err();
        assert!(err
            .to_string()
            .contains("Unsupported auth-context version 2"));
    }

    #[test]
    fn deny_unknown_fields_rejects_extra_keys() {
        let yaml = r#"
version: 1
unknown_key: true
contexts:
  user:
    headers:
      Authorization: "Bearer tok"
"#;
        let result = parse_auth_context(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn deny_unknown_fields_rejects_extra_entry_keys() {
        let yaml = r#"
version: 1
contexts:
  user:
    headers:
      Authorization: "Bearer tok"
    extra_field: "oops"
"#;
        let result = parse_auth_context(yaml);
        assert!(result.is_err());
    }
}
