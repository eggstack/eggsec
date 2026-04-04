use crate::config::Scope;
use crate::error::{Result, SlapperError};
use crate::utils::target::extract_target_from_url;

pub fn check_scope(scope: &Scope, target: &str) -> Result<()> {
    if !scope.is_target_allowed(target)? {
        return Err(SlapperError::ScopeViolation(format!(
            "Target {} is not in allowed scope",
            target
        )));
    }
    Ok(())
}

pub fn check_scope_from_url(scope: &Scope, url: &str) -> Result<()> {
    let target = extract_target_from_url(url)
        .ok_or_else(|| SlapperError::Parse(format!("Failed to parse URL: {}", url)))?;
    check_scope(scope, &target)
}
