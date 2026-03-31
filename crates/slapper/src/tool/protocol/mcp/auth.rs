use subtle::ConstantTimeEq;

use super::types::McpError;

pub fn validate_auth_internal(
    api_key: &Option<String>,
    key_input: Option<&str>,
) -> Result<(), McpError> {
    if let Some(ref key) = api_key {
        match key_input {
            Some(v) if key.as_bytes().ct_eq(v.as_bytes()).unwrap_u8() == 1 => Ok(()),
            _ => Err(McpError::unauthorized()),
        }
    } else {
        Ok(())
    }
}

pub fn validate_auth(
    api_key: &Option<String>,
    headers: &axum::http::HeaderMap,
) -> Result<(), McpError> {
    let key = headers
        .get("authorization")
        .or_else(|| headers.get("x-api-key"))
        .and_then(|v| v.to_str().ok());
    validate_auth_internal(api_key, key)
}

pub fn validate_auth_params(
    api_key: &Option<String>,
    params: &Option<serde_json::Value>,
) -> Result<(), McpError> {
    let key = params
        .as_ref()
        .and_then(|p| p.get("api_key"))
        .and_then(|v| v.as_str());
    validate_auth_internal(api_key, key)
}
